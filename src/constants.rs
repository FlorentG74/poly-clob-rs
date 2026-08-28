// Common constants shared across the `poly-clob-rs` crate.

/// Multiplier to convert raw integer amounts to human decimals (6 decimals)
pub const RAW_UNIT_MULTIPLIER: u64 = 1_000_000;

/// Default Polygon chain id used by clients and relayer
pub const POLYGON_CHAIN_ID: u64 = 137;

/// Minimum token (share) quantity Polymarket accepts on a buy order.
///
/// Buy orders for fewer shares than this are rejected by `Order::validate_order`,
/// so callers should clamp risk-scaled sizing up to this floor.
pub const MIN_POLY_TOKEN_QUANTITY: u64 = 5;

use rust_decimal::Decimal;

/// Return the raw multiplier as a Decimal for consistent conversions
#[must_use]
pub fn raw_multiplier_decimal() -> Decimal {
    Decimal::from(RAW_UNIT_MULTIPLIER)
}
