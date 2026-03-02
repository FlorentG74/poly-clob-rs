//! Request builders for the Polymarket crypto price API.
//!
//! This module provides request builders for fetching opening and closing prices
//! for crypto up/down events, used for strike setting and settlement resolution.

use reqwest::Method;
use typed_builder::TypedBuilder;

use crate::api::error::Result;
use crate::api::http_client::get_http_client;
use crate::models::CryptoPriceResponse;

use super::{WebserviceRequest, GET_CRYPTO_PRICE, POLYMARKET_API};

/// Request builder for fetching crypto prices at a specific event time.
///
/// This endpoint returns both opening and closing prices for a crypto symbol
/// at a given event start time. Used for:
/// - **Strike setting**: Get the `open_price` at event start
/// - **Settlement**: Get the `close_price` at event maturity (when `completed=true`)
///
/// # Example
///
/// ```rust,no_run
/// use poly_clob_rs::api::crypto_price_requests::CryptoPriceRequest;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let response = CryptoPriceRequest::builder()
///         .symbol("ETH")
///         .event_start_time(1738023000)
///         .build()
///         .execute()
///         .await?;
///
///     println!("Open: {}, Close: {}", response.open_price, response.close_price);
///     if response.is_valid_for_settlement() {
///         println!("Event is settled");
///     }
///     Ok(())
/// }
/// ```
#[derive(TypedBuilder)]
pub struct CryptoPriceRequest<'a> {
    /// Crypto symbol (e.g., "BTC", "ETH", "SOL", "XRP")
    #[builder(setter(into))]
    pub symbol: &'a str,

    /// Event start time as Unix timestamp in seconds
    pub event_start_time: i64,
}

impl<'a> CryptoPriceRequest<'a> {
    /// Execute the request and return the crypto price response.
    pub async fn execute(&self) -> Result<CryptoPriceResponse> {
        let client = get_http_client(None);

        let web_service_request = WebserviceRequest {
            api: POLYMARKET_API.to_string(),
            url: GET_CRYPTO_PRICE.to_string(),
            method: Method::GET,
            with_pagination: false,
            args: vec![
                ("symbol".to_string(), self.symbol.to_string()),
                ("eventStartTime".to_string(), self.event_start_time.to_string()),
            ],
            body: None,
        };

        WebserviceRequest::fetch_one::<CryptoPriceResponse>(client, &web_service_request).await
    }
}

impl WebserviceRequest {
    /// Create a new request for fetching crypto prices at an event time.
    ///
    /// # Arguments
    /// * `symbol` - Crypto symbol (e.g., "BTC", "ETH", "SOL", "XRP")
    /// * `event_start_time` - Event start time as Unix timestamp in seconds
    pub fn new_crypto_price_request(symbol: &str, event_start_time: i64) -> Self {
        WebserviceRequest {
            api: POLYMARKET_API.to_string(),
            url: GET_CRYPTO_PRICE.to_string(),
            method: Method::GET,
            with_pagination: false,
            args: vec![
                ("symbol".to_string(), symbol.to_string()),
                ("eventStartTime".to_string(), event_start_time.to_string()),
            ],
            body: None,
        }
    }
}
