use crate::{config::DiscordConfig, Result};
use serenity::{framework::StandardFramework, prelude::TypeMapKey, Client};
use std::sync::Arc;
use tokio::sync::RwLock;

mod commands;
mod event_handler;

pub struct InitialActivityKey;

impl TypeMapKey for InitialActivityKey {
    type Value = String;
}

pub struct Discord {
    client: Client,
}

impl Discord {
    pub async fn new(
        config: &DiscordConfig,
    ) -> Result<Discord> {
        let framework = StandardFramework::new()
            .configure(|c| {
                c.prefix(config.command_prefix.as_ref())
                    .owners(config.owners.clone())
                    .blocked_users(config.blocked_users.clone())
                    .allowed_channels(config.command_channels.clone())
                    .case_insensitivity(true)
            })
            .bucket("derpi", |b| b.delay(1).time_span(10).limit(5))
            .await
            .group(&commands::MISC_GROUP)
            .help(&commands::HELP_COMMAND);

        let client = Client::new(&config.token)
            .event_handler(event_handler::Handler)
            .framework(framework)
            .type_map_insert::<InitialActivityKey>(String::new())
            .await?;

        client.cache_and_http.cache.set_max_messages(1024).await;

        Ok(Discord { client })
    }

    pub async fn run(&mut self) -> Result<()> {
        self.client.start().await?;
        Ok(())
    }
}
