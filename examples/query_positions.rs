//! Example: Query user positions
//!
//! Fetches open positions for a user address via the Data API.
//!
//! Run with:
//! ```bash
//! cargo run --example query_positions [ethereum_address]
//! ```
//! or set `POLY_ADDRESS` in the environment / `.env`.

use poly_clob_rs::api::http_client::get_http_client;
use poly_clob_rs::{ApiResponse, PositionsResponse, WebserviceRequest};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Install the crate configuration (network policy, credentials) from .env / env vars.
    poly_clob_rs::config::init_from_env();

    // Get user address from command line argument or environment variable
    let user_address = env::args()
        .nth(1)
        .or_else(|| env::var("POLY_ADDRESS").ok())
        .expect("Please provide user address as argument or set POLY_ADDRESS env var");

    println!("Fetching positions for address: {}\n", user_address);

    let request = WebserviceRequest::new_positions_ws_request(&user_address);
    let client = get_http_client(Some(&request.api));

    let (_next_offset, positions): (i32, PositionsResponse) =
        WebserviceRequest::fetch_batch(client, &request, 0).await?;
    let count = positions.nb_results();

    if count == 0 {
        println!("No positions found for this address.");
        return Ok(());
    }

    println!("Found {} positions:\n", count);

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
