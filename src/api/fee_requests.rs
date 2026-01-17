//! Fee rate request.
use anyhow::{Context, Result};
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

    WebserviceRequest::fetch_one::<FeeRate>(client, &request)
        .await
        .context("Failed to fetch fee rate")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_fee_rate_with_market_token_ids() {
        // First, fetch a market with active orders
        use crate::api::market_requests::MarketsRequest;

        println!("Fetching active markets...");
        match MarketsRequest::builder()
            .closed(Some(false))
            .limit(5)
            .build()
            .execute()
            .await
        {
            Ok(markets) => {
                if let Some(first_market) = markets.first() {
                    println!("✓ Found market: {}", first_market.question.as_deref().unwrap_or("N/A"));

                    if let Some(clob_token_ids) = &first_market.clob_token_ids {
                        println!("  CLOB Token IDs (raw): {}", clob_token_ids);

                        // Token IDs are in JSON array format: ["id1", "id2", ...]
                        match serde_json::from_str::<Vec<String>>(clob_token_ids) {
                            Ok(token_ids) => {
                                if let Some(first_token_id) = token_ids.first() {
                                    println!("  Testing with token ID: {}", first_token_id);

                                    match get_fee_rate(first_token_id).await {
                                        Ok(fee_rate) => {
                                            println!("✓ Successfully fetched fee rate: {:?}", fee_rate);
                                            println!("  Fee Rate BPS: {}", fee_rate.base_fee);
                                            assert!(fee_rate.base_fee >= 0, "Fee rate should be non-negative");
                                        }
                                        Err(e) => {
                                            println!("✗ Failed to fetch fee rate: {}", e);
                                            panic!("Fee rate fetch failed: {}", e);
                                        }
                                    }
                                }
                            }
                            Err(parse_err) => {
                                println!("✗ Failed to parse token IDs: {}", parse_err);
                            }
                        }
                    } else {
                        println!("✗ No CLOB token IDs found in market");
                    }
                }
            }
            Err(e) => {
                println!("✗ Failed to fetch markets: {}", e);
                panic!("Market fetch failed: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_get_fee_rate_with_real_api() {
        // Test fetching a fee rate - using a market's token ID for reliability
        use crate::api::market_requests::MarketsRequest;

        match MarketsRequest::builder()
            .closed(Some(false))
            .limit(1)
            .build()
            .execute()
            .await
        {
            Ok(markets) => {
                if let Some(market) = markets.first() {
                    if let Some(clob_token_ids_str) = &market.clob_token_ids {
                        if let Ok(token_ids) = serde_json::from_str::<Vec<String>>(clob_token_ids_str) {
                            if let Some(token_id) = token_ids.first() {
                                match get_fee_rate(token_id).await {
                                    Ok(fee_rate) => {
                                        println!("✓ Successfully fetched fee rate: {:?}", fee_rate);
                                        println!("  Fee Rate BPS: {}", fee_rate.base_fee);
                                        assert!(fee_rate.base_fee >= 0, "Fee rate should be non-negative");
                                    }
                                    Err(e) => {
                                        panic!("Fee rate fetch failed: {}", e);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                panic!("Failed to fetch markets: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_get_fee_rate_debug_response() {
        // Test to see the raw API response for debugging
        // First fetch a market to get a valid token ID
        use crate::api::market_requests::MarketsRequest;

        println!("Fetching active markets to get valid token ID...");
        let markets = MarketsRequest::builder()
            .closed(Some(false))
            .limit(1)
            .build()
            .execute()
            .await
            .expect("Failed to fetch markets");

        if let Some(first_market) = markets.first() {
            if let Some(clob_token_ids_str) = &first_market.clob_token_ids {
                if let Ok(token_ids) = serde_json::from_str::<Vec<String>>(clob_token_ids_str) {
                    if let Some(token_id) = token_ids.first() {
                        println!("Using token ID: {}", token_id);

                        match get_fee_rate(token_id).await {
                            Ok(fee_rate) => {
                                println!("✓ Successfully fetched fee rate: {:?}", fee_rate);
                                println!("  Fee Rate BPS: {}", fee_rate.base_fee);
                            }
                            Err(e) => {
                                println!("✗ Failed to fetch fee rate: {}", e);
                                panic!("Fee rate fetch failed: {}", e);
                            }
                        }
                    }
                }
            }
        }
    }
}
