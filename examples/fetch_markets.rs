//! Example: Fetch active markets from Polymarket (all pages)
//!
//! Demonstrates cursor-based pagination with the `/markets/keyset` endpoint.
//!
//! Run with:
//! ```bash
//! cargo run --example fetch_markets
//! ```

use poly_clob_rs::api::market_requests::MarketsRequest;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Fetching active markets from Polymarket...\n");

    let mut cursor: Option<String> = None;
    let mut total = 0usize;
    let mut page_num = 0usize;

    loop {
        page_num += 1;
        let page = MarketsRequest::builder()
            .closed(Some(false))
            .limit(100)
            .cursor(cursor.clone())
            .build()
            .execute()
            .await?;

        let count = page.data.len();
        total += count;
        println!("Page {page_num}: {count} markets (total so far: {total})");

        // Display first 5 markets on the first page
        if page_num == 1 {
            for (i, market) in page.data.iter().take(5).enumerate() {
                println!("  {}. {}", i + 1, market.question.as_deref().unwrap_or("N/A"));
                println!("     Slug: {}", market.slug.as_deref().unwrap_or("N/A"));
                println!("     Active: {}", market.active.unwrap_or(false));
                if let Some(volume) = &market.volume
                    && let Ok(vol) = volume.parse::<f64>() {
                        println!("     Volume: ${:.2}", vol);
                    }
                println!();
            }
        }

        cursor = page.next_cursor;
        if cursor.is_none() {
            break;
        }
    }

    println!("Done. Fetched {total} markets across {page_num} page(s).");
    Ok(())
}
