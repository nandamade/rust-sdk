//! Utility functions for the SDK

use crate::error::{Result, SdkError};
use chrono::Utc;
use std::fmt;
use std::time::Duration;

/// Generate trace ID
pub fn generate_trace_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Calculate exponential backoff
pub fn exponential_backoff(attempt: u32, base_ms: u64, max_ms: u64, multiplier: f64) -> Duration {
    let backoff_ms = (base_ms as f64 * multiplier.powi(attempt as i32)) as u64;
    let backoff_ms = backoff_ms.min(max_ms);

    // Add jitter (0-20% of backoff)
    let jitter_ms = (backoff_ms as f64 * 0.2 * rand::random::<f64>()) as u64;

    Duration::from_millis(backoff_ms + jitter_ms)
}

/// Rate limiter with metrics
pub struct RateLimiterMetrics {
    total_requests: std::sync::atomic::AtomicU64,
    rejected_requests: std::sync::atomic::AtomicU64,
}

impl RateLimiterMetrics {
    /// Create new metrics
    pub fn new() -> Self {
        Self {
            total_requests: std::sync::atomic::AtomicU64::new(0),
            rejected_requests: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Record request
    pub fn record_request(&self) {
        self.total_requests
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    /// Record rejection
    pub fn record_rejection(&self) {
        self.rejected_requests
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    /// Get total requests
    pub fn total_requests(&self) -> u64 {
        self.total_requests.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Get rejected requests
    pub fn rejected_requests(&self) -> u64 {
        self.rejected_requests
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Get acceptance rate
    pub fn acceptance_rate(&self) -> f64 {
        let total = self.total_requests();
        if total == 0 {
            100.0
        } else {
            ((total - self.rejected_requests()) as f64 / total as f64) * 100.0
        }
    }
}

impl Default for RateLimiterMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for RateLimiterMetrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RateLimiterMetrics")
            .field("total_requests", &self.total_requests())
            .field("rejected_requests", &self.rejected_requests())
            .field("acceptance_rate", &format!("{}%", self.acceptance_rate()))
            .finish()
    }
}

/// Retry policy builder
pub struct RetryPolicyBuilder {
    max_attempts: u32,
    initial_backoff_ms: u64,
    max_backoff_ms: u64,
    multiplier: f64,
    jitter: bool,
}

impl RetryPolicyBuilder {
    /// Create new retry policy builder
    pub fn new() -> Self {
        Self {
            max_attempts: 3,
            initial_backoff_ms: 100,
            max_backoff_ms: 30000,
            multiplier: 2.0,
            jitter: true,
        }
    }

    /// Set max attempts
    pub fn max_attempts(mut self, attempts: u32) -> Self {
        self.max_attempts = attempts;
        self
    }

    /// Set initial backoff
    pub fn initial_backoff_ms(mut self, ms: u64) -> Self {
        self.initial_backoff_ms = ms;
        self
    }

    /// Set max backoff
    pub fn max_backoff_ms(mut self, ms: u64) -> Self {
        self.max_backoff_ms = ms;
        self
    }

    /// Set multiplier
    pub fn multiplier(mut self, mult: f64) -> Self {
        self.multiplier = mult;
        self
    }

    /// Build retry policy
    pub fn build(self) -> RetryPolicy {
        RetryPolicy {
            max_attempts: self.max_attempts,
            initial_backoff_ms: self.initial_backoff_ms,
            max_backoff_ms: self.max_backoff_ms,
            multiplier: self.multiplier,
            jitter: self.jitter,
        }
    }
}

impl Default for RetryPolicyBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Retry policy
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub initial_backoff_ms: u64,
    pub max_backoff_ms: u64,
    pub multiplier: f64,
    pub jitter: bool,
}

impl RetryPolicy {
    /// Calculate backoff for attempt
    pub fn backoff(&self, attempt: u32) -> Duration {
        exponential_backoff(
            attempt,
            self.initial_backoff_ms,
            self.max_backoff_ms,
            self.multiplier,
        )
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        RetryPolicyBuilder::new().build()
    }
}

/// Request signature generation for authentication
pub struct SignatureGenerator;

impl SignatureGenerator {
    /// Generate HMAC-SHA256 signature
    pub fn generate_signature(secret: &str, data: &str) -> String {
        let signature = hmac_sha256::HMAC::mac(data.as_bytes(), secret.as_bytes());
        hex::encode(signature)
    }

    /// Verify signature
    pub fn verify_signature(secret: &str, data: &str, signature: &str) -> Result<()> {
        let calculated = Self::generate_signature(secret, data);

        if calculated == signature {
            Ok(())
        } else {
            Err(SdkError::AuthenticationError("Signature verification failed".to_string()))
        }
    }
}

/// Time utilities
pub struct TimeUtils;

impl TimeUtils {
    /// Get current UTC timestamp
    pub fn now_utc_secs() -> u64 {
        Utc::now().timestamp() as u64
    }

    /// Convert seconds to milliseconds
    pub fn secs_to_millis(secs: u64) -> u64 {
        secs * 1000
    }

    /// Convert milliseconds to seconds
    pub fn millis_to_secs(millis: u64) -> u64 {
        millis / 1000
    }
}

/// Retry helper
pub struct RetryHelper;

impl RetryHelper {
    /// Retry with exponential backoff
    pub async fn retry_with_backoff<F, T, Fut>(
        mut f: F,
        policy: &RetryPolicy,
    ) -> Result<T>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut attempt = 0;

        loop {
            match f().await {
                Ok(result) => return Ok(result),
                Err(e) if attempt < policy.max_attempts - 1 && e.is_retryable() => {
                    attempt += 1;
                    let backoff = policy.backoff(attempt);
                    tracing::warn!(
                        attempt = attempt,
                        backoff_ms = backoff.as_millis(),
                        error = %e,
                        "Retrying operation"
                    );
                    tokio::time::sleep(backoff).await;
                }
                Err(e) => {
                    return Err(SdkError::RetryExhausted {
                        attempts: attempt + 1,
                        reason: e.to_string(),
                    })
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exponential_backoff() {
        let backoff = exponential_backoff(0, 100, 30000, 2.0);
        assert!(backoff.as_millis() >= 100);

        let backoff = exponential_backoff(5, 100, 30000, 2.0);
        assert!(backoff.as_millis() <= 30000);
    }

    #[test]
    fn test_rate_limiter_metrics() {
        let metrics = RateLimiterMetrics::new();
        metrics.record_request();
        metrics.record_request();

        assert_eq!(metrics.total_requests(), 2);
        assert_eq!(metrics.rejected_requests(), 0);
        assert_eq!(metrics.acceptance_rate(), 100.0);
    }

    #[test]
    fn test_signature_generation() {
        let secret = "my-secret";
        let data = "test-data";
        let sig = SignatureGenerator::generate_signature(secret, data);

        assert!(SignatureGenerator::verify_signature(secret, data, &sig).is_ok());
        assert!(SignatureGenerator::verify_signature(secret, data, "invalid").is_err());
    }

    #[test]
    fn test_time_utils() {
        let now_secs = TimeUtils::now_utc_secs();
        assert!(now_secs > 0);

        assert_eq!(TimeUtils::secs_to_millis(1), 1000);
        assert_eq!(TimeUtils::millis_to_secs(1000), 1);
    }
}
