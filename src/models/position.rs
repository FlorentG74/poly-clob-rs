use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// A `/v2/positions` row. `rename` marks fields whose Rust name differs from the wire name.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Position {
    pub proxy_wallet: String,
    #[serde(rename = "token_id")]
    pub asset: String,
    pub condition_id: String,
    /// Current holding in shares.
    #[serde(rename = "current_size")]
    pub size: f64,
    pub avg_price: f64,
    /// Fee-exclusive entry basis.
    #[serde(rename = "entry_cost_usdc")]
    pub initial_value: f64,
    pub current_value: f64,
    /// Unrealized profit and loss: `current_value - initial_value`.
    #[serde(rename = "unrealized_pnl")]
    pub cash_pnl: f64,
    pub percent_pnl: f64,
    /// Lifetime bought shares.
    #[serde(rename = "total_size")]
    pub total_bought: f64,
    pub realized_pnl: f64,
    pub percent_realized_pnl: f64,
    #[serde(rename = "current_price")]
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
    #[serde(rename = "opposite_token_id")]
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
