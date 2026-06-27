use crate::api::error::{AuthError, Result};
use crate::api::relayer::SignatureType;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum AccountType {
    PaperAccount,
    PolymarketAccount,
    BinanceAccount,
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
    pub telegram_chat_id: Option<i64>,
    pub telegram_bot_token: Option<String>,
    /// Builder API key for relayer transactions (POLY_BUILDER_API_KEY).
    #[serde(default)]
    pub builder_api_key: Option<String>,
    /// Builder API secret for relayer transactions (POLY_BUILDER_API_SECRET).
    #[serde(default)]
    pub builder_api_secret: Option<String>,
    /// Builder API passphrase for relayer transactions (POLY_BUILDER_API_PASSPHRASE).
    #[serde(default)]
    pub builder_api_passphrase: Option<String>,
    /// Wallet/Signature type: EOA (0), POLY_PROXY (1), or GNOSIS_SAFE (2).
    #[serde(default)]
    pub signature_type: SignatureType,
}

/// Telegram configuration loaded from environment variables.
struct TelegramConfig {
    chat_id: Option<i64>,
    bot_token: Option<String>,
}

/// Loads telegram configuration from environment variables.
fn load_telegram_config() -> TelegramConfig {
    use std::env;

    let chat_id = env::var("TELEGRAM_CHAT_ID")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .filter(|&id| id != 0);

    let bot_token = env::var("TELEGRAM_BOT_TOKEN")
        .ok()
        .filter(|t| !t.is_empty());

    TelegramConfig { chat_id, bot_token }
}

impl Account {
    pub fn load_poly_account() -> Result<Self> {
        use dotenvy::dotenv;
        use std::env;

        dotenv().ok();

        let telegram = load_telegram_config();

        // Load builder credentials (optional)
        let builder_api_key = env::var("POLY_BUILDER_API_KEY").ok();
        let builder_api_secret = env::var("POLY_BUILDER_API_SECRET").ok();
        let builder_api_passphrase = env::var("POLY_BUILDER_API_PASSPHRASE").ok();

        // Load signature type (defaults to POLY_PROXY for backwards compatibility)
        let signature_type = env::var("SIGNATURE_TYPE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(SignatureType::PolyProxy);

        Ok(Account {
            poly_address: env::var("POLY_ADDRESS").map_err(|_| AuthError::MissingEnvVar {
                var_name: "POLY_ADDRESS".to_string(),
            })?,
            pub_key: env::var("PUB_KEY").map_err(|_| AuthError::MissingEnvVar {
                var_name: "PUB_KEY".to_string(),
            })?,
            private_key: env::var("PRIVATE_KEY").map_err(|_| AuthError::MissingEnvVar {
                var_name: "PRIVATE_KEY".to_string(),
            })?,
            api_key: env::var("API_KEY").map_err(|_| AuthError::MissingEnvVar {
                var_name: "API_KEY".to_string(),
            })?,
            api_secret: env::var("API_SECRET").map_err(|_| AuthError::MissingEnvVar {
                var_name: "API_SECRET".to_string(),
            })?,
            api_passphrase: env::var("API_PASSPHRASE").map_err(|_| AuthError::MissingEnvVar {
                var_name: "API_PASSPHRASE".to_string(),
            })?,
            account_type: AccountType::PolymarketAccount,
            telegram_chat_id: telegram.chat_id,
            telegram_bot_token: telegram.bot_token,
            builder_api_key,
            builder_api_secret,
            builder_api_passphrase,
            signature_type,
        })
    }

    pub fn load_paper_account(account_name: &str, with_telegram: bool) -> Self {
        let telegram = if with_telegram {
            load_telegram_config()

        } else {
            TelegramConfig { chat_id: None, bot_token: None }
        };

        Account {
            poly_address: account_name.to_string(),
            pub_key: Default::default(),
            private_key: Default::default(),
            api_key: Default::default(),
            api_secret: Default::default(),
            api_passphrase: Default::default(),
            account_type: AccountType::PaperAccount,
            telegram_chat_id: telegram.chat_id,
            telegram_bot_token: telegram.bot_token,
            builder_api_key: None,
            builder_api_secret: None,
            builder_api_passphrase: None,
            signature_type: SignatureType::PolyProxy,
        }
    }

    pub fn load_binance_account(account_name: &str) -> Self {
        use dotenvy::dotenv;

        dotenv().ok();

        let telegram = load_telegram_config();

        Account {
            poly_address: account_name.to_string(),
            pub_key: Default::default(),
            private_key: Default::default(),
            api_key: Default::default(),
            api_secret: Default::default(),
            api_passphrase: Default::default(),
            account_type: AccountType::BinanceAccount,
            telegram_chat_id: telegram.chat_id,
            telegram_bot_token: telegram.bot_token,
            builder_api_key: None,
            builder_api_secret: None,
            builder_api_passphrase: None,
            signature_type: SignatureType::PolyProxy,
        }
    }

    /// Returns builder credentials if all required fields are present.
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
