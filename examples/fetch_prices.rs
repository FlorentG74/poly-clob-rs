//! Example: Fetch token prices from Polymarket
//!
//! This example demonstrates how to query real-time prices for prediction market tokens.
//!
//! Run with:
//! ```bash
//! cargo run --example fetch_prices
//! ```

use poly_clob_rs::{PolymarketPricesResponse, WebserviceRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Example token IDs (replace with actual token IDs from markets)
    let token_ids = vec![
        "52114319501245915516055106046884209969926127482827954674443846427813813942700".to_string(),
        "48331043336612883890938759509493159234755048973500640148014422747788308965732".to_string(),
    ];

    println!("Fetching prices for {} tokens...\n", token_ids.len());

    // Create a price request
    let request = WebserviceRequest::new_polymarket_price_request(&token_ids);

    // Build the URL
    let url = request.get_callable_url(0);
    println!("Request URL: {}\n", url);

    // Make the HTTP request
    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;

    // Parse the response
    let prices: PolymarketPricesResponse = response.json().await?;

    println!("Received prices for {} tokens:\n", prices.len());

    for (token_id, price) in prices.iter() {
        println!("Token ID: {}", token_id);

        if let Some(buy) = &price.buy {
            println!("  Buy Price:  {}", buy);
        } else {
            println!("  Buy Price:  N/A");
        }

        if let Some(sell) = &price.sell {
            println!("  Sell Price: {}", sell);
        } else {
            println!("  Sell Price: N/A");
        }

        println!();
    }

    Ok(())
}
