//! Integration test for crypto price API with event series.

use chrono::DateTime;
use poly_clob_rs::api::crypto_price_requests::CryptoPriceRequest;
use poly_clob_rs::api::event_requests::EventSeriesRequest;

/// Test fetching the open price for the current BTC hourly event.
///
/// This test:
/// 1. Fetches the BTC hourly event series
/// 2. Gets the first (current) event from the series
/// 3. Parses the event start_date to get the timestamp
/// 4. Calls the crypto price API to get the open price
#[tokio::test]
async fn test_fetch_btc_hourly_open_price() {
    // Fetch the BTC hourly event series
    let series = EventSeriesRequest::builder()
        .slug("btc-up-or-down-hourly")
        .build()
        .execute()
        .await
        .expect("Failed to fetch BTC hourly event series");

    println!("Series: {} - {}", series.slug, series.title);
    println!("Number of events: {}", series.events.len());

    // Get the first event (current/upcoming)
    let event = series
        .events
        .first()
        .expect("No events in BTC hourly series");

    println!(
        "Current event: {} (active: {:?}, closed: {:?})",
        event.slug, event.active, event.closed
    );

    // Parse the start_date to get the Unix timestamp
    let start_date_str = event
        .start_date
        .as_ref()
        .expect("Event has no start_date");
    println!("Event start_date: {}", start_date_str);

    let start_datetime = DateTime::parse_from_rfc3339(start_date_str)
        .expect("Failed to parse start_date as RFC3339");
    let event_start_time = start_datetime.timestamp();
    println!("Event start timestamp: {}", event_start_time);

    // Fetch the crypto price for this event
    let crypto_price = CryptoPriceRequest::builder()
        .symbol("BTC")
        .event_start_time(event_start_time)
        .build()
        .execute()
        .await
        .expect("Failed to fetch crypto price");

    println!("\nCrypto Price Response:");
    println!("  Open Price:  ${:.2}", crypto_price.open_price);
    println!("  Close Price: ${:.2}", crypto_price.close_price);
    println!("  Completed:   {}", crypto_price.completed);
    println!("  Incomplete:  {}", crypto_price.incomplete);

    // Verify we got a valid open price
    assert!(
        crypto_price.open_price > 0.0,
        "Open price should be positive"
    );
    assert!(
        crypto_price.has_open_price(),
        "Should have open price available"
    );

    println!(
        "\nBTC strike price for event {}: ${:.2}",
        event.slug, crypto_price.open_price
    );
}
