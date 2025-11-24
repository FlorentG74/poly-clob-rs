//! Webservice utilities for fetching data from Polymarket APIs.
//!
//! This module provides generic functions for making HTTP requests to the Polymarket API
//! with automatic retry logic and error handling.

use reqwest::{Client, Method, RequestBuilder};
use std::time::Duration;
use tokio::time::sleep;

use crate::models::ApiResponse;
use super::WebserviceRequest;

const MAX_RETRIES: u32 = 3;
const RETRY_DELAY_MS: u64 = 2000;

/// Fetch data from API with pagination support.
///
/// This function makes HTTP requests with automatic retry logic for rate limits and transient errors.
/// Returns a tuple of (next_offset, optional_data) where next_offset is -1 if no more pages exist.
///
/// # Arguments
///
/// * `client` - The reqwest HTTP client to use
/// * `web_service_request` - The request configuration (URL, method, parameters)
/// * `next_offset` - The pagination offset (0 for first page)
///
/// # Returns
///
/// A tuple of:
/// - `i32`: The next offset to use for pagination, or -1 if no more pages
/// - `Option<T>`: The deserialized response data, or None if the request failed
///
/// # Example
///
/// ```rust,no_run
/// use poly_clob_rs::{WebserviceRequest, MarketsResponse, api::webservice::fetch_batch};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = reqwest::Client::new();
/// let mut request = WebserviceRequest::new_markets_ws_request();
/// request.with_active_only();
///
/// let (next_offset, markets) = fetch_batch::<MarketsResponse>(&client, &request, 0).await;
/// if let Some(data) = markets {
///     println!("Retrieved {} markets", data.len());
/// }
/// # Ok(())
/// # }
/// ```
pub async fn fetch_batch<T>(
    client: &Client,
    web_service_request: &WebserviceRequest,
    next_offset: i32,
) -> (i32, Option<T>)
where
    T: for<'a> serde::Deserialize<'a> + ApiResponse,
{
    for attempt in 1..=MAX_RETRIES {
        let callable_url = web_service_request.get_callable_url(next_offset);
        let request: RequestBuilder;

        log::debug!(
            "Calling method: {} on url: {} (attempt {}/{})",
            web_service_request.method,
            callable_url,
            attempt,
            MAX_RETRIES
        );

        match web_service_request.method {
            Method::GET => {
                request = client.get(&callable_url);
            }
            Method::POST => {
                request = client
                    .post(&callable_url)
                    .body(web_service_request.get_body());
            }
            _ => {
                log::error!("Unsupported Method");
                return (-1, None);
            }
        }

        match request.send().await {
            Ok(response) => {
                match response.status() {
                    reqwest::StatusCode::OK => {
                        let text = response
                            .text()
                            .await
                            .expect("Error - can't extract API Response");
                        log::trace!("API Response: {}", text);

                        match serde_json::from_str::<T>(&text) {
                            Ok(ws_response) => {
                                let nb_results_retrieved: i32 =
                                    ws_response.nb_results().try_into().unwrap();

                                log::debug!("Retrieved {:?} results", nb_results_retrieved);

                                if nb_results_retrieved > 0 {
                                    if nb_results_retrieved == web_service_request.get_limit() {
                                        return (
                                            next_offset + web_service_request.get_limit(),
                                            Some(ws_response),
                                        );
                                    } else {
                                        return (-1, Some(ws_response));
                                    }
                                } else {
                                    return (-1, None);
                                }
                            }
                            Err(err) => {
                                log::error!("Error - can't deserialize API Response. Err: {}", err);
                                return (-1, None);
                            }
                        }
                    }
                    reqwest::StatusCode::TOO_MANY_REQUESTS
                    | reqwest::StatusCode::GATEWAY_TIMEOUT
                    | reqwest::StatusCode::BAD_GATEWAY => {
                        if attempt < MAX_RETRIES {
                            log::warn!(
                                "Err {} - retrying after {} ms (attempt {}/{})",
                                response.status(),
                                RETRY_DELAY_MS,
                                attempt,
                                MAX_RETRIES
                            );
                            sleep(Duration::from_millis(RETRY_DELAY_MS)).await;
                            continue;
                        } else {
                            log::error!("Err {} - max retries exceeded", response.status());
                            return (next_offset, None);
                        }
                    }
                    reqwest::StatusCode::UNAUTHORIZED => {
                        log::error!("Authentication failed for request {}", callable_url);
                        return (next_offset, None);
                    }
                    other => {
                        log::error!(
                            "Unexpected error in service call: {:?}; url: {}",
                            other,
                            callable_url
                        );
                        return (next_offset, None);
                    }
                }
            }
            Err(err) => {
                log::error!("Error - request failed. Err: {}", err);
                return (-1, None);
            }
        }
    }

    (next_offset, None)
}

/// Fetch a single item from API without pagination.
///
/// Similar to [`fetch_batch`] but for single-item queries (like fetching by slug or ID).
/// Does not handle pagination.
///
/// # Arguments
///
/// * `client` - The reqwest HTTP client to use
/// * `web_service_request` - The request configuration (URL, method, parameters)
///
/// # Returns
///
/// The deserialized response data, or None if the request failed
///
/// # Example
///
/// ```rust,no_run
/// use poly_clob_rs::{WebserviceRequest, PolyResponseMarket, api::webservice::fetch_one};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = reqwest::Client::new();
/// let mut request = WebserviceRequest::new_market_ws_request();
/// request.with_slug("bitcoin-above-100k");
///
/// let market = fetch_one::<PolyResponseMarket>(&client, &request).await;
/// if let Some(m) = market {
///     println!("Found market: {:?}", m.question);
/// }
/// # Ok(())
/// # }
/// ```
pub async fn fetch_one<T>(
    client: &Client,
    web_service_request: &WebserviceRequest,
) -> Option<T>
where
    T: for<'a> serde::Deserialize<'a> + ApiResponse,
{
    for attempt in 1..=MAX_RETRIES {
        let callable_url = web_service_request.get_callable_url(0);
        let request: RequestBuilder;

        log::debug!(
            "Calling method: {} on url: {} (attempt {}/{})",
            web_service_request.method,
            callable_url,
            attempt,
            MAX_RETRIES
        );

        match web_service_request.method {
            Method::GET => {
                request = client.get(&callable_url);
            }
            Method::POST => {
                request = client
                    .post(&callable_url)
                    .body(web_service_request.get_body());
            }
            _ => {
                log::error!("Unsupported Method");
                return None;
            }
        }

        let response = match request.send().await {
            Ok(r) => r,
            Err(err) => {
                log::error!("Error - request failed. Err: {}", err);
                return None;
            }
        };

        match response.status() {
            reqwest::StatusCode::OK => {
                let text = response
                    .text()
                    .await
                    .expect("Error - can't extract API Response");
                log::trace!("API Response: {}", text);

                match serde_json::from_str::<T>(&text) {
                    Ok(ws_response) => {
                        return Some(ws_response);
                    }
                    Err(err) => {
                        log::error!("Error - can't deserialize API Response. Err: {}", err);
                        return None;
                    }
                }
            }
            reqwest::StatusCode::TOO_MANY_REQUESTS
            | reqwest::StatusCode::GATEWAY_TIMEOUT
            | reqwest::StatusCode::BAD_GATEWAY => {
                if attempt < MAX_RETRIES {
                    log::warn!(
                        "Err {} - retrying after {} ms (attempt {}/{})",
                        response.status(),
                        RETRY_DELAY_MS,
                        attempt,
                        MAX_RETRIES
                    );
                    sleep(Duration::from_millis(RETRY_DELAY_MS)).await;
                    continue;
                } else {
                    log::error!("Err {} - max retries exceeded", response.status());
                    return None;
                }
            }
            reqwest::StatusCode::UNAUTHORIZED => {
                log::error!("Authentication failed for request {}", callable_url);
                return None;
            }
            other => {
                log::error!(
                    "Unexpected error in service call: {:?}; url: {}",
                    other,
                    callable_url
                );
                return None;
            }
        }
    }

    None
}

pub fn add_param_to_url(url: &mut String, name: &str, value: &str) {
    if value.is_empty() {
        return;
    }

    if !url.contains("?") {
        url.push_str(format!("?{}={}", name, value).as_str());
    } else {
        url.push_str(format!("&{}={}", name, value).as_str());
    }
}