# Examples Guide

Each example is a runnable binary that makes real API calls to Polymarket.

## Prerequisites

- A recent Rust toolchain (the workspace pins nightly via `rust-toolchain.toml`)
- Internet access to `*.polymarket.com`
- Every example calls `poly_clob_rs::config::init_from_env()` at startup, which loads `.env` (searched upward from the working directory) and installs the crate configuration. No credentials are needed for the read-only examples; network-policy variables (`DNS_RESOLVER`, `SPLIT_TUNNEL_IFACE`) are honored if set.

Run from the `poly-clob-rs` directory:

```bash
cargo run --example <example_name>
```

## Examples

### fetch_markets

Paginates through all active markets on the Gamma `/markets/keyset` endpoint using `MarketsRequest` and cursor-based pagination, printing per-page counts and details for the first few markets.

### fetch_prices

Fetches an active market, extracts its `clob_token_ids`, then queries real-time buy/sell prices for those tokens from the CLOB `/prices` endpoint (a POST built by `WebserviceRequest::new_polymarket_price_request`).

### fetch_events

Fetches a single event (a collection of related markets) by ID and prints its metadata and market count. Edit the `event_id` variable to inspect a different event.

### fetch_activity

Queries the Data API for a user's recent trade activity using the `ActivityRequest` builder (type/sort/limit filters). Uses a sample public address; substitute your own.

### fetch_crypto_price

Fetches open/close prices for the most recent completed 15-minute ETH up/down window via `CryptoPriceRequest` — the same call used for strike setting and settlement of up/down events. The endpoint only serves timestamps from the last ~30 days.

### query_positions

Fetches open positions for an address from the Data API:

```bash
cargo run --example query_positions <ethereum_address>
# or
POLY_ADDRESS=0x... cargo run --example query_positions
```

## Building on the examples

- **Pagination**: offset-paginated endpoints use `fetch_batch` (returns the next offset, `-1` when exhausted); keyset endpoints use `next_cursor` until it is `None`.
- **HTTP clients**: always use `api::http_client::get_http_client(Some(url))` — it applies the configured Polymarket network policy and caches clients process-wide.
- **Authentication**: write operations (orders, cancels) need credentials in the installed `Config`; see the README's authentication section.

## Common issues

- **Panic: `config::init() must be called`** — the process made a request before installing a `Config`. Call `config::init(...)` (or `init_from_env()`) at the top of `main`.
- **Empty results / "Invalid payload"** — placeholder IDs; fetch real IDs first (markets before prices).
- **Network errors** — check connectivity to `*.polymarket.com`; if your resolver interferes, set `DNS_RESOLVER`.
