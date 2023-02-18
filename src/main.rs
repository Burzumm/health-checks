use async_process::Command;
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::time::Duration;
use tokio::time;
use tracing::{error, info, warn};
use tracing_attributes::instrument;
use addr::parse_domain_name;
use clap::Parser;
use std::fs;

use telegram_bot_rust::{Message, TelegramBot};
use tokio::task::JoinHandle;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct AppConfig {
    addresses: Vec<String>,
    timeout_secs: i64,
    telegram_api_token: String,
    telegram_chat_ids: Vec<i64>,
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
    let config = get_config();
    tracing_subscriber::fmt().try_init().unwrap();
    let mut tasks: Vec<JoinHandle<()>> = Vec::new();
    for addr in &config.addresses {
        let conf = config.clone();
        match addr.parse() {
            Ok(IpAddr::V4(_addr)) => {
                tasks.push(tokio::spawn(ping_handler(_addr.to_string(), conf)))
            }
            Ok(IpAddr::V6(_addr)) => {
                tasks.push(tokio::spawn(ping_handler(_addr.to_string(), conf)))
            }
            Err(e) => {
                let domain = parse_domain_name(addr);
                match domain {
                    Ok(_addr) => tasks.push(tokio::spawn(ping_handler(_addr.to_string(), conf))),
                    Err(err) => panic!("{} parse to addr error: {} {}", addr, e, err),
                }
            }
        }
    }
    join_all(tasks).await;
    Ok(())
}

#[instrument]
async fn ping_handler<'a>(addr: String, app_config: AppConfig) {
    let mut interval = time::interval(Duration::from_secs(app_config.timeout_secs as u64));
    let telegram_bot = TelegramBot::new(app_config.telegram_api_token);
    loop {
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
