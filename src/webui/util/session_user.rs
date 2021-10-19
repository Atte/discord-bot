use super::super::guilds::{guild_member, guild_roles};
use super::super::BotGuilds;
use super::{Json, RateLimiter};
use crate::config::Config;
use futures::future::join_all;
use itertools::Itertools;
use log::trace;
use rocket::{delete, get, http::Status, post, put, routes, Build, Rocket, State};
use rocket::{
    outcome::IntoOutcome,
    request::{FromRequest, Outcome, Request},
};
use serde::Serialize;
use serenity::model::user::CurrentUser;
use serenity::{
    model::{
        channel::{GuildChannel, Message},
        guild::{Member, Role},
        id::{ChannelId, GuildId, RoleId, UserId},
        permissions::Permissions,
    },
    CacheAndHttp,
};
use std::{collections::HashMap, sync::Arc};
use std::{
    convert::Infallible,
    ops::{Deref, DerefMut},
};

pub type AuthError = (Status, &'static str);

pub struct SessionUser {
    user: CurrentUser,
    bot_guilds: BotGuilds,
    discord: Arc<CacheAndHttp>,
}

impl SessionUser {
    pub async fn member(&self, guild_id: GuildId) -> Result<Member, AuthError> {
        if !self.bot_guilds.contains_key(&guild_id) {
            return Err((Status::BadRequest, "invalid guild"));
        }

        Ok(guild_member(guild_id, self.user.id, &self.discord)
            .await
            .ok_or((Status::BadGateway, "can't fetch member"))?)
    }

    pub async fn admin(&self, guild_id: GuildId) -> Result<Member, AuthError> {
        let member = self.member(guild_id).await?;
        if guild_roles(guild_id, &self.discord)
            .await
            .ok_or((Status::BadGateway, "can't fetch roles"))?
            .into_values()
            .filter_map(|role| {
                if role.has_permission(Permissions::ADMINISTRATOR) {
                    Some(role.id)
                } else {
                    None
                }
            })
            .any(|role_id| member.roles.contains(&role_id))
        {
            Ok(member)
        } else {
            Err((Status::Forbidden, "not an admin"))
        }
    }
}

impl Deref for SessionUser {
    type Target = CurrentUser;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.user
    }
}

impl DerefMut for SessionUser {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.user
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for &'r SessionUser {
    type Error = Infallible;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        request
            .local_cache_async(async {
                let bot_guilds = request
                    .guard::<&State<BotGuilds>>()
                    .await
                    .succeeded()?
                    .inner()
                    .clone();

                let discord = request
                    .guard::<&State<Arc<CacheAndHttp>>>()
                    .await
                    .succeeded()?
                    .inner()
                    .clone();

                let user = request
                    .cookies()
                    .get_private("user")
                    .and_then(|cookie| serde_json::from_str::<CurrentUser>(cookie.value()).ok())
                    .map(move |user| SessionUser {
                        user,
                        bot_guilds,
                        discord,
                    });

                if let Some(ref user) = user {
                    request
                        .guard::<&State<RateLimiter<u64>>>()
                        .await
                        .expect("no RateLimiter in request state")
                        .apply_to_request(&user.id.0, request)
                        .await;
                }

                user
            })
            .await
            .as_ref()
            .or_forward(())
    }
}
