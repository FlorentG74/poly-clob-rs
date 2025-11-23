use reqwest::Method;

use super::{WebserviceRequest, DATA_API, POSITIONS};

impl WebserviceRequest {
    pub fn new_positions_ws_request(user: &str) -> Self {
        let mut args = Vec::<(String, String)>::new();
        args.push((String::from("user"), user.to_string()));

        WebserviceRequest {
            api: DATA_API.to_string(),
            url: POSITIONS.to_string(),
            method: Method::GET,
            args,
            body: None,
        }
    }
}
