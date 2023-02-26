use serde::{Deserialize, Serialize};
use telegram_bot_rust::{BotCommand, TelegramBot};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Commands {
    Stop,
    Sleep,
}

pub trait TelegramCommand {
    fn handle(&self);

    fn get_command_key(&self) -> Commands;
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StopCommand {}

impl TelegramCommand for StopCommand {
    fn handle(&self) {
        todo!()
    }

    #[inline]
    fn get_command_key(&self) -> Commands {
        return Commands::Stop;
    }
}

impl TelegramCommand for SleepCommand {
    fn handle(&self) {
        todo!()
    }

    #[inline]
    fn get_command_key(&self) -> Commands {
        return Commands::Sleep;
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SleepCommand {
    sleep_time: u64,
}

pub async fn set_telegram_commands(commands: &Vec<BotCommand>, telegram_bot: &TelegramBot) {
    let gg = telegram_bot
        .set_commands(commands)
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
}
