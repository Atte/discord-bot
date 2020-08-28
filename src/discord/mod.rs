use crate::{config::DiscordConfig, eyre::Result};
use serenity::{framework::StandardFramework, Client};

mod commands;
mod event_handler;

pub struct Discord {
    client: Client,
}

impl Discord {
    pub async fn new(config: &DiscordConfig) -> Result<Discord> {
        let framework = StandardFramework::new()
            .configure(|c| c.prefix(config.command_prefix.as_ref()))
            .group(&commands::MISC_GROUP);

        let client = Client::new(&config.token)
            .event_handler(event_handler::Handler)
            .framework(framework)
            .await?;

        Ok(Discord { client })
    }

    pub async fn run(&mut self) -> Result<()> {
        self.client.start().await?;
        Ok(())
    }
}
