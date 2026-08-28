//! Client for the Polymarket Bridge API.
//!
//! The Bridge API converts assets from multiple blockchains into pUSD on
//! Polygon for use as trading collateral (and back out again via withdrawals).
//! Unlike the CLOB and Relayer APIs, the bridge endpoints are **unauthenticated**
//! — the caller's Polymarket wallet address is the only identifier needed.
//!
//! # Example
//!
//! ```rust,no_run
//! use poly_clob_rs::api::bridge::BridgeClient;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let bridge = BridgeClient::default();
//!
//!     // Generate per-network deposit addresses for a Polymarket wallet.
//!     let deposit = bridge
//!         .create_deposit_addresses("0x56687bf447db6ffa42ffe2204a05edaa20f55839")
//!         .await?;
//!     if let Some(ref evm) = deposit.address.evm {
//!         println!("Send USDC on any EVM chain to: {}", evm);
//!     }
//!
//!     // Monitor a deposit through to completion.
//!     if let Some(addr) = deposit.address.evm.or(deposit.address.svm) {
//!         for tx in bridge.get_transaction_status(&addr).await? {
//!             println!("{:?}: {:?}", tx.status, tx.tx_hash);
//!         }
//!     }
//!     Ok(())
//! }
//! ```

use serde::de::DeserializeOwned;
use serde::Serialize;
use typed_builder::TypedBuilder;

use super::endpoints::{
    BRIDGE_API, CREATE_DEPOSIT_ADDRESSES, CREATE_WITHDRAWAL_ADDRESSES, GET_QUOTE,
    GET_SUPPORTED_ASSETS, GET_TRANSACTION_STATUS,
};
use super::types::{
    BridgeTransaction, DepositRequest, DepositResponse, QuoteRequest, QuoteResponse,
    SupportedAsset, SupportedAssetsResponse, TransactionStatusResponse, WithdrawalRequest,
    WithdrawalResponse,
};
use crate::api::error::{ApiError, HttpError, Result, SerializationError};
use crate::api::http_client::get_http_client;

/// Client for the Polymarket Bridge API.
///
/// The client holds only a base URL; the bridge endpoints require no
/// authentication. Construct it via [`BridgeClient::default`] for the production
/// endpoint, or with the typed builder to override the base URL (e.g. for tests
/// against a mock server):
///
/// ```rust
/// use poly_clob_rs::api::bridge::BridgeClient;
///
/// let client = BridgeClient::builder()
///     .base_url("http://127.0.0.1:8080".to_string())
///     .build();
/// assert_eq!(client.base_url, "http://127.0.0.1:8080");
/// ```
#[derive(Debug, Clone, TypedBuilder)]
pub struct BridgeClient {
    /// Base URL for the bridge API (defaults to [`BRIDGE_API`]).
    #[builder(default = BRIDGE_API.to_string())]
    pub base_url: String,
}

impl Default for BridgeClient {
    fn default() -> Self {
        Self::builder().build()
    }
}

impl BridgeClient {
    /// Create bridge (deposit) addresses for a Polymarket wallet.
    ///
    /// `POST /deposit`. Returns per-network addresses; send supported assets to
    /// the matching address to have them credited as pUSD on Polygon.
    ///
    /// # Arguments
    ///
    /// * `address` - The Polymarket wallet address to credit deposits to.
    ///
    /// # Errors
    ///
    /// If the request fails, the API returns a non-success status, or the body does not
    /// deserialize into the expected shape.
    pub async fn create_deposit_addresses(&self, address: &str) -> Result<DepositResponse> {
        let req = DepositRequest {
            address: address.to_string(),
        };
        self.post(CREATE_DEPOSIT_ADDRESSES, &req).await
    }

    /// Create withdrawal addresses for a destination chain/token.
    ///
    /// `POST /withdraw`. Returns the bridge addresses to send pUSD to in order to
    /// withdraw to the requested destination.
    ///
    /// # Errors
    ///
    /// If the request fails, the API returns a non-success status, or the body does not
    /// deserialize into the expected shape.
    pub async fn create_withdrawal_addresses(
        &self,
        request: &WithdrawalRequest,
    ) -> Result<WithdrawalResponse> {
        self.post(CREATE_WITHDRAWAL_ADDRESSES, request).await
    }

    /// Get the list of supported assets and their minimum deposit/withdrawal amounts.
    ///
    /// `GET /supported-assets`.
    ///
    /// # Errors
    ///
    /// If the request fails, the API returns a non-success status, or the body does not
    /// deserialize into the expected shape.
    pub async fn get_supported_assets(&self) -> Result<Vec<SupportedAsset>> {
        let url = format!("{}{}", self.base_url, GET_SUPPORTED_ASSETS);
        let response: SupportedAssetsResponse = self.get(&url).await?;
        Ok(response.supported_assets)
    }

    /// Get a swap/bridge quote for a prospective transfer.
    ///
    /// `POST /quote`.
    ///
    /// # Errors
    ///
    /// If the request fails, the API returns a non-success status, or the body does not
    /// deserialize into the expected shape.
    pub async fn get_quote(&self, request: &QuoteRequest) -> Result<QuoteResponse> {
        self.post(GET_QUOTE, request).await
    }

    /// Get the status of transactions for a bridge address.
    ///
    /// `GET /status/{address}`. The `address` is one of the per-network addresses
    /// returned by [`create_deposit_addresses`](Self::create_deposit_addresses) or
    /// [`create_withdrawal_addresses`](Self::create_withdrawal_addresses).
    ///
    /// # Errors
    ///
    /// If the request fails, the API returns a non-success status, or the body does not
    /// deserialize into the expected shape.
    pub async fn get_transaction_status(&self, address: &str) -> Result<Vec<BridgeTransaction>> {
        let url = format!("{}{}/{}", self.base_url, GET_TRANSACTION_STATUS, address);
        let response: TransactionStatusResponse = self.get(&url).await?;
        Ok(response.transactions)
    }

    /// Issue a GET request and deserialize the JSON response.
    async fn get<T: DeserializeOwned>(&self, url: &str) -> Result<T> {
        let client = get_http_client(Some(url));
        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| HttpError::from_reqwest(e, url))?;
        Self::parse_json(url, response).await
    }

    /// Serialize `body` to JSON, POST it, and deserialize the JSON response.
    async fn post<B: Serialize, T: DeserializeOwned>(&self, path: &str, body: &B) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        let body = serde_json::to_string(body).map_err(|e| SerializationError::JsonSerialize {
            message: e.to_string(),
        })?;

        let client = get_http_client(Some(&url));
        let response = client
            .post(&url)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await
            .map_err(|e| HttpError::from_reqwest(e, &url))?;
        Self::parse_json(&url, response).await
    }

    /// Read the response body, enforce a success status, and deserialize to `T`.
    ///
    /// Bridge endpoints return `200` (GET) or `201` (POST create) on success, so
    /// success is determined via [`reqwest::StatusCode::is_success`]. On failure
    /// the raw body (typically `{"error": "..."}`) is surfaced in the error.
    async fn parse_json<T: DeserializeOwned>(
        url: &str,
        response: reqwest::Response,
    ) -> Result<T> {
        let status = response.status();
        let body = response.text().await.map_err(|e| HttpError::ReadBody {
            url: url.to_string(),
            message: e.to_string(),
        })?;

        if !status.is_success() {
            log::error!("Bridge request to {} failed ({}): {}", url, status, body);
            return Err(ApiError::UnexpectedStatus {
                status: status.as_u16(),
                url: url.to_string(),
                message: body.clone(),
                response_body: body,
            }
            .into());
        }

        log::trace!("Bridge response from {}: {}", url, body);
        serde_json::from_str(&body).map_err(|e| {
            SerializationError::JsonDeserialize {
                message: e.to_string(),
                raw_response: body,
            }
            .into()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_uses_production_base_url() {
        let client = BridgeClient::default();
        assert_eq!(client.base_url, BRIDGE_API);
    }

    #[test]
    fn builder_overrides_base_url() {
        let client = BridgeClient::builder()
            .base_url("http://localhost:9999".to_string())
            .build();
        assert_eq!(client.base_url, "http://localhost:9999");
    }
}
