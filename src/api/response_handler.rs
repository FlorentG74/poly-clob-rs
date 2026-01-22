//! HTTP response handling utilities.
//!
//! Provides consistent error handling for API responses with rich context.

use std::time::Duration;

use reqwest::{Response, StatusCode};

use super::error::{ApiError, HttpError, Result};

/// Default retry delay for rate limiting (in seconds).
pub const DEFAULT_RATE_LIMIT_DELAY_SECS: u64 = 5;

/// Handle HTTP API responses with consistent error handling.
///
/// Returns `Ok(String)` with response body on success, or `Err` with detailed error context.
///
/// # Arguments
///
/// * `response` - The HTTP response to handle
/// * `url` - The URL that was called (for logging and error context)
///
/// # Behavior
///
/// - **200 OK**: Returns the response body as a string
/// - **400 Bad Request**: Returns `ApiError::BadRequest` with error details and raw response
/// - **401 Unauthorized**: Returns `ApiError::Unauthorized` with optional details
/// - **403 Forbidden**: Returns `ApiError::Forbidden` with optional details
/// - **404 Not Found**: Returns `ApiError::NotFound` with resource type inferred from URL
/// - **429 Too Many Requests**: Returns `ApiError::RateLimited` with retry delay from header
/// - **5xx Server Errors**: Returns `ApiError::ServerError` with transient flag (502/503/504 are transient)
/// - **Other**: Returns `ApiError::UnexpectedStatus` with full context
///
/// # Rate Limiting
///
/// This function does NOT automatically sleep on rate limits. Instead, it returns
/// an `ApiError::RateLimited` error with a suggested retry delay parsed from the
/// `Retry-After` header. Callers can use `ClobError::is_retryable()` and
/// `ClobError::retry_after()` to implement their own retry strategy.
///
/// # Example
///
/// ```no_run
/// use poly_clob_rs::api::response_handler::handle_api_response;
///
/// # async fn example() -> Result<(), poly_clob_rs::ClobError> {
/// let client = reqwest::Client::new();
/// let response = client.get("https://api.example.com").send().await
///     .map_err(|e| poly_clob_rs::HttpError::RequestFailed {
///         url: "https://api.example.com".to_string(),
///         source: e,
///     })?;
/// let text = handle_api_response(response, "https://api.example.com").await?;
/// println!("Response: {}", text);
/// # Ok(())
/// # }
/// ```
pub async fn handle_api_response(response: Response, url: &str) -> Result<String> {
    let status = response.status();

    match status {
        StatusCode::OK => {
            let text = response.text().await.map_err(|e| {
                HttpError::ReadBody {
                    url: url.to_string(),
                    message: e.to_string(),
                }
            })?;
            log::trace!("API response: {}", text);
            Ok(text)
        }
        StatusCode::BAD_REQUEST => {
            let raw_response = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read response body".to_string());

            log::error!("Bad request for {}: {}", url, raw_response);

            // Parse error message and map to specific error types
            let error = parse_bad_request_error(&raw_response, url);
            Err(error.into())
        }
        StatusCode::UNAUTHORIZED => {
            let details = response.text().await.ok();
            log::error!("Authentication failed for request {}", url);

            Err(ApiError::Unauthorized {
                url: url.to_string(),
                details,
            }
            .into())
        }
        StatusCode::FORBIDDEN => {
            let details = response.text().await.ok();
            log::error!("Access forbidden for request {}", url);

            Err(ApiError::Forbidden {
                url: url.to_string(),
                details,
            }
            .into())
        }
        StatusCode::NOT_FOUND => {
            log::error!("Resource not found: {}", url);

            // Extract last path segment as resource type, or use "resource" as fallback
            let resource = url
                .trim_end_matches('/')
                .rsplit('/')
                .next()
                .filter(|s| !s.is_empty() && !s.starts_with('?'))
                .unwrap_or("resource")
                .to_string();

            Err(ApiError::NotFound {
                url: url.to_string(),
                resource,
            }
            .into())
        }
        StatusCode::TOO_MANY_REQUESTS => {
            let (retry_after, retry_after_header) = parse_retry_after(&response);

            log::warn!(
                "Rate limit reached for {}, retry after {:?}",
                url,
                retry_after
            );

            Err(ApiError::RateLimited {
                retry_after,
                url: url.to_string(),
                retry_after_header,
            }
            .into())
        }
        // Server errors
        StatusCode::INTERNAL_SERVER_ERROR => {
            let response_body = response.text().await.ok();
            log::error!("Internal server error for {}", url);

            Err(ApiError::ServerError {
                status: 500,
                url: url.to_string(),
                is_transient: false, // 500 is typically not transient
                response_body,
            }
            .into())
        }
        StatusCode::BAD_GATEWAY => {
            let response_body = response.text().await.ok();
            log::error!("Bad gateway for {}", url);

            Err(ApiError::ServerError {
                status: 502,
                url: url.to_string(),
                is_transient: true,
                response_body,
            }
            .into())
        }
        StatusCode::SERVICE_UNAVAILABLE => {
            let response_body = response.text().await.ok();
            log::error!("Service unavailable for {}", url);

            Err(ApiError::ServerError {
                status: 503,
                url: url.to_string(),
                is_transient: true,
                response_body,
            }
            .into())
        }
        StatusCode::GATEWAY_TIMEOUT => {
            let response_body = response.text().await.ok();
            log::error!("Gateway timeout for {}", url);

            Err(ApiError::ServerError {
                status: 504,
                url: url.to_string(),
                is_transient: true,
                response_body,
            }
            .into())
        }
        other => {
            let status_code = other.as_u16();
            let response_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read response body".to_string());

            log::error!(
                "Unexpected error {} in service call to {}: {}",
                status_code,
                url,
                response_body
            );

            Err(ApiError::UnexpectedStatus {
                status: status_code,
                url: url.to_string(),
                message: response_body.clone(),
                response_body,
            }
            .into())
        }
    }
}

/// Parse the Retry-After header from a response.
///
/// Returns a tuple of (Duration, Option<String>) where the second element
/// is the raw header value for debugging.
fn parse_retry_after(response: &Response) -> (Duration, Option<String>) {
    let header_value = response
        .headers()
        .get("retry-after")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let duration = header_value
        .as_ref()
        .and_then(|v| v.parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or(Duration::from_secs(DEFAULT_RATE_LIMIT_DELAY_SECS));

    (duration, header_value)
}

/// Parse a 400 Bad Request response and map it to the appropriate error type.
///
/// Maps Polymarket-specific error messages to typed errors for better handling:
/// - FOK/FAK order fill failures -> OrderNotFillable
/// - Tick size violations -> InvalidTickSize
/// - Minimum size violations -> InvalidOrderSize
/// - Duplicate orders -> DuplicateOrder
/// - Insufficient balance -> InsufficientBalance
/// - Invalid expiration -> InvalidExpiration
/// - Post-only errors -> InvalidPostOnlyType or PostOnlyCrossesBook
/// - Market not ready -> MarketNotReady
/// - Order delayed -> OrderDelayed
/// - Everything else -> BadRequest
fn parse_bad_request_error(raw_response: &str, url: &str) -> ApiError {
    let message_lower = raw_response.to_lowercase();

    // Check for FOK/FAK order fill failures
    if message_lower.contains("couldn't be fully filled")
        || message_lower.contains("fok orders are fully filled")
        || message_lower.contains("fill-or-kill")
        || message_lower.contains("fill-and-kill") {
        return ApiError::OrderNotFillable {
            url: url.to_string(),
            message: raw_response.to_string(),
            raw_response: raw_response.to_string(),
        };
    }

    // Check for minimum tick size errors
    if message_lower.contains("minimum tick size")
        || message_lower.contains("price breaks minimum tick") {
        return ApiError::InvalidTickSize {
            url: url.to_string(),
            message: raw_response.to_string(),
        };
    }

    // Check for minimum size errors
    if message_lower.contains("size lower than")
        || message_lower.contains("minimum size") {
        return ApiError::InvalidOrderSize {
            url: url.to_string(),
            message: raw_response.to_string(),
        };
    }

    // Check for duplicate order errors
    if message_lower.contains("duplicated")
        || message_lower.contains("same order has already been placed") {
        return ApiError::DuplicateOrder {
            url: url.to_string(),
            message: raw_response.to_string(),
        };
    }

    // Check for insufficient balance errors
    if message_lower.contains("not enough balance")
        || message_lower.contains("not enough allowance")
        || message_lower.contains("insufficient balance") {
        return ApiError::InsufficientBalance {
            url: url.to_string(),
            message: raw_response.to_string(),
        };
    }

    // Check for expiration errors
    if message_lower.contains("invalid expiration")
        || message_lower.contains("expiration timestamp") {
        return ApiError::InvalidExpiration {
            url: url.to_string(),
            message: raw_response.to_string(),
        };
    }

    // Check for post-only order type errors
    if message_lower.contains("invalid post-only order")
        && message_lower.contains("only gtc and gtd") {
        return ApiError::InvalidPostOnlyType {
            url: url.to_string(),
            message: raw_response.to_string(),
        };
    }

    // Check for post-only crosses book errors
    if message_lower.contains("post-only")
        && message_lower.contains("crosses book") {
        return ApiError::PostOnlyCrossesBook {
            url: url.to_string(),
            message: raw_response.to_string(),
        };
    }

    // Check for market not ready errors
    if message_lower.contains("market is not yet ready")
        || message_lower.contains("market not ready") {
        return ApiError::MarketNotReady {
            url: url.to_string(),
            message: raw_response.to_string(),
        };
    }

    // Check for order delayed (non-fatal)
    if message_lower.contains("order match delayed")
        || message_lower.contains("order delayed") {
        return ApiError::OrderDelayed {
            url: url.to_string(),
            message: raw_response.to_string(),
        };
    }

    // Default to BadRequest for unrecognized errors
    ApiError::BadRequest {
        url: url.to_string(),
        message: raw_response.to_string(),
        raw_response: raw_response.to_string(),
    }
}
