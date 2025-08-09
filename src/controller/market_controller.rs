#![warn(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use super::load_all;
use super::webservice;
use super::WebserviceRequest;
use crate::model::Market;
use crate::model::MarketsResponse;
use crate::model::PolyResponseMarket;
use crate::model::PolymarketTagsResponse;
use crate::model::Tag;

use std::collections::HashMap;
use std::io::Error;

pub const BINANCE: &str = "Binance";
pub const POLYMARKET: &str = "Polymarket";
pub const MATIC: &str = "Matic";

pub const USDT: &str = "USDT";

#[derive(Debug)]
pub struct MarketController {}

impl Default for MarketController {
    fn default() -> Self {
        Self::new()
    }
}

impl MarketController {
    pub fn new() -> MarketController {
        MarketController {}
    }

    pub async fn get_multiple_market_by_condition_ids_ws(
        condition_ids: &Vec<String>,
    ) -> Result<HashMap<String, PolyResponseMarket>, Error> {
        let mut markets_map: HashMap<String, PolyResponseMarket> = HashMap::new();

        //If market isnt available in database, try to load it from the API
        let markets_vec =
            MarketController::load_and_store_market_by_condition_ids(condition_ids, 0)
                .await
                .unwrap();

        for m in markets_vec.into_iter() {
            let condition_id = m.condition_id.clone();
            markets_map.insert(condition_id.unwrap(), m);
        }

        Ok(markets_map)
    }

    pub async fn load_and_store_market_by_condition_ids(
        condition_ids: &Vec<String>,
        next_offset: i32,
    ) -> Option<MarketsResponse> {
        let client = reqwest::Client::builder()
            .build()
            .expect("Error creating client");

        let mut web_service_request = WebserviceRequest::new_markets_ws_request();
        web_service_request.with_condition_ids(condition_ids);

        let (_, result) =
            webservice::load_batch::<MarketsResponse>(&client, &web_service_request, next_offset)
                .await;

        return result;
    }

}
