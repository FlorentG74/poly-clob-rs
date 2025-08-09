use std::time::Duration;
use tokio::time::sleep;

use core::str;
use reqwest::header::*;

use serde::{Deserialize, Serialize};

use crate::model::{Account, AssetType, OpenOrder, Order};

use crate::controller::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct MarketOrders {
    pub data: Vec<MarketOrder>,
    next_cursor: String,
    limit: i64,
    count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MarketOrder {
    pub id: String,
    pub status: String,
    pub owner: String,
    pub maker_address: String,
    pub market: String,
    pub asset_id: String,
    pub side: String,
    pub original_size: String,
    pub size_matched: String,
    pub price: String,
    pub outcome: String,
    pub expiration: String,
    pub order_type: String,
    pub associate_trades: Vec<Option<serde_json::Value>>,
    pub created_at: i64,
}

// Reserved for future use
#[allow(dead_code)]
pub async fn get_balance_allowance(
    signer: &Account,
    asset_type: AssetType,
    token_id: &str,
    signature_type: i32,
) {
    let client = reqwest::Client::builder()
        .build()
        .expect("Error creating client");

    let method = "GET";
    let request_path = GET_BALANCE_ALLOWANCE;
    let body = "";

    let mut url = format!("https://clob.polymarket.com{}", request_path);
    add_param_to_url(&mut url, "asset_type", asset_type.into());
    add_param_to_url(&mut url, "token_id", token_id);

    if signature_type != -1 {
        let signature_str = format!("{}", signature_type);
        add_param_to_url(&mut url, "signature_type", signature_str.as_str());
    }

    //https://clob.polymarket.com/balance-allowance?asset_type=COLLATERAL&signature_type=0

    let l2_headers = build_l2_headers(signer, method, request_path, body);

    let response = client
        .get(&url)
        .header(CONTENT_TYPE, "application/json")
        .header(ACCEPT, "application/json")
        .headers(l2_headers)
        .send()
        .await
        .unwrap();

    match response.status() {
        reqwest::StatusCode::OK => {
            // on success, parse our JSON to an APIResponse
            let text = response.text().await.expect("msg");

            log::debug!("API response: {}", text);
        }
        reqwest::StatusCode::TOO_MANY_REQUESTS => {
            log::warn!("Rate Limit reached - pausing for 5 secs");
        }
        reqwest::StatusCode::UNAUTHORIZED => {
            log::error!("Authentication failed for request {}", url);
        }
        other => {
            log::error!("Unexpected error in service call: {:?}", other);
        }
    }
}

pub async fn get_all_open_orders(signer: &Account) -> Vec<OpenOrder> {
    return get_open_orders_by_market(signer, "").await;
}

pub async fn get_open_orders_by_market(signer: &Account, market_id: &str) -> Vec<OpenOrder> {
    let client = reqwest::Client::builder()
        .build()
        .expect("Error creating client");

    let method = "GET";
    let request_path = ORDERS;
    let body = "";

    let mut url = format!("https://clob.polymarket.com{}", request_path);
    add_param_to_url(&mut url, "market", market_id);

    let l2_headers = build_l2_headers(signer, method, request_path, body);

    let response = client
        .get(&url)
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

            // Attach markets to orders & convert to Vec<OpenOrder>
            // Retrieve unique condition_ids and load markets
            let mut condition_ids = Vec::<String>::new();
            for market_position in market_orders.data.iter() {
                condition_ids.push(market_position.market.clone());
            }
            condition_ids.sort();
            condition_ids.dedup();

            log::debug!("API response: {}", text);

            // Load markets as a batch - aither from DB or webservice
            let markets = MarketController::get_multiple_market_by_condition_ids_ws(&condition_ids)
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
            log::warn!("Rate Limit reached - pausing for 5 secs");
            sleep(Duration::from_millis(5000)).await;
            open_orders
        }
        reqwest::StatusCode::UNAUTHORIZED => {
            log::error!("Authentication failed for request {}", url);
            open_orders
        }
        other => {
            log::error!(
                "Unexpected error in service call - Returning empty dataset: {:?}",
                other
            );
            Vec::<OpenOrder>::new()
        }
    }
}

// Reserved for future use
#[allow(dead_code)]
pub async fn place_limit_order(
    signer: &Account,
    _price: f64,
    _size: f64,
    _side: &str,
    token_id: &str,
) -> i32 {
    let client = reqwest::Client::builder()
        .build()
        .expect("Error creating client");

    let method = "POST";
    let request_path = POST_ORDER;

    let url = format!("https://clob.polymarket.com{}", request_path);

    let maker_amount = 50000000;
    let taker_amount = 100000000;
    let expiration: i64 = 1000000000000;
    let fee_rate_bps = 0;
    let side = 0;

    let mut order = Order::new(
        signer.pub_key.as_str(),
        signer.pub_key.as_str(),
        &get_zero_address(),
        token_id,
        maker_amount,
        taker_amount,
        expiration,
        fee_rate_bps,
        side,
        "GTC",
    );

    let salt = get_timestamp();
    let body = order.build_order_query_body(
        salt.as_str(),
        signer.api_key.as_str(),
        signer.private_key.as_str(),
    );

    //let l1_headers = build_l1_headers(signer,0);
    let l2_headers = build_l2_headers(signer, method, request_path, &body);

    log::debug!("Signed Order body: {}", &body);

    let response = client
        .post(&url)
        .header(CONTENT_TYPE, "application/json")
        .header(ACCEPT, "application/json")
        .headers(l2_headers)
        .body(body)
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

            // Attach markets to orders & convert to Vec<OpenOrder>
            // Retrieve unique condition_ids and load markets
            let mut condition_ids = Vec::<String>::new();
            for market_position in market_orders.data.iter() {
                condition_ids.push(market_position.market.clone());
            }
            condition_ids.sort();
            condition_ids.dedup();

            log::debug!("API response: {}", text);

            // Load markets as a batch - either from DB or webservice
            let markets = MarketController::get_multiple_market_by_condition_ids_ws(&condition_ids)
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

            500
        }
        reqwest::StatusCode::BAD_REQUEST => {
            log::error!("Error encountered while posting the order");
            log::error!("API Response: {:?}", response.text().await);
            -1
        }
        reqwest::StatusCode::TOO_MANY_REQUESTS => {
            log::warn!("Rate Limit reached - pausing for 5 secs");
            sleep(Duration::from_millis(5000)).await;
            -1
        }
        reqwest::StatusCode::UNAUTHORIZED => {
            log::error!("Authentication failed for request {}", url);
            -1
        }
        other => {
            log::error!("Unexpected error in service call: {:?}", other);
            -1
        }
    }
}


#[cfg(test)]
mod clob_auth_tests {
    use crate::{controller::get_open_orders_by_market, model::Account};


    #[tokio::test]
    async fn test_load_orders() {

    let signer = &Account::actual_account_from_env();
    let market_id: &str = "0x5e8e585d855c4288c3805064e74fc7ea1dab47dc0e9b42a0dbd9ca5f49c997f9";

    let orders = get_open_orders_by_market(signer, market_id).await;

    print!("{:?}", orders);

    assert_eq!("","");
    }
}
