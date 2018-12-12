#![deny(clippy::all, clippy::pedantic)]
#![allow(clippy::missing_docs_in_private_items, clippy::stutter)]

// TODO: remove these once macro dependencies are handled properly
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate serde_derive;

use lazy_static::lazy_static;
use log::error;

mod cache;
mod config;
mod substituting_string;

lazy_static! {
    pub static ref CONFIG: config::Config = config::Config::from_file(
        ::std::env::var("CONFIG_PATH").unwrap_or_else(|_| "config.toml".to_owned())
    )
    .expect("Error loading config");
    pub static ref CACHE: cache::Cache =
        cache::Cache::from_file(&CONFIG.cache_path).expect("Error loading cache");
}

mod commands;
mod discord;
mod discord_eventhandler;
mod reddit;
mod serialization;
mod util;

fn main() {
    env_logger::Builder::from_default_env()
        .filter(None, log::LevelFilter::Info)
        .init();

    lazy_static::initialize(&CONFIG);
    lazy_static::initialize(&CACHE);

    let reddit_thread = reddit::spawn();
    if let Err(ref err) = reddit_thread {
        error!("Error spawning Reddit thread: {}", err);
    }

    discord::run_forever();

    if let Ok(handle) = reddit_thread {
        if handle.join().is_err() {
            error!("Error joining Reddit thread");
        }
    }
}
