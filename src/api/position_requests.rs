use reqwest::Method;

use super::{WebserviceRequest, DATA_API, POSITIONS};

impl WebserviceRequest {
    pub fn new_positions_ws_request(user: &str) -> Self {
        let args = vec![(String::from("user"), user.to_string())];

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

    /// Prints the raw JSON from the positions API so we can inspect the exact field names
    /// and whether `avgPrice` is present and populated in the live response.
    ///
    /// Run with: cargo test -p poly-clob-rs test_positions_raw_response -- --nocapture
    #[tokio::test]
    async fn test_positions_raw_response() {
        let account = crate::models::Account::load_poly_account()
            .expect("load poly account from .env");

        // Query our own wallet first
        let url = format!("{}{}?user={}&sizeThreshold=.1", DATA_API, POSITIONS, account.poly_address);

        let client = reqwest::Client::new();
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
