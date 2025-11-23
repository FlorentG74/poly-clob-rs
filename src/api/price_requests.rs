use reqwest::Method;
use serde::Serialize;

use super::{WebserviceRequest, CLOB_API, GET_PRICES};

#[derive(Serialize)]
struct PolymarketPriceRequest {
    token_id: String,
    side: String,
}

impl WebserviceRequest {
    pub fn new_polymarket_price_request(token_ids: &[String]) -> Self {
        let args = Vec::<(String, String)>::new();

        let body = build_prices_query(token_ids);

        WebserviceRequest {
            api: CLOB_API.to_string(),
            url: GET_PRICES.to_string(),
            method: Method::POST,
            args,
            body: Some(body),
        }
    }
}

fn build_prices_query(token_ids: &[String]) -> String {
    let mut instruments = Vec::<PolymarketPriceRequest>::new();

    for token_id in token_ids {
        instruments.push(PolymarketPriceRequest {
            token_id: token_id.to_string(),
            side: "sell".to_string(),
        });
        instruments.push(PolymarketPriceRequest {
            token_id: token_id.to_string(),
            side: "buy".to_string(),
        });
    }

    serde_json::to_string(&instruments).unwrap()
}
