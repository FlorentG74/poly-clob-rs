//! A CLOB trade as the user websocket `trade` event and `GET /data/trades` report it.
//! Numbers arrive as decimal strings; absent fields default to empty.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ClobTrade {
    /// Trade id; one trade can fill several of our orders (one taker leg, maker legs).
    pub id: String,
    pub taker_order_id: String,
    /// Condition id.
    pub market: String,
    /// Token of the taker leg.
    pub asset_id: String,
    /// Taker side, `BUY` or `SELL`.
    pub side: String,
    /// Taker size.
    pub size: String,
    /// Taker price.
    pub price: String,
    /// `MATCHED`, `MINED`, `CONFIRMED`, `RETRYING`, `FAILED` (the SDK prefixes `TRADE_STATUS_`).
    pub status: String,
    /// Unix seconds.
    pub match_time: String,
    pub outcome: String,
    /// API key of the account the event is for.
    pub owner: String,
    /// `TAKER` or `MAKER`: this account's role.
    pub trader_side: String,
    pub maker_orders: Vec<ClobMakerOrder>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ClobMakerOrder {
    pub order_id: String,
    /// API key of the maker.
    pub owner: String,
    pub matched_amount: String,
    pub price: String,
    pub asset_id: String,
    pub side: String,
    pub outcome: String,
}

/// One page of `GET /data/trades`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ClobTradesPage {
    pub data: Vec<ClobTrade>,
    pub next_cursor: String,
}
