//! Retry logic with exponential backoff and jitter.

use crate::errors::ApiError;
use std::time::Duration;
use tracing::warn;

/// Configuration for retry behavior.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts.
    pub max_retries: u32,
    /// Base delay between retries.
    pub base_delay: Duration,
    /// Maximum delay cap.
    pub max_delay: Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(30),
        }
    }
}

/// Execute a fallible async operation with retries.
///
/// Only retries on errors where [`ApiError::is_retryable`] returns `true`.
/// Uses exponential backoff with jitter. For rate-limit errors, respects
/// the `retry_after_ms` value if it exceeds the calculated backoff.
pub async fn with_retry<F, Fut, T>(config: &RetryConfig, f: F) -> Result<T, ApiError>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, ApiError>>,
{
    let mut attempt = 0u32;

    loop {
        match f().await {
            Ok(val) => return Ok(val),
            Err(e) => {
                if !e.is_retryable() || attempt >= config.max_retries {
                    return Err(e);
                }
                let backoff_ms = config
                    .base_delay
                    .as_millis()
                    .saturating_mul(1u128 << attempt.min(10));
                let capped_ms = backoff_ms.min(config.max_delay.as_millis()) as u64;

                // Add jitter: 50-150% of backoff
                let jitter_factor = 0.5 + rand::random::<f64>();
                let delay_ms = ((capped_ms as f64) * jitter_factor) as u64;

                // Respect rate-limit retry_after
                let delay_ms = if let ApiError::RateLimited { retry_after_ms } = &e {
                    delay_ms.max(*retry_after_ms)
                } else {
                    delay_ms
                };

                warn!(
                    attempt = attempt + 1,
                    max = config.max_retries,
                    delay_ms,
                    error = %e,
                    "Retrying after error"
                );

                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                attempt += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[tokio::test]
    async fn test_retry_succeeds_first_try() {
        let config = RetryConfig::default();
        let result = with_retry(&config, || async { Ok::<_, ApiError>(42) }).await;
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_retry_not_retryable_fails_immediately() {
        let attempts = AtomicU32::new(0);
        let config = RetryConfig::default();
        let result = with_retry(&config, || async {
            attempts.fetch_add(1, Ordering::SeqCst);
            Err::<(), _>(ApiError::InvalidRequest("bad".into()))
        })
        .await;
        assert!(result.is_err());
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_retry_succeeds_after_failures() {
        let attempts = AtomicU32::new(0);
        let config = RetryConfig {
            max_retries: 3,
            base_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(10),
        };
        let result = with_retry(&config, || async {
            let n = attempts.fetch_add(1, Ordering::SeqCst);
            if n < 2 {
                Err::<u32, _>(ApiError::Overloaded("busy".into()))
            } else {
                Ok(99)
            }
        })
        .await;
        assert_eq!(result.unwrap(), 99);
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_exhausted() {
        let attempts = AtomicU32::new(0);
        let config = RetryConfig {
            max_retries: 2,
            base_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(5),
        };
        let result = with_retry(&config, || async {
            attempts.fetch_add(1, Ordering::SeqCst);
            Err::<(), _>(ApiError::Overloaded("busy".into()))
        })
        .await;
        assert!(result.is_err());
        assert_eq!(attempts.load(Ordering::SeqCst), 3); // initial + 2 retries
    }

    #[test]
    fn test_retry_config_default() {
        let cfg = RetryConfig::default();
        assert_eq!(cfg.max_retries, 3);
        assert_eq!(cfg.base_delay, Duration::from_secs(1));
        assert_eq!(cfg.max_delay, Duration::from_secs(30));
    }
}
