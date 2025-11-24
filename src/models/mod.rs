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
//! let account = Account::actual_account_from_env();
//! let order = Order::new(
//!     &account.poly_address,
//!     &account.poly_address,
//!     "0x0000000000000000000000000000000000000000",
//!     "token_id",
//!     100,
//!     50,
//!     9999999999,
//!     10,
//!     Side::Buy,
//!     OrderType::GTC,
//! );
//! ```

pub mod account;
pub mod api_response;
pub mod clob_types;
pub mod clob_orders;
pub mod market;
pub mod order;
pub mod order_type;
pub mod event;
pub mod event_series;
pub mod tag;
pub mod position;
pub mod open_order;
pub mod polymarket_price;
pub mod side;

pub use account::*;
pub use api_response::*;
pub use clob_types::*;
pub use clob_orders::*;
pub use market::*;
pub use order::*;
pub use order_type::*;
pub use event::*;
pub use event_series::*;
pub use tag::*;
pub use position::*;
pub use open_order::*;
pub use polymarket_price::*;
pub use side::*;
