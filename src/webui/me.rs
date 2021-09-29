use super::{auth::SessionUser, BotGuilds, Json};
use rocket::{delete, get, http::Status, post, routes, Build, Rocket, State};
use serde::Serialize;
use serenity::{
    model::{
        guild::{GuildInfo, Role},
        id::{GuildId, RoleId},
        user::CurrentUser,
    },
    CacheAndHttp,
};
use std::sync::Arc;

pub fn init(vega: Rocket<Build>) -> Rocket<Build> {
    vega.mount(
        "/",
        routes![
            user,
            guilds,
            guild_ranks,
            guild_ranks_add,
            guild_ranks_delete
        ],
    )
}

#[get("/me/user")]
fn user(user: &SessionUser) -> Json<CurrentUser> {
    Json(user.0.clone())
}

#[get("/me/guilds")]
async fn guilds(
    user: &SessionUser,
    discord: &State<Arc<CacheAndHttp>>,
    bot_guilds: &State<BotGuilds>,
) -> Result<Json<Vec<GuildInfo>>, (Status, &'static str)> {
    let guilds = user
        .0
        .guilds(&discord.http)
        .await
        .map_err(|_| (Status::BadGateway, "can't fetch user's guilds"))?;
    Ok(Json(
        guilds
            .into_iter()
            .filter(|guild| bot_guilds.contains_key(&guild.id))
            .collect(),
    ))
}

async fn ranks_from_guild(
    guild_id: GuildId,
    discord: Arc<CacheAndHttp>,
) -> Result<impl Iterator<Item = Role>, (Status, &'static str)> {
    let bot_member = guild_id
        .member(discord.clone(), discord.cache.current_user_id().await)
        .await
        .map_err(|_| (Status::BadGateway, "can't fetch bot member"))?;

    let roles = guild_id
        .roles(&discord.http)
        .await
        .map_err(|_| (Status::BadGateway, "can't fetch guild's roles"))?;

    let cutoff_position = roles
        .values()
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
        .into_values()
        .filter(move |role| role.position < cutoff_position && !role.name.starts_with('@')))
}

#[derive(Serialize)]
struct GuildRanksResponse {
    current: Vec<Role>,
    available: Vec<Role>,
}

#[get("/me/guilds/<guild_id>/ranks")]
async fn guild_ranks(
    guild_id: u64,
    user: &SessionUser,
    discord: &State<Arc<CacheAndHttp>>,
    bot_guilds: &State<BotGuilds>,
) -> Result<Json<GuildRanksResponse>, (Status, &'static str)> {
    let guild_id = GuildId(guild_id);
    if !bot_guilds.contains_key(&guild_id) {
        return Err((Status::BadRequest, "invalid guild"));
    }

    let member = guild_id
        .member(discord.inner().clone(), user.0.id)
        .await
        .map_err(|_| (Status::BadGateway, "can't fetch member"))?;

    let (current, available) = ranks_from_guild(guild_id, discord.inner().clone())
        .await?
        .partition(|role| member.roles.contains(&role.id));

    Ok(Json(GuildRanksResponse { current, available }))
}

#[post("/me/guilds/<guild_id>/ranks/<role_id>")]
async fn guild_ranks_add(
    guild_id: u64,
    role_id: u64,
    user: &SessionUser,
    discord: &State<Arc<CacheAndHttp>>,
    bot_guilds: &State<BotGuilds>,
) -> Result<(), (Status, &'static str)> {
    let guild_id = GuildId(guild_id);
    if !bot_guilds.contains_key(&guild_id) {
        return Err((Status::BadRequest, "invalid guild"));
    }

    let role_id = RoleId(role_id);
    if !ranks_from_guild(guild_id, discord.inner().clone())
        .await?
        .any(|role| role.id == role_id)
    {
        return Err((Status::BadRequest, "invalid role"));
    }

    guild_id
        .member(discord.inner().clone(), user.0.id)
        .await
        .map_err(|_| (Status::BadGateway, "can't fetch member"))?
        .add_role(&discord.http, role_id)
        .await
        .map_err(|_| (Status::BadGateway, "can't add member to role"))?;

    Ok(())
}

#[delete("/me/guilds/<guild_id>/ranks/<role_id>")]
async fn guild_ranks_delete(
    guild_id: u64,
    role_id: u64,
    user: &SessionUser,
    discord: &State<Arc<CacheAndHttp>>,
    bot_guilds: &State<BotGuilds>,
) -> Result<(), (Status, &'static str)> {
    let guild_id = GuildId(guild_id);
    if !bot_guilds.contains_key(&guild_id) {
        return Err((Status::BadRequest, "invalid guild"));
    }

    let role_id = RoleId(role_id);
    if !ranks_from_guild(guild_id, discord.inner().clone())
        .await?
        .any(|role| role.id == role_id)
    {
        return Err((Status::BadRequest, "invalid role"));
    }

    guild_id
        .member(discord.inner().clone(), user.0.id)
        .await
        .map_err(|_| (Status::BadGateway, "can't fetch member"))?
        .remove_role(&discord.http, role_id)
        .await
        .map_err(|_| (Status::BadGateway, "can't add member to role"))?;

    Ok(())
}
