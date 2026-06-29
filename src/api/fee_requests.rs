//! Fee rate request.
use crate::api::error::Result;
use serde::Deserialize;
use reqwest::Method;

use crate::api::clob_endpoints::{CLOB_API, FEE_RATE};
use crate::api::http_client::get_http_client;
use crate::api::webservice_request::WebserviceRequest;
use crate::models::ApiResponse;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FeeRate {
    /// Base fee in basis points (bps)
    #[serde(rename = "base_fee")]
    pub base_fee: i32,
}

impl ApiResponse for FeeRate {
    fn nb_results(&self) -> usize {
        1
    }
}

/// Fetches the fee rate for a given token ID.
pub async fn get_fee_rate(token_id: &str) -> Result<FeeRate> {
    let client = get_http_client(None);

    let mut request = WebserviceRequest {
        api: CLOB_API.to_string(),
        url: FEE_RATE.to_string(),
        method: Method::GET,
         with_pagination: false,
        args: Vec::new(),
        body: None,
    };
    request.add_arg("token_id".to_string(), token_id.to_string());

    WebserviceRequest::fetch_one::<FeeRate>(client, &request).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_fee_rate() {
        use crate::api::market_requests::MarketsRequest;

        let markets = MarketsRequest::builder()
            .closed(Some(false))
            .limit(1)
            .build()
            .execute()
            .await
            .expect("Failed to fetch markets");

        let market = markets.data.first().expect("No markets returned");
        let token_id = market
            .clob_token_ids
            .first()
            .expect("Token IDs array is empty");

        let fee_rate = get_fee_rate(token_id).await.expect("Failed to fetch fee rate");
        assert!(fee_rate.base_fee >= 0, "Fee rate should be non-negative");
    }
}
