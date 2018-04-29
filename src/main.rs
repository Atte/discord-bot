#![cfg_attr(feature = "cargo-clippy", deny(clippy, clippy_pedantic))]
#![cfg_attr(feature = "cargo-clippy", allow(missing_docs_in_private_items, stutter))]

#[macro_use]
extern crate log;
extern crate env_logger;
#[macro_use]
extern crate serenity;
extern crate reqwest;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate toml;
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate lazy_static;
extern crate regex;
#[macro_use]
extern crate maplit;

mod cache;
mod config;
mod substituting_string;

lazy_static! {
    pub static ref CONFIG: config::Config = config::Config::from_file(
        ::std::env::var("CONFIG_PATH").unwrap_or_else(|_| "config.toml".to_owned())
    ).expect("Error loading config");
    pub static ref CACHE: cache::Cache =
        cache::Cache::from_file(&CONFIG.cache_path).expect("Error loading cache");
}

mod commands;
mod discord;
mod discord_eventhandler;
mod reddit;
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
