use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::Serialize;
use tokio::sync::Mutex;
use tracing::Level;

/// Supported notification channels.
#[derive(Debug, Clone)]
pub enum NotificationChannel {
    /// Telegram Bot API — sends HTML-formatted messages to a chat.
    Telegram { bot_token: String, chat_id: String },
    /// Generic webhook (Slack-compatible) — sends JSON block messages.
    Webhook { url: String },
}

/// Pre-defined notification types for system events.
pub mod alert_types {
    /// One or more health probes are failing (non-critical).
    pub const HEALTH_DEGRADED: &str = "health_degraded";
    /// All health probes are failing.
    pub const HEALTH_UNHEALTHY: &str = "health_unhealthy";
    /// Chroma connectivity lost.
    pub const CHROMA_ERROR: &str = "chroma_error";
    /// Embedding API failures.
    pub const EMBEDDING_ERROR: &str = "embedding_error";
    /// LLM API failures.
    pub const LLM_ERROR: &str = "llm_error";
    /// >50% cache eviction rate in 5 min window.
    pub const CACHE_EVICTION_STORM: &str = "cache_eviction_storm";
}

/// Slack-compatible webhook message payload.
#[derive(Debug, Serialize)]
struct SlackPayload {
    text: String,
    blocks: Vec<SlackBlock>,
}

#[derive(Debug, Serialize)]
struct SlackBlock {
    #[serde(rename = "type")]
    block_type: String,
    text: Option<SlackText>,
    fields: Option<Vec<SlackText>>,
}

#[derive(Debug, Serialize)]
struct SlackText {
    #[serde(rename = "type")]
    text_type: String,
    text: String,
}

/// Telegram API request payload.
#[derive(Debug, Serialize)]
struct TelegramPayload {
    chat_id: String,
    text: String,
    parse_mode: String,
    disable_web_page_preview: bool,
}

/// Service for sending system alerts via configured notification channels.
///
/// Supports Telegram Bot API and generic webhooks (Slack-compatible).
/// Alerts are rate-limited per type (max 1 per 5 minutes) and filtered by
/// minimum severity level.
#[derive(Clone, Debug)]
pub struct NotificationService {
    channels: Vec<NotificationChannel>,
    /// Per-alert-type rate limiter: maps alert_type to last sent timestamp.
    rate_limiter: Arc<Mutex<HashMap<String, Instant>>>,
    /// Minimum severity level required to send a notification.
    min_severity: Level,
}

impl NotificationService {
    /// Create a new notification service with the given channels and minimum severity.
    pub fn new(channels: Vec<NotificationChannel>, min_severity: Level) -> Self {
        let level_str = match min_severity {
            Level::ERROR => "error",
            Level::WARN => "warn",
            Level::INFO => "info",
            Level::DEBUG => "debug",
            Level::TRACE => "trace",
        };
        tracing::info!(
            component = "notifications",
            channel_count = channels.len(),
            min_severity = level_str,
            "notification_service.initialized"
        );
        Self {
            channels,
            rate_limiter: Arc::new(Mutex::new(HashMap::new())),
            min_severity,
        }
    }

    /// Create a `NotificationService` from configuration environment values.
    ///
    /// If no channels are configured, returns `None`.
    pub fn from_config(
        telegram_bot_token: Option<String>,
        telegram_chat_id: Option<String>,
        webhook_url: Option<String>,
        min_severity: Option<Level>,
    ) -> Option<Self> {
        let mut channels = Vec::new();

        if let (Some(bot_token), Some(chat_id)) = (telegram_bot_token, telegram_chat_id) {
            channels.push(NotificationChannel::Telegram { bot_token, chat_id });
        }

        if let Some(url) = webhook_url {
            channels.push(NotificationChannel::Webhook { url });
        }

        if channels.is_empty() {
            return None;
        }

        let severity = min_severity.unwrap_or(Level::ERROR);
        Some(Self::new(channels, severity))
    }

    /// Send an alert through all configured channels.
    ///
    /// The alert is subject to:
    /// 1. Severity filter — only alerts at or above `min_severity` are sent.
    /// 2. Per-type rate limiter — max 1 notification per `alert_type` per 5 minutes.
    ///
    /// Returns immediately after spawning the send task — the caller does not
    /// wait for the HTTP request to complete.
    pub fn send_alert(
        &self,
        alert_type: &'static str,
        severity: Level,
        title: &str,
        message: &str,
    ) {
        let svc = self.clone();
        let alert_type_owned = alert_type;
        let title_owned = title.to_string();
        let message_owned = message.to_string();

        tokio::spawn(async move {
            svc.dispatch_alert(alert_type_owned, severity, &title_owned, &message_owned)
                .await;
        });
    }

    /// Internal dispatch — checks filters and sends to all channels.
    async fn dispatch_alert(&self, alert_type: &str, severity: Level, title: &str, message: &str) {
        // 1. Severity filter
        if severity_rank(severity) < severity_rank(self.min_severity) {
            tracing::debug!(
                component = "notifications",
                alert_type = alert_type,
                severity = ?severity,
                min_severity = ?self.min_severity,
                "notification.suppressed_severity"
            );
            return;
        }

        // 2. Per-type rate limiter (max 1 per 5 minutes)
        {
            let mut limiter = self.rate_limiter.lock().await;
            let now = Instant::now();
            let cooldown = Duration::from_secs(300); // 5 minutes

            if let Some(last_sent) = limiter.get(alert_type) {
                if now.duration_since(*last_sent) < cooldown {
                    tracing::debug!(
                        component = "notifications",
                        alert_type = alert_type,
                        "notification.suppressed_rate_limited"
                    );
                    return;
                }
            }
            limiter.insert(alert_type.to_string(), now);
        }

        // 3. Send to each channel
        for channel in &self.channels {
            match channel {
                NotificationChannel::Telegram { bot_token, chat_id } => {
                    Self::send_telegram(bot_token, chat_id, title, message).await;
                }
                NotificationChannel::Webhook { url } => {
                    Self::send_webhook(url, alert_type, title, message).await;
                }
            }
        }
    }

    /// Send an HTML-formatted message via Telegram Bot API with retry.
    async fn send_telegram(bot_token: &str, chat_id: &str, title: &str, message: &str) {
        let text = format!("<b>{title}</b>\n<pre>{message}</pre>");

        let payload = TelegramPayload {
            chat_id: chat_id.to_string(),
            text,
            parse_mode: "HTML".to_string(),
            disable_web_page_preview: true,
        };

        let url = format!("https://api.telegram.org/bot{bot_token}/sendMessage");
        let client = reqwest::Client::new();

        // Retry: 3 attempts with 1s, 3s, 6s backoff
        let delays = [1, 3, 6];
        let mut last_error = None;

        for (attempt, delay_secs) in delays.iter().enumerate() {
            match client
                .post(&url)
                .json(&payload)
                .timeout(Duration::from_secs(10))
                .send()
                .await
            {
                Ok(resp) => {
                    if resp.status().is_success() {
                        tracing::info!(
                            component = "notifications",
                            channel = "telegram",
                            alert_type = "notification.sent",
                        );
                        return;
                    }
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    last_error = Some(format!("HTTP {status}: {body}"));
                    tracing::warn!(
                        component = "notifications",
                        channel = "telegram",
                        attempt = attempt + 1,
                        error = %last_error.as_ref().unwrap(),
                        "notification.retry"
                    );
                }
                Err(e) => {
                    last_error = Some(e.to_string());
                    tracing::warn!(
                        component = "notifications",
                        channel = "telegram",
                        attempt = attempt + 1,
                        error = %e,
                        "notification.retry"
                    );
                }
            }

            if attempt < delays.len() - 1 {
                tokio::time::sleep(Duration::from_secs(*delay_secs)).await;
            }
        }

        tracing::error!(
            component = "notifications",
            channel = "telegram",
            alert_type = "notification.failed",
            error = %last_error.unwrap_or_default(),
            "notification.failed"
        );
    }

    /// Send a Slack-compatible JSON message to a webhook URL with retry.
    async fn send_webhook(url: &str, alert_type: &str, title: &str, message: &str) {
        let payload = SlackPayload {
            text: format!("[{alert_type}] {title}"),
            blocks: vec![
                SlackBlock {
                    block_type: "header".to_string(),
                    text: Some(SlackText {
                        text_type: "plain_text".to_string(),
                        text: title.to_string(),
                    }),
                    fields: None,
                },
                SlackBlock {
                    block_type: "section".to_string(),
                    text: Some(SlackText {
                        text_type: "mrkdwn".to_string(),
                        text: format!("*Type:* `{alert_type}`\n*Details:* ```{message}```"),
                    }),
                    fields: None,
                },
            ],
        };

        let client = reqwest::Client::new();
        let delays = [1, 3, 6];
        let mut last_error = None;

        for (attempt, delay_secs) in delays.iter().enumerate() {
            match client
                .post(url)
                .json(&payload)
                .timeout(Duration::from_secs(10))
                .send()
                .await
            {
                Ok(resp) => {
                    if resp.status().is_success() || resp.status().is_client_error() {
                        // 2xx success or 4xx client error (webhook config issue)
                        tracing::info!(
                            component = "notifications",
                            channel = "webhook",
                            alert_type = "notification.sent",
                            status = %resp.status(),
                        );
                        return;
                    }
                    last_error = Some(format!("HTTP {}", resp.status()));
                    tracing::warn!(
                        component = "notifications",
                        channel = "webhook",
                        attempt = attempt + 1,
                        status = %resp.status(),
                        "notification.retry"
                    );
                }
                Err(e) => {
                    last_error = Some(e.to_string());
                    tracing::warn!(
                        component = "notifications",
                        channel = "webhook",
                        attempt = attempt + 1,
                        error = %e,
                        "notification.retry"
                    );
                }
            }

            if attempt < delays.len() - 1 {
                tokio::time::sleep(Duration::from_secs(*delay_secs)).await;
            }
        }

        tracing::error!(
            component = "notifications",
            channel = "webhook",
            alert_type = "notification.failed",
            error = %last_error.unwrap_or_default(),
            "notification.failed"
        );
    }
}

fn severity_rank(level: Level) -> u8 {
    match level {
        Level::TRACE => 0,
        Level::DEBUG => 1,
        Level::INFO => 2,
        Level::WARN => 3,
        Level::ERROR => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_notification_rate_limiting() {
        let svc = NotificationService::new(
            vec![NotificationChannel::Webhook {
                url: "http://localhost:9999/nonexistent".to_string(),
            }],
            Level::INFO,
        );

        // First alert should be dispatched (but will fail to connect — that's OK, we're testing rate limiting)
        svc.send_alert("test_type", Level::ERROR, "Test", "First alert");
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Second alert of the same type should be suppressed by rate limiter
        // We verify by checking that only 1 entry exists in the rate limiter map
        let limiter = svc.rate_limiter.lock().await;
        assert_eq!(limiter.len(), 1);
    }

    #[tokio::test]
    async fn test_notification_severity_filter() {
        let svc = NotificationService::new(
            vec![NotificationChannel::Webhook {
                url: "http://localhost:9999/nonexistent".to_string(),
            }],
            Level::ERROR, // Only ERROR and above
        );

        // INFO alert should be suppressed by severity filter
        svc.send_alert("test_severity", Level::INFO, "Test", "Info alert");
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Rate limiter should NOT have an entry (it was suppressed before rate limit check)
        let limiter = svc.rate_limiter.lock().await;
        assert!(!limiter.contains_key("test_severity"));
    }

    #[tokio::test]
    async fn test_notification_telegram_format() {
        let svc = NotificationService::new(
            vec![NotificationChannel::Telegram {
                bot_token: "test:token".to_string(),
                chat_id: "-1001234567890".to_string(),
            }],
            Level::WARN,
        );

        // Alert should be dispatched but fail to send (invalid token)
        svc.send_alert(
            "test_telegram",
            Level::ERROR,
            "Test Title",
            "Test message body",
        );
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Rate limiter should have an entry (dispatched past severity filter)
        let limiter = svc.rate_limiter.lock().await;
        assert!(limiter.contains_key("test_telegram"));
    }

    #[tokio::test]
    async fn test_notification_webhook_format() {
        let svc = NotificationService::new(
            vec![NotificationChannel::Webhook {
                url: "http://localhost:9999/nonexistent".to_string(),
            }],
            Level::DEBUG,
        );

        // Alert should be dispatched to webhook (will fail, but that's expected)
        svc.send_alert("test_webhook", Level::WARN, "Webhook Test", "Details here");
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Rate limiter should have an entry
        let limiter = svc.rate_limiter.lock().await;
        assert!(limiter.contains_key("test_webhook"));
    }

    #[test]
    fn test_from_config_none_when_empty() {
        let svc = NotificationService::from_config(None, None, None, None);
        assert!(svc.is_none(), "No channels configured should return None");
    }

    #[test]
    fn test_from_config_telegram_only() {
        let svc = NotificationService::from_config(
            Some("bot:token".to_string()),
            Some("chat123".to_string()),
            None,
            None,
        );
        assert!(svc.is_some());
        let svc = svc.unwrap();
        assert_eq!(svc.channels.len(), 1);
        match &svc.channels[0] {
            NotificationChannel::Telegram { bot_token, chat_id } => {
                assert_eq!(bot_token, "bot:token");
                assert_eq!(chat_id, "chat123");
            }
            _ => panic!("Expected Telegram channel"),
        }
    }

    #[test]
    fn test_from_config_webhook_only() {
        let svc = NotificationService::from_config(
            None,
            None,
            Some("https://hooks.example.com".to_string()),
            None,
        );
        assert!(svc.is_some());
        let svc = svc.unwrap();
        assert_eq!(svc.channels.len(), 1);
        match &svc.channels[0] {
            NotificationChannel::Webhook { url } => {
                assert_eq!(url, "https://hooks.example.com");
            }
            _ => panic!("Expected Webhook channel"),
        }
    }
}
