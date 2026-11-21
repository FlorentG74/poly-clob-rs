//! User activity request builders.
//!
//! This module provides functions for fetching user activity from the Polymarket Data API v2.

use crate::api::error::Result;
use reqwest::Method;
use typed_builder::TypedBuilder;

use crate::api::http_client::get_http_client;
use crate::models::UserActivity;

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

/// Parameters for fetching user activity from the Polymarket Data API v2.
///
/// Results are always ordered by timestamp (the only order v2 can page by).
///
/// # Required Fields
///
/// * `user` - The user profile address (0x-prefixed, 40 hex characters)
///
/// # Optional Fields (with defaults)
///
/// * `limit` - Page size for [`ActivityRequest::execute`] (default: 100, max 1000)
/// * `condition` - Filter by market condition IDs (at most 20)
/// * `event_id` - Filter by event IDs (at most 20; exclusive with `condition`)
/// * `activity_type` - Filter by activity types
/// * `start` - Start timestamp (inclusive); omitted means three years back, `1` means all
/// * `end` - End timestamp (inclusive)
/// * `sort_direction` - Sort direction (default: DESC)
/// * `side` - Filter by trade side
///
/// # Example
///
/// ```no_run
/// use poly_clob_rs::api::activity_requests::{ActivityRequest, ActivityType, SortDirection};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let request = ActivityRequest::builder()
///     .user("0x961afce6bd9aec79c5cf09d2d4dac2b434b23361")
///     .limit(50)
///     .activity_type(vec![ActivityType::TRADE])
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
    /// Page size for `execute`; `execute_all` pages at the API maximum instead
    #[builder(default = 100)]
    pub limit: i32,
    /// Filter by market condition IDs
    #[builder(default, setter(into))]
    pub condition: Vec<&'a str>,
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
    /// Sort direction
    #[builder(default = SortDirection::DESC)]
    pub sort_direction: SortDirection,
    /// Filter by trade side
    #[builder(default)]
    pub side: Option<ActivitySide>,
}

/// Hard cap enforced by `/v2/activity`; a larger `limit` is rejected.
const MAX_ACTIVITY_PAGE_LIMIT: i32 = 1000;

fn join<T: ToString>(values: &[T]) -> String {
    values.iter().map(ToString::to_string).collect::<Vec<_>>().join(",")
}

impl<'a> ActivityRequest<'a> {
    fn build_web_service_request(&self, limit: i32) -> WebserviceRequest {
        let mut web_service_request = WebserviceRequest::new_activity_ws_request(self.user);

        web_service_request.add_arg("limit".to_string(), limit.to_string());

        if !self.condition.is_empty() {
            web_service_request.add_arg("condition".to_string(), join(&self.condition));
        }

        if !self.event_id.is_empty() {
            web_service_request.add_arg("event_id".to_string(), join(&self.event_id));
        }

        if !self.activity_type.is_empty() {
            let types: Vec<&str> = self.activity_type.iter().map(ActivityType::as_str).collect();
            web_service_request.add_arg("type".to_string(), types.join(","));
        }

        if let Some(start) = self.start {
            web_service_request.add_arg("start".to_string(), start.to_string());
        }

        if let Some(end) = self.end {
            web_service_request.add_arg("end".to_string(), end.to_string());
        }

        web_service_request.add_arg("sort_direction".to_string(), self.sort_direction.as_str().to_string());

        if let Some(side) = self.side {
            web_service_request.add_arg("side".to_string(), side.as_str().to_string());
        }

        web_service_request
    }

    /// Executes the activity request, fetching a single page of `self.limit` rows.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * The HTTP request fails
    /// * The API returns an error response
    /// * The response cannot be deserialized
    pub async fn execute(&self) -> Result<Vec<UserActivity>> {
        let client = get_http_client(Some(DATA_API));
        let web_service_request = self.build_web_service_request(self.limit);
        log::debug!("Activity request URL: {}", web_service_request.get_callable_url(0));

        let page = WebserviceRequest::fetch_data_api_page::<UserActivity>(client, &web_service_request, None)
            .await?;
        Ok(page.data)
    }

    /// Executes the activity request, following the cursor through *all* matching
    /// activity. Ignores `self.limit` — pages at [`MAX_ACTIVITY_PAGE_LIMIT`].
    ///
    /// # Errors
    ///
    /// Returns an error if any page's request fails (network error, API error
    /// response, or deserialization failure).
    pub async fn execute_all(&self) -> Result<Vec<UserActivity>> {
        let client = get_http_client(Some(DATA_API));
        let web_service_request = self.build_web_service_request(MAX_ACTIVITY_PAGE_LIMIT);
        WebserviceRequest::fetch_data_api_all::<UserActivity>(client, &web_service_request).await
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
            with_pagination: false,
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
        assert!(request.condition.is_empty());
        assert!(request.event_id.is_empty());
        assert!(request.activity_type.is_empty());
        assert!(request.start.is_none());
        assert!(request.end.is_none());
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

    /// v2 rejects `offset` with a 400 and takes multi-valued filters comma-separated.
    #[test]
    fn activity_url_is_v2_shaped() {
        let url = ActivityRequest::builder()
            .user("0xabc")
            .activity_type(vec![ActivityType::TRADE, ActivityType::REDEEM])
            .condition(vec!["0x1", "0x2"])
            .start(Some(1))
            .build()
            .build_web_service_request(MAX_ACTIVITY_PAGE_LIMIT)
            .get_callable_url(0);
        assert!(url.starts_with("https://data-api.polymarket.com/v2/activity?user=0xabc"), "{url}");
        assert!(!url.contains("offset"), "{url}");
        assert!(url.contains("&limit=1000"), "{url}");
        assert!(url.contains("&type=TRADE,REDEEM"), "{url}");
        assert!(url.contains("&condition=0x1,0x2"), "{url}");
        assert!(url.contains("&start=1"), "{url}");
        assert!(url.contains("&sort_direction=DESC"), "{url}");
    }

    #[test]
    fn activity_row_deserializes_from_v2_shape() {
        let json = r#"{"data":[{"proxy_wallet":"0xw","timestamp":1792614365,"condition_id":"0xc",
            "type":"REDEEM","size":7.39,"usdc_size":0.0,"transaction_hash":"0xh","price":0.0,
            "token_id":"282","side":"","outcome_index":1,"title":"t","slug":"s","icon":"i",
            "event_slug":"e","outcome":"Down","name":"n","pseudonym":"p","bio":"","profile_image":""}],
            "pagination":{"has_more":true,"limit":1,"offset":0,"next_cursor":"eyJk"}}"#;
        let page: crate::models::DataApiPage<UserActivity> = serde_json::from_str(json).unwrap();
        assert_eq!(page.data[0].asset, "282");
        assert_eq!(page.data[0].welcome_type, "REDEEM");
        assert_eq!(crate::models::KeysetApiResponse::next_cursor(&page), Some("eyJk"));
    }

    /// Live cursor walk over our own wallet's trades, through the same builder, client
    /// and deserializer as the bot, the redeemer and the web UI.
    #[test]
    fn test_activity_live() {
        crate::test_runtime().block_on(async {
            crate::config::init_from_env();
            let account = crate::models::Account::load_poly_account()
                .expect("load poly account from .env");

            let first_page = ActivityRequest::builder()
                .user(account.poly_address.as_str())
                .limit(5)
                .activity_type(vec![ActivityType::TRADE, ActivityType::REDEEM])
                .build()
                .execute()
                .await
                .expect("first activity page");
            assert!(first_page.len() <= 5);

            let all = ActivityRequest::builder()
                .user(account.poly_address.as_str())
                .activity_type(vec![ActivityType::TRADE])
                .start(Some(1))
                .build()
                .execute_all()
                .await
                .expect("full activity walk");
            println!("Own wallet: {} trade(s) over all history", all.len());
            assert!(all.iter().all(|a| a.welcome_type == "TRADE" && !a.asset.is_empty()));
            assert!(all.windows(2).all(|w| w[0].timestamp >= w[1].timestamp), "DESC order across pages");
        });
    }
}
