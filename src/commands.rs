use core::fmt;
use serde::{Deserialize, Serialize};

use telegram_bot_rust::{BotCommand, TelegramBot};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Commands {
    StopAll,
    SleepAll,
    Stop,
    Sleep,
}

impl fmt::Display for Commands {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub trait TelegramCommand {
    fn handle(&self);
}

pub struct StopCommand {}

pub struct StopAllCommand {}

pub struct SleepAllCommand {}

impl TelegramCommand for StopCommand {
    fn handle(&self) {
        todo!()
    }
}

impl TelegramCommand for SleepCommand {
    fn handle(&self) {
        todo!()
    }
}

impl TelegramCommand for SleepAllCommand {
    fn handle(&self) {
        todo!()
    }
}

//
impl TelegramCommand for StopAllCommand {
    fn handle(&self) {
        todo!()
    }
}

pub struct SleepCommand {
    pub sleep_time: u64,
}

pub async fn set_telegram_commands(commands: &Vec<BotCommand>, telegram_bot: &TelegramBot) {
    let _ = telegram_bot
        .set_commands(commands)
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
}
