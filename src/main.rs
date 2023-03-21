extern crate core;

mod commands;
mod configs;

use crate::configs::{AppConfig};
use addr::parse_domain_name;
use async_process::Command;
use clap::Parser;
use futures::future::join_all;

use std::net::IpAddr;
use std::rc::Rc;
use std::{fs, thread};

use crate::commands::{TelegramCommand};

use std::sync::Arc;
use std::time::Duration;
use telegram_bot_rust::{Message, TelegramBot, TelegramUpdate};
use tokio::sync::broadcast;
use tokio::sync::broadcast::{Receiver, Sender};
use tokio::task::JoinHandle;
use tokio::time;
use tracing::{error, info, warn};
use tracing_attributes::instrument;



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
        .unwrap_or_else(|| "./config.json".to_string());
    let data = &fs::read_to_string(config_path).expect("Should have been able to read the file");
    let config: AppConfig = serde_json::from_str(data).unwrap();
    return config;
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Arc::new(get_config());
    let telegram_bot = TelegramBot::new(config.telegram_config.telegram_api_token.to_string());
    telegram_bot.get_me().await;
    tracing_subscriber::fmt().try_init().unwrap();
    let (tx, _) = broadcast::channel(16);

    let mut tasks: Vec<JoinHandle<()>> = Vec::new();
    for (i, addr) in config.ping_config.addresses.iter().enumerate() {
        let conf = Arc::clone(&config);
        let rx = tx.subscribe();

        match addr.address.parse() {
            Ok(IpAddr::V4(_addr)) => {
                tasks.push(tokio::spawn(ping_handler(i, conf, rx)));
            }
            Ok(IpAddr::V6(_addr)) => {
                tasks.push(tokio::spawn(ping_handler(i, conf, rx)));
            }
            Err(e) => match parse_domain_name(&*addr.address) {
                Ok(_addr) => {
                    tasks.push(tokio::spawn(ping_handler(i, conf, rx)));
                }
                Err(err) => panic!("{} parse to addr error: {} {}", addr.address, e, err),
            },
        }
    }

    // for addr in config.request_config.addresses.iter() {
    //     let conf = Arc::clone(&config);
    //     let rx = tx.subscribe();
    //     match Url::parse(&*addr.address) {
    //         Ok(_addr) => {
    //             tasks.push(tokio::spawn(request_handler(_addr.to_string(), conf, rx)));
    //         }
    //         Err(err) => panic!("{} parse to addr error: {}", addr.address, err),
    //     }
    // }

    let _conf = Arc::clone(&config);
    //tasks.push(tokio::spawn(get_updates(conf, tx)));
    join_all(tasks).await;
    Ok(())
}

async fn get_updates(app_config: Arc<AppConfig>, tx: Sender<TelegramUpdate>) {
    let mut update_id: Option<i64> = None;
    let mut telegram_bot =
        TelegramBot::new(app_config.telegram_config.telegram_api_token.to_string());
    loop {
        let updates = telegram_bot.get_updates(10, update_id).await.unwrap();
        for update in updates {
            update_id = Some(update.update_id);
            tx.send(update).unwrap();
        }
    }
}

#[instrument]
async fn request_handler(addr: String, app_config: Arc<AppConfig>, rx: Receiver<TelegramUpdate>) {
    let mut retry_count = 0;
    let telegram_bot = TelegramBot::new(app_config.telegram_config.telegram_api_token.to_string());
    let mut interval = time::interval(Duration::from_secs(
        app_config.request_config.timeout_secs as u64,
    ));
    loop {
        let response = reqwest::get(addr.to_string()).await;
        match response {
            Ok(result) => {
                if result.status() != 200 {
                    let message = format!(
                        "🔥🔥🔥 HOST: {} UNAVAILABLE 🔥🔥🔥, status code: {}, body: {} \n",
                        addr,
                        result.status(),
                        result.text().await.unwrap()
                    );
                    warn!(message);
                    if retry_count > app_config.request_config.retry {
                        retry_count += 1;
                        send_telegram_alert(
                            &message,
                            &telegram_bot,
                            &app_config.telegram_config.telegram_chat_ids,
                        )
                        .await;
                        retry_count = 0
                    }
                } else {
                    info!("host: {} available \n", addr);
                }
            }
            Err(err) => {
                let message = format!("error request to addr: {}, error: {}", addr, err);
                error!("error request err: {}", err);
                send_telegram_alert(
                    &message,
                    &telegram_bot,
                    &app_config.telegram_config.telegram_chat_ids,
                )
                .await;
            }
        }
        interval.tick().await;
    }
}

async fn send_telegram_alert(message: &String, telegram_bot: &TelegramBot, recipients: &[i64]) {
    for telegram_chat_id in recipients.iter() {
        let response = telegram_bot
            .send_message(&Message::new(*telegram_chat_id, message.to_string()))
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

#[instrument]
async fn ping_handler(
    address_index: usize,
    app_config: Arc<AppConfig>,
    mut rx: Receiver<TelegramUpdate>,
) {
    let addr = &app_config.ping_config.addresses[address_index];
    let mut retry_count = 0;
    let mut interval = time::interval(Duration::from_secs(
        app_config.ping_config.timeout_secs as u64,
    ));
    let telegram_bot = TelegramBot::new(app_config.telegram_config.telegram_api_token.to_string());
    loop {
        let rx_result = rx.is_empty();
        if !rx_result {
            let _message = rx.recv().await.unwrap();
        }
        let output_ping = Command::new("ping")
            .arg(&addr.address)
            .arg("-c")
            .arg("1")
            .output()
            .await;
        match output_ping {
            Ok(output) => {
                if output.status.success() {
                    info!("host: {} available \n", addr.address);
                } else {
                    retry_count += 1;
                    let message = format!(
                        "🔥🔥🔥 HOST: {} - {} UNAVAILABLE 🔥🔥🔥\n",
                        addr.address, addr.description
                    );
                    warn!(message);
                    if retry_count > app_config.request_config.retry -1  {
                        send_telegram_alert(
                            &message,
                            &telegram_bot,
                            &app_config.telegram_config.telegram_chat_ids,
                        )
                        .await;
                        retry_count = 0;
                        thread::sleep(Duration::from_secs(
                            app_config.ping_config.sleep_after_alert_secs as u64,
                        ))
                    }
                }
            }
            Err(err) => {
                let message = format!("error request to addr: {}, error: {}", addr.address, err);
                error!("error ping err: {}", err);
                send_telegram_alert(
                    &message,
                    &telegram_bot,
                    &app_config.telegram_config.telegram_chat_ids,
                )
                .await;
                thread::sleep(Duration::from_secs(
                    app_config.ping_config.sleep_after_alert_secs as u64,
                ))
            }
        }
        interval.tick().await;
    }
}

async fn handle_telegram_command(
    telegram_update: &TelegramUpdate,
    _telegram_bot: &TelegramBot,
) -> Vec<Rc<dyn TelegramCommand>> {
    {
        let result: Vec<Rc<dyn TelegramCommand>> = Vec::new();
        for entity in telegram_update.message.entities.iter() {
            if &entity.message_type == "bot_command" {
                match &telegram_update.message.text {
                    Some(text) => match text.as_str() {
                        "" => {}
                        &_ => {}
                    },
                    _ => {}
                };
            }
        }
        return result;
    }
}
