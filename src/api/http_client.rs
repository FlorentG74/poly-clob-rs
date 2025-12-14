//! Shared HTTP client for all API requests.
//!
//! This module provides a global HTTP client singleton for connection pooling and resource efficiency.

use reqwest::Client;
use std::sync::OnceLock;
use std::time::Duration;

/// Global HTTP client singleton.
///
/// Uses connection pooling for efficient resource usage across all API requests.
static HTTP_CLIENT: OnceLock<Client> = OnceLock::new();

/// Returns a shared HTTP client instance.
///
/// The client is lazily initialized on first use with:
/// - 10 max idle connections per host
/// - 30 second request timeout
///
/// # Panics
///
/// Panics if the client cannot be created (extremely unlikely in practice).
///
/// # Example
///
/// ```rust
/// use poly_clob_rs::api::http_client::get_http_client;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = get_http_client();
/// let response = client.get("https://api.example.com").send().await?;
/// # Ok(())
/// # }
/// ```
pub fn get_http_client() -> &'static Client {
    HTTP_CLIENT.get_or_init(|| {
        Client::builder()
            .pool_max_idle_per_host(10)
            .timeout(Duration::from_secs(30))
            .build()
            .expect("failed to create HTTP client")
    })
}
