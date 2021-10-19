use super::util::{Json, SessionUser};
use rocket::{get, routes, Build, Rocket};
use serenity::model::user::CurrentUser;

pub fn init(vega: Rocket<Build>) -> Rocket<Build> {
    vega.mount("/api/me", routes![user])
}

#[get("/")]
fn user(user: &SessionUser) -> Json<&CurrentUser> {
    Json(&*user)
}
