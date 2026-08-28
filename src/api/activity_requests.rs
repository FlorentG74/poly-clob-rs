//! User activity request builders.
//!
//! This module provides functions for fetching user activity from the Polymarket Data API.

use crate::api::error::Result;
use reqwest::Method;
use typed_builder::TypedBuilder;

use crate::api::http_client::get_http_client;
use crate::models::{UserActivityResponse, UserActivity};

pub use super::SortDirection;
use super::{WebserviceRequest, ACTIVITY, DATA_API};

/// Activity types for filtering user activity.
#[derive(Debug, Clone, Copy)]
pub enum ActivityType {
    TRADE,
    SPLIT,
    MERGE,
    REDEEM,
    REWARD,
    CONVERSION,
}

impl ActivityType {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            ActivityType::TRADE => "TRADE",
            ActivityType::SPLIT => "SPLIT",
            ActivityType::MERGE => "MERGE",
            ActivityType::REDEEM => "REDEEM",
            ActivityType::REWARD => "REWARD",
            ActivityType::CONVERSION => "CONVERSION",
        }
    }
}

/// Sort fields for activity requests.
#[derive(Debug, Clone, Copy)]
pub enum ActivitySortBy {
    TIMESTAMP,
    TOKENS,
    CASH,
}

impl ActivitySortBy {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            ActivitySortBy::TIMESTAMP => "TIMESTAMP",
            ActivitySortBy::TOKENS => "TOKENS",
            ActivitySortBy::CASH => "CASH",
        }
    }
}

/// Side for filtering activity (BUY/SELL).
#[derive(Debug, Clone, Copy)]
pub enum ActivitySide {
    BUY,
    SELL,
}

impl ActivitySide {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            ActivitySide::BUY => "BUY",
            ActivitySide::SELL => "SELL",
        }
    }
}

/// Parameters for fetching user activity from the Polymarket Data API.
///
/// # Required Fields
///
/// * `user` - The user profile address (0x-prefixed, 40 hex characters)
///
/// # Optional Fields (with defaults)
///
/// * `limit` - Maximum number of activities to return (default: 100)
/// * `offset` - Pagination offset (default: 0)
/// * `market` - Filter by market condition IDs
/// * `event_id` - Filter by event IDs
/// * `activity_type` - Filter by activity types
/// * `start` - Start timestamp for filtering
/// * `end` - End timestamp for filtering
/// * `sort_by` - Sort field (default: TIMESTAMP)
/// * `sort_direction` - Sort direction (default: DESC)
/// * `side` - Filter by trade side
///
/// # Example
///
/// ```no_run
/// use poly_clob_rs::api::activity_requests::{ActivityRequest, ActivityType, ActivitySortBy, SortDirection};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let request = ActivityRequest::builder()
///     .user("0x961afce6bd9aec79c5cf09d2d4dac2b434b23361")
///     .limit(50)
///     .activity_type(vec![ActivityType::TRADE])
///     .sort_by(ActivitySortBy::TIMESTAMP)
///     .sort_direction(SortDirection::DESC)
///     .build();
///
/// let activities = request.execute().await?;
/// # Ok(())
/// # }
/// ```
#[derive(TypedBuilder)]
pub struct ActivityRequest<'a> {
    /// The user profile address (0x-prefixed, 40 hex characters)
    #[builder(setter(into))]
    pub user: &'a str,
    /// Maximum number of activities to return
    #[builder(default = 100)]
    pub limit: i32,
    /// Pagination offset
    #[builder(default = 0)]
    pub offset: i32,
    /// Filter by market condition IDs
    #[builder(default, setter(into))]
    pub market: Vec<&'a str>,
    /// Filter by event IDs
    #[builder(default)]
    pub event_id: Vec<i64>,
    /// Filter by activity types
    #[builder(default)]
    pub activity_type: Vec<ActivityType>,
    /// Start timestamp for filtering
    #[builder(default)]
    pub start: Option<i64>,
    /// End timestamp for filtering
    #[builder(default)]
    pub end: Option<i64>,
    /// Sort field
    #[builder(default = ActivitySortBy::TIMESTAMP)]
    pub sort_by: ActivitySortBy,
    /// Sort direction
    #[builder(default = SortDirection::DESC)]
    pub sort_direction: SortDirection,
    /// Filter by trade side
    #[builder(default)]
    pub side: Option<ActivitySide>,
}

/// Hard cap enforced by the Data API's `/activity` endpoint; requests with a
/// larger `limit` 400 with `"max activity limit of 500 exceeded"`.
const MAX_ACTIVITY_PAGE_LIMIT: i32 = 500;

impl<'a> ActivityRequest<'a> {
    fn build_web_service_request(&self, limit_override: Option<i32>) -> WebserviceRequest {
        let mut web_service_request = WebserviceRequest::new_activity_ws_request(self.user);

        let limit = limit_override.unwrap_or(self.limit);
        if limit != 100 {
            web_service_request.add_arg("limit".to_string(), limit.to_string());
        }
        if limit_override.is_none() && self.offset != 0 {
            web_service_request.add_arg("offset".to_string(), self.offset.to_string());
        }

        for market in &self.market {
            web_service_request.add_arg("market".to_string(), market.to_string());
        }

        for event_id in &self.event_id {
            web_service_request.add_arg("eventId".to_string(), event_id.to_string());
        }

        for activity_type in &self.activity_type {
            web_service_request.add_arg("type".to_string(), activity_type.as_str().to_string());
        }

        if let Some(start) = self.start {
            web_service_request.add_arg("start".to_string(), start.to_string());
        }

        if let Some(end) = self.end {
            web_service_request.add_arg("end".to_string(), end.to_string());
        }

        web_service_request.add_arg("sortBy".to_string(), self.sort_by.as_str().to_string());
        web_service_request.add_arg("sortDirection".to_string(), self.sort_direction.as_str().to_string());

        if let Some(side) = self.side {
            web_service_request.add_arg("side".to_string(), side.as_str().to_string());
        }

        web_service_request
    }

    /// Executes the activity request, fetching a single page.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Vec<UserActivity>)` with the user's activity on success, or an error on failure.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * The HTTP request fails
    /// * The API returns an error response
    /// * The response cannot be deserialized
    pub async fn execute(&self) -> Result<Vec<UserActivity>> {
        let client = get_http_client(Some(DATA_API));
        let web_service_request = self.build_web_service_request(None);

        let callable_url = web_service_request.get_callable_url(0);
        log::debug!("Activity request URL: {}", callable_url);

        let result = WebserviceRequest::fetch_one::<UserActivityResponse>(client, &web_service_request)
            .await?;

        Ok(result)
    }

    /// Executes the activity request, transparently paging through *all* matching
    /// activity via offset-based pagination (`WebserviceRequest::fetch_batch`) until
    /// a page comes back short of the page size.
    ///
    /// Ignores `self.limit`/`self.offset` — pages internally at
    /// [`MAX_ACTIVITY_PAGE_LIMIT`], the API's hard per-request cap.
    ///
    /// # Errors
    ///
    /// Returns an error if any page's request fails (network error, API error
    /// response, or deserialization failure).
    pub async fn execute_all(&self) -> Result<Vec<UserActivity>> {
        let client = get_http_client(Some(DATA_API));
        let web_service_request = self.build_web_service_request(Some(MAX_ACTIVITY_PAGE_LIMIT));

        let mut all = Vec::new();
        let mut next_offset = 0i32;
        loop {
            let (offset, page) = WebserviceRequest::fetch_batch::<UserActivityResponse>(
                client,
                &web_service_request,
                next_offset,
            )
            .await?;
            all.extend(page);
            if offset < 0 {
                break;
            }
            next_offset = offset;
        }

        Ok(all)
    }
}

impl WebserviceRequest {
    #[must_use]
    pub fn new_activity_ws_request(user: &str) -> Self {
        let args = vec![("user".to_string(), user.to_string())];

        WebserviceRequest {
            api: DATA_API.to_string(),
            url: ACTIVITY.to_string(),
            method: Method::GET,
            with_pagination: true,
            args,
            body: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_activity_type_as_str() {
        assert_eq!(ActivityType::TRADE.as_str(), "TRADE");
        assert_eq!(ActivityType::SPLIT.as_str(), "SPLIT");
        assert_eq!(ActivityType::MERGE.as_str(), "MERGE");
        assert_eq!(ActivityType::REDEEM.as_str(), "REDEEM");
        assert_eq!(ActivityType::REWARD.as_str(), "REWARD");
        assert_eq!(ActivityType::CONVERSION.as_str(), "CONVERSION");
    }

    #[test]
    fn test_activity_sort_by_as_str() {
        assert_eq!(ActivitySortBy::TIMESTAMP.as_str(), "TIMESTAMP");
        assert_eq!(ActivitySortBy::TOKENS.as_str(), "TOKENS");
        assert_eq!(ActivitySortBy::CASH.as_str(), "CASH");
    }

    #[test]
    fn test_sort_direction_as_str() {
        assert_eq!(SortDirection::ASC.as_str(), "ASC");
        assert_eq!(SortDirection::DESC.as_str(), "DESC");
    }

    #[test]
    fn test_activity_side_as_str() {
        assert_eq!(ActivitySide::BUY.as_str(), "BUY");
        assert_eq!(ActivitySide::SELL.as_str(), "SELL");
    }

    #[test]
    fn test_activity_request_builder_defaults() {
        let request = ActivityRequest::builder()
            .user("0x961afce6bd9aec79c5cf09d2d4dac2b434b23361")
            .build();

        assert_eq!(request.user, "0x961afce6bd9aec79c5cf09d2d4dac2b434b23361");
        assert_eq!(request.limit, 100);
        assert_eq!(request.offset, 0);
        assert!(request.market.is_empty());
        assert!(request.event_id.is_empty());
        assert!(request.activity_type.is_empty());
        assert!(request.start.is_none());
        assert!(request.end.is_none());
        assert!(matches!(request.sort_by, ActivitySortBy::TIMESTAMP));
        assert!(matches!(request.sort_direction, SortDirection::DESC));
        assert!(request.side.is_none());
    }

    #[test]
    fn test_webservice_request_new_activity() {
        let ws_request = WebserviceRequest::new_activity_ws_request("0x123");

        assert_eq!(ws_request.api, DATA_API);
        assert_eq!(ws_request.url, ACTIVITY);
        assert_eq!(ws_request.method, Method::GET);
        assert_eq!(ws_request.args.len(), 1);
        assert_eq!(ws_request.args[0], ("user".to_string(), "0x123".to_string()));
        assert!(ws_request.body.is_none());
    }
}
