use serde::{Deserialize, Serialize};

use crate::model::PolyResponseMarket;

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenOrder {
    pub id: String,
    pub status: String,
    pub owner: String,
    pub maker_address: String,
    pub market: PolyResponseMarket,
    pub asset_id: String,
    pub side: String,
    pub original_size: f64,
    pub size_matched: f64,
    pub price: f64,
    pub outcome: String,
    pub expiration: String,
    pub order_type: String,
}
