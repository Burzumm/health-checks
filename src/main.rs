extern crate core;

mod commands;
mod configs;
use std::option::Option;

use crate::configs::AppConfig;
use addr::parse_domain_name;
use async_process::Command;
use clap::Parser;
use configs::{AddressConfig, PingConfig, TelegramConfig};
use futures::future::join_all;

use std::net::IpAddr;

use std::{fs, thread};

use std::sync::Arc;
use std::time::Duration;
use telegram_bot_rust::{Message, TelegramBot, TelegramMessage, UpdatedMessage};
use tokio::task::JoinHandle;
use tokio::time;
use tracing::{error, info, warn};
use tracing_attributes::instrument;

struct HealthChecker {
    pub allert_manager: Arc<AllertManager>,
}

impl HealthChecker {
    async fn ping(&self, address: &str) -> bool {
        let output_ping = Command::new("ping")
            .arg(&address)
            .arg("-c")
            .arg("1")
            .output()
            .await;

        match output_ping {
            Ok(output) => {
                if output.status.success() {
                    true
                } else {
                    false
                }
            }

            Err(err) => {
                let message = format!("error request to addr: {}, error: {}", address, err);
                warn!("error ping err: {}\n", err);
                self.allert_manager.send_telegram_alert(&message).await;
                return false;
            }
        }
    }

    pub async fn ping_health_handler(&self, address_index: usize, ping_config: Arc<PingConfig>) {
        let addr = &ping_config.addresses[address_index];
        let mut retry_count = 0;
        let mut interval = time::interval(Duration::from_secs(ping_config.timeout_secs as u64));
        let mut send_alert_messages: Vec<Option<TelegramMessage>> = Vec::new();
        loop {
            let output_ping = self.ping(&addr.address).await;
            match output_ping {
                true => {
                    info!("host: {} available", addr.address);
                    retry_count = 0;
                    for (i, msg) in send_alert_messages.clone().iter().enumerate() {
                        if let Some(msg) = msg {
                            match &msg.text {
                                Some(text) => {
                                    if text.contains(&addr.address) {
                                        let edit_result = self
                                            .allert_manager
                                            .edit_telegram_about_unavailability(
                                                &addr.address,
                                                &addr.description,
                                                msg,
                                            )
                                            .await;
                                        if edit_result {
                                            send_alert_messages.remove(i);
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
                false => {
                    if send_alert_messages.len() == 0 {
                        retry_count += 1;
                        warn!("host: {} {} unavailable", &addr.address, &addr.description);
                        if retry_count > ping_config.retry - 1 {
                            send_alert_messages = self
                                .allert_manager
                                .send_a_telegram_about_unavailability(&addr.address, &addr.description)
                                .await;
                            if send_alert_messages.iter().any(|x| x.is_none())
                            {
                                loop {
                                    send_alert_messages = self
                                        .allert_manager
                                        .send_a_telegram_about_unavailability(
                                            &addr.address,
                                            &addr.description,
                                        )
                                        .await;

                                    if !send_alert_messages.iter().any(|x| x.is_none())
                                    {
                                        break;
                                    }
                                    thread::sleep(Duration::from_secs(
                                        5,
                                    ))
                                }
                            }
                            retry_count = 0;
                            thread::sleep(Duration::from_secs(
                                ping_config.sleep_after_alert_secs as u64,
                            ))
                        }
                    }

                    thread::sleep(Duration::from_secs(
                        ping_config.sleep_after_alert_secs as u64,
                    ))
                }
            }

            interval.tick().await;
        }
    }

    
    pub fn new(allert_manager: Arc<AllertManager>) -> Self {
        Self { allert_manager }
    }
}

struct AllertManager {
    telegram_bot: Arc<TelegramBot>,
    telegram_config: Arc<TelegramConfig>,
}

impl AllertManager {
    pub fn new(telegram_bot: Arc<TelegramBot>, telegram_config: Arc<TelegramConfig>) -> Self {
        Self {
            telegram_bot,
            telegram_config,
        }
    }

    pub async fn edit_telegram_about_unavailability(
        &self,
        ip: &String,
        descr: &String,
        msg: &TelegramMessage,
    ) -> bool {
        let message = format!("✅✅✅\nHOST: {}\n{}\nAVAILABLE\n✅✅✅\n", ip, descr).to_string();

        let edit_message_result = self
            .telegram_bot
            .edit_message_text(&UpdatedMessage::from(&msg, message))
            .await;
        match edit_message_result {
            Ok(_) => return true,
            Err(err) => {
                error!("failed edit message error: {}", err);
                return false;
            }
        }
    }

    pub async fn guaranteed_to_send_a_telegram_about_unavailability(
        &self,
        ip: &String,
        descr: &String,
    ) -> Vec<TelegramMessage> {
        let mut result: Vec<TelegramMessage> = Vec::new();

        let send_result = self.send_a_telegram_about_unavailability(ip, descr).await;
        if send_result.iter().any(|x| x.is_none()) {
            loop {
                let result = self.send_a_telegram_about_unavailability(ip, descr).await;
                if !result.iter().any(|x| x.is_none()) {
                    return result.iter().filter_map(|x| x.to_owned()).collect();
                }
                thread::sleep(Duration::from_secs(5))
            }
        }
        return result;
    }

    pub async fn send_a_telegram_about_unavailability(
        &self,
        ip: &String,
        descr: &String,
    ) -> Vec<Option<TelegramMessage>> {
        let message: String = format!("🔥🔥🔥\nHOST: {}\n{}\nUNAVAILABLE\n🔥🔥🔥\n", ip, descr);
        return self.send_telegram_alert(&message).await;
    }

    pub async fn send_telegram_alert(&self, message: &String) -> Vec<Option<TelegramMessage>> {
        let mut result: Vec<Option<TelegramMessage>> = Vec::new();
        for telegram_chat_id in self.telegram_config.telegram_chat_ids.iter() {
            let response = self
                .telegram_bot
                .send_message(&Message::new(*telegram_chat_id, message.to_string()))
                .await;
            let msg: Option<TelegramMessage> = match response {
                Ok(msg) => Some(msg.result),
                Err(err) => {
                    error!("failed send alert  error: {}", err);
                    None
                }
            };
            result.push(msg);
        }
        return result;
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    config_path: Option<String>,
}

fn get_config() -> AppConfig {
    let args = Args::parse();
    let config_path = args
        .config_path
        .unwrap_or_else(|| "./config/config.json".to_string());
    let data = &fs::read_to_string(config_path).expect("Should have been able to read the file");
    let config: AppConfig = serde_json::from_str(data).unwrap();
    return config;
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().try_init().unwrap();
    let config = Arc::new(get_config());

    let allert_manager = Arc::new(AllertManager::new(
        Arc::new(TelegramBot::new(
            config.telegram_config.telegram_api_token.to_string(),
        )),
        config.telegram_config.clone(),
    ));

    let health_check = Arc::new(HealthChecker::new(allert_manager));
    let _ = &health_check.allert_manager.telegram_bot.get_me().await;

    let mut tasks: Vec<JoinHandle<()>> = Vec::new();
    for (i, addr) in config.ping_config.addresses.iter().enumerate() {
        let conf = Arc::clone(&config.ping_config);
        let health_check_scope = Arc::clone(&health_check);
        match addr.address.parse() {
            Ok(IpAddr::V4(_addr)) => {
                tasks.push(tokio::spawn(
                    health_check_scope.ping_health_handler(i, conf),
                ));
            }
            Ok(IpAddr::V6(_addr)) => {
                tasks.push(tokio::spawn(
                    health_check_scope.ping_health_handler(i, conf),
                ));
            }
            Err(e) => match parse_domain_name(&*addr.address) {
                Ok(_addr) => {
                    tasks.push(tokio::spawn(
                        health_check_scope.ping_health_handler(i, conf),
                    ));
                }
                Err(err) => panic!("{} parse to addr error: {} {}", addr.address, e, err),
            },
        }
    }
    join_all(tasks).await;
    Ok(())
}
