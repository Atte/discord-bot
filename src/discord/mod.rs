use crate::config::Config;
use color_eyre::eyre::{eyre, Result};
use serenity::{
    client::{Client, Context},
    framework::StandardFramework,
    model::gateway::GatewayIntents,
    prelude::TypeMapKey,
};

#[cfg(feature = "openai")]
use crate::openai::{OpenAi, OpenAiKey};

#[cfg(feature = "openai")]
use std::sync::Arc;

mod commands;
mod event_handler;
mod hooks;
pub mod limits;
mod log_channel;
mod rules_check;
mod safe_reply;
mod stats;
mod sticky_roles;

pub use stats::COLLECTION_NAME as STATS_COLLECTION_NAME;

#[derive(Debug)]
pub struct ActivityKey;

impl TypeMapKey for ActivityKey {
    type Value = String;
}

#[derive(Debug)]
struct ConfigKey;

impl TypeMapKey for ConfigKey {
    type Value = Config;
}

#[derive(Debug)]
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

pub async fn get_data_or_insert_with<T, F>(ctx: &Context, f: F) -> T::Value
where
    T: TypeMapKey,
    T::Value: Clone,
    F: FnOnce() -> T::Value,
{
    let mut data = ctx.data.write().await;
    data.entry::<T>().or_insert_with(f).clone()
}

pub struct Discord {
    pub client: Client,
}

impl Discord {
    pub async fn try_new(
        config: Config,
        db: mongodb::Database,
        #[cfg(feature = "openai")] openai: OpenAi,
    ) -> Result<Self> {
        let framework = StandardFramework::new()
            .configure(|c| {
                c.prefix(config.discord.command_prefix.to_string())
                    .owners(config.discord.owners.clone())
                    .blocked_users(config.discord.blocked_users.clone())
                    .allowed_channels(config.discord.command_channels.clone())
                    .case_insensitivity(true)
            })
            .normal_message(hooks::normal_message)
            .unrecognised_command(hooks::unrecognised_command)
            .on_dispatch_error(hooks::dispatch_error)
            .after(hooks::after)
            .group(&commands::HORSE_GROUP)
            .group(&commands::RANKS_GROUP)
            .group(&commands::MISC_GROUP)
            .help(&commands::HELP_COMMAND);

        let builder = Client::builder(&config.discord.token, GatewayIntents::all())
            .cache_settings(|c| c.max_messages(1024))
            .event_handler(event_handler::Handler)
            .framework(framework)
            .type_map_insert::<ActivityKey>(String::new())
            .type_map_insert::<ConfigKey>(config)
            .type_map_insert::<DbKey>(db);

        #[cfg(feature = "openai")]
        let builder = builder.type_map_insert::<OpenAiKey>(Arc::new(openai));

        Ok(Self {
            client: builder.await?,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        self.client.start().await?;
        Ok(())
    }
}
