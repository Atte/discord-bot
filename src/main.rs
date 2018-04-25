#![cfg_attr(feature = "cargo-clippy", deny(clippy, clippy_pedantic))]
#![cfg_attr(feature = "cargo-clippy", allow(missing_docs_in_private_items, unreadable_literal))]

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

mod commands;
mod discord;
mod reddit;

fn main() {
    env_logger::Builder::from_default_env()
        .filter(None, log::LevelFilter::Info)
        .init();

    let reddit_thread = reddit::spawn();
    discord::run_forever();
    reddit_thread.join().expect("Error joining Reddit thread");
}
