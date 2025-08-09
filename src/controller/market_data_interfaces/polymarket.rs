use crate::controller::load_batch;
use crate::controller::WebserviceRequest;
use crate::controller::WebserviceResponse;
use crate::controller::CLOB_API;
use crate::controller::GET_PRICES;
use crate::model::Price;

use super::MarketDataConnector;
use chrono::Utc;
use futures::SinkExt;
use futures::TryStreamExt;
use reqwest::Client;
use reqwest::Method;
use reqwest_websocket::Message;
use reqwest_websocket::RequestBuilderExt;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use string_builder::Builder;

#[derive(Serialize)]
struct PolymarketPriceRequest {
    token_id: String,
    side: String,
}

pub type PolymarketPricesResponse = HashMap<String, PolymarketPrice>;

impl WebserviceRequest {
    pub fn new_polymarket_price_request(token_ids: &[String]) -> Self {
        let args = HashMap::<String, String>::new();

        let body = PolymarketMarketDataInterface::build_prices_query(token_ids);

        return WebserviceRequest {
            api: CLOB_API.to_string(),
            url: GET_PRICES.to_string(),
            method: Method::POST,
            args: args,
            body: Some(body),
        };
    }

    pub fn get_body(&self) -> String {
        return self.body.clone().unwrap();
    }
}

impl WebserviceResponse for PolymarketPricesResponse {
    async fn store(&self) {
    }

    fn nb_results(&self) -> usize {
        self.len()
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct PolymarketPrice {
    pub buy: Option<String>,
    pub sell: Option<String>,
}

pub struct PolymarketMarketDataInterface {}

impl PolymarketMarketDataInterface {
    fn build_prices_query(token_ids: &[String]) -> String {
        let mut instruments = Vec::<PolymarketPriceRequest>::new();

        for token_id in token_ids {
            instruments.push(PolymarketPriceRequest {
                token_id: token_id.to_string(),
                side: "sell".to_string(),
            });
            instruments.push(PolymarketPriceRequest {
                token_id: token_id.to_string(),
                side: "buy".to_string(),
            });
        }

        serde_json::to_string(&instruments).unwrap()
    }

    fn build_prices_streaming_query(token_ids: &Vec<String>) -> String {
        let mut builder = Builder::default();

        builder.append("{\"assets_ids\": [");

        let mut it = token_ids.iter().peekable();
        while let Some(token_id) = it.next() {
            builder.append("\"");
            builder.append(token_id.clone());
            builder.append("\"");
            if it.peek().is_some() {
                builder.append(",");
            }
        }

        builder.append("],\"type\": \"market\"}");

        builder.string().expect("Error in String conversion")
    }
}

impl MarketDataConnector for PolymarketMarketDataInterface {
    fn new() -> Self {
        Self {}
    }

    async fn retrieve_and_cache_prices(&mut self, token_ids: &[String]) {
        let web_service_request = WebserviceRequest::new_polymarket_price_request(&token_ids);

        let client = reqwest::Client::builder()
            .build()
            .expect("Error creating client");

        load_batch::<PolymarketPricesResponse>(&client, &web_service_request, 0).await;
    }

    async fn subscribe_to_prices_stream(&self, token_ids: &Vec<String>) {
        //https://docs.polymarket.com/?python#websocket-api

        // Creates a GET request, upgrades and sends it.
        let response = Client::default()
            .get("wss://ws-subscriptions-clob.polymarket.com/ws/market")
            .upgrade() // Prepares the WebSocket upgrade.
            .send()
            .await
            .expect("");

        // Turns the response into a WebSocket stream.
        let mut websocket = response.into_websocket().await.expect("");

        let sub_message = PolymarketMarketDataInterface::build_prices_streaming_query(token_ids);

        println!("Sub message: {sub_message}");

        // The WebSocket implements `Sink<Message>`.
        websocket.send(Message::Text(sub_message)).await.expect("");

        // The WebSocket is also a `TryStream` over `Message`s.
        while let Some(message) = websocket.try_next().await.expect("") {
            if let Message::Text(text) = message {
                println!("Price received: {}", text);
            }
        }
    }
}

#[cfg(test)]
mod polymarket_tests {
    use crate::controller::PolymarketMarketDataInterface;

    #[test]
    fn format_polymarket_price_query() {
        let expected_result = "[{\"token_id\":\"101669189743438912873361127612589311253202068943959811456820079057046819967115\",\"side\":\"sell\"},{\"token_id\":\"101669189743438912873361127612589311253202068943959811456820079057046819967115\",\"side\":\"buy\"},{\"token_id\":\"113332423559050930347591987511234765387649957428761857688151517507261414072694\",\"side\":\"sell\"},{\"token_id\":\"113332423559050930347591987511234765387649957428761857688151517507261414072694\",\"side\":\"buy\"}]";

        let mut instruments = Vec::<String>::new();
        instruments.push(
            "101669189743438912873361127612589311253202068943959811456820079057046819967115"
                .to_string(),
        );
        instruments.push(
            "113332423559050930347591987511234765387649957428761857688151517507261414072694"
                .to_string(),
        );

        let query = PolymarketMarketDataInterface::build_prices_query(&instruments.as_slice());

        assert_eq!(expected_result, query);
    }

    #[test]
    fn format_polymarket_streaming_query() {
        let expected_result = "{\"assets_ids\": [\"101669189743438912873361127612589311253202068943959811456820079057046819967115\",\"113332423559050930347591987511234765387649957428761857688151517507261414072694\"],\"type\": \"market\"}";

        let mut instruments = Vec::<String>::new();
        instruments.push(
            "101669189743438912873361127612589311253202068943959811456820079057046819967115"
                .to_string(),
        );
        instruments.push(
            "113332423559050930347591987511234765387649957428761857688151517507261414072694"
                .to_string(),
        );

        let query = PolymarketMarketDataInterface::build_prices_streaming_query(&instruments);

        assert_eq!(expected_result, query);
    }
}
