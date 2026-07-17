//! # poly-clob-rs
//!
//! A Rust client library for the [Polymarket](https://polymarket.com) CLOB (Central Limit Order Book) API.
//!
//! This library provides a comprehensive interface to interact with Polymarket's prediction markets:
//! - Fetch market data, events, and positions
//! - Query real-time prices
//! - Place and manage orders
//! - Authenticate via EIP-712 signatures (L1) or HMAC-based API keys (L2)
//!
//! ## Configuration
//!
//! The library never reads `.env` or the process environment on its own. Install a
//! [`config::Config`] once, early in `main`, before any request is made:
//!
//! ```rust,no_run
//! use poly_clob_rs::config::{self, Config};
//!
//! fn main() {
//!     dotenvy::dotenv().ok();            // the caller decides to use .env
//!     config::init(Config::from_env());  // ... and installs the result
//! }
//! ```
//!
//! See [`config`] for details (credentials, split tunnelling, DNS overrides).
//!
//! ## Quick Start
//!
//! ### Fetching Markets
//!
//! ```rust,no_run
//! use poly_clob_rs::api::market_requests::MarketsRequest;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     poly_clob_rs::config::init_from_env();
//!
//!     let page = MarketsRequest::builder()
//!         .closed(Some(false))
//!         .limit(100)
//!         .build()
//!         .execute()
//!         .await?;
//!
//!     for market in &page.data {
//!         println!("{}: {}",
//!             market.question.as_deref().unwrap_or("No question"),
//!             market.slug.as_deref().unwrap_or("no-slug"));
//!     }
//!     Ok(())
//! }
//! ```
//!
//! ### Querying Prices
//!
//! ```rust,no_run
//! use poly_clob_rs::api::http_client::get_http_client;
//! use poly_clob_rs::{WebserviceRequest, PolymarketPricesResponse};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     poly_clob_rs::config::init_from_env();
//!
//!     let token_ids = vec!["token_id".to_string()];
//!     let request = WebserviceRequest::new_polymarket_price_request(&token_ids);
//!     let client = get_http_client(Some(&request.api));
//!
//!     let prices: PolymarketPricesResponse =
//!         WebserviceRequest::fetch_one(client, &request).await?;
//!
//!     for (token_id, price) in &prices {
//!         println!("{token_id}: buy={:?} sell={:?}", price.buy, price.sell);
//!     }
//!     Ok(())
//! }
//! ```
//!
//! ### Placing Orders
//!
//! ```rust,no_run
//! use poly_clob_rs::{Account, Side, OrderType, api::order_requests::LimitOrderRequest};
//! use rust_decimal::Decimal;
//! use std::str::FromStr;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     poly_clob_rs::config::init_from_env();
//!     let account = Account::load_poly_account()?;
//!
//!     // Simple order with defaults (GTC, expiration=0)
//!     // Note: Polymarket API enforces precision limits:
//!     // - USDC amounts (price × size): max 4 decimals
//!     // - Token amounts (size): max 2 decimals
//!     // The library automatically rounds to these limits.
//!     let result = LimitOrderRequest::builder()
//!         .signer(&account)
//!         .price(Decimal::from_str("0.52")?)
//!         .size(Decimal::from_str("10.0")?)
//!         .side(Side::Buy)
//!         .token_id("token_id")
//!         .build()
//!         .execute()
//!         .await?;
//!
//!     println!("Order placed: {}", result);
//!     Ok(())
//! }
//! ```
//!
//! ## Modules
//!
//! - [`api`] - API request builders, HTTP client factory, and authentication
//! - [`models`] - Data models for API requests and responses
//! - [`config`] - Caller-supplied process-wide configuration
//! - [`ws`] - Polymarket websocket message types
//!
//! ## Authentication
//!
//! The library supports two authentication methods:
//!
//! - **L1 Authentication (EIP-712)**: Sign orders with your Ethereum private key using EIP-712 typed data signatures
//! - **L2 Authentication (HMAC)**: Use API keys with HMAC-SHA256 signatures for authenticated requests
//!
//! See [`api::auth`] for authentication utilities.

pub mod api;
pub mod config;
pub mod models;
pub mod constants;
pub mod ws;

// Re-export constants at crate root for convenience
pub use constants::*;

// Re-export commonly used items for convenience
pub use api::*;
pub use models::*;

// Re-export error types for ergonomic error handling
pub use api::error::{
    ApiError, AuthError, ClobError, HttpError, RelayerError, Result, SerializationError,
    ValidationError,
};
