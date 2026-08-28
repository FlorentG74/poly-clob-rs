//! Example: Fetch crypto opening and closing prices for an event
//!
//! This example demonstrates fetching crypto prices for strike setting
//! and settlement resolution of up/down events.
//!
//! Run with: cargo run --example `fetch_crypto_price`

use poly_clob_rs::api::crypto_price_requests::CryptoPriceRequest;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Install the crate configuration (network policy, credentials) from .env / env vars.
    poly_clob_rs::config::init_from_env();

    // Example: the most recent completed ETH up/down 15m event window.
    // (The API only serves timestamps from the last ~30 days.)
    let symbol = "ETH";
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs() as i64;
    let event_start_time: i64 = (now / 900) * 900 - 1800;

    println!("Fetching crypto price for {} at timestamp {}", symbol, event_start_time);

    let response = CryptoPriceRequest::builder()
        .symbol(symbol)
        .event_start_time(event_start_time)
        .variant("fifteen") // 15m event
        .build()
        .execute()
        .await?;

    println!("\nCrypto Price Response:");
    println!("  Open Price:  {:?}", response.open_price);
    println!("  Close Price: {:?}", response.close_price);
    println!("  Completed:   {}", response.completed);
    println!("  Incomplete:  {}", response.incomplete);
    println!("  Timestamp:   {}", response.timestamp);

    // Check if valid for different use cases
    if let Some(open) = response.open_price {
        println!("\n  Strike price available: ${:.2}", open);
    }

    if response.is_valid_for_settlement() {
        if let (Some(close), Some(open)) = (response.close_price, response.open_price) {
            let direction = if close > open { "UP" } else { "DOWN" };
            println!("  Settlement price available: ${:.2}", close);
            println!("  Outcome: {} (close {} open)", direction, if close > open { ">" } else { "<=" });
        }
    } else {
        println!("\n  Event not yet completed - settlement price not final");
    }

    Ok(())
}
