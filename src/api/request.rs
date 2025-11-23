use reqwest::Method;

use super::add_param_to_url;

/// Represents an HTTP request to the Polymarket API
pub struct WebserviceRequest {
    pub api: String,
    pub url: String,
    pub method: Method,
    pub args: Vec<(String, String)>,
    pub body: Option<String>,
}

impl WebserviceRequest {
    pub fn get_limit(&self) -> i32 {
        for (name, value) in self.args.iter() {
            if name.eq("limit") {
                return value.parse().unwrap();
            }
        }
        100
    }

    pub fn add_arg(&mut self, name: String, value: String) {
        self.args.push((name, value));
    }

    pub fn get_callable_url(&self, next_offset: i32) -> String {
        let api = &self.api;
        let url = &self.url;
        let limit = self.get_limit();

        let mut callable_url = format!("{api}{url}?limit={limit}&offset={next_offset}");

        for (param_name, param_value) in self.args.iter() {
            add_param_to_url(&mut callable_url, param_name.as_str(), param_value.as_str());
        }
        callable_url
    }

    pub fn get_body(&self) -> String {
        self.body.clone().unwrap()
    }
}
