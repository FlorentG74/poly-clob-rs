use base64::engine::general_purpose::URL_SAFE;
use base64::prelude::*;
use chrono::prelude::*;
use hmac::Mac;
use reqwest::header::HeaderMap;

pub const YEAR_MILLIS: i64 = 31536000000;
pub const DAY_MILLIS: i64 = 86400 * 1000;
pub const HOUR_MILLIS: i64 = DAY_MILLIS / 24;
pub const MINUTE_MILLIS: i64 = HOUR_MILLIS / 60;
pub const SECOND_MILLIS: i64 = 1000;

pub fn get_timestamp() -> String {
    let now = Utc::now();
    now.timestamp().to_string()
}

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

pub fn build_l2_headers(
    poly_address: &str,
    api_key: &str,
    api_secret: &str,
    api_passphrase: &str,
    method: &str,
    request_path: &str,
    body: &str,
    salt: &str,
) -> HeaderMap {
    let mut headers = HeaderMap::new();

    headers.append("POLY_ADDRESS", poly_address.parse().unwrap());
    headers.append("POLY_API_KEY", api_key.parse().unwrap());
    headers.append("POLY_PASSPHRASE", api_passphrase.parse().unwrap());

    let timestamp = if "".eq(salt) { get_timestamp() } else { salt.to_string() };

    headers.append("POLY_TIMESTAMP", timestamp.parse().unwrap());
    let signature =
        build_hmac_signature(api_secret, &timestamp, method, request_path, body);
    headers.append("POLY_SIGNATURE", signature.parse().unwrap());

    headers
}

pub fn build_hmac_signature(
    api_secret: &str,
    timestamp: &str,
    method: &str,
    request_path: &str,
    request_body: &str,
) -> String {
    let message = timestamp.to_string() + method + request_path + request_body;

    let b64_decoded_secret = URL_SAFE.decode(api_secret).unwrap();
    let b64_decoded_secret_slice = b64_decoded_secret.as_slice();

    type HmacSha256 = hmac::Hmac<sha2::Sha256>;
    let mut mac = HmacSha256::new_from_slice(b64_decoded_secret_slice)
        .expect("HMAC can take key of any size");
    mac.update(message.as_bytes());

    let bytes = mac.finalize().into_bytes();

    let mut signature: String = Default::default();
    URL_SAFE.encode_string(bytes, &mut signature);

    signature
}

pub fn add_param_to_url(url: &mut String, name: &str, value: &str) {
    if value.is_empty() {
        return;
    }

    if !url.contains("?") {
        url.push_str(format!("?{}={}", name, value).as_str());
    } else {
        url.push_str(format!("&{}={}", name, value).as_str());
    }
}

pub fn get_zero_address() -> String {
    "0x0000000000000000000000000000000000000000".to_string()
}
