use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Commands {
    Stop,
    Sleep,
}

pub trait TelegramCommand{
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
