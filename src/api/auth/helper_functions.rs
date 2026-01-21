use chrono::prelude::*;

use crate::api::error::{Result, SerializationError, ValidationError};

pub const YEAR_MILLIS: i64 = 31536000000;
pub const DAY_MILLIS: i64 = 86400 * 1000;
pub const HOUR_MILLIS: i64 = DAY_MILLIS / 24;
pub const MINUTE_MILLIS: i64 = HOUR_MILLIS / 60;
pub const SECOND_MILLIS: i64 = 1000;

pub fn timestamp_to_datetime(millis: i64) -> Result<chrono::NaiveDateTime> {
    DateTime::from_timestamp_millis(millis)
        .map(|dt| dt.naive_utc())
        .ok_or_else(|| {
            ValidationError::InvalidParameter {
                parameter: "timestamp".to_string(),
                reason: format!("invalid timestamp millis: {}", millis),
            }
            .into()
        })
}

pub fn offset_current_time(offset: i64) -> i64 {
    let ts: DateTime<Utc> = Utc::now();
    ts.timestamp_millis() + offset
}

pub fn format_redis_ts(string_ts: &str) -> Result<i64> {
    let ts_vec: Vec<&str> = string_ts.split('-').collect();
    let ts_str = ts_vec.first().ok_or_else(|| SerializationError::FieldParse {
        field: "redis_timestamp".to_string(),
        message: "empty redis timestamp".to_string(),
    })?;

    ts_str.parse().map_err(|e| {
        SerializationError::FieldParse {
            field: "redis_timestamp".to_string(),
            message: format!("failed to parse redis timestamp: {}", e),
        }
        .into()
    })
}

pub fn convert_string_to_nullable_time(input: Option<&String>) -> Result<Option<DateTime<Utc>>> {
    match input {
        Some(str_date) => {
            let dt = DateTime::parse_from_rfc3339(str_date).map_err(|e| {
                SerializationError::FieldParse {
                    field: "datetime".to_string(),
                    message: format!("invalid RFC3339 date '{}': {}", str_date, e),
                }
            })?;
            Ok(Some(dt.into()))
        }
        None => Ok(None),
    }
}
