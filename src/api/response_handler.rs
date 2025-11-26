//! HTTP response handling utilities.
//!
//! Provides consistent error handling and rate limiting for API responses.

use reqwest::Response;

/// Handle HTTP API responses with consistent error handling and rate limiting.
///
/// Returns `Ok(String)` with response body on success, or `Err(String)` on failure.
///
/// # Arguments
///
/// * `response` - The HTTP response to handle
/// * `url` - The URL that was called (for logging purposes)
///
/// # Behavior
///
/// - **200 OK**: Returns the response body as a string
/// - **400 Bad Request**: Logs error with response body, returns error
/// - **401 Unauthorized**: Logs authentication error, returns error
/// - **429 Too Many Requests**: Logs warning, sleeps for 5 seconds, returns error
/// - **Other**: Logs unexpected error, returns error
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
pub async fn handle_api_response(response: Response, url: &str) -> Result<String, String> {
    match response.status() {
        reqwest::StatusCode::OK => {
            let text = response
                .text()
                .await
                .map_err(|e| format!("Failed to read response body: {}", e))?;
            log::info!("API response: {}", text);
            Ok(text)
        }
        reqwest::StatusCode::BAD_REQUEST => {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            log::error!("Bad request for {}: {}", url, error_text);
            Err(format!("Bad request: {}", error_text))
        }
        reqwest::StatusCode::UNAUTHORIZED => {
            log::error!("Authentication failed for request {}", url);
            Err("Unauthorized".to_string())
        }
        reqwest::StatusCode::TOO_MANY_REQUESTS => {
            log::warn!("Rate limit reached - pausing for 5 secs");
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            Err("Rate limited".to_string())
        }
        other => {
            log::error!(
                "Unexpected error in service call: {:?}; url: {}",
                other,
                url
            );
            Err(format!("HTTP {}", other))
        }
    }
}
