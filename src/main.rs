use async_process::Command;
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use std::{env, net::IpAddr};
use tokio::time;
use tracing::{info, warn};
use tracing_attributes::instrument;
//use serde_json::Result;
use clap::Parser;
use std::fs;
use addr::{parse_domain_name, parse_dns_name};


#[derive(Serialize, Deserialize)]
struct AppConfig {
    addreses: Vec<String>,
    tiemout_secs: i64,
    telegram_api_token: String,
    teelgram_chat_ids: i64,
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
    let data = fs::read_to_string(config_path).expect("Should have been able to read the file");
    let config: AppConfig = serde_json::from_str(&data).unwrap();
    return config;
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = get_config();
    tracing_subscriber::fmt().try_init().unwrap();

    let mut tasks = Vec::new();
    for addr in config.addreses {
        match addr.parse() {
            Ok(IpAddr::V4(_addr)) => tasks.push(tokio::spawn(ping(_addr.to_string(), config.tiemout_secs))),
            Ok(IpAddr::V6(_addr)) => tasks.push(tokio::spawn(ping(_addr.to_string(), config.tiemout_secs))),
            Err(e) => {
                let domain = parse_domain_name(&addr);
                    match domain {
                        Ok(_addr) => tasks.push(tokio::spawn(ping(_addr.to_string(), config.tiemout_secs))),
                        Err(err) => panic!("{} parse to addr error: {} {}", addr, e, err)
                    }
            }
        }
    }
    join_all(tasks).await;
    Ok(())
}

#[instrument]
async fn ping(addr: String, timeout: i64) {
    let mut interval = time::interval(Duration::from_secs(10));
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
                    info!("host : {} available \n", addr);
                } else {
                    warn!("host : {} not available \n", addr) // Allert
                }
            }
            Err(_) => warn!("host : {} not available \n", addr), // Allert
        }
        interval.tick().await;
    }
}
