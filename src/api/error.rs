//! Comprehensive error types for poly-clob-rs.
//!
//! This module provides granular, type-safe error handling with rich context.
//! Errors are designed to give callers maximum control over retry strategies,
//! logging, and error handling decisions.
//!
//! # Architecture
//!
//! The error system uses a nested enum architecture:
//! - [`ClobError`] - Top-level error type for all operations
//! - [`HttpError`] - Network and transport errors
//! - [`ApiError`] - HTTP API errors with full context
//! - [`SerializationError`] - JSON serialization/deserialization errors
//! - [`AuthError`] - Authentication and signing errors
//! - [`ValidationError`] - Client-side validation errors
//! - [`RelayerError`] - Relayer-specific errors
//!
//! # Examples
//!
//! ## Pattern matching for specific handling
//!
//! ```no_run
//! use poly_clob_rs::{ClobError, ApiError};
//!
//! async fn handle_request() -> Result<(), ClobError> {
//!     // ... make request
//!     Ok(())
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     match handle_request().await {
//!         Ok(_) => println!("Success"),
//!
//!         Err(ClobError::Api(ApiError::RateLimited { retry_after, .. })) => {
//!             eprintln!("Rate limited, retry after {:?}", retry_after);
//!         }
//!
//!         Err(ClobError::Api(ApiError::BadRequest { message, raw_response, .. })) => {
//!             eprintln!("Invalid request: {}", message);
//!             eprintln!("Raw response: {}", raw_response);
//!         }
//!
//!         Err(e) if e.is_retryable() => {
//!             eprintln!("Transient error, can retry: {}", e);
//!         }
//!
//!         Err(e) => eprintln!("Fatal error: {:#}", e),
//!     }
//! }
//! ```
//!
//! ## Using helper methods
//!
//! ```no_run
//! use poly_clob_rs::ClobError;
//! use std::time::Duration;
//!
//! async fn execute_with_retry<F, Fut, T>(f: F, max_retries: u32) -> Result<T, ClobError>
//! where
//!     F: Fn() -> Fut,
//!     Fut: std::future::Future<Output = Result<T, ClobError>>,
//! {
//!     let mut attempt = 0;
//!     loop {
//!         match f().await {
//!             Ok(result) => return Ok(result),
//!             Err(e) if e.is_retryable() && attempt < max_retries => {
//!                 attempt += 1;
//!                 let delay = e.retry_after().unwrap_or(Duration::from_secs(1));
//!                 tokio::time::sleep(delay).await;
//!             }
//!             Err(e) => return Err(e),
//!         }
//!     }
//! }
//! ```

use std::time::Duration;

/// Result type alias for operations in poly-clob-rs.
///
/// This is a convenience alias for `std::result::Result<T, ClobError>`.
pub type Result<T> = std::result::Result<T, ClobError>;

/// Top-level error type for all poly-clob-rs operations.
///
/// This enum provides granular error categorization with rich context.
/// Use pattern matching to handle specific error cases, or use the
/// helper methods for common queries.
#[derive(Debug, thiserror::Error)]
pub enum ClobError {
    /// HTTP/network transport error
    #[error("HTTP error: {0}")]
    Http(#[from] HttpError),

    /// API error from the CLOB or relayer
    #[error("API error: {0}")]
    Api(#[from] ApiError),

    /// Serialization or deserialization error
    #[error("serialization error: {0}")]
    Serialization(#[from] SerializationError),

    /// Authentication or signing error
    #[error("authentication error: {0}")]
    Auth(#[from] AuthError),

    /// Client-side validation error
    #[error("validation error: {0}")]
    Validation(#[from] ValidationError),

    /// Relayer-specific error
    #[error("relayer error: {0}")]
    Relayer(#[from] RelayerError),

    /// Feature is not implemented
    #[error("not implemented: {feature}")]
    NotImplemented {
        /// Name of the feature
        feature: String,
    },
}

impl ClobError {
    /// Returns true if this error is transient and the operation could be retried.
    ///
    /// # Examples
    ///
    /// ```
    /// use poly_clob_rs::{ClobError, HttpError};
    ///
    /// let err = ClobError::Http(HttpError::Timeout {
    ///     url: "https://api.example.com".to_string(),
    ///     timeout: std::time::Duration::from_secs(30),
    /// });
    ///
    /// assert!(err.is_retryable());
    /// ```
    pub fn is_retryable(&self) -> bool {
        match self {
            ClobError::Http(e) => e.is_retryable(),
            ClobError::Api(e) => e.is_retryable(),
            ClobError::Serialization(_) => false,
            ClobError::Auth(_) => false,
            ClobError::Validation(_) => false,
            ClobError::Relayer(e) => e.is_retryable(),
            ClobError::NotImplemented { .. } => false,
        }
    }

    /// Returns the suggested retry delay, if this error is retryable.
    ///
    /// # Examples
    ///
    /// ```
    /// use poly_clob_rs::{ClobError, ApiError};
    /// use std::time::Duration;
    ///
    /// let err = ClobError::Api(ApiError::RateLimited {
    ///     retry_after: Duration::from_secs(60),
    ///     url: "https://api.example.com".to_string(),
    ///     retry_after_header: Some("60".to_string()),
    /// });
    ///
    /// assert_eq!(err.retry_after(), Some(Duration::from_secs(60)));
    /// ```
    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            ClobError::Http(e) => e.retry_after(),
            ClobError::Api(e) => e.retry_after(),
            ClobError::Relayer(e) => e.retry_after(),
            _ => None,
        }
    }

    /// Returns the URL associated with this error, if available.
    pub fn url(&self) -> Option<&str> {
        match self {
            ClobError::Http(e) => e.url(),
            ClobError::Api(e) => e.url(),
            _ => None,
        }
    }

    /// Returns true if this is an authentication or authorization error.
    ///
    /// This includes both auth errors (missing credentials, invalid keys)
    /// and authorization errors (401, 403).
    pub fn is_auth_error(&self) -> bool {
        matches!(self, ClobError::Auth(_) | ClobError::Api(ApiError::Unauthorized { .. }) | ClobError::Api(ApiError::Forbidden { .. }))
    }

    /// Returns true if this error indicates a client-side bug or invalid input.
    ///
    /// These errors typically should not be retried and may indicate
    /// bugs in the calling code.
    pub fn is_client_error(&self) -> bool {
        matches!(self, ClobError::Validation(_) | ClobError::Api(ApiError::BadRequest { .. }) | ClobError::Api(ApiError::NotFound { .. }) | ClobError::Serialization(_) | ClobError::NotImplemented { .. })
    }

    /// Returns true if this is a recoverable order error.
    ///
    /// Recoverable order errors are temporary conditions where the strategy can skip
    /// the current order attempt and continue operating. These are NOT retryable
    /// (retrying immediately won't help), but the strategy shouldn't halt.
    ///
    /// Includes:
    /// - FOK/FAK orders that couldn't fill (OrderNotFillable)
    /// - Market not ready to accept orders (MarketNotReady)
    /// - Order delayed due to market conditions (OrderDelayed)
    /// - Post-only orders that would cross the book (PostOnlyCrossesBook)
    /// - Insufficient balance (InsufficientBalance)
    /// - Invalid order sizes or tick sizes (InvalidOrderSize, InvalidTickSize)
    /// - Invalid expirations (InvalidExpiration)
    /// - Duplicate orders (DuplicateOrder)
    ///
    /// # Examples
    ///
    /// ```
    /// use poly_clob_rs::{ClobError, ApiError};
    ///
    /// let err = ClobError::Api(ApiError::OrderNotFillable {
    ///     url: "https://api.example.com".to_string(),
    ///     message: "order couldn't be fully filled".to_string(),
    ///     raw_response: "{}".to_string(),
    /// });
    ///
    /// assert!(err.is_recoverable_order_error());
    /// ```
    pub fn is_recoverable_order_error(&self) -> bool {
        match self {
            ClobError::Api(api_err) => api_err.is_recoverable_order_error(),
            _ => false,
        }
    }
}

//
// HTTP Errors
//

/// HTTP and network transport errors.
#[derive(Debug, thiserror::Error)]
pub enum HttpError {
    /// General HTTP request failure
    #[error("request failed for {url}: {source}")]
    RequestFailed {
        /// The URL of the failed request
        url: String,
        /// The underlying error
        #[source]
        source: reqwest::Error,
    },

    /// Connection timeout
    #[error("connection timeout after {timeout:?} for {url}")]
    Timeout {
        /// The URL that timed out
        url: String,
        /// The timeout duration
        timeout: Duration,
    },

    /// Connection failure
    #[error("connection failed for {url}: {message}")]
    Connection {
        /// The URL that failed to connect
        url: String,
        /// Error message
        message: String,
    },

    /// Failed to read response body
    #[error("failed to read response body from {url}: {message}")]
    ReadBody {
        /// The URL of the request
        url: String,
        /// Error message
        message: String,
    },
}

impl HttpError {
    /// Create HttpError from reqwest::Error with URL context.
    ///
    /// This helper is used when we have a URL to provide better context.
    pub fn from_reqwest(err: reqwest::Error, url: impl Into<String>) -> Self {
        let url = url.into();
        if err.is_timeout() {
            HttpError::Timeout {
                url,
                timeout: Duration::from_secs(30), // Default timeout assumption
            }
        } else if err.is_connect() {
            HttpError::Connection {
                url,
                message: err.to_string(),
            }
        } else {
            HttpError::RequestFailed { url, source: err }
        }
    }
}

impl HttpError {
    /// Returns true if this error is transient and retryable.
    pub fn is_retryable(&self) -> bool {
        match self {
            HttpError::Timeout { .. } => true,
            HttpError::Connection { .. } => true,
            HttpError::RequestFailed { source, .. } => {
                // Timeouts and connection errors are retryable
                source.is_timeout() || source.is_connect()
            }
            HttpError::ReadBody { .. } => false,
        }
    }

    /// Returns suggested retry delay.
    pub fn retry_after(&self) -> Option<Duration> {
        if self.is_retryable() {
            Some(Duration::from_secs(1))
        } else {
            None
        }
    }

    /// Returns the URL associated with this error.
    pub fn url(&self) -> Option<&str> {
        match self {
            HttpError::RequestFailed { url, .. } => Some(url),
            HttpError::Timeout { url, .. } => Some(url),
            HttpError::Connection { url, .. } => Some(url),
            HttpError::ReadBody { url, .. } => Some(url),
        }
    }
}

//
// API Errors
//

/// HTTP API errors with rich context for debugging and handling.
///
/// Each variant includes the URL, response body (when available),
/// and other context to help callers make informed decisions.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    /// Rate limited (HTTP 429)
    #[error("rate limited (retry after {retry_after:?}): {url}")]
    RateLimited {
        /// Suggested retry delay
        retry_after: Duration,
        /// The URL that was rate limited
        url: String,
        /// Raw retry-after header value, if present
        retry_after_header: Option<String>,
    },

    /// Unauthorized (HTTP 401)
    #[error("unauthorized: {url}{}", details.as_ref().map(|d| format!(" - {}", d)).unwrap_or_default())]
    Unauthorized {
        /// The URL that returned 401
        url: String,
        /// Optional details from response body
        details: Option<String>,
    },

    /// Forbidden (HTTP 403)
    #[error("forbidden: {url}{}", details.as_ref().map(|d| format!(" - {}", d)).unwrap_or_default())]
    Forbidden {
        /// The URL that returned 403
        url: String,
        /// Optional details from response body
        details: Option<String>,
    },

    /// Not found (HTTP 404)
    #[error("not found: {resource} at {url}")]
    NotFound {
        /// The URL that returned 404
        url: String,
        /// The type of resource (inferred from URL)
        resource: String,
    },

    /// Bad request (HTTP 400)
    #[error("bad request to {url}: {message}")]
    BadRequest {
        /// The URL of the bad request
        url: String,
        /// Error message
        message: String,
        /// Raw response body for debugging
        raw_response: String,
    },

    /// Order could not be filled (FOK/FAK orders that can't execute)
    /// This is a recoverable error that should not halt trading strategies
    #[error("order not fillable: {message}")]
    OrderNotFillable {
        /// The URL of the request
        url: String,
        /// Error message from the API
        message: String,
        /// Raw response body for debugging
        raw_response: String,
    },

    /// Order validation error - price breaks minimum tick size rules
    #[error("invalid order tick size: {message}")]
    InvalidTickSize {
        /// The URL of the request
        url: String,
        /// Error message
        message: String,
    },

    /// Order validation error - size below minimum
    #[error("invalid order size: {message}")]
    InvalidOrderSize {
        /// The URL of the request
        url: String,
        /// Error message
        message: String,
    },

    /// Order validation error - duplicate order
    #[error("duplicate order: {message}")]
    DuplicateOrder {
        /// The URL of the request
        url: String,
        /// Error message
        message: String,
    },

    /// Order validation error - insufficient balance or allowance
    #[error("insufficient balance: {message}")]
    InsufficientBalance {
        /// The URL of the request
        url: String,
        /// Error message
        message: String,
    },

    /// Order validation error - invalid expiration timestamp
    #[error("invalid expiration: {message}")]
    InvalidExpiration {
        /// The URL of the request
        url: String,
        /// Error message
        message: String,
    },

    /// Post-only order validation error - invalid order type
    #[error("invalid post-only order type: {message}")]
    InvalidPostOnlyType {
        /// The URL of the request
        url: String,
        /// Error message
        message: String,
    },

    /// Post-only order validation error - would cross the book
    #[error("post-only order crosses book: {message}")]
    PostOnlyCrossesBook {
        /// The URL of the request
        url: String,
        /// Error message
        message: String,
    },

    /// Market is not ready to accept orders (recoverable)
    #[error("market not ready: {message}")]
    MarketNotReady {
        /// The URL of the request
        url: String,
        /// Error message
        message: String,
    },

    /// Order was delayed due to market conditions (non-fatal)
    #[error("order delayed: {message}")]
    OrderDelayed {
        /// The URL of the request
        url: String,
        /// Error message
        message: String,
    },

    /// Server error (HTTP 5xx)
    #[error("server error {status} for {url} (transient: {is_transient})")]
    ServerError {
        /// HTTP status code
        status: u16,
        /// The URL that returned the error
        url: String,
        /// Whether this error is likely transient
        is_transient: bool,
        /// Response body, if available
        response_body: Option<String>,
    },

    /// Unexpected HTTP status code
    #[error("unexpected status {status} for {url}: {message}")]
    UnexpectedStatus {
        /// HTTP status code
        status: u16,
        /// The URL that returned the status
        url: String,
        /// Error message
        message: String,
        /// Raw response body
        response_body: String,
    },
}

impl ApiError {
    /// Returns true if this error is transient and retryable.
    pub fn is_retryable(&self) -> bool {
        match self {
            ApiError::RateLimited { .. } => true,
            ApiError::MarketNotReady { .. } => true,
            ApiError::ServerError { is_transient, .. } => *is_transient,
            _ => false,
        }
    }

    /// Returns suggested retry delay.
    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            ApiError::RateLimited { retry_after, .. } => Some(*retry_after),
            ApiError::MarketNotReady { .. } => Some(Duration::from_secs(5)),
            ApiError::ServerError {
                is_transient: true, ..
            } => Some(Duration::from_secs(2)),
            _ => None,
        }
    }

    /// Returns the URL associated with this error.
    pub fn url(&self) -> Option<&str> {
        match self {
            ApiError::RateLimited { url, .. } => Some(url),
            ApiError::Unauthorized { url, .. } => Some(url),
            ApiError::Forbidden { url, .. } => Some(url),
            ApiError::NotFound { url, .. } => Some(url),
            ApiError::BadRequest { url, .. } => Some(url),
            ApiError::OrderNotFillable { url, .. } => Some(url),
            ApiError::InvalidTickSize { url, .. } => Some(url),
            ApiError::InvalidOrderSize { url, .. } => Some(url),
            ApiError::DuplicateOrder { url, .. } => Some(url),
            ApiError::InsufficientBalance { url, .. } => Some(url),
            ApiError::InvalidExpiration { url, .. } => Some(url),
            ApiError::InvalidPostOnlyType { url, .. } => Some(url),
            ApiError::PostOnlyCrossesBook { url, .. } => Some(url),
            ApiError::MarketNotReady { url, .. } => Some(url),
            ApiError::OrderDelayed { url, .. } => Some(url),
            ApiError::ServerError { url, .. } => Some(url),
            ApiError::UnexpectedStatus { url, .. } => Some(url),
        }
    }

    /// Returns true if this is a recoverable order error.
    ///
    /// These errors indicate temporary conditions where an order cannot be placed,
    /// but the trading strategy should continue (skip this order and try again later).
    pub fn is_recoverable_order_error(&self) -> bool {
        matches!(
            self,
            ApiError::OrderNotFillable { .. }
                | ApiError::MarketNotReady { .. }
                | ApiError::OrderDelayed { .. }
                | ApiError::PostOnlyCrossesBook { .. }
                | ApiError::InsufficientBalance { .. }
                | ApiError::InvalidOrderSize { .. }
                | ApiError::InvalidTickSize { .. }
                | ApiError::InvalidExpiration { .. }
                | ApiError::DuplicateOrder { .. }
        )
    }
}

//
// Serialization Errors
//

/// JSON serialization and deserialization errors.
#[derive(Debug, thiserror::Error)]
pub enum SerializationError {
    /// Failed to deserialize JSON response
    #[error("failed to deserialize JSON: {message}")]
    JsonDeserialize {
        /// Error message
        message: String,
        /// Raw response that failed to deserialize
        raw_response: String,
    },

    /// Failed to serialize to JSON
    #[error("failed to serialize to JSON: {message}")]
    JsonSerialize {
        /// Error message
        message: String,
    },

    /// Failed to parse a specific field
    #[error("failed to parse field '{field}': {message}")]
    FieldParse {
        /// Field name
        field: String,
        /// Error message
        message: String,
    },
}

//
// Authentication Errors
//

/// Authentication and signing errors.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    /// Missing required environment variable
    #[error("missing environment variable: {var_name}")]
    MissingEnvVar {
        /// Name of the missing variable
        var_name: String,
    },

    /// Invalid private key format
    #[error("invalid private key: {message}")]
    InvalidPrivateKey {
        /// Error message
        message: String,
    },

    /// EIP712 signature failed
    #[error("failed to sign EIP712 message: {message}")]
    SignatureFailed {
        /// Error message
        message: String,
    },

    /// Invalid address format
    #[error("invalid address: {address}")]
    InvalidAddress {
        /// The invalid address
        address: String,
    },

    /// Failed to build auth headers
    #[error("failed to build auth headers: {message}")]
    HeaderBuildFailed {
        /// Error message
        message: String,
    },
}

//
// Validation Errors
//

/// Client-side validation errors.
///
/// These errors indicate invalid input that should be corrected
/// before retrying the request.
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    /// Invalid order parameters
    #[error("invalid order: {reason}")]
    InvalidOrder {
        /// Reason for invalidity
        reason: String,
    },

    /// Invalid expiration time
    #[error("invalid expiration: {reason}")]
    InvalidExpiration {
        /// Reason for invalidity
        reason: String,
    },

    /// Invalid amount
    #[error("invalid amount: {reason}")]
    InvalidAmount {
        /// Reason for invalidity
        reason: String,
    },

    /// Invalid parameter
    #[error("invalid parameter '{parameter}': {reason}")]
    InvalidParameter {
        /// Parameter name
        parameter: String,
        /// Reason for invalidity
        reason: String,
    },
}

//
// Relayer Errors
//

/// Relayer-specific errors.
#[derive(Debug, thiserror::Error)]
pub enum RelayerError {
    /// Transaction failed
    #[error("transaction failed with state: {state}")]
    TransactionFailed {
        /// Transaction state
        state: String,
        /// Optional error message
        message: Option<String>,
    },

    /// Polling timeout
    #[error("polling timeout after {timeout:?}")]
    PollingTimeout {
        /// Timeout duration
        timeout: Duration,
        /// Last known state
        last_state: Option<String>,
    },

    /// Wallet not deployed
    #[error("wallet not deployed: {address}")]
    WalletNotDeployed {
        /// Wallet address
        address: String,
    },

    /// Invalid nonce
    #[error("invalid nonce: expected {expected}, got {actual}")]
    InvalidNonce {
        /// Expected nonce
        expected: u64,
        /// Actual nonce
        actual: u64,
    },
}

impl RelayerError {
    /// Returns true if this error is retryable.
    pub fn is_retryable(&self) -> bool {
        matches!(self, RelayerError::PollingTimeout { .. } | RelayerError::InvalidNonce { .. })
    }

    /// Returns suggested retry delay.
    pub fn retry_after(&self) -> Option<Duration> {
        if self.is_retryable() {
            Some(Duration::from_secs(1))
        } else {
            None
        }
    }
}
