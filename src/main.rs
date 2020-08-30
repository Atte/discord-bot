#![deny(clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

use stable_eyre::{eyre, Result};
use log::error;
use tokio::time::{delay_for, Duration};

mod substituting_string;
mod util;
use substituting_string::SubstitutingString;

//mod serialization;
mod config;
mod discord;
mod berrytube;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::from_env(env_logger::Env::default().default_filter_or("info")).init();
    stable_eyre::install()?;

    let config = config::Config::from_file(
        std::env::var("CONFIG_PATH").unwrap_or_else(|_| String::from("config.toml")),
    )
    .await?;

    let mut discord = discord::Discord::try_new(config.discord).await?;
    let mut berrytube = berrytube::Berrytube::new(discord.client.shard_manager.clone(), discord.client.data.clone());
    
    tokio::spawn(async move {
        loop {
            if let Err(report) = berrytube.run().await {
                error!("Berrytube error: {}", report);
            }
            delay_for(Duration::from_secs(10)).await;
        }
    });
    discord.run().await?;

    Ok(())
}
