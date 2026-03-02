use crate::api::error::Result;
use reqwest::Method;
use typed_builder::TypedBuilder;

use crate::api::http_client::get_http_client;
use super::{WebserviceRequest, GAMMA_API, GET_EVENTS, GET_EVENT_SERIES};
use crate::models::event::PolyResponseEvent;
use crate::models::event_series::PolyResponseEventSeries;

impl WebserviceRequest {
    pub fn new_event_by_id_request(id: &str) -> Self {
        WebserviceRequest {
            api: GAMMA_API.to_string(),
            url: GET_EVENTS.to_string(),
            method: Method::GET,
            with_pagination: false,
            args: vec![("id".to_string(), id.to_string())],
            body: None,
        }
    }

    pub fn new_event_series_request(slug: &str) -> Self {
        WebserviceRequest {
            api: GAMMA_API.to_string(),
            url: GET_EVENT_SERIES.to_string(),
            method: Method::GET,
            with_pagination: false,
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
            with_pagination: false,
            args: vec![("slug".to_string(), self.slug.to_string())],
            body: None,
        };

        let callable_url = web_service_request.get_callable_url(0);
        let events =
            WebserviceRequest::fetch_one::<Vec<PolyResponseEvent>>(client, &web_service_request)
                .await?;

        events.into_iter().next().ok_or_else(|| {
            crate::ApiError::NotFound {
                url: callable_url,
                resource: format!("event with slug: {}", self.slug),
            }
            .into()
        })
    }
}

/// Request builder for fetching an event series by slug.
///
/// Returns the series with its list of events, useful for finding
/// the current/upcoming event in a recurring series.
#[derive(TypedBuilder)]
pub struct EventSeriesRequest<'a> {
    #[builder(setter(into))]
    pub slug: &'a str,
}

impl<'a> EventSeriesRequest<'a> {
    pub async fn execute(&self) -> Result<PolyResponseEventSeries> {
        let client = get_http_client(None);

        let web_service_request = WebserviceRequest {
            api: GAMMA_API.to_string(),
            url: GET_EVENT_SERIES.to_string(),
            method: Method::GET,
            with_pagination: false,
            args: vec![("slug".to_string(), self.slug.to_string())],
            body: None,
        };

        let callable_url = web_service_request.get_callable_url(0);
        let series =
            WebserviceRequest::fetch_one::<Vec<PolyResponseEventSeries>>(client, &web_service_request)
                .await?;

        series.into_iter().next().ok_or_else(|| {
            crate::ApiError::NotFound {
                url: callable_url,
                resource: format!("event series with slug: {}", self.slug),
            }
            .into()
        })
    }
}
