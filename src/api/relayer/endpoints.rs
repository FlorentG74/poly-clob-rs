//! Relayer API endpoint constants.
//!
//! This module defines the base URL and endpoint paths for the Polymarket
//! Relayer V2 API.

/// Base URL for the Polymarket Relayer V2 API.
pub static RELAYER_API: &str = "https://relayer-v2.polymarket.com";

/// Get nonce for a signer address.
/// Query params: `address`, `type` (SAFE or PROXY)
pub static GET_NONCE: &str = "/nonce";

/// Get a specific transaction by ID.
/// Query params: `transactionId`
pub static GET_TRANSACTION: &str = "/transaction";

/// Get all transactions for the authenticated builder.
pub static GET_TRANSACTIONS: &str = "/transactions";

/// Submit a transaction to the relayer.
pub static SUBMIT_TRANSACTION: &str = "/submit";

/// Check if a Safe/Proxy is deployed.
/// Query params: `address`
pub static GET_DEPLOYED: &str = "/deployed";

/// Get relay payload for proxy transactions.
/// Query params: `address`, `type`
pub static GET_RELAY_PAYLOAD: &str = "/relay-payload";
