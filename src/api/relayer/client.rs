//! Relayer client for submitting gasless transactions to Polymarket.
//!
//! This module provides the `RelayerClient` struct for interacting with the
//! Polymarket Relayer V2 API. The relayer enables gasless transactions by
//! submitting transactions on behalf of users through Safe or Proxy wallets.

use super::auth::{build_builder_headers, BuilderCredentials, sign_transaction_eip712};
use super::endpoints::{
    GET_DEPLOYED, GET_NONCE, GET_TRANSACTION, GET_TRANSACTIONS, RELAYER_API, SUBMIT_TRANSACTION,
};
use super::types::{
    DeployedResponse, NonceResponse, ProxyTransaction, ProxyTransactionArgs, RelayerTransaction,
    RelayerTransactionResponse, RelayerTransactionState, RelayerTxType, SafeTransaction,
    SignatureType, Transaction, TransactionSubmitRequest, SignatureParamsRequest,
};
use crate::api::http_client::get_http_client;
use alloy::primitives::Address;
use anyhow::{bail, Context, Result};
use hex;
use std::time::Duration;
use typed_builder::TypedBuilder;

/// Default chain ID for Polygon mainnet.
pub const POLYGON_CHAIN_ID: u64 = 137;

/// Default polling interval for transaction state checks.
pub const DEFAULT_POLL_INTERVAL_MS: u64 = 2000;

/// Default maximum polling attempts.
pub const DEFAULT_MAX_POLL_ATTEMPTS: u32 = 30;

/// Client for interacting with the Polymarket Relayer API.
///
/// The relayer client provides methods for:
/// - Querying nonces and deployment status
/// - Submitting Safe or Proxy transactions
/// - Polling for transaction confirmations
///
/// # Example
///
/// ```rust,no_run
/// use poly_clob_rs::api::relayer::{
///     RelayerClient, BuilderCredentials, RelayerTxType,
///     create_redeem_tx, RedeemParams,
/// };
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let creds = BuilderCredentials::from_env()?;
///     let client = RelayerClient::builder()
///         .credentials(creds)
///         .signer_address("0x...".to_string())
///         .build();
///
///     // Get current nonce
///     let nonce = client.get_nonce().await?;
///     println!("Current nonce: {}", nonce);
///
///     // Create and submit a redeem transaction
///     let redeem_tx = create_redeem_tx(&RedeemParams {
///         condition_id: "0x...".to_string(),
///         outcome_index: 0,
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

    /// Signer address (Safe owner or Proxy owner).
    pub signer_address: String,

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
    pub async fn get_nonce(&self) -> Result<u64> {
        self.get_nonce_for_address(&self.signer_address).await
    }

    /// Get the current nonce for a specific address.
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
            .context("failed to send get_nonce request")?;

        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        if !status.is_success() {
            bail!("get_nonce failed with status {}: {}", status, body);
        }

        let nonce_response: NonceResponse = serde_json::from_str(&body)
            .with_context(|| format!("failed to parse nonce response: {}", body))?;

        Ok(nonce_response.nonce)
    }

    /// Check if the Safe/Proxy wallet is deployed.
    pub async fn get_deployed(&self, address: &str) -> Result<bool> {
        let url = format!("{}{}?address={}", self.base_url, GET_DEPLOYED, address);

        let headers = build_builder_headers(&self.credentials, "GET", GET_DEPLOYED, "")?;

        let client = get_http_client(Some(&url));
        let response = client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .context("failed to send get_deployed request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            bail!("get_deployed failed with status {}: {}", status, body);
        }

        let deployed_response: DeployedResponse = response
            .json()
            .await
            .context("failed to parse deployed response")?;

        Ok(deployed_response.deployed)
    }

    /// Get a specific transaction by ID.
    pub async fn get_transaction(&self, transaction_id: &str) -> Result<RelayerTransaction> {
        let url = format!(
            "{}{}?transactionId={}",
            self.base_url, GET_TRANSACTION, transaction_id
        );

        let headers = build_builder_headers(&self.credentials, "GET", GET_TRANSACTION, "")?;

        let client = get_http_client(Some(&url));
        let response = client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .context("failed to send get_transaction request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            bail!("get_transaction failed with status {}: {}", status, body);
        }

        response
            .json()
            .await
            .context("failed to parse transaction response")
    }

    /// Get all transactions for the authenticated builder.
    pub async fn get_transactions(&self) -> Result<Vec<RelayerTransaction>> {
        let url = format!("{}{}", self.base_url, GET_TRANSACTIONS);

        let headers = build_builder_headers(&self.credentials, "GET", GET_TRANSACTIONS, "")?;

        let client = get_http_client(Some(&url));
        let response = client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .context("failed to send get_transactions request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            bail!("get_transactions failed with status {}: {}", status, body);
        }

        response
            .json()
            .await
            .context("failed to parse transactions response")
    }

    /// Submit transactions to the relayer.
    ///
    /// This method automatically routes to either Safe or Proxy submission
    /// based on the client's `tx_type` configuration.
    pub async fn submit(&self, transactions: Vec<Transaction>) -> Result<RelayerTransactionResponse> {
        match self.tx_type {
            RelayerTxType::Safe => {
                let safe_txs: Vec<SafeTransaction> =
                    transactions.into_iter().map(|t| t.into()).collect();
                self.submit_safe(safe_txs).await
            }
            RelayerTxType::Proxy => {
                if transactions.len() != 1 {
                    bail!("Proxy transactions only support a single transaction at a time");
                }
                let proxy_tx: ProxyTransaction = transactions.into_iter().next().unwrap().into();
                self.submit_proxy(proxy_tx).await
            }
        }
    }

    /// Submit Safe transactions to the relayer.
    ///
    /// Safe transactions can be batched - multiple transactions will be executed
    /// atomically in a single on-chain transaction.
    pub async fn submit_safe(
        &self,
        transactions: Vec<SafeTransaction>,
    ) -> Result<RelayerTransactionResponse> {
        if transactions.is_empty() {
            bail!("cannot submit empty transaction list");
        }

        let nonce = self.get_nonce().await?;

        let sender: Address = self
            .signer_address
            .parse()
            .context("invalid signer address")?;

        // For now, we can only submit single transactions via this format
        // TODO: Implement EIP-712 signing to properly batch transactions
        if transactions.len() > 1 {
            log::warn!("Polymarket relayer currently supports single transactions; batching {} transactions as separate requests", transactions.len());
        }

        // Submit each transaction individually
        for tx in transactions {
            // For POLY_PROXY wallets, signature is optional (auth is via Builder API credentials)
            // For other wallet types, we would need to sign with EIP-712
            let signature = if self.signature_type == SignatureType::PolyProxy {
                None
            } else {
                // For EOA and GnosisSafe, include EIP-712 signature
                let sig = sign_transaction_eip712(sender, tx.to, sender, &tx.data, nonce)?;
                Some(sig)
            };

            // For POLY_PROXY, use format with explicit proxyWallet
            let req = if self.signature_type == SignatureType::PolyProxy {
                // For POLY_PROXY, both from and proxyWallet should be the same (poly_address)
                serde_json::json!({
                    "from": sender.to_string(),
                    "to": tx.to.to_string(),
                    "proxyWallet": sender.to_string(),
                    "data": format!("0x{}", hex::encode(tx.data.as_ref())),
                    "type": "SAFE"
                })
            } else {
                // Full format for other wallet types
                serde_json::to_value(&TransactionSubmitRequest {
                    from: sender,
                    to: tx.to,
                    proxy_wallet: sender,
                    data: tx.data,
                    nonce,
                    signature,
                    tx_type: "SAFE".to_string(),
                    signature_params: SignatureParamsRequest {
                        gas_price: "0".to_string(),
                        safe_txn_gas: "0".to_string(),
                        base_gas: "0".to_string(),
                        gas_token: Address::ZERO,
                        refund_receiver: Address::ZERO,
                    },
                    metadata: None,
                })?
            };

            let body = serde_json::to_string(&req).context("failed to serialize transaction request")?;
            log::debug!("Submitting request:\n{}", serde_json::to_string_pretty(&req).unwrap_or_default());

            let url = format!("{}{}", self.base_url, SUBMIT_TRANSACTION);
            let headers = build_builder_headers(&self.credentials, "POST", SUBMIT_TRANSACTION, &body)?;

            let client = get_http_client(Some(&url));
            let response = client
                .post(&url)
                .headers(headers)
                .header("Content-Type", "application/json")
                .body(body.clone())
                .send()
                .await
                .context("failed to send submit_safe request")?;

            let status = response.status();
            let body_text = response.text().await.unwrap_or_default();

            if !status.is_success() {
                log::error!("Submit Safe failed with status {}: {}", status, body_text);
                bail!("submit_safe failed with status {}: {}", status, body_text);
            }

            // Return the first successful response (TODO: collect all responses if batching)
            return serde_json::from_str(&body_text)
                .with_context(|| format!("failed to parse submit response: {}", body_text));
        }

        bail!("no transactions to submit");
    }

    /// Submit a Proxy transaction to the relayer.
    ///
    /// Proxy transactions can only submit one transaction at a time.
    pub async fn submit_proxy(
        &self,
        transaction: ProxyTransaction,
    ) -> Result<RelayerTransactionResponse> {
        let nonce = self.get_nonce().await?;

        let sender: Address = self
            .signer_address
            .parse()
            .context("invalid signer address")?;

        // For proxy, we need to encode the transaction data differently
        let args = ProxyTransactionArgs {
            sender,
            nonce,
            gas_price: alloy::primitives::U256::ZERO, // Let relayer estimate
            gas_limit: None,
            data: transaction.data,
            relayer: Address::ZERO, // Let relayer fill in
        };

        let body = serde_json::to_string(&args).context("failed to serialize proxy transaction")?;
        let url = format!("{}{}", self.base_url, SUBMIT_TRANSACTION);

        let headers = build_builder_headers(&self.credentials, "POST", SUBMIT_TRANSACTION, &body)?;

        let client = get_http_client(Some(&url));
        let response = client
            .post(&url)
            .headers(headers)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await
            .context("failed to send submit_proxy request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            bail!("submit_proxy failed with status {}: {}", status, body);
        }

        response
            .json()
            .await
            .context("failed to parse submit response")
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
    pub async fn poll_until_state_with_config(
        &self,
        transaction_id: &str,
        target_state: RelayerTransactionState,
        max_attempts: u32,
        interval_ms: u64,
    ) -> Result<RelayerTransaction> {
        for attempt in 0..max_attempts {
            let tx = self.get_transaction(transaction_id).await?;

            log::debug!(
                "Poll attempt {}/{}: transaction {} is {:?}",
                attempt + 1,
                max_attempts,
                transaction_id,
                tx.state
            );

            if tx.state == target_state {
                return Ok(tx);
            }

            // Check for terminal failure states
            if matches!(
                tx.state,
                RelayerTransactionState::Failed | RelayerTransactionState::Invalid
            ) {
                bail!(
                    "Transaction {} reached terminal state {:?}",
                    transaction_id,
                    tx.state
                );
            }

            tokio::time::sleep(Duration::from_millis(interval_ms)).await;
        }

        bail!(
            "Polling timed out after {} attempts waiting for transaction {} to reach {:?}",
            max_attempts,
            transaction_id,
            target_state
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_credentials() -> BuilderCredentials {
        BuilderCredentials::new(
            "test_api_key".to_string(),
            "dGVzdF9zZWNyZXQ=".to_string(), // Base64 encoded "test_secret"
            "test_passphrase".to_string(),
        )
    }

    #[test]
    fn test_client_builder() {
        let client = RelayerClient::builder()
            .credentials(test_credentials())
            .signer_address("0x1234567890123456789012345678901234567890".to_string())
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
}
