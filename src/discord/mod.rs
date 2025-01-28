use std::{sync::Arc, time::Duration};

use crate::config::Config;
use color_eyre::eyre::{eyre, Result};
use poise::{EditTracker, Framework, FrameworkOptions, PrefixFrameworkOptions};
use serenity::{
    cache::Settings as CacheSettings, client::Client, model::gateway::GatewayIntents,
    prelude::TypeMapKey,
};

#[cfg(feature = "openai")]
use crate::openai::{OpenAi, OpenAiKey};

pub mod automod;
pub mod commands;
mod event_handler;
pub mod limits;
mod log_channel;
mod stats;
mod sticky_roles;

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
pub struct DbKey;

impl TypeMapKey for DbKey {
    type Value = mongodb::Database;
}

pub async fn get_data<T>(ctx: &serenity::all::Context) -> Result<T::Value>
where
    T: TypeMapKey,
    T::Value: Clone,
{
    let data = ctx.data.read().await;
    data.get::<T>()
        .cloned()
        .ok_or_else(|| eyre!("get_data called with missing TypeMapKey"))
}

pub async fn get_data_or_insert_with<T, F>(ctx: &serenity::all::Context, f: F) -> T::Value
where
    T: TypeMapKey,
    T::Value: Clone,
    F: FnOnce() -> T::Value,
{
    let mut data = ctx.data.write().await;
    data.entry::<T>().or_insert_with(f).clone()
}

#[derive(Debug)]
struct PoiseData {
    config: Config,
}

type Context<'a> = poise::Context<'a, PoiseData, crate::Error>;

pub struct Discord {
    pub client: Client,
}

impl Discord {
    pub async fn try_new(
        config: Config,
        db: mongodb::Database,
        #[cfg(feature = "openai")] openai: OpenAi,
    ) -> Result<Self> {
        let setup_config = config.clone();
        let framework = Framework::<PoiseData, crate::Error>::builder()
            .setup(|_ctx, _ready, _framework| {
                Box::pin(async move {
                    Ok(PoiseData {
                        config: setup_config,
                    })
                })
            })
            .options(FrameworkOptions {
                prefix_options: PrefixFrameworkOptions {
                    prefix: Some(config.discord.command_prefix.to_string()),
                    mention_as_prefix: false,
                    edit_tracker: Some(Arc::new(EditTracker::for_timespan(Duration::from_secs(
                        3600,
                    )))),
                    ..Default::default()
                },
                owners: config.discord.owners.clone(),
                command_check: Some(|ctx| {
                    Box::pin(async move {
                        Ok(ctx
                            .data()
                            .config
                            .discord
                            .command_channels
                            .contains(&ctx.channel_id())
                            && !ctx
                                .data()
                                .config
                                .discord
                                .blocked_users
                                .contains(&ctx.author().id))
                    })
                }),
                commands: commands::get_all(),
                on_error: |err| {
                    Box::pin(async move {
                        log::warn!("{err}");
                        if let Some(ctx) = err.ctx() {
                            let _ = ctx.reply(err.to_string()).await;
                        }
                    })
                },
                ..Default::default()
            })
            .build();

        let mut cache_settings = CacheSettings::default();
        cache_settings.max_messages = 1024;

        let builder = Client::builder(&config.discord.token, GatewayIntents::all())
            .cache_settings(cache_settings)
            .event_handler(event_handler::Handler)
            .framework(framework)
            .type_map_insert::<ActivityKey>(String::new())
            .type_map_insert::<ConfigKey>(config)
            .type_map_insert::<DbKey>(db);

        #[cfg(feature = "openai")]
        let builder = builder.type_map_insert::<OpenAiKey>(std::sync::Arc::new(openai));

        Ok(Self {
            client: builder.await?,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        self.client.start().await?;
        Ok(())
    }
}
