use std::collections::HashMap;

use reqwest::Method;
use string_builder::Builder;

use crate::{MarketsResponse, PolyResponseMarket};

use super::{webservice, WebserviceRequest, GAMMA_API, GET_MARKET, GET_MARKETS, WITH_SLUG};

impl WebserviceRequest {
    pub fn new_market_ws_request() -> Self {
        let args = Vec::<(String, String)>::new();

        WebserviceRequest {
            api: GAMMA_API.to_string(),
            url: GET_MARKET.to_string(),
            method: Method::GET,
            args,
            body: None,
        }
    }

    pub fn new_markets_ws_request() -> Self {
        let args = Vec::<(String, String)>::new();

        WebserviceRequest {
            api: GAMMA_API.to_string(),
            url: GET_MARKETS.to_string(),
            method: Method::GET,
            args,
            body: None,
        }
    }

    pub fn with_active_only(&mut self) {
        self.args.push(("active".to_string(), "true".to_string()));
        self.args.push(("closed".to_string(), "false".to_string()));
    }

    pub fn with_from_start_date(&mut self, start_date_min: String) {
        self.args
            .push(("start_date_min".to_string(), start_date_min));
    }

    pub fn with_tag_id(&mut self, tag_id: &str) {
        self.args.push(("tag_id".to_string(), tag_id.to_string()));
    }

    pub fn with_related_tags(&mut self) {
        self.args
            .push(("related_tags".to_string(), "true".to_string()));
    }

    pub fn with_condition_ids(&mut self, condition_ids: &Vec<String>) {
        self.args.push((
            "condition_ids".to_string(),
            Self::format_condition_ids_query(condition_ids),
        ));
    }

    pub fn with_slug(&mut self, slug: &str) {
        self.url = self.url.to_owned() + WITH_SLUG + slug;
    }

    fn format_condition_ids_query(condition_ids: &Vec<String>) -> String {
        let mut builder = Builder::default();

        let mut it = condition_ids.iter().peekable();
        while let Some(condition_id) = it.next() {
            builder.append(condition_id.clone());
            if it.peek().is_some() {
                builder.append("&condition_ids=");
            }
        }

        builder.string().expect("Error in String conversion")
    }
}

    pub async fn map_multiple_market_by_condition_ids_ws(
        condition_ids: &Vec<String>,
    ) -> Result<HashMap<String, PolyResponseMarket>, String> {
        let mut markets_map: HashMap<String, PolyResponseMarket> = HashMap::new();

        //If market isnt available in database, try to load it from the API
        let markets_vec =
            load_market_by_condition_ids(condition_ids, 0)
                .await
                .unwrap();

        for m in markets_vec.into_iter() {
            let condition_id = m.condition_id.clone();
            markets_map.insert(condition_id.unwrap(), m);
        }

        Ok(markets_map)
    }

    
    pub async fn load_market_by_condition_ids(
        condition_ids: &Vec<String>,
        next_offset: i32,
    ) -> Option<MarketsResponse> {

        let client = reqwest::Client::builder()
            .build()
            .expect("Error creating client");

        let mut web_service_request = WebserviceRequest::new_markets_ws_request();
        web_service_request.with_condition_ids(condition_ids);

        let (_, result) =
            webservice::fetch_batch::<MarketsResponse>(&client, &web_service_request, next_offset)
                .await;

        result
    }

    pub async fn fetch_market_by_slug(slug: &str) -> Option<PolyResponseMarket> {
        let client = reqwest::Client::builder()
            .build()
            .expect("Error creating client");

        let mut web_service_request = WebserviceRequest::new_markets_ws_request();
        web_service_request.with_slug(slug);

        webservice::fetch_one::<PolyResponseMarket>(&client, &web_service_request).await
    }