//! Order book request builders for the Polymarket CLOB API.
//!
//! This module provides builders for fetching order book data from the Polymarket CLOB API.
//! It supports fetching multiple order books in a single request.
//!
//! # Examples
//!
//! ## Fetch order books for multiple tokens
//!
//! ```no_run
//! use poly_clob_rs::api::orderbook_requests::OrderBooksRequest;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let books = OrderBooksRequest::builder()
//!     .token_ids(vec!["token_id_1".to_string(), "token_id_2".to_string()])
//!     .build()
//!     .execute()
//!     .await?;
//!
//! for book in books {
//!     println!("Token {}: {} bids, {} asks",
//!         book.asset_id,
//!         book.bids.len(),
//!         book.asks.len());
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Fetch order books with side filter
//!
//! ```no_run
//! use poly_clob_rs::api::orderbook_requests::OrderBooksRequest;
//! use poly_clob_rs::Side;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let books = OrderBooksRequest::builder()
//!     .token_ids_with_side(vec![("token_id_1".to_string(), Some(Side::Buy))])
//!     .build()
//!     .execute()
//!     .await?;
//! # Ok(())
//! # }
//! ```

use crate::api::error::{Result, SerializationError, ValidationError};
use reqwest::Method;
use serde::Serialize;
use typed_builder::TypedBuilder;

use crate::api::http_client::get_http_client;
use crate::models::Side;
use crate::OrderBooksResponse;

use super::{CLOB_API, GET_ORDER_BOOKS};

// ============================================================================
// Request Body Types
// ============================================================================

/// A single order book query item in the request body.
///
/// Used internally to build the JSON request body for the `/books` endpoint.
#[derive(Debug, Clone, Serialize)]
struct OrderBookQueryItem {
    /// The token ID to query
    token_id: String,
    /// Optional side filter (BUY or SELL)
    #[serde(skip_serializing_if = "Option::is_none")]
    side: Option<String>,
}

// ============================================================================
// OrderBooksRequest
// ============================================================================

/// Request builder for fetching multiple order books from the Polymarket CLOB API.
///
/// The `/books` endpoint accepts a POST request with a JSON array of token IDs
/// and returns order book summaries for each token.
///
/// # Required Fields
///
/// One of the following must be provided:
/// * `token_ids` - List of token IDs to query (without side filter)
/// * `token_ids_with_side` - List of (token_id, optional_side) tuples
///
/// # Limits
///
/// The API accepts a maximum of 500 items per request.
///
/// # Example
///
/// ```no_run
/// use poly_clob_rs::api::orderbook_requests::OrderBooksRequest;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let books = OrderBooksRequest::builder()
///     .token_ids(vec!["token_id_1".to_string(), "token_id_2".to_string()])
///     .build()
///     .execute()
///     .await?;
///
/// for book in &books {
///     if let Some(bid) = book.bids.first() {
///         println!("Best bid for {}: {:?} @ {:?}", book.asset_id, bid.size, bid.price);
///     }
/// }
/// # Ok(())
/// # }
/// ```
#[derive(TypedBuilder)]
pub struct OrderBooksRequest {
    /// Token IDs to query (without side filter).
    ///
    /// Use this for simple queries where you want both bid and ask data.
    #[builder(default, setter(into))]
    pub token_ids: Vec<String>,

    /// Token IDs with optional side filters.
    ///
    /// Use this when you need to specify a side filter for certain tokens.
    /// The side filter is optional per token.
    #[builder(default)]
    pub token_ids_with_side: Vec<(String, Option<Side>)>,
}

impl OrderBooksRequest {
    /// Executes the order books request.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Vec<OrderBook>)` with the order book data on success, or an error on failure.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * No token IDs are provided
    /// * The HTTP request fails
    /// * The API returns an error response
    /// * The response cannot be deserialized
    ///
    /// # Example
    ///
    /// ```no_run
    /// use poly_clob_rs::api::orderbook_requests::OrderBooksRequest;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let books = OrderBooksRequest::builder()
    ///     .token_ids(vec![String::from("token_id")])
    ///     .build()
    ///     .execute()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn execute(&self) -> Result<OrderBooksResponse> {
        use super::webservice_request::WebserviceRequest;

        // Build the query items from both sources
        let mut query_items: Vec<OrderBookQueryItem> = Vec::new();

        // Add token_ids (without side filter)
        for token_id in &self.token_ids {
            query_items.push(OrderBookQueryItem {
                token_id: String::from(token_id),
                side: None,
            });
        }

        // Add token_ids_with_side
        for (token_id, side) in &self.token_ids_with_side {
            query_items.push(OrderBookQueryItem {
                token_id: String::from(token_id),
                side: side.map(|s| s.to_string()),
            });
        }

        if query_items.is_empty() {
            return Err(ValidationError::InvalidParameter {
                parameter: String::from("token_ids"),
                reason: "OrderBooksRequest requires at least one token_id".to_string(),
            }.into());
        }

        if query_items.len() > 500 {
            return Err(ValidationError::InvalidParameter {
                parameter: String::from("token_ids"),
                reason: format!("OrderBooksRequest accepts a maximum of 500 items, got {}", query_items.len()),
            }.into());
        }

        let body = serde_json::to_string(&query_items)
            .map_err(|e| SerializationError::JsonSerialize {
                message: e.to_string(),
            })?;

        // Enhanced logging with token count
        log::debug!(
            "Fetching order books for {} tokens from {}{}",
            query_items.len(),
            CLOB_API,
            GET_ORDER_BOOKS
        );
        log::debug!(
            "Token IDs: {:?}",
            query_items
                .iter()
                .map(|q| &q.token_id[..q.token_id.len().min(10)])
                .collect::<Vec<_>>()
        );

        // Use WebserviceRequest for retry logic (3 attempts with 2s delay)
        let ws_request = WebserviceRequest {
            api: CLOB_API.to_string(),
            url: GET_ORDER_BOOKS.to_string(),
            method: Method::POST,
            with_pagination: false, // Order books don't use pagination
            args: vec![],           // No query parameters
            body: Some(body),
        };

        let client = get_http_client(None);

        // fetch_one returns Option<T>, handle the None case
        match WebserviceRequest::fetch_one::<OrderBooksResponse>(client, &ws_request).await {
            Ok(books) => {
                log::debug!("Successfully fetched {} order books", books.len());
                Ok(books)
            }
            Err(e) => {
                log::error!("Failed to fetch order books: {}", e);
                Err(e)
            }
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Fetches order books for a list of token IDs.
///
/// This is a convenience function that creates an `OrderBooksRequest` internally.
///
/// # Arguments
///
/// * `token_ids` - Slice of token IDs to fetch order books for
///
/// # Returns
///
/// Returns a vector of `OrderBook` items on success.
///
/// # Example
///
/// ```no_run
/// use poly_clob_rs::api::orderbook_requests::fetch_order_books;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let books = fetch_order_books(&["token_id_1", "token_id_2"]).await?;
/// # Ok(())
/// # }
/// ```
pub async fn fetch_order_books(token_ids: &[&str]) -> Result<OrderBooksResponse> {
    OrderBooksRequest::builder()
        .token_ids(token_ids.iter().map(|s| s.to_string()).collect::<Vec<String>>())
        .build()
        .execute()
        .await
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orderbooks_request_builder_defaults() {
        let request = OrderBooksRequest::builder().build();

        assert!(request.token_ids.is_empty());
        assert!(request.token_ids_with_side.is_empty());
    }

    #[test]
    fn test_orderbooks_request_with_token_ids() {
        let request = OrderBooksRequest::builder()
            .token_ids(vec!["token1".to_string(), "token2".to_string(), "token3".to_string()])
            .build();

        assert_eq!(request.token_ids.len(), 3);
        assert_eq!(request.token_ids[0], "token1");
        assert_eq!(request.token_ids[1], "token2");
        assert_eq!(request.token_ids[2], "token3");
    }

    #[test]
    fn test_orderbooks_request_with_side() {
        let request = OrderBooksRequest::builder()
            .token_ids_with_side(vec![
                ("token1".to_string(), Some(Side::Buy)),
                ("token2".to_string(), Some(Side::Sell)),
                ("token3".to_string(), None),
            ])
            .build();

        assert_eq!(request.token_ids_with_side.len(), 3);
        assert_eq!(request.token_ids_with_side[0], ("token1".to_string(), Some(Side::Buy)));
        assert_eq!(request.token_ids_with_side[1], ("token2".to_string(), Some(Side::Sell)));
        assert_eq!(request.token_ids_with_side[2], ("token3".to_string(), None));
    }

    #[test]
    fn test_orderbook_query_item_serialization() {
        let item = OrderBookQueryItem {
            token_id: "12345".to_string(),
            side: None,
        };
        let json = serde_json::to_string(&item).unwrap();
        assert_eq!(json, r#"{"token_id":"12345"}"#);

        let item_with_side = OrderBookQueryItem {
            token_id: "12345".to_string(),
            side: Some("BUY".to_string()),
        };
        let json = serde_json::to_string(&item_with_side).unwrap();
        assert_eq!(json, r#"{"token_id":"12345","side":"BUY"}"#);
    }

    #[test]
    fn test_orderbook_query_items_array_serialization() {
        let items = vec![
            OrderBookQueryItem {
                token_id: "token1".to_string(),
                side: None,
            },
            OrderBookQueryItem {
                token_id: "token2".to_string(),
                side: Some("SELL".to_string()),
            },
        ];
        let json = serde_json::to_string(&items).unwrap();
        assert_eq!(
            json,
            r#"[{"token_id":"token1"},{"token_id":"token2","side":"SELL"}]"#
        );
    }

    /// Integration test that fetches order books for the current event in the
    /// sol-up-or-down-15m event series.
    ///
    /// This test:
    /// 1. Fetches the event series by slug
    /// 2. Finds the current active event (first with end_date > now)
    /// 3. Fetches the full event data to get market token IDs
    /// 4. Queries order books for all token IDs
    #[tokio::test]
    #[ignore = "live gamma+CLOB APIs, depends on current wall-clock event — run with --ignored"]
    async fn test_fetch_orderbooks_for_sol_15m_current_event() {
        use crate::api::event_requests::{EventBySlugRequest, SeriesEventsRequest};
        use chrono::Utc;

        let event_series_slug = "sol-up-or-down-15m";

        // Step 1: Fetch active events for the series
        println!("Fetching event series: {}", event_series_slug);
        let events = SeriesEventsRequest::builder()
            .series_slug(event_series_slug)
            .build()
            .execute()
            .await
            .expect("Failed to fetch series events");

        assert!(!events.is_empty(), "No events returned for series");

        // Step 2: Find the current active event (first with end_date > now)
        let now = Utc::now();
        let current_event = events
            .iter()
            .find(|e| e.end_date > now)
            .expect("No active event found in series");

        println!(
            "Current event: {} (ends: {})",
            current_event.slug, current_event.end_date
        );

        // Step 3: Fetch the full event data to get market token IDs
        let full_event = EventBySlugRequest::builder()
            .slug(current_event.slug.as_str())
            .build()
            .execute()
            .await
            .expect("Failed to fetch full event data");

        println!("Event has {} markets", full_event.markets.len());

        // Extract token IDs from all markets
        let mut token_ids: Vec<String> = Vec::new();
        for market in &full_event.markets {
            token_ids.extend(market.clob_token_ids.iter().cloned());
        }

        assert!(!token_ids.is_empty(), "No token IDs found in event markets");
        println!("Found {} token IDs: {:?}", token_ids.len(), token_ids);

        // Step 4: Query order books for all token IDs
        let books = OrderBooksRequest::builder()
            .token_ids(token_ids.clone())
            .build()
            .execute()
            .await
            .expect("Failed to fetch order books");

        println!("Fetched {} order books", books.len());
        assert_eq!(
            books.len(),
            token_ids.len(),
            "Should receive one order book per token ID"
        );

        // Verify order book data
        for book in &books {
            println!(
                "Order book for token {}: {} bids, {} asks",
                book.asset_id,
                book.bids.len(),
                book.asks.len()
            );

            // Verify the asset_id matches one of our requested token IDs
            assert!(
                token_ids.contains(&book.asset_id),
                "Unexpected asset_id in response: {}",
                book.asset_id
            );

            // Log best bid/ask if available
            if let Some(best_bid) = book.bids.last() {
                println!("  Best bid: {:?} @ size {:?}", best_bid.price, best_bid.size);
            }
            if let Some(best_ask) = book.asks.last() {
                println!("  Best ask: {:?} @ size {:?}", best_ask.price, best_ask.size);
            }
        }

        println!("Test passed successfully!");
    }
}
