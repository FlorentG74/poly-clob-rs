//! Transaction builders for Polymarket CTF (Conditional Token Framework) operations.
//!
//! This module provides functions to create encoded transaction data for various
//! CTF operations like redeeming positions, converting tokens, etc.

use super::types::Transaction;
use crate::api::error::{Result, ValidationError};
use alloy::primitives::{Address, Bytes, B256, U256};

/// Polygon mainnet contract addresses for Polymarket.
pub mod contracts {
    /// CTF (Conditional Token Framework) contract address on Polygon.
    /// LEGACY redeem path: redeeming here with USDC.e collateral pays out raw
    /// USDC.e, which is NOT Polymarket's current spendable collateral and must be
    /// manually wrapped/deposited in the UI. Use REDEEM_ROUTER + PUSD_COLLATERAL.
    pub const CTF_CONTRACT: &str = "0x4d97dcd97ec945f40cf65f87097ace5ea0476045";

    /// USDC.e (bridged USDC) — the LEGACY collateral. See PUSD_COLLATERAL.
    pub const PUSD_CONTRACT: &str = "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174";

    /// Polymarket redeem router that settles proceeds directly as spendable pUSD.
    /// Confirmed on-chain from a working UI redeem (selector + ABI identical to the
    /// legacy CTF path; only the target contract and collateral differ). Redeeming
    /// here with PUSD_COLLATERAL credits the proxy wallet with usable pUSD — no
    /// manual UI deposit needed.
    pub const REDEEM_ROUTER: &str = "0xada100Db00CA00073811820692005400218fce1F";

    /// pUSD — Polymarket's current spendable collateral token (wraps USDC.e).
    pub const PUSD_COLLATERAL: &str = "0xC011a7E12a19f7B1f670d46F03B03f3342E82DFB";

    /// CTF Exchange contract address (v2).
    pub const CTF_EXCHANGE: &str = "0xE111180000d2663C0091e4f400237545B87B996B";

    /// Neg Risk CTF Exchange contract address (v2).
    pub const NEG_RISK_CTF_EXCHANGE: &str = "0xe2222d279d744050d28e00520010520000310F59";

    /// Neg Risk Adapter contract address.
    pub const NEG_RISK_ADAPTER: &str = "0xd91E80cF2E7be2e162c6513ceD06f1dD0dA35296";

    /// Gnosis Safe Proxy Factory address on Polygon (used for CREATE2 derivation).
    pub const SAFE_FACTORY: &str = "0xaacFeEa03eb1561C4e67d661e40682Bd20E3541b";

    /// Gnosis Safe MultiSend contract address on Polygon.
    pub const SAFE_MULTISEND: &str = "0xA238CBeb142c10Ef7Ad8442C6D1f9E89e07e7761";

    /// Polymarket Proxy Wallet Factory address on Polygon mainnet.
    pub const PROXY_FACTORY: &str = "0xaB45c5A4B0c941a2F231C04C3f49182e1A254052";

    /// Polymarket Relay Hub address on Polygon mainnet.
    pub const RELAY_HUB: &str = "0xD216153c06E857cD7f72665E0aF1d7D82172F494";

    /// Init code hash for CREATE2 Proxy wallet address derivation.
    pub const PROXY_INIT_CODE_HASH: [u8; 32] = [
        0xd2, 0x1d, 0xf8, 0xdc, 0x65, 0x88, 0x0a, 0x86,
        0x06, 0xf0, 0x9f, 0xe0, 0xce, 0x3d, 0xf9, 0xb8,
        0x86, 0x92, 0x87, 0xab, 0x0b, 0x05, 0x8b, 0xe0,
        0x5a, 0xa9, 0xe8, 0xaf, 0x63, 0x30, 0xa0, 0x0b,
    ];

    /// Init code hash for CREATE2 Safe address derivation.
    /// keccak256 of the Safe proxy creation code with the singleton address.
    pub const SAFE_INIT_CODE_HASH: [u8; 32] = [
        0x2b, 0xce, 0x21, 0x27, 0xff, 0x07, 0xfb, 0x63,
        0x2d, 0x16, 0xc8, 0x34, 0x7c, 0x4e, 0xbf, 0x50,
        0x1f, 0x48, 0x41, 0x16, 0x8b, 0xed, 0x00, 0xd9,
        0xe6, 0xef, 0x71, 0x5d, 0xdb, 0x6f, 0xce, 0xcf,
    ];
}

/// Function selectors for CTF contract calls.
mod selectors {
    /// `redeemPositions(address,bytes32,bytes32,uint256[])` function selector.
    /// keccak256("redeemPositions(address,bytes32,bytes32,uint256[])")[:4]
    pub const REDEEM_POSITIONS: [u8; 4] = [0x01, 0xb7, 0x03, 0x7c];
}

/// Parameters for creating a redeem transaction.
#[derive(Debug, Clone)]
pub struct RedeemParams {
    /// The condition ID of the market (bytes32 hex string with 0x prefix).
    pub condition_id: String,
    /// The outcome index to redeem (0 for YES/UP, 1 for NO/DOWN in binary markets).
    /// If None, redeems both outcomes (index sets [1, 2]).
    pub outcome_index: Option<u8>,
}

/// Creates a transaction to redeem positions from a resolved market.
///
/// This encodes a call to the CTF contract's `redeemPositions` function:
/// ```solidity
/// function redeemPositions(
///     address collateralToken,
///     bytes32 parentCollectionId,
///     bytes32 conditionId,
///     uint256[] indexSets
/// )
/// ```
///
/// # Arguments
///
/// * `params` - The redeem parameters including condition ID and outcome index.
///
/// # Returns
///
/// A `Transaction` struct with the encoded call data targeting the CTF contract.
///
/// # Example
///
/// ```rust
/// use poly_clob_rs::api::relayer::transactions::{create_redeem_tx, RedeemParams};
///
/// let tx = create_redeem_tx(&RedeemParams {
///     condition_id: "0x6d36239527622360000000000000000000000000000000000000000000000000".to_string(),
///     outcome_index: Some(0), // Redeem YES tokens
/// }).unwrap();
///
/// println!("Target: {}", tx.to);
/// println!("Data: {}", tx.data);
/// ```
pub fn create_redeem_tx(params: &RedeemParams) -> Result<Transaction> {
    // Parse the condition ID
    let condition_id: B256 = params
        .condition_id
        .parse()
        .map_err(|_| ValidationError::InvalidParameter {
            parameter: "condition_id".to_string(),
            reason: "invalid format, expected 0x-prefixed bytes32".to_string(),
        })?;

    // Parent collection ID is always zero for Polymarket markets
    let parent_collection_id = B256::ZERO;

    // Index sets: if outcome_index is None, redeem both outcomes [1, 2]
    // Otherwise redeem single outcome: 1 << outcome_index
    let index_sets: Vec<U256> = match params.outcome_index {
        Some(idx) => vec![U256::from(1u64 << idx)],
        None => vec![U256::from(1), U256::from(2)], // Both outcomes
    };

    // Parse contract addresses. Redeem via the pUSD router so proceeds settle as
    // spendable pUSD collateral (matches the working UI redeem), NOT the legacy
    // CTF+USDC.e path which required a manual UI deposit to become usable.
    let redeem_target: Address = contracts::REDEEM_ROUTER
        .parse()
        .map_err(|_| ValidationError::InvalidParameter {
            parameter: "REDEEM_ROUTER".to_string(),
            reason: "invalid redeem router address".to_string(),
        })?;
    let collateral_address: Address = contracts::PUSD_COLLATERAL
        .parse()
        .map_err(|_| ValidationError::InvalidParameter {
            parameter: "PUSD_COLLATERAL".to_string(),
            reason: "invalid pUSD collateral address".to_string(),
        })?;

    // Encode the function call:
    // redeemPositions(address collateralToken, bytes32 parentCollectionId, bytes32 conditionId, uint256[] indexSets)
    let array_len = index_sets.len();
    let mut data = Vec::with_capacity(4 + 32 * 4 + 32 + 32 * array_len); // selector + 4 params + array length + array elements

    // Function selector
    data.extend_from_slice(&selectors::REDEEM_POSITIONS);

    // Encode parameters (all padded to 32 bytes):
    // 1. collateralToken (address) - left-padded to 32 bytes
    data.extend_from_slice(&[0u8; 12]); // 12 zero bytes padding
    data.extend_from_slice(collateral_address.as_slice()); // 20 bytes address

    // 2. parentCollectionId (bytes32)
    data.extend_from_slice(parent_collection_id.as_slice());

    // 3. conditionId (bytes32)
    data.extend_from_slice(condition_id.as_slice());

    // 4. indexSets (uint256[]) - dynamic array, need offset first
    // Offset to array data (4 * 32 = 128 bytes from start of params)
    let offset = U256::from(128);
    data.extend_from_slice(&offset.to_be_bytes::<32>());

    // Array length
    let array_length = U256::from(array_len);
    data.extend_from_slice(&array_length.to_be_bytes::<32>());

    // Array elements (the index set values)
    for index_set in index_sets {
        data.extend_from_slice(&index_set.to_be_bytes::<32>());
    }

    Ok(Transaction {
        to: redeem_target,
        data: Bytes::from(data),
        value: U256::ZERO,
    })
}

/// Creates a transaction to redeem all positions (both outcomes) from a resolved market.
///
/// This is useful when you hold tokens for both outcomes and want to redeem them all
/// in a single transaction.
///
/// # Arguments
///
/// * `condition_id` - The condition ID of the market.
///
/// # Returns
///
/// A single `Transaction` struct for redeeming both outcomes with indexSets [1, 2].
pub fn create_redeem_all_tx(condition_id: &str) -> Result<Transaction> {
    create_redeem_tx(&RedeemParams {
        condition_id: condition_id.to_string(),
        outcome_index: None, // Redeem both outcomes
    })
}

/// Encode transactions as a call to the ProxyWalletFactory's `proxy` function.
///
/// The proxy function signature is:
/// `proxy((uint8 typeCode, address to, uint256 value, bytes data)[])`
///
/// This wraps the given transactions so they can be executed through the proxy wallet.
pub fn encode_proxy_call_data(transactions: &[Transaction]) -> Bytes {
    use alloy::primitives::keccak256;

    // Function selector: keccak256("proxy((uint8,address,uint256,bytes)[])")[:4]
    let selector = &keccak256("proxy((uint8,address,uint256,bytes)[])".as_bytes())[..4];

    // ABI encoding of tuple[] argument
    // The array is a dynamic type, so first word is offset to array data
    let mut encoded = Vec::new();

    // Offset to array data (= 32, one word)
    encoded.extend_from_slice(&U256::from(32).to_be_bytes::<32>());

    // Array length
    encoded.extend_from_slice(&U256::from(transactions.len()).to_be_bytes::<32>());

    // For dynamic-type elements, first encode offsets, then elements
    // Each tuple contains `bytes` (dynamic), so tuples are dynamic
    let mut offsets = Vec::new();
    let mut elements = Vec::new();

    // First pass: encode all elements and compute offsets
    // Offsets are relative to the start of the elements section
    // The offsets section itself is transactions.len() * 32 bytes
    let offsets_section_size = transactions.len() * 32;

    for tx in transactions {
        offsets.push(offsets_section_size + elements.len());

        // Encode tuple: (uint8, address, uint256, bytes)
        // typeCode = 1 (Call). Values: 0=Invalid, 1=Call, 2=DelegateCall
        elements.extend_from_slice(&U256::from(1).to_be_bytes::<32>());
        // to (address, left-padded)
        elements.extend_from_slice(&[0u8; 12]);
        elements.extend_from_slice(tx.to.as_slice());
        // value
        elements.extend_from_slice(&tx.value.to_be_bytes::<32>());
        // offset to bytes data (from start of this tuple = 4 * 32 = 128)
        elements.extend_from_slice(&U256::from(128).to_be_bytes::<32>());
        // bytes data: length + padded data
        let data = tx.data.as_ref();
        elements.extend_from_slice(&U256::from(data.len()).to_be_bytes::<32>());
        elements.extend_from_slice(data);
        // Pad to 32-byte boundary
        let padding = (32 - (data.len() % 32)) % 32;
        elements.extend_from_slice(&vec![0u8; padding]);
    }

    // Write offsets
    for offset in offsets {
        encoded.extend_from_slice(&U256::from(offset).to_be_bytes::<32>());
    }

    // Write elements
    encoded.extend_from_slice(&elements);

    // Prepend selector
    let mut result = Vec::with_capacity(4 + encoded.len());
    result.extend_from_slice(selector);
    result.extend_from_slice(&encoded);

    Bytes::from(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::hex;

    #[test]
    fn test_create_redeem_tx_outcome_0() {
        let params = RedeemParams {
            condition_id: "0x0000000000000000000000000000000000000000000000000000000000000001"
                .to_string(),
            outcome_index: Some(0),
        };

        let tx = create_redeem_tx(&params).unwrap();

        // Verify target address is the pUSD redeem router
        assert_eq!(
            tx.to,
            contracts::REDEEM_ROUTER.parse::<Address>().unwrap()
        );

        // Verify value is 0
        assert_eq!(tx.value, U256::ZERO);

        // Verify function selector
        let data = tx.data.as_ref();
        assert_eq!(&data[0..4], &selectors::REDEEM_POSITIONS);

        // Verify pUSD collateral address is correctly encoded (after 12 bytes of padding)
        let collateral_in_data = &data[16..36]; // Skip selector (4) + padding (12)
        let expected_collateral: Address = contracts::PUSD_COLLATERAL.parse().unwrap();
        assert_eq!(collateral_in_data, expected_collateral.as_slice());

        // Verify index set is 1 (for outcome_index 0: 1 << 0 = 1)
        let index_set_offset = 4 + 32 * 4 + 32; // selector + 4 params + array length
        let index_set_bytes = &data[index_set_offset..index_set_offset + 32];
        let index_set = U256::from_be_slice(index_set_bytes);
        assert_eq!(index_set, U256::from(1));

        println!("Encoded data: 0x{}", hex::encode(data));
    }

    #[test]
    fn test_create_redeem_tx_outcome_1() {
        let params = RedeemParams {
            condition_id: "0x0000000000000000000000000000000000000000000000000000000000000001"
                .to_string(),
            outcome_index: Some(1),
        };

        let tx = create_redeem_tx(&params).unwrap();

        // Verify index set is 2 (for outcome_index 1: 1 << 1 = 2)
        let data = tx.data.as_ref();
        let index_set_offset = 4 + 32 * 4 + 32; // selector + 4 params + array length
        let index_set_bytes = &data[index_set_offset..index_set_offset + 32];
        let index_set = U256::from_be_slice(index_set_bytes);
        assert_eq!(index_set, U256::from(2));
    }

    #[test]
    fn test_create_redeem_all_tx() {
        let condition_id =
            "0x0000000000000000000000000000000000000000000000000000000000000001";
        let tx = create_redeem_all_tx(condition_id).unwrap();

        // Verify it targets the pUSD redeem router
        assert_eq!(
            tx.to,
            contracts::REDEEM_ROUTER.parse::<Address>().unwrap()
        );

        // Verify array length is 2 (both outcomes)
        let data = tx.data.as_ref();
        let array_len_offset = 4 + 32 * 4; // selector + 4 params
        let array_len_bytes = &data[array_len_offset..array_len_offset + 32];
        let array_len = U256::from_be_slice(array_len_bytes);
        assert_eq!(array_len, U256::from(2));
    }

    #[test]
    fn test_invalid_condition_id() {
        let params = RedeemParams {
            condition_id: "invalid".to_string(),
            outcome_index: Some(0),
        };

        assert!(create_redeem_tx(&params).is_err());
    }
}

    #[test]
    fn test_compute_ctf_selectors() {
        use alloy::primitives::keccak256;
        let sigs = [
            "payoutDenominator(bytes32)",
            "payoutNumerators(bytes32,uint256)",
            "redeemPositions(address,bytes32,bytes32,uint256[])",
            "nonces(address)",
            "getNonce(address)",
        ];
        for sig in &sigs {
            let h = keccak256(sig.as_bytes());
            println!("{}: 0x{}", sig, hex::encode(&h[..4]));
        }
    }
