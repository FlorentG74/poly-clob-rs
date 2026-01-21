//! Market request builders for the Polymarket Gamma API.
//!
//! This module provides builders for fetching market data from the Polymarket Gamma API.
//! It supports fetching individual markets by slug or listing/filtering multiple markets
//! with comprehensive filtering and sorting options.
//!
//! # Examples
//!
//! ## Fetch a single market by slug
//!
//! ```no_run
//! use poly_clob_rs::api::market_requests::MarketBySlugRequest;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let market = MarketBySlugRequest::builder()
//!     .slug("bitcoin-above-100k")
//!     .build()
//!     .execute()
//!     .await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## List active markets with volume filter
//!
//! ```no_run
//! use poly_clob_rs::api::market_requests::MarketsRequest;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let markets = MarketsRequest::builder()
//!     .closed(Some(false))
//!     .volume_num_min(Some(1000.0))
//!     .limit(50)
//!     .build()
//!     .execute()
//!     .await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Filter markets by condition IDs
//!
//! ```no_run
//! use poly_clob_rs::api::market_requests::MarketsRequest;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let markets = MarketsRequest::builder()
//!     .condition_ids(vec!["0x123", "0x456"])
//!     .build()
//!     .execute()
//!     .await?;
//! # Ok(())
//! # }
//! ```

use std::collections::HashMap;

use crate::api::error::Result;
use reqwest::Method;
use typed_builder::TypedBuilder;

use crate::api::http_client::get_http_client;
use crate::{MarketsResponse, PolyResponseMarket};

use super::{GAMMA_API, GET_MARKETS, WITH_SLUG};

// ============================================================================
// Enums
// ============================================================================

/// Sort fields for market requests.
#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum MarketSortBy {
    /// Sort by total volume traded
    VOLUME,
    /// Sort by liquidity
    LIQUIDITY,
    /// Sort by creation time
    CREATED_AT,
    /// Sort by event start date
    START_DATE,
    /// Sort by event end date
    END_DATE,
    /// Sort by 24-hour volume
    VOLUME_24HR,
    /// Sort by last trade price
    LAST_TRADE_PRICE,
}

impl MarketSortBy {
    /// Returns the API parameter value for this sort field.
    pub fn as_str(&self) -> &'static str {
        match self {
            MarketSortBy::VOLUME => "volume",
            MarketSortBy::LIQUIDITY => "liquidity",
            MarketSortBy::CREATED_AT => "createdAt",
            MarketSortBy::START_DATE => "startDate",
            MarketSortBy::END_DATE => "endDate",
            MarketSortBy::VOLUME_24HR => "volume24hr",
            MarketSortBy::LAST_TRADE_PRICE => "lastTradePrice",
        }
    }
}

/// UMA resolution status for filtering markets.
#[derive(Debug, Clone, Copy)]
pub enum UmaResolutionStatus {
    /// Market has been resolved
    RESOLVED,
    /// Market resolution is pending
    PENDING,
    /// Market resolution is disputed
    DISPUTED,
}

impl UmaResolutionStatus {
    /// Returns the API parameter value for this status.
    pub fn as_str(&self) -> &'static str {
        match self {
            UmaResolutionStatus::RESOLVED => "resolved",
            UmaResolutionStatus::PENDING => "pending",
            UmaResolutionStatus::DISPUTED => "disputed",
        }
    }
}

/// Sports market types for filtering.
#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum SportsMarketType {
    /// Moneyline market (winner/loser)
    MONEYLINE,
    /// Spread market (point spread)
    SPREAD,
    /// Over/Under market (total score)
    OVER_UNDER,
}

impl SportsMarketType {
    /// Returns the API parameter value for this market type.
    pub fn as_str(&self) -> &'static str {
        match self {
            SportsMarketType::MONEYLINE => "moneyline",
            SportsMarketType::SPREAD => "spread",
            SportsMarketType::OVER_UNDER => "over_under",
        }
    }
}

// ============================================================================
// MarketBySlugRequest - for fetching a single market by slug
// ============================================================================

/// Request builder for fetching a single market by slug.
///
/// # Required Fields
///
/// * `slug` - The market slug identifier
///
/// # Example
///
/// ```no_run
/// use poly_clob_rs::api::market_requests::MarketBySlugRequest;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let request = MarketBySlugRequest::builder()
///     .slug("bitcoin-above-100k")
///     .build();
///
/// let market = request.execute().await?;
/// # Ok(())
/// # }
/// ```
#[derive(TypedBuilder)]
pub struct MarketBySlugRequest<'a> {
    /// The market slug identifier
    #[builder(setter(into))]
    pub slug: &'a str,
}

impl<'a> MarketBySlugRequest<'a> {
    /// Executes the market request.
    ///
    /// # Returns
    ///
    /// Returns `Ok(PolyResponseMarket)` with the market data on success, or an error on failure.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * The HTTP request fails
    /// * The API returns an error response (e.g., 404 if market not found)
    /// * The response cannot be deserialized
    pub async fn execute(&self) -> Result<PolyResponseMarket> {
        let client = get_http_client(None);

        let web_service_request = super::webservice_request::WebserviceRequest {
            api: GAMMA_API.to_string(),
            url: format!("{}{}{}", GET_MARKETS, WITH_SLUG, self.slug),
            method: Method::GET,
            with_pagination: false,
            args: Vec::new(),
            body: None,
        };

        let callable_url = web_service_request.get_callable_url(0);
        super::webservice_request::WebserviceRequest::fetch_one::<PolyResponseMarket>(
            &client,
            &web_service_request,
        )
        .await
        .ok_or_else(|| crate::api::error::ApiError::NotFound {
            url: callable_url,
            resource: format!("market with slug: {}", self.slug),
        }.into())
    }
}

// ============================================================================
// MarketsRequest - for listing and filtering markets
// ============================================================================

/// Request builder for listing and filtering markets from the Polymarket Gamma API.
///
/// This builder provides comprehensive filtering, pagination, and sorting options for querying
/// the Polymarket markets. All fields are optional with sensible defaults.
///
/// # Optional Fields (with defaults)
///
/// ## Pagination
/// * `limit` - Maximum number of markets to return (default: 100)
/// * `offset` - Pagination offset (default: 0)
///
/// ## Sorting
/// * `order` - Comma-separated list of fields to order by (e.g., "volume,liquidity")
/// * `ascending` - Sort in ascending order (default: false/descending)
///
/// ## ID Filters
/// * `id` - Filter by market IDs (can specify multiple)
/// * `slug` - Filter by market slugs (query parameter, different from MarketBySlugRequest)
/// * `clob_token_ids` - Filter by CLOB token IDs (can specify multiple)
/// * `condition_ids` - Filter by condition IDs (can specify multiple)
/// * `market_maker_address` - Filter by market maker addresses (can specify multiple)
/// * `question_ids` - Filter by question IDs (can specify multiple)
///
/// ## Numeric Range Filters
/// * `liquidity_num_min` - Minimum liquidity
/// * `liquidity_num_max` - Maximum liquidity
/// * `volume_num_min` - Minimum volume
/// * `volume_num_max` - Maximum volume
/// * `rewards_min_size` - Minimum rewards size
///
/// ## Date Filters
/// * `start_date_min` - Minimum event start date (ISO 8601 format)
/// * `start_date_max` - Maximum event start date (ISO 8601 format)
/// * `end_date_min` - Minimum event end date (ISO 8601 format)
/// * `end_date_max` - Maximum event end date (ISO 8601 format)
///
/// ## Boolean Filters
/// * `closed` - Filter by closed status
/// * `related_tags` - Include related tags
/// * `cyom` - Filter CYOM (Create Your Own Market) markets
/// * `include_tag` - Include tag information in response
///
/// ## Category/Type Filters
/// * `tag_id` - Filter by tag ID
/// * `uma_resolution_status` - Filter by UMA resolution status
/// * `game_id` - Filter by game ID
/// * `sports_market_types` - Filter by sports market types (can specify multiple)
///
/// # Example
///
/// ```no_run
/// use poly_clob_rs::api::market_requests::MarketsRequest;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let request = MarketsRequest::builder()
///     .limit(50)
///     .closed(Some(false))
///     .volume_num_min(Some(1000.0))
///     .order("volume")
///     .ascending(false)
///     .build();
///
/// let markets = request.execute().await?;
/// # Ok(())
/// # }
/// ```
#[derive(TypedBuilder)]
pub struct MarketsRequest<'a> {
    // Pagination
    /// Maximum number of markets to return
    #[builder(default = 100)]
    pub limit: i32,
    /// Pagination offset
    #[builder(default = 0)]
    pub offset: i32,

    // Sorting
    /// Comma-separated list of fields to order by (e.g., "volume,liquidity")
    #[builder(default, setter(into))]
    pub order: Option<&'a str>,
    /// Sort in ascending order (default: false/descending)
    #[builder(default = false)]
    pub ascending: bool,

    // ID Filters
    /// Filter by market IDs
    #[builder(default)]
    pub id: Vec<i64>,
    /// Filter by market slugs
    #[builder(default, setter(into))]
    pub slug: Vec<&'a str>,
    /// Filter by CLOB token IDs
    #[builder(default, setter(into))]
    pub clob_token_ids: Vec<&'a str>,
    /// Filter by condition IDs
    #[builder(default, setter(into))]
    pub condition_ids: Vec<&'a str>,
    /// Filter by market maker addresses
    #[builder(default, setter(into))]
    pub market_maker_address: Vec<&'a str>,
    /// Filter by question IDs
    #[builder(default, setter(into))]
    pub question_ids: Vec<&'a str>,

    // Numeric filters
    /// Minimum liquidity
    #[builder(default)]
    pub liquidity_num_min: Option<f64>,
    /// Maximum liquidity
    #[builder(default)]
    pub liquidity_num_max: Option<f64>,
    /// Minimum volume
    #[builder(default)]
    pub volume_num_min: Option<f64>,
    /// Maximum volume
    #[builder(default)]
    pub volume_num_max: Option<f64>,
    /// Minimum rewards size
    #[builder(default)]
    pub rewards_min_size: Option<f64>,

    // Date filters
    /// Minimum event start date (ISO 8601 format)
    #[builder(default, setter(into))]
    pub start_date_min: Option<&'a str>,
    /// Maximum event start date (ISO 8601 format)
    #[builder(default, setter(into))]
    pub start_date_max: Option<&'a str>,
    /// Minimum event end date (ISO 8601 format)
    #[builder(default, setter(into))]
    pub end_date_min: Option<&'a str>,
    /// Maximum event end date (ISO 8601 format)
    #[builder(default, setter(into))]
    pub end_date_max: Option<&'a str>,

    // Boolean filters
    /// Filter by closed status
    #[builder(default)]
    pub closed: Option<bool>,
    /// Include related tags
    #[builder(default)]
    pub related_tags: Option<bool>,
    /// Filter CYOM (Create Your Own Market) markets
    #[builder(default)]
    pub cyom: Option<bool>,
    /// Include tag information in response
    #[builder(default)]
    pub include_tag: Option<bool>,

    // Category/Type filters
    /// Filter by tag ID
    #[builder(default)]
    pub tag_id: Option<i32>,
    /// Filter by UMA resolution status
    #[builder(default, setter(into))]
    pub uma_resolution_status: Option<&'a str>,
    /// Filter by game ID
    #[builder(default, setter(into))]
    pub game_id: Option<&'a str>,
    /// Filter by sports market types
    #[builder(default)]
    pub sports_market_types: Vec<SportsMarketType>,
}

impl<'a> MarketsRequest<'a> {
    /// Executes the markets request.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Vec<PolyResponseMarket>)` with the markets on success, or an error on failure.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * The HTTP request fails
    /// * The API returns an error response
    /// * The response cannot be deserialized
    pub async fn execute(&self) -> Result<MarketsResponse> {
        let client = get_http_client(None);

        let mut web_service_request = super::webservice_request::WebserviceRequest {
            api: GAMMA_API.to_string(),
            url: GET_MARKETS.to_string(),
            method: Method::GET,
            with_pagination: true,
            args: Vec::new(),
            body: None,
        };

        // Pagination
        if self.limit != 100 {
            web_service_request.add_arg("limit".to_string(), self.limit.to_string());
        }
        if self.offset != 0 {
            web_service_request.add_arg("offset".to_string(), self.offset.to_string());
        }

        // Sorting
        if let Some(order) = self.order {
            web_service_request.add_arg("order".to_string(), order.to_string());
        }
        if self.ascending {
            web_service_request.add_arg("ascending".to_string(), "true".to_string());
        }

        // ID Filters - add each value separately
        for id in &self.id {
            web_service_request.add_arg("id".to_string(), id.to_string());
        }
        for slug in &self.slug {
            web_service_request.add_arg("slug".to_string(), slug.to_string());
        }
        for token_id in &self.clob_token_ids {
            web_service_request.add_arg("clob_token_ids".to_string(), token_id.to_string());
        }
        for condition_id in &self.condition_ids {
            web_service_request.add_arg("condition_ids".to_string(), condition_id.to_string());
        }
        for address in &self.market_maker_address {
            web_service_request.add_arg(
                "market_maker_address".to_string(),
                address.to_string(),
            );
        }
        for question_id in &self.question_ids {
            web_service_request.add_arg("question_ids".to_string(), question_id.to_string());
        }

        // Numeric filters
        if let Some(min) = self.liquidity_num_min {
            web_service_request.add_arg("liquidity_num_min".to_string(), min.to_string());
        }
        if let Some(max) = self.liquidity_num_max {
            web_service_request.add_arg("liquidity_num_max".to_string(), max.to_string());
        }
        if let Some(min) = self.volume_num_min {
            web_service_request.add_arg("volume_num_min".to_string(), min.to_string());
        }
        if let Some(max) = self.volume_num_max {
            web_service_request.add_arg("volume_num_max".to_string(), max.to_string());
        }
        if let Some(size) = self.rewards_min_size {
            web_service_request.add_arg("rewards_min_size".to_string(), size.to_string());
        }

        // Date filters
        if let Some(date) = self.start_date_min {
            web_service_request.add_arg("start_date_min".to_string(), date.to_string());
        }
        if let Some(date) = self.start_date_max {
            web_service_request.add_arg("start_date_max".to_string(), date.to_string());
        }
        if let Some(date) = self.end_date_min {
            web_service_request.add_arg("end_date_min".to_string(), date.to_string());
        }
        if let Some(date) = self.end_date_max {
            web_service_request.add_arg("end_date_max".to_string(), date.to_string());
        }

        // Boolean filters
        if let Some(closed) = self.closed {
            web_service_request.add_arg("closed".to_string(), closed.to_string());
        }
        if let Some(related) = self.related_tags {
            web_service_request.add_arg("related_tags".to_string(), related.to_string());
        }
        if let Some(cyom) = self.cyom {
            web_service_request.add_arg("cyom".to_string(), cyom.to_string());
        }
        if let Some(include) = self.include_tag {
            web_service_request.add_arg("include_tag".to_string(), include.to_string());
        }

        // Category/Type filters
        if let Some(tag_id) = self.tag_id {
            web_service_request.add_arg("tag_id".to_string(), tag_id.to_string());
        }
        if let Some(status) = self.uma_resolution_status {
            web_service_request.add_arg(
                "uma_resolution_status".to_string(),
                status.to_string(),
            );
        }
        if let Some(game_id) = self.game_id {
            web_service_request.add_arg("game_id".to_string(), game_id.to_string());
        }
        for market_type in &self.sports_market_types {
            web_service_request.add_arg(
                "sports_market_types".to_string(),
                market_type.as_str().to_string(),
            );
        }

        let callable_url = web_service_request.get_callable_url(self.offset);
        log::debug!("Markets request URL: {}", callable_url);

        let (_, result) = super::webservice_request::WebserviceRequest::fetch_batch::<
            MarketsResponse,
        >(&client, &web_service_request, self.offset)
        .await;

        result.ok_or_else(|| crate::api::error::ApiError::NotFound {
            url: callable_url,
            resource: "markets".to_string(),
        }.into())
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Fetches a single market by slug.
///
/// # Arguments
///
/// * `slug` - The market slug identifier
///
/// # Returns
///
/// Returns the market data if found, or an error if not.
pub async fn fetch_market_by_slug(slug: &str) -> Result<PolyResponseMarket> {
    MarketBySlugRequest::builder()
        .slug(slug)
        .build()
        .execute()
        .await
}

/// Fetches markets by condition IDs and returns them as a HashMap.
///
/// # Arguments
///
/// * `condition_ids` - Slice of condition IDs to fetch markets for
///
/// # Returns
///
/// Returns a HashMap mapping condition IDs to PolyResponseMarket data.
pub async fn map_multiple_market_by_condition_ids_ws(
    condition_ids: &[String],
) -> Result<HashMap<String, PolyResponseMarket>> {
    let mut markets_map: HashMap<String, PolyResponseMarket> = HashMap::new();

    let condition_id_refs: Vec<&str> = condition_ids.iter().map(|s| s.as_str()).collect();

    let markets = MarketsRequest::builder()
        .condition_ids(condition_id_refs)
        .build()
        .execute()
        .await?;

    for m in markets.into_iter() {
        if let Some(condition_id) = m.condition_id.clone() {
            markets_map.insert(condition_id, m);
        }
    }

    Ok(markets_map)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_market_sort_by_as_str() {
        assert_eq!(MarketSortBy::VOLUME.as_str(), "volume");
        assert_eq!(MarketSortBy::LIQUIDITY.as_str(), "liquidity");
        assert_eq!(MarketSortBy::CREATED_AT.as_str(), "createdAt");
        assert_eq!(MarketSortBy::START_DATE.as_str(), "startDate");
        assert_eq!(MarketSortBy::END_DATE.as_str(), "endDate");
        assert_eq!(MarketSortBy::VOLUME_24HR.as_str(), "volume24hr");
        assert_eq!(MarketSortBy::LAST_TRADE_PRICE.as_str(), "lastTradePrice");
    }

    #[test]
    fn test_uma_resolution_status_as_str() {
        assert_eq!(UmaResolutionStatus::RESOLVED.as_str(), "resolved");
        assert_eq!(UmaResolutionStatus::PENDING.as_str(), "pending");
        assert_eq!(UmaResolutionStatus::DISPUTED.as_str(), "disputed");
    }

    #[test]
    fn test_sports_market_type_as_str() {
        assert_eq!(SportsMarketType::MONEYLINE.as_str(), "moneyline");
        assert_eq!(SportsMarketType::SPREAD.as_str(), "spread");
        assert_eq!(SportsMarketType::OVER_UNDER.as_str(), "over_under");
    }

    #[test]
    fn test_market_by_slug_request_builder() {
        let request = MarketBySlugRequest::builder()
            .slug("bitcoin-above-100k")
            .build();

        assert_eq!(request.slug, "bitcoin-above-100k");
    }

    #[test]
    fn test_markets_request_builder_defaults() {
        let request = MarketsRequest::builder().build();

        assert_eq!(request.limit, 100);
        assert_eq!(request.offset, 0);
        assert_eq!(request.ascending, false);
        assert!(request.id.is_empty());
        assert!(request.slug.is_empty());
        assert!(request.clob_token_ids.is_empty());
        assert!(request.condition_ids.is_empty());
        assert!(request.market_maker_address.is_empty());
        assert!(request.question_ids.is_empty());
        assert!(request.liquidity_num_min.is_none());
        assert!(request.liquidity_num_max.is_none());
        assert!(request.volume_num_min.is_none());
        assert!(request.volume_num_max.is_none());
        assert!(request.rewards_min_size.is_none());
        assert!(request.start_date_min.is_none());
        assert!(request.start_date_max.is_none());
        assert!(request.end_date_min.is_none());
        assert!(request.end_date_max.is_none());
        assert!(request.closed.is_none());
        assert!(request.related_tags.is_none());
        assert!(request.cyom.is_none());
        assert!(request.include_tag.is_none());
        assert!(request.tag_id.is_none());
        assert!(request.uma_resolution_status.is_none());
        assert!(request.game_id.is_none());
        assert!(request.sports_market_types.is_empty());
    }

    #[test]
    fn test_markets_request_with_filters() {
        let request = MarketsRequest::builder()
            .limit(50)
            .offset(10)
            .closed(Some(false))
            .volume_num_min(Some(1000.0))
            .volume_num_max(Some(5000.0))
            .condition_ids(vec!["0x123", "0x456"])
            .tag_id(Some(42))
            .build();

        assert_eq!(request.limit, 50);
        assert_eq!(request.offset, 10);
        assert_eq!(request.closed, Some(false));
        assert_eq!(request.volume_num_min, Some(1000.0));
        assert_eq!(request.volume_num_max, Some(5000.0));
        assert_eq!(request.condition_ids.len(), 2);
        assert_eq!(request.condition_ids[0], "0x123");
        assert_eq!(request.condition_ids[1], "0x456");
        assert_eq!(request.tag_id, Some(42));
    }

    #[test]
    fn test_markets_request_with_multiple_filters() {
        let request = MarketsRequest::builder()
            .closed(Some(false))
            .related_tags(Some(true))
            .include_tag(Some(true))
            .order("volume,liquidity")
            .ascending(true)
            .slug(vec!["market-1", "market-2"])
            .build();

        assert_eq!(request.closed, Some(false));
        assert_eq!(request.related_tags, Some(true));
        assert_eq!(request.include_tag, Some(true));
        assert_eq!(request.order, Some("volume,liquidity"));
        assert_eq!(request.ascending, true);
        assert_eq!(request.slug.len(), 2);
    }
}
