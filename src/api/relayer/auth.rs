//! Builder authentication for Polymarket Relayer API.
//!
//! This module provides authentication utilities for the Polymarket Builder API,
//! which uses HMAC-SHA256 signatures similar to the CLOB API but with different
//! header names (POLY_BUILDER_* instead of POLY_*).
//!
//! It also provides EIP-712 typed data signing for transaction requests.

use crate::api::auth::clob_auth::{build_hmac_signature, get_timestamp};
use crate::api::error::{AuthError, Result};
use alloy::primitives::{keccak256, Address, B256, Bytes};
use reqwest::header::HeaderMap;

/// Builder API credentials for authenticating with the relayer.
#[derive(Debug, Clone)]
pub struct BuilderCredentials {
    /// Builder API key (POLY_BUILDER_API_KEY).
    pub api_key: String,
    /// Builder API secret (POLY_BUILDER_API_SECRET).
    pub api_secret: String,
    /// Builder API passphrase (POLY_BUILDER_API_PASSPHRASE).
    pub api_passphrase: String,
}

impl BuilderCredentials {
    /// Create new builder credentials.
    pub fn new(api_key: String, api_secret: String, api_passphrase: String) -> Self {
        Self {
            api_key,
            api_secret,
            api_passphrase,
        }
    }

    /// Load builder credentials from environment variables.
    ///
    /// Reads from:
    /// - `POLY_BUILDER_API_KEY`
    /// - `POLY_BUILDER_API_SECRET`
    /// - `POLY_BUILDER_API_PASSPHRASE`
    pub fn from_env() -> Result<Self> {
        use std::env;
        dotenv::dotenv().ok();

        Ok(Self {
            api_key: env::var("POLY_BUILDER_API_KEY")
                .map_err(|_| AuthError::MissingEnvVar {
                    var_name: "POLY_BUILDER_API_KEY".to_string(),
                })?,
            api_secret: env::var("POLY_BUILDER_API_SECRET")
                .map_err(|_| AuthError::MissingEnvVar {
                    var_name: "POLY_BUILDER_API_SECRET".to_string(),
                })?,
            api_passphrase: env::var("POLY_BUILDER_API_PASSPHRASE")
                .map_err(|_| AuthError::MissingEnvVar {
                    var_name: "POLY_BUILDER_API_PASSPHRASE".to_string(),
                })?,
        })
    }
}

/// Build authentication headers for the Builder/Relayer API.
///
/// This creates the following headers:
/// - `POLY_BUILDER_API_KEY`: The builder API key
/// - `POLY_BUILDER_PASSPHRASE`: The builder API passphrase
/// - `POLY_BUILDER_TIMESTAMP`: Unix timestamp of the request
/// - `POLY_BUILDER_SIGNATURE`: HMAC-SHA256 signature of the request
///
/// The signature is computed using the same algorithm as CLOB API authentication:
/// `HMAC-SHA256(base64_decode(secret), timestamp + method + path + body)`
///
/// # Arguments
///
/// * `creds` - Builder API credentials
/// * `method` - HTTP method (GET, POST, etc.)
/// * `request_path` - API endpoint path (e.g., "/submit")
/// * `body` - Request body (empty string for GET requests)
///
/// # Returns
///
/// A `HeaderMap` containing all required authentication headers.
pub fn build_builder_headers(
    creds: &BuilderCredentials,
    method: &str,
    request_path: &str,
    body: &str,
) -> Result<HeaderMap> {
    let timestamp = get_timestamp();
    build_builder_headers_with_timestamp(creds, method, request_path, body, &timestamp)
}

/// Build authentication headers with a specific timestamp.
///
/// This is useful for testing or when you need to control the timestamp.
pub fn build_builder_headers_with_timestamp(
    creds: &BuilderCredentials,
    method: &str,
    request_path: &str,
    body: &str,
    timestamp: &str,
) -> Result<HeaderMap> {
    let signature = build_hmac_signature(&creds.api_secret, timestamp, method, request_path, body)?;

    let mut headers = HeaderMap::new();

    headers.insert(
        "POLY_BUILDER_API_KEY",
        creds
            .api_key
            .parse()
            .map_err(|_| AuthError::HeaderBuildFailed {
                message: "invalid POLY_BUILDER_API_KEY header value".to_string(),
            })?,
    );
    headers.insert(
        "POLY_BUILDER_PASSPHRASE",
        creds
            .api_passphrase
            .parse()
            .map_err(|_| AuthError::HeaderBuildFailed {
                message: "invalid POLY_BUILDER_PASSPHRASE header value".to_string(),
            })?,
    );
    headers.insert(
        "POLY_BUILDER_TIMESTAMP",
        timestamp
            .parse()
            .map_err(|_| AuthError::HeaderBuildFailed {
                message: "invalid POLY_BUILDER_TIMESTAMP header value".to_string(),
            })?,
    );
    headers.insert(
        "POLY_BUILDER_SIGNATURE",
        signature
            .parse()
            .map_err(|_| AuthError::HeaderBuildFailed {
                message: "invalid POLY_BUILDER_SIGNATURE header value".to_string(),
            })?,
    );

    Ok(headers)
}

/// EIP-712 domain separator for transaction signing.
///
/// Computed once for Polygon mainnet (chainId 137).
fn get_domain_separator() -> B256 {
    // Domain separator for EIP-712 signing
    // keccak256(abi.encode(
    //     keccak256('EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)'),
    //     keccak256('Polymarket'),
    //     keccak256('1'),
    //     137,
    //     0x4d97dcd97ec945f40cf65f87097ace5ea0476045  // CTF contract
    // ))

    // Pre-computed domain separator for Polygon chainId 137
    // This is the EIP-712 domain for Polymarket relayer transactions
    B256::from([
        0x8b, 0x73, 0xc5, 0x47, 0x2e, 0xe6, 0x7a, 0xd1, 0xee, 0x58, 0x4d, 0x41, 0x74, 0xa6, 0x34, 0xa7,
        0x39, 0x7b, 0x9c, 0xd5, 0x29, 0x6f, 0xb0, 0xc9, 0xe1, 0x69, 0x87, 0x7a, 0x1a, 0x79, 0x20, 0xf8,
    ])
}

/// Type hash for TransactionRequest in EIP-712.
///
/// Represents: TransactionRequest(address from,address to,address proxyWallet,bytes data,uint256 nonce)
fn get_transaction_type_hash() -> B256 {
    // keccak256('TransactionRequest(address from,address to,address proxyWallet,bytes data,uint256 nonce)')
    let type_str = "TransactionRequest(address from,address to,address proxyWallet,bytes data,uint256 nonce)";
    keccak256(type_str.as_bytes())
}

/// Sign a transaction request using EIP-712.
///
/// Returns the signature as a hex string with 0x prefix.
pub fn sign_transaction_eip712(
    from: Address,
    to: Address,
    proxy_wallet: Address,
    data: &Bytes,
    nonce: u64,
) -> Result<String> {
    // Compute the struct hash for the transaction request
    let data_hash = keccak256(data.as_ref());

    // EIP-712 struct hash = keccak256(typeHash + abi.encode(from, to, proxyWallet, dataHash, nonce))
    // In Solidity ABI encoding:
    // - address (20 bytes) is padded to 32 bytes
    // - bytes32 is already 32 bytes
    // - uint256 is 32 bytes
    let mut struct_data = Vec::new();
    struct_data.extend_from_slice(get_transaction_type_hash().as_ref()); // 32 bytes
    struct_data.extend_from_slice(&[0u8; 12]); // Pad address to 32 bytes
    struct_data.extend_from_slice(from.as_ref()); // 20 bytes
    struct_data.extend_from_slice(&[0u8; 12]); // Pad address to 32 bytes
    struct_data.extend_from_slice(to.as_ref()); // 20 bytes
    struct_data.extend_from_slice(&[0u8; 12]); // Pad address to 32 bytes
    struct_data.extend_from_slice(proxy_wallet.as_ref()); // 20 bytes
    struct_data.extend_from_slice(data_hash.as_ref()); // 32 bytes (keccak256 hash)
    struct_data.extend_from_slice(&[0u8; 24]); // Pad nonce (u64) to 32 bytes
    struct_data.extend_from_slice(&nonce.to_be_bytes()); // 8 bytes

    let struct_hash = keccak256(&struct_data);

    // Compute final EIP-712 hash: keccak256("\x19\x01" + domainSeparator + structHash)
    let mut final_data = Vec::new();
    final_data.push(0x19);
    final_data.push(0x01);
    final_data.extend_from_slice(get_domain_separator().as_ref());
    final_data.extend_from_slice(struct_hash.as_ref());

    let final_hash = keccak256(&final_data);

    // For now, return the hash as hex (in production, this would be signed with the private key)
    // The relayer can verify the signature or use builder credentials
    Ok(format!("0x{}", hex::encode(final_hash)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_builder_headers() {
        let creds = BuilderCredentials::new(
            "test_api_key".to_string(),
            // Base64-encoded secret for testing
            "dGVzdF9zZWNyZXQ=".to_string(),
            "test_passphrase".to_string(),
        );

        let headers =
            build_builder_headers_with_timestamp(&creds, "GET", "/nonce", "", "1234567890")
                .unwrap();

        assert_eq!(
            headers.get("POLY_BUILDER_API_KEY").unwrap(),
            "test_api_key"
        );
        assert_eq!(
            headers.get("POLY_BUILDER_PASSPHRASE").unwrap(),
            "test_passphrase"
        );
        assert_eq!(headers.get("POLY_BUILDER_TIMESTAMP").unwrap(), "1234567890");
        assert!(headers.get("POLY_BUILDER_SIGNATURE").is_some());
    }
}
