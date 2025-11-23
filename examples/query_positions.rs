//! Example: Query user positions
//!
//! This example demonstrates how to fetch positions for a specific user address.
//!
//! Run with:
//! ```bash
//! cargo run --example query_positions
//! ```

use poly_clob_rs::{ApiResponse, PositionsResponse, WebserviceRequest};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    // Get user address from command line argument or environment variable
    let user_address = env::args()
        .nth(1)
        .or_else(|| env::var("POLY_ADDRESS").ok())
        .expect("Please provide user address as argument or set POLY_ADDRESS env var");

    println!("Fetching positions for address: {}\n", user_address);

    // Create a positions request
    let request = WebserviceRequest::new_positions_ws_request(&user_address);

    // Build the URL
    let url = request.get_callable_url(0);
    println!("Request URL: {}\n", url);

    // Make the HTTP request
    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;

    // Parse the response
    let positions: PositionsResponse = response.json().await?;
    let count = positions.nb_results();

    if count == 0 {
        println!("No positions found for this address.");
        return Ok(());
    }

    println!("Found {} positions:\n", count);

    // Display all positions
    for (i, position) in positions.iter().enumerate() {
        println!("{}. {} - {}", i + 1, position.title, position.outcome);
        println!("   Condition ID: {}", position.condition_id);
        println!("   Size: {}", position.size);
        println!("   Average Price: ${:.4}", position.avg_price);
        println!("   Current Price: ${:.4}", position.cur_price);
        println!("   Total Bought: ${:.2}", position.total_bought);
        println!("   Cash P&L: ${:.2}", position.cash_pnl);
        println!("   Percent P&L: {:.2}%", position.percent_pnl);

        println!();
    }

    Ok(())
}
