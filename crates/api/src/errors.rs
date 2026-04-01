//! API error types.

use thiserror::Error;

/// Errors returned by the Anthropic API client.
#[derive(Debug, Error)]
pub enum ApiError {
    /// Authentication failed (invalid key, expired token).
    #[error("Authentication failed: {0}")]
    AuthError(String),

    /// Request was rate-limited.
    #[error("Rate limited: retry after {retry_after_ms}ms")]
    RateLimited {
        /// Milliseconds to wait before retrying.
        retry_after_ms: u64,
    },

    /// API is temporarily overloaded.
    #[error("Overloaded: {0}")]
    Overloaded(String),

    /// Request was malformed.
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// Network-level error.
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    /// Error during SSE stream processing.
    #[error("Streaming error: {0}")]
    StreamError(String),

    /// Server returned an error status.
    #[error("API error {status}: {message}")]
    ServerError {
        /// HTTP status code.
        status: u16,
        /// Error message from the server.
        message: String,
    },
}

impl ApiError {
    /// Whether this error is retryable.
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::RateLimited { .. }
                | Self::Overloaded(_)
                | Self::NetworkError(_)
                | Self::ServerError { status: 500..=599, .. }
        )
    }

    /// Create an `ApiError` from an HTTP status code and response body.
    #[must_use]
    pub fn from_status(status: u16, body: &str) -> Self {
        match status {
            401 => Self::AuthError(body.to_string()),
            429 => {
                // Try to extract retry-after from body
                let retry_after_ms = serde_json::from_str::<serde_json::Value>(body)
                    .ok()
                    .and_then(|v| v["error"]["retry_after"].as_f64())
                    .map(|s| (s * 1000.0) as u64)
                    .unwrap_or(1000);
                Self::RateLimited { retry_after_ms }
            }
            529 => Self::Overloaded(body.to_string()),
            400 => Self::InvalidRequest(body.to_string()),
            _ => Self::ServerError {
                status,
                message: body.to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_retryable_rate_limited() {
        let err = ApiError::RateLimited {
            retry_after_ms: 500,
        };
        assert!(err.is_retryable());
    }

    #[test]
    fn test_is_retryable_overloaded() {
        let err = ApiError::Overloaded("busy".into());
        assert!(err.is_retryable());
    }

    #[test]
    fn test_is_retryable_server_error() {
        let err = ApiError::ServerError {
            status: 503,
            message: "down".into(),
        };
        assert!(err.is_retryable());
    }

    #[test]
    fn test_not_retryable_auth() {
        let err = ApiError::AuthError("bad key".into());
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_not_retryable_invalid_request() {
        let err = ApiError::InvalidRequest("bad".into());
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_from_status_401() {
        let err = ApiError::from_status(401, "unauthorized");
        assert!(matches!(err, ApiError::AuthError(_)));
    }

    #[test]
    fn test_from_status_429() {
        let err = ApiError::from_status(429, "{}");
        assert!(matches!(err, ApiError::RateLimited { .. }));
    }

    #[test]
    fn test_from_status_529() {
        let err = ApiError::from_status(529, "overloaded");
        assert!(matches!(err, ApiError::Overloaded(_)));
    }

    #[test]
    fn test_from_status_400() {
        let err = ApiError::from_status(400, "bad request");
        assert!(matches!(err, ApiError::InvalidRequest(_)));
    }

    #[test]
    fn test_from_status_500() {
        let err = ApiError::from_status(500, "internal error");
        assert!(matches!(err, ApiError::ServerError { status: 500, .. }));
    }

    #[test]
    fn test_error_display() {
        let err = ApiError::RateLimited {
            retry_after_ms: 1000,
        };
        assert_eq!(err.to_string(), "Rate limited: retry after 1000ms");
    }

    #[test]
    fn test_from_status_429_with_retry_after() {
        let body = r#"{"error":{"retry_after":2.5}}"#;
        let err = ApiError::from_status(429, body);
        if let ApiError::RateLimited { retry_after_ms } = err {
            assert_eq!(retry_after_ms, 2500);
        } else {
            panic!("expected RateLimited");
        }
    }
}
