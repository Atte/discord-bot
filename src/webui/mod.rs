use crate::{
    config::{DiscordConfig, WebUIConfig},
    Result,
};

mod auth;
mod r#static;

pub async fn run(config: WebUIConfig, discord_config: DiscordConfig) -> Result<()> {
    let vega = rocket::build().manage(config);
    let vega = r#static::init(vega);
    let vega = auth::init(vega, &discord_config)?;
    vega.launch().await?;
    Ok(())
}
