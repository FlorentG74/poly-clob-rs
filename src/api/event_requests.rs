use reqwest::Method;

use super::{WebserviceRequest, GAMMA_API, GET_EVENTS, GET_EVENT_SERIES};

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
