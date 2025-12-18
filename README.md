# poly-clob-rs

A Rust client library for the [Polymarket](https://polymarket.com) CLOB (Central Limit Order Book) API.

This library provides a comprehensive interface to interact with Polymarket's prediction markets, including:
- Fetching market data, events, and positions
- Querying real-time prices
- Placing and managing orders
- Authentication via EIP-712 signatures (L1) and HMAC-based API keys (L2)

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
poly-clob-rs = "0.1.0"
```

## Features

- **Market Data**: Query markets, events, event series, tags, and positions
- **Price Information**: Get real-time bid/ask prices for prediction market tokens
- **Order Management**: Place, cancel, and query orders using authenticated API access
- **Dual Authentication**: Supports both L1 (EIP-712 wallet signatures) and L2 (HMAC API key) authentication
- **Type-Safe**: Strongly typed models for all API responses
- **Builder Pattern**: Fluent API for constructing requests

## Quick Start

### Fetching Market Data

```rust
use poly_clob_rs::{WebserviceRequest, MarketsResponse};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a request for active markets
    let mut request = WebserviceRequest::new_markets_ws_request();
    request.with_active_only();

    // Build the URL and make the request
    let url = request.get_callable_url(0);
    let client = reqwest::Client::new();
    let markets: MarketsResponse = client.get(&url).send().await?.json().await?;

    // Print market information
    for market in markets {
        println!("{}: {}", market.question, market.slug);
    }

    Ok(())
}
```

### Fetching Prices

```rust
use poly_clob_rs::{WebserviceRequest, PolymarketPricesResponse};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token_ids = vec!["token_id_1".to_string(), "token_id_2".to_string()];
    let request = WebserviceRequest::new_polymarket_price_request(&token_ids);

    let url = request.get_callable_url(0);
    let client = reqwest::Client::new();
    let prices: PolymarketPricesResponse = client.get(&url).send().await?.json().await?;

    for (token_id, price) in prices {
        println!("{}: buy={:?}, sell={:?}", token_id, price.buy, price.sell);
    }

    Ok(())
}
```

### Authentication and Order Placement

```rust
use poly_clob_rs::{Account, Side, OrderType, api::order_requests::LimitOrderRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load account credentials from environment
    let account = Account::load_poly_account()?;

    // Place a simple GTC order with sensible defaults
    let result = LimitOrderRequest::builder()
        .signer(&account)
        .price(0.52)
        .size(10.0)
        .side(Side::Buy)
        .token_id("token_id")
        .build()
        .execute()
        .await?;

    println!("Order placed: {}", result);

    // Or with explicit options (GTD order with expiration)
    let result = LimitOrderRequest::builder()
        .signer(&account)
        .price(0.52)
        .size(10.0)
        .side(Side::Buy)
        .token_id("token_id")
        .neg_risk(true)
        .order_type(OrderType::GTD)
        .expiration(1735689600)
        .build()
        .execute()
        .await?;

    Ok(())
}
```

### Querying User Positions

```rust
use poly_clob_rs::{WebserviceRequest, PositionsResponse};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let user_address = "0x1234...";
    let request = WebserviceRequest::new_positions_ws_request(user_address);

    let url = request.get_callable_url(0);
    let client = reqwest::Client::new();
    let positions: PositionsResponse = client.get(&url).send().await?.json().await?;

    for position in positions {
        println!(
            "{} {}: size={}, avg_price={}, pnl={}%",
            position.title,
            position.outcome,
            position.size,
            position.avg_price,
            position.percent_pnl
        );
    }

    Ok(())
}
```

## Environment Variables

To use authenticated endpoints, set the following environment variables:

```bash
# L1 Authentication (EIP-712 wallet signatures)
POLY_ADDRESS="0x..."           # Your Polygon wallet address
PUB_KEY="0x..."                # Your public key
PRIVATE_KEY="0x..."            # Your private key

# L2 Authentication (API key-based)
POLY_API_KEY="your-api-key"
POLY_API_SECRET="your-api-secret"
POLY_API_PASSPHRASE="your-passphrase"

# Optional: Telegram integration
TELEGRAM_CHAT_ID="123456789"
TELEGRAM_BOT_TOKEN="bot-token"
```

## API Endpoints

The library provides access to three Polymarket API bases:

- **GAMMA API** (`https://gamma-api.polymarket.com`) - Market and event data
- **CLOB API** (`https://clob.polymarket.com`) - Order book and trading
- **DATA API** (`https://data-api.polymarket.com`) - Historical data and prices

## Authentication

### L1 Authentication (EIP-712)

L1 authentication uses EIP-712 signatures for order placement. Orders are signed with your Ethereum private key:

```rust
use poly_clob_rs::{Order, Side, OrderType};

let order = Order::builder()
    .maker("0x...")
    .signer("0x...")
    .taker("0x0000000000000000000000000000000000000000")
    .token_id("token_id")
    .maker_amount(100)
    .taker_amount(50)
    .side(Side::Buy)
    .order_type(OrderType::GTC)
    .build();

let signed_body = order.build_order_query_body(salt, nonce, api_key, private_key)?;
```

### L2 Authentication (HMAC)

L2 authentication uses HMAC signatures for API requests:

```rust
use poly_clob_rs::auth::build_l2_headers;

let headers = build_l2_headers(
    &address,
    &api_key,
    &api_secret,
    &api_passphrase,
    "GET",
    "/path",
    "",  // query string
    "",  // body
);
```

## Data Models

The library provides strongly-typed models for all API responses:

- **PolyResponseMarket** - Market information with 39+ fields
- **PolyResponseEvent** - Event data (groups of related markets)
- **PolyResponseEventSeries** - Event series (recurring events)
- **Position** - User position in a market
- **OpenOrder** - Open order information
- **PolymarketPrice** - Bid/ask price data
- **PolyResponseTag** - Market categories/tags

See the [API documentation](https://docs.rs/poly-clob-rs) for complete type definitions.

## Request Builder

The `WebserviceRequest` type provides a fluent API for building requests:

```rust
let mut request = WebserviceRequest::new_markets_ws_request();
request
    .with_active_only()              // Only active markets
    .with_from_start_date("2024-01-01")  // Markets starting after date
    .with_tag_id("crypto");          // Filter by tag

let url = request.get_callable_url(0);  // offset = 0
```

## Pagination

API responses that support pagination implement the `ApiResponse` trait:

```rust
use poly_clob_rs::ApiResponse;

let markets: MarketsResponse = /* fetch from API */;
let count = markets.nb_results();  // Number of results in this page
```

Use the offset parameter in `get_callable_url(offset)` to fetch subsequent pages.

## Examples

See the `examples/` directory for complete working examples:

- `fetch_markets.rs` - Fetch and display market data
- `fetch_prices.rs` - Query token prices
- `place_order.rs` - Place an order with authentication
- `query_positions.rs` - Get user positions

Run an example:

```bash
cargo run --example fetch_markets
```

## Network

This library targets the **Polygon** blockchain (Chain ID: 137). All orders and authentication are scoped to this network.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Disclaimer

This library is not officially affiliated with Polymarket. Use at your own risk. Always test with small amounts first.

## Resources

- [Polymarket](https://polymarket.com)
- [Polymarket Documentation](https://docs.polymarket.com)
- [API Documentation](https://docs.rs/poly-clob-rs)
