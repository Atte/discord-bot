use super::{auth::SessionUser, BotGuilds};
use rocket::{get, http::Status, routes, serde::json::Json, Build, Rocket, State};
use serde::Serialize;
use serenity::{
    model::{
        guild::{GuildInfo, Role},
        id::GuildId,
        user::CurrentUser,
    },
    CacheAndHttp,
};
use std::sync::Arc;

pub fn init(vega: Rocket<Build>) -> Rocket<Build> {
    vega.mount("/", routes![user, guilds, guild_ranks])
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

    let mut roles: Vec<Role> = guild_id
        .roles(&discord.http)
        .await
        .map_err(|_| (Status::BadGateway, "can't fetch guild's roles"))?
        .into_values()
        .collect();
    roles.sort_by_key(|role| -role.position);

    let (current, available) = roles
        .into_iter()
        .partition(|role| member.roles.contains(&role.id));
    Ok(Json(GuildRanksResponse { current, available }))
}
