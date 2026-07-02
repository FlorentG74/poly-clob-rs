use crate::api::error::Result;
use reqwest::Method;
use typed_builder::TypedBuilder;

use crate::api::http_client::get_http_client;
use super::{WebserviceRequest, GAMMA_API, GET_EVENTS, GET_EVENTS_KEYSET};
use crate::models::event::{KeysetEventsResponse, PolyResponseEvent};
use crate::models::market::Event;


impl WebserviceRequest {
    /// Build a request to fetch events by ID via the keyset endpoint.
    pub fn new_event_by_id_request(id: &str) -> Self {
        WebserviceRequest {
            api: GAMMA_API.to_string(),
            url: GET_EVENTS_KEYSET.to_string(),
            method: Method::GET,
            with_pagination: false,
            args: vec![("id".to_string(), id.to_string())],
            body: None,
        }
    }
}

/// Request builder for fetching a single event by slug.
/// The events endpoint returns complete market data including token IDs,
/// unlike the markets endpoint which may return incomplete data for recurring markets.
#[derive(TypedBuilder)]
pub struct EventBySlugRequest<'a> {
    #[builder(setter(into))]
    pub slug: &'a str,
}

impl<'a> EventBySlugRequest<'a> {
    pub async fn execute(&self) -> Result<PolyResponseEvent> {
        let client = get_http_client(None);

        let web_service_request = WebserviceRequest {
            api: GAMMA_API.to_string(),
            url: GET_EVENTS_KEYSET.to_string(),
            method: Method::GET,
            with_pagination: false,
            args: vec![("slug".to_string(), self.slug.to_string())],
            body: None,
        };

        let callable_url = web_service_request.get_keyset_url(None);
        let page =
            WebserviceRequest::fetch_keyset::<KeysetEventsResponse>(client, &web_service_request, None)
                .await?;

        page.data.into_iter().next().ok_or_else(|| {
            crate::ApiError::NotFound {
                url: callable_url,
                resource: format!("event with slug: {}", self.slug),
            }
            .into()
        })
    }
}

// ============================================================================
// SeriesEventsRequest - fetch active events for a series ordered by endDate
// ============================================================================

/// Request builder for fetching the active events of a recurring series, ordered
/// by `end_date` ascending.
///
/// Unlike the `/series` endpoint (capped at the 20 most-recently *created* events),
/// this queries `GET /events` filtered by `series_slug` so it finds whichever
/// events are currently open regardless of when they were created.
/// The first result whose `end_date > now` is the currently-running event.
#[derive(TypedBuilder)]
pub struct SeriesEventsRequest<'a> {
    /// Series slug (e.g. `"btc-up-or-down-hourly"`)
    #[builder(setter(into))]
    pub series_slug: &'a str,
    /// Maximum number of events to return (default: 20).
    #[builder(default = 20)]
    pub limit: i32,
}

impl<'a> SeriesEventsRequest<'a> {
    /// Execute the request and return events ordered by `end_date` ascending
    /// (as requested from the API; no client-side sort).
    pub async fn execute(&self) -> Result<Vec<Event>> {
        let client = get_http_client(None);

        let mut req = WebserviceRequest {
            api: GAMMA_API.to_string(),
            url: GET_EVENTS.to_string(),
            method: Method::GET,
            with_pagination: false,
            args: Vec::new(),
            body: None,
        };

        req.add_arg("series_slug".to_string(), self.series_slug.to_string());
        req.add_arg("order".to_string(), "end_date".to_string());
        req.add_arg("ascending".to_string(), "true".to_string());
        req.add_arg("closed".to_string(), "false".to_string());
        req.add_arg("limit".to_string(), self.limit.to_string());

        WebserviceRequest::fetch_one::<Vec<Event>>(client, &req).await
    }
}


// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use crate::models::{KeysetApiResponse, KeysetEventsResponse, KeysetMarketsResponse};

    // -------------------------------------------------------------------------
    // KeysetApiResponse deserialization tests
    // The empty-string invariant is enforced at parse time by deserialize_cursor,
    // so these tests drive through JSON rather than constructing structs directly.
    // -------------------------------------------------------------------------

    fn parse_markets(json: &str) -> KeysetMarketsResponse {
        serde_json::from_str(json).expect("invalid JSON")
    }

    fn parse_events(json: &str) -> KeysetEventsResponse {
        serde_json::from_str(json).expect("invalid JSON")
    }

    #[test]
    fn test_markets_cursor_present() {
        let resp = parse_markets(r#"{"markets":[],"next_cursor":"cursor_abc"}"#);
        assert_eq!(resp.next_cursor(), Some("cursor_abc"));
        assert_eq!(resp.next_cursor, Some("cursor_abc".to_string()));
    }

    #[test]
    fn test_markets_cursor_null() {
        let resp = parse_markets(r#"{"markets":[],"next_cursor":null}"#);
        assert_eq!(resp.next_cursor(), None);
        assert!(resp.next_cursor.is_none());
    }

    #[test]
    fn test_markets_cursor_empty_string_normalised_to_none() {
        let resp = parse_markets(r#"{"markets":[],"next_cursor":""}"#);
        assert_eq!(resp.next_cursor(), None);
        assert!(resp.next_cursor.is_none(), "empty string must be None after parse");
    }

    #[test]
    fn test_markets_cursor_absent_defaults_to_none() {
        let resp = parse_markets(r#"{"markets":[]}"#);
        assert!(resp.next_cursor.is_none());
    }

    #[test]
    fn test_events_cursor_present() {
        let resp = parse_events(r#"{"events":[],"next_cursor":"event_cursor"}"#);
        assert_eq!(resp.next_cursor(), Some("event_cursor"));
    }

    #[test]
    fn test_events_cursor_empty_string_normalised_to_none() {
        let resp = parse_events(r#"{"events":[],"next_cursor":""}"#);
        assert_eq!(resp.next_cursor(), None);
        assert!(resp.next_cursor.is_none(), "empty string must be None after parse");
    }

    #[test]
    fn test_events_cursor_absent_defaults_to_none() {
        let resp = parse_events(r#"{"events":[]}"#);
        assert!(resp.next_cursor.is_none());
    }

}
