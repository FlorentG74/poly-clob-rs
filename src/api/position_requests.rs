use reqwest::Method;

use super::{WebserviceRequest, DATA_API, POSITIONS};

/// Default `sizeThreshold` for `/positions`.
///
/// **This parameter must be sent explicitly.** Omitting it is NOT "no filter": the
/// data-api defaults to `sizeThreshold=1`, silently dropping every position under one
/// share. Verified against the live endpoint on 2026-07-28 — for the same wallet,
/// no param and `sizeThreshold=1` both returned 0 positions, while `.1` and `0`
/// returned a real 0.5645-share redeemable winner.
///
/// That default is how partial GTD fills landing under one share became invisible to
/// both the trading path and the redeemer, leaving them permanently unredeemable while
/// the activity API still listed their buys (so the web UI showed them as open forever).
const DEFAULT_POSITION_SIZE_THRESHOLD: &str = "0.1";

/// `sizeThreshold` for the redeemer's `--scan-dust` pass: everything, no floor.
const DUST_SCAN_SIZE_THRESHOLD: &str = "0";

impl WebserviceRequest {
    /// Positions request with the standard floor — drops sub-0.1-share residue.
    #[must_use]
    pub fn new_positions_ws_request(user: &str) -> Self {
        Self::positions_request(user, DEFAULT_POSITION_SIZE_THRESHOLD)
    }

    /// Positions request with NO size floor, for the redeemer's `--scan-dust`.
    ///
    /// That pass exists to find what the normal path misses, so inheriting the normal
    /// path's floor would make it skip exactly the residue it is meant to discover.
    #[must_use]
    pub fn new_positions_ws_request_all_sizes(user: &str) -> Self {
        Self::positions_request(user, DUST_SCAN_SIZE_THRESHOLD)
    }

    fn positions_request(user: &str, size_threshold: &str) -> Self {
        let args = vec![
            (String::from("user"), user.to_string()),
            (String::from("sizeThreshold"), size_threshold.to_string()),
        ];

        WebserviceRequest {
            api: DATA_API.to_string(),
            url: POSITIONS.to_string(),
            method: Method::GET,
            with_pagination: true,
            args,
            body: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arg<'a>(req: &'a WebserviceRequest, key: &str) -> Option<&'a str> {
        req.args.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }

    /// `sizeThreshold` must always be sent. Omitting it is not "no filter" — the data-api
    /// defaults to 1 and silently hides every sub-one-share position, which is how
    /// redeemable dust became invisible to both the bot and the redeemer.
    #[test]
    fn positions_request_always_sends_size_threshold() {
        let req = WebserviceRequest::new_positions_ws_request("0xabc");
        assert_eq!(arg(&req, "user"), Some("0xabc"));
        assert_eq!(arg(&req, "sizeThreshold"), Some("0.1"));
    }

    /// The dust scan must not inherit the standard floor, or it skips the very residue
    /// it exists to find.
    #[test]
    fn dust_scan_request_sends_zero_threshold() {
        let req = WebserviceRequest::new_positions_ws_request_all_sizes("0xabc");
        assert_eq!(arg(&req, "sizeThreshold"), Some("0"));
    }

    /// Prints the raw JSON from the positions API so we can inspect the exact field names
    /// and whether `avgPrice` is present and populated in the live response.
    ///
    /// Run with: cargo test -p poly-clob-rs `test_positions_raw_response` -- --nocapture
    #[tokio::test]
    async fn test_positions_raw_response() {
        crate::config::init_from_env();
        let account = crate::models::Account::load_poly_account()
            .expect("load poly account from .env");

        // Query our own wallet first
        let url = format!("{}{}?user={}&sizeThreshold=.1", DATA_API, POSITIONS, account.poly_address);

        // Same client the bot uses, so split tunnelling and the DNS override apply.
        let client = crate::api::http_client::get_http_client(Some(DATA_API));
        let resp = client.get(&url).send().await.expect("HTTP request failed");
        let status = resp.status();
        let body = resp.text().await.expect("read body");

        println!("\n=== Own wallet positions (status={}) ===", status);
        match serde_json::from_str::<serde_json::Value>(&body) {
            Ok(json) => println!("{}", serde_json::to_string_pretty(&json).unwrap()),
            Err(_) => println!("{}", body),
        }

        // Query a reference wallet known to hold positions so we can see the
        // actual field names and types (avgPrice, size, conditionId, etc.)
        let ref_url = format!(
            "{}{}?user={}&sizeThreshold=.1",
            DATA_API, POSITIONS, "0xcd5af8372a943034b65fdac6ef39ceb7826bf7a4"
        );
        let resp2 = client.get(&ref_url).send().await.expect("HTTP request failed");
        let status2 = resp2.status();
        let body2 = resp2.text().await.expect("read body");

        println!("\n=== Reference wallet positions (status={}) ===", status2);
        match serde_json::from_str::<serde_json::Value>(&body2) {
            Ok(json) => {
                // Show only first entry to keep output manageable
                if let Some(arr) = json.as_array() {
                    println!("Total positions: {}", arr.len());
                    if let Some(first) = arr.first() {
                        println!("First entry:\n{}", serde_json::to_string_pretty(first).unwrap());
                    }
                } else {
                    println!("{}", serde_json::to_string_pretty(&json).unwrap());
                }
            }
            Err(_) => println!("{}", body2),
        }
    }
}
