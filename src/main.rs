#![deny(clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

use stable_eyre::{eyre, Result};

mod substituting_string;
mod util;
use substituting_string::SubstitutingString;

//mod serialization;
mod config;
mod discord;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::from_env(env_logger::Env::default().default_filter_or("info")).init();
    stable_eyre::install()?;

    let config = config::Config::from_file(
        std::env::var("CONFIG_PATH").unwrap_or_else(|_| String::from("config.toml")),
    )
    .await?;

    let mut discord = discord::Discord::new(config.discord).await?;
    // TODO: pass discord.client.data to now-playing handler for setting InitialActivityKey
    discord.run().await?;

    Ok(())
}
