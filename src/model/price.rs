use chrono::{NaiveDateTime, Utc};

use polars::prelude::*;

use crate::controller::{
    offset_current_time, timestamp_to_datetime, PolymarketPrice,
};

use super::Outcome;

#[derive(strum_macros::IntoStaticStr)]
pub enum PricingSource {
    POLYMARKET,
    BINANCE,
}

#[derive(Debug)]
pub struct PriceRequest {
    pub source: String,
    pub instrument: String,
    pub _retention: i64,
}

impl PriceRequest {
    pub fn new(source: String, instrument: String, retention: i64) -> Self {
        PriceRequest {
            source,
            instrument,
            _retention: retention,
        }
    }
}

pub struct Price {
    pub instrument: String,
    pub timestamp: NaiveDateTime,
    pub bid: f64,
    pub ask: f64,
    pub mid: f64,
}

impl Price {
    pub fn from_poly_response_price(
        instrument: &String,
        timestamp: NaiveDateTime,
        pp: &PolymarketPrice,
    ) -> Self {
        let instrument = format!("Polymarket:{}", instrument.clone());
        let bid: f64 = pp
            .buy
            .clone()
            .expect("Missing bid price")
            .parse()
            .expect("Missing bid price");
        let ask: f64 = pp
            .sell
            .clone()
            .expect("Missing ask price")
            .parse()
            .expect("Missing ask price");

        Price {
            instrument,
            timestamp: timestamp,
            bid,
            ask,
            mid: (bid + ask) / 2.0,
        }
    }

    pub fn from_outcome(outcome: Outcome, timestamp: NaiveDateTime) -> Self {
        let instrument = format!("Polymarket:{}", outcome.token_id);

        Price {
            instrument,
            timestamp: timestamp,
            bid: outcome.price,
            ask: outcome.price,
            mid: outcome.price,
        }
    }
}

#[cfg(test)]
mod price_tests {

}
