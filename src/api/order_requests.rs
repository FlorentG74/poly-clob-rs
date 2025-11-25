//! Order placement and management request builders.
//!
//! This module provides functions for placing and managing orders on the Polymarket CLOB.

use crate::api::auth::{build_l2_headers, get_timestamp, get_zero_address};
use crate::models::{Account, Order, OrderType, Side};
use crate::{market_requests, webservice, MarketOrders, OpenOrder, ORDERS};
use reqwest::header::*;

use super::clob_endpoints::{CLOB_API, POST_ORDER};

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
/// * `size` - The size of the order in USDC
/// * `side` - Whether this is a buy or sell order
/// * `token_id` - The token ID for the outcome
///
/// # Returns
///
/// Returns `Ok(String)` with the API response on success, or `Err(String)` with an error message on failure.
///
/// # Example
///
/// ```no_run
/// use poly_clob_rs::{Account, Side, api::order_requests::place_limit_order};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let account = Account::actual_account_from_env();
/// let result = place_limit_order(
///     &account,
///     0.52,
///     10.0,
///     Side::Buy,
///     "1234567890"
/// ).await?;
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
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .build()
        .map_err(|e| format!("Error creating HTTP client: {}", e))?;

    let method = "POST";
    let request_path = POST_ORDER;

    let callable_url = format!("{}{}", CLOB_API, request_path);

    // Note: maker amount supports a max accuracy of 2 decimals, taker amount a max of 4 decimals
    let maker_amount = ((100.0 * size).round() * 10000.0).round() as i32;

    let taker_amount = if side == Side::Buy {
        ((10000.0 * size / price).round() * 100.0).round() as i32
    } else {
        ((10000.0 * size * price).round() * 100.0).round() as i32
    };

    let expiration: i64 = 0;
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
        OrderType::FOK,
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

    log::trace!("Signed Order body: {}", &body);

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

    match response.status() {
        reqwest::StatusCode::OK => {
            let text = response
                .text()
                .await
                .map_err(|e| format!("Failed to read response: {}", e))?;

            log::debug!("Post Order response: {}", text);

            Ok(text)
        }
        reqwest::StatusCode::BAD_REQUEST => {
            log::error!(
                "Error encountered while posting the order: {}",
                order.side.to_lowercase_str()
            );
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            log::error!("API Response: {:?}", error_text);
            Err(format!("Bad request: {}", error_text))
        }
        reqwest::StatusCode::TOO_MANY_REQUESTS => {
            log::error!("Rate limit reached for request {}", callable_url);
            Err("Rate limit reached".to_string())
        }
        reqwest::StatusCode::UNAUTHORIZED => {
            log::error!("Authentication failed for request {}", callable_url);
            Err("Authentication failed".to_string())
        }
        other => {
            log::error!(
                "Unexpected error in service call: {:?}; url: {}",
                other,
                callable_url
            );
            Err(format!("Unexpected error: {:?}", other))
        }
    }
}

pub async fn get_all_open_orders(signer: &Account) -> Vec<OpenOrder> {
    get_open_orders_by_market(signer, "").await
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
