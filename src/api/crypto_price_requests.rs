//! Request builders for the Polymarket crypto price API.
//!
//! This module provides request builders for fetching opening and closing prices
//! for crypto up/down events, used for strike setting and settlement resolution.

use reqwest::Method;
use typed_builder::TypedBuilder;

use crate::api::error::Result;
use crate::api::http_client::get_http_client;
use crate::models::CryptoPriceResponse;

use super::{WebserviceRequest, GET_CRYPTO_PRICE, POLYMARKET_API};

/// Request builder for fetching crypto prices at a specific event time.
///
/// This endpoint returns both opening and closing prices for a crypto symbol
/// at a given event start time. Used for:
/// - **Strike setting**: Get the `open_price` at event start
/// - **Settlement**: Get the `close_price` at event maturity (when `completed=true`)
///
/// Parameters are formatted as ISO 8601 datetime strings. `endDate` is computed
/// automatically from `event_start_time + variant_duration`.
///
/// # Example
///
/// ```rust,no_run
/// use poly_clob_rs::api::crypto_price_requests::CryptoPriceRequest;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let response = CryptoPriceRequest::builder()
///         .symbol("BTC")
///         .event_start_time(1776326400) // Unix seconds
///         .variant("hourly")
///         .build()
///         .execute()
///         .await?;
///
///     println!("Open: {:?}, Close: {:?}", response.open_price, response.close_price);
///     Ok(())
/// }
/// ```
#[derive(TypedBuilder)]
pub struct CryptoPriceRequest<'a> {
    /// Crypto symbol (e.g., "BTC", "ETH", "SOL", "XRP")
    #[builder(setter(into))]
    pub symbol: &'a str,

    /// Event start time as Unix timestamp in **seconds** (formatted as ISO 8601 internally)
    pub event_start_time: i64,

    /// Market duration variant: "fiveminute" (5m), "fifteen" (15m), "hourly", "fourhour", "daily".
    #[builder(setter(into))]
    pub variant: &'a str,
}

impl<'a> CryptoPriceRequest<'a> {
    /// Duration in seconds for each variant, used to compute `endDate`.
    pub fn variant_duration_secs(variant: &str) -> i64 {
        match variant {
            "fiveminute" => 5 * 60,
            "fifteen"    => 15 * 60,
            "hourly"     => 60 * 60,
            "fourhour"   => 4 * 60 * 60,
            "daily"      => 24 * 60 * 60,
            _            => 60 * 60, // fallback: 1 hour
        }
    }

    /// Build the query parameters for the request (extracted for testability).
    pub fn build_params(&self) -> Result<Vec<(String, String)>> {
        use chrono::{TimeZone, Utc};

        let start_dt = Utc.timestamp_opt(self.event_start_time, 0)
            .single()
            .ok_or_else(|| crate::ClobError::Validation(crate::ValidationError::InvalidParameter {
                parameter: "event_start_time".to_string(),
                reason: format!("invalid Unix timestamp: {}", self.event_start_time),
            }))?;
        let end_dt = Utc.timestamp_opt(
            self.event_start_time + Self::variant_duration_secs(self.variant), 0,
        )
        .single()
        .ok_or_else(|| crate::ClobError::Validation(crate::ValidationError::InvalidParameter {
            parameter: "event_start_time".to_string(),
            reason: "end timestamp overflow".to_string(),
        }))?;

        Ok(vec![
            ("symbol".to_string(), self.symbol.to_string()),
            ("eventStartTime".to_string(), start_dt.format("%Y-%m-%dT%H:%M:%SZ").to_string()),
            ("variant".to_string(), self.variant.to_string()),
            ("endDate".to_string(), end_dt.format("%Y-%m-%dT%H:%M:%SZ").to_string()),
        ])
    }

    /// Execute the request and return the crypto price response.
    pub async fn execute(&self) -> Result<CryptoPriceResponse> {
        let client = get_http_client(None);

        let web_service_request = WebserviceRequest {
            api: POLYMARKET_API.to_string(),
            url: GET_CRYPTO_PRICE.to_string(),
            method: Method::GET,
            with_pagination: false,
            args: self.build_params()?,
            body: None,
        };

        WebserviceRequest::fetch_one::<CryptoPriceResponse>(client, &web_service_request).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(symbol: &str, event_start_time: i64, variant: &str) -> std::collections::HashMap<String, String> {
        CryptoPriceRequest::builder()
            .symbol(symbol)
            .event_start_time(event_start_time)
            .variant(variant)
            .build()
            .build_params()
            .unwrap()
            .into_iter()
            .collect()
    }

    // 2026-04-16 08:00:00 UTC = 4am EDT, used as reference for hourly event tests
    const APRIL_16_8AM_UTC: i64 = 1776326400;
    // 2026-03-23 13:05:00 UTC, from the API example in the issue
    const MARCH_23_1305_UTC: i64 = 1774271100;

    #[test]
    fn test_hourly_params() {
        let p = params("BTC", APRIL_16_8AM_UTC, "hourly");
        assert_eq!(p["symbol"], "BTC");
        assert_eq!(p["variant"], "hourly");
        assert_eq!(p["eventStartTime"], "2026-04-16T08:00:00Z");
        assert_eq!(p["endDate"],        "2026-04-16T09:00:00Z");
    }

    #[test]
    fn test_fiveminute_params() {
        let p = params("BTC", MARCH_23_1305_UTC, "fiveminute");
        assert_eq!(p["symbol"], "BTC");
        assert_eq!(p["variant"], "fiveminute");
        assert_eq!(p["eventStartTime"], "2026-03-23T13:05:00Z");
        assert_eq!(p["endDate"],        "2026-03-23T13:10:00Z"); // +5 min — matches API example
    }

    #[test]
    fn test_fifteen_params() {
        let p = params("ETH", MARCH_23_1305_UTC, "fifteen");
        assert_eq!(p["symbol"], "ETH");
        assert_eq!(p["variant"], "fifteen");
        assert_eq!(p["eventStartTime"], "2026-03-23T13:05:00Z");
        assert_eq!(p["endDate"],        "2026-03-23T13:20:00Z"); // +15 min
    }

    #[test]
    fn test_fourhour_params() {
        let p = params("SOL", APRIL_16_8AM_UTC, "fourhour");
        assert_eq!(p["eventStartTime"], "2026-04-16T08:00:00Z");
        assert_eq!(p["endDate"],        "2026-04-16T12:00:00Z"); // +4h
    }

    #[test]
    fn test_daily_params() {
        let p = params("XRP", APRIL_16_8AM_UTC, "daily");
        assert_eq!(p["eventStartTime"], "2026-04-16T08:00:00Z");
        assert_eq!(p["endDate"],        "2026-04-17T08:00:00Z"); // +24h
    }

    #[test]
    fn test_variant_durations() {
        assert_eq!(CryptoPriceRequest::variant_duration_secs("fiveminute"),    300);
        assert_eq!(CryptoPriceRequest::variant_duration_secs("fifteen"),       900);
        assert_eq!(CryptoPriceRequest::variant_duration_secs("hourly"),       3600);
        assert_eq!(CryptoPriceRequest::variant_duration_secs("fourhour"),    14400);
        assert_eq!(CryptoPriceRequest::variant_duration_secs("daily"),       86400);
        assert_eq!(CryptoPriceRequest::variant_duration_secs("unknown"),      3600); // fallback
    }

    #[test]
    fn test_midnight_boundary() {
        // 2026-04-16 23:00:00 UTC — hourly event spanning midnight
        let p = params("BTC", 1776380400, "hourly");
        assert_eq!(p["eventStartTime"], "2026-04-16T23:00:00Z");
        assert_eq!(p["endDate"],        "2026-04-17T00:00:00Z");
    }
}

