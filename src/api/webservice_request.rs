//! Webservice utilities for fetching data from Polymarket APIs.
//!
//! This module provides generic functions for making HTTP requests to the Polymarket API
//! with automatic retry logic and error handling.
use crate::api::response_handler::handle_api_response;
use reqwest::{Client, Method};

use crate::models::{ApiResponse, KeysetApiResponse};



/// Represents an HTTP request to the Polymarket API
pub struct WebserviceRequest {
    pub api: String,
    pub url: String,
    pub method: Method,
    pub with_pagination: bool,
    pub args: Vec<(String, String)>,
    pub body: Option<String>,
}

impl WebserviceRequest {
    #[must_use]
    pub fn get_limit(&self) -> i32 {
        for (name, value) in self.args.iter() {
            if name.eq("limit") {
                return value.parse().unwrap_or(100);
            }
        }
        100
    }

    pub fn add_arg(&mut self, name: String, value: String) {
        self.args.push((name, value));
    }

    #[must_use]
    pub fn get_callable_url(&self, next_offset: i32) -> String {
        let api = &self.api;
        let url = &self.url;
        let limit = self.get_limit();

        let mut callable_url = if self.with_pagination {
            format!("{api}{url}?limit={limit}&offset={next_offset}")
        } else {
            format!("{api}{url}")
        };

        for (param_name, param_value) in self.args.iter() {
            Self::add_param_to_url(&mut callable_url, param_name.as_str(), param_value.as_str());
        }
        callable_url
    }

    #[must_use]
    pub fn get_body(&self) -> Option<String> {
        self.body.clone()
    }

    /// Fetch data from API with pagination support.
    ///
    /// This function makes HTTP requests with automatic retry logic for rate limits and transient errors.
    /// Returns a tuple of (`next_offset`, `optional_data`) where `next_offset` is -1 if no more pages exist.
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
    /// use poly_clob_rs::api::http_client::get_http_client;
    /// use poly_clob_rs::{WebserviceRequest, MarketsResponse};
    /// use reqwest::Method;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let request = WebserviceRequest {
    ///     api: "https://gamma-api.polymarket.com".to_string(),
    ///     url: "/markets".to_string(),
    ///     method: Method::GET,
    ///     with_pagination: true,
    ///     args: vec![("active".to_string(), "true".to_string())],
    ///     body: None,
    /// };
    /// let client = get_http_client(Some(&request.api));
    ///
    /// let (next_offset, markets) = WebserviceRequest::fetch_batch::<MarketsResponse>(client, &request, 0).await?;
    /// if !markets.is_empty() {
    ///     println!("Retrieved {} markets", markets.len());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
/// # Errors
///
/// If the request fails, the API returns a non-success status, or the body does not
/// deserialize into the expected shape.
pub async fn fetch_batch<T>(
        client: &Client,
        web_service_request: &WebserviceRequest,
        next_offset: i32,
    ) -> crate::Result<(i32, T)>
    where
        T: for<'a> serde::Deserialize<'a> + ApiResponse,
    {
        let callable_url = web_service_request.get_callable_url(next_offset);

        log::debug!(
            "Calling method: {} on url: {}",
            web_service_request.method,
            callable_url,
        );

        const MAX_RETRIES: u32 = 3;
        let mut attempt = 0u32;

        loop {
            let request = match web_service_request.method {
                Method::GET => client.get(&callable_url),
                Method::POST => {
                    let req = client.post(&callable_url);
                    if let Some(body) = web_service_request.get_body() {
                        req.body(body)
                    } else {
                        req
                    }
                }
                _ => {
                    return Err(crate::ClobError::Validation(
                        crate::ValidationError::InvalidParameter {
                            parameter: "method".to_string(),
                            reason: format!("Unsupported method: {}", web_service_request.method),
                        },
                    ));
                }
            };

            let result: crate::Result<(i32, T)> = async {
                let response = request
                    .send()
                    .await
                    .map_err(|e| crate::HttpError::from_reqwest(e, &callable_url))?;

                let response_text = handle_api_response(response, &callable_url).await?;

                let ws_response: T = serde_json::from_str(&response_text).map_err(|e| {
                    crate::ClobError::from(crate::SerializationError::JsonDeserialize {
                        message: e.to_string(),
                        raw_response: response_text.clone(),
                    })
                })?;

                let nb_results_retrieved = ws_response.nb_results() as i32;
                log::debug!("Retrieved {:?} results", nb_results_retrieved);

                let new_offset = if nb_results_retrieved == web_service_request.get_limit() {
                    next_offset + web_service_request.get_limit()
                } else {
                    -1
                };

                Ok((new_offset, ws_response))
            }
            .await;

            match result {
                Ok(v) => return Ok(v),
                Err(e) if e.is_retryable() && attempt < MAX_RETRIES => {
                    attempt += 1;
                    let delay = e
                        .retry_after()
                        .unwrap_or(std::time::Duration::from_secs(2));
                    log::warn!(
                        "Transient error on attempt {}/{} for {}: {}. Retrying after {:?}",
                        attempt,
                        MAX_RETRIES,
                        callable_url,
                        e,
                        delay
                    );
                    tokio::time::sleep(delay).await;
                }
                Err(e) => return Err(e),
            }
        }
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
    /// use poly_clob_rs::api::http_client::get_http_client;
    /// use poly_clob_rs::{WebserviceRequest, PolyResponseMarket};
    /// use reqwest::Method;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let request = WebserviceRequest {
    ///     api: "https://gamma-api.polymarket.com".to_string(),
    ///     url: "/markets/slug/bitcoin-above-100k".to_string(),
    ///     method: Method::GET,
    ///     with_pagination: false,
    ///     args: Vec::new(),
    ///     body: None,
    /// };
    /// let client = get_http_client(Some(&request.api));
    ///
    /// let market = WebserviceRequest::fetch_one::<PolyResponseMarket>(client, &request).await?;
    /// println!("Found market: {:?}", market.question);
    /// # Ok(())
    /// # }
    /// ```
    ///
/// # Errors
///
/// If the request fails, the API returns a non-success status, or the body does not
/// deserialize into the expected shape.
pub async fn fetch_one<T>(client: &Client, web_service_request: &WebserviceRequest) -> crate::Result<T>
    where
        T: for<'a> serde::Deserialize<'a>,
    {
        let callable_url = web_service_request.get_callable_url(0);
        log::debug!(
            "Calling method: {} on url: {}",
            web_service_request.method,
            callable_url,
        );

        const MAX_RETRIES: u32 = 3;
        let mut attempt = 0u32;

        loop {
            let request = match web_service_request.method {
                Method::GET => client.get(&callable_url),
                Method::POST => {
                    let req = client.post(&callable_url);
                    if let Some(body) = web_service_request.get_body() {
                        req.body(body)
                    } else {
                        req
                    }
                }
                _ => {
                    return Err(crate::ClobError::Validation(
                        crate::ValidationError::InvalidParameter {
                            parameter: "method".to_string(),
                            reason: format!("Unsupported method: {}", web_service_request.method),
                        },
                    ));
                }
            };

            let result: crate::Result<T> = async {
                let response = request
                    .send()
                    .await
                    .map_err(|e| crate::HttpError::from_reqwest(e, &callable_url))?;

                let response_text = handle_api_response(response, &callable_url).await?;

                serde_json::from_str::<T>(&response_text).map_err(|e| {
                    crate::ClobError::from(crate::SerializationError::JsonDeserialize {
                        message: e.to_string(),
                        raw_response: response_text.clone(),
                    })
                })
            }
            .await;

            match result {
                Ok(v) => return Ok(v),
                Err(e) if e.is_retryable() && attempt < MAX_RETRIES => {
                    attempt += 1;
                    let delay = e
                        .retry_after()
                        .unwrap_or(std::time::Duration::from_secs(2));
                    log::warn!(
                        "Transient error on attempt {}/{} for {}: {}. Retrying after {:?}",
                        attempt,
                        MAX_RETRIES,
                        callable_url,
                        e,
                        delay
                    );
                    tokio::time::sleep(delay).await;
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Build a URL for a keyset-paginated request.
    ///
    /// Unlike [`get_callable_url`], this method does **not** inject an `offset`
    /// parameter. Instead it appends `after_cursor` when a cursor is provided.
    /// Other query params from `self.args` are appended as usual.
    #[must_use]
    pub fn get_keyset_url(&self, cursor: Option<&str>) -> String {
        let api = &self.api;
        let url = &self.url;
        let limit = self.get_limit();

        let mut callable_url = format!("{api}{url}?limit={limit}");

        if let Some(c) = cursor
            && !c.is_empty() {
                Self::add_param_to_url(&mut callable_url, "after_cursor", c);
            }

        for (param_name, param_value) in self.args.iter() {
            if param_name == "limit" {
                continue; // already written in the ?limit= prefix
            }
            Self::add_param_to_url(&mut callable_url, param_name.as_str(), param_value.as_str());
        }
        callable_url
    }

    /// Fetch one page from a keyset-paginated endpoint.
    ///
    /// Returns `(next_cursor, response)` where `next_cursor` is `None` when
    /// there are no more pages.
    ///
    /// # Arguments
    ///
    /// * `client` - The reqwest HTTP client to use
    /// * `web_service_request` - The request configuration (URL, method, parameters)
    /// * `cursor` - The cursor from the previous response, or `None` for the first page
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use poly_clob_rs::{WebserviceRequest, models::KeysetMarketsResponse};
    /// use reqwest::Method;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = reqwest::Client::new();
    /// let request = WebserviceRequest {
    ///     api: "https://gamma-api.polymarket.com".to_string(),
    ///     url: "/markets/keyset".to_string(),
    ///     method: Method::GET,
    ///     with_pagination: false,
    ///     args: vec![],
    ///     body: None,
    /// };
    ///
    /// let mut cursor: Option<String> = None;
    /// loop {
    ///     let page = WebserviceRequest::fetch_keyset::<KeysetMarketsResponse>(
    ///         &client, &request, cursor.as_deref(),
    ///     ).await?;
    ///     println!("Got {} markets", page.data.len());
    ///     cursor = page.next_cursor.clone();
    ///     if cursor.is_none() { break; }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
/// # Errors
///
/// If the request fails, the API returns a non-success status, or the body does not
/// deserialize into the expected shape.
pub async fn fetch_keyset<T>(
        client: &Client,
        web_service_request: &WebserviceRequest,
        cursor: Option<&str>,
    ) -> crate::Result<T>
    where
        T: for<'a> serde::Deserialize<'a> + KeysetApiResponse,
    {
        let callable_url = web_service_request.get_keyset_url(cursor);

        log::debug!(
            "Calling keyset method: {} on url: {}",
            web_service_request.method,
            callable_url,
        );

        const MAX_RETRIES: u32 = 3;
        let mut attempt = 0u32;

        loop {
            let request = match web_service_request.method {
                Method::GET => client.get(&callable_url),
                Method::POST => {
                    let req = client.post(&callable_url);
                    if let Some(body) = web_service_request.get_body() {
                        req.body(body)
                    } else {
                        req
                    }
                }
                _ => {
                    return Err(crate::ClobError::Validation(
                        crate::ValidationError::InvalidParameter {
                            parameter: "method".to_string(),
                            reason: format!("Unsupported method: {}", web_service_request.method),
                        },
                    ));
                }
            };

            let result: crate::Result<T> = async {
                let response = request
                    .send()
                    .await
                    .map_err(|e| crate::HttpError::from_reqwest(e, &callable_url))?;

                let response_text = handle_api_response(response, &callable_url).await?;

                let ws_response: T = serde_json::from_str(&response_text).map_err(|e| {
                    crate::ClobError::from(crate::SerializationError::JsonDeserialize {
                        message: e.to_string(),
                        raw_response: response_text.clone(),
                    })
                })?;

                log::debug!("Keyset page: next_cursor={:?}", ws_response.next_cursor());

                Ok(ws_response)
            }
            .await;

            match result {
                Ok(v) => return Ok(v),
                Err(e) if e.is_retryable() && attempt < MAX_RETRIES => {
                    attempt += 1;
                    let delay = e
                        .retry_after()
                        .unwrap_or(std::time::Duration::from_secs(2));
                    log::warn!(
                        "Transient error on attempt {}/{} for {}: {}. Retrying after {:?}",
                        attempt,
                        MAX_RETRIES,
                        callable_url,
                        e,
                        delay
                    );
                    tokio::time::sleep(delay).await;
                }
                Err(e) => return Err(e),
            }
        }
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::Method;

    fn keyset_req(url: &str, args: Vec<(String, String)>) -> WebserviceRequest {
        WebserviceRequest {
            api: "https://gamma-api.polymarket.com".to_string(),
            url: url.to_string(),
            method: Method::GET,
            with_pagination: false,
            args,
            body: None,
        }
    }

    #[test]
    fn test_get_keyset_url_no_cursor() {
        let url = keyset_req("/markets/keyset", vec![]).get_keyset_url(None);
        assert_eq!(url, "https://gamma-api.polymarket.com/markets/keyset?limit=100");
    }

    #[test]
    fn test_get_keyset_url_with_cursor() {
        let url = keyset_req("/markets/keyset", vec![]).get_keyset_url(Some("cursor_abc"));
        assert!(url.contains("after_cursor=cursor_abc"), "url={}", url);
        assert!(url.contains("limit=100"), "url={}", url);
    }

    #[test]
    fn test_get_keyset_url_empty_cursor_skipped() {
        let url = keyset_req("/markets/keyset", vec![]).get_keyset_url(Some(""));
        assert!(!url.contains("after_cursor"), "empty cursor should not appear: {}", url);
    }

    #[test]
    fn test_get_keyset_url_with_extra_args() {
        let url = keyset_req(
            "/markets/keyset",
            vec![("closed".to_string(), "false".to_string())],
        )
        .get_keyset_url(None);
        assert!(url.contains("closed=false"), "url={}", url);
        assert!(url.contains("limit=100"), "url={}", url);
    }

    #[test]
    fn test_get_keyset_url_with_filters_and_cursor() {
        let url = keyset_req(
            "/events/keyset",
            vec![
                ("closed".to_string(), "false".to_string()),
                ("active".to_string(), "true".to_string()),
            ],
        )
        .get_keyset_url(Some("cur123"));
        assert!(url.contains("after_cursor=cur123"), "url={}", url);
        assert!(url.contains("closed=false"), "url={}", url);
        assert!(url.contains("active=true"), "url={}", url);
    }

    #[test]
    fn test_get_keyset_url_no_duplicate_limit_when_limit_in_args() {
        let url = keyset_req(
            "/markets/keyset",
            vec![("limit".to_string(), "50".to_string())],
        )
        .get_keyset_url(None);
        let count = url.matches("limit=").count();
        assert_eq!(count, 1, "limit should appear exactly once: {}", url);
        assert!(url.contains("limit=50"), "url={}", url);
    }
}
