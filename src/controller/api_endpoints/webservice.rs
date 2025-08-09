use reqwest::{Client, Method, RequestBuilder};
use std::{collections::HashMap, time::Duration};
use tokio::time::sleep;

use crate::controller::add_param_to_url;

pub struct WebserviceRequest {
    pub api: String,
    pub url: String,
    pub method: Method,
    pub args: HashMap<String, String>,
    pub body: Option<String>,
}

impl WebserviceRequest {
    fn get_limit(&self) -> i32 {
        let lim = self.args.get("limit");
        match lim {
            Some(l) => l.parse().unwrap(),
            None => 100,
        }
    }

    pub fn add_arg(&mut self, name: String, value: String) {
        self.args.insert(name, value);
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
}

pub trait WebserviceResponse {
    #[allow(async_fn_in_trait)]
    async fn store(&self);

    fn nb_results(&self) -> usize;
}

pub fn get_client() -> Client {
    reqwest::Client::builder()
        .build()
        .expect("Error creating client")
}

pub async fn load_all<T: for<'a> serde::Deserialize<'a> + WebserviceResponse>(
    web_service_request: WebserviceRequest,
) {
    let mut next_offset: i32 = 0;

    let client = reqwest::Client::builder()
        .build()
        .expect("Error creating client");

    while next_offset > -1 {
        log::debug!("next_cursor {:?}", next_offset);
        (next_offset, _) = load_batch::<T>(&client, &web_service_request, next_offset).await;
    }
}

pub async fn load_batch<T: for<'a> serde::Deserialize<'a> + WebserviceResponse>(
    client: &Client,
    web_service_request: &WebserviceRequest,
    next_offset: i32,
) -> (i32, Option<T>) {
    let callable_url = web_service_request.get_callable_url(next_offset);

    let request: RequestBuilder;

    match web_service_request.method {
        Method::GET => {
            request = client.get(&callable_url);
        }
        Method::POST => {
            request = client
                .post(&callable_url)
                .body(web_service_request.get_body());
        }
        _ => {
            log::debug!("Unsupported Method");
            return (-1, None);
        }
    }

    let response = request.send().await.unwrap();

    match response.status() {
        reqwest::StatusCode::OK => {
            // on success, parse our JSON to an APIResponse
            let text = response
                .text()
                .await
                .expect("Error - can't extract API Response");
            log::debug!("API Response: {}", text);

            // Can be used to get more detailed error message in deserialization
            let _res = serde_json::from_str::<T>(&text).expect("");

            match serde_json::from_str::<T>(&text) {
                Ok(ws_response) => {
                    let nb_results_retrieved: i32 = ws_response.nb_results().try_into().unwrap();

                    log::debug!("Retrieved {:?} results", nb_results_retrieved);

                    if nb_results_retrieved > 0 {
                        ws_response.store().await;

                        if nb_results_retrieved == web_service_request.get_limit() {
                            (
                                next_offset + web_service_request.get_limit(),
                                Some(ws_response),
                            )
                        } else {
                            (-1, Some(ws_response))
                        }
                    } else {
                        (-1, None)
                    }
                }
                Err(_) => {
                    println!("Error - can't deserialize API Response");
                    (-1, None)
                }
            }
        }
        reqwest::StatusCode::TOO_MANY_REQUESTS => {
            log::warn!("Rate Limit reached - pausing for 5 secs");
            sleep(Duration::from_millis(5000)).await;
            (next_offset, None)
        }
        reqwest::StatusCode::UNAUTHORIZED => {
            log::error!("Authentication failed for request {}", callable_url);
            (next_offset, None)
        }
        other => {
            log::error!("Unexpected error in service call: {:?}", other);
            (next_offset, None)
        }
    }
}

#[cfg(test)]
mod market_data_controller_tests {
    use crate::{
        controller::{load_all, load_batch, PolymarketPricesResponse, WebserviceRequest},
        model::{MarketsResponse, PositionsResponse},
    };

    #[tokio::test]
    pub async fn test_load_all_markets() {
        let mut web_service_request = WebserviceRequest::new_markets_ws_request();

        web_service_request.with_active_only();
        web_service_request.with_from_start_date("1900-01-01T00:00:00.00000Z".to_string());

        web_service_request.with_tag_id("235");
        web_service_request.with_tag_id("1312");
        web_service_request.with_tag_id("21");

        web_service_request.with_related_tags();

        load_all::<MarketsResponse>(web_service_request).await;
    }

    #[tokio::test]
    pub async fn test_load_prices() {
        let mut instruments = Vec::<String>::new();
        instruments.push(
            "101669189743438912873361127612589311253202068943959811456820079057046819967115"
                .to_string(),
        );
        instruments.push(
            "113332423559050930347591987511234765387649957428761857688151517507261414072694"
                .to_string(),
        );

        let web_service_request = WebserviceRequest::new_polymarket_price_request(&instruments);

        let client = reqwest::Client::builder()
            .build()
            .expect("Error creating client");

        load_batch::<PolymarketPricesResponse>(&client, &web_service_request, 0).await;
    }

    #[tokio::test]
    pub async fn test_load_positions() {
        let web_service_request = WebserviceRequest::new_positions_ws_request(
            "0x3736eb1cc870b9bef096217d98e68f8d6f86243f",
        );

        let client = reqwest::Client::builder()
            .build()
            .expect("Error creating client");

        let (_, _) = load_batch::<PositionsResponse>(&client, &web_service_request, 0).await;
    }
}
