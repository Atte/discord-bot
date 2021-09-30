#![allow(clippy::let_unit_value, clippy::needless_pass_by_value)]

use crate::config::WebUIConfig;
use anyhow::Result;
use nonzero_ext::nonzero;
use rocket::{
    fairing::AdHoc,
    http::Header,
    shield::{self, Shield},
};
use serenity::{
    model::{guild::GuildInfo, id::GuildId},
    CacheAndHttp,
};
use std::{collections::HashMap, sync::Arc};

mod json;
use json::Json;

mod auth;
mod bot;
mod me;
mod r#static;
mod util;

pub type BotGuilds = HashMap<GuildId, GuildInfo>;
pub type RateLimiter = governor::RateLimiter<
    u64,
    governor::state::keyed::DefaultKeyedStateStore<u64>,
    governor::clock::DefaultClock,
>;

pub struct WebUI {
    config: WebUIConfig,
    discord: Arc<CacheAndHttp>,
    guilds: BotGuilds,
}

impl WebUI {
    pub async fn try_new(config: WebUIConfig, discord: Arc<CacheAndHttp>) -> Result<Self> {
        let guilds = discord
            .http
            .get_current_user()
            .await?
            .guilds(&discord.http)
            .await?
            .into_iter()
            .map(|guild| (guild.id, guild))
            .collect();
        Ok(Self {
            config,
            discord,
            guilds,
        })
    }

    pub async fn run(&self) -> Result<()> {
        let vega = rocket::build()
            .manage(self.config.clone())
            .manage(self.discord.clone())
            .manage(self.guilds.clone())
            .manage(RateLimiter::keyed(governor::Quota::per_second(nonzero!(
                1_u32
            ))))
            .attach(
                Shield::default()
                    .enable(shield::Referrer::NoReferrer)
                    .disable::<shield::Hsts>(),
            )
            .attach(AdHoc::on_response("Cache-Control", |_request, response| {
                Box::pin(async move {
                    response.set_header(Header::new("Cache-Control", "no-store"));
                })
            }));
        let vega = r#static::init(vega);
        let vega = bot::init(vega);
        let vega = auth::init(vega, &self.config)?;
        let vega = me::init(vega);
        vega.launch().await?;
        Ok(())
    }
}
