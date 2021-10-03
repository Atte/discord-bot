#![allow(clippy::let_unit_value, clippy::needless_pass_by_value)]

use crate::config::Config;
use anyhow::Result;
use indoc::indoc;
use nonzero_ext::nonzero;
use rocket::{data::ToByteUnit, fairing::AdHoc, http::Header, shield::Shield};
use serenity::{
    model::{guild::GuildInfo, id::GuildId},
    CacheAndHttp,
};
use std::{
    collections::{HashMap, HashSet},
    net::Ipv4Addr,
    sync::Arc,
};

mod auth;
mod json;
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
    config: Config,
    discord: Arc<CacheAndHttp>,
    guilds: BotGuilds,
}

impl WebUI {
    pub async fn try_new(config: Config, discord: Arc<CacheAndHttp>) -> Result<Self> {
        let guilds = discord
            .http
            .get_current_user()
            .await?
            .guilds(&discord.http)
            .await?
            .into_iter()
            .filter_map(|guild| {
                if config.webui.guilds.contains(&guild.id) {
                    Some((guild.id, guild))
                } else {
                    None
                }
            })
            .collect();
        Ok(Self {
            config,
            discord,
            guilds,
        })
    }

    pub async fn run(&self) -> Result<()> {
        let vega = rocket::custom(
            // "merge" = replace, "join" = set if not defined already
            rocket::Config::figment()
                .merge((
                    "shutdown",
                    rocket::config::Shutdown {
                        ctrlc: false,
                        #[cfg(unix)]
                        signals: HashSet::new(),
                        ..rocket::config::Shutdown::default()
                    },
                ))
                .merge((
                    "limits",
                    rocket::data::Limits::default()
                        .limit("form", 0.bytes())
                        .limit("data-form", 0.bytes())
                        .limit("file", 0.bytes())
                        .limit("string", 0.bytes())
                        .limit("bytes", 0.bytes())
                        .limit("json", 0.bytes())
                        .limit("msgpack", 0.bytes()),
                ))
                .join(("address", Ipv4Addr::UNSPECIFIED))
                .join(("port", 8787))
                .join(("ident", rocket::config::Ident::none())),
        )
        .manage(self.config.clone())
        .manage(self.discord.clone())
        .manage(self.guilds.clone())
        .manage(RateLimiter::keyed(governor::Quota::per_second(nonzero!(
            1_u32
        ))))
        .attach(util::ServerTimingFairing)
        .attach(
            // security headers are set manually below
            Shield::new(),
        )
        .attach(AdHoc::on_response(
            "custom headers",
            |_request, response| {
                Box::pin(async move {
                    const CSP: &str = indoc!(
                        r#"
                            default-src 'none'
                            script-src 'self'
                            style-src 'self'
                            connect-src 'self'
                            img-src data: https://cdn.discordapp.com
                            form-action 'self' https://discord.com/api/oauth2/authorize
                            base-uri 'self'
                            frame-ancestors 'none'
                            block-all-mixed-content
                            disown-opener
                        "#
                    );
                    response.set_header(Header::new(
                        "Content-Security-Policy",
                        CSP.trim().replace('\n', "; "),
                    ));

                    response.set_header(Header::new("X-XSS-Protection", "1; mode=block"));
                    response.set_header(Header::new("X-Frame-Options", "DENY"));
                    response.set_header(Header::new("X-Content-Type-Options", "nosniff"));
                    response.set_header(Header::new("Referrer-Policy", "no-referrer"));
                    response.set_header(Header::new(
                        "Permissions-Policy",
                        "payment=(), interest-cohort=()",
                    ));

                    response
                        .set_header(Header::new("Cross-Origin-Embedder-Policy", "require-corp"));
                    response.set_header(Header::new("Cross-Origin-Opener-Policy", "same-origin"));
                    response.set_header(Header::new("Cross-Origin-Resource-Policy", "same-origin"));

                    if !response.headers().contains("Cache-Control") {
                        response.set_header(Header::new("Cache-Control", "no-store, max-age=0"));
                    }
                })
            },
        ));
        let vega = r#static::init(vega);
        let vega = auth::init(vega, &self.config)?;
        let vega = me::init(vega);
        vega.launch().await?;
        Ok(())
    }
}
