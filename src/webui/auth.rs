use crate::config::DiscordConfig;
use super::r#static::rocket_uri_macro_index;
use oauth2::{
    basic::BasicClient, reqwest::async_http_client, AuthUrl, AuthorizationCode, ClientId,
    ClientSecret, CsrfToken, Scope, TokenResponse, TokenUrl,
};
use serenity::{model::oauth2::OAuth2Scope, http::Http, model::id::UserId};
use rocket::{
    get,
    uri,
    http::{Cookie, Status, CookieJar},
    request::{Request, FromRequest, Outcome},
    response::Redirect,
    State,
};
use log::error;

pub struct SessionUser(Option<UserId>);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for &'r SessionUser {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        Outcome::Success(request.local_cache(|| {
            if let Some(cookie) = request.cookies().get_private("user_id") {
                if let Ok(id) = cookie.value().parse() {
                    return SessionUser(Some(UserId(id)));
                }
            }
            SessionUser(None)
        }))
    }
}

pub fn client(discord_config: &DiscordConfig) -> crate::Result<BasicClient> {
    Ok(BasicClient::new(
        ClientId::new(discord_config.client_id.to_string()),
        Some(ClientSecret::new(discord_config.client_secret.to_string())),
        AuthUrl::new("https://discord.com/api/oauth2/authorize".to_string())?,
        Some(TokenUrl::new(
            "https://discord.com/api/oauth2/token".to_string(),
        )?),
    ))
}

#[get("/auth/redirect")]
pub fn redirect(client: &State<BasicClient>, cookies: &CookieJar<'_>) -> Redirect {
    let (auth_url, csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new(OAuth2Scope::Identify.to_string()))
        .url();
    cookies.add_private(Cookie::build("csrf_token", csrf_token.secret().to_string()).http_only(true).permanent().finish());
    Redirect::to(auth_url.to_string())
}

#[get("/auth/callback?<state>&<code>")]
pub async fn callback(state: &str, code: &str, client: &State<BasicClient>, cookies: &CookieJar<'_>) -> Result<Redirect, (Status, &'static str)> {
    let csrf_token = cookies.get_private("csrf_token").ok_or((Status::BadRequest, "missing or invalid csrf_token cookie"))?;
    if csrf_token.value() != state {
        return Err((Status::BadRequest, "state parameter doesn't match csrf_token cookie"));
    }
    
    let token = client.exchange_code(AuthorizationCode::new(code.to_owned())).request_async(async_http_client).await.map_err(|err| {
        error!("exchange_code {}", err);
        (Status::BadGateway, "unable to exchange code for token")
    })?;

    let api = Http::new_with_token(token.access_token().secret());
    let user = api.get_current_user().await.map_err(|err| {
        error!("get_current_user {}", err);
        (Status::BadGateway, "unable to get user information")
    })?;

    cookies.add_private(Cookie::build("user_id", user.id.to_string()).http_only(true).permanent().finish());

    Ok(Redirect::to(uri!(index)))
}
