//! Polymarket Relayer Client Module.
//!
//! This module provides a client for interacting with the Polymarket Relayer V2 API,
//! enabling gasless transactions through Safe or Proxy wallets.
//!
//! # Overview
//!
//! The relayer client allows you to:
//! - Submit transactions without paying gas fees
//! - Redeem positions from resolved markets
//! - Execute CTF (Conditional Token Framework) operations
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use poly_clob_rs::api::relayer::{
//!     RelayerClient, BuilderCredentials, create_redeem_tx, RedeemParams,
//!     RelayerTransactionState,
//! };
//! use alloy::signers::local::PrivateKeySigner;
//! use std::str::FromStr;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Load credentials from environment
//!     let creds = BuilderCredentials::from_env()?;
//!     let signer = PrivateKeySigner::from_str("0x...")?;
//!
//!     // Create relayer client
//!     let client = RelayerClient::builder()
//!         .credentials(creds)
//!         .signer_address("0x...".to_string())
//!         .signer(signer)
//!         .build();
//!
//!     // Create a redeem transaction
//!     let tx = create_redeem_tx(&RedeemParams {
//!         condition_id: "0x...".to_string(),
//!         outcome_index: Some(0),
//!     })?;
//!
//!     // Submit and wait for confirmation
//!     let response = client.submit(vec![tx]).await?;
//!     let confirmed = client.poll_until_state(
//!         &response.transaction_id,
//!         RelayerTransactionState::Confirmed,
//!     ).await?;
//!
//!     println!("Transaction confirmed: {:?}", confirmed.hash);
//!     Ok(())
//! }
//! ```
//!
//! # Wallet Types
//!
//! Polymarket supports three wallet/signature types:
//!
//! - **EOA (0)**: Standard Ethereum wallet. The funder pays gas.
//! - **POLY_PROXY (1)**: Magic Link/Google login proxy wallet.
//! - **GNOSIS_SAFE (2)**: Gnosis Safe multisig wallet (most common).
//!
//! Set the wallet type via the `SIGNATURE_TYPE` environment variable or
//! the `signature_type` field on the client.
//!
//! # Transaction Types
//!
//! The relayer supports two transaction execution modes:
//!
//! - **Safe**: Gnosis Safe transactions. Can batch multiple operations.
//! - **Proxy**: Polymarket Proxy transactions. Single operation only.

pub mod auth;
pub mod client;
pub mod endpoints;
pub mod transactions;
pub mod types;

// Re-export commonly used items
pub use auth::{BuilderCredentials, derive_proxy_address, derive_safe_address, sign_proxy_transaction, sign_safe_transaction};
pub use client::RelayerClient;
pub use crate::constants::POLYGON_CHAIN_ID;
pub use endpoints::RELAYER_API;
pub use transactions::{contracts, create_redeem_tx, encode_proxy_call_data, RedeemParams};
pub use types::{
    CallType, DeployedResponse, NonceResponse, OperationType, ProxyTransaction,
    ProxyTransactionArgs, RelayerTransaction, RelayerTransactionResponse, RelayerTransactionState,
    RelayerTxType, SafeTransaction, SafeTransactionArgs, SignatureType, SignatureParamsRequest,
    TransactionSubmitRequest, Transaction,
};
