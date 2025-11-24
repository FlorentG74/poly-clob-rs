//! Example: Fetch active markets from Polymarket
//!
//! This example demonstrates how to query active markets using the Polymarket CLOB API.
//!
//! Run with:
//! ```bash
//! cargo run --example fetch_markets
//! ```

use poly_clob_rs::{ApiResponse, MarketsResponse, WebserviceRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    println!("Fetching active markets from Polymarket...\n");

    // Create a request for active markets
    let mut request = WebserviceRequest::new_markets_ws_request();
    request.with_active_only();

    // Build the URL
    let url = request.get_callable_url(0); // offset = 0
    println!("Request URL: {}\n", url);

    // Make the HTTP request
    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;

    // Parse the response
    let markets: MarketsResponse = response.json().await?;
    let count = markets.nb_results();

    println!("Found {} markets:\n", count);

    // Display first 10 markets
    for (i, market) in markets.iter().take(10).enumerate() {
        println!("{}. {}", i + 1, market.question.as_deref().unwrap_or("N/A"));
        println!("   Slug: {}", market.slug.as_deref().unwrap_or("N/A"));
        println!("   Active: {}", market.active.unwrap_or(false));

        if let Some(volume) = &market.volume {
            if let Ok(vol) = volume.parse::<f64>() {
                println!("   Volume: ${:.2}", vol);
            }
        }

        if let (Some(bid), Some(ask)) = (&market.best_bid, &market.best_ask) {
            println!("   Best Bid: {}, Best Ask: {}", bid, ask);
        }
        println!();
    }

    if count > 10 {
        println!("... and {} more markets", count - 10);
    }

    Ok(())
}
