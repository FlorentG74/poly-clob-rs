//! Live network check for the `DNS_RESOLVER` override.
//!
//! Ignored by default: needs outbound network, and it is meaningful only where the
//! system resolver disagrees with the configured one (e.g. an ISP resolver that
//! answers Polymarket hostnames with loopback).
//!
//! Run with:
//! ```text
//! cargo test -p poly-clob-rs --test dns_override_test -- --ignored --nocapture
//! ```

use std::time::Instant;

use poly_clob_rs::api::clob_endpoints::GAMMA_API;
use poly_clob_rs::api::dns::configured_resolver;
use poly_clob_rs::api::http_client::get_http_client;
use poly_clob_rs::config::{self, Config};
use reqwest::dns::{Name, Resolve};

/// Installs a config that resolves via 1.1.1.1, regardless of the host's `.env`.
///
/// The first install wins, so both tests in this binary share it.
fn init_config_with_resolver() {
    config::init(Config {
        dns_resolver: vec!["1.1.1.1".parse().unwrap()],
        ..Config::default()
    });
}

/// The override must resolve Polymarket names even when the system resolver lies.
#[tokio::test]
#[ignore = "requires network access"]
async fn dns_override_reaches_gamma_api() {
    init_config_with_resolver();

    let url = format!("{GAMMA_API}/events?series_slug=btc-up-or-down-15m&limit=1");
    let client = get_http_client(Some(GAMMA_API));

    let response = client
        .get(&url)
        .send()
        .await
        .expect("gamma-api reachable via the configured resolver");

    println!("status = {}", response.status());
    assert!(
        response.status().is_success(),
        "expected 2xx from gamma-api, got {}",
        response.status()
    );
}

/// Repeat lookups must be served from hickory's cache, not re-queried on the wire.
#[tokio::test]
#[ignore = "requires network access"]
async fn repeat_lookups_are_cached() {
    init_config_with_resolver();

    let resolver = configured_resolver().expect("a resolver is configured");
    let lookup = |host: &str| {
        let name: Name = host.parse().unwrap();
        resolver.resolve(name)
    };

    // A host no other test in this binary resolves, so the first lookup is genuinely cold.
    let host = "data-api.polymarket.com";

    let cold_start = Instant::now();
    let cold: Vec<_> = lookup(host).await.unwrap().collect();
    let cold_elapsed = cold_start.elapsed();

    let warm_start = Instant::now();
    let warm: Vec<_> = lookup(host).await.unwrap().collect();
    let warm_elapsed = warm_start.elapsed();

    println!("cold: {cold_elapsed:?} -> {cold:?}");
    println!("warm: {warm_elapsed:?} -> {warm:?}");

    assert!(!cold.is_empty(), "cold lookup returned no addresses");
    assert_eq!(cold, warm, "cached answer must match the queried one");
    assert!(
        warm_elapsed * 10 < cold_elapsed,
        "second lookup ({warm_elapsed:?}) should be served from cache, \
         far faster than the first ({cold_elapsed:?})"
    );
}
