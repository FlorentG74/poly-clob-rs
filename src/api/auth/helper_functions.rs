use chrono::prelude::*;

pub const YEAR_MILLIS: i64 = 31536000000;
pub const DAY_MILLIS: i64 = 86400 * 1000;
pub const HOUR_MILLIS: i64 = DAY_MILLIS / 24;
pub const MINUTE_MILLIS: i64 = HOUR_MILLIS / 60;
pub const SECOND_MILLIS: i64 = 1000;

pub fn timestamp_to_datetime(millis: i64) -> chrono::NaiveDateTime {
    DateTime::from_timestamp_millis(millis).unwrap().naive_utc()
}

pub fn offset_current_time(offset: i64) -> i64 {
    let ts: DateTime<Utc> = Utc::now();
    ts.timestamp_millis() + offset
}

pub fn format_redis_ts(string_ts: &str) -> i64 {
    let ts_vec: Vec<&str> = string_ts.split('-').collect();
    let timestamp: i64 = ts_vec[0].parse().unwrap();

    timestamp
}

pub fn convert_string_to_nullable_time(input: Option<&String>) -> Option<DateTime<Utc>> {
    match input {
        Some(str_date) => {
            return Some(
                DateTime::parse_from_rfc3339(str_date.as_str())
                    .unwrap()
                    .into(),
            );
        }
        None => return None,
    }
}
