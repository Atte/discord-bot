#![deny(clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

mod substituting_string;
//mod serialization;
mod event_handler;

#[tokio::main]
async fn main() {
    let token = "TODO";

    let mut client = serenity::Client::new(token)
        .event_handler(event_handler::Handler)
        .await
        .expect("Unable to create Discord client");
}
