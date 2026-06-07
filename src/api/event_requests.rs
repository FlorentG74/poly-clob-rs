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
// EventsRequest - cursor-based pagination for /events/keyset
// ============================================================================

/// Request builder for listing events using the keyset (cursor-based) pagination
/// endpoint `/events/keyset`.
///
/// This replaces offset-based event listing which is being deprecated.
/// Leave `cursor` as `None` for the first page; pass the `next_cursor` from the
/// previous response for subsequent pages.
///
/// # Example
///
/// ```no_run
/// use poly_clob_rs::api::event_requests::EventsRequest;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let mut cursor: Option<String> = None;
/// loop {
///     let page = EventsRequest::builder()
///         .closed(Some(false))
///         .cursor(cursor.clone())
///         .build()
///         .execute()
///         .await?;
///
///     println!("Got {} events", page.data.len());
///     cursor = page.next_cursor.clone();
///     if cursor.is_none() { break; }
/// }
/// # Ok(())
/// # }
/// ```
#[derive(TypedBuilder)]
pub struct EventsRequest<'a> {
    /// Number of events per page (default: 100)
    #[builder(default = 100)]
    pub limit: i32,
    /// Cursor from the previous response. `None` fetches the first page.
    #[builder(default)]
    pub cursor: Option<String>,

    // Sorting
    /// Field to sort results by
    #[builder(default, setter(into))]
    pub order: Option<&'a str>,
    /// Sort in ascending order (default: false/descending)
    #[builder(default = false)]
    pub ascending: bool,

    // Filters
    /// Filter by event slug
    #[builder(default, setter(into))]
    pub slug: Option<&'a str>,
    /// Filter by event title (partial match)
    #[builder(default, setter(into))]
    pub title: Option<&'a str>,
    /// Filter by closed status
    #[builder(default)]
    pub closed: Option<bool>,
    /// Filter by archived status
    #[builder(default)]
    pub archived: Option<bool>,
    /// Filter by active status
    #[builder(default)]
    pub active: Option<bool>,
    /// Minimum liquidity filter
    #[builder(default)]
    pub liquidity_min: Option<f64>,
    /// Minimum volume filter
    #[builder(default)]
    pub volume_min: Option<f64>,
    /// Filter by tag ID
    #[builder(default)]
    pub tag_id: Option<i32>,
}

impl<'a> EventsRequest<'a> {
    /// Executes a single page fetch against `/events/keyset`.
    ///
    /// Returns a [`KeysetEventsResponse`] whose `next_cursor` field indicates
    /// whether more pages exist.
    pub async fn execute(&self) -> Result<KeysetEventsResponse> {
        let client = get_http_client(None);

        let mut web_service_request = WebserviceRequest {
            api: GAMMA_API.to_string(),
            url: GET_EVENTS_KEYSET.to_string(),
            method: Method::GET,
            with_pagination: false,
            args: Vec::new(),
            body: None,
        };

        if self.limit != 100 {
            web_service_request.add_arg("limit".to_string(), self.limit.to_string());
        }

        if let Some(order) = self.order {
            web_service_request.add_arg("order".to_string(), order.to_string());
        }
        if self.ascending {
            web_service_request.add_arg("ascending".to_string(), "true".to_string());
        }

        if let Some(slug) = self.slug {
            web_service_request.add_arg("slug".to_string(), slug.to_string());
        }
        if let Some(title) = self.title {
            web_service_request.add_arg("title".to_string(), title.to_string());
        }
        if let Some(closed) = self.closed {
            web_service_request.add_arg("closed".to_string(), closed.to_string());
        }
        if let Some(archived) = self.archived {
            web_service_request.add_arg("archived".to_string(), archived.to_string());
        }
        if let Some(active) = self.active {
            web_service_request.add_arg("active".to_string(), active.to_string());
        }
        if let Some(min) = self.liquidity_min {
            web_service_request.add_arg("liquidity_min".to_string(), min.to_string());
        }
        if let Some(min) = self.volume_min {
            web_service_request.add_arg("volume_min".to_string(), min.to_string());
        }
        if let Some(tag_id) = self.tag_id {
            web_service_request.add_arg("tag_id".to_string(), tag_id.to_string());
        }

        let page =
            WebserviceRequest::fetch_keyset::<KeysetEventsResponse>(
                client,
                &web_service_request,
                self.cursor.as_deref(),
            )
            .await?;

        Ok(page)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{KeysetApiResponse, KeysetEventsResponse, KeysetMarketsResponse};

    #[test]
    fn test_events_keyset_request_defaults() {
        let req = EventsRequest::builder().build();
        assert_eq!(req.limit, 100);
        assert!(req.cursor.is_none());
        assert!(req.slug.is_none());
        assert!(req.closed.is_none());
        assert!(!req.ascending);
    }

    #[test]
    fn test_events_keyset_request_with_cursor() {
        let req = EventsRequest::builder()
            .cursor(Some("tok_xyz".to_string()))
            .limit(50)
            .closed(Some(true))
            .build();

        assert_eq!(req.cursor, Some("tok_xyz".to_string()));
        assert_eq!(req.limit, 50);
        assert_eq!(req.closed, Some(true));
    }

    #[test]
    fn test_events_keyset_request_with_filters() {
        let req = EventsRequest::builder()
            .slug(Some("btc-up-or-down-15m"))
            .active(Some(true))
            .volume_min(Some(1000.0))
            .order("volume")
            .ascending(true)
            .build();

        assert_eq!(req.slug, Some("btc-up-or-down-15m"));
        assert_eq!(req.active, Some(true));
        assert_eq!(req.volume_min, Some(1000.0));
        assert_eq!(req.order, Some("volume"));
        assert!(req.ascending);
    }

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
