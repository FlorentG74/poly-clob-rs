//! Bridge API request and response types.
//!
//! These types mirror the Polymarket Bridge API schemas (see the bridge
//! `OpenAPI` spec). All wire structs use `#[serde(rename_all = "camelCase")]`
//! to match the JSON field naming used by the API.

use serde::{Deserialize, Serialize};

/// Request body for `POST /deposit`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositRequest {
    /// Your Polymarket wallet address where deposited funds are credited as pUSD.
    pub address: String,
}

/// Bridge addresses returned for the different blockchain networks.
///
/// Each field is optional because the API only returns address types that are
/// currently supported for the request.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BridgeAddresses {
    /// EVM-compatible bridge address (Ethereum, Polygon, Arbitrum, Base, etc.).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub evm: Option<String>,
    /// Solana Virtual Machine bridge address.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub svm: Option<String>,
    /// Bitcoin bridge address.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub btc: Option<String>,
    /// Tron (TVM) bridge address.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tvm: Option<String>,
}

/// Response from `POST /deposit` and `POST /withdraw`.
///
/// Both endpoints return the same shape: a set of per-network bridge addresses
/// and an optional human-readable note.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositResponse {
    /// Bridge addresses for the different blockchain networks.
    pub address: BridgeAddresses,
    /// Additional information about the bridge addresses.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub note: Option<String>,
}

/// Response from `POST /withdraw` (alias of [`DepositResponse`]).
pub type WithdrawalResponse = DepositResponse;

/// Request body for `POST /withdraw`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WithdrawalRequest {
    /// Source Polymarket wallet address on Polygon.
    pub address: String,
    /// Destination chain ID (e.g. `"1"` Ethereum, `"8453"` Base, `"1151111081099710"` Solana).
    pub to_chain_id: String,
    /// Destination token contract address.
    pub to_token_address: String,
    /// Destination wallet address where funds will be sent.
    pub recipient_addr: String,
}

/// A token description returned within [`SupportedAsset`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    /// Full token name (e.g. "USD Coin").
    pub name: String,
    /// Token symbol (e.g. "USDC").
    pub symbol: String,
    /// Token contract address.
    pub address: String,
    /// Token decimals.
    pub decimals: u8,
}

/// A single supported asset entry from `GET /supported-assets`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SupportedAsset {
    /// Chain ID (string form, e.g. `"1"`).
    pub chain_id: String,
    /// Human-readable chain name (e.g. "Ethereum").
    pub chain_name: String,
    /// The supported token on this chain.
    pub token: Token,
    /// Minimum amount in USD for deposits and withdrawals.
    pub min_checkout_usd: f64,
}

/// Response from `GET /supported-assets`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SupportedAssetsResponse {
    /// List of supported assets with minimum amounts.
    #[serde(default)]
    pub supported_assets: Vec<SupportedAsset>,
}

/// Request body for `POST /quote`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuoteRequest {
    /// Amount of tokens to send, in base units (no decimals).
    pub from_amount_base_unit: String,
    /// Source chain ID.
    pub from_chain_id: String,
    /// Source token address.
    pub from_token_address: String,
    /// Address of the recipient.
    pub recipient_address: String,
    /// Destination chain ID.
    pub to_chain_id: String,
    /// Destination token address.
    pub to_token_address: String,
}

/// Breakdown of the estimated fees returned within [`QuoteResponse`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeeBreakdown {
    /// Label of the app fee.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub app_fee_label: Option<String>,
    /// App fees as a percentage of the total amount sent.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub app_fee_percent: Option<f64>,
    /// App fees in USD.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub app_fee_usd: Option<f64>,
    /// Fill cost percentage of the total amount sent.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub fill_cost_percent: Option<f64>,
    /// Fill cost in USD.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub fill_cost_usd: Option<f64>,
    /// Gas fee in USD.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub gas_usd: Option<f64>,
    /// Maximum potential slippage as a percentage.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub max_slippage: Option<f64>,
    /// Amount after factoring slippage.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub min_received: Option<f64>,
    /// Swap impact as a percentage of the total amount sent.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub swap_impact: Option<f64>,
    /// Swap impact of the transaction in USD.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub swap_impact_usd: Option<f64>,
    /// Total impact as a percentage of the total amount sent.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub total_impact: Option<f64>,
    /// Impact cost of the transaction in USD.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub total_impact_usd: Option<f64>,
}

/// Response from `POST /quote`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuoteResponse {
    /// Estimated time to complete the checkout, in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub est_checkout_time_ms: Option<u64>,
    /// Breakdown of the estimated fees.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub est_fee_breakdown: Option<FeeBreakdown>,
    /// Estimated input amount in USD.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub est_input_usd: Option<f64>,
    /// Estimated output amount in USD.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub est_output_usd: Option<f64>,
    /// Estimated token amount received, in base units.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub est_to_token_base_unit: Option<String>,
    /// Unique quote ID of the request.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub quote_id: Option<String>,
}

/// Status of a bridge transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BridgeTransactionStatus {
    /// Deposit has been detected but not yet processed.
    DepositDetected,
    /// Transaction is being processed.
    Processing,
    /// Origin-chain transaction has been confirmed.
    OriginTxConfirmed,
    /// Transaction has been submitted to the destination chain.
    Submitted,
    /// Transaction completed; funds credited.
    Completed,
    /// Transaction failed.
    Failed,
    /// Unknown / future status not yet modeled by this client.
    #[serde(other)]
    Unknown,
}

/// A single bridge transaction from `GET /status/{address}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeTransaction {
    /// Source chain ID.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub from_chain_id: Option<String>,
    /// Source token contract address.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub from_token_address: Option<String>,
    /// Amount in base units (without decimals).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub from_amount_base_unit: Option<String>,
    /// Destination chain ID.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub to_chain_id: Option<String>,
    /// Destination token contract address.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub to_token_address: Option<String>,
    /// Current status of the transaction.
    pub status: BridgeTransactionStatus,
    /// Transaction hash (only available when status is `COMPLETED`).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tx_hash: Option<String>,
    /// Unix timestamp in milliseconds when the transaction was created
    /// (missing when status is `DEPOSIT_DETECTED`).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub created_time_ms: Option<u64>,
}

/// Response from `GET /status/{address}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionStatusResponse {
    /// List of transactions for the given address.
    #[serde(default)]
    pub transactions: Vec<BridgeTransaction>,
}

/// Error body returned by the Bridge API on failure (`{ "error": "..." }`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeErrorResponse {
    /// Human-readable error message.
    pub error: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deposit_request_serializes_address() {
        let req = DepositRequest {
            address: "0x56687bf447db6ffa42ffe2204a05edaa20f55839".to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert_eq!(
            json,
            r#"{"address":"0x56687bf447db6ffa42ffe2204a05edaa20f55839"}"#
        );
    }

    #[test]
    fn deposit_response_deserializes_partial_addresses() {
        // The API only includes supported address types.
        let body = r#"{
            "address": {
                "evm": "0x23566f8b2E82aDfCf01846E54899d110e97AC053",
                "svm": "CrvTBvzryYxBHbWu2TiQpcqD5M7Le7iBKzVmEj3f36Jb",
                "btc": "bc1q8eau83qffxcj8ht4hsjdza3lha9r3egfqysj3g"
            },
            "note": "Only certain chains and tokens are supported."
        }"#;
        let resp: DepositResponse = serde_json::from_str(body).unwrap();
        assert_eq!(
            resp.address.evm.as_deref(),
            Some("0x23566f8b2E82aDfCf01846E54899d110e97AC053")
        );
        assert_eq!(
            resp.address.svm.as_deref(),
            Some("CrvTBvzryYxBHbWu2TiQpcqD5M7Le7iBKzVmEj3f36Jb")
        );
        assert!(resp.address.tvm.is_none());
        assert!(resp.note.is_some());
    }

    #[test]
    fn withdrawal_request_uses_camel_case() {
        let req = WithdrawalRequest {
            address: "0x9156dd10bea4c8d7e2d591b633d1694b1d764756".to_string(),
            to_chain_id: "1".to_string(),
            to_token_address: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".to_string(),
            recipient_addr: "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045".to_string(),
        };
        let value: serde_json::Value = serde_json::to_value(&req).unwrap();
        assert_eq!(value["toChainId"], "1");
        assert_eq!(value["toTokenAddress"], "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");
        assert_eq!(value["recipientAddr"], "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045");
    }

    #[test]
    fn supported_assets_deserialize() {
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
        let resp: SupportedAssetsResponse = serde_json::from_str(body).unwrap();
        assert_eq!(resp.supported_assets.len(), 1);
        let asset = &resp.supported_assets[0];
        assert_eq!(asset.chain_name, "Ethereum");
        assert_eq!(asset.token.symbol, "USDC");
        assert_eq!(asset.token.decimals, 6);
        assert_eq!(asset.min_checkout_usd, 45.0);
    }

    #[test]
    fn quote_request_uses_camel_case() {
        let req = QuoteRequest {
            from_amount_base_unit: "10000000".to_string(),
            from_chain_id: "137".to_string(),
            from_token_address: "0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359".to_string(),
            recipient_address: "0x17eC161f126e82A8ba337f4022d574DBEaFef575".to_string(),
            to_chain_id: "137".to_string(),
            to_token_address: "0xC011a7E12a19f7B1f670d46F03B03f3342E82DFB".to_string(),
        };
        let value: serde_json::Value = serde_json::to_value(&req).unwrap();
        assert_eq!(value["fromAmountBaseUnit"], "10000000");
        assert_eq!(value["recipientAddress"], "0x17eC161f126e82A8ba337f4022d574DBEaFef575");
    }

    #[test]
    fn quote_response_deserialize() {
        let body = r#"{
            "estCheckoutTimeMs": 25000,
            "estFeeBreakdown": { "appFeeLabel": "Fun.xyz fee", "gasUsd": 0.003854 },
            "estInputUsd": 14.488305,
            "estOutputUsd": 14.488305,
            "estToTokenBaseUnit": "14491203",
            "quoteId": "0x00c34ba467184b0146406d62b0e60aaa24ed52460bd456222b6155a0d9de0ad5"
        }"#;
        let resp: QuoteResponse = serde_json::from_str(body).unwrap();
        assert_eq!(resp.est_checkout_time_ms, Some(25000));
        assert_eq!(resp.est_to_token_base_unit.as_deref(), Some("14491203"));
        let fees = resp.est_fee_breakdown.unwrap();
        assert_eq!(fees.app_fee_label.as_deref(), Some("Fun.xyz fee"));
        assert_eq!(fees.gas_usd, Some(0.003854));
    }

    #[test]
    fn transaction_status_deserialize_all_states() {
        let body = r#"{
            "transactions": [
                {
                    "fromChainId": "1151111081099710",
                    "fromTokenAddress": "11111111111111111111111111111111",
                    "fromAmountBaseUnit": "13566635",
                    "toChainId": "137",
                    "toTokenAddress": "0xC011a7E12a19f7B1f670d46F03B03f3342E82DFB",
                    "status": "DEPOSIT_DETECTED"
                },
                {
                    "fromChainId": "1151111081099710",
                    "fromAmountBaseUnit": "13500152",
                    "toChainId": "137",
                    "txHash": "3atr19NAiNCYt24RHM1WnzZp47RXskpTDzspJoCBBaMFwUB8fk37hFkxz35P5UEnnmWz21rb2t5wJ8pq3EE2XnxU",
                    "createdTimeMs": 1757531217339,
                    "status": "COMPLETED"
                }
            ]
        }"#;
        let resp: TransactionStatusResponse = serde_json::from_str(body).unwrap();
        assert_eq!(resp.transactions.len(), 2);
        assert_eq!(
            resp.transactions[0].status,
            BridgeTransactionStatus::DepositDetected
        );
        assert!(resp.transactions[0].created_time_ms.is_none());
        assert_eq!(
            resp.transactions[1].status,
            BridgeTransactionStatus::Completed
        );
        assert_eq!(resp.transactions[1].created_time_ms, Some(1757531217339));
        assert!(resp.transactions[1].tx_hash.is_some());
    }

    #[test]
    fn transaction_status_unknown_variant_is_tolerated() {
        let body = r#"{"status":"SOME_FUTURE_STATE"}"#;
        let tx: BridgeTransaction = serde_json::from_str(body).unwrap();
        assert_eq!(tx.status, BridgeTransactionStatus::Unknown);
    }

    #[test]
    fn status_round_trips_to_screaming_snake_case() {
        let json = serde_json::to_string(&BridgeTransactionStatus::OriginTxConfirmed).unwrap();
        assert_eq!(json, r#""ORIGIN_TX_CONFIRMED""#);
    }

    #[test]
    fn error_response_deserialize() {
        let body = r#"{"error":"address is required"}"#;
        let err: BridgeErrorResponse = serde_json::from_str(body).unwrap();
        assert_eq!(err.error, "address is required");
    }
}
