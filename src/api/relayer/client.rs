//! Relayer client for submitting gasless transactions to Polymarket.
//!
//! This module provides the `RelayerClient` struct for interacting with the
//! Polymarket Relayer V2 API. The relayer enables gasless transactions by
//! submitting transactions on behalf of users through Safe wallets.

use super::auth::{
    build_builder_headers, derive_proxy_address, derive_safe_address, sign_proxy_transaction,
    sign_safe_transaction, BuilderCredentials,
};
use super::transactions::{contracts, encode_proxy_call_data};
use super::endpoints::{
    GET_DEPLOYED, GET_NONCE, GET_RELAY_PAYLOAD, GET_TRANSACTION, GET_TRANSACTIONS, RELAYER_API,
    SUBMIT_TRANSACTION,
};
use super::types::{
    DeployedResponse, NonceResponse, RelayPayloadResponse, RelayerTransaction,
    RelayerTransactionResponse, RelayerTransactionState, RelayerTxType, SafeTransaction,
    SignatureType, Transaction,
};
use crate::api::error::{
    ApiError, AuthError, HttpError, RelayerError, Result, SerializationError, ValidationError,
};
use crate::api::http_client::get_http_client;
use alloy::primitives::Address;
use alloy::signers::local::PrivateKeySigner;
use hex;
use std::time::Duration;
use typed_builder::TypedBuilder;

/// Default chain ID for Polygon mainnet.
use crate::constants::POLYGON_CHAIN_ID;

/// Default polling interval for transaction state checks.
pub const DEFAULT_POLL_INTERVAL_MS: u64 = 2000;

/// Default maximum polling attempts.
pub const DEFAULT_MAX_POLL_ATTEMPTS: u32 = 30;

/// Gas limit for a proxy (GSN) submission wrapping `n` batched sub-calls.
///
/// The gas limit must cover EVERY batched sub-call: an under-budgeted relayed call
/// fails silently (`RelayHub` reports `RelayedCallFailed` while the outer tx still mines).
///
/// The per-call budget is measured, not assumed. `eth_estimateGas` on single
/// `redeemPositions` calls against `REDEEM_ROUTER` ranges from ~265k to ~472k depending
/// on how many storage slots the payout touches, so a batch of 2 can need ~740k. The
/// previous 220k/call figure came from the Polymarket UI's legacy-CTF path and left a
/// 2-redeem batch (640k) short of its ~737k requirement. We budget 500k per sub-call
/// plus a 200k base. An empty batch is treated as one call so it still gets a budget.
#[must_use]
pub fn proxy_gas_limit(num_transactions: usize) -> u64 {
    let n = (num_transactions.max(1)) as u64;
    (500_000 * n + 200_000).max(500_000)
}

/// Client for interacting with the Polymarket Relayer API.
///
/// The relayer client provides methods for:
/// - Querying nonces and deployment status
/// - Submitting Safe transactions with proper EIP-712 signing
/// - Polling for transaction confirmations
///
/// # Example
///
/// ```rust,no_run
/// use poly_clob_rs::api::relayer::{
///     RelayerClient, BuilderCredentials, RelayerTxType,
///     create_redeem_tx, RedeemParams,
/// };
/// use alloy::signers::local::PrivateKeySigner;
/// use std::str::FromStr;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let creds = BuilderCredentials::from_env()?;
///     let signer = PrivateKeySigner::from_str("0x...")?;
///     let client = RelayerClient::builder()
///         .credentials(creds)
///         .signer_address("0x...".to_string())
///         .signer(signer)
///         .build();
///
///     // Get current nonce
///     let nonce = client.get_nonce().await?;
///     println!("Current nonce: {}", nonce);
///
///     // Create and submit a redeem transaction
///     let redeem_tx = create_redeem_tx(&RedeemParams {
///         condition_id: "0x...".to_string(),
///         outcome_index: Some(0),
///     })?;
///
///     let response = client.submit(vec![redeem_tx]).await?;
///     println!("Transaction submitted: {}", response.transaction_id);
///
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone, TypedBuilder)]
pub struct RelayerClient {
    /// Builder API credentials for authentication.
    pub credentials: BuilderCredentials,

    /// Signer address (EOA / Safe owner).
    pub signer_address: String,

    /// Private key signer for EIP-712 Safe transaction signing.
    pub signer: PrivateKeySigner,

    /// Base URL for the relayer API.
    #[builder(default = RELAYER_API.to_string())]
    pub base_url: String,

    /// Chain ID (137 for Polygon mainnet).
    #[builder(default = POLYGON_CHAIN_ID)]
    pub chain_id: u64,

    /// Transaction type (Safe or Proxy).
    #[builder(default = RelayerTxType::Safe)]
    pub tx_type: RelayerTxType,

    /// Signature type for the wallet.
    #[builder(default = SignatureType::PolyProxy)]
    pub signature_type: SignatureType,

    /// Safe/Proxy wallet address (if different from signer).
    #[builder(default)]
    pub wallet_address: Option<String>,

    /// Polling interval in milliseconds for transaction state checks.
    #[builder(default = DEFAULT_POLL_INTERVAL_MS)]
    pub poll_interval_ms: u64,

    /// Maximum polling attempts before timeout.
    #[builder(default = DEFAULT_MAX_POLL_ATTEMPTS)]
    pub max_poll_attempts: u32,
}

impl RelayerClient {
    /// Get the current nonce for the signer address.
    ///
    /// The nonce is used to order transactions and prevent replay attacks.
    ///
    /// # Errors
    ///
    /// If the request fails, the API returns a non-success status, or the body does not
    /// deserialize into the expected shape.
    pub async fn get_nonce(&self) -> Result<u64> {
        self.get_nonce_for_address(&self.signer_address).await
    }

    /// Get the current nonce for a specific address.
    ///
    /// # Errors
    ///
    /// If the request fails, the API returns a non-success status, or the body does not
    /// deserialize into the expected shape.
    pub async fn get_nonce_for_address(&self, address: &str) -> Result<u64> {
        let tx_type_str = match self.tx_type {
            RelayerTxType::Safe => "SAFE",
            RelayerTxType::Proxy => "PROXY",
        };

        let url = format!(
            "{}{}?address={}&type={}",
            self.base_url, GET_NONCE, address, tx_type_str
        );

        let headers = build_builder_headers(&self.credentials, "GET", GET_NONCE, "")?;

        let client = get_http_client(Some(&url));
        let response = client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| HttpError::from_reqwest(e, &url))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| HttpError::ReadBody {
                url: url.clone(),
                message: e.to_string(),
            })?;

        if !status.is_success() {
            return Err(ApiError::UnexpectedStatus {
                status: status.as_u16(),
                url,
                message: "get_nonce failed".to_string(),
                response_body: body,
            }
            .into());
        }

        let nonce_response: NonceResponse = serde_json::from_str(&body).map_err(|e| {
            SerializationError::JsonDeserialize {
                message: e.to_string(),
                raw_response: body.clone(),
            }
        })?;

        Ok(nonce_response.nonce)
    }

    /// Check if the Safe/Proxy wallet is deployed.
    ///
    /// # Errors
    ///
    /// If the request fails, the API returns a non-success status, or the body does not
    /// deserialize into the expected shape.
    pub async fn get_deployed(&self, address: &str) -> Result<bool> {
        let url = format!("{}{}?address={}", self.base_url, GET_DEPLOYED, address);

        let headers = build_builder_headers(&self.credentials, "GET", GET_DEPLOYED, "")?;

        let client = get_http_client(Some(&url));
        let response = client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| HttpError::from_reqwest(e, &url))?;

        let status = response.status();
        if !status.is_success() {
            let body = response
                .text()
                .await
                .map_err(|e| HttpError::ReadBody {
                    url: url.clone(),
                    message: e.to_string(),
                })?;
            return Err(ApiError::UnexpectedStatus {
                status: status.as_u16(),
                url,
                message: "get_deployed failed".to_string(),
                response_body: body,
            }
            .into());
        }

        let deployed_response: DeployedResponse = response.json().await.map_err(|e| {
            SerializationError::JsonDeserialize {
                message: e.to_string(),
                raw_response: String::new(),
            }
        })?;

        Ok(deployed_response.deployed)
    }

    /// Get a specific transaction by ID.
    ///
    /// # Errors
    ///
    /// If the request fails, the API returns a non-success status, or the body does not
    /// deserialize into the expected shape.
    pub async fn get_transaction(&self, transaction_id: &str) -> Result<RelayerTransaction> {
        let url = format!(
            "{}{}?id={}",
            self.base_url, GET_TRANSACTION, transaction_id
        );

        let headers = build_builder_headers(&self.credentials, "GET", GET_TRANSACTION, "")?;

        let client = get_http_client(Some(&url));
        let response = client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| HttpError::from_reqwest(e, &url))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| HttpError::ReadBody {
                url: url.clone(),
                message: e.to_string(),
            })?;

        if !status.is_success() {
            return Err(ApiError::UnexpectedStatus {
                status: status.as_u16(),
                url,
                message: format!("get_transaction failed: {}", body),
                response_body: body,
            }
            .into());
        }

        log::debug!("get_transaction response: {}", body);

        // The relayer returns an array of transactions; take the first match
        let txns: Vec<RelayerTransaction> = serde_json::from_str(&body).map_err(|e| {
            SerializationError::JsonDeserialize {
                message: format!("{} (raw: {})", e, &body[..body.len().min(200)]),
                raw_response: body.clone(),
            }
        })?;

        txns.into_iter().next().ok_or_else(|| {
            RelayerError::TransactionFailed {
                state: "NOT_FOUND".to_string(),
                message: Some(format!("transaction {} not found", transaction_id)),
            }
            .into()
        })
    }

    /// Get all transactions for the authenticated builder.
    ///
    /// # Errors
    ///
    /// If the request fails, the API returns a non-success status, or the body does not
    /// deserialize into the expected shape.
    pub async fn get_transactions(&self) -> Result<Vec<RelayerTransaction>> {
        let url = format!("{}{}", self.base_url, GET_TRANSACTIONS);

        let headers = build_builder_headers(&self.credentials, "GET", GET_TRANSACTIONS, "")?;

        let client = get_http_client(Some(&url));
        let response = client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| HttpError::from_reqwest(e, &url))?;

        let status = response.status();
        if !status.is_success() {
            let body = response
                .text()
                .await
                .map_err(|e| HttpError::ReadBody {
                    url: url.clone(),
                    message: e.to_string(),
                })?;
            return Err(ApiError::UnexpectedStatus {
                status: status.as_u16(),
                url,
                message: "get_transactions failed".to_string(),
                response_body: body,
            }
            .into());
        }

        response.json().await.map_err(|e| {
            SerializationError::JsonDeserialize {
                message: e.to_string(),
                raw_response: String::new(),
            }
            .into()
        })
    }

    /// Get relay payload for proxy transactions.
    ///
    /// Returns the relay address and nonce needed for proxy transaction signing.
    ///
    /// # Errors
    ///
    /// If the request fails, the API returns a non-success status, or the body does not
    /// deserialize into the expected shape.
    pub async fn get_relay_payload(&self, address: &str) -> Result<RelayPayloadResponse> {
        let tx_type_str = match self.tx_type {
            RelayerTxType::Safe => "SAFE",
            RelayerTxType::Proxy => "PROXY",
        };

        let url = format!(
            "{}{}?address={}&type={}",
            self.base_url, GET_RELAY_PAYLOAD, address, tx_type_str
        );

        let headers = build_builder_headers(&self.credentials, "GET", GET_RELAY_PAYLOAD, "")?;

        let client = get_http_client(Some(&url));
        let response = client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| HttpError::from_reqwest(e, &url))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| HttpError::ReadBody {
                url: url.clone(),
                message: e.to_string(),
            })?;

        if !status.is_success() {
            return Err(ApiError::UnexpectedStatus {
                status: status.as_u16(),
                url,
                message: "get_relay_payload failed".to_string(),
                response_body: body,
            }
            .into());
        }

        serde_json::from_str(&body).map_err(|e| {
            SerializationError::JsonDeserialize {
                message: e.to_string(),
                raw_response: body.clone(),
            }
            .into()
        })
    }

    /// Submit transactions to the relayer.
    ///
    /// Routes to Safe or Proxy submission based on `tx_type`.
    ///
    /// # Errors
    ///
    /// If the builder headers cannot be built, or the relayer rejects the submission.
    pub async fn submit(&self, transactions: Vec<Transaction>) -> Result<RelayerTransactionResponse> {
        match self.tx_type {
            RelayerTxType::Safe => {
                let safe_txs: Vec<SafeTransaction> =
                    transactions.into_iter().map(|t| t.into()).collect();
                self.submit_safe(safe_txs).await
            }
            RelayerTxType::Proxy => {
                self.submit_proxy(transactions).await
            }
        }
    }

    /// Submit a Safe transaction to the relayer.
    ///
    /// Implements the official Polymarket relayer format matching the TypeScript
    /// `builder-relayer-client`:
    /// 1. Get nonce from relayer
    /// 2. Determine Safe address (from `wallet_address` config, or derive via CREATE2)
    /// 3. Sign with EIP-712 `SafeTx` schema
    /// 4. Submit with proper request format including signatureParams
    ///
    /// # Errors
    ///
    /// If signing the Safe transaction fails, or the relayer rejects the submission.
    pub async fn submit_safe(
        &self,
        transactions: Vec<SafeTransaction>,
    ) -> Result<RelayerTransactionResponse> {
        if transactions.is_empty() {
            return Err(ValidationError::InvalidParameter {
                parameter: "transactions".to_string(),
                reason: "cannot submit empty transaction list".to_string(),
            }
            .into());
        }

        // Use the signer's derived address as the authoritative EOA
        // (matches TypeScript's `signer.getAddress()`)
        let eoa = self.signer.address();
        let eoa_str = format!("{}", eoa);

        // Determine the Safe/proxy wallet address:
        // 1. If wallet_address is configured (POLY_ADDRESS), use it directly
        // 2. Otherwise, derive via CREATE2 (matches TypeScript deriveSafe)
        let safe_address: Address = if let Some(ref wallet_addr) = self.wallet_address {
            let addr: Address = wallet_addr.parse().map_err(|_| {
                AuthError::InvalidAddress {
                    address: wallet_addr.clone(),
                }
            })?;
            let derived = derive_safe_address(&eoa);
            if addr != derived {
                log::info!(
                    "Using configured wallet address {} (derived would be {})",
                    addr, derived
                );
            }
            addr
        } else {
            let derived = derive_safe_address(&eoa);
            log::info!("No wallet_address configured, using derived Safe: {}", derived);
            derived
        };

        log::info!("EOA: {}, Safe/Proxy wallet: {}", eoa, safe_address);

        // Get nonce (query with the EOA address, matching TypeScript's getNonce(signerAddress))
        let nonce = self.get_nonce_for_address(&eoa_str).await?;

        // Use the first transaction (single tx support)
        if transactions.len() > 1 {
            log::warn!(
                "Multiple transactions ({}) not yet supported for Safe submission; using first only",
                transactions.len()
            );
        }
        let tx = &transactions[0];

        // Sign the transaction
        let signature = sign_safe_transaction(
            &self.signer,
            &safe_address,
            self.chain_id,
            &tx.to,
            &tx.data,
            tx.operation as u8,
            nonce,
        )
        .await?;

        // Build request body matching the official TypeScript format
        let req = serde_json::json!({
            "type": "SAFE",
            "from": eoa_str,
            "to": format!("{}", tx.to),
            "proxyWallet": format!("{}", safe_address),
            "data": format!("0x{}", hex::encode(tx.data.as_ref())),
            "nonce": nonce.to_string(),
            "signature": signature,
            "signatureParams": {
                "gasPrice": "0",
                "operation": "0",
                "safeTxnGas": "0",
                "baseGas": "0",
                "gasToken": format!("{}", Address::ZERO),
                "refundReceiver": format!("{}", Address::ZERO)
            },
            "metadata": ""
        });

        let body = serde_json::to_string(&req).map_err(|e| SerializationError::JsonSerialize {
            message: e.to_string(),
        })?;
        log::info!(
            "Submitting Safe request:\n{}",
            serde_json::to_string_pretty(&req).unwrap_or_default()
        );

        let url = format!("{}{}", self.base_url, SUBMIT_TRANSACTION);
        let headers =
            build_builder_headers(&self.credentials, "POST", SUBMIT_TRANSACTION, &body)?;

        let client = get_http_client(Some(&url));
        let response = client
            .post(&url)
            .headers(headers)
            .header("Content-Type", "application/json")
            .body(body.clone())
            .send()
            .await
            .map_err(|e| HttpError::from_reqwest(e, &url))?;

        let status = response.status();
        let body_text = response
            .text()
            .await
            .map_err(|e| HttpError::ReadBody {
                url: url.clone(),
                message: e.to_string(),
            })?;

        if !status.is_success() {
            log::error!("Submit Safe failed with status {}: {}", status, body_text);
            return Err(ApiError::UnexpectedStatus {
                status: status.as_u16(),
                url,
                message: format!("submit_safe failed: {}", body_text),
                response_body: body_text,
            }
            .into());
        }

        log::info!("Submit Safe response: {}", body_text);
        Self::parse_submit_response(&body_text)
    }

    /// Submit a Proxy transaction to the relayer using GSN v1 signing.
    ///
    /// The PROXY flow:
    /// 1. Fetch relay address + nonce from `/relay-payload?address=&type=PROXY`
    /// 2. Wrap inner transactions with `proxy(...)` call targeting `ProxyWalletFactory`
    /// 3. Sign using GSN "rlx:" scheme (EIP-191 personal sign over struct hash)
    /// 4. Submit with GSN-style signatureParams: gasLimit, relayerFee, relayHub, relay
    ///
    /// # Errors
    ///
    /// If signing the Proxy transaction fails, or the relayer rejects the submission.
    pub async fn submit_proxy(
        &self,
        transactions: Vec<Transaction>,
    ) -> Result<RelayerTransactionResponse> {
        if transactions.is_empty() {
            return Err(ValidationError::InvalidParameter {
                parameter: "transactions".to_string(),
                reason: "cannot submit empty transaction list".to_string(),
            }
            .into());
        }

        let eoa = self.signer.address();
        let eoa_str = format!("{}", eoa);

        // Determine the proxy wallet address
        let proxy_address: Address = if let Some(ref wallet_addr) = self.wallet_address {
            let addr: Address = wallet_addr.parse().map_err(|_| {
                AuthError::InvalidAddress {
                    address: wallet_addr.clone(),
                }
            })?;
            let derived = derive_proxy_address(&eoa);
            if addr != derived {
                log::info!(
                    "Using configured wallet address {} (derived proxy would be {})",
                    addr, derived
                );
            }
            addr
        } else {
            let derived = derive_proxy_address(&eoa);
            log::info!("No wallet_address configured, using derived proxy: {}", derived);
            derived
        };

        log::info!("EOA: {}, Proxy wallet: {}", eoa, proxy_address);

        // Fetch relay address and nonce from relay-payload endpoint
        let relay_payload = self.get_relay_payload(&eoa_str).await?;
        let nonce = relay_payload.nonce;
        let relay_address_str = relay_payload.address.clone();
        let relay_address: Address = relay_address_str.parse().map_err(|_| {
            AuthError::InvalidAddress {
                address: relay_payload.address.clone(),
            }
        })?;

        log::info!("Relay address: {}, nonce: {}", relay_address, nonce);

        // Wrap all inner transactions in a proxy(...) call targeting the ProxyWalletFactory
        let proxy_factory: Address = contracts::PROXY_FACTORY.parse().map_err(|_| {
            AuthError::InvalidAddress {
                address: contracts::PROXY_FACTORY.to_string(),
            }
        })?;
        let relay_hub: Address = contracts::RELAY_HUB.parse().map_err(|_| {
            AuthError::InvalidAddress {
                address: contracts::RELAY_HUB.to_string(),
            }
        })?;

        let proxy_call_data = encode_proxy_call_data(&transactions);
        let gas_limit: u64 = proxy_gas_limit(transactions.len());

        // Sign using GSN "rlx:" scheme
        let signature = sign_proxy_transaction(
            &self.signer,
            &eoa,
            &proxy_factory,
            &proxy_call_data,
            gas_limit,
            nonce,
            &relay_hub,
            &relay_address,
        )
        .await?;

        let req = serde_json::json!({
            "type": "PROXY",
            "from": eoa_str,
            "to": format!("{}", proxy_factory),
            "proxyWallet": format!("{}", proxy_address),
            "data": format!("0x{}", hex::encode(proxy_call_data.as_ref())),
            "nonce": nonce.to_string(),
            "signature": signature,
            "signatureParams": {
                "gasPrice": "0",
                "gasLimit": gas_limit.to_string(),
                "relayerFee": "0",
                "relayHub": contracts::RELAY_HUB,
                "relay": relay_address_str
            },
            "metadata": ""
        });

        let body = serde_json::to_string(&req).map_err(|e| SerializationError::JsonSerialize {
            message: e.to_string(),
        })?;
        log::info!(
            "Submitting Proxy request:\n{}",
            serde_json::to_string_pretty(&req).unwrap_or_default()
        );

        let url = format!("{}{}", self.base_url, SUBMIT_TRANSACTION);
        let headers =
            build_builder_headers(&self.credentials, "POST", SUBMIT_TRANSACTION, &body)?;

        let client = get_http_client(Some(&url));
        let response = client
            .post(&url)
            .headers(headers)
            .header("Content-Type", "application/json")
            .body(body.clone())
            .send()
            .await
            .map_err(|e| HttpError::from_reqwest(e, &url))?;

        let status = response.status();
        let body_text = response
            .text()
            .await
            .map_err(|e| HttpError::ReadBody {
                url: url.clone(),
                message: e.to_string(),
            })?;

        if !status.is_success() {
            log::error!("Submit Proxy failed with status {}: {}", status, body_text);
            return Err(ApiError::UnexpectedStatus {
                status: status.as_u16(),
                url,
                message: format!("submit_proxy failed: {}", body_text),
                response_body: body_text,
            }
            .into());
        }

        log::info!("Submit Proxy response: {}", body_text);
        Self::parse_submit_response(&body_text)
    }

    /// Parse a submit response from the relayer, handling different field naming conventions.
    fn parse_submit_response(body_text: &str) -> Result<RelayerTransactionResponse> {
        let json_value: serde_json::Value = serde_json::from_str(body_text).map_err(|e| {
            SerializationError::JsonDeserialize {
                message: e.to_string(),
                raw_response: body_text.to_string(),
            }
        })?;

        // The relayer uses varying field names across versions
        let transaction_id = json_value
            .get("transactionID")
            .or_else(|| json_value.get("transactionId"))
            .or_else(|| json_value.get("transaction_id"))
            .or_else(|| json_value.get("id"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let state_str = json_value
            .get("state")
            .and_then(|v| v.as_str())
            .unwrap_or("New");

        let state = match state_str {
            "STATE_NEW" | "NEW" | "New" => RelayerTransactionState::New,
            "STATE_EXECUTED" | "EXECUTED" => RelayerTransactionState::Executed,
            "STATE_MINED" | "MINED" => RelayerTransactionState::Mined,
            "STATE_CONFIRMED" | "CONFIRMED" => RelayerTransactionState::Confirmed,
            "STATE_FAILED" | "FAILED" => RelayerTransactionState::Failed,
            "STATE_INVALID" | "INVALID" => RelayerTransactionState::Invalid,
            _ => {
                log::warn!("Unknown transaction state: {}", state_str);
                RelayerTransactionState::New
            }
        };

        let hash = json_value
            .get("transactionHash")
            .or_else(|| json_value.get("hash"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Ok(RelayerTransactionResponse {
            transaction_id,
            state,
            hash,
        })
    }

    /// Poll until a transaction reaches the target state or times out.
    ///
    /// # Arguments
    ///
    /// * `transaction_id` - The transaction ID to poll
    /// * `target_state` - The desired final state
    ///
    /// # Returns
    ///
    /// The final transaction state, or an error if polling times out.
    ///
    /// # Errors
    ///
    /// If polling times out before the transaction reaches the requested state, or a poll
    /// request fails.
    pub async fn poll_until_state(
        &self,
        transaction_id: &str,
        target_state: RelayerTransactionState,
    ) -> Result<RelayerTransaction> {
        self.poll_until_state_with_config(
            transaction_id,
            target_state,
            self.max_poll_attempts,
            self.poll_interval_ms,
        )
        .await
    }

    /// Poll until a transaction reaches the target state with custom configuration.
    ///
    /// # Errors
    ///
    /// If polling times out before the transaction reaches the requested state, or a poll
    /// request fails.
    pub async fn poll_until_state_with_config(
        &self,
        transaction_id: &str,
        target_state: RelayerTransactionState,
        max_attempts: u32,
        interval_ms: u64,
    ) -> Result<RelayerTransaction> {
        let mut last_state: Option<String> = None;

        for attempt in 0..max_attempts {
            let tx = self.get_transaction(transaction_id).await?;

            log::debug!(
                "Poll attempt {}/{}: transaction {} is {:?}",
                attempt + 1,
                max_attempts,
                transaction_id,
                tx.state
            );

            last_state = Some(format!("{:?}", tx.state));

            if tx.state == target_state {
                return Ok(tx);
            }

            // Check for terminal failure states
            if matches!(
                tx.state,
                RelayerTransactionState::Failed | RelayerTransactionState::Invalid
            ) {
                return Err(RelayerError::TransactionFailed {
                    state: format!("{:?}", tx.state),
                    message: Some(format!("Transaction {} reached terminal state", transaction_id)),
                }
                .into());
            }

            tokio::time::sleep(Duration::from_millis(interval_ms)).await;
        }

        Err(RelayerError::PollingTimeout {
            timeout: Duration::from_millis(interval_ms * max_attempts as u64),
            last_state,
        }
        .into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn test_credentials() -> BuilderCredentials {
        BuilderCredentials::new(
            "test_api_key".to_string(),
            "dGVzdF9zZWNyZXQ=".to_string(), // Base64 encoded "test_secret"
            "test_passphrase".to_string(),
        )
    }

    fn test_signer() -> PrivateKeySigner {
        PrivateKeySigner::from_str(
            "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
        ).unwrap()
    }

    #[test]
    fn test_client_builder() {
        let client = RelayerClient::builder()
            .credentials(test_credentials())
            .signer_address("0x1234567890123456789012345678901234567890".to_string())
            .signer(test_signer())
            .build();

        assert_eq!(client.base_url, RELAYER_API);
        assert_eq!(client.chain_id, POLYGON_CHAIN_ID);
        assert_eq!(client.tx_type, RelayerTxType::Safe);
    }

    #[test]
    fn test_client_builder_with_options() {
        let client = RelayerClient::builder()
            .credentials(test_credentials())
            .signer_address("0x1234567890123456789012345678901234567890".to_string())
            .signer(test_signer())
            .tx_type(RelayerTxType::Proxy)
            .chain_id(80001) // Mumbai testnet
            .poll_interval_ms(1000)
            .max_poll_attempts(10)
            .build();

        assert_eq!(client.tx_type, RelayerTxType::Proxy);
        assert_eq!(client.chain_id, 80001);
        assert_eq!(client.poll_interval_ms, 1000);
        assert_eq!(client.max_poll_attempts, 10);
    }

    // ── Gas scaling (proxy batch submissions) ───────────────────────────────
    //
    // Guards against silent fund-stranding: a fixed gas limit makes batches of 2+
    // redeems run out of gas inside the relayed call (RelayHub reports
    // RelayedCallFailed while the outer tx mines), so the limit must scale.

    #[test]
    fn test_proxy_gas_limit_floor_for_small_batches() {
        // An empty batch still gets a whole call's budget.
        assert_eq!(proxy_gas_limit(0), 700_000);
        assert_eq!(proxy_gas_limit(1), 700_000);
    }

    #[test]
    fn test_proxy_gas_limit_scales_linearly() {
        // Gas grows 500k per redeem + 200k base.
        assert_eq!(proxy_gas_limit(2), 1_200_000);
        assert_eq!(proxy_gas_limit(3), 1_700_000);
        assert_eq!(proxy_gas_limit(5), 2_700_000);
    }

    #[test]
    fn test_proxy_gas_limit_covers_worst_measured_batch() {
        // Regression for the 2026-08-13 RelayedCallFailed: two loser redeems
        // estimated at 471_882 + 265_178 = 737_060 against a 640_000 budget.
        assert!(proxy_gas_limit(2) >= 737_060);
    }

    #[test]
    fn test_proxy_gas_limit_is_monotonic() {
        // Larger batches never get less gas than smaller ones.
        let mut prev = 0u64;
        for n in 0..=20 {
            let g = proxy_gas_limit(n);
            assert!(g >= prev, "gas limit must be non-decreasing (n={n})");
            prev = g;
        }
    }

    // ── Relay-status parsing (parse_submit_response) ─────────────────────────
    //
    // The relayer uses varying field names across versions; a mis-parse here
    // could book a rejected submission as success (or vice-versa).

    #[test]
    fn test_parse_submit_response_state_aliases() {
        let cases = [
            ("STATE_NEW", RelayerTransactionState::New),
            ("NEW", RelayerTransactionState::New),
            ("New", RelayerTransactionState::New),
            ("STATE_EXECUTED", RelayerTransactionState::Executed),
            ("EXECUTED", RelayerTransactionState::Executed),
            ("STATE_MINED", RelayerTransactionState::Mined),
            ("MINED", RelayerTransactionState::Mined),
            ("STATE_CONFIRMED", RelayerTransactionState::Confirmed),
            ("CONFIRMED", RelayerTransactionState::Confirmed),
            ("STATE_FAILED", RelayerTransactionState::Failed),
            ("FAILED", RelayerTransactionState::Failed),
            ("STATE_INVALID", RelayerTransactionState::Invalid),
            ("INVALID", RelayerTransactionState::Invalid),
        ];
        for (raw, expected) in cases {
            let body = format!(r#"{{"transactionID":"abc","state":"{raw}"}}"#);
            let parsed = RelayerClient::parse_submit_response(&body).unwrap();
            assert_eq!(parsed.state, expected, "state {raw} mis-parsed");
            assert_eq!(parsed.transaction_id, "abc");
        }
    }

    #[test]
    fn test_parse_submit_response_unknown_state_defaults_to_new() {
        // An unrecognized state must NOT be treated as a terminal failure/success;
        // it defaults to New so the caller keeps polling rather than mis-booking it.
        let body = r#"{"transactionID":"abc","state":"SOMETHING_ELSE"}"#;
        let parsed = RelayerClient::parse_submit_response(body).unwrap();
        assert_eq!(parsed.state, RelayerTransactionState::New);
    }

    #[test]
    fn test_parse_submit_response_id_field_variants() {
        for field in ["transactionID", "transactionId", "transaction_id", "id"] {
            let body = format!(r#"{{"{field}":"tx-123","state":"NEW"}}"#);
            let parsed = RelayerClient::parse_submit_response(&body).unwrap();
            assert_eq!(parsed.transaction_id, "tx-123", "id field {field} not read");
        }
    }

    #[test]
    fn test_parse_submit_response_hash_variants_and_absence() {
        let with_hash = r#"{"transactionID":"a","state":"MINED","transactionHash":"0xdead"}"#;
        assert_eq!(
            RelayerClient::parse_submit_response(with_hash).unwrap().hash,
            Some("0xdead".to_string())
        );
        let with_alt = r#"{"transactionID":"a","state":"MINED","hash":"0xbeef"}"#;
        assert_eq!(
            RelayerClient::parse_submit_response(with_alt).unwrap().hash,
            Some("0xbeef".to_string())
        );
        let without = r#"{"transactionID":"a","state":"NEW"}"#;
        assert_eq!(RelayerClient::parse_submit_response(without).unwrap().hash, None);
    }

    #[test]
    fn test_parse_submit_response_rejects_non_json() {
        assert!(RelayerClient::parse_submit_response("not json at all").is_err());
    }
}
