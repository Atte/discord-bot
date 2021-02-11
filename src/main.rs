#![deny(clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![recursion_limit = "512"]

#[cfg(feature = "nightly")]
use eyre::{self, Result};
#[cfg(not(feature = "nightly"))]
use stable_eyre::{eyre, Result};

use log::{error, info, warn};
use std::time::Duration;
use tokio::time::sleep;

mod substituting_string;
mod util;
use substituting_string::SubstitutingString;

mod berrytube;
mod config;
mod cron;
mod discord;
mod serialization;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    #[cfg(not(feature = "nightly"))]
    stable_eyre::install()?;

    let config = config::Config::from_file(
        std::env::var("CONFIG_PATH").unwrap_or_else(|_| String::from("config.toml")),
    )
    .await?;

    let mongo_client = mongodb::Client::with_uri_str(config.mongodb.uri.as_ref()).await?;
    let db = mongo_client.database(config.mongodb.database.as_ref());

    let mut discord = discord::Discord::try_new(config.discord, db).await?;

    let cron_rate = config.cron.rate;
    if cron_rate > 0 {
        let mut cron = cron::Cron::new(config.cron, discord.client.cache_and_http.http.clone());
        tokio::spawn(async move {
            loop {
                if let Err(report) = cron.run().await {
                    error!("Cron error: {}", report);
                }
                sleep(Duration::from_secs(cron_rate)).await;
            }
        });
    }

    if config.berrytube.enabled {
        let mut berrytube = berrytube::Berrytube::try_new(
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
                sleep(Duration::from_secs(10)).await;
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
        sleep(Duration::from_secs(60)).await;
    }
}
