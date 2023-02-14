use async_process::Command;
use futures::future::join_all;
use std::net::IpAddr;
use std::time::Duration;
use tokio::time;
use tracing::{info, warn};
use tracing_attributes::instrument;

struct AppConfig<'a> {
    ips: Box<[&'a str]>,
    tiemout_secs: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ips = ["8.8.8.8", "192.168.1.122"];
    tracing_subscriber::fmt().try_init().unwrap();

    let mut tasks = Vec::new();
    for ip in &ips {
        match ip.parse() {
            Ok(IpAddr::V4(_addr)) => tasks.push(tokio::spawn(ping(ip))),
            Ok(IpAddr::V6(_addr)) => tasks.push(tokio::spawn(ping(ip))),
            Err(e) => println!("{} parse to ipaddr error: {}", ip, e),
        }
    }
    join_all(tasks).await;
    Ok(())
}

#[instrument]
async fn ping(addr: &str) {
    let mut interval = time::interval(Duration::from_secs(10));
    loop {
        let output_ping = Command::new("ping")
            .arg(addr)
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
