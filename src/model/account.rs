use crate::controller::config::ConfigProvider;
use crate::controller::DotEnvConfigProvider;

#[derive(Debug, Clone)]
pub enum AccountType {
    PaperAccount,
    PolymarketAccount,
}

#[derive(Debug, Clone)]
pub struct Account {
    pub poly_address: String,
    pub pub_key: String,
    pub private_key: String,
    pub api_key: String,
    pub api_secret: String,
    pub api_passphrase: String,
    pub account_type: AccountType,
}

impl Default for Account {
    fn default() -> Self {
        Self::actual_account_from_env()
    }
}

impl Account {
    pub fn actual_account_from_env() -> Self {
        let config_provider = DotEnvConfigProvider::new();

        Account {
            poly_address: config_provider.get_config().poly_address.clone(),
            pub_key: config_provider.get_config().pub_key.clone(),
            private_key: config_provider.get_config().private_key.clone(),
            api_key: config_provider.get_config().api_key.clone(),
            api_secret: config_provider.get_config().api_secret.clone(),
            api_passphrase: config_provider.get_config().api_passphrase.clone(),
            account_type: AccountType::PolymarketAccount,
        }
    }

    pub fn paper_account(account_name: &str) -> Self {
        Account {
            poly_address: account_name.to_string(),
            pub_key: String::from(""),
            private_key: String::from(""),
            api_key: String::from(""),
            api_secret: String::from(""),
            api_passphrase: String::from(""),
            account_type: AccountType::PaperAccount,
        }
    }
}
