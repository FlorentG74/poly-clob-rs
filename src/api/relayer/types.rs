//! Relayer client types for Polymarket Builder API transactions.
//!
//! This module defines the core types used for interacting with the Polymarket
//! relayer infrastructure, including wallet signature types, transaction structures,
//! and response models.

use alloy::primitives::{Address, Bytes, U256};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Wallet/Signature type that determines how transactions are signed and submitted.
///
/// This corresponds to the `signature_type` parameter used by Polymarket's order
/// and relayer APIs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[repr(u8)]
pub enum SignatureType {
    /// Standard Ethereum wallet (MetaMask, hardware wallet, etc.).
    /// The funder is the EOA address and will need POL to pay gas on transactions.
    Eoa = 0,

    /// Custom proxy wallet for Magic Link email/Google login users.
    /// This requires the user to have exported their private key from Polymarket.com
    /// and imported it into your application.
    #[default]
    PolyProxy = 1,

    /// Gnosis Safe multisig proxy wallet (most common).
    /// Use this for any new or returning user who does not fit the other two types.
    /// This is the standard wallet type for users who signed up via Polymarket.com.
    GnosisSafe = 2,
}

impl SignatureType {
    /// Returns the numeric value of the signature type.
    pub fn as_u8(&self) -> u8 {
        *self as u8
    }
}

impl FromStr for SignatureType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "EOA" | "0" => Ok(SignatureType::Eoa),
            "POLY_PROXY" | "1" => Ok(SignatureType::PolyProxy),
            "GNOSIS_SAFE" | "2" => Ok(SignatureType::GnosisSafe),
            _ => Err(anyhow::anyhow!("Invalid signature type: '{}'. Expected one of: EOA, POLY_PROXY, GNOSIS_SAFE, 0, 1, 2", s)),
        }
    }
}

impl std::fmt::Display for SignatureType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SignatureType::Eoa => write!(f, "EOA"),
            SignatureType::PolyProxy => write!(f, "POLY_PROXY"),
            SignatureType::GnosisSafe => write!(f, "GNOSIS_SAFE"),
        }
    }
}

/// Transaction operation type for Safe transactions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum OperationType {
    /// Standard call operation.
    #[default]
    Call = 0,
    /// Delegate call operation (executes code in the context of the caller).
    DelegateCall = 1,
}

impl Serialize for OperationType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u8(*self as u8)
    }
}

impl<'de> Deserialize<'de> for OperationType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, Visitor};
        use std::fmt;

        struct OperationTypeVisitor;

        impl<'de> Visitor<'de> for OperationTypeVisitor {
            type Value = OperationType;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("0 or 1")
            }

            fn visit_u64<E>(self, value: u64) -> Result<OperationType, E>
            where
                E: de::Error,
            {
                match value {
                    0 => Ok(OperationType::Call),
                    1 => Ok(OperationType::DelegateCall),
                    _ => Err(de::Error::custom("invalid operation type")),
                }
            }
        }

        deserializer.deserialize_u8(OperationTypeVisitor)
    }
}

/// Call type for Proxy transactions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum CallType {
    #[default]
    Call,
    DelegateCall,
}

/// Relayer transaction type - determines which wallet infrastructure to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum RelayerTxType {
    /// Gnosis Safe wallet transactions.
    #[default]
    Safe,
    /// Polymarket Proxy wallet transactions.
    Proxy,
}

/// Transaction state in the relayer pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RelayerTransactionState {
    /// Transaction is newly submitted.
    New,
    /// Transaction has been executed by the relayer.
    Executed,
    /// Transaction has been mined on-chain.
    Mined,
    /// Transaction is invalid.
    Invalid,
    /// Transaction has been confirmed with sufficient block confirmations.
    Confirmed,
    /// Transaction failed.
    Failed,
}

/// A basic transaction structure with target, data, and value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// Target contract address.
    pub to: Address,
    /// Encoded function call data.
    pub data: Bytes,
    /// ETH value to send (usually 0 for Polymarket operations).
    pub value: U256,
}

/// A Safe transaction with operation type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SafeTransaction {
    /// Target contract address.
    pub to: Address,
    /// Operation type (Call or DelegateCall).
    pub operation: OperationType,
    /// Encoded function call data.
    pub data: Bytes,
    /// ETH value to send.
    pub value: U256,
}

impl From<Transaction> for SafeTransaction {
    fn from(tx: Transaction) -> Self {
        SafeTransaction {
            to: tx.to,
            operation: OperationType::Call,
            data: tx.data,
            value: tx.value,
        }
    }
}

/// A Proxy transaction with call type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyTransaction {
    /// Call type (Call or DelegateCall).
    pub type_code: CallType,
    /// Target contract address.
    pub to: Address,
    /// Encoded function call data.
    pub data: Bytes,
    /// ETH value to send.
    pub value: U256,
}

impl From<Transaction> for ProxyTransaction {
    fn from(tx: Transaction) -> Self {
        ProxyTransaction {
            type_code: CallType::Call,
            to: tx.to,
            data: tx.data,
            value: tx.value,
        }
    }
}

/// Arguments for submitting Safe transactions to the relayer.
/// This is the raw transaction data, not the API request format.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SafeTransactionArgs {
    /// Sender (Safe owner) address.
    pub sender: Address,
    /// Transaction nonce.
    pub nonce: u64,
    /// Chain ID (137 for Polygon mainnet).
    pub chain_id: u64,
    /// List of transactions to execute.
    pub transactions: Vec<SafeTransaction>,
}

/// Request payload for submitting transactions to the relayer.
/// This is the format expected by the Polymarket relayer API.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionSubmitRequest {
    /// Sender (from address).
    pub from: Address,
    /// Transaction destination.
    pub to: Address,
    /// Safe/Proxy wallet address.
    pub proxy_wallet: Address,
    /// Encoded transaction data.
    pub data: Bytes,
    /// Transaction nonce.
    pub nonce: u64,
    /// EIP-712 signature (optional - not needed for POLY_PROXY with Builder API auth).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    /// Transaction type.
    #[serde(rename = "type")]
    pub tx_type: String,
    /// Signature parameters (gas config, etc).
    pub signature_params: SignatureParamsRequest,
    /// Optional metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<String>,
}

/// Signature parameters for transaction submission.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignatureParamsRequest {
    /// Gas price in wei.
    pub gas_price: String,
    /// Safe transaction gas.
    pub safe_txn_gas: String,
    /// Base gas cost.
    pub base_gas: String,
    /// Gas token address.
    pub gas_token: Address,
    /// Refund receiver address.
    pub refund_receiver: Address,
}

/// Arguments for submitting Proxy transactions to the relayer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyTransactionArgs {
    /// Sender address.
    pub sender: Address,
    /// Transaction nonce.
    pub nonce: u64,
    /// Gas price in wei.
    pub gas_price: U256,
    /// Optional gas limit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_limit: Option<u64>,
    /// Encoded transaction data.
    pub data: Bytes,
    /// Relayer address.
    pub relayer: Address,
}

/// Response from the relayer after submitting a transaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelayerTransactionResponse {
    /// Unique transaction ID assigned by the relayer.
    pub transaction_id: String,
    /// Current state of the transaction.
    pub state: RelayerTransactionState,
    /// Transaction hash (if mined).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
}

/// Full transaction details from the relayer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelayerTransaction {
    /// Unique transaction ID.
    pub transaction_id: String,
    /// Transaction hash (if mined).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    /// Sender address.
    pub sender: Address,
    /// Recipient/target address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recipient: Option<Address>,
    /// Proxy/Safe address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy_address: Option<Address>,
    /// Encoded transaction data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Bytes>,
    /// Transaction nonce.
    pub nonce: u64,
    /// ETH value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<U256>,
    /// Current transaction state.
    pub state: RelayerTransactionState,
    /// Transaction type (SAFE or SAFE-CREATE).
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_type: Option<String>,
    /// Additional metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    /// Creation timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    /// Last update timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

/// Nonce response from the relayer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NonceResponse {
    /// Current nonce value (returned as string by API).
    #[serde(deserialize_with = "deserialize_string_to_u64")]
    pub nonce: u64,
}

/// Deserialize a string or number to u64.
fn deserialize_string_to_u64<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Visitor};
    use std::fmt;

    struct StringOrU64Visitor;

    impl<'de> Visitor<'de> for StringOrU64Visitor {
        type Value = u64;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string or u64")
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(value)
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            value.parse().map_err(de::Error::custom)
        }
    }

    deserializer.deserialize_any(StringOrU64Visitor)
}

/// Deployed status response from the relayer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployedResponse {
    /// Whether the Safe/Proxy is deployed.
    pub deployed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signature_type_from_str() {
        assert_eq!(SignatureType::from_str("EOA").unwrap(), SignatureType::Eoa);
        assert_eq!(SignatureType::from_str("0").unwrap(), SignatureType::Eoa);
        assert_eq!(
            SignatureType::from_str("POLY_PROXY").unwrap(),
            SignatureType::PolyProxy
        );
        assert_eq!(
            SignatureType::from_str("1").unwrap(),
            SignatureType::PolyProxy
        );
        assert_eq!(
            SignatureType::from_str("GNOSIS_SAFE").unwrap(),
            SignatureType::GnosisSafe
        );
        assert_eq!(
            SignatureType::from_str("2").unwrap(),
            SignatureType::GnosisSafe
        );
        assert!(SignatureType::from_str("INVALID").is_err());
    }

    #[test]
    fn test_signature_type_display() {
        assert_eq!(SignatureType::Eoa.to_string(), "EOA");
        assert_eq!(SignatureType::PolyProxy.to_string(), "POLY_PROXY");
        assert_eq!(SignatureType::GnosisSafe.to_string(), "GNOSIS_SAFE");
    }

    #[test]
    fn test_signature_type_as_u8() {
        assert_eq!(SignatureType::Eoa.as_u8(), 0);
        assert_eq!(SignatureType::PolyProxy.as_u8(), 1);
        assert_eq!(SignatureType::GnosisSafe.as_u8(), 2);
    }
}
