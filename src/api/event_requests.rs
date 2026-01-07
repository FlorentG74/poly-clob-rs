use anyhow::{Context, Result};
use reqwest::Method;
use typed_builder::TypedBuilder;

use crate::api::http_client::get_http_client;
use super::{WebserviceRequest, GAMMA_API, GET_EVENTS, GET_EVENT_SERIES};
use crate::models::event::PolyResponseEvent;

impl WebserviceRequest {
    pub fn new_event_by_id_request(id: &str) -> Self {
        WebserviceRequest {
            api: GAMMA_API.to_string(),
            url: GET_EVENTS.to_string(),
            method: Method::GET,
            args: vec![("id".to_string(), id.to_string())],
            body: None,
        }
    }

    pub fn new_event_series_request(slug: &str) -> Self {
        WebserviceRequest {
            api: GAMMA_API.to_string(),
            url: GET_EVENT_SERIES.to_string(),
            method: Method::GET,
            args: vec![("slug".to_string(), slug.to_string())],
            body: None,
        }
    }
}

/// Request builder for fetching a single event by slug.
/// The events endpoint returns complete market data including token IDs,
/// unlike the markets endpoint which may return incomplete data for recurring markets.
#[derive(TypedBuilder)]
pub struct EventBySlugRequest<'a> {
    #[builder(setter(into))]
    pub slug: &'a str,
}

impl<'a> EventBySlugRequest<'a> {
    pub async fn execute(&self) -> Result<PolyResponseEvent> {
        let client = get_http_client(None);

        let web_service_request = WebserviceRequest {
            api: GAMMA_API.to_string(),
            url: GET_EVENTS.to_string(),
            method: Method::GET,
            args: vec![("slug".to_string(), self.slug.to_string())],
            body: None,
        };

        let events = WebserviceRequest::fetch_batch::<Vec<PolyResponseEvent>>(
            &client,
            &web_service_request,
            0,
        )
        .await
        .1
        .with_context(|| format!("event not found for slug: {}", self.slug))?;

        events.into_iter().next()
            .with_context(|| format!("no event returned for slug: {}", self.slug))
    }
}
