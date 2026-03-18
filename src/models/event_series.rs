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
    /// Events sorted ascending by end_date. Allocates once; callers share this.
    fn sorted_events(&self) -> Vec<&Event> {
        let mut v: Vec<&Event> = self.events.iter().collect();
        v.sort_by_key(|e| e.end_date);
        v
    }

    /// Returns the currently live event: the first event (by end_date) whose end_date is in the
    /// future. Events are sorted internally so the caller does not need to pre-sort.
    pub fn current_event(&self) -> Option<&Event> {
        let now = Utc::now();
        self.sorted_events().into_iter().find(|e| e.end_date > now)
    }

    /// Derives the event window duration in seconds from the gap between the first two
    /// consecutive events. Returns `None` when the series contains fewer than two events.
    pub fn event_duration_secs(&self) -> Option<i64> {
        let sorted = self.sorted_events();
        sorted
            .windows(2)
            .map(|w| (w[1].end_date - w[0].end_date).num_seconds())
            .find(|&d| d > 0)
    }

    /// Returns the start timestamp (Unix seconds) of the currently live event, computed as
    /// `end_date − event_duration`. This is the value expected by `CryptoPriceRequest` as
    /// `event_start_time`.
    ///
    /// Returns `None` if there is no active event or if the duration cannot be determined.
    pub fn current_event_start_ts(&self) -> Option<i64> {
        let sorted = self.sorted_events();
        let now = Utc::now();

        // Derive duration from first consecutive pair (all gaps should be equal).
        let duration = sorted
            .windows(2)
            .map(|w| (w[1].end_date - w[0].end_date).num_seconds())
            .find(|&d| d > 0)?;

        let current = sorted.into_iter().find(|e| e.end_date > now)?;
        Some(current.end_date.timestamp() - duration)
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
