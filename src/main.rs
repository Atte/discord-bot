#![deny(clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![recursion_limit = "512"]

use log::{error, info, warn, LevelFilter};
use stable_eyre::{eyre, Result};
use tokio::time::{delay_for, Duration};

mod substituting_string;
mod util;
use substituting_string::SubstitutingString;

//mod serialization;
mod berrytube;
mod config;
mod discord;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::from_env(
        env_logger::Env::default().default_filter_or(LevelFilter::Info.to_string()),
    )
    .init();
    stable_eyre::install()?;

    let config = config::Config::from_file(
        std::env::var("CONFIG_PATH").unwrap_or_else(|_| String::from("config.toml")),
    )
    .await?;

    let mongo_client = mongodb::Client::with_uri_str(config.mongodb.uri.as_ref()).await?;
    let db = mongo_client.database(config.mongodb.database.as_ref());

    let mut discord = discord::Discord::try_new(config.discord, db).await?;

    if config.berrytube.enabled {
        let berrytube = berrytube::Berrytube::try_new(
            &config.berrytube,
            discord.client.shard_manager.clone(),
            discord.client.data.clone(),
        )?;
        tokio::spawn(async move {
            loop {
                if let Err(report) = berrytube.run().await {
                    error!("Berrytube error: {}", report);
                } else {
                    warn!("Berrytube ended!");
                }
                delay_for(Duration::from_secs(10)).await;
            }
        });
    } else {
        info!("Berrytube is disabled in config");
    }

    loop {
        if let Err(report) = discord.run().await {
            error!("Discord error: {}", report);
        } else {
            warn!("Discord ended!");
        }
        delay_for(Duration::from_secs(60)).await;
    }
}
