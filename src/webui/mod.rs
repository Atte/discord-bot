use crate::{
    config::{DiscordConfig, WebUIConfig},
    Result,
};
use serenity::http::client::Http;
use std::sync::Arc;

mod auth;
mod r#static;
mod util;

pub struct WebUI {
    config: WebUIConfig,
    discord_config: DiscordConfig,
    http: Arc<Http>,
}

impl WebUI {
    pub fn new(config: WebUIConfig, discord_config: DiscordConfig, http: Arc<Http>) -> Self {
        Self {
            config,
            discord_config,
            http,
        }
    }

    pub async fn run(&self) -> Result<()> {
        let vega = rocket::build()
            .manage(self.config.clone())
            .manage(self.http.clone());
        let vega = r#static::init(vega);
        let vega = auth::init(vega, &self.discord_config)?;
        vega.launch().await?;
        Ok(())
    }
}
