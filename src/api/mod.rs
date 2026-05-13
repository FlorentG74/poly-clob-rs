//! API request builders and authentication utilities.
//!
//! This module provides all the tools needed to interact with the Polymarket CLOB API:
//!
//! - **Request Builders**: Construct API requests using the [`WebserviceRequest`] type
//! - **Authentication**: L1 (EIP-712) and L2 (HMAC) authentication utilities
//! - **Endpoints**: Constants for all Polymarket API endpoints
//!
//! ## Submodules
//!
//! - [`auth`] - Authentication helpers for L1 and L2 auth
//! - [`clob_endpoints`] - API endpoint URL constants
//! - [`crypto_price_requests`] - Crypto opening/closing price request builders (strike/settlement)
//! - [`webservice_request`] - Core request builder type
//! - [`event_requests`] - Event and event series request builders
//! - [`market_requests`] - Market data request builders
//! - [`order_requests`] - Order placement and management request builders
//! - [`position_requests`] - User position request builders
//! - [`price_requests`] - Price data request builders
//! - [`tag_requests`] - Tag/category request builders
//! - [`relayer`] - Polymarket Relayer V2 API client for gasless transactions
//! - [`utils`] - Utility functions for API interactions
//!
//! ## Example
//!
//! ```rust
//! use poly_clob_rs::WebserviceRequest;
//! use reqwest::Method;
//!
//! let request = WebserviceRequest {
//!     api: "https://gamma-api.polymarket.com".to_string(),
//!     url: "/markets".to_string(),
//!     method: Method::GET,
//!     with_pagination: true,
//!     args: vec![("active".to_string(), "true".to_string())],
//!     body: None,
//! };
//! let url = request.get_callable_url(0);
//! ```

pub mod account_requests;
pub mod auth;
pub mod activity_requests;
pub mod clob_endpoints;
pub mod crypto_price_requests;
pub mod error;
pub mod event_requests;
pub mod fee_requests;
pub mod http_client;
pub mod market_requests;
pub mod order_requests;
pub mod orderbook_requests;
pub mod position_requests;
pub mod price_history_requests;
pub mod price_requests;
pub mod relayer;
pub mod response_handler;
pub mod tag_requests;
pub mod webservice_request;
pub mod utils;

/// Sort direction for requests (ascending or descending).
#[derive(Debug, Clone, Copy)]
pub enum SortDirection {
    /// Ascending order
    ASC,
    /// Descending order
    DESC,
}

impl SortDirection {
    /// Returns the string representation for API requests.
    pub fn as_str(&self) -> &'static str {
        match self {
            SortDirection::ASC => "ASC",
            SortDirection::DESC => "DESC",
        }
    }
}

pub use auth::*;
pub use activity_requests::*;
pub use clob_endpoints::*;
pub use crypto_price_requests::*;
pub use fee_requests::*;
pub use market_requests::*;
pub use order_requests::*;
pub use orderbook_requests::*;
pub use relayer::*;
pub use webservice_request::*;
pub use utils::*;
