use super::{auth::SessionUser, json::Json, BotGuilds};
use futures::future::join_all;
use itertools::Itertools;
use log::trace;
use rocket::{delete, get, http::Status, post, routes, Build, Rocket, State};
use serde::Serialize;
use serenity::{
    model::{
        guild::{Member, Role},
        id::{GuildId, RoleId, UserId},
        permissions::Permissions,
        user::CurrentUser,
    },
    CacheAndHttp,
};
use std::{collections::HashMap, sync::Arc};

pub fn init(vega: Rocket<Build>) -> Rocket<Build> {
    vega.mount(
        "/api/me",
        routes![user, guilds, guild_ranks_add, guild_ranks_delete],
    )
}

#[get("/user")]
fn user(user: &SessionUser) -> Json<&CurrentUser> {
    Json(&*user)
}

async fn guild_roles(
    guild_id: GuildId,
    discord: &Arc<CacheAndHttp>,
) -> Option<HashMap<RoleId, Role>> {
    if let Some(guild) = guild_id.to_guild_cached(&discord.cache).await {
        Some(guild.roles)
    } else {
        guild_id.roles(&discord.http).await.ok()
    }
}

async fn guild_member(
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
}

#[get("/guilds")]
async fn guilds(
    user: &SessionUser,
    discord: &State<Arc<CacheAndHttp>>,
    bot_guilds: &State<BotGuilds>,
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
                ranks: GuildRanksResponse {
                    current: current_ranks,
                    available: available_ranks,
                },
                admin,
            })
        }))
        .await
        .into_iter()
        .flatten()
        .sorted_by_key(|guild| usize::MAX - guild.ranks.current.len() - guild.ranks.available.len())
        .collect(),
    ))
}

#[post("/guilds/<guild_id>/ranks/<role_id>")]
async fn guild_ranks_add(
    guild_id: u64,
    role_id: u64,
    user: &SessionUser,
    discord: &State<Arc<CacheAndHttp>>,
    bot_guilds: &State<BotGuilds>,
) -> Result<Json<GuildRanksResponse>, (Status, &'static str)> {
    let guild_id = GuildId(guild_id);
    if !bot_guilds.contains_key(&guild_id) {
        return Err((Status::BadRequest, "invalid guild"));
    }

    let role = ranks_from_guild(guild_id, discord)
        .await?
        .find(|role| role.id == role_id)
        .ok_or((Status::BadRequest, "invalid role"))?;

    let mut member = guild_member(guild_id, user.id, discord)
        .await
        .ok_or((Status::BadGateway, "can't fetch member"))?;

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

#[delete("/guilds/<guild_id>/ranks/<role_id>")]
async fn guild_ranks_delete(
    guild_id: u64,
    role_id: u64,
    user: &SessionUser,
    discord: &State<Arc<CacheAndHttp>>,
    bot_guilds: &State<BotGuilds>,
) -> Result<Json<GuildRanksResponse>, (Status, &'static str)> {
    let guild_id = GuildId(guild_id);
    if !bot_guilds.contains_key(&guild_id) {
        return Err((Status::BadRequest, "invalid guild"));
    }

    let role = ranks_from_guild(guild_id, discord)
        .await?
        .find(|role| role.id == role_id)
        .ok_or((Status::BadRequest, "invalid role"))?;

    let mut member = guild_member(guild_id, user.id, discord)
        .await
        .ok_or((Status::BadGateway, "can't fetch member"))?;

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
