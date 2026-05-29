//! Builder authentication for Polymarket Relayer API.
//!
//! This module provides authentication utilities for the Polymarket Builder API,
//! which uses HMAC-SHA256 signatures similar to the CLOB API but with different
//! header names (POLY_BUILDER_* instead of POLY_*).
//!
//! It also provides Gnosis Safe EIP-712 transaction signing and
//! CREATE2-based Safe address derivation.

use crate::api::auth::clob_auth::{build_hmac_signature, get_timestamp};
use crate::api::error::{AuthError, Result};
use alloy::primitives::{keccak256, Address, Bytes, U256};
use alloy::signers::local::PrivateKeySigner;
use alloy::signers::Signer as AlloySigner;
use reqwest::header::HeaderMap;

use super::transactions::contracts;

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
        dotenvy::dotenv().ok();

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

/// Derive the Gnosis Safe proxy address for an EOA using CREATE2.
///
/// Matches the TypeScript `deriveProxyAddress` from `builder-relayer-client`:
/// - Factory: `SAFE_FACTORY`
/// - Salt: `keccak256(abi.encode(eoa_address))`
/// - Init code hash: `SAFE_INIT_CODE_HASH`
///
/// CREATE2 formula: `keccak256(0xff ++ factory ++ salt ++ init_code_hash)[12..]`
pub fn derive_safe_address(eoa: &Address) -> Address {
    let factory: Address = contracts::SAFE_FACTORY.parse().expect("valid SAFE_FACTORY address");

    // salt = keccak256(abi.encode(eoa_address))
    // abi.encode for an address is left-padded to 32 bytes
    let mut salt_input = [0u8; 32];
    salt_input[12..].copy_from_slice(eoa.as_slice());
    let salt = keccak256(salt_input);

    // CREATE2: keccak256(0xff ++ factory ++ salt ++ init_code_hash)
    let mut create2_input = Vec::with_capacity(1 + 20 + 32 + 32);
    create2_input.push(0xff);
    create2_input.extend_from_slice(factory.as_slice());
    create2_input.extend_from_slice(salt.as_ref());
    create2_input.extend_from_slice(&contracts::SAFE_INIT_CODE_HASH);

    let hash = keccak256(&create2_input);
    // Take last 20 bytes as the address
    Address::from_slice(&hash[12..])
}

/// Sign a Gnosis Safe transaction using EIP-712.
///
/// Matches the TypeScript `buildSafeTransactionRequest` from `builder-relayer-client`:
/// 1. Compute EIP-712 struct hash for `SafeTx` type
/// 2. Compute domain hash (no name/version, just chainId + verifyingContract)
/// 3. Compute final EIP-712 hash
/// 4. Sign with eth_sign (adds Ethereum prefix)
/// 5. Adjust v-value for Gnosis Safe eth_sign mode (v += 4)
///
/// Returns the packed signature as a hex string with 0x prefix.
pub async fn sign_safe_transaction(
    signer: &PrivateKeySigner,
    safe_address: &Address,
    chain_id: u64,
    to: &Address,
    data: &Bytes,
    operation: u8,
    nonce: u64,
) -> Result<String> {
    // SafeTx type hash
    let safe_tx_type_hash = keccak256(
        "SafeTx(address to,uint256 value,bytes data,uint8 operation,uint256 safeTxGas,uint256 baseGas,uint256 gasPrice,address gasToken,address refundReceiver,uint256 nonce)"
            .as_bytes(),
    );

    // Encode struct data: typeHash + abi.encode(to, value, keccak256(data), operation, safeTxGas, baseGas, gasPrice, gasToken, refundReceiver, nonce)
    let data_hash = keccak256(data.as_ref());

    let mut struct_data = Vec::with_capacity(32 * 11);
    struct_data.extend_from_slice(safe_tx_type_hash.as_ref());   // typeHash
    // to (address, left-padded to 32 bytes)
    struct_data.extend_from_slice(&[0u8; 12]);
    struct_data.extend_from_slice(to.as_slice());
    // value = 0
    struct_data.extend_from_slice(&U256::ZERO.to_be_bytes::<32>());
    // keccak256(data)
    struct_data.extend_from_slice(data_hash.as_ref());
    // operation (uint8 as uint256)
    struct_data.extend_from_slice(&U256::from(operation).to_be_bytes::<32>());
    // safeTxGas = 0
    struct_data.extend_from_slice(&U256::ZERO.to_be_bytes::<32>());
    // baseGas = 0
    struct_data.extend_from_slice(&U256::ZERO.to_be_bytes::<32>());
    // gasPrice = 0
    struct_data.extend_from_slice(&U256::ZERO.to_be_bytes::<32>());
    // gasToken = address(0)
    struct_data.extend_from_slice(&[0u8; 32]);
    // refundReceiver = address(0)
    struct_data.extend_from_slice(&[0u8; 32]);
    // nonce
    struct_data.extend_from_slice(&U256::from(nonce).to_be_bytes::<32>());

    let struct_hash = keccak256(&struct_data);

    // Domain separator: keccak256(abi.encode(keccak256("EIP712Domain(uint256 chainId,address verifyingContract)"), chainId, safeAddress))
    // Note: NO name/version — matches TypeScript implementation
    let domain_type_hash = keccak256("EIP712Domain(uint256 chainId,address verifyingContract)".as_bytes());
    let mut domain_data = Vec::with_capacity(32 * 3);
    domain_data.extend_from_slice(domain_type_hash.as_ref());
    domain_data.extend_from_slice(&U256::from(chain_id).to_be_bytes::<32>());
    domain_data.extend_from_slice(&[0u8; 12]);
    domain_data.extend_from_slice(safe_address.as_slice());
    let domain_separator = keccak256(&domain_data);

    // EIP-712 hash: keccak256("\x19\x01" + domainSeparator + structHash)
    let mut eip712_data = Vec::with_capacity(2 + 32 + 32);
    eip712_data.push(0x19);
    eip712_data.push(0x01);
    eip712_data.extend_from_slice(domain_separator.as_ref());
    eip712_data.extend_from_slice(struct_hash.as_ref());
    let eip712_hash = keccak256(&eip712_data);

    // Sign using eth_sign mode (sign_message adds the Ethereum prefix)
    // This matches the TypeScript: signer.signMessage(structHash)
    // where structHash is the raw EIP-712 hash bytes
    let signature = signer
        .sign_message(eip712_hash.as_ref())
        .await
        .map_err(|e| AuthError::SignatureFailed {
            message: format!("failed to sign Safe transaction: {}", e),
        })?;

    // Get r, s, v components
    let sig_bytes = signature.as_bytes();
    let r = &sig_bytes[..32];
    let s = &sig_bytes[32..64];
    let mut v = sig_bytes[64];

    // Adjust v for Gnosis Safe eth_sign mode:
    // Standard v values (0,1 or 27,28) need to be adjusted to (31,32)
    // v=0|1 -> v+31, v=27|28 -> v+4
    if v <= 1 {
        v += 31;
    } else if (27..=28).contains(&v) {
        v += 4;
    }

    // Pack as r (32 bytes) + s (32 bytes) + v (1 byte)
    let mut packed = Vec::with_capacity(65);
    packed.extend_from_slice(r);
    packed.extend_from_slice(s);
    packed.push(v);

    Ok(format!("0x{}", hex::encode(packed)))
}

/// Derive the Polymarket Proxy wallet address for an EOA using CREATE2.
///
/// Uses the Proxy factory with packed (not ABI-encoded) salt.
/// - Factory: `PROXY_FACTORY`
/// - Salt: `keccak256(encodePacked(eoa_address))` (20 raw bytes, no padding)
/// - Init code hash: `PROXY_INIT_CODE_HASH`
pub fn derive_proxy_address(eoa: &Address) -> Address {
    let factory: Address = contracts::PROXY_FACTORY.parse().expect("valid PROXY_FACTORY address");

    // salt = keccak256(encodePacked(address)) — 20 raw bytes, NOT left-padded
    let salt = keccak256(eoa.as_slice());

    // CREATE2: keccak256(0xff ++ factory ++ salt ++ init_code_hash)
    let mut create2_input = Vec::with_capacity(1 + 20 + 32 + 32);
    create2_input.push(0xff);
    create2_input.extend_from_slice(factory.as_slice());
    create2_input.extend_from_slice(salt.as_ref());
    create2_input.extend_from_slice(&contracts::PROXY_INIT_CODE_HASH);

    let hash = keccak256(&create2_input);
    Address::from_slice(&hash[12..])
}

/// Sign a Polymarket Proxy transaction.
///
/// The struct hash is: `keccak256(concat([
///   "rlx:",
///   from, to, data,
///   txFee (32 bytes), gasPrice (32 bytes), gasLimit (32 bytes), nonce (32 bytes),
///   relayHubAddress, relayAddress
/// ]))`
///
/// Then signed with eth_sign mode (`signMessage` adds Ethereum prefix).
/// Returns the packed signature as hex with 0x prefix (r + s + v, no v adjustment).
#[allow(clippy::too_many_arguments)]
pub async fn sign_proxy_transaction(
    signer: &PrivateKeySigner,
    from: &Address,
    to: &Address,
    data: &Bytes,
    gas_limit: u64,
    nonce: u64,
    relay_hub: &Address,
    relay_address: &Address,
) -> Result<String> {
    // Build the struct hash by concatenating fields
    let prefix = b"rlx:";

    let mut hash_input = Vec::new();
    hash_input.extend_from_slice(prefix);
    hash_input.extend_from_slice(from.as_slice());       // 20 bytes (raw address)
    hash_input.extend_from_slice(to.as_slice());         // 20 bytes
    hash_input.extend_from_slice(data.as_ref());         // variable length
    // txFee = 0 (32 bytes big-endian)
    hash_input.extend_from_slice(&U256::ZERO.to_be_bytes::<32>());
    // gasPrice = 0 (32 bytes big-endian)
    hash_input.extend_from_slice(&U256::ZERO.to_be_bytes::<32>());
    // gasLimit (32 bytes big-endian)
    hash_input.extend_from_slice(&U256::from(gas_limit).to_be_bytes::<32>());
    // nonce (32 bytes big-endian)
    hash_input.extend_from_slice(&U256::from(nonce).to_be_bytes::<32>());
    hash_input.extend_from_slice(relay_hub.as_slice());  // 20 bytes
    hash_input.extend_from_slice(relay_address.as_slice()); // 20 bytes

    let struct_hash = keccak256(&hash_input);

    // Sign with eth_sign mode (signMessage adds Ethereum prefix)
    let signature = signer
        .sign_message(struct_hash.as_ref())
        .await
        .map_err(|e| AuthError::SignatureFailed {
            message: format!("failed to sign proxy transaction: {}", e),
        })?;

    // Pack as r + s + v (no v adjustment for proxy, unlike Safe)
    let sig_bytes = signature.as_bytes();
    Ok(format!("0x{}", hex::encode(sig_bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

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

    #[test]
    fn test_derive_addresses_for_actual_eoa() {
        let eoa: Address = "0xCdD5dACeDfDD8e4571b6475F729699a52C1B2E97".parse().unwrap();
        let safe = derive_safe_address(&eoa);
        let proxy = derive_proxy_address(&eoa);
        println!("EOA: {}", eoa);
        println!("Derived Safe: {}", safe);
        println!("Derived Proxy: {}", proxy);
    }

    #[test]
    fn test_derive_safe_address_deterministic() {
        // Verify that derive_safe_address produces consistent results
        let eoa: Address = "0x1234567890123456789012345678901234567890".parse().unwrap();
        let safe1 = derive_safe_address(&eoa);
        let safe2 = derive_safe_address(&eoa);
        assert_eq!(safe1, safe2, "derive_safe_address must be deterministic");
        // The derived address should be different from the EOA
        assert_ne!(safe1, eoa, "Safe address must differ from EOA");
    }

    /// Known-answer test: proxy signature matches the TypeScript builder-relayer-client
    /// test vector from tests/signatures/index.test.ts
    #[tokio::test]
    async fn test_sign_proxy_transaction_known_answer() {
        use crate::api::relayer::transactions::{encode_proxy_call_data, contracts};
        use super::super::types::Transaction;
        use alloy::primitives::U256;

        let signer = PrivateKeySigner::from_str(
            "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
        ).unwrap();
        let from = signer.address();

        let proxy_factory: Address = contracts::PROXY_FACTORY.parse().unwrap();
        let relay_hub: Address = contracts::RELAY_HUB.parse().unwrap();
        let relay_addr: Address = "0xae700edfd9ab986395f3999fe11177b9903a52f1".parse().unwrap();

        // approve(CTF, uint256_max) on USDC — same as in the TypeScript test vector
        let usdc: Address = "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174".parse().unwrap();
        let approve_calldata = hex::decode(
            "095ea7b30000000000000000000000004d97dcd97ec945f40cf65f87097ace5ea0476045\
             ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
        ).unwrap();

        let inner_tx = Transaction {
            to: usdc,
            data: Bytes::from(approve_calldata),
            value: U256::ZERO,
        };
        let proxy_data = encode_proxy_call_data(&[inner_tx]);

        let sig = sign_proxy_transaction(
            &signer, &from, &proxy_factory, &proxy_data,
            85338, // gasLimit from test vector
            0,     // nonce from test vector
            &relay_hub, &relay_addr,
        ).await.unwrap();

        assert_eq!(
            sig,
            "0x4c18e2d2294a00d686714aff8e7936ab657cb4655dfccb2b556efadcb7e835f8\
             00dc2fecec69c501e29bb36ecb54b4da6b7c410c4dc740a33af2afde2b77297e1b",
            "Proxy signature must match TypeScript reference"
        );
    }

    #[tokio::test]
    async fn test_sign_safe_transaction() {
        // Use a test private key (DO NOT use in production)
        let signer = PrivateKeySigner::from_str(
            "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
        ).unwrap();

        let safe_addr: Address = "0x1234567890123456789012345678901234567890".parse().unwrap();
        let to: Address = "0x4d97dcd97ec945f40cf65f87097ace5ea0476045".parse().unwrap();
        let data = Bytes::from(vec![0x31, 0x1d, 0x8a, 0x8e]);

        let sig = sign_safe_transaction(&signer, &safe_addr, 137, &to, &data, 0, 0)
            .await
            .unwrap();

        // Verify signature format
        assert!(sig.starts_with("0x"), "Signature must start with 0x");
        assert_eq!(sig.len(), 132, "Signature must be 65 bytes (130 hex chars + 0x prefix)");

        // Verify v-value is adjusted for Safe (should be 31 or 32)
        let v = u8::from_str_radix(&sig[130..132], 16).unwrap();
        assert!(v == 31 || v == 32, "v-value must be 31 or 32 for Safe eth_sign, got {}", v);
    }
}
