//! Order book data models for the Polymarket CLOB API.
//!
//! This module contains types for representing order book data returned by the `/books` endpoint.

use crate::models::ApiResponse;
use crate::utils::deserialize_string_to_option_f32;
use serde::{Deserialize, Serialize};

/// A single price level in the order book.
///
/// Represents a bid or ask level with price and cumulative size.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookLevel {
    /// Price at this level
    #[serde(default, deserialize_with = "deserialize_string_to_option_f32")]
    pub price: Option<f32>,
    /// Cumulative size at this price level
    #[serde(default, deserialize_with = "deserialize_string_to_option_f32")]
    pub size: Option<f32>,
}

/// Order book summary for a single token.
///
/// Contains bid and ask levels along with market metadata.
///
/// # Example
///
/// ```rust,no_run
/// use poly_clob_rs::OrderBook;
///
/// // OrderBook is returned from the /books endpoint
/// // let books: Vec<OrderBook> = OrderBooksRequest::builder()
/// //     .token_ids(vec!["token_id"])
/// //     .build()
/// //     .execute()
/// //     .await?;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBook {
    /// Market identifier (condition ID hash)
    pub market: String,
    /// Asset/token identifier
    pub asset_id: String,
    /// Timestamp of this order book snapshot
    #[serde(default)]
    pub timestamp: Option<String>,
    /// Hash of the order book state
    #[serde(default)]
    pub hash: Option<String>,
    /// Bid levels (buy orders). Ordering is source-dependent: the WS cache sorts
    /// descending (best first), the REST `/books` endpoint returns ascending (best
    /// last). Use `best_bid()`, which selects by price, rather than assuming order.
    pub bids: Vec<OrderBookLevel>,
    /// Ask levels (sell orders). Ordering is source-dependent: the WS cache sorts
    /// ascending (best first), the REST `/books` endpoint returns descending (best
    /// last). Use `best_ask()`, which selects by price, rather than assuming order.
    pub asks: Vec<OrderBookLevel>,
    /// Minimum order size for this market
    #[serde(default)]
    pub min_order_size: Option<String>,
    /// Minimum price increment (tick size)
    #[serde(default)]
    pub tick_size: Option<String>,
    /// Whether this is a negative risk market
    #[serde(default)]
    pub neg_risk: Option<bool>,
    /// Authoritative best bid from WS `price_change` message (server-reported, not level-derived).
    /// Set by the WS cache on every incremental update; None for REST/replay snapshots.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ws_best_bid: Option<f32>,
    /// Authoritative best ask from WS `price_change` message (server-reported, not level-derived).
    /// Prevents stale ghost ask levels from being used as entry prices.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ws_best_ask: Option<f32>,
}

impl OrderBook {
    /// Returns the best (highest price) bid level.
    ///
    /// Selected by price, not position, so it is correct regardless of how the
    /// source ordered the levels. This matters because the two ingestion paths
    /// sort oppositely: the WS cache sorts bids descending (best first), while the
    /// REST `/books` endpoint returns bids ascending (best last). Relying on
    /// `.first()` would therefore return the worst bid for REST/replay books.
    #[must_use]
    pub fn best_bid(&self) -> Option<&OrderBookLevel> {
        self.bids
            .iter()
            .filter(|l| l.price.is_some())
            .max_by(|a, b| {
                a.price
                    .partial_cmp(&b.price)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    /// Returns the best (lowest price) ask level.
    ///
    /// Selected by price, not position, so it is correct regardless of how the
    /// source ordered the levels (WS sorts asks ascending = best first; REST
    /// returns asks descending = best last). Zero/negative-priced ghost levels
    /// are skipped so they can never be picked as the best ask.
    #[must_use]
    pub fn best_ask(&self) -> Option<&OrderBookLevel> {
        self.asks
            .iter()
            .filter(|l| l.price.map(|p| p > 0.0).unwrap_or(false))
            .min_by(|a, b| {
                a.price
                    .partial_cmp(&b.price)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    /// Returns the total bid depth (sum of all bid sizes).
    ///
    /// Sums each bid's size field. None values are treated as 0.
    #[must_use]
    pub fn get_bid_depth(&self) -> f32 {
        self.bids.iter().filter_map(|level| level.size).sum()
    }

    /// Returns the total ask depth (sum of all ask sizes).
    ///
    /// Sums each ask's size field. None values are treated as 0.
    #[must_use]
    pub fn get_ask_depth(&self) -> f32 {
        self.asks.iter().filter_map(|level| level.size).sum()
    }

    /// Returns the best bid price (highest buy price).
    ///
    /// Prefers the server-authoritative value from WS `price_change` messages
    /// (`ws_best_bid`) when available. Falls back to the level-derived value
    /// for REST/replay snapshots that have no WS-authoritative data.
    #[must_use]
    pub fn best_bid_price(&self) -> f64 {
        if let Some(v) = self.ws_best_bid {
            return v as f64;
        }
        self.best_bid()
            .and_then(|l| l.price)
            .unwrap_or(0.0) as f64
    }

    /// Returns the best ask price (lowest sell price).
    ///
    /// Prefers the server-authoritative value from WS `price_change` messages
    /// (`ws_best_ask`) when available. This prevents stale ghost ask levels
    /// (consumed orders whose `size=0` removal message was delayed/dropped)
    /// from being used as entry prices.
    ///
    /// Falls back to the level-derived value for REST/replay snapshots.
    #[must_use]
    pub fn best_ask_price(&self) -> f64 {
        if let Some(v) = self.ws_best_ask {
            return v as f64;
        }
        self.best_ask()
            .and_then(|l| l.price)
            .unwrap_or(0.0) as f64
    }

    /// Returns the bid depth (sum of sizes) for prices >= the given price.
    /// This is useful for estimating how much liquidity is available
    /// to fill a sell order at or above a certain price.
    #[must_use]
    pub fn bid_depth_to_price(&self, price: f64) -> f64 {
        let price_f32 = price as f32;
        self.bids
            .iter()
            .filter(|level| level.price.unwrap_or(0.0) >= price_f32)
            .filter_map(|level| level.size)
            .sum::<f32>() as f64
    }

    /// Returns the ask depth (sum of sizes) for prices <= the given price.
    /// This is useful for estimating how much liquidity is available
    /// to fill a buy order at or below a certain price.
    #[must_use]
    pub fn ask_depth_to_price(&self, price: f64) -> f64 {
        let price_f32 = price as f32;
        self.asks
            .iter()
            .filter(|level| level.price.unwrap_or(f32::MAX) <= price_f32)
            .filter_map(|level| level.size)
            .sum::<f32>() as f64
    }

    /// Returns bid depth as f64 (convenience wrapper around `get_bid_depth`)
    #[must_use]
    pub fn bid_depth(&self) -> f64 {
        self.get_bid_depth() as f64
    }

    /// Returns ask depth as f64 (convenience wrapper around `get_ask_depth`)
    #[must_use]
    pub fn ask_depth(&self) -> f64 {
        self.get_ask_depth() as f64
    }
}

/// Response type for the order books endpoint.
///
/// This is a type alias for a vector of [`OrderBook`] items.
pub type OrderBooksResponse = Vec<OrderBook>;

impl ApiResponse for OrderBooksResponse {
    fn nb_results(&self) -> usize {
        self.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orderbook_deserialization() {
        let json = r#"{
            "market": "0x1b6f76e5b8587ee896c35847e12d11e75290a8c3934c5952e8a9d6e4c6f03cfa",
            "asset_id": "1234567890",
            "timestamp": "2023-10-01T12:00:00Z",
            "bids": [{"price": "0.50", "size": "100.5"}],
            "asks": [{"price": "0.52", "size": "200.0"}],
            "tick_size": "0.01",
            "neg_risk": false
        }"#;

        let orderbook: OrderBook = serde_json::from_str(json).unwrap();
        assert_eq!(orderbook.asset_id, "1234567890");
        assert_eq!(orderbook.bids.len(), 1);
        assert_eq!(orderbook.bids[0].price, Some(0.50));
        assert_eq!(orderbook.bids[0].size, Some(100.5));
        assert_eq!(orderbook.asks.len(), 1);
        assert_eq!(orderbook.asks[0].price, Some(0.52));
        assert_eq!(orderbook.asks[0].size, Some(200.0));
        assert_eq!(orderbook.tick_size, Some("0.01".to_string()));
        assert_eq!(orderbook.neg_risk, Some(false));
    }

    #[test]
    fn test_orderbook_response_api_response() {
        let response: OrderBooksResponse = vec![
            OrderBook {
                market: "market1".to_string(),
                asset_id: "asset1".to_string(),
                timestamp: None,
                hash: None,
                bids: vec![],
                asks: vec![],
                min_order_size: None,
                tick_size: None,
                neg_risk: None,
                ws_best_bid: None,
                ws_best_ask: None,
            },
            OrderBook {
                market: "market2".to_string(),
                asset_id: "asset2".to_string(),
                timestamp: None,
                hash: None,
                bids: vec![],
                asks: vec![],
                min_order_size: None,
                tick_size: None,
                neg_risk: None,
                ws_best_bid: None,
                ws_best_ask: None,
            },
        ];

        assert_eq!(response.nb_results(), 2);
    }

    #[test]
    fn test_orderbook_depth_methods() {
        let orderbook = OrderBook {
            market: "market1".to_string(),
            asset_id: "asset1".to_string(),
            timestamp: None,
            hash: None,
            bids: vec![
                OrderBookLevel {
                    price: Some(0.50),
                    size: Some(100.5),
                },
                OrderBookLevel {
                    price: Some(0.49),
                    size: Some(200.25),
                },
                OrderBookLevel {
                    price: Some(0.48),
                    size: Some(50.0),
                },
            ],
            asks: vec![
                OrderBookLevel {
                    price: Some(0.51),
                    size: Some(75.0),
                },
                OrderBookLevel {
                    price: Some(0.52),
                    size: Some(125.5),
                },
            ],
            min_order_size: None,
            tick_size: None,
            neg_risk: None,
            ws_best_bid: None,
            ws_best_ask: None,
        };

        // Test bid depth: 100.5 + 200.25 + 50.0 = 350.75
        assert!((orderbook.get_bid_depth() - 350.75).abs() < 0.001);

        // Test ask depth: 75.0 + 125.5 = 200.5
        assert!((orderbook.get_ask_depth() - 200.5).abs() < 0.001);

        // Test best bid (highest price, regardless of level ordering)
        let best_bid = orderbook.best_bid().unwrap();
        assert_eq!(best_bid.price, Some(0.50));

        // Test best ask (lowest price, regardless of level ordering)
        let best_ask = orderbook.best_ask().unwrap();
        assert_eq!(best_ask.price, Some(0.51));
    }

    /// Regression test for the wrong-side bug: the WS cache sorts bids descending /
    /// asks ascending (best first), while the REST `/books` endpoint returns bids
    /// ascending / asks descending (best last). `best_bid`/`best_ask` must select by
    /// price and return the same top-of-book for both orderings.
    #[test]
    fn test_best_bid_ask_independent_of_level_ordering() {
        let lvl = |p: f32| OrderBookLevel { price: Some(p), size: Some(100.0) };

        // WS ordering: bids descending (best first), asks ascending (best first).
        let ws = OrderBook {
            market: "m".into(),
            asset_id: "a".into(),
            timestamp: None,
            hash: None,
            bids: vec![lvl(0.40), lvl(0.30), lvl(0.10)],
            asks: vec![lvl(0.41), lvl(0.50), lvl(0.90)],
            min_order_size: None,
            tick_size: None,
            neg_risk: None,
            ws_best_bid: None,
            ws_best_ask: None,
        };

        // REST ordering: bids ascending (best last), asks descending (best last).
        let rest = OrderBook {
            bids: vec![lvl(0.10), lvl(0.30), lvl(0.40)],
            asks: vec![lvl(0.90), lvl(0.50), lvl(0.41)],
            ..ws.clone()
        };

        // Both must agree on the true top-of-book.
        assert_eq!(ws.best_bid().unwrap().price, Some(0.40));
        assert_eq!(ws.best_ask().unwrap().price, Some(0.41));
        assert_eq!(rest.best_bid().unwrap().price, Some(0.40));
        assert_eq!(rest.best_ask().unwrap().price, Some(0.41));

        // And the derived mid must not collapse to 0.50 on the REST ordering.
        assert!((rest.best_bid_price() - 0.40).abs() < 1e-6);
        assert!((rest.best_ask_price() - 0.41).abs() < 1e-6);
    }

    /// A zero-priced ghost ask must never be selected as the best ask.
    #[test]
    fn test_best_ask_skips_zero_price_ghost() {
        let lvl = |p: f32| OrderBookLevel { price: Some(p), size: Some(10.0) };
        let book = OrderBook {
            market: "m".into(),
            asset_id: "a".into(),
            timestamp: None,
            hash: None,
            bids: vec![lvl(0.30)],
            asks: vec![lvl(0.0), lvl(0.55), lvl(0.60)],
            min_order_size: None,
            tick_size: None,
            neg_risk: None,
            ws_best_bid: None,
            ws_best_ask: None,
        };
        assert_eq!(book.best_ask().unwrap().price, Some(0.55));
    }

    #[test]
    fn test_orderbook_depth_with_none_values() {
        let orderbook = OrderBook {
            market: "market1".to_string(),
            asset_id: "asset1".to_string(),
            timestamp: None,
            hash: None,
            bids: vec![
                OrderBookLevel {
                    price: Some(0.50),
                    size: Some(100.0),
                },
                OrderBookLevel {
                    price: Some(0.49),
                    size: None, // None value should be skipped
                },
                OrderBookLevel {
                    price: None,
                    size: Some(50.0),
                },
            ],
            asks: vec![],
            min_order_size: None,
            tick_size: None,
            neg_risk: None,
            ws_best_bid: None,
            ws_best_ask: None,
        };

        // Only 100.0 + 50.0 = 150.0 (None is skipped)
        assert!((orderbook.get_bid_depth() - 150.0).abs() < 0.001);
    }

    #[test]
    fn test_orderbook_depth_empty() {
        let orderbook = OrderBook {
            market: "market1".to_string(),
            asset_id: "asset1".to_string(),
            timestamp: None,
            hash: None,
            bids: vec![],
            asks: vec![],
            min_order_size: None,
            tick_size: None,
            neg_risk: None,
            ws_best_bid: None,
            ws_best_ask: None,
        };

        assert_eq!(orderbook.get_bid_depth(), 0.0);
        assert_eq!(orderbook.get_ask_depth(), 0.0);
        assert!(orderbook.best_bid().is_none());
        assert!(orderbook.best_ask().is_none());
    }

    #[test]
    fn test_orderbook_price_methods() {
        let orderbook = OrderBook {
            market: "market1".to_string(),
            asset_id: "asset1".to_string(),
            timestamp: None,
            hash: None,
            bids: vec![
                OrderBookLevel { price: Some(0.50), size: Some(100.0) },
                OrderBookLevel { price: Some(0.49), size: Some(200.0) },
                OrderBookLevel { price: Some(0.48), size: Some(50.0) },
            ],
            asks: vec![
                OrderBookLevel { price: Some(0.51), size: Some(75.0) },
                OrderBookLevel { price: Some(0.52), size: Some(125.0) },
            ],
            min_order_size: None,
            tick_size: None,
            neg_risk: None,
            ws_best_bid: None,
            ws_best_ask: None,
        };

        // Bids sorted descending: first() is highest price (0.50) = true best bid
        // Asks sorted ascending: first() is lowest price (0.51) = true best ask
        assert!((orderbook.best_bid_price() - 0.50).abs() < 0.001);
        assert!((orderbook.best_ask_price() - 0.51).abs() < 0.001);

        // bid_depth_to_price: sum of sizes for bids >= 0.49
        // 0.50 (100) + 0.49 (200) = 300
        assert!((orderbook.bid_depth_to_price(0.49) - 300.0).abs() < 0.001);

        // ask_depth_to_price: sum of sizes for asks <= 0.51
        // 0.51 (75) = 75
        assert!((orderbook.ask_depth_to_price(0.51) - 75.0).abs() < 0.001);

        // Test wrapper methods
        assert!((orderbook.bid_depth() - 350.0).abs() < 0.001);
        assert!((orderbook.ask_depth() - 200.0).abs() < 0.001);
    }

    #[test]
    fn test_orderbook_empty_prices() {
        let orderbook = OrderBook {
            market: "m".to_string(),
            asset_id: "a".to_string(),
            timestamp: None,
            hash: None,
            bids: vec![],
            asks: vec![],
            min_order_size: None,
            tick_size: None,
            neg_risk: None,
            ws_best_bid: None,
            ws_best_ask: None,
        };

        assert_eq!(orderbook.best_bid_price(), 0.0);
        assert_eq!(orderbook.best_ask_price(), 0.0);
        assert_eq!(orderbook.bid_depth_to_price(0.5), 0.0);
        assert_eq!(orderbook.ask_depth_to_price(0.5), 0.0);
        assert_eq!(orderbook.bid_depth(), 0.0);
        assert_eq!(orderbook.ask_depth(), 0.0);
    }
}
