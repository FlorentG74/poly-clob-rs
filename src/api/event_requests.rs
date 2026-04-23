use crate::api::error::Result;
use reqwest::Method;
use typed_builder::TypedBuilder;

use crate::api::http_client::get_http_client;
use super::{WebserviceRequest, GAMMA_API, GET_EVENTS_KEYSET, GET_EVENT_SERIES};
use crate::models::event::{KeysetEventsResponse, PolyResponseEvent};
use crate::models::event_series::PolyResponseEventSeries;

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

    pub fn new_event_series_request(slug: &str) -> Self {
        WebserviceRequest {
            api: GAMMA_API.to_string(),
            url: GET_EVENT_SERIES.to_string(),
            method: Method::GET,
            with_pagination: false,
            args: vec![("slug".to_string(), slug.to_string())],
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

/// Request builder for fetching an event series by slug.
///
/// Returns the series with its list of events, useful for finding
/// the current/upcoming event in a recurring series.
#[derive(TypedBuilder)]
pub struct EventSeriesRequest<'a> {
    #[builder(setter(into))]
    pub slug: &'a str,
}

impl<'a> EventSeriesRequest<'a> {
    pub async fn execute(&self) -> Result<PolyResponseEventSeries> {
        let client = get_http_client(None);

        let web_service_request = WebserviceRequest {
            api: GAMMA_API.to_string(),
            url: GET_EVENT_SERIES.to_string(),
            method: Method::GET,
            with_pagination: false,
            args: vec![("slug".to_string(), self.slug.to_string())],
            body: None,
        };

        let callable_url = web_service_request.get_callable_url(0);
        let series =
            WebserviceRequest::fetch_one::<Vec<PolyResponseEventSeries>>(client, &web_service_request)
                .await?;

        series.into_iter().next().ok_or_else(|| {
            crate::ApiError::NotFound {
                url: callable_url,
                resource: format!("event series with slug: {}", self.slug),
            }
            .into()
        })
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
///     cursor = page.next_cursor.clone().filter(|s| !s.is_empty());
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
        assert_eq!(req.ascending, false);
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
    // KeysetApiResponse trait tests (pure model logic, no network)
    // -------------------------------------------------------------------------

    #[test]
    fn test_keyset_markets_response_next_cursor_some() {
        let resp = KeysetMarketsResponse {
            data: vec![],
            next_cursor: Some("cursor_abc".to_string()),
            limit: Some(100),
            count: Some(0),
        };
        assert_eq!(resp.next_cursor(), Some("cursor_abc"));
    }

    #[test]
    fn test_keyset_markets_response_next_cursor_none() {
        let resp = KeysetMarketsResponse {
            data: vec![],
            next_cursor: None,
            limit: Some(100),
            count: Some(0),
        };
        assert_eq!(resp.next_cursor(), None);
    }

    #[test]
    fn test_keyset_markets_response_next_cursor_empty_string() {
        let resp = KeysetMarketsResponse {
            data: vec![],
            next_cursor: Some("".to_string()),
            limit: Some(100),
            count: Some(0),
        };
        // Empty string means no more pages
        assert_eq!(resp.next_cursor(), None);
    }

    #[test]
    fn test_keyset_markets_response_nb_results() {
        let resp = KeysetMarketsResponse {
            data: vec![],
            next_cursor: None,
            limit: Some(100),
            count: Some(0),
        };
        assert_eq!(resp.nb_results(), 0);
    }

    #[test]
    fn test_keyset_events_response_next_cursor_some() {
        let resp = KeysetEventsResponse {
            data: vec![],
            next_cursor: Some("event_cursor".to_string()),
            limit: Some(100),
            count: Some(0),
        };
        assert_eq!(resp.next_cursor(), Some("event_cursor"));
    }

    #[test]
    fn test_keyset_events_response_next_cursor_none() {
        let resp = KeysetEventsResponse {
            data: vec![],
            next_cursor: None,
            limit: Some(100),
            count: Some(0),
        };
        assert_eq!(resp.next_cursor(), None);
    }

    #[test]
    fn test_keyset_events_response_next_cursor_empty_string() {
        let resp = KeysetEventsResponse {
            data: vec![],
            next_cursor: Some("".to_string()),
            limit: Some(100),
            count: Some(0),
        };
        assert_eq!(resp.next_cursor(), None);
    }

}
