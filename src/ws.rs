//! WebSocket transport: a shared HTTP/1.1 client and a connect helper.
//!
//! Every Polymarket/Binance live feed upgrades an HTTP request to a WebSocket.
//! Centralising the client and the upgrade here keeps the HTTP/1.1 requirement
//! in one place and stops each cache from constructing its own client.

use std::sync::OnceLock;
use std::time::Duration;

use reqwest::Client;
use reqwest_websocket::{Upgrade, WebSocket};

/// Default delay before reconnecting a dropped WebSocket, in seconds.
pub const RECONNECT_DELAY_SECS: u64 = 5;

/// Default interval between application-level WebSocket pings, in seconds.
pub const PING_INTERVAL_SECS: u64 = 10;

static WS_HTTP_CLIENT: OnceLock<Client> = OnceLock::new();

/// Shared HTTP/1.1 client used for every WebSocket upgrade.
///
/// Forces HTTP/1.1 via `http1_only`: the `Upgrade: websocket` handshake is
/// HTTP/1.1-only, so with the reqwest `http2` feature enabled ALPN would
/// otherwise negotiate h2 and the upgrade would fail. Sets no overall request
/// timeout — WS streams are long-lived and a client-wide `.timeout()` would
/// abort them — only a connect timeout applies.
///
/// Safe to share across feeds: each [`connect`] takes its connection out of the
/// pool on upgrade, so the streams hold no shared state, and the pool is keyed
/// per-host.
pub fn ws_client() -> &'static Client {
    WS_HTTP_CLIENT.get_or_init(|| {
        Client::builder()
            .http1_only()
            .connect_timeout(Duration::from_secs(10))
            .tcp_keepalive(Some(Duration::from_secs(60)))
            .tcp_nodelay(true)
            .build()
            .expect("failed to build WebSocket HTTP client")
    })
}

/// Open a WebSocket connection to `url` using the shared HTTP/1.1 client.
pub async fn connect(url: &str) -> Result<WebSocket, reqwest_websocket::Error> {
    let response = ws_client().get(url).upgrade().send().await?;
    response.into_websocket().await
}
