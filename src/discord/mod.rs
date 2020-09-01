use crate::{config::DiscordConfig, eyre::eyre, Result};
use serenity::{
    client::{Client, Context},
    framework::StandardFramework,
    prelude::TypeMapKey,
};

mod commands;
mod event_handler;
pub mod limits;
mod log_channel;

pub struct ActivityKey;

impl TypeMapKey for ActivityKey {
    type Value = String;
}

struct DiscordConfigKey;

impl TypeMapKey for DiscordConfigKey {
    type Value = DiscordConfig;
}

struct DbKey;

impl TypeMapKey for DbKey {
    type Value = mongodb::Database;
}

pub async fn get_data<T>(ctx: &Context) -> Result<T::Value>
where
    T: TypeMapKey,
    T::Value: Clone,
{
    let data = ctx.data.read().await;
    data.get::<T>()
        .cloned()
        .ok_or_else(|| eyre!("get_data called with missing TypeMapKey"))
}

pub struct Discord {
    pub client: Client,
}

impl Discord {
    pub async fn try_new(config: DiscordConfig, db: mongodb::Database) -> Result<Self> {
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
            .group(&commands::HORSE_GROUP)
            .group(&commands::RANKS_GROUP)
            .group(&commands::MISC_GROUP)
            .help(&commands::HELP_COMMAND);

        let client = Client::new(&config.token)
            .event_handler(event_handler::Handler)
            .framework(framework)
            .type_map_insert::<ActivityKey>(String::new())
            .type_map_insert::<DiscordConfigKey>(config)
            .type_map_insert::<DbKey>(db)
            .await?;

        client.cache_and_http.cache.set_max_messages(1024).await;

        Ok(Self { client })
    }

    pub async fn run(&mut self) -> Result<()> {
        self.client.start().await?;
        Ok(())
    }
}
