use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppConfig {
    pub request_config: RequestConfig,
    pub telegram_config: TelegramConfig,
    pub ping_config: PingConfig,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RequestConfig {
    pub addresses: Vec<AddressConfig>,
    pub timeout_secs: i64,
    pub retry: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PingConfig {
    pub addresses: Vec<AddressConfig>,
    pub timeout_secs: i64,
    pub retry: i64,
    pub sleep_after_alert_secs: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TelegramConfig {
    pub telegram_api_token: String,
    pub telegram_chat_ids: Vec<i64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AddressConfig {
    pub address: String,
    pub description: String,
}
