use serde::{Deserialize, Serialize};

use super::Side;

#[derive(Debug, Serialize, Deserialize)]
pub struct MarketOrders {
    pub data: Vec<MarketOrder>,
    pub next_cursor: String,
    pub limit: i64,
    pub count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MarketOrder {
    pub id: String,
    pub status: String,
    pub owner: String,
    pub maker_address: String,
    pub market: String,
    pub asset_id: String,
    pub side: Side,
    pub original_size: String,
    pub size_matched: String,
    pub price: String,
    pub outcome: String,
    pub expiration: String,
    pub order_type: String,
    pub associate_trades: Vec<Option<serde_json::Value>>,
    pub created_at: i64,
}
