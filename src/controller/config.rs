use crate::model::Config;

pub trait ConfigProvider {
    fn get_config(&self) -> &Config;
}

pub struct DotEnvConfigProvider(Config);

impl DotEnvConfigProvider {
    pub fn new() -> Self {
        use dotenv::dotenv;
        use std::env;
        dotenv().ok();
        let config = Config {
            db_mode: env::var("DB_MODE").expect("Missing database Mode").parse().unwrap(),
            
            db_url: env::var("DB_URL").expect("Missing database URL"),
            db_host: env::var("DB_HOST").expect("Missing database host"),
            db_port: env::var("DB_PORT").expect("Missing database port"),
            db_name: env::var("DB_NAME").expect("Missing database name"),
            db_user: env::var("DB_USER").expect("Missing database user"),
            db_password: env::var("DB_PASSWORD").expect("Missing database password"),

            pub_key: env::var("PUB_KEY").expect("Missing public key"),
            poly_address: env::var("POLY_ADDRESS").expect("Missing Polygon Address"),
            private_key: env::var("PRIVATE_KEY").expect("Missing Private key"),
            api_key: env::var("API_KEY").expect("Missing API  key"),
            api_secret: env::var("API_SECRET").expect("Missing API Secret"),
            api_passphrase: env::var("API_PASSPHRASE").expect("Missing API Passphrase"),

            telegram_chat_id: env::var("TELEGRAM_CHAT_ID").expect("Missing Telegram chatId"),
            telegram_bot_token: env::var("TELEGRAM_BOT_TOKEN").expect("Missing Telegram bot token"),

            binance_api_key: env::var("BINANCE_API_KEY").expect("Missing Binance API Key"),
            binance_api_secret: env::var("BINANCE_API_SECRET").expect("Missing Binance API secret"),
        };

        DotEnvConfigProvider(config)
    }

    pub fn get_db_connection_string(&self) -> String {
        // Build DB connection string
        format!(
            "host={} port={} dbname={} user={} password={} sslmode=prefer",
            self.get_config().db_host,
            self.get_config().db_port,
            self.get_config().db_name,
            self.get_config().db_user,
            self.get_config().db_password
        )
    }
}

impl ConfigProvider for DotEnvConfigProvider {
    fn get_config(&self) -> &Config {
        &self.0
    }
}

impl Default for DotEnvConfigProvider {
    fn default() -> Self {
        Self::new()
    }
}
