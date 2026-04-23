use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{ApiResponse, KeysetApiResponse, api_response::deserialize_cursor};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PolyResponseMarket {
    pub id: Option<String>,
    pub question: Option<String>,
    pub condition_id: Option<String>,
    pub slug: Option<String>,
    pub resolution_source: Option<String>,
    pub end_date: Option<String>,
    pub liquidity: Option<String>,
    pub start_date: Option<String>,
    pub image: Option<String>,
    pub icon: Option<String>,
    pub description: Option<String>,
    pub outcomes: Option<String>,
    pub outcome_prices: Option<String>,
    pub volume: Option<String>,
    pub active: Option<bool>,
    pub closed: Option<bool>,
    pub market_maker_address: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub new: Option<bool>,
    pub featured: Option<bool>,
    pub submitted_by: Option<String>,
    pub archived: Option<bool>,
    pub resolved_by: Option<String>,
    pub restricted: Option<bool>,
    pub group_item_title: Option<String>,
    pub group_item_threshold: Option<String>,
    #[serde(rename = "questionID")]
    pub question_id: Option<String>,
    pub enable_order_book: Option<bool>,
    pub order_price_min_tick_size: Option<f64>,
    pub order_min_size: Option<f64>,
    pub volume_num: Option<f64>,
    pub liquidity_num: Option<f64>,
    pub end_date_iso: Option<String>,
    pub start_date_iso: Option<String>,
    pub has_reviewed_dates: Option<bool>,
    #[serde(rename = "volume24hr")]
    pub volume24_hr: Option<f64>,
    pub clob_token_ids: Option<String>,
    pub uma_bond: Option<String>,
    pub uma_reward: Option<String>,
    #[serde(rename = "volume24hrClob")]
    pub volume24_hr_clob: Option<f64>,
    pub volume_clob: Option<f64>,
    pub liquidity_clob: Option<f64>,
    pub accepting_orders: Option<bool>,
    pub neg_risk: Option<bool>,
    pub comment_count: Option<i64>,
    #[serde(rename = "_sync")]
    pub sync: Option<bool>,
    pub events: Option<Vec<Event>>,
    pub ready: Option<bool>,
    pub funded: Option<bool>,
    pub accepting_orders_timestamp: Option<String>,
    pub cyom: Option<bool>,
    pub competitive: Option<f64>,
    pub pager_duty_notification_enabled: Option<bool>,
    pub approved: Option<bool>,
    pub clob_rewards: Option<Vec<ClobReward>>,
    pub rewards_min_size: Option<f64>,
    pub rewards_max_spread: Option<f64>,
    pub spread: Option<f64>,
    pub one_day_price_change: Option<f64>,
    pub last_trade_price: Option<f64>,
    pub best_bid: Option<f64>,
    pub best_ask: Option<f64>,
    pub automatically_active: Option<bool>,
    pub clear_book_on_start: Option<bool>,
    pub game_start_time: Option<String>,
    pub seconds_delay: Option<i64>,
    pub event_start_time: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ClobReward {
    pub id: Option<String>,
    pub condition_id: Option<String>,
    pub asset_address: Option<String>,
    pub rewards_amount: Option<f64>,
    pub rewards_daily_rate: Option<f64>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub id: String,
    pub ticker: Option<String>,
    pub slug: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub start_date: Option<String>,
    pub creation_date: Option<String>,
    #[serde(default)]
    pub end_date: DateTime<Utc>,
    pub image: Option<String>,
    pub icon: Option<String>,
    pub active: Option<bool>,
    pub closed: Option<bool>,
    pub archived: Option<bool>,
    pub new: Option<bool>,
    pub featured: Option<bool>,
    pub restricted: Option<bool>,
    pub liquidity: Option<f64>,
    pub volume: Option<f64>,
    pub open_interest: Option<f64>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub competitive: Option<f64>,
    #[serde(rename = "volume24hr")]
    pub volume24_hr: Option<f64>,
    pub enable_order_book: Option<bool>,
    #[serde(rename = "_sync")]
    pub sync: Option<bool>,
    pub neg_risk: Option<bool>,
    pub comment_count: Option<i64>,
    pub cyom: Option<bool>,
    pub show_all_outcomes: Option<bool>,
    pub show_market_images: Option<bool>,
    pub enable_neg_risk: Option<bool>,
    pub automatically_active: Option<bool>,
}

// Response type aliases for clarity
pub type MarketsResponse = Vec<PolyResponseMarket>;

/// Response envelope for the `/markets/keyset` cursor-based pagination endpoint.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KeysetMarketsResponse {
    /// Markets returned in this page (JSON key: `"markets"`).
    #[serde(rename = "markets")]
    pub data: Vec<PolyResponseMarket>,
    /// Cursor for the next page. `None` when no more pages exist (empty string normalised at parse time).
    #[serde(default, deserialize_with = "deserialize_cursor")]
    pub next_cursor: Option<String>,
    /// Page size limit used for this request.
    pub limit: Option<i32>,
    /// Number of items in this page (may differ from `limit` on the last page).
    pub count: Option<i32>,
}

// ApiResponse implementations
impl ApiResponse for PolyResponseMarket {
    fn nb_results(&self) -> usize {
        0 // Single market response
    }
}

impl ApiResponse for MarketsResponse {
    fn nb_results(&self) -> usize {
        self.len()
    }
}

impl KeysetApiResponse for KeysetMarketsResponse {
    fn next_cursor(&self) -> Option<&str> {
        self.next_cursor.as_deref()
    }
}
