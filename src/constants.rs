/// Common constants shared across the `poly-clob-rs` crate.
///
/// Aim: centralize values that were previously duplicated across modules
/// (e.g., `RAW_UNIT_MULTIPLIER`, chain ids).
/// Multiplier to convert raw integer amounts to human decimals (6 decimals)
pub const RAW_UNIT_MULTIPLIER: i64 = 1_000_000;

/// Default Polygon chain id used by clients and relayer
pub const POLYGON_CHAIN_ID: u64 = 137;

/// Minimum token (share) quantity Polymarket accepts on a buy order.
/// Buy orders for fewer shares than this are rejected by `Order::validate_order`,
/// so callers should clamp risk-scaled sizing up to this floor.
pub const MIN_POLY_TOKEN_QUANTITY: i64 = 5;

use rust_decimal::Decimal;

/// Return the raw multiplier as a Decimal for consistent conversions
pub fn raw_multiplier_decimal() -> Decimal {
    Decimal::from(RAW_UNIT_MULTIPLIER)
}
