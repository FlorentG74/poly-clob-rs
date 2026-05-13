//! Account management request builders.
//!
//! This module provides functions for managing account allowances and API keys on the Polymarket CLOB.

use crate::api::error::Result;

use crate::api::auth::build_l2_headers;
use crate::api::http_client::get_http_client;
use crate::api::response_handler::handle_api_response;
use crate::models::{Account, AssetType};
use crate::WebserviceRequest;
use reqwest::header::*;

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
    let method = "GET";
    let request_path = GET_BALANCE_ALLOWANCE;
    let body = "";

    let mut callable_url = format!("{}{}", CLOB_API, request_path);

    let client = get_http_client(Some(request_path));

    WebserviceRequest::add_param_to_url(&mut callable_url, "asset_type", asset_type.into());
    WebserviceRequest::add_param_to_url(&mut callable_url, "token_id", token_id);

    if signature_type != -1 {
        let signature_str = format!("{}", signature_type);
        WebserviceRequest::add_param_to_url(&mut callable_url, "signature_type", signature_str.as_str());
    }

    let l2_headers = build_l2_headers(signer, method, request_path, body, "")?;

    let response = client
        .get(&callable_url)
        .header(CONTENT_TYPE, "application/json")
        .header(ACCEPT, "application/json")
        .headers(l2_headers)
        .send()
        .await
        .map_err(|e| crate::api::error::HttpError::from_reqwest(e, callable_url.clone()))?;

    handle_api_response(response, &callable_url).await
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

    let method = "GET";
    let request_path = GET_API_KEYS;
    let body = "";

    let mut callable_url = format!("{}{}", CLOB_API, request_path);

    let client = get_http_client(Some(request_path));

    if signature_type != -1 {
        let signature_str = format!("{}", signature_type);
        WebserviceRequest::add_param_to_url(&mut callable_url, "signature_type", signature_str.as_str());
    }

    let l2_headers = build_l2_headers(signer, method, request_path, body, "")?;

    let response = client
        .get(&callable_url)
        .header(CONTENT_TYPE, "application/json")
        .header(ACCEPT, "application/json")
        .headers(l2_headers)
        .send()
        .await
        .map_err(|e| crate::api::error::HttpError::from_reqwest(e, callable_url.clone()))?;

    handle_api_response(response, &callable_url).await
}