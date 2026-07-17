//! Example: Fetch token prices from Polymarket
//!
//! Fetches an active market, then queries real-time buy/sell prices for its
//! outcome tokens via the CLOB API.
//!
//! Run with:
//! ```bash
//! cargo run --example fetch_prices
//! ```

use poly_clob_rs::api::http_client::get_http_client;
use poly_clob_rs::api::market_requests::MarketsRequest;
use poly_clob_rs::{PolymarketPricesResponse, WebserviceRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Install the crate configuration (network policy, credentials) from .env / env vars.
    poly_clob_rs::config::init_from_env();

    // Grab a live market so we have real token IDs to price.
    let page = MarketsRequest::builder()
        .closed(Some(false))
        .limit(10)
        .build()
        .execute()
        .await?;

    let market = page
        .data
        .iter()
        .find(|m| !m.clob_token_ids.is_empty())
        .ok_or("no active market with CLOB token IDs found")?;

    println!(
        "Market: {}\n",
        market.question.as_deref().unwrap_or("unknown")
    );

    let token_ids = market.clob_token_ids.clone();
    println!("Fetching prices for {} tokens...\n", token_ids.len());

    // POST /prices with a JSON body listing (token_id, side) pairs
    let request = WebserviceRequest::new_polymarket_price_request(&token_ids);
    let client = get_http_client(Some(&request.api));

    let prices: PolymarketPricesResponse = WebserviceRequest::fetch_one(client, &request).await?;

    println!("Received prices for {} tokens:\n", prices.len());

    for (token_id, price) in prices.iter() {
        println!("Token ID: {}", token_id);
        println!("  Buy Price:  {}", price.buy.as_deref().unwrap_or("N/A"));
        println!("  Sell Price: {}", price.sell.as_deref().unwrap_or("N/A"));
        println!();
    }

    Ok(())
}
