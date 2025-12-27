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
//! ## Quick Start
//!
//! ### Fetching Markets
//!
//! ```rust,no_run
//! use poly_clob_rs::{WebserviceRequest, MarketsResponse};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut request = WebserviceRequest::new_markets_ws_request();
//!     request.with_active_only();
//!
//!     let url = request.get_callable_url(0);
//!     let client = reqwest::Client::new();
//!     let markets: MarketsResponse = client.get(&url).send().await?.json().await?;
//!
//!     for market in markets {
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
//! use poly_clob_rs::{WebserviceRequest, PolymarketPricesResponse};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let token_ids = vec!["token_id".to_string()];
//!     let request = WebserviceRequest::new_polymarket_price_request(&token_ids);
//!
//!     let url = request.get_callable_url(0);
//!     let client = reqwest::Client::new();
//!     let prices: PolymarketPricesResponse = client.get(&url).send().await?.json().await?;
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
//!     let account = Account::load_poly_account()?;
//!
//!     // Simple order with defaults (GTC, expiration=0)
//!     // Note: Polymarket API enforces precision limits:
//!     // - USDC amounts (price × size): max 4 decimals
//!     // - Token amounts (size): max 2 decimals
//!     // The library automatically rounds to these limits.
//!     let result = LimitOrderRequest::builder()
//!         .signer(&account)
//!         .price(Decimal::from_f64("0.52_f64)?)
//!         .size(Decimal::from_f64("10.0_f64)?)
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
//! - [`api`] - API request builders and authentication
//! - [`models`] - Data models for API requests and responses
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
pub mod models;

// Re-export commonly used items for convenience
pub use api::*;
pub use models::*;
