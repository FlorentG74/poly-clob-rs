//! Integration tests for the Bridge API client ([`BridgeClient`]).
//!
//! These tests drive the real `BridgeClient` against a minimal in-process mock
//! HTTP server (built on the existing `tokio` dependency) instead of hitting the
//! live `bridge.polymarket.com` endpoint. This keeps them deterministic and
//! offline while still exercising URL construction, HTTP method/body handling,
//! status-code handling, and JSON (de)serialization end-to-end.

use poly_clob_rs::api::bridge::{BridgeClient, BridgeTransactionStatus, QuoteRequest, WithdrawalRequest};
use poly_clob_rs::ClobError;

use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// A request captured by the mock server (method + path).
#[derive(Debug, Clone)]
struct CapturedRequest {
    method: String,
    path: String,
}

/// Spawn a mock HTTP server that answers each request via `handler`.
///
/// The handler receives the request `(method, path)` and returns
/// `(status_code, json_body)`. Responses set `Connection: close` so each
/// request uses a fresh connection (avoids reqwest keep-alive pooling getting
/// in the way of the simple line-oriented server). Returns the base URL and a
/// shared log of captured requests.
async fn spawn_mock<F>(handler: F) -> (String, Arc<tokio::sync::Mutex<Vec<CapturedRequest>>>)
where
    F: Fn(&str, &str) -> (u16, String) + Send + Sync + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{}", addr);

    let captured = Arc::new(tokio::sync::Mutex::new(Vec::new()));
    let captured_srv = captured.clone();
    let handler = Arc::new(handler);

    tokio::spawn(async move {
        loop {
            let (mut socket, _) = match listener.accept().await {
                Ok(s) => s,
                Err(_) => break,
            };
            let handler = handler.clone();
            let captured = captured_srv.clone();
            tokio::spawn(async move {
                // Read the request head (everything up to the blank line).
                let mut buf = Vec::new();
                let mut tmp = [0u8; 1024];
                loop {
                    let n = match socket.read(&mut tmp).await {
                        Ok(0) | Err(_) => return,
                        Ok(n) => n,
                    };
                    buf.extend_from_slice(&tmp[..n]);
                    if let Some(pos) = find_header_end(&buf) {
                        // Drain any remaining body bytes per Content-Length so the
                        // client's write completes cleanly before we respond.
                        if let Some(len) = content_length(&buf) {
                            let have = buf.len() - pos;
                            let mut remaining = len.saturating_sub(have);
                            while remaining > 0 {
                                let n = match socket.read(&mut tmp).await {
                                    Ok(0) | Err(_) => break,
                                    Ok(n) => n,
                                };
                                remaining = remaining.saturating_sub(n);
                            }
                        }
                        break;
                    }
                }

                let head = String::from_utf8_lossy(&buf);
                let request_line = head.lines().next().unwrap_or_default();
                let mut parts = request_line.split_whitespace();
                let method = parts.next().unwrap_or_default().to_string();
                let path = parts.next().unwrap_or_default().to_string();

                captured.lock().await.push(CapturedRequest {
                    method: method.clone(),
                    path: path.clone(),
                });

                let (status, body) = handler(&method, &path);
                let reason = match status {
                    200 => "OK",
                    201 => "Created",
                    400 => "Bad Request",
                    500 => "Internal Server Error",
                    _ => "Status",
                };
                let response = format!(
                    "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    status,
                    reason,
                    body.len(),
                    body
                );
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.flush().await;
            });
        }
    });

    (base_url, captured)
}

fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n").map(|p| p + 4)
}

fn content_length(buf: &[u8]) -> Option<usize> {
    let head = String::from_utf8_lossy(buf);
    for line in head.lines() {
        if let Some((name, value)) = line.split_once(':') {
            if name.trim().eq_ignore_ascii_case("content-length") {
                return value.trim().parse().ok();
            }
        }
    }
    None
}

#[tokio::test]
async fn create_deposit_addresses_parses_201_response() {
    let (base_url, captured) = spawn_mock(|_method, _path| {
        let body = r#"{
            "address": {
                "evm": "0x23566f8b2E82aDfCf01846E54899d110e97AC053",
                "svm": "CrvTBvzryYxBHbWu2TiQpcqD5M7Le7iBKzVmEj3f36Jb",
                "btc": "bc1q8eau83qffxcj8ht4hsjdza3lha9r3egfqysj3g"
            },
            "note": "Only certain chains and tokens are supported."
        }"#;
        (201, body.to_string())
    })
    .await;

    let client = BridgeClient::builder().base_url(base_url).build();
    let resp = client
        .create_deposit_addresses("0x56687bf447db6ffa42ffe2204a05edaa20f55839")
        .await
        .expect("deposit should succeed");

    assert_eq!(
        resp.address.evm.as_deref(),
        Some("0x23566f8b2E82aDfCf01846E54899d110e97AC053")
    );
    assert!(resp.note.is_some());

    let reqs = captured.lock().await;
    assert_eq!(reqs.len(), 1);
    assert_eq!(reqs[0].method, "POST");
    assert_eq!(reqs[0].path, "/deposit");
}

#[tokio::test]
async fn get_supported_assets_unwraps_inner_list() {
    let (base_url, captured) = spawn_mock(|_method, _path| {
        let body = r#"{
            "supportedAssets": [
                {
                    "chainId": "1",
                    "chainName": "Ethereum",
                    "token": {
                        "name": "USD Coin",
                        "symbol": "USDC",
                        "address": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
                        "decimals": 6
                    },
                    "minCheckoutUsd": 45
                }
            ]
        }"#;
        (200, body.to_string())
    })
    .await;

    let client = BridgeClient::builder().base_url(base_url).build();
    let assets = client
        .get_supported_assets()
        .await
        .expect("supported-assets should succeed");

    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0].chain_name, "Ethereum");
    assert_eq!(assets[0].token.symbol, "USDC");

    let reqs = captured.lock().await;
    assert_eq!(reqs[0].method, "GET");
    assert_eq!(reqs[0].path, "/supported-assets");
}

#[tokio::test]
async fn get_transaction_status_builds_path_and_parses_statuses() {
    let (base_url, captured) = spawn_mock(|_method, _path| {
        let body = r#"{
            "transactions": [
                { "status": "DEPOSIT_DETECTED" },
                { "status": "COMPLETED", "txHash": "abc", "createdTimeMs": 1757531217339 }
            ]
        }"#;
        (200, body.to_string())
    })
    .await;

    let client = BridgeClient::builder().base_url(base_url).build();
    let txs = client
        .get_transaction_status("EXoZue2avJae1d45B3fVw2unhkrtToSYQqHtHgfZ2cbE")
        .await
        .expect("status should succeed");

    assert_eq!(txs.len(), 2);
    assert_eq!(txs[0].status, BridgeTransactionStatus::DepositDetected);
    assert_eq!(txs[1].status, BridgeTransactionStatus::Completed);
    assert_eq!(txs[1].created_time_ms, Some(1757531217339));

    let reqs = captured.lock().await;
    assert_eq!(reqs[0].method, "GET");
    assert_eq!(
        reqs[0].path,
        "/status/EXoZue2avJae1d45B3fVw2unhkrtToSYQqHtHgfZ2cbE"
    );
}

#[tokio::test]
async fn get_quote_posts_and_parses_response() {
    let (base_url, captured) = spawn_mock(|_method, _path| {
        let body = r#"{
            "estCheckoutTimeMs": 25000,
            "estOutputUsd": 14.488305,
            "estToTokenBaseUnit": "14491203",
            "quoteId": "0xdeadbeef"
        }"#;
        (200, body.to_string())
    })
    .await;

    let client = BridgeClient::builder().base_url(base_url).build();
    let quote = client
        .get_quote(&QuoteRequest {
            from_amount_base_unit: "10000000".to_string(),
            from_chain_id: "137".to_string(),
            from_token_address: "0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359".to_string(),
            recipient_address: "0x17eC161f126e82A8ba337f4022d574DBEaFef575".to_string(),
            to_chain_id: "137".to_string(),
            to_token_address: "0xC011a7E12a19f7B1f670d46F03B03f3342E82DFB".to_string(),
        })
        .await
        .expect("quote should succeed");

    assert_eq!(quote.est_checkout_time_ms, Some(25000));
    assert_eq!(quote.quote_id.as_deref(), Some("0xdeadbeef"));

    let reqs = captured.lock().await;
    assert_eq!(reqs[0].method, "POST");
    assert_eq!(reqs[0].path, "/quote");
}

#[tokio::test]
async fn create_withdrawal_addresses_posts_withdraw() {
    let (base_url, captured) = spawn_mock(|_method, _path| {
        let body = r#"{
            "address": { "evm": "0x23566f8b2E82aDfCf01846E54899d110e97AC053" },
            "note": "Send funds to these addresses."
        }"#;
        (201, body.to_string())
    })
    .await;

    let client = BridgeClient::builder().base_url(base_url).build();
    let resp = client
        .create_withdrawal_addresses(&WithdrawalRequest {
            address: "0x9156dd10bea4c8d7e2d591b633d1694b1d764756".to_string(),
            to_chain_id: "1".to_string(),
            to_token_address: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".to_string(),
            recipient_addr: "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045".to_string(),
        })
        .await
        .expect("withdraw should succeed");

    assert!(resp.address.evm.is_some());

    let reqs = captured.lock().await;
    assert_eq!(reqs[0].method, "POST");
    assert_eq!(reqs[0].path, "/withdraw");
}

#[tokio::test]
async fn error_status_surfaces_clob_error_with_body() {
    let (base_url, _captured) = spawn_mock(|_method, _path| {
        (400, r#"{"error":"address is required"}"#.to_string())
    })
    .await;

    let client = BridgeClient::builder().base_url(base_url).build();
    let err = client
        .get_transaction_status("")
        .await
        .expect_err("empty address should fail");

    match err {
        ClobError::Api(api_err) => {
            // The raw error body is preserved for the caller.
            assert!(api_err.to_string().contains("address is required"));
        }
        other => panic!("expected ApiError, got {:?}", other),
    }
}
