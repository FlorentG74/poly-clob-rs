# poly-clob-rs

A Rust client library for the [Polymarket](https://polymarket.com) APIs: the CLOB (Central Limit Order Book), the Gamma market-data API, and the Data API.

- Typed request builders for markets, events, prices, order books, positions, and user activity
- Order placement and cancellation with EIP-712 signing (L1) and HMAC API keys (L2)
- Gasless relayer transactions (redeem, approvals) and bridge withdrawals
- WebSocket transport helpers for live market/user feeds
- Caller-supplied configuration — the library never reads `.env` or the environment on its own
- Built-in network policy for Polymarket hosts: optional interface binding (split tunnel) and DNS override

## Installation

```toml
[dependencies]
poly-clob-rs = "0.1.0"
```

## Configuration

Install a `Config` once, early in `main`, before any request is made. Every HTTP client, resolver, and credential lookup in the crate reads from this snapshot:

```rust
use poly_clob_rs::config::{self, Config};

fn main() {
    dotenvy::dotenv().ok();              // the caller decides to use .env
    config::init(Config::from_env());    // ... and installs the result
}
```

`config::init_from_env()` combines both lines for binaries whose configuration lives in `.env`. `Config::from_env()` is a convenience; construct the struct directly to source values from a file, a secrets manager, or literals in a test. Requests made before `init` panic with an actionable message — there are no silent defaults.

### Settings

| Env var (via `Config::from_env`) | Purpose |
|---|---|
| `POLY_ADDRESS`, `PUB_KEY`, `PRIVATE_KEY` | Wallet identity and L1 signing key |
| `API_KEY`, `API_SECRET`, `API_PASSPHRASE` | L2 (HMAC) API credentials |
| `SIGNATURE_TYPE` | Wallet type: `EOA`, `POLY_PROXY` (default), or `GNOSIS_SAFE` |
| `POLY_BUILDER_API_KEY/_SECRET/_PASSPHRASE` | Builder credentials for relayer transactions |
| `TELEGRAM_CHAT_ID`, `TELEGRAM_BOT_TOKEN` | Optional Telegram notification identity carried on `Account` |
| `SPLIT_TUNNEL_IFACE` | Bind Polymarket sockets to a specific network interface |
| `DNS_RESOLVER` | Comma-separated nameservers used for Polymarket hostnames |

All credentials are optional: read-only usage needs none, and a missing credential only errors where it is actually required.

## Quick Start

### Fetching markets

```rust
use poly_clob_rs::api::market_requests::MarketsRequest;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    poly_clob_rs::config::init_from_env();

    let mut cursor: Option<String> = None;
    loop {
        let page = MarketsRequest::builder()
            .closed(Some(false))
            .limit(100)
            .cursor(cursor.clone())
            .build()
            .execute()
            .await?;

        for market in &page.data {
            println!("{}: {}",
                market.question.as_deref().unwrap_or("?"),
                market.slug.as_deref().unwrap_or("?"));
        }

        cursor = page.next_cursor;
        if cursor.is_none() { break; }
    }
    Ok(())
}
```

### Querying prices

```rust
use poly_clob_rs::api::http_client::get_http_client;
use poly_clob_rs::{PolymarketPricesResponse, WebserviceRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    poly_clob_rs::config::init_from_env();

    let token_ids = vec!["token_id".to_string()];
    let request = WebserviceRequest::new_polymarket_price_request(&token_ids);
    let client = get_http_client(Some(&request.api));

    let prices: PolymarketPricesResponse =
        WebserviceRequest::fetch_one(client, &request).await?;

    for (token_id, price) in &prices {
        println!("{token_id}: buy={:?}, sell={:?}", price.buy, price.sell);
    }
    Ok(())
}
```

### Placing orders

```rust
use poly_clob_rs::{Account, Side, OrderType, api::order_requests::LimitOrderRequest};
use rust_decimal::Decimal;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    poly_clob_rs::config::init_from_env();
    let account = Account::load_poly_account()?;

    // GTC order with defaults
    let result = LimitOrderRequest::builder()
        .signer(&account)
        .price(Decimal::from_str("0.52")?)
        .size(Decimal::from_str("10.0")?)
        .side(Side::Buy)
        .token_id("token_id")
        .build()
        .execute()
        .await?;
    println!("Order placed: {result}");

    // GTD order with explicit expiration
    let result = LimitOrderRequest::builder()
        .signer(&account)
        .price(Decimal::from_str("0.52")?)
        .size(Decimal::from_str("10.0")?)
        .side(Side::Buy)
        .token_id("token_id")
        .neg_risk(true)
        .order_type(OrderType::GTD)
        .expiration(1735689600)
        .build()
        .execute()
        .await?;
    println!("Order placed: {result}");
    Ok(())
}
```

Supported order types: `FOK`, `FAK`, `GTC` (default), `GTD`. The API enforces precision limits (USDC amounts max 4 decimals, token amounts max 2); the library rounds automatically.

### Querying positions

```rust
use poly_clob_rs::api::http_client::get_http_client;
use poly_clob_rs::{ApiResponse, PositionsResponse, WebserviceRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    poly_clob_rs::config::init_from_env();

    let request = WebserviceRequest::new_positions_ws_request("0x1234...");
    let client = get_http_client(Some(&request.api));

    let (_next, positions): (i32, PositionsResponse) =
        WebserviceRequest::fetch_batch(client, &request, 0).await?;

    for position in positions.iter() {
        println!("{} {}: size={}, avg_price={}, pnl={}%",
            position.title, position.outcome,
            position.size, position.avg_price, position.percent_pnl);
    }
    Ok(())
}
```

## Request builders

Typed builders (`TypedBuilder`-based, with an async `execute()`) cover the common endpoints:

| Builder | Endpoint |
|---|---|
| `MarketsRequest` / `MarketBySlugRequest` | Gamma markets (keyset pagination / by slug) |
| `EventBySlugRequest` / `SeriesEventsRequest` | Gamma events and event series |
| `OrderBooksRequest` | CLOB batch order books |
| `LimitOrderRequest` / `CancelOrderRequest` | CLOB order placement / cancellation |
| `ActivityRequest` | Data API user activity |
| `CryptoPriceRequest` | Crypto open/close prices for up/down event strike and settlement |

Endpoints without a typed builder use `WebserviceRequest` constructors (e.g. `new_positions_ws_request`, `new_polymarket_price_request`) together with `fetch_one` / `fetch_batch` (offset pagination) / `fetch_keyset` (cursor pagination). All three retry transient errors automatically.

## HTTP clients and network policy

Always obtain clients from `api::http_client::get_http_client(Some(url))` rather than constructing `reqwest::Client` directly: Polymarket-bound clients apply the configured split-tunnel interface binding and DNS override, and clients are cached process-wide. WebSocket upgrades go through `ws::` helpers, which apply the same policy over HTTP/1.1.

## API endpoints

- **Gamma API** (`https://gamma-api.polymarket.com`) — market and event data
- **CLOB API** (`https://clob.polymarket.com`) — order books and trading
- **Data API** (`https://data-api.polymarket.com`) — positions and activity
- **Relayer** — gasless transactions (redeem, approvals) via the Builder API

## Authentication

- **L1 (EIP-712)** — orders are signed with your Ethereum private key as EIP-712 typed data. Handled internally by `LimitOrderRequest`/`CancelOrderRequest`; the low-level path is `Order::build_order_query_body`.
- **L2 (HMAC)** — authenticated REST requests carry HMAC-SHA256 headers built by `api::auth::build_l2_headers`.

The library targets the **Polygon** network (chain ID 137).

## Error handling

All fallible APIs return `poly_clob_rs::Result<T>` with the `ClobError` enum (`Api`, `Http`, `Auth`, `Validation`, `Serialization`, `Relayer` variants). `ClobError::is_retryable()` and `retry_after()` support backoff logic; the built-in fetch helpers already retry transient failures.

## Examples

See [`examples/`](examples/) — each is a runnable binary (`cargo run --example <name>`):

- `fetch_markets` — paginate all active markets
- `fetch_prices` — fetch a live market, then query its token prices
- `fetch_events` — fetch an event with its markets
- `fetch_activity` — query a user's trade activity
- `fetch_crypto_price` — open/close prices for an up/down event window
- `query_positions` — open positions for an address

See [docs/EXAMPLES_GUIDE.md](docs/EXAMPLES_GUIDE.md) for details.

## License

Licensed under either of [Apache License 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT), at your option.

## Disclaimer

This library is not officially affiliated with Polymarket. Use at your own risk. Always test with small amounts first.
