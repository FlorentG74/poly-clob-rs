//! Order placement and management request builders.
//!
//! This module provides functions for placing and managing orders on the Polymarket CLOB.

use anyhow::{Context, Result};
use typed_builder::TypedBuilder;

use crate::api::auth::{build_l2_headers, get_timestamp, get_zero_address};
use crate::api::http_client::get_http_client;
use crate::api::response_handler::handle_api_response;
use crate::models::{Account, Order, OrderType, Side};
use crate::{market_requests, MarketOrders, OpenOrder, WebserviceRequest, ORDERS};
use reqwest::header::*;

use super::clob_endpoints::{CLOB_API, POST_ORDER};

/// Parameters for placing a limit order on the Polymarket CLOB.
///
/// # Required Fields
///
/// * `signer` - The account to sign and place the order with
/// * `price` - The price per share (0.0 to 1.0)
/// * `size` - The size of the order in token quantity (number of shares)
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
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let account = Account::load_poly_account()?;
///
/// // Simple GTC order with defaults
/// let request = LimitOrderRequest::builder()
///     .signer(&account)
///     .price(0.52)
///     .size(10.0)
///     .side(Side::Buy)
///     .token_id("1234567890")
///     .build();
///
/// // GTD order with explicit expiration
/// let request = LimitOrderRequest::builder()
///     .signer(&account)
///     .price(0.52)
///     .size(10.0)
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
    /// The price per share (0.0 to 1.0)
    pub price: f64,
    /// The size of the order in token quantity (number of shares)
    pub size: f64,
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
                anyhow::ensure!(
                    self.expiration != 0,
                    "GTD orders require a non-zero expiration timestamp"
                );
            }
            OrderType::FOK | OrderType::FAK | OrderType::GTC => {
                anyhow::ensure!(
                    self.expiration == 0,
                    "{} orders must have expiration set to 0",
                    self.order_type
                );
            }
        }

        let client = get_http_client();

        let method = "POST";
        let request_path = POST_ORDER;

        let callable_url = format!("{}{}", CLOB_API, request_path);

        // Polymarket API precision requirements:
        // - For BUY orders: maker_amount (USDC) max 4 decimals, taker_amount (tokens) max 2 decimals
        // - For SELL orders: maker_amount (tokens) max 2 decimals, taker_amount (USDC) max 4 decimals
        // USDC precision: 4 decimals (10^4 = 10000), Token precision: 2 decimals (10^2 = 100)
        // Both are converted to raw units (10^6) for the API
        //
        // Note: size parameter represents token quantity (number of shares) for both BUY and SELL

        let (maker_amount, taker_amount) = if self.side == Side::Buy {
            // BUY: giving USDC (maker), receiving tokens (taker)
            // maker_amount = size × price (USDC with 4 decimal precision)
            // taker_amount = size (tokens with 2 decimal precision)
            let maker_amount = ((10000.0 * self.size * self.price).round() * 100.0).round() as i32;
            let taker_amount = ((100.0 * self.size).round() * 10000.0).round() as i32;
            (maker_amount, taker_amount)
        } else {
            // SELL: giving tokens (maker), receiving USDC (taker)
            // maker_amount = size (tokens with 2 decimal precision)
            // taker_amount = size × price (USDC with 4 decimal precision)
            let maker_amount = ((100.0 * self.size).round() * 10000.0).round() as i32;
            let taker_amount = ((10000.0 * self.size * self.price).round() * 100.0).round() as i32;
            (maker_amount, taker_amount)
        };

        let order = Order::builder()
            .maker(&self.signer.poly_address)
            .signer(&self.signer.pub_key)
            .taker(get_zero_address())
            .token_id(self.token_id)
            .maker_amount(maker_amount)
            .taker_amount(taker_amount)
            .expiration(self.expiration)
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

        let response = client
            .post(&callable_url)
            .header(CONTENT_TYPE, "application/json")
            .header(ACCEPT, "application/json")
            .headers(l2_headers)
            .body(body)
            .send()
            .await
            .context("HTTP request failed")?;

        log::trace!("API Call Raw Response: {:?}", response);

        // Log order side for debugging
        if !response.status().is_success() {
            log::error!(
                "Error encountered while posting {} order",
                order.side.to_lowercase_str()
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
    let client = get_http_client();

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
        .context("failed to send request")?;

    handle_orders_response(response, &callable_url).await
}

/// Handles the HTTP response for orders requests.
async fn handle_orders_response(response: reqwest::Response, url: &str) -> Result<MarketOrders> {
    match response.status() {
        reqwest::StatusCode::OK => {
            let text = response.text().await.context("failed to read response")?;
            log::trace!("API response: {}", text);
            serde_json::from_str(&text).context("failed to parse market orders")
        }
        reqwest::StatusCode::TOO_MANY_REQUESTS => {
            log::error!("Rate Limit reached - pausing for 5 secs");
            anyhow::bail!("rate limited")
        }
        reqwest::StatusCode::UNAUTHORIZED => {
            log::error!("Authentication failed for request {}", url);
            anyhow::bail!("unauthorized")
        }
        other => {
            log::error!(
                "Unexpected error in service call: {:?}; url: {}",
                other,
                url
            );
            anyhow::bail!("HTTP {}", other)
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
                .with_context(|| format!("market not found for order: {}", order.market))?
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
            .context("failed to parse original_size")?,
        size_matched: order
            .size_matched
            .parse::<f64>()
            .context("failed to parse size_matched")?,
        price: order
            .price
            .parse::<f64>()
            .context("failed to parse price")?,
        outcome: order.outcome,
        expiration: order.expiration,
        order_type: order.order_type,
    })
}
