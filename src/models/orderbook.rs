//! Order book data models for the Polymarket CLOB API.
//!
//! This module contains types for representing order book data returned by the `/books` endpoint.

use serde::{Deserialize, Serialize};

use crate::models::ApiResponse;

/// A single price level in the order book.
///
/// Represents a bid or ask level with price and cumulative size.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookLevel {
    /// Price at this level
    pub price: String,
    /// Cumulative size at this price level
    pub size: String,
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
}

impl OrderBook {
    /// Returns the best (highest price) bid level.
    pub fn best_bid(&self) -> Option<&OrderBookLevel> {
        self.bids.last()
    }

    /// Returns the best (lowest price) ask level.
    pub fn best_ask(&self) -> Option<&OrderBookLevel> {
        self.asks.last()
    }

    /// Returns the total bid depth (sum of all bid sizes).
    ///
    /// Parses each bid's size field as f64 and sums them.
    /// Invalid size values are treated as 0.
    pub fn get_bid_depth(&self) -> f64 {
        self.bids
            .iter()
            .filter_map(|level| level.size.parse::<f64>().ok())
            .sum()
    }

    /// Returns the total ask depth (sum of all ask sizes).
    ///
    /// Parses each ask's size field as f64 and sums them.
    /// Invalid size values are treated as 0.
    pub fn get_ask_depth(&self) -> f64 {
        self.asks
            .iter()
            .filter_map(|level| level.size.parse::<f64>().ok())
            .sum()
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
        assert_eq!(orderbook.bids[0].price, "0.50");
        assert_eq!(orderbook.asks.len(), 1);
        assert_eq!(orderbook.asks[0].price, "0.52");
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
                    price: "0.50".to_string(),
                    size: "100.5".to_string(),
                },
                OrderBookLevel {
                    price: "0.49".to_string(),
                    size: "200.25".to_string(),
                },
                OrderBookLevel {
                    price: "0.48".to_string(),
                    size: "50.0".to_string(),
                },
            ],
            asks: vec![
                OrderBookLevel {
                    price: "0.51".to_string(),
                    size: "75.0".to_string(),
                },
                OrderBookLevel {
                    price: "0.52".to_string(),
                    size: "125.5".to_string(),
                },
            ],
            min_order_size: None,
            tick_size: None,
            neg_risk: None,
        };

        // Test bid depth: 100.5 + 200.25 + 50.0 = 350.75
        assert!((orderbook.get_bid_depth() - 350.75).abs() < 0.001);

        // Test ask depth: 75.0 + 125.5 = 200.5
        assert!((orderbook.get_ask_depth() - 200.5).abs() < 0.001);

        // Test best bid (last in the list, which is highest price after sorting)
        let best_bid = orderbook.best_bid().unwrap();
        assert_eq!(best_bid.price, "0.48");

        // Test best ask (last in the list, which is lowest price after sorting)
        let best_ask = orderbook.best_ask().unwrap();
        assert_eq!(best_ask.price, "0.52");
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
        };

        assert_eq!(orderbook.get_bid_depth(), 0.0);
        assert_eq!(orderbook.get_ask_depth(), 0.0);
        assert!(orderbook.best_bid().is_none());
        assert!(orderbook.best_ask().is_none());
    }
}
