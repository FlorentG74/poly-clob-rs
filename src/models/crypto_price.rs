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
/// Both `openPrice` and `closePrice` may be `null` when the API hasn't
/// populated the price yet (e.g. very early in a new market's life).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CryptoPriceResponse {
    /// The opening price at event start time. `None` if not yet available.
    pub open_price: Option<f64>,
    /// The closing price at event maturity (only valid if completed=true). `None` if not yet available.
    pub close_price: Option<f64>,
    /// Server timestamp in milliseconds
    pub timestamp: i64,
    /// Whether the event has completed and `close_price` is final
    pub completed: bool,
    /// Whether the price data is incomplete/unavailable
    pub incomplete: bool,
}

impl CryptoPriceResponse {
    /// Returns true if the price data is valid for settlement
    #[must_use]
    pub fn is_valid_for_settlement(&self) -> bool {
        self.completed && !self.incomplete
    }

    /// Returns true if the open price is available for strike setting.
    #[must_use]
    pub fn has_open_price(&self) -> bool {
        self.open_price.map(|p| p > 0.0).unwrap_or(false)
    }
}

impl ApiResponse for CryptoPriceResponse {
    fn nb_results(&self) -> usize {
        1
    }
}
