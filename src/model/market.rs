use std::collections::HashMap;

use chrono::{DateTime, Utc};

use super::Outcome;


use crate::controller::{
    convert_string_to_nullable_time, WebserviceRequest, WebserviceResponse, GAMMA_API, GET_MARKETS,
};

use reqwest::Method;
use serde::{Deserialize, Serialize};
use string_builder::Builder;

#[allow(dead_code)]
pub struct Market {
    id: String,
    pub question: String,
    pub condition_id: String,
    pub slug: String,
    pub end_date: Option<DateTime<Utc>>,
    start_date: Option<DateTime<Utc>>,
    pub description: String,
    pub outcomes: String,
    pub outcome_prices: String,
    pub clob_token_ids: String,
    pub active: bool,
    pub closed: bool,
    created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    archived: bool,
    pub group_item_title: String,
    question_id: String,
    enable_order_book: bool,
    order_price_min_tick_size: f64,
    order_min_size: f64,
    volume_num: f64,
    liquidity_num: f64,
    accepting_orders: bool,
    neg_risk: bool,
    last_trade_price: f64,
    best_bid: f64,
    best_ask: f64,
}
#[warn(dead_code)]

impl Market {
    pub fn from_poly_response_market(prm: &PolyResponseMarket) -> Self {
        Market {
            id: prm.id.clone().unwrap_or("default".to_string()),
            question: prm.question.clone().unwrap_or("default".to_string()),
            condition_id: prm.condition_id.clone().unwrap_or("default".to_string()),
            slug: prm.slug.clone().unwrap_or("default".to_string()),
            end_date: convert_string_to_nullable_time(prm.end_date.as_ref()),
            start_date: convert_string_to_nullable_time(prm.start_date.as_ref()),
            description: prm.description.clone().unwrap_or("default".to_string()),
            outcomes: prm.outcomes.clone().unwrap_or("default".to_string()),
            outcome_prices: prm.outcome_prices.clone().unwrap_or("default".to_string()),
            clob_token_ids: prm.clob_token_ids.clone().unwrap_or("default".to_string()),
            active: prm.active.unwrap_or(false),
            closed: prm.closed.unwrap_or(false),
            created_at: convert_string_to_nullable_time(prm.created_at.as_ref()),
            updated_at: convert_string_to_nullable_time(prm.updated_at.as_ref()),
            archived: prm.archived.unwrap_or(false),
            group_item_title: prm
                .group_item_title
                .clone()
                .unwrap_or("default".to_string()),
            question_id: prm.question_id.clone().unwrap_or("default".to_string()),
            enable_order_book: prm.enable_order_book.unwrap_or(false),
            order_price_min_tick_size: prm.order_price_min_tick_size.unwrap_or(f64::from(0.0)),
            order_min_size: prm.order_min_size.unwrap_or(f64::from(0.0)),
            volume_num: prm.volume_num.unwrap_or(f64::from(0.0)),
            liquidity_num: prm.liquidity_num.unwrap_or(f64::from(0.0)),
            accepting_orders: prm.accepting_orders.unwrap_or(false),
            neg_risk: prm.neg_risk.unwrap_or(false),
            last_trade_price: prm.last_trade_price.unwrap_or(f64::from(0.0)),
            best_bid: prm.best_bid.unwrap_or(f64::from(0.0)),
            best_ask: prm.best_ask.unwrap_or(f64::from(0.0)),
        }
    }

    pub fn get_outcomes(&self) -> (Outcome, Outcome) {
        let outcomes =
            serde_json::from_str::<Vec<&str>>(&self.outcomes).expect("Cant parse outcomes");
        let prices =
            serde_json::from_str::<Vec<&str>>(&self.outcome_prices).expect("Cant parse outcomes");
        let token_ids =
            serde_json::from_str::<Vec<&str>>(&self.clob_token_ids).expect("Cant parse outcomes");

        let o1: Outcome = Outcome {
            outcome: outcomes[0].to_string(),
            price: prices[0].parse::<f64>().unwrap(),
            token_id: token_ids[0].to_string(),
        };
        let o2: Outcome = Outcome {
            outcome: outcomes[1].to_string(),
            price: prices[1].parse::<f64>().unwrap(),
            token_id: token_ids[1].to_string(),
        };

        (o1, o2)
    }

    pub fn get_yes_outcome(&self) -> Outcome {
        let (a, b) = self.get_outcomes();

        if a.outcome.eq("Yes") {
            a
        } else {
            b
        }
    }
}

pub type MarketsResponse = Vec<PolyResponseMarket>;

impl WebserviceRequest {
    pub fn new_markets_ws_request() -> Self {
        let args = HashMap::<String, String>::new();

        return WebserviceRequest {
            api: GAMMA_API.to_string(),
            url: GET_MARKETS.to_string(),
            method: Method::GET,
            args: args,
            body: None,
        };
    }

    pub fn with_active_only(&mut self) {
        self.args.insert("active".to_string(), "true".to_string());
    }

    pub fn with_from_start_date(&mut self, start_date_min: String) {
        self.args
            .insert("start_date_min".to_string(), start_date_min);
    }

    pub fn with_tag_id(&mut self, tag_id: &str) {
        self.args.insert("tag_id".to_string(), tag_id.to_string());
    }

    pub fn with_related_tags(&mut self) {
        self.args
            .insert("related_tags".to_string(), "true".to_string());
    }

    pub fn with_condition_ids(&mut self, condition_ids: &Vec<String>) {
        self.args.insert(
            "condition_ids".to_string(),
            Self::format_condition_ids_query(condition_ids),
        );
    }

    fn format_condition_ids_query(condition_ids: &Vec<String>) -> String {
        let mut builder = Builder::default();

        let mut it = condition_ids.iter().peekable();
        while let Some(condition_id) = it.next() {
            builder.append(condition_id.clone());
            if it.peek().is_some() {
                builder.append("&condition_ids=");
            }
        }

        builder.string().expect("Error in String conversion")
    }
}

impl WebserviceResponse for MarketsResponse {
    async fn store(&self) {
    }

    fn nb_results(&self) -> usize {
        self.len()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PolyResponseMarket {
    id: Option<String>,
    pub question: Option<String>,
    pub condition_id: Option<String>,
    pub slug: Option<String>,
    resolution_source: Option<String>,
    end_date: Option<String>,
    liquidity: Option<String>,
    start_date: Option<String>,
    image: Option<String>,
    icon: Option<String>,
    description: Option<String>,
    pub outcomes: Option<String>,
    pub outcome_prices: Option<String>,
    volume: Option<String>,
    active: Option<bool>,
    pub closed: Option<bool>,
    market_maker_address: Option<String>,
    created_at: Option<String>,
    updated_at: Option<String>,
    new: Option<bool>,
    featured: Option<bool>,
    submitted_by: Option<String>,
    archived: Option<bool>,
    resolved_by: Option<String>,
    restricted: Option<bool>,
    pub group_item_title: Option<String>,
    group_item_threshold: Option<String>,
    #[serde(rename = "questionID")]
    question_id: Option<String>,
    enable_order_book: Option<bool>,
    order_price_min_tick_size: Option<f64>,
    order_min_size: Option<f64>,
    volume_num: Option<f64>,
    liquidity_num: Option<f64>,
    end_date_iso: Option<String>,
    start_date_iso: Option<String>,
    has_reviewed_dates: Option<bool>,
    #[serde(rename = "volume24hr")]
    volume24_hr: Option<f64>,
    pub clob_token_ids: Option<String>,
    uma_bond: Option<String>,
    uma_reward: Option<String>,
    #[serde(rename = "volume24hrClob")]
    volume24_hr_clob: Option<f64>,
    volume_clob: Option<f64>,
    liquidity_clob: Option<f64>,
    accepting_orders: Option<bool>,
    neg_risk: Option<bool>,
    comment_count: Option<i64>,
    #[serde(rename = "_sync")]
    sync: Option<bool>,
    pub events: Option<Vec<Event>>,
    ready: Option<bool>,
    funded: Option<bool>,
    accepting_orders_timestamp: Option<String>,
    cyom: Option<bool>,
    competitive: Option<f64>,
    pager_duty_notification_enabled: Option<bool>,
    approved: Option<bool>,
    clob_rewards: Option<Vec<ClobReward>>,
    rewards_min_size: Option<f64>,
    rewards_max_spread: Option<f64>,
    spread: Option<f64>,
    one_day_price_change: Option<f64>,
    last_trade_price: Option<f64>,
    best_bid: Option<f64>,
    best_ask: Option<f64>,
    automatically_active: Option<bool>,
    clear_book_on_start: Option<bool>,
    game_start_time: Option<String>,
    seconds_delay: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ClobReward {
    id: Option<String>,
    condition_id: Option<String>,
    asset_address: Option<String>,
    rewards_amount: Option<f64>,
    rewards_daily_rate: Option<f64>,
    start_date: Option<String>,
    end_date: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    id: Option<String>,
    ticker: Option<String>,
    slug: Option<String>,
    pub title: Option<String>,
    description: Option<String>,
    start_date: Option<String>,
    creation_date: Option<String>,
    end_date: Option<String>,
    image: Option<String>,
    icon: Option<String>,
    active: Option<bool>,
    closed: Option<bool>,
    archived: Option<bool>,
    new: Option<bool>,
    featured: Option<bool>,
    restricted: Option<bool>,
    liquidity: Option<f64>,
    volume: Option<f64>,
    open_interest: Option<f64>,
    created_at: Option<String>,
    updated_at: Option<String>,
    competitive: Option<f64>,
    #[serde(rename = "volume24hr")]
    volume24_hr: Option<f64>,
    enable_order_book: Option<bool>,
    liquidity_clob: Option<f64>,
    #[serde(rename = "_sync")]
    sync: Option<bool>,
    neg_risk: Option<bool>,
    comment_count: Option<i64>,
    cyom: Option<bool>,
    show_all_outcomes: Option<bool>,
    show_market_images: Option<bool>,
    enable_neg_risk: Option<bool>,
    automatically_active: Option<bool>,
}

#[cfg(test)]
mod tags_tests {
    use regex::Regex;

    #[test]
    fn parse_underlyer_from_description() {
        let description = "This market will immediately resolve to \"Yes\" if any Binance 1 minute candle for Bitcoin (BTCUSDT) between December 2, 2024, 00:00 and December 31, 2024, 23:59 in the ET timezone has a final \"High\" price of $120,000.00 or higher. Otherwise, this market will resolve to \"No.\"\n\nThe resolution source for this market is Binance, specifically the BTCUSDT \"High\" prices available at https://www.binance.com/en/trade/BTC_USDT, with the chart settings on \"1m\" for one-minute candles selected on the top bar.\n\nPlease note that the outcome of this market depends solely on the price data from the Binance BTCUSDT trading pair. Prices from other exchanges, different trading pairs, or spot markets will not be considered for the resolution of this market.\n";

        //let strike_re = Regex::new(r"https://www.binance.com/en/trade/.*").unwrap();
        let strike_re = Regex::new(r"https://www.binance.com/en/trade/(?<pair>.*?)(,|\s)").unwrap();

        //https://www.binance.com/en/trade/BTC_USDT

        let Some(caps) = strike_re.captures(description) else {
            return;
        };

        let pair = &caps["pair"].replace("_", "");

        println!("Result {:?}", pair);
    }
}
