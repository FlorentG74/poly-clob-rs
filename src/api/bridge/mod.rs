//! Polymarket Bridge API client module.
//!
//! This module provides [`BridgeClient`], a client for the Polymarket Bridge API
//! documented at <https://docs.polymarket.com/trading/bridge/deposit>.
//!
//! # Overview
//!
//! The Bridge API lets users deposit assets from multiple blockchains, which are
//! automatically converted to pUSD on Polygon for use as trading collateral, and
//! withdraw collateral back out to other chains/tokens. It supports:
//!
//! - Generating per-network deposit addresses (`/deposit`)
//! - Generating withdrawal addresses (`/withdraw`)
//! - Listing supported assets and minimum amounts (`/supported-assets`)
//! - Quoting a prospective swap/bridge (`/quote`)
//! - Tracking transaction status to completion (`/status/{address}`)
//!
//! Unlike the CLOB and Relayer APIs, the bridge endpoints are **unauthenticated**:
//! the caller's Polymarket wallet address is the only identifier required.
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use poly_clob_rs::api::bridge::{BridgeClient, WithdrawalRequest};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let bridge = BridgeClient::default();
//!
//!     // What can we deposit, and what is the minimum?
//!     for asset in bridge.get_supported_assets().await? {
//!         println!(
//!             "{} {} (min ${})",
//!             asset.chain_name, asset.token.symbol, asset.min_checkout_usd
//!         );
//!     }
//!
//!     // Generate deposit addresses for a Polymarket wallet.
//!     let deposit = bridge
//!         .create_deposit_addresses("0x56687bf447db6ffa42ffe2204a05edaa20f55839")
//!         .await?;
//!     println!("EVM deposit address: {:?}", deposit.address.evm);
//!     Ok(())
//! }
//! ```

pub mod client;
pub mod endpoints;
pub mod types;

// Re-export commonly used items.
pub use client::BridgeClient;
pub use endpoints::{
    BRIDGE_API, CREATE_DEPOSIT_ADDRESSES, CREATE_WITHDRAWAL_ADDRESSES, GET_QUOTE,
    GET_SUPPORTED_ASSETS, GET_TRANSACTION_STATUS,
};
pub use types::{
    BridgeAddresses, BridgeErrorResponse, BridgeTransaction, BridgeTransactionStatus,
    DepositRequest, DepositResponse, FeeBreakdown, QuoteRequest, QuoteResponse, SupportedAsset,
    SupportedAssetsResponse, Token, TransactionStatusResponse, WithdrawalRequest,
    WithdrawalResponse,
};
