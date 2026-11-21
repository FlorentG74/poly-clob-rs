use reqwest::Method;

use super::{WebserviceRequest, DATA_API, POSITIONS};

/// Default `filter_amount` (shares) for `/v2/positions`. Sent explicitly so a change
/// to the server default can't hide sub-one-share partial GTD fills.
const DEFAULT_POSITION_MIN_TOKENS: &str = "0.1";

/// `filter_amount` for the redeemer's `--scan-dust` pass: everything, no floor.
const DUST_SCAN_MIN_TOKENS: &str = "0";

/// First-page size; the bot holds a handful of positions, so this is normally one call.
const POSITIONS_PAGE_LIMIT: &str = "500";

impl WebserviceRequest {
    /// Positions request with the standard floor — drops sub-0.1-share residue.
    ///
    /// Default `status=OPEN` includes settled-but-unredeemed positions (rows carry
    /// `redeemable = true`), which the redeemer relies on.
    #[must_use]
    pub fn new_positions_ws_request(user: &str) -> Self {
        Self::positions_request(user, DEFAULT_POSITION_MIN_TOKENS)
    }

    /// Positions request with NO size floor, for the redeemer's `--scan-dust`.
    ///
    /// That pass exists to find what the normal path misses, so inheriting the normal
    /// path's floor would make it skip exactly the residue it is meant to discover.
    #[must_use]
    pub fn new_positions_ws_request_all_sizes(user: &str) -> Self {
        Self::positions_request(user, DUST_SCAN_MIN_TOKENS)
    }

    fn positions_request(user: &str, min_tokens: &str) -> Self {
        let args = vec![
            (String::from("user"), user.to_string()),
            (String::from("filter_type"), String::from("TOKENS")),
            (String::from("filter_amount"), min_tokens.to_string()),
            (String::from("limit"), String::from(POSITIONS_PAGE_LIMIT)),
        ];

        WebserviceRequest {
            api: DATA_API.to_string(),
            url: POSITIONS.to_string(),
            method: Method::GET,
            with_pagination: false,
            args,
            body: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Position;

    fn arg<'a>(req: &'a WebserviceRequest, key: &str) -> Option<&'a str> {
        req.args.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }

    /// The token floor must always be sent explicitly rather than trusting the server
    /// default, which is how sub-one-share dust once became invisible.
    #[test]
    fn positions_request_always_sends_token_floor() {
        let req = WebserviceRequest::new_positions_ws_request("0xabc");
        assert_eq!(arg(&req, "user"), Some("0xabc"));
        assert_eq!(arg(&req, "filter_type"), Some("TOKENS"));
        assert_eq!(arg(&req, "filter_amount"), Some("0.1"));
    }

    /// The dust scan must not inherit the standard floor, or it skips the very residue
    /// it exists to find.
    #[test]
    fn dust_scan_request_sends_zero_floor() {
        let req = WebserviceRequest::new_positions_ws_request_all_sizes("0xabc");
        assert_eq!(arg(&req, "filter_amount"), Some("0"));
    }

    /// v2 rejects `offset` (even `offset=0`) with a 400.
    #[test]
    fn positions_url_has_no_offset() {
        let url = WebserviceRequest::new_positions_ws_request("0xabc").get_callable_url(0);
        assert!(!url.contains("offset"), "{url}");
        assert!(url.starts_with("https://data-api.polymarket.com/v2/positions?"), "{url}");
    }

    #[test]
    fn position_row_deserializes_from_v2_shape() {
        let json = r#"{"data":[{"proxy_wallet":"0xw","token_id":"123","condition_id":"0xc",
            "current_size":198.78,"avg_price":0.2984,"entry_cost_usdc":59.33,"entry_fees_usdc":0.0,
            "total_cost_usdc":59.33,"current_price":0.0,"current_value":0.0,"total_size":198.78,
            "realized_pnl":0.0,"unrealized_pnl":-59.33,"total_pnl":-59.33,"percent_pnl":-99.99,
            "percent_realized_pnl":-100.0,"status":"REDEEMABLE","redeemable":true,"mergeable":false,
            "negative_risk":false,"archived":false,"title":"t","slug":"s","icon":"i","event_id":"1",
            "event_slug":"e","outcome":"Up","outcome_index":0,"opposite_outcome":"Down",
            "opposite_token_id":"456","end_date":"2026-02-20","first_entry_at":0,"last_event_at":0}],
            "pagination":{"has_more":false,"limit":1,"offset":0,"next_cursor":null}}"#;
        let page: crate::models::DataApiPage<Position> = serde_json::from_str(json).unwrap();
        let p = &page.data[0];
        assert_eq!(p.asset, "123");
        assert_eq!(p.opposite_asset, "456");
        assert!((p.size - 198.78).abs() < 1e-9);
        assert!((p.initial_value - 59.33).abs() < 1e-9);
        assert!((p.cash_pnl + 59.33).abs() < 1e-9);
        assert!((p.total_bought - 198.78).abs() < 1e-9);
        assert!(p.redeemable);
        assert!(crate::models::KeysetApiResponse::next_cursor(&page).is_none());
    }

    /// Walks the live v2 positions of our own wallet and of a reference wallet known to
    /// hold many positions, through the same request builder, client and deserializer
    /// the bot and redeemer use.
    ///
    /// Run with: cargo test -p poly-clob-rs `test_positions_live` -- --nocapture
    #[test]
    fn test_positions_live() {
        crate::test_runtime().block_on(async {
            crate::config::init_from_env();
            let client = crate::api::http_client::get_http_client(Some(DATA_API));

            let account = crate::models::Account::load_poly_account()
                .expect("load poly account from .env");
            let own = WebserviceRequest::fetch_data_api_all::<Position>(
                client,
                &WebserviceRequest::new_positions_ws_request_all_sizes(&account.poly_address),
            )
            .await
            .expect("own wallet positions");
            println!("Own wallet: {} position(s)", own.len());

            // Small page so the cursor walk is exercised across several pages.
            let mut req = WebserviceRequest::new_positions_ws_request(
                "0xcd5af8372a943034b65fdac6ef39ceb7826bf7a4",
            );
            req.args.retain(|(k, _)| k != "limit");
            req.add_arg("limit".to_string(), "25".to_string());
            let reference = WebserviceRequest::fetch_data_api_all::<Position>(client, &req)
                .await
                .expect("reference wallet positions");
            println!("Reference wallet: {} position(s)", reference.len());
            assert!(reference.len() > 25, "expected a multi-page walk, got {}", reference.len());
            assert!(reference.iter().all(|p| !p.asset.is_empty() && p.size >= 0.1));
            if let Some(first) = reference.first() {
                println!("First entry: {first:?}");
            }
        });
    }
}
