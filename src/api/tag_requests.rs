use reqwest::Method;

use super::{WebserviceRequest, GAMMA_API, GET_TAGS};

impl WebserviceRequest {
    pub fn new_polymarket_tag_request() -> Self {
        WebserviceRequest {
            api: GAMMA_API.to_string(),
            url: GET_TAGS.to_string(),
            method: Method::GET,
            args: Vec::<(String, String)>::new(),
            body: None,
        }
    }
}
