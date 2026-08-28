use crate::api::error::Result;
use crate::api::relayer::SignatureType;
use crate::config::get_config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum AccountType {
    PaperAccount,
    PolymarketAccount,
    /// In-process ("`MemoryWallet`") account: no database, no network. Orders and
    /// position queries are served from an in-memory `InMemoryPaperWallet`. This is
    /// the default wallet in replay mode; select it explicitly in a config with
    /// `account_type = "MemoryAccount"`.
    MemoryAccount,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub poly_address: String,
    pub pub_key: String,
    pub private_key: String,
    pub api_key: String,
    pub api_secret: String,
    pub api_passphrase: String,
    pub account_type: AccountType,
    pub account_name: String,
    pub telegram_chat_id: Option<i64>,
    pub telegram_bot_token: Option<String>,
    /// Builder API key for relayer transactions (`POLY_BUILDER_API_KEY`).
    #[serde(default)]
    pub builder_api_key: Option<String>,
    /// Builder API secret for relayer transactions (`POLY_BUILDER_API_SECRET`).
    #[serde(default)]
    pub builder_api_secret: Option<String>,
    /// Builder API passphrase for relayer transactions (`POLY_BUILDER_API_PASSPHRASE`).
    #[serde(default)]
    pub builder_api_passphrase: Option<String>,
    /// Wallet/Signature type: EOA (0), `POLY_PROXY` (1), or `GNOSIS_SAFE` (2).
    #[serde(default)]
    pub signature_type: SignatureType,
}

impl Account {
    /// Builds the live Polymarket account from [`get_config`].
    ///
    /// Errors when a required credential is absent — which is the normal case for
    /// paper- and replay-only runs, so callers are expected to handle it.
    ///
    /// # Errors
    ///
    /// If any required `POLY_*` credential is unset, or the private key does not parse.
    pub fn load_poly_account() -> Result<Self> {
        let config = get_config();

        Ok(Account {
            poly_address: config.require(&config.poly_address, "POLY_ADDRESS")?,
            pub_key: config.require(&config.pub_key, "PUB_KEY")?,
            private_key: config.require(&config.private_key, "PRIVATE_KEY")?,
            api_key: config.require(&config.api_key, "API_KEY")?,
            api_secret: config.require(&config.api_secret, "API_SECRET")?,
            api_passphrase: config.require(&config.api_passphrase, "API_PASSPHRASE")?,
            account_type: AccountType::PolymarketAccount,
            account_name: config
                .account_name
                .clone()
                .unwrap_or_else(|| "Polymarket".to_string()),
            telegram_chat_id: config.telegram_chat_id,
            telegram_bot_token: config.telegram_bot_token.clone(),
            builder_api_key: config.poly_builder_api_key.clone(),
            builder_api_secret: config.poly_builder_api_secret.clone(),
            builder_api_passphrase: config.poly_builder_api_passphrase.clone(),
            signature_type: config.signature_type,
        })
    }

    #[must_use]
    pub fn load_paper_account(account_name: &str, with_telegram: bool) -> Self {
        let config = get_config();
        let (telegram_chat_id, telegram_bot_token) = if with_telegram {
            (config.telegram_chat_id, config.telegram_bot_token.clone())
        } else {
            (None, None)
        };

        Account {
            poly_address: account_name.to_string(),
            pub_key: Default::default(),
            private_key: Default::default(),
            api_key: Default::default(),
            api_secret: Default::default(),
            api_passphrase: Default::default(),
            account_type: AccountType::PaperAccount,
            account_name: account_name.to_string(),
            telegram_chat_id,
            telegram_bot_token,
            builder_api_key: None,
            builder_api_secret: None,
            builder_api_passphrase: None,
            signature_type: SignatureType::PolyProxy,
        }
    }

    /// Loads an in-memory ("`MemoryWallet`") account. Metadata is identical to a paper
    /// account, but the `MemoryAccount` tag signals the trading bot to route all orders
    /// and position queries through the in-process `InMemoryPaperWallet` (zero DB, zero
    /// network). This is the default wallet used in replay mode.
    #[must_use]
    pub fn load_memory_account(account_name: &str, with_telegram: bool) -> Self {
        let mut account = Self::load_paper_account(account_name, with_telegram);
        account.account_type = AccountType::MemoryAccount;
        account
    }

    /// Returns builder credentials if all required fields are present.
    #[must_use]
    pub fn get_builder_credentials(&self) -> Option<crate::api::relayer::BuilderCredentials> {
        match (
            &self.builder_api_key,
            &self.builder_api_secret,
            &self.builder_api_passphrase,
        ) {
            (Some(key), Some(secret), Some(passphrase)) => {
                Some(crate::api::relayer::BuilderCredentials::new(
                    key.clone(),
                    secret.clone(),
                    passphrase.clone(),
                ))
            }
            _ => None,
        }
    }
}

impl Default for Account {
    fn default() -> Self {
        Self::load_poly_account().expect("failed to load account from environment")
    }
}
