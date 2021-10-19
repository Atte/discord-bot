use super::{
    util::{Json, SessionUser},
    BotGuilds,
};
use crate::config::Config;
use futures::future::join_all;
use itertools::Itertools;
use log::trace;
use rocket::{delete, get, http::Status, post, put, routes, Build, Rocket, State};
use serde::Serialize;
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

pub fn init(vega: Rocket<Build>) -> Rocket<Build> {
    vega.mount(
        "/api/guilds",
        routes![
            guilds,
            guild_ranks_post,
            guild_ranks_delete,
            guild_rules_put
        ],
    )
}

pub async fn guild_member(
    guild_id: GuildId,
    user_id: UserId,
    discord: &Arc<CacheAndHttp>,
) -> Option<Member> {
    if let Some(guild) = guild_id.to_guild_cached(&discord.cache).await {
        guild.members.get(&user_id).cloned()
    } else {
        guild_id.member(discord.clone(), user_id).await.ok()
    }
}

pub async fn guild_roles(
    guild_id: GuildId,
    discord: &Arc<CacheAndHttp>,
) -> Option<HashMap<RoleId, Role>> {
    if let Some(guild) = guild_id.to_guild_cached(&discord.cache).await {
        Some(guild.roles)
    } else {
        guild_id.roles(&discord.http).await.ok()
    }
}

pub async fn guild_channels(
    guild_id: GuildId,
    discord: &Arc<CacheAndHttp>,
) -> Option<HashMap<ChannelId, GuildChannel>> {
    if let Some(guild) = guild_id.to_guild_cached(&discord.cache).await {
        Some(guild.channels)
    } else {
        guild_id.channels(&discord.http).await.ok()
    }
}

async fn guild_rules(
    guild_id: GuildId,
    discord: &Arc<CacheAndHttp>,
    config: &Config,
) -> Option<Message> {
    let channels = guild_channels(guild_id, discord).await?;
    for channel_id in &config.discord.rules_channels {
        if let Some(channel) = channels.get(channel_id) {
            for message in channel
                .id
                .messages(&discord.http, |get| get.limit(10))
                .await
                .ok()?
            {
                if message.is_own(&discord.cache).await {
                    return Some(message);
                }
            }
        }
    }
    None
}

async fn ranks_from_roles(
    guild_id: GuildId,
    roles: impl IntoIterator<Item = Role> + Clone,
    discord: &Arc<CacheAndHttp>,
) -> Result<impl Iterator<Item = Role>, (Status, &'static str)> {
    let bot_member = guild_id
        .member(discord.clone(), discord.cache.current_user_id().await)
        .await
        .map_err(|_| (Status::BadGateway, "can't fetch bot member"))?;

    let cutoff_position = roles
        .clone()
        .into_iter()
        .filter_map(|role| {
            if bot_member.roles.contains(&role.id) {
                Some(role.position)
            } else {
                None
            }
        })
        .min()
        .ok_or((Status::InternalServerError, "empty roles for bot"))?;

    Ok(roles
        .into_iter()
        .filter(move |role| role.position < cutoff_position && !role.name.starts_with('@')))
}

async fn ranks_from_guild(
    guild_id: GuildId,
    discord: &Arc<CacheAndHttp>,
) -> Result<impl Iterator<Item = Role>, (Status, &'static str)> {
    let roles: Vec<_> = guild_roles(guild_id, discord)
        .await
        .ok_or((Status::InternalServerError, "can't find roles for guild"))?
        .into_values()
        .collect();
    Ok(ranks_from_roles(guild_id, roles, discord).await?)
}

#[derive(Serialize)]
struct GuildRanksResponse {
    current: Vec<Role>,
    available: Vec<Role>,
}

#[derive(Serialize)]
struct GuildsResponse {
    pub id: GuildId,
    pub icon: Option<String>,
    pub name: String,
    pub admin: bool,
    pub ranks: GuildRanksResponse,
    pub rules: Option<Message>,
}

#[get("/")]
async fn guilds(
    user: &SessionUser,
    discord: &State<Arc<CacheAndHttp>>,
    bot_guilds: &State<BotGuilds>,
    config: &State<Config>,
) -> Result<Json<Vec<GuildsResponse>>, (Status, &'static str)> {
    Ok(Json(
        join_all(bot_guilds.iter().map(|(guild_id, guild_info)| async move {
            let member = guild_member(*guild_id, user.id, discord).await?;
            let roles: Vec<_> = guild_roles(*guild_id, discord)
                .await?
                .into_values()
                .collect();
            let admin = roles
                .iter()
                .filter_map(|role| {
                    if role.has_permission(Permissions::ADMINISTRATOR) {
                        Some(role.id)
                    } else {
                        None
                    }
                })
                .any(|role_id| member.roles.contains(&role_id));
            let (current_ranks, available_ranks) = ranks_from_roles(*guild_id, roles, discord)
                .await
                .ok()?
                .partition(|role| member.roles.contains(&role.id));
            Some(GuildsResponse {
                id: guild_info.id,
                icon: guild_info.icon.clone(),
                name: guild_info.name.clone(),
                admin,
                ranks: GuildRanksResponse {
                    current: current_ranks,
                    available: available_ranks,
                },
                rules: if admin {
                    guild_rules(*guild_id, discord, &*config).await
                } else {
                    None
                },
            })
        }))
        .await
        .into_iter()
        .flatten()
        .sorted_by_key(|guild| usize::MAX - guild.ranks.current.len() - guild.ranks.available.len())
        .collect(),
    ))
}

#[post("/<guild_id>/ranks/<role_id>")]
async fn guild_ranks_post(
    guild_id: u64,
    role_id: u64,
    user: &SessionUser,
    discord: &State<Arc<CacheAndHttp>>,
) -> Result<Json<GuildRanksResponse>, (Status, &'static str)> {
    let guild_id = GuildId(guild_id);
    let mut member = user.member(guild_id).await?;

    let role = ranks_from_guild(guild_id, discord)
        .await?
        .find(|role| role.id == role_id)
        .ok_or((Status::BadRequest, "invalid role"))?;

    member
        .add_role(&discord.http, &role)
        .await
        .map_err(|_| (Status::BadGateway, "can't add member to role"))?;

    trace!(
        "{} ({}) joined rank {} ({})",
        user.tag(),
        user.id,
        role.name,
        role.id
    );

    let (current, available) = ranks_from_guild(guild_id, discord)
        .await?
        .partition(|role| member.roles.contains(&role.id));

    Ok(Json(GuildRanksResponse { current, available }))
}

#[delete("/<guild_id>/ranks/<role_id>")]
async fn guild_ranks_delete(
    guild_id: u64,
    role_id: u64,
    user: &SessionUser,
    discord: &State<Arc<CacheAndHttp>>,
) -> Result<Json<GuildRanksResponse>, (Status, &'static str)> {
    let guild_id = GuildId(guild_id);
    let mut member = user.member(guild_id).await?;

    let role = ranks_from_guild(guild_id, discord)
        .await?
        .find(|role| role.id == role_id)
        .ok_or((Status::BadRequest, "invalid role"))?;

    member
        .remove_role(&discord.http, &role)
        .await
        .map_err(|_| (Status::BadGateway, "can't remove member from role"))?;

    trace!(
        "{} ({}) left rank {} ({})",
        user.tag(),
        user.id,
        role.name,
        role.id
    );

    let (current, available) = ranks_from_guild(guild_id, discord)
        .await?
        .partition(|role| member.roles.contains(&role.id));

    Ok(Json(GuildRanksResponse { current, available }))
}

#[put("/<guild_id>/rules")]
async fn guild_rules_put(
    guild_id: u64,
    user: &SessionUser,
    discord: &State<Arc<CacheAndHttp>>,
) -> Result<Json<Option<Message>>, (Status, &'static str)> {
    let guild_id = GuildId(guild_id);
    let member = user.admin(guild_id).await?;

    Ok(Json(None))
}
