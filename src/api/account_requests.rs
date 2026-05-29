//! Account management request builders.
//!
//! This module provides functions for managing account allowances and API keys on the Polymarket CLOB.

use crate::api::error::Result;

use crate::api::authed_request::{append_signature_type, send_authed_text, Method};
use crate::models::{Account, AssetType};
use crate::WebserviceRequest;

use super::clob_endpoints::{CLOB_API, GET_API_KEYS, GET_BALANCE_ALLOWANCE};

/// Get balance and allowance for an account.
///
/// # Arguments
///
/// * `signer` - The account to query
/// * `asset_type` - The type of asset (e.g., COLLATERAL)
/// * `token_id` - The token ID to check
/// * `signature_type` - Optional signature type (-1 to omit)
///
/// # Returns
///
/// Returns `Ok(String)` with the API response on success, or an error on failure.
pub async fn get_balance_allowance(
    signer: &Account,
    asset_type: AssetType,
    token_id: &str,
    signature_type: i32,
) -> Result<String> {
    let request_path = GET_BALANCE_ALLOWANCE;
    let mut callable_url = format!("{}{}", CLOB_API, request_path);

    WebserviceRequest::add_param_to_url(&mut callable_url, "asset_type", asset_type.into());
    WebserviceRequest::add_param_to_url(&mut callable_url, "token_id", token_id);
    append_signature_type(&mut callable_url, signature_type);

    send_authed_text(signer, Method::Get, request_path, &callable_url, "", "").await
}

/// Get API keys for an account.
///
/// # Arguments
///
/// * `signer` - The account to query
/// * `signature_type` - Optional signature type (-1 to omit)
///
/// # Returns
///
/// Returns `Ok(String)` with the API response on success, or an error on failure.
pub async fn get_api_key(signer: &Account, signature_type: i32) -> Result<String> {
    let request_path = GET_API_KEYS;
    let mut callable_url = format!("{}{}", CLOB_API, request_path);

    append_signature_type(&mut callable_url, signature_type);

    send_authed_text(signer, Method::Get, request_path, &callable_url, "", "").await
}