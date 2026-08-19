use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};

use super::{ApiResponse, KeysetApiResponse, api_response::deserialize_cursor};

/// The gamma API encodes `outcomes` / `outcomePrices` / `clobTokenIds` as a JSON
/// *string* containing a JSON array, e.g. `"[\"Up\",\"Down\"]"`. We parse that once
/// here, at the serde boundary, so downstream code holds typed values and never
/// re-parses per access. A missing/null/empty field yields an empty Vec.
fn de_json_string_to_string_vec<'de, D>(de: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(de)?;
    match opt.as_deref().map(str::trim) {
        Some(s) if !s.is_empty() => serde_json::from_str(s).map_err(serde::de::Error::custom),
        _ => Ok(Vec::new()),
    }
}

/// Like [`de_json_string_to_string_vec`] but for prices. The inner array elements
/// arrive as either bare numbers (`[0.35, 0.65]`) or quoted strings
/// (`["0.55","0.45"]`) depending on the endpoint, so coerce both to `f64`.
fn de_json_string_to_f64_vec<'de, D>(de: D) -> Result<Vec<f64>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(de)?;
    let Some(s) = opt.as_deref().map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(Vec::new());
    };
    let values: Vec<serde_json::Value> =
        serde_json::from_str(s).map_err(serde::de::Error::custom)?;
    values
        .into_iter()
        .map(|v| match v {
            serde_json::Value::Number(n) => n
                .as_f64()
                .ok_or_else(|| serde::de::Error::custom("price not representable as f64")),
            serde_json::Value::String(s) => s.trim().parse::<f64>().map_err(serde::de::Error::custom),
            other => Err(serde::de::Error::custom(format!("unexpected price element: {other}"))),
        })
        .collect()
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
// `default` makes deserialization resilient to fields the gamma API omits for some market
// types (e.g. ladder rungs lack `resolutionSource`). All fields are `Option`, so a missing
// key becomes `None` instead of a hard "missing field" error.
#[serde(rename_all = "camelCase", default)]
pub struct PolyResponseMarket {
    pub id: Option<String>,
    pub question: Option<String>,
    pub condition_id: Option<String>,
    pub slug: Option<String>,
    pub resolution_source: Option<String>,
    pub end_date: Option<String>,
    pub liquidity: Option<String>,
    pub start_date: Option<String>,
    pub image: Option<String>,
    pub icon: Option<String>,
    pub description: Option<String>,
    #[serde(deserialize_with = "de_json_string_to_string_vec")]
    pub outcomes: Vec<String>,
    #[serde(deserialize_with = "de_json_string_to_f64_vec")]
    pub outcome_prices: Vec<f64>,
    pub volume: Option<String>,
    pub active: Option<bool>,
    pub closed: Option<bool>,
    pub market_maker_address: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub new: Option<bool>,
    pub featured: Option<bool>,
    pub submitted_by: Option<String>,
    pub archived: Option<bool>,
    pub resolved_by: Option<String>,
    pub restricted: Option<bool>,
    pub group_item_title: Option<String>,
    pub group_item_threshold: Option<String>,
    #[serde(rename = "questionID")]
    pub question_id: Option<String>,
    pub enable_order_book: Option<bool>,
    pub order_price_min_tick_size: Option<f64>,
    pub order_min_size: Option<f64>,
    pub volume_num: Option<f64>,
    pub liquidity_num: Option<f64>,
    pub end_date_iso: Option<String>,
    pub start_date_iso: Option<String>,
    pub has_reviewed_dates: Option<bool>,
    #[serde(rename = "volume24hr")]
    pub volume24_hr: Option<f64>,
    #[serde(deserialize_with = "de_json_string_to_string_vec")]
    pub clob_token_ids: Vec<String>,
    pub uma_bond: Option<String>,
    pub uma_reward: Option<String>,
    #[serde(rename = "volume24hrClob")]
    pub volume24_hr_clob: Option<f64>,
    pub volume_clob: Option<f64>,
    pub liquidity_clob: Option<f64>,
    pub accepting_orders: Option<bool>,
    pub neg_risk: Option<bool>,
    pub comment_count: Option<i64>,
    #[serde(rename = "_sync")]
    pub sync: Option<bool>,
    pub events: Option<Vec<Event>>,
    pub ready: Option<bool>,
    pub funded: Option<bool>,
    pub accepting_orders_timestamp: Option<String>,
    pub cyom: Option<bool>,
    pub competitive: Option<f64>,
    pub pager_duty_notification_enabled: Option<bool>,
    pub approved: Option<bool>,
    pub clob_rewards: Option<Vec<ClobReward>>,
    pub rewards_min_size: Option<f64>,
    pub rewards_max_spread: Option<f64>,
    pub spread: Option<f64>,
    pub one_day_price_change: Option<f64>,
    pub last_trade_price: Option<f64>,
    pub best_bid: Option<f64>,
    pub best_ask: Option<f64>,
    pub automatically_active: Option<bool>,
    pub clear_book_on_start: Option<bool>,
    pub game_start_time: Option<String>,
    pub seconds_delay: Option<i64>,
    pub event_start_time: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ClobReward {
    pub id: Option<String>,
    pub condition_id: Option<String>,
    pub asset_address: Option<String>,
    pub rewards_amount: Option<f64>,
    pub rewards_daily_rate: Option<f64>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub id: String,
    pub ticker: Option<String>,
    pub slug: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub start_date: Option<String>,
    pub creation_date: Option<String>,
    #[serde(default)]
    pub end_date: DateTime<Utc>,
    pub image: Option<String>,
    pub icon: Option<String>,
    pub active: Option<bool>,
    pub closed: Option<bool>,
    pub archived: Option<bool>,
    pub new: Option<bool>,
    pub featured: Option<bool>,
    pub restricted: Option<bool>,
    pub liquidity: Option<f64>,
    pub volume: Option<f64>,
    pub open_interest: Option<f64>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub competitive: Option<f64>,
    #[serde(rename = "volume24hr")]
    pub volume24_hr: Option<f64>,
    pub enable_order_book: Option<bool>,
    #[serde(rename = "_sync")]
    pub sync: Option<bool>,
    pub neg_risk: Option<bool>,
    pub comment_count: Option<i64>,
    pub cyom: Option<bool>,
    pub show_all_outcomes: Option<bool>,
    pub show_market_images: Option<bool>,
    pub enable_neg_risk: Option<bool>,
    pub automatically_active: Option<bool>,
}

// Response type aliases for clarity
pub type MarketsResponse = Vec<PolyResponseMarket>;

/// Response envelope for the `/markets/keyset` cursor-based pagination endpoint.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KeysetMarketsResponse {
    /// Markets returned in this page (JSON key: `"markets"`).
    #[serde(rename = "markets")]
    pub data: Vec<PolyResponseMarket>,
    /// Cursor for the next page. `None` when no more pages exist (empty string normalised at parse time).
    #[serde(default, deserialize_with = "deserialize_cursor")]
    pub next_cursor: Option<String>,
    /// Page size limit used for this request.
    pub limit: Option<i32>,
    /// Number of items in this page (may differ from `limit` on the last page).
    pub count: Option<i32>,
}

// ApiResponse implementations
impl ApiResponse for PolyResponseMarket {
    fn nb_results(&self) -> usize {
        0 // Single market response
    }
}

impl ApiResponse for MarketsResponse {
    fn nb_results(&self) -> usize {
        self.len()
    }
}

impl KeysetApiResponse for KeysetMarketsResponse {
    fn next_cursor(&self) -> Option<&str> {
        self.next_cursor.as_deref()
    }
}

// ============================================================================
// ClobMarket - the CLOB API's `/markets/{condition_id}` single-resource shape
// ============================================================================

/// One outcome token of a [`ClobMarket`].
///
/// `price` and `winner` are deliberately **private**: this type backs a long-lived
/// metadata cache (see `MarketPositionsController`), so any price read off it would be
/// stale by up to the market's whole duration. Live prices come from the order book
/// (`apply_live_prices_if_requested`) and published resolutions from
/// `market_settlement`; neither goes through here. Keeping the fields unreachable makes
/// that a compile error rather than a subtle mispricing.
#[derive(Debug, Deserialize, Clone, Default)]
pub struct ClobToken {
    pub token_id: String,
    pub outcome: String,
    #[allow(dead_code)]
    price: f64,
    #[allow(dead_code)]
    winner: bool,
}

/// A market as returned by the CLOB's `/markets/{condition_id}` endpoint.
///
/// Unlike gamma's listing endpoints — which apply `closed=false` by default and can miss a
/// short-lived market for its entire lifetime — the CLOB serves any market it accepts orders
/// for, immediately and at any lifecycle stage. This is the authoritative source for the
/// immutable metadata (question, slug, token→outcome mapping) a held position needs to render.
#[derive(Debug, Deserialize, Clone, Default)]
pub struct ClobMarket {
    pub condition_id: String,
    pub question: String,
    pub market_slug: String,
    /// RFC-3339. Note the CLOB reports this as the *event day* rollover for recurring
    /// intraday markets, not the individual window's close.
    pub end_date_iso: Option<String>,
    pub tokens: Vec<ClobToken>,
}

impl ClobMarket {
    /// The outcome name ("Up" / "Down" / …) for `token_id`, if this market holds it.
    pub fn outcome_for_token(&self, token_id: &str) -> Option<&str> {
        self.tokens
            .iter()
            .find(|t| t.token_id == token_id)
            .map(|t| t.outcome.as_str())
    }
}

impl ApiResponse for ClobMarket {
    fn nb_results(&self) -> usize {
        0 // Single market response
    }
}

#[cfg(test)]
mod clob_market_tests {
    use super::*;

    /// Trimmed capture of a real `GET clob.polymarket.com/markets/{condition_id}` response
    /// (eth-updown-5m, Aug 19 2026) — the exact market gamma's listing endpoint failed to
    /// return for its whole lifetime.
    const CAPTURE: &str = r#"{
        "condition_id": "0x9eb18aded66332f579d606f0d9399ffbd680f366bddfddb8a0e1878f90407685",
        "question_id": "0x95bd76984e2e4317ddfe95f23d5c151768d79bac6103646312e3f9bcd34b244f",
        "question": "Ethereum Up or Down - August 19, 5:05AM-5:10AM ET",
        "market_slug": "eth-updown-5m-1787130300",
        "end_date_iso": "2026-08-19T00:00:00Z",
        "closed": true,
        "tokens": [
            {"token_id": "59698098977111906550687924409636834067516700982846510488718568126341509195345",
             "outcome": "Up", "price": 0, "winner": false},
            {"token_id": "17339602082358126254674486477378227513153577250956102310840282339776999176209",
             "outcome": "Down", "price": 1, "winner": true}
        ]
    }"#;

    #[test]
    fn deserializes_real_clob_payload() {
        let m: ClobMarket = serde_json::from_str(CAPTURE).expect("CLOB payload should parse");
        assert_eq!(m.market_slug, "eth-updown-5m-1787130300");
        assert_eq!(m.question, "Ethereum Up or Down - August 19, 5:05AM-5:10AM ET");
        assert_eq!(m.end_date_iso.as_deref(), Some("2026-08-19T00:00:00Z"));
        assert_eq!(m.tokens.len(), 2);
    }

    #[test]
    fn maps_token_id_to_outcome_name() {
        let m: ClobMarket = serde_json::from_str(CAPTURE).unwrap();
        assert_eq!(
            m.outcome_for_token(
                "17339602082358126254674486477378227513153577250956102310840282339776999176209"
            ),
            Some("Down")
        );
        assert_eq!(m.outcome_for_token("not-a-token"), None);
    }
}
