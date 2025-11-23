use reqwest::Method;
use string_builder::Builder;

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
