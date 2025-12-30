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
//! - [`webservice_request`] - Core request builder type
//! - [`event_requests`] - Event and event series request builders
//! - [`market_requests`] - Market data request builders
//! - [`order_requests`] - Order placement and management request builders
//! - [`position_requests`] - User position request builders
//! - [`price_requests`] - Price data request builders
//! - [`tag_requests`] - Tag/category request builders
//!
//! ## Example
//!
//! ```rust
//! use poly_clob_rs::WebserviceRequest;
//!
//! let mut request = WebserviceRequest::new_markets_ws_request();
//! request.with_active_only();
//! let url = request.get_callable_url(0);
//! ```

pub mod auth;
pub mod clob_endpoints;
pub mod error;
pub mod event_requests;
pub mod http_client;
pub mod market_requests;
pub mod order_requests;
pub mod position_requests;
pub mod price_requests;
pub mod response_handler;
pub mod settlement_requests;
pub mod tag_requests;
pub mod webservice_request;

pub use auth::*;
pub use clob_endpoints::*;
pub use market_requests::*;
pub use order_requests::*;
pub use settlement_requests::*;
pub use webservice_request::*;
