//! Fee rate request.
use anyhow::{Context, Result};
use serde::Deserialize;

use crate::api::clob_endpoints::{CLOB_API, FEE_RATE};
use crate::api::http_client::get_http_client;
use crate::api::response_handler::handle_api_response;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FeeRate {
    pub fee_rate_bps: i32,
}

/// Fetches the fee rate for a given token ID.
pub async fn get_fee_rate(token_id: &str) -> Result<FeeRate> {
    let client = get_http_client(Some(CLOB_API));
    let url = format!("{}{}?token_id={}", CLOB_API, FEE_RATE, token_id);

    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to send fee rate request")?;

    let response_body = handle_api_response(response, &url).await?;

    serde_json::from_str(&response_body).context("Failed to parse fee rate response")
}
