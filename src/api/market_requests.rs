use std::collections::HashMap;

use anyhow::{Context, Result};
use reqwest::Method;

use crate::api::http_client::get_http_client;
use crate::{MarketsResponse, PolyResponseMarket};

use super::{WebserviceRequest, GAMMA_API, GET_MARKET, GET_MARKETS, WITH_SLUG};

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

    pub fn with_condition_ids(&mut self, condition_ids: &[String]) {
        self.args.push((
            "condition_ids".to_string(),
            Self::format_condition_ids_query(condition_ids),
        ));
    }

    pub fn with_slug(&mut self, slug: &str) {
        self.url = self.url.to_owned() + WITH_SLUG + slug;
    }

    fn format_condition_ids_query(condition_ids: &[String]) -> String {
        condition_ids.join("&condition_ids=")
    }
}

pub async fn map_multiple_market_by_condition_ids_ws(
    condition_ids: &[String],
) -> Result<HashMap<String, PolyResponseMarket>> {
    let mut markets_map: HashMap<String, PolyResponseMarket> = HashMap::new();

    let markets_vec = load_markets_by_condition_ids(condition_ids, 0)
        .await
        .context("failed to load markets by condition IDs")?;

    for m in markets_vec.into_iter() {
        if let Some(condition_id) = m.condition_id.clone() {
            markets_map.insert(condition_id, m);
        }
    }

    Ok(markets_map)
}

pub async fn load_markets_by_condition_ids(
    condition_ids: &[String],
    next_offset: i32,
) -> Result<MarketsResponse> {
    let client = get_http_client(None);

    let mut web_service_request = WebserviceRequest::new_markets_ws_request();
    web_service_request.with_condition_ids(condition_ids);

    let (_, result) = WebserviceRequest::fetch_batch::<MarketsResponse>(
        client,
        &web_service_request,
        next_offset,
    )
    .await;

    result.context("no markets found for condition IDs")
}

pub async fn fetch_market_by_slug(slug: &str) -> Result<PolyResponseMarket> {
    let client = get_http_client(None);

    let mut web_service_request = WebserviceRequest::new_markets_ws_request();
    web_service_request.with_slug(slug);

    WebserviceRequest::fetch_one::<PolyResponseMarket>(client, &web_service_request)
        .await
        .with_context(|| format!("market not found for slug: {slug}"))
}
