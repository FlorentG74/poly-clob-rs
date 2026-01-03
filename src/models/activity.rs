use serde::{Serialize, Deserialize};

pub type UserActivityResponse = Vec<UserActivity>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserActivity {
    proxy_wallet: String,
    timestamp: i64,
    condition_id: String,
    #[serde(rename = "type")]
    welcome_type: String,
    size: i64,
    usdc_size: i64,
    transaction_hash: String,
    price: i64,
    asset: String,
    side: String,
    outcome_index: i64,
    title: String,
    slug: String,
    icon: String,
    event_slug: String,
    outcome: String,
    name: String,
    pseudonym: String,
    bio: String,
    profile_image: String,
    profile_image_optimized: String,
}
