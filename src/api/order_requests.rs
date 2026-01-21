//! Order placement and management request builders.
//!
//! This module provides functions for placing and managing orders on the Polymarket CLOB.

use crate::api::error::{Result, ValidationError};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use typed_builder::TypedBuilder;

use crate::api::auth::{build_l2_headers, get_timestamp, get_zero_address};
use crate::api::response_handler::handle_api_response;
use crate::http_client::get_http_client;
use crate::models::{Account, Order, OrderType, Side};
use crate::{market_requests, MarketOrders, OpenOrder, WebserviceRequest, ORDERS, fee_requests};
use reqwest::header::*;

use super::clob_endpoints::{CLOB_API, CANCEL, POST_ORDER};

/// Multiplier to convert decimal amounts to raw units (10^6) for the Polymarket API
const RAW_UNIT_MULTIPLIER: i64 = 1_000_000;

/// Parameters for placing a limit order on the Polymarket CLOB.
///
/// # Required Fields
///
/// * `signer` - The account to sign and place the order with
/// * `price` - The price per share (0.0 to 1.0) as Decimal
/// * `size` - The size of the order in token quantity (number of shares) as Decimal
/// * `side` - Whether this is a buy or sell order
/// * `token_id` - The token ID for the outcome
///
/// # Optional Fields (with defaults)
///
/// * `condition_id` - The market condition ID (default: empty, used for paper trading)
/// * `neg_risk` - Whether this is a neg-risk market (default: false)
/// * `order_type` - The type of order (default: GTC)
/// * `expiration` - Order expiration timestamp (default: 0, required non-zero for GTD orders)
///
/// # Precision Limits
///
/// The Polymarket API enforces strict decimal precision on order amounts:
/// * **USDC amounts** (price × size): maximum 4 decimal places
/// * **Token amounts** (size): maximum 2 decimal places
///
/// This library automatically rounds amounts to comply with these limits:
/// * For BUY orders: maker_amount (USDC) is rounded to 4 decimals, taker_amount (tokens) to 2 decimals
/// * For SELL orders: maker_amount (tokens) is rounded to 2 decimals, taker_amount (USDC) to 4 decimals
///
/// # Order Types
///
/// * `FOK` (Fill-Or-Kill) - Must be executed immediately in full or cancelled. Expiration must be 0.
/// * `FAK` (Fill-And-Kill) - Execute immediately for available shares, cancel the rest. Expiration must be 0.
/// * `GTC` (Good-Til-Cancelled) - Active until fulfilled or manually cancelled. Expiration must be 0.
/// * `GTD` (Good-Til-Date) - Active until specified date. Expiration must be non-zero Unix timestamp.
///
/// # Example
///
/// ```no_run
/// use poly_clob_rs::{Account, Side, OrderType, api::order_requests::LimitOrderRequest};
/// use rust_decimal::Decimal;
/// use std::str::FromStr;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let account = Account::load_poly_account()?;
///
/// // Simple GTC order with defaults
/// let request = LimitOrderRequest::builder()
///     .signer(&account)
///     .price(Decimal::from_str("0.52")?)
///     .size(Decimal::from_str("10.0")?)
///     .side(Side::Buy)
///     .token_id("1234567890")
///     .build();
///
/// // GTD order with explicit expiration
/// let request = LimitOrderRequest::builder()
///     .signer(&account)
///     .price(Decimal::from_str("0.52")?)
///     .size(Decimal::from_str("10.0")?)
///     .side(Side::Buy)
///     .token_id("1234567890")
///     .order_type(OrderType::GTD)
///     .expiration(1735689600)
///     .build();
///
/// let result = request.execute().await?;
/// # Ok(())
/// # }
/// ```
#[derive(TypedBuilder)]
pub struct LimitOrderRequest<'a> {
    /// The account to sign and place the order with
    pub signer: &'a Account,
    /// The price per share (0.0 to 1.0) with up to 4 decimal places
    pub price: Decimal,
    /// The size of the order in token quantity (number of shares)
    pub size: Decimal,
    /// Whether this is a buy or sell order
    pub side: Side,
    /// The token ID for the outcome
    #[builder(setter(into))]
    pub token_id: &'a str,
    /// The market condition ID (used for paper trading integration)
    #[builder(default = "", setter(into))]
    pub condition_id: &'a str,
    /// Whether this is a neg-risk market
    #[builder(default = false)]
    pub neg_risk: bool,
    /// The type of order (FOK, FAK, GTC, or GTD)
    #[builder(default = OrderType::GTC)]
    pub order_type: OrderType,
    /// Order expiration timestamp (required for GTD, must be 0 for others)
    #[builder(default = 0)]
    pub expiration: i64,
    /// Whether to fetch the fee rate from the API (default: false)
    #[builder(default = false)]
    pub with_fee: bool,
}

impl<'a> LimitOrderRequest<'a> {
    /// Executes the limit order request.
    ///
    /// # Returns
    ///
    /// Returns `Ok(String)` with the API response on success, or an error on failure.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * Expiration is non-zero for FOK/FAK/GTC orders
    /// * Expiration is zero for GTD orders
    /// * The HTTP request fails
    /// * The API returns an error response
    pub async fn execute(&self) -> Result<String> {
        // Validate expiration based on order type
        match self.order_type {
            OrderType::GTD => {
                if self.expiration == 0 {
                    return Err(ValidationError::InvalidParameter {
                        parameter: "expiration".to_string(),
                        reason: "GTD orders require a non-zero expiration timestamp".to_string(),
                    }.into());
                }
            }
            OrderType::FOK | OrderType::FAK | OrderType::GTC => {
                if self.expiration != 0 {
                    return Err(ValidationError::InvalidParameter {
                        parameter: "expiration".to_string(),
                        reason: format!("{} orders must have expiration set to 0", self.order_type),
                    }.into());
                }
            }
        }

        let method = "POST";
        let request_path = POST_ORDER;

        let callable_url = format!("{}{}", CLOB_API, request_path);

        let client = get_http_client(Some(request_path));

        // Polymarket API amounts are in raw units (10^6):
        // - For BUY orders: maker_amount is USDC (price denominated), taker_amount is tokens
        // - For SELL orders: maker_amount is tokens, taker_amount is USDC (price denominated)
        // Using Decimal ensures exact arithmetic with no floating-point rounding errors.
        // Note: size parameter represents token quantity (number of shares) for both BUY and SELL

        let raw_multiplier = Decimal::from(RAW_UNIT_MULTIPLIER);

        // Polymarket API precision requirements:
        // - Token amounts (size): max 2 decimals
        // - USDC amounts (size × price): max 4 decimals
        // Important: Round size FIRST, then use rounded size for USDC calculation
        let rounded_size = self.size.round_dp(2);

        let (maker_amount, taker_amount) = if self.side == Side::Buy {
            // BUY: giving USDC (maker), receiving tokens (taker)
            let maker_amount = ((rounded_size * self.price).round_dp(4) * raw_multiplier)
                .to_i32()
                .expect("maker_amount overflow");
            let taker_amount = (rounded_size * raw_multiplier)
                .to_i32()
                .expect("taker_amount overflow");
            (maker_amount, taker_amount)
        } else {
            // SELL: giving tokens (maker), receiving USDC (taker)
            let maker_amount = (rounded_size * raw_multiplier)
                .to_i32()
                .expect("maker_amount overflow");
            let taker_amount = ((rounded_size * self.price).round_dp(4) * raw_multiplier)
                .to_i32()
                .expect("taker_amount overflow");
            (maker_amount, taker_amount)
        };

        let fee_rate_bps = if self.with_fee {
            match fee_requests::get_fee_rate(self.token_id).await {
                Ok(rate) => rate.base_fee,
                Err(e) => {
                    log::error!("Failed to fetch fee rate; using 0 as default: {}", e);
                    0
                },
            }
        } else {
            0
        };

        let order = Order::builder()
            .maker(&self.signer.poly_address)
            .signer(&self.signer.pub_key)
            .taker(get_zero_address())
            .token_id(self.token_id)
            .maker_amount(maker_amount)
            .taker_amount(taker_amount)
            .expiration(self.expiration)
            .fee_rate_bps(fee_rate_bps)
            .side(self.side)
            .neg_risk(self.neg_risk)
            .order_type(self.order_type)
            .build();

        let salt = get_timestamp();
        let nonce = 0; // Nonce for order signing
        let body = order.build_order_query_body(
            salt.as_str(),
            nonce,
            self.signer.api_key.as_str(),
            self.signer.private_key.as_str(),
        )?;

        let l2_headers = build_l2_headers(self.signer, method, request_path, &body, &salt)?;

        log::debug!("Signed Order body: {}", &body);

        // Validate order prior to sending
        order.validate_order()?;

        // Send the order placement request
        let response = client
            .post(&callable_url)
            .header(CONTENT_TYPE, "application/json")
            .header(ACCEPT, "application/json")
            .headers(l2_headers)
            .body(body)
            .send()
            .await
            .map_err(|e| crate::api::error::HttpError::from_reqwest(e, callable_url.clone()))?;

        log::trace!("API Call Raw Response: {:?}", response);

        let status = response.status();
        log::debug!("Order response status: {}", status);

        // Log order side for debugging
        if !status.is_success() {
            log::error!(
                "Error encountered while posting {} order: HTTP status {}",
                order.side.to_lowercase_str(),
                status
            );
        }

        handle_api_response(response, &callable_url).await
    }
}

/// Parameters for canceling an order on the Polymarket CLOB.
///
/// # Required Fields
///
/// * `signer` - The account that placed the order
/// * `order_id` - The ID of the order to cancel
///
/// # Example
///
/// ```no_run
/// use poly_clob_rs::{Account, api::order_requests::CancelOrderRequest};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let account = Account::load_poly_account()?;
///
/// let request = CancelOrderRequest::builder()
///     .signer(&account)
///     .order_id("order_12345")
///     .build();
///
/// let result = request.execute().await?;
/// # Ok(())
/// # }
/// ```
#[derive(TypedBuilder)]
pub struct CancelOrderRequest<'a> {
    /// The account that placed the order
    pub signer: &'a Account,
    /// The ID of the order to cancel
    #[builder(setter(into))]
    pub order_id: &'a str,
}

impl<'a> CancelOrderRequest<'a> {
    /// Executes the cancel order request.
    ///
    /// # Returns
    ///
    /// Returns `Ok(String)` with the API response on success, or an error on failure.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * The HTTP request fails
    /// * The API returns an error response
    pub async fn execute(&self) -> Result<String> {
        let method = "DELETE";
        let request_path = CANCEL;

        let callable_url = format!("{}{}", CLOB_API, request_path);

        let client = get_http_client(Some(CANCEL));

        // Build request body with orderID
        let body = format!(r#"{{"orderID":"{}"}}"#, self.order_id);

        let salt = get_timestamp();
        let l2_headers = build_l2_headers(self.signer, method, request_path, &body, &salt)?;

        log::debug!("Canceling order: {}", self.order_id);
        log::debug!("Cancel request body: {}", &body);

        let response = client
            .delete(&callable_url)
            .header(CONTENT_TYPE, "application/json")
            .header(ACCEPT, "application/json")
            .headers(l2_headers)
            .body(body)
            .send()
            .await
            .map_err(|e| crate::api::error::HttpError::from_reqwest(e, callable_url.clone()))?;

        log::trace!("API Call Raw Response: {:?}", response);

        if !response.status().is_success() {
            log::error!(
                "Error encountered while canceling order {}",
                self.order_id
            );
        }

        handle_api_response(response, &callable_url).await
    }
}

/// Returns all open orders for the given account.
pub async fn get_all_open_orders(signer: &Account) -> Result<Vec<OpenOrder>> {
    get_open_orders_by_market(signer, "").await
}

/// Returns open orders for the given account, optionally filtered by market.
pub async fn get_open_orders_by_market(signer: &Account, market_id: &str) -> Result<Vec<OpenOrder>> {
    let market_orders = fetch_raw_orders(signer, market_id).await?;

    if market_orders.data.is_empty() {
        return Ok(Vec::new());
    }

    enrich_orders_with_markets(market_orders).await
}

/// Fetches raw orders from the CLOB API.
async fn fetch_raw_orders(signer: &Account, market_id: &str) -> Result<MarketOrders> {
    let client = get_http_client(None);

    let method = "GET";
    let request_path = ORDERS;
    let body = "";

    let mut callable_url = format!("{}{}", CLOB_API, request_path);
    WebserviceRequest::add_param_to_url(&mut callable_url, "market", market_id);

    let l2_headers = build_l2_headers(signer, method, request_path, body, "")?;

    let response = client
        .get(&callable_url)
        .header(CONTENT_TYPE, "application/json")
        .header(ACCEPT, "application/json")
        .headers(l2_headers)
        .send()
        .await
        .map_err(|e| crate::api::error::HttpError::from_reqwest(e, callable_url.clone()))?;

    handle_orders_response(response, &callable_url).await
}

/// Handles the HTTP response for orders requests.
async fn handle_orders_response(response: reqwest::Response, url: &str) -> Result<MarketOrders> {
    match response.status() {
        reqwest::StatusCode::OK => {
            let text = response.text().await
                .map_err(|e| crate::api::error::HttpError::ReadBody {
                    url: url.to_string(),
                    message: e.to_string(),
                })?;
            log::trace!("API response: {}", text);
            serde_json::from_str(&text)
                .map_err(|e| crate::api::error::SerializationError::JsonDeserialize {
                    message: e.to_string(),
                    raw_response: text.clone(),
                }.into())
        }
        reqwest::StatusCode::TOO_MANY_REQUESTS => {
            log::error!("Rate Limit reached - pausing for 5 secs");
            Err(crate::api::error::ApiError::RateLimited {
                retry_after: std::time::Duration::from_secs(5),
                url: url.to_string(),
                retry_after_header: None,
            }.into())
        }
        reqwest::StatusCode::UNAUTHORIZED => {
            log::error!("Authentication failed for request {}", url);
            Err(crate::api::error::ApiError::Unauthorized {
                url: url.to_string(),
                details: None,
            }.into())
        }
        other => {
            log::error!(
                "Unexpected error in service call: {:?}; url: {}",
                other,
                url
            );
            Err(crate::api::error::ApiError::UnexpectedStatus {
                status: other.as_u16(),
                url: url.to_string(),
                message: format!("HTTP {}", other),
                response_body: String::new(),
            }.into())
        }
    }
}

/// Enriches raw orders with market data.
async fn enrich_orders_with_markets(market_orders: MarketOrders) -> Result<Vec<OpenOrder>> {
    let condition_ids = extract_unique_condition_ids(&market_orders.data);

    let markets = market_requests::map_multiple_market_by_condition_ids_ws(&condition_ids).await?;

    market_orders
        .data
        .into_iter()
        .map(|order| {
            let market = markets
                .get(&order.market)
                .ok_or_else(|| crate::api::error::ApiError::NotFound {
                    url: String::new(),
                    resource: format!("market with condition_id: {}", order.market),
                })?
                .clone();
            parse_market_order(order, market)
        })
        .collect()
}

/// Extracts unique condition IDs from orders.
fn extract_unique_condition_ids(orders: &[crate::MarketOrder]) -> Vec<String> {
    let mut condition_ids: Vec<String> = orders.iter().map(|p| p.market.clone()).collect();
    condition_ids.sort();
    condition_ids.dedup();
    condition_ids
}

/// Parses a raw market order into an OpenOrder with market data.
fn parse_market_order(
    order: crate::MarketOrder,
    market: crate::PolyResponseMarket,
) -> Result<OpenOrder> {
    Ok(OpenOrder {
        id: order.id,
        status: order.status,
        owner: order.owner,
        maker_address: order.maker_address,
        market,
        asset_id: order.asset_id,
        side: order.side,
        original_size: order
            .original_size
            .parse::<f64>()
            .map_err(|e| crate::api::error::SerializationError::FieldParse {
                field: "original_size".to_string(),
                message: e.to_string(),
            })?,
        size_matched: order
            .size_matched
            .parse::<f64>()
            .map_err(|e| crate::api::error::SerializationError::FieldParse {
                field: "size_matched".to_string(),
                message: e.to_string(),
            })?,
        price: order
            .price
            .parse::<f64>()
            .map_err(|e| crate::api::error::SerializationError::FieldParse {
                field: "price".to_string(),
                message: e.to_string(),
            })?,
        outcome: order.outcome,
        expiration: order.expiration,
        order_type: order.order_type,
    })
}

#[cfg(test)]
mod tests {
    use rust_decimal::prelude::FromPrimitive;

    use super::*;

    #[test]
    fn test_buy_order_amount_calculation_with_exact_decimals() {
        // Test: size=8.82, price=0.45 should produce maker_amount=3969000
        // This reproduces the rounding issue from the error:
        // "the maker amount for a $0.45 order of size 8.82 should be '3.969' but the value submited is '3.9692'"
        let size = Decimal::from_f64(8.82_f64).unwrap();
        let price = Decimal::from_f64(0.45_f64).unwrap();

        let raw_multiplier = Decimal::from(RAW_UNIT_MULTIPLIER);
        let expected_maker_amount = 3_969_000i32; // 8.82 * 0.45 * 1_000_000 = 3_969_000

        let calculated_maker = (size * price * raw_multiplier)
            .round()
            .to_i32()
            .expect("overflow");

        assert_eq!(calculated_maker, expected_maker_amount,
            "BUY order maker_amount calculation failed: expected {}, got {}",
            expected_maker_amount, calculated_maker);
    }

    #[test]
    fn test_buy_order_taker_amount_calculation() {
        // Test: size=8.82 should produce taker_amount=8820000
        let size = Decimal::from_f64(8.82_f64).unwrap();

        let raw_multiplier = Decimal::from(RAW_UNIT_MULTIPLIER);
        let expected_taker_amount = 8_820_000i32; // 8.82 * 1_000_000 = 8_820_000

        let calculated_taker = (size * raw_multiplier)
            .round()
            .to_i32()
            .expect("overflow");

        assert_eq!(calculated_taker, expected_taker_amount,
            "BUY order taker_amount calculation failed: expected {}, got {}",
            expected_taker_amount, calculated_taker);
    }

    #[test]
    fn test_sell_order_amounts_with_exact_decimals() {
        // Test SELL: size=8.82, price=0.45
        let size = Decimal::from_f64(8.82_f64).unwrap();
        let price = Decimal::from_f64(0.45_f64).unwrap();

        let raw_multiplier = Decimal::from(RAW_UNIT_MULTIPLIER);
        let expected_maker_amount = 8_820_000i32; // tokens: 8.82 * 1_000_000
        let expected_taker_amount = 3_969_000i32; // USDC: 8.82 * 0.45 * 1_000_000

        let calculated_maker = (size * raw_multiplier)
            .round()
            .to_i32()
            .expect("overflow");
        let calculated_taker = (size * price * raw_multiplier)
            .round()
            .to_i32()
            .expect("overflow");

        assert_eq!(calculated_maker, expected_maker_amount, "SELL maker_amount mismatch");
        assert_eq!(calculated_taker, expected_taker_amount, "SELL taker_amount mismatch");
    }

    #[test]
    fn test_decimal_precision_no_float_errors() {
        // Verify Decimal handles cases that would lose precision with f64
        let problematic_price = Decimal::from_f64(0.33_f64).unwrap(); // 1/3 repeating
        let size = Decimal::from_f64(100_f64).unwrap();

        let raw_multiplier = Decimal::from(RAW_UNIT_MULTIPLIER);
        let result = (size * problematic_price * raw_multiplier)
            .round()
            .to_i32()
            .expect("overflow");

        // 100 * 0.33 * 1_000_000 = 33_000_000
        assert_eq!(result, 33_000_000);
    }
}
