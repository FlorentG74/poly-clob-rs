//! Example: Fetch events from Polymarket
//!
//! This example demonstrates how to query event data (collections of related markets).
//!
//! Run with:
//! ```bash
//! cargo run --example fetch_events
//! ```

use poly_clob_rs::api::http_client::get_http_client;
use poly_clob_rs::{models::KeysetEventsResponse, WebserviceRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Install the crate configuration (network policy, credentials) from .env / env vars.
    poly_clob_rs::config::init_from_env();

    println!("Fetching events from Polymarket...\n");

    // Example: Fetch a specific event by ID (replace with actual event ID)
    let event_id = "21742";

    let request = WebserviceRequest::new_event_by_id_request(event_id);
    let url = request.get_keyset_url(None);

    println!("Request URL: {}\n", url);

    let client = get_http_client(Some(&url));
    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        println!("Error: {}", response.status());
        println!("Note: Replace the event_id with a valid Polymarket event ID");
        return Ok(());
    }

    let page: KeysetEventsResponse = response.json().await?;
    let count = page.data.len();

    println!("Found {} event(s):\n", count);

    for (i, event) in page.data.iter().enumerate() {
        println!("{}. {}", i + 1, event.title);
        println!("   Ticker: {}", event.ticker);
        println!("   Slug: {}", event.slug);

        if !event.description.is_empty() {
            println!("   Description: {}", event.description);
        }

        println!("   Start Date: {}", event.start_date);
        println!("   End Date: {}", event.end_date);
        println!("   Active: {}", event.active);
        println!("   Markets: {}", event.markets.len());
        println!();
    }

    Ok(())
}
