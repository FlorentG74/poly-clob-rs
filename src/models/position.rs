use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

use super::ApiResponse;

pub type PositionsResponse = Vec<Position>;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Position {
    pub proxy_wallet: String,
    pub asset: String,
    pub condition_id: String,
    pub size: f64,
    pub avg_price: f64,
    pub initial_value: f64,
    pub current_value: f64,
    pub cash_pnl: f64,
    pub percent_pnl: f64,
    pub total_bought: f64,
    pub realized_pnl: f64,
    pub percent_realized_pnl: f64,
    pub cur_price: f64,
    pub redeemable: bool,
    pub mergeable: bool,
    pub title: String,
    pub slug: String,
    pub icon: String,
    pub event_slug: String,
    pub outcome: String,
    pub outcome_index: f64,
    pub opposite_outcome: String,
    pub opposite_asset: String,
    pub end_date: String,
    pub negative_risk: bool,
}

impl Position {
    #[must_use]
    pub fn position_vec_to_map(positions: Vec<Self>, floor: f64) -> HashMap<String, Self> {
        let mut positions_map = HashMap::<String, Self>::new();

        for pos in positions {
            if pos.size > floor {
                let market_id = (pos.condition_id).clone();
                positions_map.insert(market_id, pos);
            }
        }

        positions_map
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MarketPosition {{ Market Name: {:?}, Condition Id: {:?}, position b: {:?} }}",
            self.title, self.condition_id, self.total_bought
        )
    }
}

// ApiResponse implementations
impl ApiResponse for PositionsResponse {
    fn nb_results(&self) -> usize {
        self.len()
    }
}

impl ApiResponse for Position {
    fn nb_results(&self) -> usize {
        0 // Single position response
    }
}
