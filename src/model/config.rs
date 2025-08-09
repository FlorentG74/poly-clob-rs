use strum_macros::EnumString;

#[derive(Debug, Default, EnumString, Clone, PartialEq, Eq)]
pub enum DbMode {
    #[default]
    OFFLINE,
    ONLINE,
}

#[derive(Debug, Default)]
pub struct Config {
    pub db_mode: DbMode,

    pub db_url: String,
    pub db_host: String,
    pub db_port: String,
    pub db_name: String,
    pub db_user: String,
    pub db_password: String,

    pub pub_key: String,
    pub poly_address: String,
    pub private_key: String,
    pub api_key: String,
    pub api_secret: String,
    pub api_passphrase: String,

    pub telegram_chat_id: String,
    pub telegram_bot_token: String,

    pub binance_api_key: String,
    pub binance_api_secret: String,
}
