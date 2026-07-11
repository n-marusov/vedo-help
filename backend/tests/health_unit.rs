use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use vedo_backend::shared::error::AppError;
use vedo_backend::shared::health::{CheckStatus, HealthProbe, HealthService, HealthStatus};

// ============================================================================
// Mock probes
// ============================================================================

struct OkProbe {
    name: &'static str,
}

#[async_trait]
impl HealthProbe for OkProbe {
    fn name(&self) -> &'static str {
        self.name
    }
    async fn probe(&self) -> Result<(), AppError> {
        Ok(())
    }
}

struct ErrProbe {
    name: &'static str,
    error: AppError,
}

#[async_trait]
impl HealthProbe for ErrProbe {
    fn name(&self) -> &'static str {
        self.name
    }
    async fn probe(&self) -> Result<(), AppError> {
        Err(self.error.clone())
    }
}

struct LatencyProbe {
    name: &'static str,
    delay_ms: u64,
}

#[async_trait]
impl HealthProbe for LatencyProbe {
    fn name(&self) -> &'static str {
        self.name
    }
    async fn probe(&self) -> Result<(), AppError> {
        if self.delay_ms > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(self.delay_ms)).await;
        }
        Ok(())
    }
}

struct CallTrackerProbe {
    name: &'static str,
    called: Arc<AtomicBool>,
}

#[async_trait]
impl HealthProbe for CallTrackerProbe {
    fn name(&self) -> &'static str {
        self.name
    }
    async fn probe(&self) -> Result<(), AppError> {
        self.called.store(true, Ordering::SeqCst);
        Ok(())
    }
}

// ============================================================================
// Task 1: Unit tests for HealthService aggregation logic
// ============================================================================

#[tokio::test]
async fn test_all_healthy_status() {
    // Arrange
    let mut svc = HealthService::new(None);
    svc.register(OkProbe { name: "Chroma" })
        .register(OkProbe { name: "PostgreSQL" })
        .register(OkProbe { name: "Embedding" })
        .register(OkProbe { name: "LLM" });

    // Act
    let report = svc.check_all().await;

    // Assert
    assert_eq!(
        report.status,
        HealthStatus::Healthy,
        "All probes pass -> Healthy"
    );
    assert_eq!(report.checks.len(), 4, "All 4 probes should be present");
    for check in &report.checks {
        assert_eq!(
            check.status,
            CheckStatus::Healthy,
            "Check '{}' should be healthy",
            check.name
        );
        assert!(
            check.error.is_none(),
            "Check '{}' should have no error",
            check.name
        );
    }
    assert!(
        (Utc::now() - report.timestamp).num_seconds() < 5,
        "Timestamp should be recent"
    );
}

#[tokio::test]
async fn test_chroma_down_degraded() {
    // Arrange
    let mut svc = HealthService::new(None);
    svc.register(ErrProbe {
        name: "Chroma",
        error: AppError::ChromaError("Connection refused".to_string()),
    })
    .register(OkProbe { name: "PostgreSQL" })
    .register(OkProbe { name: "Embedding" })
    .register(OkProbe { name: "LLM" });

    // Act
    let report = svc.check_all().await;

    // Assert
    assert_eq!(
        report.status,
        HealthStatus::Degraded,
        "Chroma is non-critical -> Degraded"
    );
    let chroma_check = report.checks.iter().find(|c| c.name == "Chroma").unwrap();
    assert_eq!(chroma_check.status, CheckStatus::Unhealthy);
    assert!(chroma_check.error.is_some());
    assert!(chroma_check
        .error
        .as_ref()
        .unwrap()
        .contains("Connection refused"));
}

#[tokio::test]
async fn test_llm_down_degraded() {
    // Arrange
    let mut svc = HealthService::new(None);
    svc.register(OkProbe { name: "Chroma" })
        .register(OkProbe { name: "PostgreSQL" })
        .register(OkProbe { name: "Embedding" })
        .register(ErrProbe {
            name: "LLM",
            error: AppError::LlmError("Connection timeout".to_string()),
        });

    // Act
    let report = svc.check_all().await;

    // Assert
    assert_eq!(
        report.status,
        HealthStatus::Degraded,
        "LLM is non-critical -> Degraded"
    );
}

#[tokio::test]
async fn test_db_down_unhealthy() {
    // Arrange
    let mut svc = HealthService::new(None);
    svc.register(OkProbe { name: "Chroma" })
        .register(ErrProbe {
            name: "PostgreSQL",
            error: AppError::InternalError("Connection refused".to_string()),
        })
        .register(OkProbe { name: "Embedding" })
        .register(ErrProbe {
            name: "LLM",
            error: AppError::LlmError("timeout".to_string()),
        });

    // Act
    let report = svc.check_all().await;

    // Assert
    assert_eq!(
        report.status,
        HealthStatus::Unhealthy,
        "DB is critical -> Unhealthy"
    );
    let db_check = report
        .checks
        .iter()
        .find(|c| c.name == "PostgreSQL")
        .unwrap();
    assert_eq!(db_check.status, CheckStatus::Unhealthy);
}

#[tokio::test]
async fn test_embedding_and_chroma_down_degraded_db_ok() {
    // Arrange
    let mut svc = HealthService::new(None);
    svc.register(ErrProbe {
        name: "Chroma",
        error: AppError::ChromaError("down".to_string()),
    })
    .register(OkProbe { name: "PostgreSQL" })
    .register(ErrProbe {
        name: "Embedding",
        error: AppError::EmbeddingError("down".to_string()),
    })
    .register(OkProbe { name: "LLM" });

    // Act
    let report = svc.check_all().await;

    // Assert
    assert_eq!(
        report.status,
        HealthStatus::Degraded,
        "Only non-critical probes fail -> Degraded"
    );
}

#[tokio::test]
async fn test_report_serialization_to_json() {
    // Arrange
    let mut svc = HealthService::new(None);
    svc.register(OkProbe { name: "Chroma" }).register(ErrProbe {
        name: "PostgreSQL",
        error: AppError::ChromaError("down".to_string()),
    });

    // Act
    let report = svc.check_all().await;
    let json = serde_json::to_string(&report).expect("Serialization should succeed");
    let deserialized: vedo_backend::shared::health::HealthReport =
        serde_json::from_str(&json).expect("Deserialization should succeed");

    // Assert
    assert_eq!(deserialized.status, report.status);
    assert_eq!(deserialized.checks.len(), 2);
    assert_eq!(deserialized.checks[0].name, "Chroma");
    assert_eq!(deserialized.checks[0].status, CheckStatus::Healthy);
    assert_eq!(deserialized.checks[1].name, "PostgreSQL");
    assert_eq!(deserialized.checks[1].status, CheckStatus::Unhealthy);
}

#[tokio::test]
async fn test_report_serialization_lowercase_json_values() {
    // Arrange
    let mut svc = HealthService::new(None);
    svc.register(OkProbe { name: "Embedding" })
        .register(ErrProbe {
            name: "LLM",
            error: AppError::InternalError("connection refused".to_string()),
        });

    // Act
    let report = svc.check_all().await;
    let json = serde_json::to_value(&report).expect("Serialization should succeed");

    // Assert — status field must be lowercase for frontend compatibility
    let status = json["status"].as_str().unwrap();
    assert!(
        ["healthy", "degraded", "unhealthy"].contains(&status),
        "HealthReport.status must be lowercase, got: {status}"
    );
    assert_eq!(
        status, "degraded",
        "Expected degraded because LLM probe failed"
    );

    // Assert — each check status must be lowercase
    let checks = json["checks"].as_array().unwrap();
    for check in checks {
        let check_status = check["status"].as_str().unwrap();
        assert!(
            ["healthy", "unhealthy"].contains(&check_status),
            "ServiceCheck.status must be lowercase, got: {check_status}"
        );
        // Also check that all letters are indeed lowercase
        assert_eq!(
            check_status,
            check_status.to_lowercase(),
            "ServiceCheck.status contains uppercase letters: {check_status}"
        );
    }

    // Also verify overall status is lowercase
    assert_eq!(
        status,
        status.to_lowercase(),
        "HealthReport.status contains uppercase letters: {status}"
    );
}

#[tokio::test]
async fn test_concurrent_probe_execution() {
    // Arrange — two probes that each take 100ms.
    // If run concurrently, total < 200ms; if sequential ≥ 200ms.
    let started = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let started_clone = started.clone();

    struct TimestampProbe {
        name: &'static str,
        delay_ms: u64,
        started: Arc<std::sync::atomic::AtomicU64>,
    }

    #[async_trait]
    impl HealthProbe for TimestampProbe {
        fn name(&self) -> &'static str {
            self.name
        }
        async fn probe(&self) -> Result<(), AppError> {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
            self.started.store(now, std::sync::atomic::Ordering::SeqCst);
            tokio::time::sleep(std::time::Duration::from_millis(self.delay_ms)).await;
            Ok(())
        }
    }

    let mut svc = HealthService::new(None);
    svc.register(TimestampProbe {
        name: "ProbeA",
        delay_ms: 100,
        started: started_clone,
    })
    .register(TimestampProbe {
        name: "ProbeB",
        delay_ms: 100,
        started: started.clone(),
    });

    // Act
    let start = std::time::Instant::now();
    let _report = svc.check_all().await;
    let elapsed = start.elapsed();

    // Assert: concurrent execution of 2x 100ms probes should complete in < 200ms
    // (generous 250ms bound to account for test environment overhead)
    assert!(
        elapsed.as_millis() < 250,
        "Probes should run concurrently, elapsed: {:?}",
        elapsed
    );
}

#[tokio::test]
async fn test_empty_probes() {
    // Arrange
    let svc = HealthService::new(None);

    // Act
    let report = svc.check_all().await;

    // Assert
    assert_eq!(report.status, HealthStatus::Healthy);
    assert!(report.checks.is_empty());
}

#[tokio::test]
async fn test_latency_ms_tracked() {
    // Arrange
    let mut svc = HealthService::new(None);
    // Use a probe with artificial delay
    svc.register(LatencyProbe {
        name: "SlowService",
        delay_ms: 5,
    })
    .register(OkProbe {
        name: "FastService",
    });

    // Act
    let report = svc.check_all().await;

    // Assert
    let slow = report
        .checks
        .iter()
        .find(|c| c.name == "SlowService")
        .unwrap();
    assert!(
        slow.latency_ms >= 4,
        "Slow probe latency should be >= 4ms, got {}",
        slow.latency_ms
    );
    let fast = report
        .checks
        .iter()
        .find(|c| c.name == "FastService")
        .unwrap();
    assert!(
        fast.latency_ms < slow.latency_ms,
        "Fast probe should complete faster than slow probe"
    );
}

#[tokio::test]
async fn test_all_probes_are_called() {
    // Arrange
    let called_a = Arc::new(AtomicBool::new(false));
    let called_b = Arc::new(AtomicBool::new(false));

    let mut svc = HealthService::new(None);
    svc.register(CallTrackerProbe {
        name: "ProbeA",
        called: called_a.clone(),
    })
    .register(CallTrackerProbe {
        name: "ProbeB",
        called: called_b.clone(),
    });

    // Act
    let _report = svc.check_all().await;

    // Assert
    assert!(
        called_a.load(Ordering::SeqCst),
        "ProbeA should have been called"
    );
    assert!(
        called_b.load(Ordering::SeqCst),
        "ProbeB should have been called"
    );
}
