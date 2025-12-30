use anyhow::{Context, Result};
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
        use dotenv::dotenv;
        use std::env;

        dotenv().ok();

        let telegram = load_telegram_config();

        Ok(Account {
            poly_address: env::var("POLY_ADDRESS").context("missing POLY_ADDRESS env var")?,
            pub_key: env::var("PUB_KEY").context("missing PUB_KEY env var")?,
            private_key: env::var("PRIVATE_KEY").context("missing PRIVATE_KEY env var")?,
            api_key: env::var("API_KEY").context("missing API_KEY env var")?,
            api_secret: env::var("API_SECRET").context("missing API_SECRET env var")?,
            api_passphrase: env::var("API_PASSPHRASE").context("missing API_PASSPHRASE env var")?,
            account_type: AccountType::PolymarketAccount,
            telegram_chat_id: telegram.chat_id,
            telegram_bot_token: telegram.bot_token,
        })
    }

    pub fn load_paper_account(account_name: &str) -> Self {
        use dotenv::dotenv;

        dotenv().ok();

        let telegram = load_telegram_config();

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
        }
    }

    pub fn load_binance_account(account_name: &str) -> Self {
        use dotenv::dotenv;

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
        }
    }
}

impl Default for Account {
    fn default() -> Self {
        Self::load_poly_account().expect("failed to load account from environment")
    }
}
