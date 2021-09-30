use super::Json;
use rocket::{get, routes, Build, Rocket, State};
use serenity::{model::user::CurrentUser, CacheAndHttp};
use std::sync::Arc;

pub fn init(vega: Rocket<Build>) -> Rocket<Build> {
    vega.mount("/", routes![user])
}

#[get("/bot/user")]
async fn user(discord: &State<Arc<CacheAndHttp>>) -> Json<CurrentUser> {
    Json(discord.cache.current_user().await)
}
