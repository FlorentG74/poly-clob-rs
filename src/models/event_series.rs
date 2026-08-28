use chrono::Utc;
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
    #[serde(default)]
    pub volume: f64,
    #[serde(default)]
    pub liquidity: f64,
    pub events: Vec<Event>,
    //comment_count: i64,
}

impl PolyResponseEventSeries {
    /// Events sorted ascending by `end_date`. Allocates once; callers share this.
    fn sorted_events(&self) -> Vec<&Event> {
        let mut v: Vec<&Event> = self.events.iter().collect();
        v.sort_by_key(|e| e.end_date);
        v
    }

    /// Returns the currently live event: the first event (by `end_date`) whose `end_date` is in the
    /// future. Events are sorted internally so the caller does not need to pre-sort.
    #[must_use]
    pub fn current_event(&self) -> Option<&Event> {
        let now = Utc::now();
        self.sorted_events().into_iter().find(|e| e.end_date > now)
    }

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
