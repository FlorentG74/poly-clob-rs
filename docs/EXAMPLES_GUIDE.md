# Examples Guide

This document describes how to run the included examples and what each one demonstrates.

## Prerequisites

All examples require the `poly-clob-rs` library and its dependencies. Make sure you have:
- Rust toolchain installed (1.70.0 or later recommended)
- Internet connection (examples make real API calls to Polymarket)

## Running Examples

From the `poly-clob-rs` directory:

```bash
cargo run --example <example_name>
```

## Available Examples

### 1. fetch_markets

Fetches and displays active markets from Polymarket.

**Run:**
```bash
cargo run --example fetch_markets
```

**What it does:**
- Queries the Polymarket GAMMA API for active markets
- Displays the first 10 markets with details:
  - Question/title
  - Slug (URL-friendly identifier)
  - Active status
  - Trading volume
  - Best bid/ask prices

**Sample output:**
```
Fetching active markets from Polymarket...

Found 100 markets:

1. Fed rate hike in 2025?
   Slug: fed-rate-hike-in-2025
   Active: true
   Volume: $803858.78
   Best Bid: 0.011, Best Ask: 0.014
```

### 2. fetch_prices

Fetches real-time prices for prediction market tokens.

**Run:**
```bash
cargo run --example fetch_prices
```

**What it does:**
- Queries token prices from the CLOB API
- Displays buy/sell prices for specified tokens

**Note:** The example uses placeholder token IDs. For real use, you need to:
1. First fetch markets using `fetch_markets`
2. Extract `clob_token_ids` from the markets
3. Use those actual token IDs in the price request

### 3. query_positions

Queries positions for a specific user address.

**Run with address argument:**
```bash
cargo run --example query_positions <ethereum_address>
```

**Or with environment variable:**
```bash
export POLY_ADDRESS="0x1234..."
cargo run --example query_positions
```

**What it does:**
- Fetches all open positions for a user
- Displays position details:
  - Market title and outcome
  - Position size
  - Average and current price
  - P&L (profit/loss) metrics

**Requirements:**
- Valid Ethereum address on Polygon network
- Address must have traded on Polymarket

### 4. fetch_events

Fetches event data (collections of related markets).

**Run:**
```bash
cargo run --example fetch_events
```

**What it does:**
- Queries a specific event by ID
- Displays event metadata and associated markets

**Note:** The example uses a placeholder event ID. For real use:
1. Find valid event IDs from Polymarket's website
2. Update the `event_id` variable in the example

## Using Examples as Templates

These examples are designed to be used as templates for your own code:

1. **Copy the pattern** - Each example shows the basic request/response pattern
2. **Add authentication** - For write operations (placing orders), add L1/L2 auth
3. **Handle pagination** - Use `offset` parameter to fetch more results
4. **Error handling** - Add robust error handling for production use

## Example: Building on fetch_markets

```rust
use poly_clob_rs::{ApiResponse, MarketsResponse, WebserviceRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut request = WebserviceRequest::new_markets_ws_request();
    request.with_active_only();
    request.with_tag_id("crypto"); // Add tag filter

    let client = reqwest::Client::new();
    let mut offset = 0;

    // Paginate through all results
    loop {
        let url = request.get_callable_url(offset);
        let markets: MarketsResponse = client.get(&url).send().await?.json().await?;

        if markets.is_empty() {
            break;
        }

        // Process markets...
        for market in markets {
            println!("{}", market.question.unwrap_or_default());
        }

        offset += 100; // Next page
    }

    Ok(())
}
```

## Common Issues

### "Connection refused" or network errors
- Check your internet connection
- Polymarket API might be temporarily unavailable
- Try again in a few moments

### "Invalid payload" errors
- You're using placeholder/invalid IDs
- Fetch real IDs from the API first (e.g., get markets before getting prices)

### Environment variable not found
- Set required environment variables (e.g., `POLY_ADDRESS`)
- Or pass values as command-line arguments where supported

## Next Steps

After exploring the examples:

1. **Read the API docs** - Run `cargo doc --open` to see full documentation
2. **Check the README** - See [README.md](README.md) for more usage patterns
3. **Review the source** - Example code is in `examples/` directory
4. **Add authentication** - See README for L1/L2 authentication examples

## Contributing

Found an issue with an example? Contributions welcome!
- Open an issue on GitHub
- Submit a pull request with improvements
- Share your own example use cases
