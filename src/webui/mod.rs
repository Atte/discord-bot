use crate::{
    config::{DiscordConfig, WebUIConfig},
    Result,
};
use rocket::routes;

mod r#static;
mod auth;

pub async fn run(config: WebUIConfig, discord_config: DiscordConfig) -> Result<()> {
    rocket::build()
        .manage(config)
        .manage(auth::client(&discord_config)?)
        .mount("/", routes![r#static::index, r#static::path, auth::redirect, auth::callback])
        .launch()
        .await?;
    Ok(())
}
