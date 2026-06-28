use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use tokio::time::timeout;

use crate::shared::error::AppError;

/// Trait for health probe implementations.
///
/// Each downstream dependency (Chroma, Embedding, LLM, DB) implements this
/// trait so that `HealthService::check_all()` can probe them uniformly.
#[async_trait]
pub trait HealthProbe: Send + Sync {
    /// Human-readable name for this probe (e.g. `"Chroma"`, `"PostgreSQL"`).
    fn name(&self) -> &'static str;

    /// Run the health probe against the downstream dependency.
    ///
    /// Returns `Ok(())` when the dependency is reachable and healthy,
    /// `Err(AppError)` when it is not.
    async fn probe(&self) -> Result<(), AppError>;
}

/// Aggregated health status for the entire service.
///
/// Ordering: `Healthy < Degraded < Unhealthy`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum HealthStatus {
    /// All probes passed — the service is fully operational.
    Healthy,
    /// Non-critical probes failed — the service is operational but degraded.
    Degraded,
    /// Critical probes failed — the service is not fully operational.
    Unhealthy,
}

impl fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "healthy"),
            HealthStatus::Degraded => write!(f, "degraded"),
            HealthStatus::Unhealthy => write!(f, "unhealthy"),
        }
    }
}

/// Status of a single service check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckStatus {
    /// The dependency responded successfully.
    Healthy,
    /// The dependency responded but with degraded performance.
    Degraded,
    /// The dependency is unreachable or returned an error.
    Unhealthy,
}

/// Result of a single downstream dependency health check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceCheck {
    /// Human-readable service name (e.g. `"Chroma"`, `"PostgreSQL"`).
    pub name: String,
    /// Health status of this specific check.
    pub status: CheckStatus,
    /// Round-trip latency in milliseconds.
    pub latency_ms: u64,
    /// Optional error message when the check failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Full health report returned from `GET /api/health/deep`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    /// Aggregated health status across all checks.
    pub status: HealthStatus,
    /// Per-service check results.
    pub checks: Vec<ServiceCheck>,
    /// ISO 8601 timestamp when the check was initiated.
    pub timestamp: DateTime<Utc>,
}

/// Aggregate health-checking service.
///
/// Probes all registered downstream dependencies concurrently and produces an
/// aggregated `HealthReport` with per-service latencies and error details.
#[derive(Clone)]
pub struct HealthService {
    probes: Vec<ProbeEntry>,
}

/// Internal pairing of a probe instance with an explicit name.
///
/// We store the name twice (once here, once on the trait) to keep the API
/// ergonomic — callers only supply the probe and we cache the canonical name.
#[derive(Clone)]
struct ProbeEntry {
    name: &'static str,
    probe: Arc<dyn HealthProbe>,
}

impl ProbeEntry {
    fn new(probe: impl HealthProbe + 'static) -> Self {
        let name = probe.name();
        Self {
            name,
            probe: Arc::new(probe),
        }
    }
}

impl HealthService {
    /// Create a new health service with the given probes.
    pub fn new() -> Self {
        Self { probes: Vec::new() }
    }

    /// Register a probe.
    pub fn register(&mut self, probe: impl HealthProbe + 'static) -> &mut Self {
        self.probes.push(ProbeEntry::new(probe));
        self
    }

    /// Register a probe from an `Arc<dyn HealthProbe>`.
    pub fn register_arc(&mut self, name: &'static str, probe: Arc<dyn HealthProbe>) -> &mut Self {
        self.probes.push(ProbeEntry { name, probe });
        self
    }

    /// Probe all registered dependencies concurrently and return an aggregated report.
    pub async fn check_all(&self) -> HealthReport {
        let timestamp = Utc::now();

        tracing::debug!(
            component = "health",
            probe_count = self.probes.len(),
            "check_all.start"
        );

        let tasks: Vec<_> = self
            .probes
            .iter()
            .map(|entry| {
                let probe = entry.probe.clone();
                let name = entry.name;
                async move {
                    let probe_start = std::time::Instant::now();
                    let result = timeout(Duration::from_secs(10), probe.probe()).await;
                    let latency_ms = probe_start.elapsed().as_millis() as u64;

                    match result {
                        Ok(Ok(())) => {
                            tracing::debug!(
                                component = "health",
                                probe = name,
                                latency_ms = latency_ms,
                                "check_all.probe_ok"
                            );
                            ServiceCheck {
                                name: name.to_string(),
                                status: CheckStatus::Healthy,
                                latency_ms,
                                error: None,
                            }
                        }
                        Ok(Err(e)) => {
                            tracing::warn!(
                                component = "health",
                                probe = name,
                                latency_ms = latency_ms,
                                error = %e,
                                "check_all.probe_error"
                            );
                            ServiceCheck {
                                name: name.to_string(),
                                status: CheckStatus::Unhealthy,
                                latency_ms,
                                error: Some(e.to_string()),
                            }
                        }
                        Err(_elapsed) => {
                            tracing::warn!(
                                component = "health",
                                probe = name,
                                latency_ms = latency_ms,
                                "check_all.probe_timeout"
                            );
                            ServiceCheck {
                                name: name.to_string(),
                                status: CheckStatus::Unhealthy,
                                latency_ms,
                                error: Some("Timeout after 10 seconds".to_string()),
                            }
                        }
                    }
                }
            })
            .collect();

        let checks = join_all(tasks).await;

        // Aggregate: if any critical probe (DB) is unhealthy → Unhealthy.
        // If any non-critical probe is unhealthy → Degraded.
        // All healthy → Healthy.
        let status = if checks
            .iter()
            .any(|c| c.name == "PostgreSQL" && c.status == CheckStatus::Unhealthy)
        {
            tracing::warn!(component = "health", "check_all.unhealthy_critical");
            HealthStatus::Unhealthy
        } else if checks.iter().any(|c| c.status == CheckStatus::Unhealthy) {
            tracing::warn!(component = "health", "check_all.degraded");
            HealthStatus::Degraded
        } else {
            tracing::info!(component = "health", "check_all.healthy");
            HealthStatus::Healthy
        };

        HealthReport {
            status,
            checks,
            timestamp,
        }
    }
}

impl Default for HealthService {
    fn default() -> Self {
        Self::new()
    }
}
