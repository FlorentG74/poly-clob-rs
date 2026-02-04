//! Crypto price data for strike setting and settlement resolution.
//!
//! This module provides types for the Polymarket crypto price API endpoint
//! which returns opening and closing prices for crypto events.

use serde::{Deserialize, Serialize};

use super::ApiResponse;

/// Response from the crypto price API endpoint.
///
/// Used for determining strike prices at event start and settlement prices
/// at event maturity for up/down crypto prediction markets.
///
/// # Example Response
/// ```json
/// {
///   "openPrice": 3182.44,
///   "closePrice": 3167.28,
///   "timestamp": 1770225471125,
///   "completed": true,
///   "incomplete": false,
///   "cached": false
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CryptoPriceResponse {
    /// The opening price at event start time
    pub open_price: f64,
    /// The closing price at event maturity (only valid if completed=true)
    pub close_price: f64,
    /// Server timestamp in milliseconds
    pub timestamp: i64,
    /// Whether the event has completed and close_price is final
    pub completed: bool,
    /// Whether the price data is incomplete/unavailable
    pub incomplete: bool,
    /// Whether this response was served from cache
    pub cached: bool,
}

impl CryptoPriceResponse {
    /// Returns true if the price data is valid for settlement
    pub fn is_valid_for_settlement(&self) -> bool {
        self.completed && !self.incomplete
    }

    /// Returns true if the open price is available for strike setting
    pub fn has_open_price(&self) -> bool {
        !self.incomplete
    }
}

impl ApiResponse for CryptoPriceResponse {
    fn nb_results(&self) -> usize {
        1
    }
}
