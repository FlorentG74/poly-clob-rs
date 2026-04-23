use reqwest::Method;
use serde::Deserialize;
use typed_builder::TypedBuilder;

use crate::api::error::Result;
use crate::api::http_client::get_http_client;
use super::{CLOB_API, GET_PRICES_HISTORY};

// ── Response types ────────────────────────────────────────────────────────────

/// A single (timestamp, price) data point from the price history API.
#[derive(Debug, Clone, Deserialize)]
pub struct PricePoint {
    /// Unix timestamp (seconds).
    pub t: i64,
    /// Price (0–1 for binary markets).
    pub p: f64,
}

/// Response from `GET /prices-history`.
#[derive(Debug, Clone, Deserialize)]
pub struct PriceHistoryResponse {
    pub history: Vec<PricePoint>,
}

// ── Interval enum ─────────────────────────────────────────────────────────────

/// Aggregation window for price history queries.
#[derive(Debug, Clone, Copy)]
pub enum PriceHistoryInterval {
    Max,
    All,
    OneMinute,
    OneWeek,
    OneDay,
    SixHours,
    OneHour,
}

impl PriceHistoryInterval {
    pub fn as_str(&self) -> &'static str {
        match self {
            PriceHistoryInterval::Max => "max",
            PriceHistoryInterval::All => "all",
            PriceHistoryInterval::OneMinute => "1m",
            PriceHistoryInterval::OneWeek => "1w",
            PriceHistoryInterval::OneDay => "1d",
            PriceHistoryInterval::SixHours => "6h",
            PriceHistoryInterval::OneHour => "1h",
        }
    }
}

// ── Request ───────────────────────────────────────────────────────────────────

/// Fetches price history for a single token from `GET /prices-history`.
///
/// # Example
///
/// ```no_run
/// use poly_clob_rs::api::price_history_requests::{PriceHistoryRequest, PriceHistoryInterval};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let resp = PriceHistoryRequest::builder()
///     .market("0xabc123…")
///     .start_ts(Some(1744930800))
///     .end_ts(Some(1744934400))
///     .fidelity(Some(1))
///     .build()
///     .execute()
///     .await?;
///
/// for pt in &resp.history {
///     println!("{} → {:.4}", pt.t, pt.p);
/// }
/// # Ok(())
/// # }
/// ```
#[derive(TypedBuilder)]
pub struct PriceHistoryRequest<'a> {
    /// CLOB token ID (the asset to query).
    #[builder(setter(into))]
    pub market: &'a str,

    /// Start of the time window (Unix seconds). `None` = API default.
    #[builder(default)]
    pub start_ts: Option<i64>,

    /// End of the time window (Unix seconds). `None` = API default.
    #[builder(default)]
    pub end_ts: Option<i64>,

    /// Aggregation interval. `None` = API default.
    #[builder(default)]
    pub interval: Option<PriceHistoryInterval>,

    /// Resolution in minutes (default: 1). `None` = API default.
    #[builder(default)]
    pub fidelity: Option<i32>,
}

impl<'a> PriceHistoryRequest<'a> {
    pub async fn execute(&self) -> Result<PriceHistoryResponse> {
        let client = get_http_client(None);

        let mut req = super::webservice_request::WebserviceRequest {
            api: CLOB_API.to_string(),
            url: GET_PRICES_HISTORY.to_string(),
            method: Method::GET,
            with_pagination: false,
            args: Vec::new(),
            body: None,
        };

        req.add_arg("market".to_string(), self.market.to_string());

        if let Some(ts) = self.start_ts {
            req.add_arg("startTs".to_string(), ts.to_string());
        }
        if let Some(ts) = self.end_ts {
            req.add_arg("endTs".to_string(), ts.to_string());
        }
        if let Some(interval) = self.interval {
            req.add_arg("interval".to_string(), interval.as_str().to_string());
        }
        if let Some(f) = self.fidelity {
            req.add_arg("fidelity".to_string(), f.to_string());
        }

        super::webservice_request::WebserviceRequest::fetch_one::<PriceHistoryResponse>(
            &client,
            &req,
        )
        .await
    }
}
