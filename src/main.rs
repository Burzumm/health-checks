mod commands;

use addr::parse_domain_name;
use async_process::Command;
use clap::Parser;
use futures::future::join_all;
use futures::task::Spawn;
use serde::{Deserialize, Serialize};
use std::fs;
use std::net::IpAddr;
use std::rc::Rc;
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, watch};
use tokio::time;
use tracing::{error, info, warn};
use tracing_attributes::instrument;
use telegram_bot_rust::{Message, TelegramBot};
use tokio::sync::broadcast::Sender;
use tokio::task::JoinHandle;
use crate::commands::TelegramCommand;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct AppConfig {
    addresses: Vec<String>,
    timeout_secs: i64,
    telegram_api_token: String,
    telegram_chat_ids: Vec<i64>,
}

#[derive(Clone)]
struct ChannelMessage {
    addr: String,
    command: Rc<dyn TelegramCommand>,
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
        .unwrap_or_else(|| "config.json".to_string());
    let data = &fs::read_to_string(config_path).expect("Should have been able to read the file");
    let config: AppConfig = serde_json::from_str(data).unwrap();
    return config;
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Arc::new(get_config());
    let telegram_bot = TelegramBot::new(config.telegram_api_token.to_string());
    telegram_bot.get_me().await;
    tracing_subscriber::fmt().try_init().unwrap();
    let (tx , _) = broadcast::channel(16);

    let mut tasks: Vec<JoinHandle<()>> = Vec::new();
    for addr in &config.addresses {
        let conf = Arc::clone(&config);
        let rx: Rc<Receiver<ChannelMessage>> = Rc::new(tx.subscribe());
        match addr.parse() {
            Ok(IpAddr::V4(_addr)) => {
                tasks.push(tokio::spawn(ping_handler(_addr.to_string(), conf, rx)));
            }
            Ok(IpAddr::V6(_addr)) => {
                tasks.push(tokio::spawn(ping_handler(_addr.to_string(), conf, rx)));
            }
            Err(e) => match parse_domain_name(addr) {
                Ok(_addr) => {
                    tasks.push(tokio::spawn(ping_handler(_addr.to_string(), conf, rx)));
                }
                Err(err) => panic!("{} parse to addr error: {} {}", addr, e, err),
            },
        }
    }
    let conf = Arc::clone(&config);
    tasks.push(tokio::spawn(get_updates(conf, tx)));
    join_all(tasks).await;
    Ok(())
}

async fn get_updates(app_config: Arc<AppConfig>, tx: Sender<ChannelMessage>) {
    let mut update_id: Option<i64> = None;
    let mut telegram_bot = TelegramBot::new(app_config.telegram_api_token.to_string());
    loop {
        let updates = telegram_bot.get_updates(10, update_id).await.unwrap();
        for update in updates {
            update_id = Some(update.update_id);
            println!("{}", update.update_id);
            //tx.send("1111".to_string()).expect("TODO: panic message");
        }
    }
}

#[instrument]
async fn ping_handler(
    addr: String,
    app_config: Arc<AppConfig>,
    mut rx: broadcast::Receiver<ChannelMessage>,
) {
    let mut interval = time::interval(Duration::from_secs(app_config.timeout_secs as u64));
    let telegram_bot = TelegramBot::new(app_config.telegram_api_token.to_string());
    loop {
        let rx_result = rx.is_empty();
        if !rx_result {
            let message = rx.recv().await.unwrap();
        }
        let output_ping = Command::new("ping")
            .arg(&addr)
            .arg("-c")
            .arg("1")
            .output()
            .await;
        match output_ping {
            Ok(output) => {
                if output.status.success() {
                    info!("host: {} available \n", addr);
                } else {
                    warn!("host: {} unavailable \n", addr);
                    for telegram_chat_id in app_config.telegram_chat_ids.clone().into_iter() {
                        let response = telegram_bot
                            .send_message(Message::new(
                                telegram_chat_id,
                                format!("🔥🔥🔥 HOST: {} UNAVAILABLE 🔥🔥🔥 \n", addr),
                            ))
                            .await;
                        match response {
                            Ok(res) => match res.status().as_u16() {
                                200 => info!(
                                    "was sent alert successfully, status code: {}",
                                    res.status().to_string()
                                ),
                                _ => error!(
                                    "failed sent alert  error: status code: {}",
                                    res.status().to_string()
                                ),
                            },
                            Err(err) => error!("failed sent alert  error: {}", err),
                        };
                    }
                }
            }
            Err(_) => warn!("host: {}unavailable \n", addr),
        }
        interval.tick().await;
    }
}
