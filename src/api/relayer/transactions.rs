//! Transaction builders for Polymarket CTF (Conditional Token Framework) operations.
//!
//! This module provides functions to create encoded transaction data for various
//! CTF operations like redeeming positions, converting tokens, etc.

use super::types::Transaction;
use alloy::primitives::{Address, Bytes, B256, U256};
use anyhow::{Context, Result};

/// Polygon mainnet contract addresses for Polymarket.
pub mod contracts {
    /// CTF (Conditional Token Framework) contract address on Polygon.
    pub const CTF_CONTRACT: &str = "0x4d97dcd97ec945f40cf65f87097ace5ea0476045";

    /// USDC.e (Bridged USDC) contract address on Polygon.
    pub const USDC_E_CONTRACT: &str = "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174";

    /// CTF Exchange contract address.
    pub const CTF_EXCHANGE: &str = "0x4bFb41d5B3570DeFd03C39a9A4D8dE6Bd8B8982E";

    /// Neg Risk CTF Exchange contract address.
    pub const NEG_RISK_CTF_EXCHANGE: &str = "0xC5d563A36AE78145C45a50134d48A1215220f80a";

    /// Neg Risk Adapter contract address.
    pub const NEG_RISK_ADAPTER: &str = "0xd91E80cF2E7be2e162c6513ceD06f1dD0dA35296";
}

/// Function selectors for CTF contract calls.
mod selectors {
    /// `redeemPositions(address,bytes32,bytes32,uint256[])` function selector.
    /// keccak256("redeemPositions(address,bytes32,bytes32,uint256[])")[:4]
    pub const REDEEM_POSITIONS: [u8; 4] = [0x31, 0x1d, 0x8a, 0x8e];
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
///     outcome_index: 0, // Redeem YES tokens
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
        .context("invalid condition_id format, expected 0x-prefixed bytes32")?;

    // Parent collection ID is always zero for Polymarket markets
    let parent_collection_id = B256::ZERO;

    // Index sets: if outcome_index is None, redeem both outcomes [1, 2]
    // Otherwise redeem single outcome: 1 << outcome_index
    let index_sets: Vec<U256> = match params.outcome_index {
        Some(idx) => vec![U256::from(1u64 << idx)],
        None => vec![U256::from(1), U256::from(2)], // Both outcomes
    };

    // Parse contract addresses
    let ctf_address: Address = contracts::CTF_CONTRACT
        .parse()
        .context("invalid CTF contract address")?;
    let usdc_address: Address = contracts::USDC_E_CONTRACT
        .parse()
        .context("invalid USDC.e contract address")?;

    // Encode the function call:
    // redeemPositions(address collateralToken, bytes32 parentCollectionId, bytes32 conditionId, uint256[] indexSets)
    let array_len = index_sets.len();
    let mut data = Vec::with_capacity(4 + 32 * 4 + 32 + 32 * array_len); // selector + 4 params + array length + array elements

    // Function selector
    data.extend_from_slice(&selectors::REDEEM_POSITIONS);

    // Encode parameters (all padded to 32 bytes):
    // 1. collateralToken (address) - left-padded to 32 bytes
    data.extend_from_slice(&[0u8; 12]); // 12 zero bytes padding
    data.extend_from_slice(usdc_address.as_slice()); // 20 bytes address

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
        to: ctf_address,
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

        // Verify target address
        assert_eq!(
            tx.to,
            contracts::CTF_CONTRACT.parse::<Address>().unwrap()
        );

        // Verify value is 0
        assert_eq!(tx.value, U256::ZERO);

        // Verify function selector
        let data = tx.data.as_ref();
        assert_eq!(&data[0..4], &selectors::REDEEM_POSITIONS);

        // Verify USDC address is correctly encoded (after 12 bytes of padding)
        let usdc_in_data = &data[16..36]; // Skip selector (4) + padding (12)
        let expected_usdc: Address = contracts::USDC_E_CONTRACT.parse().unwrap();
        assert_eq!(usdc_in_data, expected_usdc.as_slice());

        // Verify index set is 1 (for outcome_index 0: 1 << 0 = 1)
        let index_set_offset = 4 + 32 * 4 + 32; // selector + 4 params + array length
        let index_set_bytes = &data[index_set_offset..index_set_offset + 32];
        let index_set = U256::from_be_slice(index_set_bytes);
        assert_eq!(index_set, U256::from(1));

        println!("Encoded data: 0x{}", hex::encode(&data));
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

        // Verify it targets the CTF contract
        assert_eq!(
            tx.to,
            contracts::CTF_CONTRACT.parse::<Address>().unwrap()
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
