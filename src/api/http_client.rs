//! Shared HTTP client for all API requests.
//!
//! This module provides HTTP client singletons for efficient connection pooling and
//! optional split tunneling support for CLOB API requests.
//!
//! # Split Tunneling
//!
//! Configure split tunneling by setting the `SPLIT_TUNNEL_IFACE` environment variable
//! to the network interface name (e.g., `wg0`, `eth1`). This routes
//! all CLOB API requests through the specified interface while other requests use the
//! default system routing.

use reqwest::Client;
use std::sync::OnceLock;
use std::time::Duration;

use crate::{CANCEL, POST_ORDER};

/// Default HTTP client singleton for non-CLOB requests.
///
/// Uses connection pooling for efficient resource usage and default system routing.
static HTTP_CLIENT: OnceLock<Client> = OnceLock::new();

/// CLOB API HTTP client singleton with optional split tunneling.
///
/// Uses connection pooling and an optional network interface binding configured via
/// the `SPLIT_TUNNEL_IFACE` environment variable.
static CLOB_HTTP_CLIENT: OnceLock<Client> = OnceLock::new();

/// Returns a shared HTTP client instance appropriate for the given endpoint.
///
/// Automatically selects the client based on the endpoint URL:
/// - Requests to `CLOB_API` use the split-tunneling-capable CLOB client
/// - All other requests use the default client
///
/// Both clients are lazily initialized on first use with:
/// - 10 max idle connections per host
/// - 30 second request timeout
///
/// # Arguments
///
/// * `endpoint` - Optional API endpoint URL. If present and starts with the CLOB API
///   base URL, the CLOB-specific client is used; otherwise the default client is used.
///
/// # Environment Variables
///
/// * `SPLIT_TUNNEL_IFACE` - Network interface name for split tunneling CLOB requests
///   (e.g., `wg0`). If not set, CLOB requests use default routing.
///   Only meaningful when endpoint is `CLOB_API`.
///
/// # Panics
///
/// Panics if either HTTP client cannot be created (extremely unlikely in practice).
///
/// # Example
///
/// ```rust
/// use poly_clob_rs::api::http_client::get_http_client;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = get_http_client(Some("https://clob.polymarket.com/order"));
/// let response = client.get("https://clob.polymarket.com/time").send().await?;
/// # Ok(())
/// # }
/// ```
pub fn get_http_client(request_path: Option<&str>) -> &'static Client {
    match request_path {
        Some(path) if path.eq(POST_ORDER) | path.eq(CANCEL) => get_clob_http_client(),
        _ => get_default_http_client(),
    }
}

fn get_default_http_client() -> &'static Client {
    HTTP_CLIENT.get_or_init(|| {
        Client::builder()
            .pool_max_idle_per_host(10)
            .timeout(Duration::from_secs(30))
            .build()
            .expect("failed to create default HTTP client")
    })
}

fn get_clob_http_client() -> &'static Client {
    CLOB_HTTP_CLIENT.get_or_init(|| {
        #[allow(unused_mut)] // Mutability needed only if split tunneling is configured
        let mut builder = Client::builder()
            .pool_max_idle_per_host(20)  // Increased from 10 to handle 8 concurrent strategies
            .pool_idle_timeout(Some(Duration::from_secs(90)))  // Keep idle connections alive longer
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))  // Explicit connect timeout
            .tcp_nodelay(true)  // Disable Nagle's algorithm for lower latency
            .tcp_keepalive(Some(Duration::from_secs(60)));  // Enable TCP keepalive

        // Configure split tunneling if SPLIT_TUNNEL_IFACE environment variable is set and we're on a supported platform
        #[cfg(any(
        target_os = "android",
        target_os = "fuchsia",
        target_os = "illumos",
        target_os = "ios",
        target_os = "linux",
        target_os = "macos",
        target_os = "solaris",
        target_os = "tvos",
        target_os = "visionos",
        target_os = "watchos",
        ))]
        if let Ok(iface) = std::env::var("SPLIT_TUNNEL_IFACE") {
            builder = builder.interface(&iface);
        }

        builder.build()
            .expect("failed to create CLOB HTTP client")
    })
}
