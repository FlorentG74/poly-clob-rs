//! Market request builders for the Polymarket Gamma API.
//!
//! This module provides builders for fetching market data from the Polymarket Gamma API.
//! [`MarketsRequest`] uses the cursor-based `/markets/keyset` endpoint.
//!
//! # Examples
//!
//! ## List active markets with volume filter
//!
//! ```no_run
//! use poly_clob_rs::api::market_requests::MarketsRequest;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut cursor: Option<String> = None;
//! loop {
//!     let page = MarketsRequest::builder()
//!         .closed(Some(false))
//!         .volume_num_min(Some(1000.0))
//!         .limit(50)
//!         .cursor(cursor.clone())
//!         .build()
//!         .execute()
//!         .await?;
//!     // … process page.data …
//!     cursor = page.next_cursor;
//!     if cursor.is_none() { break; }
//! }
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
//! let page = MarketsRequest::builder()
//!     .condition_ids(vec!["0x123".to_string(), "0x456".to_string()])
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
use crate::PolyResponseMarket;
use crate::models::KeysetMarketsResponse;

use super::{GAMMA_API, GET_MARKETS, GET_MARKETS_KEYSET, WITH_SLUG};

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

        super::webservice_request::WebserviceRequest::fetch_one::<PolyResponseMarket>(
            client,
            &web_service_request,
        )
        .await
    }
}

// ============================================================================
// MarketsRequest - cursor-based pagination for /markets/keyset
// ============================================================================

/// Request builder for listing and filtering markets from the Polymarket Gamma API.
///
/// Uses the `/markets/keyset` cursor-based pagination endpoint.
/// Leave `cursor` as `None` for the first page; pass the `next_cursor` from the
/// previous [`KeysetMarketsResponse`] for subsequent pages.
///
/// # Example
///
/// ```no_run
/// use poly_clob_rs::api::market_requests::MarketsRequest;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let mut cursor: Option<String> = None;
/// loop {
///     let page = MarketsRequest::builder()
///         .closed(Some(false))
///         .cursor(cursor.clone())
///         .build()
///         .execute()
///         .await?;
///     // … process page.data …
///     cursor = page.next_cursor;
///     if cursor.is_none() { break; }
/// }
/// # Ok(())
/// # }
/// ```
#[derive(TypedBuilder)]
pub struct MarketsRequest {
    // Pagination
    /// Maximum number of markets per page
    #[builder(default = 100)]
    pub limit: i32,
    /// Cursor from the previous response. `None` fetches the first page.
    #[builder(default)]
    pub cursor: Option<String>,

    // Sorting
    /// Comma-separated list of fields to order by (e.g., "volume,liquidity")
    #[builder(default, setter(into))]
    pub order: Option<String>,
    /// Sort in ascending order (default: false/descending)
    #[builder(default = false)]
    pub ascending: bool,

    // ID Filters
    /// Filter by market IDs
    #[builder(default)]
    pub id: Vec<i64>,
    /// Filter by market slugs
    #[builder(default, setter(into))]
    pub slug: Vec<String>,
    /// Filter by CLOB token IDs
    #[builder(default, setter(into))]
    pub clob_token_ids: Vec<String>,
    /// Filter by condition IDs
    #[builder(default, setter(into))]
    pub condition_ids: Vec<String>,
    /// Filter by market maker addresses
    #[builder(default, setter(into))]
    pub market_maker_address: Vec<String>,
    /// Filter by question IDs
    #[builder(default, setter(into))]
    pub question_ids: Vec<String>,

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
    pub start_date_min: Option<String>,
    /// Maximum event start date (ISO 8601 format)
    #[builder(default, setter(into))]
    pub start_date_max: Option<String>,
    /// Minimum event end date (ISO 8601 format)
    #[builder(default, setter(into))]
    pub end_date_min: Option<String>,
    /// Maximum event end date (ISO 8601 format)
    #[builder(default, setter(into))]
    pub end_date_max: Option<String>,

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
    pub uma_resolution_status: Option<String>,
    /// Filter by game ID
    #[builder(default, setter(into))]
    pub game_id: Option<String>,
}

impl MarketsRequest {
    /// Executes a single page fetch against `/markets/keyset`.
    ///
    /// Returns a [`KeysetMarketsResponse`] whose `next_cursor` field indicates
    /// whether more pages exist. Pass it back via [`MarketsRequest::cursor`] to
    /// fetch the next page.
    pub async fn execute(&self) -> Result<KeysetMarketsResponse> {
        let client = get_http_client(None);

        let mut web_service_request = super::webservice_request::WebserviceRequest {
            api: GAMMA_API.to_string(),
            url: GET_MARKETS_KEYSET.to_string(),
            method: Method::GET,
            with_pagination: false,
            args: Vec::new(),
            body: None,
        };

        if self.limit != 100 {
            web_service_request.add_arg("limit".to_string(), self.limit.to_string());
        }

        // Sorting
        if let Some(order) = self.order.as_deref() {
            web_service_request.add_arg("order".to_string(), order.to_string());
        }
        if self.ascending {
            web_service_request.add_arg("ascending".to_string(), "true".to_string());
        }

        // ID Filters
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
        if let Some(date) = self.start_date_min.as_deref() {
            web_service_request.add_arg("start_date_min".to_string(), date.to_string());
        }
        if let Some(date) = self.start_date_max.as_deref() {
            web_service_request.add_arg("start_date_max".to_string(), date.to_string());
        }
        if let Some(date) = self.end_date_min.as_deref() {
            web_service_request.add_arg("end_date_min".to_string(), date.to_string());
        }
        if let Some(date) = self.end_date_max.as_deref() {
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
        if let Some(status) = self.uma_resolution_status.as_deref() {
            web_service_request.add_arg(
                "uma_resolution_status".to_string(),
                status.to_string(),
            );
        }
        if let Some(game_id) = self.game_id.as_deref() {
            web_service_request.add_arg("game_id".to_string(), game_id.to_string());
        }

        let page =
            super::webservice_request::WebserviceRequest::fetch_keyset::<KeysetMarketsResponse>(
                client,
                &web_service_request,
                self.cursor.as_deref(),
            )
            .await?;

        Ok(page)
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

    let markets = MarketsRequest::builder()
        .condition_ids(condition_ids.to_vec())
        .build()
        .execute()
        .await?;

    for m in markets.data.into_iter() {
        if let Some(condition_id) = m.condition_id.clone() {
            markets_map.insert(condition_id, m);
        }
    }

    // The gamma API excludes closed markets by default (closed=false). Retry any missing
    // condition_ids with closed=true to pick up resolved markets.
    let missing: Vec<String> = condition_ids
        .iter()
        .filter(|id| !markets_map.contains_key(*id))
        .map(|s| s.to_string())
        .collect();

    if !missing.is_empty() {
        let closed_markets = MarketsRequest::builder()
            .condition_ids(missing)
            .closed(Some(true))
            .build()
            .execute()
            .await?;

        for m in closed_markets.data.into_iter() {
            if let Some(condition_id) = m.condition_id.clone() {
                markets_map.insert(condition_id, m);
            }
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
    fn test_markets_request_builder_defaults() {
        let request = MarketsRequest::builder().build();

        assert_eq!(request.limit, 100);
        assert!(request.cursor.is_none());
        assert!(!request.ascending);
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
    }

    #[test]
    fn test_markets_request_with_cursor() {
        let request = MarketsRequest::builder()
            .cursor(Some("abc123".to_string()))
            .limit(50)
            .closed(Some(false))
            .build();

        assert_eq!(request.cursor, Some("abc123".to_string()));
        assert_eq!(request.limit, 50);
        assert_eq!(request.closed, Some(false));
    }

    #[test]
    fn test_markets_request_with_filters() {
        let request = MarketsRequest::builder()
            .limit(50)
            .closed(Some(false))
            .volume_num_min(Some(1000.0))
            .volume_num_max(Some(5000.0))
            .condition_ids(vec!["0x123".to_string(), "0x456".to_string()])
            .tag_id(Some(42))
            .build();

        assert_eq!(request.limit, 50);
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
            .order("volume,liquidity".to_string())
            .ascending(true)
            .slug(vec!["market-1".to_string(), "market-2".to_string()])
            .build();

        assert_eq!(request.closed, Some(false));
        assert_eq!(request.related_tags, Some(true));
        assert_eq!(request.include_tag, Some(true));
        assert_eq!(request.order.as_deref(), Some("volume,liquidity"));
        assert!(request.ascending);
        assert_eq!(request.slug.len(), 2);
    }

}
