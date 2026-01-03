//! Example: Fetch user activity from Polymarket
//!
//! This example demonstrates how to query user activity using the Polymarket Data API.
//!
//! Run with:
//! ```bash
//! cargo run --example fetch_activity
//! ```

use poly_clob_rs::api::activity_requests::{ActivityRequest, ActivityType, ActivitySortBy, SortDirection};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Fetching user activity from Polymarket...\n");

    // Create a request for user activity (using the sample user from the curl example)
    let request = ActivityRequest::builder()
        .user("0x961afce6bd9aec79c5cf09d2d4dac2b434b23361")
        .limit(10) // Limit to 10 activities for this example
        .activity_type(vec![ActivityType::TRADE]) // Only trades
        .sort_by(ActivitySortBy::TIMESTAMP)
        .sort_direction(SortDirection::DESC)
        .build();

    println!("Request parameters:");
    println!("  User: {}", request.user);
    println!("  Limit: {}", request.limit);
    println!("  Activity Types: {:?}", request.activity_type.iter().map(|t| t.as_str()).collect::<Vec<_>>());
    println!("  Sort By: {}", request.sort_by.as_str());
    println!("  Sort Direction: {}", request.sort_direction.as_str());
    println!();

    // Execute the request
    match request.execute().await {
        Ok(activities) => {
            println!("Found {} activities:\n", activities.len());

            for (i, activity) in activities.iter().enumerate() {
                println!("{}. Activity at timestamp: {}", i + 1, activity.timestamp);
                println!("   Type: {}", activity.welcome_type);
                println!("   Side: {}", activity.side);
                println!("   Size: {}", activity.size);
                println!("   USDC Size: {}", activity.usdc_size);
                println!("   Price: {}", activity.price);
                println!("   Title: {}", activity.title);
                println!("   Transaction Hash: {}", &activity.transaction_hash[..10]); // First 10 chars
                println!();
            }
        }
        Err(e) => {
            eprintln!("Error fetching activities: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}