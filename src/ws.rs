//! WebSocket transport: a shared HTTP/1.1 client and a connect helper.
//!
//! Every Polymarket/Binance live feed upgrades an HTTP request to a WebSocket.
//! Centralising the client and the upgrade here keeps the HTTP/1.1 requirement
//! in one place and stops each cache from constructing its own client.

use std::sync::OnceLock;
use std::time::Duration;

use reqwest::Client;
use reqwest_websocket::{Upgrade, WebSocket};

use crate::api::http_client::{apply_polymarket_network_policy, is_polymarket_url};

/// Default delay before reconnecting a dropped WebSocket, in seconds.
pub const RECONNECT_DELAY_SECS: u64 = 5;

/// Default interval between application-level WebSocket pings, in seconds.
pub const PING_INTERVAL_SECS: u64 = 10;

static WS_HTTP_CLIENT: OnceLock<Client> = OnceLock::new();
static WS_POLYMARKET_CLIENT: OnceLock<Client> = OnceLock::new();

/// Builds an HTTP/1.1 client suitable for WebSocket upgrades.
///
/// Forces HTTP/1.1 via `http1_only`: the `Upgrade: websocket` handshake is
/// HTTP/1.1-only, so with the reqwest `http2` feature enabled ALPN would
/// otherwise negotiate h2 and the upgrade would fail. Sets no overall request
/// timeout — WS streams are long-lived and a client-wide `.timeout()` would
/// abort them — only a connect timeout applies.
fn build_ws_client(polymarket: bool) -> Client {
    let builder = Client::builder()
        .http1_only()
        .connect_timeout(Duration::from_secs(10))
        .tcp_keepalive(Some(Duration::from_secs(60)))
        .tcp_nodelay(true);

    let builder = if polymarket {
        apply_polymarket_network_policy(builder)
    } else {
        builder
    };

    builder
        .build()
        .expect("failed to build WebSocket HTTP client")
}

/// Shared HTTP/1.1 client used for WebSocket upgrades on default routing.
///
/// Safe to share across feeds: each [`connect`] takes its connection out of the
/// pool on upgrade, so the streams hold no shared state, and the pool is keyed
/// per-host.
pub fn ws_client() -> &'static Client {
    WS_HTTP_CLIENT.get_or_init(|| build_ws_client(false))
}

/// Shared HTTP/1.1 client for Polymarket feeds, with split tunnel and DNS override.
pub fn ws_polymarket_client() -> &'static Client {
    WS_POLYMARKET_CLIENT.get_or_init(|| build_ws_client(true))
}

/// Returns the WebSocket client appropriate for `url`.
///
/// Polymarket feeds get the split tunnel and DNS override; everything else (Binance)
/// keeps default routing and the system resolver, paying no VPN latency.
#[must_use]
pub fn ws_client_for(url: &str) -> &'static Client {
    if is_polymarket_url(url) {
        ws_polymarket_client()
    } else {
        ws_client()
    }
}

/// Open a WebSocket connection to `url`, applying Polymarket policy where it applies.
///
/// # Errors
///
/// If the URL is not a valid websocket endpoint, or the upgrade handshake fails.
pub async fn connect(url: &str) -> Result<WebSocket, reqwest_websocket::Error> {
    let response = ws_client_for(url).get(url).upgrade().send().await?;
    response.into_websocket().await
}
