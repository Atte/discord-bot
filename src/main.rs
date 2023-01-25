#![warn(
    clippy::pedantic,
    future_incompatible,
    nonstandard_style,
    rust_2018_idioms,
    unused
)]
#![allow(clippy::module_name_repetitions)]

use color_eyre::eyre::Result;
use log::{error, info, warn};
use std::time::Duration;
use tokio::time::sleep;

mod substituting_string;
mod util;
use substituting_string::SubstitutingString;

#[cfg(feature = "berrytube")]
mod berrytube;
mod config;
#[cfg(feature = "cron")]
mod cron;
mod discord;
mod migrations;
#[cfg(feature = "openai")]
mod openai;
#[cfg(feature = "teamup")]
mod teamup;
#[cfg(feature = "webui")]
mod webui;
#[cfg(feature = "openai")]
mod word_chunks;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    color_eyre::install()?;

    let config = config::Config::from_file(
        std::env::var("CONFIG_PATH").unwrap_or_else(|_| String::from("config.toml")),
    )
    .await?;

    let mongo_client = mongodb::Client::with_uri_str(&config.mongodb.uri).await?;
    let db = mongo_client.database(config.mongodb.database.as_ref());
    migrations::mongo(&db).await?;

    #[cfg(feature = "openai")]
    let openai = openai::OpenAi::new(&config.openai);

    info!("Spawning Discord...");
    let mut discord = discord::Discord::try_new(
        config.clone(),
        db.clone(),
        #[cfg(feature = "openai")]
        openai,
    )
    .await?;

    #[cfg(feature = "webui")]
    {
        info!("Spawning web UI...");
        let webui =
            webui::WebUI::try_new(config.clone(), discord.client.cache_and_http.clone(), db)
                .await?;
        tokio::spawn(async move {
            loop {
                if let Err(report) = webui.run().await {
                    error!("Web UI error: {report:?}");
                } else {
                    warn!("Web UI ended!");
                }
                sleep(Duration::from_secs(10)).await;
            }
        });
    }

    #[cfg(feature = "cron")]
    {
        if config.cron.rate > 0 {
            info!("Spawning cron...");
            let mut cron = cron::Cron::new(config.cron, discord.client.cache_and_http.clone());
            tokio::spawn(async move {
                loop {
                    if let Err(report) = cron.run().await {
                        error!("Cron error: {report:?}");
                    }
                    sleep(Duration::from_secs(cron.rate)).await;
                }
            });
        }
    }

    #[cfg(feature = "berrytube")]
    {
        info!("Spawning BerryTube...");
        let mut berrytube = berrytube::Berrytube::try_new(
            &config.berrytube,
            discord.client.shard_manager.clone(),
            discord.client.data.clone(),
        )?;
        tokio::spawn(async move {
            loop {
                if let Err(report) = berrytube.run().await {
                    error!("Berrytube error: {report:?}");
                } else {
                    warn!("Berrytube ended!");
                }
                sleep(Duration::from_secs(10)).await;
            }
        });
    }

    #[cfg(feature = "teamup")]
    {
        for config in config.teamup {
            info!("Spawning Teamup for {}...", config.guild);
            let mut teamup = teamup::Teamup::new(config, discord.client.cache_and_http.clone());
            tokio::spawn(async move {
                sleep(Duration::from_secs(5)).await;
                loop {
                    if let Err(report) = teamup.run().await {
                        error!("Teamup error: {report:?}");
                    }
                    sleep(Duration::from_secs(60 * 60)).await;
                }
            });
        }
    }

    info!("Running Discord...");
    loop {
        if let Err(report) = discord.run().await {
            error!("Discord error: {report:?}");
        } else {
            warn!("Discord ended!");
        }
        sleep(Duration::from_secs(60)).await;
    }
}
