//! Comprehensive error handling for the SDK

use std::fmt;
use thiserror::Error;

/// Custom result type for SDK operations
pub type Result<T> = std::result::Result<T, SdkError>;

/// Comprehensive error types for the SDK
#[derive(Error, Debug)]
pub enum SdkError {
    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// HTTP request failed
    #[error("HTTP request failed: {0}")]
    HttpError(String),

    /// Request validation failed
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Database error
    #[error("Database error: {0}")]
    DatabaseError(String),

    /// Authentication failed
    #[error("Authentication failed: {0}")]
    AuthenticationError(String),

    /// Authorization failed
    #[error("Authorization failed: {0}")]
    AuthorizationError(String),

    /// Rate limit exceeded
    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    /// Resource not found
    #[error("Resource not found: {0}")]
    NotFound(String),

    /// Conflict error
    #[error("Conflict: {0}")]
    Conflict(String),

    /// Timeout error
    #[error("Request timeout after {ms}ms")]
    Timeout { ms: u64 },

    /// Retry exhausted
    #[error("Retry exhausted after {attempts} attempts: {reason}")]
    RetryExhausted { attempts: u32, reason: String },

    /// Cache error
    #[error("Cache error: {0}")]
    CacheError(String),

    /// Generic error
    #[error("Error: {0}")]
    Other(String),
}

impl SdkError {
    /// Create a new HTTP error
    pub fn http(msg: impl Into<String>) -> Self {
        SdkError::HttpError(msg.into())
    }

    /// Create a new validation error
    pub fn validation(msg: impl Into<String>) -> Self {
        SdkError::ValidationError(msg.into())
    }

    /// Create a new configuration error
    pub fn config(msg: impl Into<String>) -> Self {
        SdkError::ConfigError(msg.into())
    }

    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            SdkError::Timeout { .. }
                | SdkError::RateLimitExceeded(_)
                | SdkError::HttpError(_)
        )
    }

    /// Check if error is authentication-related
    pub fn is_auth_error(&self) -> bool {
        matches!(
            self,
            SdkError::AuthenticationError(_) | SdkError::AuthorizationError(_)
        )
    }
}

/// Error context for detailed error reporting
#[derive(Debug)]
pub struct ErrorContext {
    pub request_id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub source_error: String,
}

impl fmt::Display for ErrorContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] {} at {}",
            self.request_id, self.source_error, self.timestamp
        )
    }
}
