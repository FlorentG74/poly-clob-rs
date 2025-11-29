//! Order placement and management request builders.
//!
//! This module provides functions for placing and managing orders on the Polymarket CLOB.

use crate::api::auth::{build_l2_headers, get_timestamp, get_zero_address};
use crate::api::response_handler::handle_api_response;
use crate::models::{Account, AssetType, Order, OrderType, Side};
use crate::{market_requests, webservice, MarketOrders, OpenOrder, ORDERS};
use reqwest::header::*;

use super::clob_endpoints::{CLOB_API, GET_API_KEYS, GET_BALANCE_ALLOWANCE, POST_ORDER};

// Note: The following imports are commented out until the related functions are fully implemented
// use crate::OpenOrder;
// use crate::models::clob_orders::MarketOrders;
// use super::clob_endpoints::ORDERS;
// use crate::api::auth::add_param_to_url;

/// Places a limit order on the Polymarket CLOB.
///
/// # Arguments
///
/// * `signer` - The account to sign and place the order with
/// * `price` - The price per share (0.0 to 1.0)
/// * `size` - The size of the order in token quantity (number of shares)
/// * `side` - Whether this is a buy or sell order
/// * `token_id` - The token ID for the outcome
/// * `order_type` - The type of order (FOK, FAK, GTC, or GTD)
/// * `expiration` - Order expiration timestamp (required for GTD, must be 0 for others)
///
/// # Order Types
///
/// * `FOK` (Fill-Or-Kill) - Must be executed immediately in full or cancelled. Expiration must be 0.
/// * `FAK` (Fill-And-Kill) - Execute immediately for available shares, cancel the rest. Expiration must be 0.
/// * `GTC` (Good-Til-Cancelled) - Active until fulfilled or manually cancelled. Expiration must be 0.
/// * `GTD` (Good-Til-Date) - Active until specified date. Expiration must be non-zero Unix timestamp.
///
/// # Returns
///
/// Returns `Ok(String)` with the API response on success, or `Err(String)` with an error message on failure.
///
/// # Errors
///
/// Returns an error if:
/// * Expiration is non-zero for FOK/FAK/GTC orders
/// * Expiration is zero for GTD orders
///
/// # Example
///
/// ```no_run
/// use poly_clob_rs::{Account, Side, OrderType, api::order_requests::place_limit_order};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let account = Account::load_poly_account();
///
/// // FOK order (expiration must be 0)
/// let result = place_limit_order(
///     &account,
///     0.52,
///     10.0,
///     Side::Buy,
///     "1234567890",
///     OrderType::FOK,
///     0
/// ).await?;
///
/// // GTD order (expiration must be non-zero)
/// let expiration_time = 1735689600; // Some future timestamp
/// let result = place_limit_order(
///     &account,
///     0.52,
///     10.0,
///     Side::Buy,
///     "1234567890",
///     OrderType::GTD,
///     expiration_time
/// ).await?;
///
/// println!("Order placed: {}", result);
/// # Ok(())
/// # }
/// ```
pub async fn place_limit_order(
    signer: &Account,
    price: f64,
    size: f64,
    side: Side,
    token_id: &str,
    order_type: OrderType,
    expiration: i64,
) -> Result<String, String> {
    // Validate expiration based on order type
    match order_type {
        OrderType::GTD => {
            if expiration == 0 {
                return Err("GTD orders require a non-zero expiration timestamp".to_string());
            }
        }
        OrderType::FOK | OrderType::FAK | OrderType::GTC => {
            if expiration != 0 {
                return Err(format!(
                    "{} orders must have expiration set to 0",
                    order_type
                ));
            }
        }
    }

    let client = reqwest::Client::builder()
        .build()
        .map_err(|e| format!("Error creating HTTP client: {}", e))?;

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

    let (maker_amount, taker_amount) = if side == Side::Buy {
        // BUY: giving USDC (maker), receiving tokens (taker)
        // maker_amount = size × price (USDC with 4 decimal precision)
        // taker_amount = size (tokens with 2 decimal precision)
        let maker_amount = ((10000.0 * size * price).round() * 100.0).round() as i32;
        let taker_amount = ((100.0 * size).round() * 10000.0).round() as i32;
        (maker_amount, taker_amount)
    } else {
        // SELL: giving tokens (maker), receiving USDC (taker)
        // maker_amount = size (tokens with 2 decimal precision)
        // taker_amount = size × price (USDC with 4 decimal precision)
        let maker_amount = ((100.0 * size).round() * 10000.0).round() as i32;
        let taker_amount = ((10000.0 * size * price).round() * 100.0).round() as i32;
        (maker_amount, taker_amount)
    };

    let fee_rate_bps = 0;

    let order = Order::new(
        signer.poly_address.as_str(),
        signer.pub_key.as_str(),
        &get_zero_address(),
        token_id,
        maker_amount,
        taker_amount,
        expiration,
        fee_rate_bps,
        side,
        order_type,
    );

    let salt = get_timestamp();
    let nonce = 0; // Nonce for order signing
    let body = order.build_order_query_body(
        salt.as_str(),
        nonce,
        signer.api_key.as_str(),
        signer.private_key.as_str(),
    );

    let l2_headers = build_l2_headers(&signer, method, request_path, &body, &salt);

    log::debug!("Signed Order body: {}", &body);

    let response = client
        .post(&callable_url)
        .header(CONTENT_TYPE, "application/json")
        .header(ACCEPT, "application/json")
        .headers(l2_headers)
        .body(body)
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

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

pub async fn get_all_open_orders(signer: &Account) -> Vec<OpenOrder> {
    get_open_orders_by_market(signer, "").await
}

/// Get balance and allowance for an account.
///
/// # Arguments
///
/// * `signer` - The account to query
/// * `asset_type` - The type of asset (e.g., COLLATERAL)
/// * `token_id` - The token ID to check
/// * `signature_type` - Optional signature type (-1 to omit)
///
/// # Returns
///
/// Returns `Ok(String)` with the API response on success, or `Err(String)` on failure.
pub async fn get_balance_allowance(
    signer: &Account,
    asset_type: AssetType,
    token_id: &str,
    signature_type: i32,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .build()
        .map_err(|e| format!("Error creating HTTP client: {}", e))?;

    let method = "GET";
    let request_path = GET_BALANCE_ALLOWANCE;
    let body = "";

    let mut callable_url = format!("{}{}", CLOB_API, request_path);
    webservice::add_param_to_url(&mut callable_url, "asset_type", asset_type.into());
    webservice::add_param_to_url(&mut callable_url, "token_id", token_id);

    if signature_type != -1 {
        let signature_str = format!("{}", signature_type);
        webservice::add_param_to_url(&mut callable_url, "signature_type", signature_str.as_str());
    }

    let l2_headers = build_l2_headers(signer, method, request_path, body, "");

    let response = client
        .get(&callable_url)
        .header(CONTENT_TYPE, "application/json")
        .header(ACCEPT, "application/json")
        .headers(l2_headers)
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    handle_api_response(response, &callable_url).await
}

/// Get API keys for an account.
///
/// # Arguments
///
/// * `signer` - The account to query
/// * `signature_type` - Optional signature type (-1 to omit)
///
/// # Returns
///
/// Returns `Ok(String)` with the API response on success, or `Err(String)` on failure.
pub async fn get_api_key(signer: &Account, signature_type: i32) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .build()
        .map_err(|e| format!("Error creating HTTP client: {}", e))?;

    let method = "GET";
    let request_path = GET_API_KEYS;
    let body = "";

    let mut callable_url = format!("{}{}", CLOB_API, request_path);

    if signature_type != -1 {
        let signature_str = format!("{}", signature_type);
        webservice::add_param_to_url(&mut callable_url, "signature_type", signature_str.as_str());
    }

    let l2_headers = build_l2_headers(signer, method, request_path, body, "");

    let response = client
        .get(&callable_url)
        .header(CONTENT_TYPE, "application/json")
        .header(ACCEPT, "application/json")
        .headers(l2_headers)
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    handle_api_response(response, &callable_url).await
}

pub async fn get_open_orders_by_market(signer: &Account, market_id: &str) -> Vec<OpenOrder> {
    let client = reqwest::Client::builder()
        .build()
        .expect("Error creating client");

    let method = "GET";
    let request_path = ORDERS;
    let body = "";

    let mut callable_url = format!("{}{}", CLOB_API, request_path);

    webservice::add_param_to_url(&mut callable_url, "market", market_id);

    let l2_headers = build_l2_headers(signer, method, request_path, body, "");

    let response = client
        .get(&callable_url)
        .header(CONTENT_TYPE, "application/json")
        .header(ACCEPT, "application/json")
        .headers(l2_headers)
        .send()
        .await
        .unwrap();

    let mut open_orders = Vec::<OpenOrder>::new();

    match response.status() {
        reqwest::StatusCode::OK => {
            // on success, parse our JSON to an APIResponse
            let text = response.text().await.expect("msg");
            let market_orders: MarketOrders =
                serde_json::from_str::<MarketOrders>(&text).expect("x");

            // If no orders, return empty vec
            if market_orders.data.is_empty() {
                return open_orders;
            }

            // Attach markets to orders & convert to Vec<OpenOrder>
            // Retrieve unique condition_ids and load markets
            let mut condition_ids = Vec::<String>::new();
            for market_position in market_orders.data.iter() {
                condition_ids.push(market_position.market.clone());
            }
            condition_ids.sort();
            condition_ids.dedup();

            log::trace!("API response: {}", text);

            // Load markets as a batch - aither from DB or webservice
            let markets = market_requests::map_multiple_market_by_condition_ids_ws(&condition_ids)
                .await
                .unwrap();

            // Normalize positions structure
            for market_order in market_orders.data {
                let market = markets
                    .get(&market_order.market)
                    .expect("Cant attach Market to Position")
                    .clone();

                open_orders.push(OpenOrder {
                    id: market_order.id,
                    status: market_order.status,
                    owner: market_order.owner,
                    maker_address: market_order.maker_address,
                    market,
                    asset_id: market_order.asset_id,
                    side: market_order.side,
                    original_size: market_order
                        .original_size
                        .parse::<f64>()
                        .expect("Can't parse original_size"),
                    size_matched: market_order
                        .size_matched
                        .parse::<f64>()
                        .expect("Can't parse size_matched"),
                    price: market_order
                        .price
                        .parse::<f64>()
                        .expect("Can't parse price"),
                    outcome: market_order.outcome,
                    expiration: market_order.expiration,
                    order_type: market_order.order_type,
                });
            }

            open_orders
        }
        reqwest::StatusCode::TOO_MANY_REQUESTS => {
            log::error!("Rate Limit reached - pausing for 5 secs");
            open_orders
        }
        reqwest::StatusCode::UNAUTHORIZED => {
            log::error!("Authentication failed for request {}", callable_url);
            open_orders
        }
        other => {
            log::error!(
                "Unexpected error in service call - Returning empty dataset: {:?}; url: {}",
                other,
                callable_url
            );
            Vec::<OpenOrder>::new()
        }
    }
}
