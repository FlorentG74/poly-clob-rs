use crate::controller::L1Header;
use crate::model::Account;

use base64::engine::general_purpose::URL_SAFE;
use base64::prelude::*;
use chrono::prelude::*;
use hmac::Mac;
use reqwest::header::HeaderMap;

use super::build_l1_signature;

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

// Reserved for future use
#[allow(dead_code)]
pub fn build_l1_headers(signer: &Account, nonce: i32) -> HeaderMap {
    let mut headers = HeaderMap::new();

    headers.append("POLY_ADDRESS", signer.pub_key.parse().unwrap());

    let l1_header = L1Header::new(signer.pub_key.as_str());
    let timestamp = get_timestamp();

    let signature = build_l1_signature(&l1_header, timestamp.as_str(), signer.private_key.as_str());

    headers.append("POLY_SIGNATURE", signature.parse().unwrap());
    headers.append("POLY_TIMESTAMP", timestamp.parse().unwrap());
    headers.append("POLY_NONCE", nonce.into());

    headers
}

pub fn build_l2_headers(
    signer: &Account,
    method: &str,
    request_path: &str,
    body: &str,
) -> HeaderMap {
    let mut headers = HeaderMap::new();

    headers.append("POLY_ADDRESS", signer.pub_key.parse().unwrap());
    headers.append("POLY_API_KEY", signer.api_key.parse().unwrap());
    headers.append("POLY_PASSPHRASE", signer.api_passphrase.parse().unwrap());

    let timestamp = get_timestamp();
    headers.append("POLY_TIMESTAMP", timestamp.parse().unwrap());
    let signature =
        build_hmac_signature(&signer.api_secret, &timestamp, method, request_path, body);
    headers.append("POLY_SIGNATURE", signature.parse().unwrap());

    headers
}

pub fn build_hmac_signature(
    api_secret: &String,
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

#[cfg(test)]
mod helper_functions_tests {

    use super::*;
    use crate::controller::GET_BALANCE_ALLOWANCE;

    #[test]
    pub fn test_hmac() {
        let _ = env_logger::try_init();

        let signer = Account::actual_account_from_env();
        // let message = "1728848653GET/balance-allowance";
        // Expected result  XCxH8n2u9RcDQeo23RfxWl6upGjU7vKj9Ue4HV1zw6A=

        let timestamp = "1728848653";
        let method = "GET";
        let request_path = "/balance-allowance";
        let request_body = "";

        let res = build_hmac_signature(
            &signer.api_secret,
            timestamp,
            method,
            request_path,
            request_body,
        );
        log::info!("Sig: {}", res);

        assert_eq!(
            res,
            "XCxH8n2u9RcDQeo23RfxWl6upGjU7vKj9Ue4HV1zw6A=".to_string()
        );
    }

    #[test]
    pub fn test_url_builder() {
        let mut url = format!("https://clob.polymarket.com{}", GET_BALANCE_ALLOWANCE);

        // Test add 1st param
        add_param_to_url(&mut url, "param1", "value1");
        assert_eq!(
            url,
            "https://clob.polymarket.com/balance-allowance?param1=value1".to_string()
        );

        // Test add 2nd param
        add_param_to_url(&mut url, "param2", "value2");
        assert_eq!(
            url,
            "https://clob.polymarket.com/balance-allowance?param1=value1&param2=value2".to_string()
        );

        // Test add empty param
        add_param_to_url(&mut url, "param3", "");
        assert_eq!(
            url,
            "https://clob.polymarket.com/balance-allowance?param1=value1&param2=value2".to_string()
        );
    }

    #[test]
    pub fn test_time_functions() {
        let ts = timestamp_to_datetime(1731090383298);
        println!("Date: {}", ts);

        assert_eq!("2024-11-08 18:26:23.298", ts.to_string());
    }

    #[test]
    pub fn test_time_offset() {
        let now = Utc::now();
        let offsetted_time = offset_current_time(-HOUR_MILLIS);

        assert_eq!(now.timestamp_millis() - HOUR_MILLIS, offsetted_time);
    }

    #[test]
    pub fn convert_string_to_datetime_option() {
        let input_some = Some(String::from("2024-09-11T16:47:38.897537Z"));
        let result_some = convert_string_to_nullable_time(input_some.as_ref());
        assert!(result_some.is_some());

        let input_none = None;
        let result_none = convert_string_to_nullable_time(input_none);
        assert!(result_none.is_none());
    }
}
