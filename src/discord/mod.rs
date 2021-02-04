use crate::{config::DiscordConfig, eyre::eyre, Result};
use serenity::{
    client::{bridge::gateway::GatewayIntents, Client, Context},
    framework::StandardFramework,
    prelude::TypeMapKey,
};

mod commands;
mod event_handler;
mod hooks;
pub mod limits;
mod log_channel;
mod stats;
mod sticky_roles;

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
    pub async fn try_new(config: DiscordConfig, db: mongodb::Database) -> Result<Self> {
        let framework = StandardFramework::new()
            .configure(|c| {
                c.prefix(config.command_prefix.as_ref())
                    .owners(config.owners.clone())
                    .blocked_users(config.blocked_users.clone())
                    .allowed_channels(config.command_channels.clone())
                    .case_insensitivity(true)
            })
            .normal_message(hooks::normal_message)
            .unrecognised_command(hooks::unrecognised_command)
            .on_dispatch_error(hooks::dispatch_error)
            .group(&commands::HORSE_GROUP)
            .group(&commands::RANKS_GROUP)
            .group(&commands::MISC_GROUP)
            .help(&commands::HELP_COMMAND);

        let client = Client::builder(&config.token)
            .intents(GatewayIntents::all())
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
