//! HTTP response handling utilities.
//!
//! Provides consistent error handling for API responses.

use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::{Response, StatusCode};

use super::error::ApiError;

/// Default retry delay for rate limiting (in seconds).
pub const DEFAULT_RATE_LIMIT_DELAY_SECS: u64 = 5;

/// Handle HTTP API responses with consistent error handling.
///
/// Returns `Ok(String)` with response body on success, or `Err` on failure.
///
/// # Arguments
///
/// * `response` - The HTTP response to handle
/// * `url` - The URL that was called (for logging purposes)
///
/// # Behavior
///
/// - **200 OK**: Returns the response body as a string
/// - **400 Bad Request**: Returns `ApiError::BadRequest` with error details
/// - **401 Unauthorized**: Returns `ApiError::Unauthorized`
/// - **403 Forbidden**: Returns `ApiError::Forbidden`
/// - **404 Not Found**: Returns `ApiError::NotFound`
/// - **429 Too Many Requests**: Returns `ApiError::RateLimited` with retry delay
/// - **5xx Server Errors**: Returns `ApiError::ServerError` (retryable for 502/503/504)
/// - **Other**: Returns `ApiError::Other`
///
/// # Rate Limiting
///
/// This function does NOT automatically sleep on rate limits. Instead, it returns
/// an `ApiError::RateLimited` error with a suggested retry delay. Callers can use
/// `ApiError::is_retryable()` and `ApiError::retry_after()` to implement their own
/// retry strategy.
///
/// # Example
///
/// ```no_run
/// use poly_clob_rs::api::response_handler::handle_api_response;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = reqwest::Client::new();
/// let response = client.get("https://api.example.com").send().await?;
/// let text = handle_api_response(response, "https://api.example.com").await?;
/// println!("Response: {}", text);
/// # Ok(())
/// # }
/// ```
pub async fn handle_api_response(response: Response, url: &str) -> Result<String> {
    let status = response.status();

    match status {
        StatusCode::OK => {
            let text = response
                .text()
                .await
                .context("failed to read response body")?;
            log::info!("API response: {}", text);
            Ok(text)
        }
        StatusCode::BAD_REQUEST => {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            log::error!("Bad request for {}: {}", url, error_text);
            Err(ApiError::BadRequest {
                message: error_text,
            }
            .into())
        }
        StatusCode::UNAUTHORIZED => {
            log::error!("Authentication failed for request {}", url);
            Err(ApiError::Unauthorized.into())
        }
        StatusCode::FORBIDDEN => {
            log::error!("Access forbidden for request {}", url);
            Err(ApiError::Forbidden.into())
        }
        StatusCode::NOT_FOUND => {
            log::error!("Resource not found: {}", url);
            Err(ApiError::NotFound {
                url: url.to_string(),
            }
            .into())
        }
        StatusCode::TOO_MANY_REQUESTS => {
            let retry_after = parse_retry_after(&response)
                .unwrap_or(Duration::from_secs(DEFAULT_RATE_LIMIT_DELAY_SECS));
            log::warn!(
                "Rate limit reached for {}, retry after {:?}",
                url,
                retry_after
            );
            Err(ApiError::RateLimited { retry_after }.into())
        }
        // Server errors
        StatusCode::INTERNAL_SERVER_ERROR => {
            log::error!("Internal server error for {}", url);
            Err(ApiError::ServerError {
                status: 500,
                retryable: false,
            }
            .into())
        }
        StatusCode::BAD_GATEWAY => {
            log::error!("Bad gateway for {}", url);
            Err(ApiError::ServerError {
                status: 502,
                retryable: true,
            }
            .into())
        }
        StatusCode::SERVICE_UNAVAILABLE => {
            log::error!("Service unavailable for {}", url);
            Err(ApiError::ServerError {
                status: 503,
                retryable: true,
            }
            .into())
        }
        StatusCode::GATEWAY_TIMEOUT => {
            log::error!("Gateway timeout for {}", url);
            Err(ApiError::ServerError {
                status: 504,
                retryable: true,
            }
            .into())
        }
        other => {
            let status_code = other.as_u16();
            let message = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            log::error!(
                "Unexpected error {} in service call to {}: {}",
                status_code,
                url,
                message
            );
            Err(ApiError::Other {
                status: status_code,
                message,
            }
            .into())
        }
    }
}

/// Parse the Retry-After header from a response.
fn parse_retry_after(response: &Response) -> Option<Duration> {
    response
        .headers()
        .get("retry-after")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .map(Duration::from_secs)
}
