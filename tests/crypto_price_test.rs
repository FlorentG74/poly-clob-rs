//! Integration test for crypto price API with event series.

use chrono::Utc;
use poly_clob_rs::api::crypto_price_requests::CryptoPriceRequest;
use poly_clob_rs::api::event_requests::SeriesEventsRequest;

#[tokio::test]
async fn test_fetch_btc_hourly_open_price() {
    let events = SeriesEventsRequest::builder()
        .series_slug("btc-up-or-down-hourly")
        .build()
        .execute()
        .await
        .expect("Failed to fetch series events");

    let now = Utc::now();
    let current = events
        .iter()
        .find(|e| e.end_date > now)
        .expect("No active event found");

    // Derive event duration from first consecutive pair with a positive gap.
    // Falls back to 3600s (1 hour) when only one event is returned (e.g. end-of-series).
    let duration_secs = events
        .windows(2)
        .map(|w| (w[1].end_date - w[0].end_date).num_seconds())
        .find(|&d| d > 0)
        .unwrap_or(3600);

    let start_ts = current.end_date.timestamp() - duration_secs;

    println!("[btc-up-or-down-hourly] slug={} start_ts={}", current.slug, start_ts);

    let crypto_price = CryptoPriceRequest::builder()
        .symbol("BTC")
        .event_start_time(start_ts)
        .variant("hourly")
        .build()
        .execute()
        .await
        .expect("Failed to fetch crypto price");

    println!("  Open Price:  {:?}", crypto_price.open_price);
    println!("  Close Price: {:?}", crypto_price.close_price);
    println!("  Completed:   {}", crypto_price.completed);
    println!("  Incomplete:  {}", crypto_price.incomplete);

    assert!(crypto_price.has_open_price(), "Should have open price available");
    println!("BTC strike for {}: ${:.2}", current.slug, crypto_price.open_price.unwrap());
}
