//! Example: Fetch crypto opening and closing prices for an event
//!
//! This example demonstrates fetching crypto prices for strike setting
//! and settlement resolution of up/down events.
//!
//! Run with: cargo run --example fetch_crypto_price

use poly_clob_rs::api::crypto_price_requests::CryptoPriceRequest;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Example: ETH up/down 15m event starting at a specific timestamp
    let symbol = "ETH";
    let event_start_time: i64 = 1738023000; // Unix timestamp in seconds

    println!("Fetching crypto price for {} at timestamp {}", symbol, event_start_time);

    let response = CryptoPriceRequest::builder()
        .symbol(symbol)
        .event_start_time(event_start_time)
        .build()
        .execute()
        .await?;

    println!("\nCrypto Price Response:");
    println!("  Open Price:  ${:.2}", response.open_price);
    println!("  Close Price: ${:.2}", response.close_price);
    println!("  Completed:   {}", response.completed);
    println!("  Incomplete:  {}", response.incomplete);
    println!("  Cached:      {}", response.cached);
    println!("  Timestamp:   {}", response.timestamp);

    // Check if valid for different use cases
    if response.has_open_price() {
        println!("\n  Strike price available: ${:.2}", response.open_price);
    }

    if response.is_valid_for_settlement() {
        println!("  Settlement price available: ${:.2}", response.close_price);
        let direction = if response.close_price > response.open_price {
            "UP"
        } else {
            "DOWN"
        };
        println!("  Outcome: {} (close {} open)", direction,
            if response.close_price > response.open_price { ">" } else { "<=" });
    } else {
        println!("\n  Event not yet completed - settlement price not final");
    }

    Ok(())
}
