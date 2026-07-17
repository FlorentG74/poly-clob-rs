//! Shared scaffolding for L2-authenticated CLOB requests.
//!
//! Every authenticated CLOB call repeats the same shape: pick the HTTP client,
//! build L2 headers, send the request with the standard JSON content-type/accept
//! headers, and map transport errors. These helpers capture that scaffold so the
//! individual request builders only describe what varies (method, signed path,
//! URL, body).

use reqwest::header::*;

use crate::api::auth::build_l2_headers;
use crate::api::error::{HttpError, Result};
use crate::api::http_client::get_http_client;
use crate::api::response_handler::handle_api_response;
use crate::models::Account;
use crate::WebserviceRequest;

/// HTTP verb for an authenticated CLOB request.
#[derive(Clone, Copy)]
pub enum Method {
    Get,
    Post,
    Delete,
}

impl Method {
    fn as_str(self) -> &'static str {
        match self {
            Method::Get => "GET",
            Method::Post => "POST",
            Method::Delete => "DELETE",
        }
    }
}

/// Append `signature_type` as a URL query parameter unless it is the `-1` sentinel.
pub fn append_signature_type(url: &mut String, signature_type: i32) {
    if signature_type != -1 {
        WebserviceRequest::add_param_to_url(url, "signature_type", &signature_type.to_string());
    }
}

/// Build L2 headers, send the request, and return the raw [`reqwest::Response`].
///
/// `sign_path` is the request path used for HMAC signing (may differ from the URL
/// path, e.g. when a resource id is part of the signed path). `salt` is the 5th
/// `build_l2_headers` argument (timestamp/salt); pass `""` when not required.
pub async fn send_authed(
    signer: &Account,
    method: Method,
    sign_path: &str,
    url: &str,
    body: &str,
    salt: &str,
) -> Result<reqwest::Response> {
    let client = get_http_client(Some(url));
    let l2_headers = build_l2_headers(signer, method.as_str(), sign_path, body, salt)?;

    let builder = match method {
        Method::Get => client.get(url),
        Method::Post => client.post(url).body(body.to_string()),
        Method::Delete => client.delete(url).body(body.to_string()),
    }
    .header(CONTENT_TYPE, "application/json")
    .header(ACCEPT, "application/json")
    .headers(l2_headers);

    let response = builder
        .send()
        .await
        .map_err(|e| HttpError::from_reqwest(e, url.to_string()))?;

    Ok(response)
}

/// Like [`send_authed`], but returns the response body as text via
/// [`handle_api_response`]. Use for requests that don't need the raw response.
pub async fn send_authed_text(
    signer: &Account,
    method: Method,
    sign_path: &str,
    url: &str,
    body: &str,
    salt: &str,
) -> Result<String> {
    let response = send_authed(signer, method, sign_path, url, body, salt).await?;
    handle_api_response(response, url).await
}
