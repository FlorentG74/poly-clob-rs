//! Order type definitions for Polymarket CLOB orders.
//!
//! This module defines the different types of orders that can be placed on Polymarket.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Order type for Polymarket CLOB orders.
///
/// # Variants
///
/// * `FOK` - Fill-Or-Kill: A market order that must be executed immediately in its entirety;
///   otherwise, the entire order will be cancelled. For buy orders, the amount is in dollars.
///   For sell orders, the amount is in shares.
///
/// * `FAK` - Fill-And-Kill: A market order that will be executed immediately for as many shares
///   as are available; any portion not filled at once is cancelled. For buy orders, the
///   amount is in dollars. For sell orders, the amount is in shares.
///
/// * `GTC` - Good-Til-Cancelled: A limit order that is active until it is fulfilled or cancelled.
///
/// * `GTD` - Good-Til-Date: A limit order that is active until its specified date (UTC seconds
///   timestamp), unless it has already been fulfilled or cancelled. There is a security
///   threshold of one minute. If the order needs to expire in 90 seconds, the correct
///   expiration value is: now + 1 minute + 30 seconds.
///
/// # Example
///
/// ```
/// use poly_clob_rs::OrderType;
///
/// let fok_order = OrderType::FOK;
/// assert_eq!(fok_order.to_string(), "FOK");
///
/// let gtd_order = OrderType::GTD;
/// assert_eq!(gtd_order.to_string(), "GTD");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderType {
    /// Fill-Or-Kill order - must be executed immediately in full or cancelled
    FOK,
    /// Fill-And-Kill order - execute immediately for available shares, cancel the rest
    FAK,
    /// Good-Til-Cancelled order - active until fulfilled or manually cancelled
    GTC,
    /// Good-Til-Date order - active until specified date or fulfilled/cancelled
    GTD,
}

impl OrderType {
    /// Returns the string representation of the order type as expected by the Polymarket API.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            OrderType::FOK => "FOK",
            OrderType::FAK => "FAK",
            OrderType::GTC => "GTC",
            OrderType::GTD => "GTD",
        }
    }
}

impl fmt::Display for OrderType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Default for OrderType {
    /// Returns FOK as the default order type.
    fn default() -> Self {
        OrderType::FOK
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_type_to_string() {
        assert_eq!(OrderType::FOK.to_string(), "FOK");
        assert_eq!(OrderType::FAK.to_string(), "FAK");
        assert_eq!(OrderType::GTC.to_string(), "GTC");
        assert_eq!(OrderType::GTD.to_string(), "GTD");
    }

    #[test]
    fn test_order_type_as_str() {
        assert_eq!(OrderType::FOK.as_str(), "FOK");
        assert_eq!(OrderType::FAK.as_str(), "FAK");
        assert_eq!(OrderType::GTC.as_str(), "GTC");
        assert_eq!(OrderType::GTD.as_str(), "GTD");
    }

    #[test]
    fn test_order_type_default() {
        assert_eq!(OrderType::default(), OrderType::FOK);
    }

    #[test]
    fn test_order_type_equality() {
        assert_eq!(OrderType::FOK, OrderType::FOK);
        assert_ne!(OrderType::FOK, OrderType::GTC);
    }
}
