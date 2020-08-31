use crate::{config::DiscordConfig, Result};
use serenity::{
    client::{Client, Context},
    framework::StandardFramework,
    prelude::TypeMapKey,
};

mod commands;
mod event_handler;
mod log_channel;

pub const MAX_MESSAGE_LENGTH: usize = 2000;
pub const MAX_EMBED_DESC_LENGTH: usize = 2048;
pub const MAX_NICK_LENGTH: usize = 32;
pub const MAX_ACTIVITY_LENGTH: usize = 128;
pub const MAX_REPLY_LENGTH: usize = MAX_MESSAGE_LENGTH - MAX_NICK_LENGTH - 5; // extra space for

pub struct ActivityKey;

impl TypeMapKey for ActivityKey {
    type Value = String;
}

struct DiscordConfigKey;

impl TypeMapKey for DiscordConfigKey {
    type Value = DiscordConfig;
}

impl DiscordConfigKey {
    async fn get(ctx: &Context) -> DiscordConfig {
        let data = ctx.data.read().await;
        data.get::<Self>()
            .expect("DiscordConfig not in Context")
            .clone()
    }
}

pub struct Discord {
    pub client: Client,
}

impl Discord {
    pub async fn try_new(config: DiscordConfig) -> Result<Self> {
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
            .group(&commands::RANKS_GROUP)
            .group(&commands::MISC_GROUP)
            .help(&commands::HELP_COMMAND);

        let client = Client::new(&config.token)
            .event_handler(event_handler::Handler)
            .framework(framework)
            .type_map_insert::<ActivityKey>(String::new())
            .type_map_insert::<DiscordConfigKey>(config)
            .await?;

        client.cache_and_http.cache.set_max_messages(1024).await;

        Ok(Self { client })
    }

    pub async fn run(&mut self) -> Result<()> {
        self.client.start().await?;
        Ok(())
    }
}
