use anyhow::{Context, Result};
use chrono::prelude::*;

pub const YEAR_MILLIS: i64 = 31536000000;
pub const DAY_MILLIS: i64 = 86400 * 1000;
pub const HOUR_MILLIS: i64 = DAY_MILLIS / 24;
pub const MINUTE_MILLIS: i64 = HOUR_MILLIS / 60;
pub const SECOND_MILLIS: i64 = 1000;

pub fn timestamp_to_datetime(millis: i64) -> Result<chrono::NaiveDateTime> {
    DateTime::from_timestamp_millis(millis)
        .map(|dt| dt.naive_utc())
        .context("invalid timestamp millis")
}

pub fn offset_current_time(offset: i64) -> i64 {
    let ts: DateTime<Utc> = Utc::now();
    ts.timestamp_millis() + offset
}

pub fn format_redis_ts(string_ts: &str) -> Result<i64> {
    let ts_vec: Vec<&str> = string_ts.split('-').collect();
    ts_vec
        .first()
        .context("empty redis timestamp")?
        .parse()
        .context("failed to parse redis timestamp")
}

pub fn convert_string_to_nullable_time(input: Option<&String>) -> Result<Option<DateTime<Utc>>> {
    match input {
        Some(str_date) => {
            let dt = DateTime::parse_from_rfc3339(str_date)
                .with_context(|| format!("invalid RFC3339 date: {str_date}"))?;
            Ok(Some(dt.into()))
        }
        None => Ok(None),
    }
}
