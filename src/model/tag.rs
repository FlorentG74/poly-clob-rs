use std::collections::HashMap;

use reqwest::Method;
use serde::Deserialize;

use crate::controller::{WebserviceRequest, WebserviceResponse, GAMMA_API, GET_TAGS};

pub struct Tag {
    pub id: i32,
    pub label: String,
    pub slug: String,
    pub in_scope: bool,
}

impl Tag {
    pub fn from_poly_response_tag(pmt: &PolyResponseTag) -> Self {
        Tag {
            id: pmt.id.parse().unwrap(),
            label: pmt.label.clone().unwrap_or("default".to_string()),
            slug: pmt.slug.clone().unwrap_or("default".to_string()),
            in_scope: false,
        }
    }
}

pub type PolymarketTagsResponse = Vec<PolyResponseTag>;

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PolyResponseTag {
    id: String,
    label: Option<String>,
    slug: Option<String>,
    published_at: Option<String>,
    created_at: Option<String>,
    updated_at: Option<String>,
    #[serde(rename = "_sync")]
    sync: Option<bool>,
}
#[warn(dead_code)]

impl WebserviceRequest {
    pub fn new_polymarket_tag_request() -> Self {
        return WebserviceRequest {
            api: GAMMA_API.to_string(),
            url: GET_TAGS.to_string(),
            method: Method::GET,
            args: HashMap::<String, String>::new(),
            body: None,
        };
    }
}

impl WebserviceResponse for PolymarketTagsResponse {
    async fn store(&self) {
    }

    fn nb_results(&self) -> usize {
        self.len()
    }
}

#[cfg(test)]
mod tags_tests {

}
