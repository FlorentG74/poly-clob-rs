//! Integration test for crypto price API with event series.

use poly_clob_rs::api::crypto_price_requests::CryptoPriceRequest;
use poly_clob_rs::api::event_requests::EventSeriesRequest;

#[tokio::test]
async fn test_fetch_btc_hourly_open_price() {
    let series = EventSeriesRequest::builder()
        .slug("btc-up-or-down-hourly")
        .build()
        .execute()
        .await
        .expect("Failed to fetch series");

    let event_slug = series
        .current_event()
        .map(|e| e.slug.clone())
        .expect("No active event found");

    let start_ts = series
        .current_event_start_ts()
        .expect("Could not determine event start timestamp");

    println!("[btc-up-or-down-hourly] slug={} start_ts={}", event_slug, start_ts);

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
    println!("BTC strike for {}: ${:.2}", event_slug, crypto_price.open_price.unwrap());
}
