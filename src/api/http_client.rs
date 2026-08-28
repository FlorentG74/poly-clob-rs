//! Shared HTTP client for all API requests.
//!
//! This module provides HTTP client singletons for efficient connection pooling and
//! optional split tunneling support for Polymarket requests.
//!
//! # Split Tunneling
//!
//! Configure split tunneling by setting the `SPLIT_TUNNEL_IFACE` environment variable
//! to the network interface name (e.g. `wireguard-es`). Every request to a
//! `polymarket.com` host is then bound to that interface; all other traffic (Binance,
//! ESPN, ...) keeps default system routing, so unrelated feeds pay no VPN latency.
//!
//! Binding applies to the socket only — name resolution still goes through the system
//! resolver. If a Polymarket host resolves to the wrong address (e.g. a DNS-poisoning
//! resolver returning loopback), binding the socket cannot rescue it. Set `DNS_RESOLVER`
//! to resolve Polymarket hostnames independently of the system resolver; see
//! [`crate::api::dns`].

use reqwest::{Client, ClientBuilder};
use std::sync::OnceLock;
use std::time::Duration;

use crate::api::dns::configured_resolver;

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
use crate::config::get_config;

/// Default HTTP client singleton for non-Polymarket requests.
///
/// Uses connection pooling for efficient resource usage and default system routing.
static HTTP_CLIENT: OnceLock<Client> = OnceLock::new();

/// Polymarket HTTP client singleton with optional split tunneling.
///
/// Uses connection pooling and an optional network interface binding configured via
/// the `SPLIT_TUNNEL_IFACE` environment variable.
static SPLIT_TUNNEL_HTTP_CLIENT: OnceLock<Client> = OnceLock::new();

/// Extracts the host from `url`, tolerating a missing scheme, userinfo and port.
fn host_of(url: &str) -> &str {
    let after_scheme = url.split_once("://").map_or(url, |(_, rest)| rest);
    let authority = after_scheme
        .split(['/', '?', '#'])
        .next()
        .unwrap_or(after_scheme);
    let host = authority.rsplit_once('@').map_or(authority, |(_, h)| h);
    host.split_once(':').map_or(host, |(h, _)| h)
}

/// True when `url` points at a Polymarket host.
///
/// Matches `polymarket.com` and any subdomain of it, which covers every endpoint the
/// bot talks to: `clob`, `gamma-api`, `data-api`, `ws-subscriptions-clob`,
/// `ws-live-data`, `relayer-v2` and `bridge`.
#[must_use]
pub fn is_polymarket_url(url: &str) -> bool {
    let host = host_of(url).to_ascii_lowercase();
    host == "polymarket.com" || host.ends_with(".polymarket.com")
}

/// Applies the Polymarket network policy: split-tunnel binding plus DNS override.
///
/// Shared with the WebSocket transport in [`crate::ws`] so HTTP and WS feeds resolve
/// and route identically.
pub fn apply_polymarket_network_policy(builder: ClientBuilder) -> ClientBuilder {
    let builder = apply_split_tunnel(builder);

    match configured_resolver() {
        Some(resolver) => builder.dns_resolver(resolver),
        None => builder,
    }
}

/// Binds `builder` to the configured split-tunnel interface when one is configured.
///
/// See [`Config::split_tunnel_iface`](crate::config::Config::split_tunnel_iface).
pub fn apply_split_tunnel(builder: ClientBuilder) -> ClientBuilder {
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
    if let Some(iface) = get_config().split_tunnel_iface.as_deref() {
        return builder.interface(iface);
    }

    builder
}

/// Returns a shared HTTP client instance appropriate for the given endpoint.
///
/// Selects the client by host:
/// - Requests to any `polymarket.com` host use the split-tunneling-capable client
/// - All other requests use the default client
///
/// # Arguments
///
/// * `endpoint` - Optional API endpoint URL. Pass the request URL, or the API base
///   constant (`CLOB_API`, `GAMMA_API`, `DATA_API`, `POLYMARKET_API`) when the full
///   URL is assembled later. `None` always selects the default client.
///
/// # Environment Variables
///
/// * `SPLIT_TUNNEL_IFACE` - Network interface name used for Polymarket requests
///   (e.g. `wireguard-es`). If unset, Polymarket requests use default routing.
///
/// # One runtime per process
///
/// Both clients are process-global and keep idle pooled connections, but a hyper
/// connection's dispatch task lives on the Tokio runtime that created it. Sharing these
/// clients across **multiple** runtimes therefore hands out connections whose runtime has
/// already been dropped, and the request fails with
/// `client error (SendRequest): dispatch task is gone: runtime dropped the dispatch task`.
///
/// Every binary here runs a single runtime, so this only bites test harnesses: a
/// `#[tokio::test]` builds and drops a runtime per test. Tests that make requests must
/// share one long-lived runtime — see `core_services::test_runtime`.
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
#[must_use]
pub fn get_http_client(endpoint: Option<&str>) -> &'static Client {
    match endpoint {
        Some(url) if is_polymarket_url(url) => get_split_tunnel_http_client(),
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

fn get_split_tunnel_http_client() -> &'static Client {
    SPLIT_TUNNEL_HTTP_CLIENT.get_or_init(|| {
        let builder = Client::builder()
            .pool_max_idle_per_host(20)  // Increased from 10 to handle 8 concurrent strategies
            .pool_idle_timeout(Some(Duration::from_secs(90)))  // Keep idle connections alive longer
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))  // Explicit connect timeout
            .tcp_nodelay(true)  // Disable Nagle's algorithm for lower latency
            .tcp_keepalive(Some(Duration::from_secs(60)));  // Enable TCP keepalive

        apply_polymarket_network_policy(builder)
            .build()
            .expect("failed to create Polymarket HTTP client")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_every_polymarket_endpoint() {
        for url in [
            "https://clob.polymarket.com",
            "https://gamma-api.polymarket.com/events?series_slug=btc-up-or-down-15m",
            "https://data-api.polymarket.com/positions?limit=100",
            "https://polymarket.com/api/crypto-price",
            "wss://ws-subscriptions-clob.polymarket.com/ws/market",
            "wss://ws-live-data.polymarket.com",
            "https://relayer-v2.polymarket.com",
            "https://bridge.polymarket.com",
        ] {
            assert!(is_polymarket_url(url), "should match: {url}");
        }
    }

    #[test]
    fn rejects_non_polymarket_hosts() {
        for url in [
            "wss://stream.binance.com:9443/ws",
            "https://api.binance.com/api/v3/klines",
            "https://site.api.espn.com/apis/v2/sports",
            // Suffix must be a domain boundary, not a substring.
            "https://notpolymarket.com",
            "https://polymarket.com.evil.example",
        ] {
            assert!(!is_polymarket_url(url), "should not match: {url}");
        }
    }

    #[test]
    fn host_parsing_handles_port_and_path() {
        assert_eq!(host_of("wss://stream.binance.com:9443/ws"), "stream.binance.com");
        assert_eq!(host_of("https://clob.polymarket.com/order"), "clob.polymarket.com");
        assert_eq!(host_of("https://user:pw@data-api.polymarket.com/x"), "data-api.polymarket.com");
    }
}
