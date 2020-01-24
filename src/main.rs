#![deny(clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

use lazy_static::lazy_static;
use log::error;
use serenity::prelude::Mutex;
use std::sync::Arc;

mod config;
mod substituting_string;

lazy_static! {
    pub static ref CONFIG: config::Config = config::Config::from_file(
        ::std::env::var("CONFIG_PATH").unwrap_or_else(|_| "config.toml".to_owned())
    )
    .expect("Error loading config");
}

mod berrytube;
mod commands;
mod db;
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

    let database = Arc::new(Mutex::new({
        let mut conn = db::connect().expect("Error opening database");
        db::apply_migrations(&mut conn).expect("Error migrating database");
        conn
    }));

    let mut client = discord::create_client();
    client.data.write().insert::<db::DatabaseKey>(database);

    let berrytube_thread = berrytube::spawn(client.data.clone(), client.shard_manager.clone());
    if let Err(ref err) = berrytube_thread {
        error!("Error spawning Berrytube thread: {}", err);
    }

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

    if let Ok(handle) = berrytube_thread {
        handle.close();
    }
}
