use serde::{Deserialize, Serialize};

use super::{ApiResponse, KeysetApiResponse, PolyResponseMarket, api_response::deserialize_cursor};

pub type EventResponse = Vec<PolyResponseEvent>;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
// `default` tolerates event-level fields the gamma API omits for some event types — e.g.
// "what price will X hit" ladder events omit `resolutionSource` (and others) that up/down
// events always send. Missing keys fall back to type defaults instead of erroring.
#[serde(rename_all = "camelCase", default)]
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

/// Response envelope for the `/events/keyset` cursor-based pagination endpoint.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KeysetEventsResponse {
    /// Events returned in this page (JSON key: `"events"`).
    #[serde(rename = "events")]
    pub data: Vec<PolyResponseEvent>,
    /// Cursor for the next page. `None` when no more pages exist (empty string normalised at parse time).
    #[serde(default, deserialize_with = "deserialize_cursor")]
    pub next_cursor: Option<String>,
    /// Page size limit used for this request.
    pub limit: Option<i32>,
    /// Number of items in this page (may differ from `limit` on the last page).
    pub count: Option<i32>,
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

impl KeysetApiResponse for KeysetEventsResponse {
    fn next_cursor(&self) -> Option<&str> {
        self.next_cursor.as_deref()
    }
}
