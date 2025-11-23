use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::ApiResponse;

pub type PolymarketPricesResponse = HashMap<String, PolymarketPrice>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct PolymarketPrice {
    pub buy: Option<String>,
    pub sell: Option<String>,
}

// ApiResponse implementations
impl ApiResponse for PolymarketPricesResponse {
    fn nb_results(&self) -> usize {
        self.len()
    }
}

impl ApiResponse for PolymarketPrice {
    fn nb_results(&self) -> usize {
        0 // Single price response
    }
}
