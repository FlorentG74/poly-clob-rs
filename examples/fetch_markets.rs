//! Example: Fetch active markets from Polymarket
//!
//! This example demonstrates how to query active markets using the Polymarket CLOB API.
//!
//! Run with:
//! ```bash
//! cargo run --example fetch_markets
//! ```

use poly_clob_rs::api::market_requests::MarketsRequest;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Fetching active markets from Polymarket...\n");

    // Create a request for active markets
    let request = MarketsRequest::builder()
        .closed(false)
        .limit(100)
        .build();

    println!("Fetching markets...\n");

    // Execute the request
    let markets = request.execute().await?;
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
