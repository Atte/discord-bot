#![deny(clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

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

    let mut client = discord::create_client();

    let reddit_thread = reddit::spawn(client.cache_and_http.http.clone());
    if let Err(ref err) = reddit_thread {
        error!("Error spawning Reddit thread: {}", err);
    }

    if let Err(err) = client.start() {
        error!("Error running the client: {}", err);
    }

    if let Ok(handle) = reddit_thread {
        if handle.join().is_err() {
            error!("Error joining Reddit thread");
        }
    }
}
