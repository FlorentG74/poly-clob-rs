//! Live network integration test for the Bridge API client.
//!
//! Unlike `bridge_test.rs` (which runs fully offline against an in-process mock
//! server), this test hits the real `bridge.polymarket.com` endpoint. It is
//! therefore marked `#[ignore]` so it is skipped by a plain `cargo test`, and
//! must be opted into explicitly:
//!
//! ```bash
//! # Generates and prints deposit addresses for POLY_ADDRESS from .env
//! cargo test -p poly-clob-rs --test bridge_network_test -- --ignored --nocapture
//! ```
//!
//! It reads the wallet address from the `POLY_ADDRESS` environment variable
//! (loaded from the workspace `.env`). If `POLY_ADDRESS` is not set, the test
//! prints a notice and returns successfully rather than failing.

use poly_clob_rs::api::bridge::BridgeClient;

#[tokio::test]
#[ignore = "hits the live bridge.polymarket.com network; run with --ignored --nocapture"]
async fn live_show_deposit_addresses_for_env_wallet() {
    // Load .env from the crate dir or any parent (workspace root holds it).
    let _ = dotenvy::dotenv();

    let address = match std::env::var("POLY_ADDRESS") {
        Ok(addr) if !addr.trim().is_empty() => addr,
        _ => {
            eprintln!("POLY_ADDRESS not set in environment/.env — skipping live test.");
            return;
        }
    };

    let bridge = BridgeClient::default();

    // 1) Generate per-network deposit addresses for the wallet.
    let deposit = bridge
        .create_deposit_addresses(&address)
        .await
        .expect("create_deposit_addresses should succeed against the live API");

    println!("\nDeposit addresses for Polymarket wallet {address}:");
    println!("  (send the supported asset on the matching chain to credit pUSD on Polygon)\n");

    let rows = [
        ("EVM (Ethereum / Polygon / Arbitrum / Base / Optimism)", &deposit.address.evm),
        ("SVM (Solana)", &deposit.address.svm),
        ("BTC (Bitcoin)", &deposit.address.btc),
        ("TVM (Tron)", &deposit.address.tvm),
    ];
    for (label, value) in rows {
        match value {
            Some(v) => println!("  {label:<54} {v}"),
            None => println!("  {label:<54} (not supported)"),
        }
    }
    if let Some(note) = &deposit.note {
        println!("\n  Note: {note}");
    }

    // At least one network address must be returned for a valid wallet.
    let any = deposit.address.evm.is_some()
        || deposit.address.svm.is_some()
        || deposit.address.btc.is_some()
        || deposit.address.tvm.is_some();
    assert!(any, "expected at least one bridge address in the response");

    // 2) Also list the supported assets / minimums for context.
    match bridge.get_supported_assets().await {
        Ok(assets) => {
            println!("\nSupported assets ({}):", assets.len());
            for a in &assets {
                println!(
                    "  {:<10} {:<6} on {:<12} (chainId {:<20} min ${})",
                    a.token.symbol, a.token.name, a.chain_name, a.chain_id, a.min_checkout_usd
                );
            }
        }
        Err(e) => eprintln!("warning: could not fetch supported assets: {e}"),
    }
    println!();
}
