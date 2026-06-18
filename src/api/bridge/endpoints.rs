//! Bridge API endpoint constants.
//!
//! This module defines the base URL and endpoint paths for the Polymarket
//! Bridge API (deposits, withdrawals, quotes, supported assets and status).

/// Base URL for the Polymarket Bridge API.
pub static BRIDGE_API: &str = "https://bridge.polymarket.com";

/// Create bridge (deposit) addresses for a Polymarket wallet.
/// Method: `POST`. Body: [`DepositRequest`](super::types::DepositRequest).
pub static CREATE_DEPOSIT_ADDRESSES: &str = "/deposit";

/// Create withdrawal addresses for a destination chain/token.
/// Method: `POST`. Body: [`WithdrawalRequest`](super::types::WithdrawalRequest).
pub static CREATE_WITHDRAWAL_ADDRESSES: &str = "/withdraw";

/// Get the list of supported assets with minimum deposit/withdrawal amounts.
/// Method: `GET`.
pub static GET_SUPPORTED_ASSETS: &str = "/supported-assets";

/// Get a swap/bridge quote.
/// Method: `POST`. Body: [`QuoteRequest`](super::types::QuoteRequest).
pub static GET_QUOTE: &str = "/quote";

/// Get the status of transactions for a bridge address.
/// Method: `GET`. The bridge address is appended as a path segment: `/status/{address}`.
pub static GET_TRANSACTION_STATUS: &str = "/status";
