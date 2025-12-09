use reqwest::Method;
use serde::Serialize;

use super::{WebserviceRequest, CLOB_API, GET_PRICES};
use crate::models::Side;

#[derive(Serialize)]
struct PolymarketPriceRequest {
    token_id: String,
    #[serde(serialize_with = "serialize_side_lowercase")]
    side: Side,
}

fn serialize_side_lowercase<S>(side: &Side, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(side.to_lowercase_str())
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
    let instruments: Vec<PolymarketPriceRequest> = token_ids
        .iter()
        .flat_map(|token_id| {
            [
                PolymarketPriceRequest {
                    token_id: token_id.to_string(),
                    side: Side::Sell,
                },
                PolymarketPriceRequest {
                    token_id: token_id.to_string(),
                    side: Side::Buy,
                },
            ]
        })
        .collect();

    // This serialization cannot fail for this simple struct
    serde_json::to_string(&instruments).expect("price request serialization is infallible")
}
