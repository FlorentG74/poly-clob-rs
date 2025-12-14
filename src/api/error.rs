//! API error types for structured error handling.
//!
//! This module provides error types that give callers control over retry strategies
//! and error handling decisions.

use std::time::Duration;

/// API error with structured information for handling different failure modes.
#[derive(Debug)]
pub enum ApiError {
    /// Rate limited by the server. Contains suggested retry delay.
    RateLimited {
        /// Suggested delay before retrying
        retry_after: Duration,
    },
    /// Authentication failed (401)
    Unauthorized,
    /// Access denied (403)
    Forbidden,
    /// Resource not found (404)
    NotFound {
        /// The URL that was not found
        url: String,
    },
    /// Bad request (400) with error details
    BadRequest {
        /// Error message from the server
        message: String,
    },
    /// Server error (5xx) - may be retryable
    ServerError {
        /// HTTP status code
        status: u16,
        /// Whether this error is likely transient and retryable
        retryable: bool,
    },
    /// Network or other transport error
    Transport {
        /// Error message
        message: String,
    },
    /// Other unexpected error
    Other {
        /// HTTP status code
        status: u16,
        /// Error message
        message: String,
    },
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::RateLimited { retry_after } => {
                write!(f, "rate limited, retry after {:?}", retry_after)
            }
            ApiError::Unauthorized => write!(f, "unauthorized"),
            ApiError::Forbidden => write!(f, "forbidden"),
            ApiError::NotFound { url } => write!(f, "not found: {}", url),
            ApiError::BadRequest { message } => write!(f, "bad request: {}", message),
            ApiError::ServerError { status, retryable } => {
                write!(
                    f,
                    "server error ({}), retryable: {}",
                    status, retryable
                )
            }
            ApiError::Transport { message } => write!(f, "transport error: {}", message),
            ApiError::Other { status, message } => {
                write!(f, "HTTP {}: {}", status, message)
            }
        }
    }
}

impl std::error::Error for ApiError {}

impl ApiError {
    /// Returns true if the error is likely transient and the request could be retried.
    pub fn is_retryable(&self) -> bool {
        match self {
            ApiError::RateLimited { .. } => true,
            ApiError::ServerError { retryable, .. } => *retryable,
            ApiError::Transport { .. } => true,
            _ => false,
        }
    }

    /// Returns the suggested retry delay, if applicable.
    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            ApiError::RateLimited { retry_after } => Some(*retry_after),
            ApiError::ServerError { retryable: true, .. } => Some(Duration::from_secs(2)),
            ApiError::Transport { .. } => Some(Duration::from_secs(1)),
            _ => None,
        }
    }
}
