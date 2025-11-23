use serde::{Deserialize, Serialize};

use super::{ApiResponse, Event};

pub type EventSeriesResponse = Vec<PolyResponseEventSeries>;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PolyResponseEventSeries {
    pub id: String,
    pub ticker: String,
    pub slug: String,
    pub title: String,
    pub series_type: String,
    pub recurrence: String,
    //image: String,
    //icon: String,
    pub active: bool,
    pub closed: bool,
    pub archived: bool,
    //featured: bool,
    //restricted: bool,
    //created_at: String,
    //updated_at: String,
    pub volume: f64,
    pub liquidity: f64,
    pub events: Vec<Event>,
    //comment_count: i64,
}

// ApiResponse implementations
impl ApiResponse for EventSeriesResponse {
    fn nb_results(&self) -> usize {
        self.len()
    }
}

impl ApiResponse for PolyResponseEventSeries {
    fn nb_results(&self) -> usize {
        0 // Single event series response
    }
}
