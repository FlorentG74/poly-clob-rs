//! HTTP response handling utilities.
//!
//! Provides consistent error handling for API responses with rich context.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use reqwest::{Response, StatusCode, Version};

use super::error::{ApiError, HttpError, Result};
use super::GET_CRYPTO_PRICE;

/// One-time transport-degradation alert for the crypto-price endpoint.
///
/// The 2026-06-23 outage: Polymarket's Cloudflare began 403-ing HTTP/1.1 requests to
/// the crypto-price endpoint, silently starving the SVS strike/settlement feed. The
/// `#[ignore]`d live smoke test guards it in CI, but nothing warned in production. This
/// raises a loud runtime alert the first time either failure mode is observed on that
/// endpoint: (1) the connection negotiated something other than HTTP/2, or (2) a 403.
static CRYPTO_PRICE_NON_H2_ALERTED: AtomicBool = AtomicBool::new(false);
static CRYPTO_PRICE_403_ALERTED: AtomicBool = AtomicBool::new(false);

/// The transport degradation, if any, worth alerting on for a crypto-price response.
/// Pure decision fn (no I/O, no once-guard) so the gating is unit-testable.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum CryptoPriceTransportAlert {
    /// The connection negotiated something other than HTTP/2 — Cloudflare 403s h1 here.
    NonHttp2,
    /// The endpoint returned 403 Forbidden — the symptom of an h1 fallback being rejected.
    Forbidden,
}

fn crypto_price_transport_alert(
    url: &str,
    is_http2: bool,
    is_forbidden: bool,
) -> Option<CryptoPriceTransportAlert> {
    if !url.contains(GET_CRYPTO_PRICE) {
        return None;
    }
    // A 403 is the more actionable symptom, so report it in preference to non-h2.
    if is_forbidden {
        Some(CryptoPriceTransportAlert::Forbidden)
    } else if !is_http2 {
        Some(CryptoPriceTransportAlert::NonHttp2)
    } else {
        None
    }
}

/// Emit the crypto-price transport alert at most once per process per failure mode.
///
/// The `collapsible_match` suggestion here (fold each arm's `if` into a match guard) is
/// BROKEN — applying it drops the `NonHttp2` arm and leaves a non-exhaustive match that
/// does not compile (`cargo clippy --fix` rolls the whole crate back on it). It would also
/// move a side-effecting atomic swap into a match guard, which is worse to read.
#[allow(clippy::collapsible_match, reason = "the suggested fix does not compile; see above")]
fn alert_crypto_price_transport(url: &str, version: Version, status: StatusCode) {
    match crypto_price_transport_alert(url, version == Version::HTTP_2, status == StatusCode::FORBIDDEN) {
        Some(CryptoPriceTransportAlert::Forbidden) => {
            if !CRYPTO_PRICE_403_ALERTED.swap(true, Ordering::Relaxed) {
                log::error!(
                    "[transport-alert] crypto-price endpoint returned 403 ({url}). This is the \
                     2026-06-23 Cloudflare failure mode: an HTTP/1.1 fallback being rejected, which \
                     silently starves the strike/settlement feed. Verify the reqwest `http2` feature."
                );
            }
        }
        Some(CryptoPriceTransportAlert::NonHttp2) => {
            if !CRYPTO_PRICE_NON_H2_ALERTED.swap(true, Ordering::Relaxed) {
                log::error!(
                    "[transport-alert] crypto-price request to {url} negotiated {version:?}, not HTTP/2. \
                     Cloudflare 403s HTTP/1.1 on this endpoint — the client MUST negotiate h2 \
                     (reqwest `http2` feature) or the strike/settlement feed will start failing."
                );
            }
        }
        None => {}
    }
}

/// Default retry delay for rate limiting (in seconds), used when the server sends no
/// usable `Retry-After` header.
pub const DEFAULT_RATE_LIMIT_DELAY_SECS: u64 = 5;

/// Minimum retry delay enforced on any rate-limit (429) response.
///
/// Polymarket returns
/// `Retry-After: 0` on its 429s, which would otherwise produce a zero-delay retry loop
/// that hammers the endpoint (and, with no backoff, keeps the limiter tripped). Flooring
/// here guarantees every 429 backs off by at least this much regardless of the header.
pub const MIN_RATE_LIMIT_DELAY_SECS: u64 = 1;

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
/// - **5xx Server Errors**: Returns `ApiError::ServerError` with transient flag (500/502/503/504 are transient)
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
///
/// # Errors
///
/// If the status is a non-success one, or the body cannot be read. Rate-limit (429)
/// responses surface as a retryable error carrying the floored delay.
pub async fn handle_api_response(response: Response, url: &str) -> Result<String> {
    let status = response.status();

    // Runtime guard for the crypto-price endpoint's Cloudflare/h2 failure mode.
    alert_crypto_price_transport(url, response.version(), status);

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
                // Treat 500 as transient, like 502/503/504 below: on Polymarket's endpoints
                // (notably POST /order) a 500 is a server-side matching-engine hiccup, not a
                // client fault — retryable, and callers degrade-and-skip rather than halt.
                is_transient: true,
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
        // 425 Too Early — Polymarket's POST /order returns this with
        // {"error":"order manager not ready, please retry"} when its matching engine
        // is briefly warming up. It is a retry hint, not a client fault, so map it to
        // MarketNotReady (retryable + recoverable-order-error): the live buy path skips
        // the order and keeps trading instead of latching the whole arm dead.
        StatusCode::TOO_EARLY => {
            let response_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read response body".to_string());
            log::warn!("Order manager not ready (425) for {}: {}", url, response_body);

            Err(ApiError::MarketNotReady {
                url: url.to_string(),
                message: response_body,
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

    // Absent/unparseable header -> default; then floor so a server-sent `Retry-After: 0`
    // (Polymarket does this) can never yield a zero-delay retry that hammers the endpoint.
    let duration = header_value
        .as_ref()
        .and_then(|v| v.parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or(Duration::from_secs(DEFAULT_RATE_LIMIT_DELAY_SECS))
        .max(Duration::from_secs(MIN_RATE_LIMIT_DELAY_SECS));

    (duration, header_value)
}

/// Parse a 400 Bad Request response and map it to the appropriate error type.
///
/// Maps Polymarket-specific error messages to typed errors for better handling:
/// - FOK/FAK order fill failures -> `OrderNotFillable`
/// - Tick size violations -> `InvalidTickSize`
/// - Minimum size violations -> `InvalidOrderSize`
/// - Duplicate orders -> `DuplicateOrder`
/// - Insufficient balance -> `InsufficientBalance`
/// - Invalid expiration -> `InvalidExpiration`
/// - Post-only errors -> `InvalidPostOnlyType` or `PostOnlyCrossesBook`
/// - Market not ready -> `MarketNotReady`
/// - Order delayed -> `OrderDelayed`
/// - Everything else -> `BadRequest`
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

#[cfg(test)]
mod tests {
    use super::*;

    const CRYPTO_URL: &str = "https://polymarket.com/crypto/crypto-price?symbol=BTC";
    const OTHER_URL: &str = "https://polymarket.com/some/other/endpoint";
    const ORDER_URL: &str = "https://clob.polymarket.com/order";

    // A 425 "order manager not ready" on POST /order must be treated as a transient,
    // recoverable order error (skip the order, keep trading) — NOT a fatal status that
    // latches the strategy dead. Regression guard for the Jul 24 XRP overnight halt.
    #[tokio::test]
    async fn too_early_425_maps_to_market_not_ready() {
        let http_resp = http::Response::builder()
            .status(425)
            .body(r#"{"error":"order manager not ready, please retry"}"#.to_string())
            .unwrap();
        let response = Response::from(http_resp);

        let err = handle_api_response(response, ORDER_URL).await.unwrap_err();

        match &err {
            crate::ClobError::Api(api @ ApiError::MarketNotReady { .. }) => {
                assert!(api.is_retryable(), "425 should be retryable");
                assert!(
                    api.is_recoverable_order_error(),
                    "425 should be a recoverable order error so the arm keeps trading"
                );
            }
            other => panic!("expected MarketNotReady, got {other:?}"),
        }
    }

    #[test]
    fn h2_ok_on_crypto_price_no_alert() {
        // Healthy path: HTTP/2, 200 → nothing to alert on.
        assert_eq!(
            crypto_price_transport_alert(CRYPTO_URL, /*is_http2=*/ true, /*is_forbidden=*/ false),
            None
        );
    }

    #[test]
    fn non_h2_on_crypto_price_alerts() {
        assert_eq!(
            crypto_price_transport_alert(CRYPTO_URL, false, false),
            Some(CryptoPriceTransportAlert::NonHttp2)
        );
    }

    #[test]
    fn forbidden_on_crypto_price_alerts() {
        assert_eq!(
            crypto_price_transport_alert(CRYPTO_URL, true, true),
            Some(CryptoPriceTransportAlert::Forbidden)
        );
    }

    #[test]
    fn forbidden_takes_precedence_over_non_h2() {
        // Both symptoms at once → the 403 (more actionable) is reported.
        assert_eq!(
            crypto_price_transport_alert(CRYPTO_URL, false, true),
            Some(CryptoPriceTransportAlert::Forbidden)
        );
    }

    #[test]
    fn other_endpoints_never_alert() {
        // The guard is scoped to the crypto-price endpoint; h1/403 elsewhere is out of scope.
        assert_eq!(crypto_price_transport_alert(OTHER_URL, false, true), None);
        assert_eq!(crypto_price_transport_alert(OTHER_URL, false, false), None);
    }
}
