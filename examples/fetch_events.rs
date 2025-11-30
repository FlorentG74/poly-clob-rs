//! Example: Fetch events from Polymarket
//!
//! This example demonstrates how to query event data (collections of related markets).
//!
//! Run with:
//! ```bash
//! cargo run --example fetch_events
//! ```

use poly_clob_rs::{ApiResponse, EventResponse, WebserviceRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Fetching events from Polymarket...\n");

    // You would typically get a specific event by ID
    // For demonstration, we'll show how to construct the request

    // Example: Fetch a specific event by ID (replace with actual event ID)
    let event_id = "21742"; // Example event ID

    let request = WebserviceRequest::new_event_by_id_request(event_id);
    let url = request.get_callable_url(0);

    println!("Request URL: {}\n", url);

    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;

    // Check if request was successful
    if !response.status().is_success() {
        println!("Error: {}", response.status());
        println!("Note: Replace the event_id with a valid Polymarket event ID");
        return Ok(());
    }

    let events: EventResponse = response.json().await?;
    let count = events.nb_results();

    println!("Found {} event(s):\n", count);

    for (i, event) in events.iter().enumerate() {
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
