use serde::{Serialize, Deserialize};

/// A `/v2/activity` row. `rename` marks fields whose Rust name differs from the wire name.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    #[serde(rename = "token_id")]
    pub asset: String,
    pub side: String,
    pub outcome_index: i64,
    pub title: String,
    pub slug: String,
    pub icon: String,
    pub event_slug: String,
    pub outcome: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub pseudonym: String,
    #[serde(default)]
    pub bio: String,
    #[serde(default)]
    pub profile_image: String,
    #[serde(default)]
    pub profile_image_optimized: String,
}
