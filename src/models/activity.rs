use serde::{Serialize, Deserialize};

use super::ApiResponse;

pub type UserActivityResponse = Vec<UserActivity>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserActivity {
    pub proxy_wallet: String,
    pub timestamp: i64,
    pub condition_id: String,
    #[serde(rename = "type")]
    pub welcome_type: String,
    pub size: f64,
    pub usdc_size: f64,
    pub transaction_hash: String,
    pub price: f64,
    pub asset: String,
    pub side: String,
    pub outcome_index: i64,
    pub title: String,
    pub slug: String,
    pub icon: String,
    pub event_slug: String,
    pub outcome: String,
    pub name: String,
    pub pseudonym: String,
    pub bio: String,
    pub profile_image: String,
    pub profile_image_optimized: String,
}

impl ApiResponse for UserActivity {
    fn nb_results(&self) -> usize {
        1 // Single activity item
    }
}

impl ApiResponse for UserActivityResponse {
    fn nb_results(&self) -> usize {
        self.len()
    }
}
