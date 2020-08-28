#![deny(clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub use stable_eyre::{eyre, Report};

mod substituting_string;
pub use substituting_string::SubstitutingString;

//mod serialization;
mod config;
mod discord;

#[tokio::main]
async fn main() -> Result<(), Report> {
    env_logger::from_env(env_logger::Env::default().default_filter_or("info")).init();
    stable_eyre::install()?;

    let config = config::Config::from_file(
        std::env::var("CONFIG_PATH").unwrap_or_else(|_| String::from("config.toml")),
    )
    .await?;

    let mut discord = discord::Discord::new(&config.discord).await?;
    discord.run().await?;

    Ok(())
}
