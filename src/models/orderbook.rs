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
    /// Bid levels (buy orders), sorted by price descending
    pub bids: Vec<OrderBookLevel>,
    /// Ask levels (sell orders), sorted by price ascending
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
    /// Authoritative best bid from WS price_change message (server-reported, not level-derived).
    /// Set by the WS cache on every incremental update; None for REST/replay snapshots.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ws_best_bid: Option<f32>,
    /// Authoritative best ask from WS price_change message (server-reported, not level-derived).
    /// Prevents stale ghost ask levels from being used as entry prices.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ws_best_ask: Option<f32>,
}

impl OrderBook {
    /// Returns the best (highest price) bid level.
    /// Bids are sorted descending (REST convention), so the first element is the highest bid.
    pub fn best_bid(&self) -> Option<&OrderBookLevel> {
        self.bids.first()
    }

    /// Returns the best (lowest price) ask level.
    /// Asks are sorted ascending (REST convention), so the first element is the lowest ask.
    pub fn best_ask(&self) -> Option<&OrderBookLevel> {
        self.asks.first()
    }

    /// Returns the total bid depth (sum of all bid sizes).
    ///
    /// Sums each bid's size field. None values are treated as 0.
    pub fn get_bid_depth(&self) -> f32 {
        self.bids.iter().filter_map(|level| level.size).sum()
    }

    /// Returns the total ask depth (sum of all ask sizes).
    ///
    /// Sums each ask's size field. None values are treated as 0.
    pub fn get_ask_depth(&self) -> f32 {
        self.asks.iter().filter_map(|level| level.size).sum()
    }

    /// Returns the best bid price (highest buy price).
    ///
    /// Prefers the server-authoritative value from WS `price_change` messages
    /// (`ws_best_bid`) when available. Falls back to the level-derived value
    /// for REST/replay snapshots that have no WS-authoritative data.
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
    pub fn ask_depth_to_price(&self, price: f64) -> f64 {
        let price_f32 = price as f32;
        self.asks
            .iter()
            .filter(|level| level.price.unwrap_or(f32::MAX) <= price_f32)
            .filter_map(|level| level.size)
            .sum::<f32>() as f64
    }

    /// Returns bid depth as f64 (convenience wrapper around get_bid_depth)
    pub fn bid_depth(&self) -> f64 {
        self.get_bid_depth() as f64
    }

    /// Returns ask depth as f64 (convenience wrapper around get_ask_depth)
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

        // Test best bid (first in the descending-sorted list = highest price)
        let best_bid = orderbook.best_bid().unwrap();
        assert_eq!(best_bid.price, Some(0.50));

        // Test best ask (first in the ascending-sorted list = lowest price)
        let best_ask = orderbook.best_ask().unwrap();
        assert_eq!(best_ask.price, Some(0.51));
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
