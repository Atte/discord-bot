#![cfg_attr(feature = "cargo-clippy", deny(clippy, clippy_pedantic))]
#![cfg_attr(feature = "cargo-clippy", allow(missing_docs_in_private_items))]

#[macro_use]
extern crate log;
extern crate env_logger;
#[macro_use]
extern crate serenity;

pub mod discord;
pub mod commands;

fn main() {
    env_logger::Builder::from_default_env()
        .filter(None, log::LevelFilter::Info)
        .init();
    discord::run();
}
