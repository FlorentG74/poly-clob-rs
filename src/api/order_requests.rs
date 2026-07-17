//! Order placement and management request builders.
//!
//! This module provides functions for placing and managing orders on the Polymarket CLOB.

use crate::api::error::{Result, ValidationError};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use typed_builder::TypedBuilder;

use crate::api::auth::get_timestamp;
use crate::api::authed_request::{send_authed, send_authed_text, Method};
use crate::api::response_handler::handle_api_response;
use crate::models::{Account, Order, OrderType, Side};
use crate::{market_requests, MarketOrders, OpenOrder, WebserviceRequest, ORDERS};

use super::clob_endpoints::{CLOB_API, CANCEL, GET_ORDER, POST_ORDER};

use crate::constants::raw_multiplier_decimal;

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
    /// Paper-trade hint: market slug (ignored for live orders)
    #[builder(default)]
    pub market_slug: Option<&'a str>,
    /// Paper-trade hint: market title / question (ignored for live orders)
    #[builder(default)]
    pub market_title: Option<&'a str>,
    /// Paper-trade hint: outcome label e.g. "Up"/"Down" (ignored for live orders)
    #[builder(default)]
    pub outcome: Option<&'a str>,
}

impl<'a> LimitOrderRequest<'a> {
    /// Builds and validates the Order without executing it.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Order)` if the order is valid, or an error if validation fails.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * Expiration is non-zero for FOK/FAK/GTC orders
    /// * Expiration is zero for GTD orders
    /// * Order size validation fails (BUY orders: USD amount must be > $1.0 AND token quantity must be >= 5)
    pub async fn build(&self) -> Result<Order> {
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

        let raw_multiplier = raw_multiplier_decimal();

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

        let order = Order::builder()
            .maker(&self.signer.poly_address)
            .signer(&self.signer.pub_key)
            .token_id(self.token_id)
            .maker_amount(maker_amount)
            .taker_amount(taker_amount)
            .expiration(self.expiration)
            .side(self.side)
            .neg_risk(self.neg_risk)
            .order_type(self.order_type)
            .build();

        // Validate order before returning
        order.validate_order()?;

        Ok(order)
    }

    /// Execute a pre-built order, skipping the build/validation step.
    ///
    /// Use when you have already called [`build()`] for validation and want to avoid
    /// redundant signing on the hot path. The `order` value is consumed and a fresh
    /// timestamp/salt are applied before sending, matching the behaviour of [`execute()`].
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
    pub async fn execute_order(&self, order: Order) -> Result<String> {
        let mut order = order;

        let request_path = POST_ORDER;
        let callable_url = format!("{}{}", CLOB_API, request_path);

        let l2_timestamp = get_timestamp();
        let now_ms = chrono::Utc::now().timestamp_millis() as u64;
        order.timestamp = now_ms;
        let order_salt = ((rand::random::<f64>() * now_ms as f64) as u64).to_string();
        let body = order.build_order_query_body(
            order_salt.as_str(),
            self.signer.api_key.as_str(),
            self.signer.private_key.as_str(),
        )?;

        log::debug!("Signed Order body: {}", body);

        // Send the order placement request
        let response = send_authed(
            self.signer,
            Method::Post,
            request_path,
            &callable_url,
            &body,
            &l2_timestamp,
        )
        .await?;

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
    /// * Order size validation fails
    /// * The HTTP request fails
    /// * The API returns an error response
    pub async fn execute(&self) -> Result<String> {
        // Build and validate, then reuse the shared submission path.
        let order = self.build().await?;
        self.execute_order(order).await
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
        let request_path = CANCEL;
        let callable_url = format!("{}{}", CLOB_API, request_path);

        // Build request body with orderID
        let body = format!(r#"{{"orderID":"{}"}}"#, self.order_id);

        let salt = get_timestamp();

        log::debug!("Canceling order: {}", self.order_id);
        log::debug!("Cancel request body: {}", body);

        let response = send_authed(
            self.signer,
            Method::Delete,
            request_path,
            &callable_url,
            &body,
            &salt,
        )
        .await?;

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

/// Response from querying a single order by ID.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct OrderStatusResponse {
    pub id: String,
    pub status: String,
    #[serde(default)]
    pub size_matched: String,
    #[serde(default)]
    pub original_size: String,
    #[serde(default)]
    pub price: String,
}

/// Fetches the status of a single order by its ID.
///
/// Calls `GET /data/order/{order_id}` with L2 authentication headers.
///
/// # Arguments
/// * `signer` - The account that placed the order
/// * `order_id` - The order ID to query
///
/// # Returns
/// `Ok(OrderStatusResponse)` with the order's current status, or an error.
pub async fn get_order_by_id(signer: &Account, order_id: &str) -> Result<OrderStatusResponse> {
    // The full path (including order_id) must be signed — Python reference:
    // endpoint = GET_ORDER + order_id; request_path=endpoint → build_hmac_signature
    let signed_path = format!("{}{}", GET_ORDER, order_id);
    let callable_url = format!("{}{}", CLOB_API, signed_path);

    let response_text =
        send_authed_text(signer, Method::Get, &signed_path, &callable_url, "", "").await?;

    serde_json::from_str(&response_text).map_err(|e| {
        crate::SerializationError::JsonDeserialize {
            message: e.to_string(),
            raw_response: response_text,
        }
        .into()
    })
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
    let request_path = ORDERS;

    let mut callable_url = format!("{}{}", CLOB_API, request_path);
    WebserviceRequest::add_param_to_url(&mut callable_url, "market", market_id);

    let response = send_authed(signer, Method::Get, request_path, &callable_url, "", "").await?;

    handle_orders_response(response, &callable_url).await
}

/// Handles the HTTP response for orders requests.
async fn handle_orders_response(response: reqwest::Response, url: &str) -> Result<MarketOrders> {
    let response_text = handle_api_response(response, url).await?;

    serde_json::from_str(&response_text).map_err(|e| {
        crate::SerializationError::JsonDeserialize {
            message: e.to_string(),
            raw_response: response_text,
        }
        .into()
    })
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

    // -------------------------------------------------------------------------
    // Auth integration tests
    // -------------------------------------------------------------------------

    /// Verifies that get_order_by_id() auth works end-to-end.
    /// A non-401 response (including 404 "not found") means the HMAC was accepted.
    ///
    /// Run with: cargo test -p poly-clob-rs test_order_status_auth_live -- --nocapture
    #[tokio::test]
    async fn test_order_status_auth_live() {
        crate::config::init_from_env();
        let account = crate::models::Account::load_poly_account()
            .expect("load poly account from .env");

        let order_id = "0xfe71c215fb15bd66b983e98cf9f599a84b6581b6052fdb07b6317a33049cd662";

        println!("\n=== Order auth test ===");
        println!("order_id: {}", order_id);

        let result = get_order_by_id(&account, order_id).await;

        println!("result: {:?}", result);

        // Auth is working if we get anything other than a 401 error.
        // A 404 "not found" (order expired/gone) still means auth succeeded.
        if let Err(ref e) = result {
            assert!(
                !e.to_string().contains("unauthorized"),
                "L2 auth rejected: {}",
                e
            );
        }
    }

    // -------------------------------------------------------------------------
    // OrderStatusResponse deserialization tests
    //
    // These tests reproduce the real scenario of order
    // 0x3256a66ed9fb6778f9b0212c13fad538e3f80052ead53253acac331f775a5331:
    //   - GTD BUY UP at $0.53, qty 11.32 on eth-updown-15m-1771427700
    //   - Order went "live" (resting on book)
    //   - Auth failures prevented check_pending_order from detecting the fill
    //   - Order was eventually matched on-chain
    //   - fill_price parsing must NOT fall back to 0.0
    // -------------------------------------------------------------------------

    /// Matched order: all fields populated — the happy path when auth works.
    #[test]
    fn test_order_status_matched_deserialization() {
        let json = r#"{
            "id": "0x3256a66ed9fb6778f9b0212c13fad538e3f80052ead53253acac331f775a5331",
            "status": "matched",
            "size_matched": "11.32",
            "original_size": "11.32",
            "price": "0.53"
        }"#;

        let response: OrderStatusResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.id, "0x3256a66ed9fb6778f9b0212c13fad538e3f80052ead53253acac331f775a5331");
        assert_eq!(response.status, "matched");
        assert_eq!(response.size_matched, "11.32");
        assert_eq!(response.original_size, "11.32");
        assert_eq!(response.price, "0.53");

        // Simulate how check_pending_order extracts fill_price
        let limit_price = 0.53_f64;
        let fill_price = response.price.parse::<f64>().unwrap_or(limit_price);
        assert_eq!(fill_price, 0.53);

        // Simulate how check_pending_order extracts filled_quantity
        let fallback_qty = 11.32_f64;
        let filled_quantity = response.size_matched.parse::<f64>().unwrap_or(fallback_qty);
        assert_eq!(filled_quantity, 11.32);
    }

    /// Live order with no fill yet: the state the bot was stuck in during auth failures.
    /// The bot placed the GTD at 16:22:33 and got back status="live", takingAmount="", makingAmount="".
    #[test]
    fn test_order_status_live_no_fill() {
        let json = r#"{
            "id": "0x3256a66ed9fb6778f9b0212c13fad538e3f80052ead53253acac331f775a5331",
            "status": "live",
            "size_matched": "0",
            "original_size": "11.32",
            "price": "0.53"
        }"#;

        let response: OrderStatusResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.status, "live");

        let size_matched = response.size_matched.parse::<f64>().unwrap_or(0.0);
        assert_eq!(size_matched, 0.0);
        // size_matched == 0.0 → no partial fill, bot should continue waiting
    }

    /// Live order with partial fill: size_matched > 0 but status still "live".
    /// check_pending_order treats this as fully filled and cancels the remainder.
    #[test]
    fn test_order_status_live_partial_fill() {
        let json = r#"{
            "id": "0x3256a66ed9fb6778f9b0212c13fad538e3f80052ead53253acac331f775a5331",
            "status": "live",
            "size_matched": "5.66",
            "original_size": "11.32",
            "price": "0.53"
        }"#;

        let response: OrderStatusResponse = serde_json::from_str(json).unwrap();

        let size_matched = response.size_matched.parse::<f64>().unwrap_or(0.0);
        assert!(size_matched > 0.0, "partial fill should be detected");

        // fill_price should parse correctly — must NOT fall back to 0.0
        let limit_price = 0.53_f64;
        let fill_price = response.price.parse::<f64>().unwrap_or(limit_price);
        assert!(fill_price > 0.0, "fill_price must never be 0.0 — would corrupt stop loss");
        assert_eq!(fill_price, 0.53);
    }

    /// Missing price field: #[serde(default)] fills it with "".
    /// parse::<f64>() on "" fails → fallback to limit_price.
    /// This ensures entry_price is never 0.0 when the limit price is known.
    #[test]
    fn test_order_status_missing_price_falls_back_to_limit_price() {
        let json = r#"{
            "id": "0x3256a66ed9fb6778f9b0212c13fad538e3f80052ead53253acac331f775a5331",
            "status": "matched",
            "size_matched": "11.32",
            "original_size": "11.32"
        }"#;

        let response: OrderStatusResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.price, "", "missing price deserializes to empty string");

        let limit_price = 0.53_f64;
        let fill_price = response.price.parse::<f64>().unwrap_or(limit_price);

        // Must fall back to limit_price, never 0.0
        assert_eq!(fill_price, limit_price);
        assert!(fill_price > 0.0, "fill_price must never be 0.0");
    }

    /// Cancelled order: pending_order should be cleared, no fill recorded.
    #[test]
    fn test_order_status_cancelled_deserialization() {
        let json = r#"{
            "id": "0x3256a66ed9fb6778f9b0212c13fad538e3f80052ead53253acac331f775a5331",
            "status": "cancelled",
            "size_matched": "0",
            "original_size": "11.32",
            "price": "0.53"
        }"#;

        let response: OrderStatusResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.status, "cancelled");
        let size_matched = response.size_matched.parse::<f64>().unwrap_or(0.0);
        assert_eq!(size_matched, 0.0);
    }

    #[test]
    fn test_buy_order_amount_calculation_with_exact_decimals() {
        // Test: size=8.82, price=0.45 should produce maker_amount=3969000
        // This reproduces the rounding issue from the error:
        // "the maker amount for a $0.45 order of size 8.82 should be '3.969' but the value submited is '3.9692'"
        let size = Decimal::from_f64(8.82_f64).unwrap();
        let price = Decimal::from_f64(0.45_f64).unwrap();

        let raw_multiplier = raw_multiplier_decimal();
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

        let raw_multiplier = raw_multiplier_decimal();
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

        let raw_multiplier = raw_multiplier_decimal();
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

        let raw_multiplier = raw_multiplier_decimal();
        let result = (size * problematic_price * raw_multiplier)
            .round()
            .to_i32()
            .expect("overflow");

        // 100 * 0.33 * 1_000_000 = 33_000_000
        assert_eq!(result, 33_000_000);
    }
}
