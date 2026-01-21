//! Settlement requests for redeeming positions from resolved markets.
//!
//! This module provides a way to submit gasless settlement transactions
//! via the Polymarket Relayer API.

use crate::api::relayer::auth::BuilderCredentials;
use crate::api::relayer::client::RelayerClient;
use crate::api::relayer::transactions::{create_redeem_tx, RedeemParams};
use crate::api::relayer::types::{RelayerTransactionResponse, RelayerTxType};
use crate::{Account, Result};
use typed_builder::TypedBuilder;

/// Represents a settlement/redemption request for an expired market.
///
/// This request will be submitted to the Relayer API to execute a gasless
/// `redeemPositions` transaction on the CTF contract.
#[derive(Debug, Clone, TypedBuilder)]
pub struct SettlementRequest {
    /// The condition ID of the market to redeem from.
    pub condition_id: String,
    /// The winning outcome index (0 for YES/UP, 1 for NO/DOWN).
    pub winning_outcome_index: u8,
}

impl SettlementRequest {
    /// Executes the settlement request via the Relayer API.
    ///
    /// This function creates a `RelayerClient`, builds a `redeemPositions`
    /// transaction, submits it, and returns the relayer's response.
    ///
    /// # Arguments
    ///
    /// * `account` - The account containing the signer address. Builder credentials
    ///             must be available in environment variables.
    ///
    /// # Returns
    ///
    /// A `Result` containing the `RelayerTransactionResponse` on success.
    ///
    /// # Errors
    ///
    /// This function can return several error types, including:
    /// - `ClobError::Auth` if builder credentials are not found.
    /// - `ClobError::Validation` if the transaction parameters are invalid.
    /// - `ClobError::Http` for network issues.
    /// - `ClobError::Api` for relayer API errors.
    pub async fn execute(
        &self,
        account: &Account,
    ) -> Result<RelayerTransactionResponse> {
        log::info!(
            "Executing settlement for condition_id: {} with winning outcome: {}",
            self.condition_id,
            self.winning_outcome_index
        );

        // Builder credentials must be in the environment
        let creds = BuilderCredentials::from_env()?;

        // Use the account's poly_address as the signer
        let client = RelayerClient::builder()
            .credentials(creds)
            .signer_address(account.poly_address.clone())
            .tx_type(RelayerTxType::Safe) // Safe is the standard for most users
            .build();

        // Create the redeem transaction
        let redeem_tx = create_redeem_tx(&RedeemParams {
            condition_id: self.condition_id.clone(),
            outcome_index: Some(self.winning_outcome_index),
        })?;

        log::info!("Submitting redeem transaction via relayer...");

        // Submit the transaction
        let response = client.submit(vec![redeem_tx]).await?;

        log::info!(
            "Settlement transaction submitted successfully. Transaction ID: {}",
            response.transaction_id
        );

        Ok(response)
    }

    /// Check if a market is eligible for settlement.
    ///
    /// NOTE: This is a placeholder. A real implementation would check the market's
    /// state (e.g., via `get_market_by_id`) to ensure it's resolved.
    pub fn is_eligible(&self) -> bool {
        // Placeholder - a real implementation would query market state
        true
    }
}

#[cfg(test)]
mod tests {
    // use super::*;
    // use crate::Account;
    // use std::env;

    // #[tokio::test]
    // #[ignore] // Ignored because it requires valid env vars and a settled market
    // async fn test_settlement_execute() {
    //     // This test requires the following environment variables to be set:
    //     // POLY_BUILDER_API_KEY, POLY_BUILDER_API_SECRET, POLY_BUILDER_API_PASSPHRASE
    //     // and a valid Account configuration.
    //
    //     // Also requires a real, settled market condition ID and its winning outcome.
    //     let condition_id = "0x...".to_string();
    //     let winning_outcome = 0;
    //
    //     let account = Account {
    //          poly_address: env::var("POLY_ADDRESS").unwrap(),
    //          // other account fields can be dummy for this test if not used by relayer client directly
    //          ..Default::default()
    //     };
    //
    //     let request = SettlementRequest::builder()
    //         .condition_id(condition_id)
    //         .winning_outcome_index(winning_outcome)
    //         .build();
    //
    //     let result = request.execute(&account).await;
    //
    //     println!("Settlement result: {:?}", result);
    //     assert!(result.is_ok());
    //     let response = result.unwrap();
    //     assert!(!response.transaction_id.is_empty());
    // }
}