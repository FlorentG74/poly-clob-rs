use serde::{Deserialize, Serialize};

use super::{ApiResponse, PolyResponseMarket};

pub type EventResponse = Vec<PolyResponseEvent>;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PolyResponseEvent {
    pub id: String,
    pub ticker: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub resolution_source: String,
    pub start_date: String,
    pub creation_date: String,
    pub end_date: String,
    pub image: String,
    pub icon: String,
    pub active: bool,
    pub closed: bool,
    pub archived: bool,
    pub new: bool,
    pub featured: bool,
    pub restricted: bool,
    //liquidity: f64,
    //volume: f64,
    #[serde(default)]
    pub open_interest: f64,
    pub created_at: String,
    pub updated_at: String,
    //competitive: f64,
    pub enable_order_book: bool,
    //liquidity_clob: f64,
    pub neg_risk: bool,
    pub comment_count: i64,
    pub markets: Vec<PolyResponseMarket>,
    //series: Vec<Series>,
    //tags: Vec<Tag>,
    pub cyom: bool,
    pub show_all_outcomes: bool,
    pub show_market_images: bool,
    pub enable_neg_risk: bool,
    pub automatically_active: Option<bool>,
    pub series_slug: String,
    pub neg_risk_augmented: bool,
    pub pending_deployment: bool,
    pub deploying: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Series {
    pub id: String,
    pub ticker: String,
    pub slug: String,
    pub title: String,
    pub series_type: String,
    pub recurrence: String,
    pub image: String,
    pub icon: String,
    pub active: bool,
    pub closed: bool,
    pub archived: bool,
    pub featured: bool,
    pub restricted: bool,
    pub created_at: String,
    pub updated_at: String,
    pub volume: f64,
    pub liquidity: f64,
    pub comment_count: i64,
}

// ApiResponse implementations
impl ApiResponse for EventResponse {
    fn nb_results(&self) -> usize {
        self.len()
    }
}

impl ApiResponse for PolyResponseEvent {
    fn nb_results(&self) -> usize {
        0 // Single event response
    }
}
