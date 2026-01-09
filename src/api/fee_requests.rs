//! Fee rate request.
use anyhow::{Context, Result};
use serde::Deserialize;
use reqwest::Client;
use std::time::Duration;
use tokio::time::sleep;

use crate::api::clob_endpoints::{CLOB_API, FEE_RATE};

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FeeRate {
    /// Base fee in basis points (bps)
    /// API returns this as 'base_fee'
    #[serde(rename = "base_fee")]
    pub fee_rate_bps: i32,
}

/// Fetches the fee rate for a given token ID.
/// Creates a fresh client for each request to avoid connection pool issues.
pub async fn get_fee_rate(token_id: &str) -> Result<FeeRate> {
    const MAX_RETRIES: u32 = 3;
    const RETRY_DELAY_MS: u64 = 500;

    for attempt in 1..=MAX_RETRIES {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .context("Failed to create HTTP client")?;

        let url = format!("{}{}?token_id={}", CLOB_API, FEE_RATE, token_id);

        log::debug!(
            "Fetching fee rate for token_id: {} (attempt {}/{})",
            token_id,
            attempt,
            MAX_RETRIES
        );

        match client.get(&url).send().await {
            Ok(response) => {
                match response.status() {
                    reqwest::StatusCode::OK => {
                        let fee_rate: FeeRate = response.json().await
                            .context("Failed to parse fee rate response")?;
                        return Ok(fee_rate);
                    }
                    status @ (reqwest::StatusCode::TOO_MANY_REQUESTS
                    | reqwest::StatusCode::GATEWAY_TIMEOUT
                    | reqwest::StatusCode::BAD_GATEWAY
                    | reqwest::StatusCode::INTERNAL_SERVER_ERROR) => {
                        if attempt < MAX_RETRIES {
                            log::warn!(
                                "Fee rate request got {} status - retrying after {} ms (attempt {}/{})",
                                status,
                                RETRY_DELAY_MS,
                                attempt,
                                MAX_RETRIES
                            );
                            sleep(Duration::from_millis(RETRY_DELAY_MS)).await;
                            continue;
                        } else {
                            return Err(anyhow::anyhow!("Fee rate request failed with status {}: {}", status, response.text().await.unwrap_or_default()));
                        }
                    }
                    other => {
                        return Err(anyhow::anyhow!("Unexpected status code from fee rate API: {}", other));
                    }
                }
            }
            Err(err) => {
                let is_timeout = err.is_timeout();
                let is_connect = err.is_connect();

                if (is_timeout || is_connect) && attempt < MAX_RETRIES {
                    log::warn!(
                        "Fee rate request failed (timeout: {}, connect: {}) - retrying after {} ms (attempt {}/{})",
                        is_timeout,
                        is_connect,
                        RETRY_DELAY_MS,
                        attempt,
                        MAX_RETRIES
                    );
                    sleep(Duration::from_millis(RETRY_DELAY_MS)).await;
                    continue;
                } else {
                    log::error!(
                        "Fee rate request failed after {} attempts for token_id {}: {}",
                        attempt,
                        token_id,
                        err
                    );
                    return Err(err).context(format!(
                        "Failed to send fee rate request for token_id {} after {} attempts",
                        token_id,
                        attempt
                    ));
                }
            }
        }
    }

    unreachable!()
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
                                            println!("  Fee Rate BPS: {}", fee_rate.fee_rate_bps);
                                            assert!(fee_rate.fee_rate_bps >= 0, "Fee rate should be non-negative");
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
                                        println!("  Fee Rate BPS: {}", fee_rate.fee_rate_bps);
                                        assert!(fee_rate.fee_rate_bps >= 0, "Fee rate should be non-negative");
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
                                println!("  Fee Rate BPS: {}", fee_rate.fee_rate_bps);
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
