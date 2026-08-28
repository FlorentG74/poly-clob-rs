//! Data models for Polymarket API requests and responses.
//!
//! This module contains all the type definitions for interacting with the Polymarket CLOB API:
//!
//! ## Core Types
//!
//! - [`Account`] - User account credentials for authentication
//! - [`Order`] - Order structure for placing trades (implements EIP-712 signing)
//!
//! ## Response Types
//!
//! - [`PolyResponseMarket`] / [`MarketsResponse`] - Market data
//! - [`PolyResponseEvent`] / [`EventResponse`] - Event data
//! - [`PolyResponseEventSeries`] / [`EventSeriesResponse`] - Event series data
//! - [`Position`] / [`PositionsResponse`] - User positions
//! - [`OpenOrder`] - Open order information
//! - [`PolymarketPrice`] / [`PolymarketPricesResponse`] - Price data
//! - [`PolyResponseTag`] / [`PolymarketTagsResponse`] - Tags/categories
//!
//! ## Supporting Types
//!
//! - [`AssetType`] - Asset type (COLLATERAL/CONDITIONAL)
//! - [`Side`] - Order side (BUY/SELL)
//! - [`OrderType`] - Order type (FOK/FAK/GTC/GTD)
//!
//! ## Traits
//!
//! - [`ApiResponse`] - Trait for types that represent paginated API responses
//! - [`api::auth::EIP712Struct`](crate::api::auth::EIP712Struct) - Trait for types that can be signed using EIP-712
//!
//! ## Example
//!
//! ```rust,no_run
//! use poly_clob_rs::{Account, Order, Side, OrderType};
//!
//! let account = Account::load_poly_account().expect("failed to load account");
//! let order = Order::builder()
//!     .maker(&account.poly_address)
//!     .signer(&account.poly_address)
//!     .token_id("token_id")
//!     .maker_amount(100u64)
//!     .taker_amount(50u64)
//!     .side(Side::Buy)
//!     .order_type(OrderType::GTC)
//!     .build();
//! ```

pub mod account;
pub mod activity;
pub mod api_response;
pub mod clob_orders;
pub mod clob_types;
pub mod crypto_price;
pub mod event;
pub mod event_series;
pub mod market;
pub mod open_order;
pub mod order;
pub mod order_type;
pub mod orderbook;
pub mod polymarket_price;
pub mod position;
pub mod side;
pub mod tag;

pub use account::*;
pub use activity::*;
pub use api_response::*;
pub use clob_orders::*;
pub use clob_types::*;
pub use crypto_price::*;
pub use event::*;
pub use event_series::*;
pub use market::*;
pub use open_order::*;
pub use order::*;
pub use order_type::*;
pub use orderbook::*;
pub use polymarket_price::*;
pub use position::*;
pub use side::*;
pub use tag::*;
