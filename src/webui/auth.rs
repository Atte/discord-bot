use super::{r#static::rocket_uri_macro_index, util::HeaderResponder};
use crate::config::Config;
use log::{error, trace};
use oauth2::{
    basic::BasicClient, reqwest::async_http_client, AuthUrl, AuthorizationCode, ClientId,
    ClientSecret, CsrfToken, RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use rocket::{
    get,
    http::{Cookie, CookieJar, Header, SameSite, Status},
    post,
    response::Redirect,
    routes, uri, Build, Rocket, State,
};
use serenity::{http::Http, model::oauth2::OAuth2Scope};
use std::borrow::Cow;

pub fn init(vega: Rocket<Build>, config: &Config) -> color_eyre::eyre::Result<Rocket<Build>> {
    let client = BasicClient::new(
        ClientId::new(config.discord.client_id.to_string()),
        Some(ClientSecret::new(config.discord.client_secret.to_string())),
        AuthUrl::new("https://discord.com/api/oauth2/authorize".to_string())?,
        Some(TokenUrl::new(
            "https://discord.com/api/oauth2/token".to_string(),
        )?),
    );
    Ok(vega
        .manage(client)
        .mount("/", routes![redirect, callback, clear]))
}

#[post("/api/auth/redirect")]
fn redirect(
    config: &State<Config>,
    client: &State<BasicClient>,
    cookies: &CookieJar<'_>,
) -> Result<Redirect, (Status, &'static str)> {
    let origin = config.webui.url.to_string();
    let (auth_url, csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new(OAuth2Scope::Identify.to_string()))
        .set_redirect_uri(Cow::Owned(
            RedirectUrl::new(format!("{}/api/auth/callback", origin)).map_err(|err| {
                error!("authorize_url {:?}", err);
                (Status::BadGateway, "unable to form redirect URL")
            })?,
        ))
        .url();
    cookies.add_private(
        Cookie::build("csrf_token", csrf_token.secret().to_string())
            .same_site(SameSite::Lax)
            .secure(origin.starts_with("https://"))
            .finish(),
    );
    Ok(Redirect::to(auth_url.to_string()))
}

#[get("/api/auth/callback?<state>&<code>")]
async fn callback(
    state: &str,
    code: &str,
    config: &State<Config>,
    client: &State<BasicClient>,
    cookies: &CookieJar<'_>,
) -> Result<Redirect, (Status, &'static str)> {
    let csrf_token = cookies
        .get_private("csrf_token")
        .ok_or((Status::BadRequest, "missing or invalid csrf_token cookie"))?;
    if csrf_token.value() != state {
        return Err((
            Status::BadRequest,
            "state parameter doesn't match csrf_token cookie",
        ));
    }
    let origin = config.webui.url.to_string();
    let token = client
        .exchange_code(AuthorizationCode::new(code.to_owned()))
        .set_redirect_uri(Cow::Owned(
            RedirectUrl::new(format!("{}/api/auth/callback", origin)).map_err(|err| {
                error!("exchange_code {:?}", err);
                (Status::BadGateway, "unable to form redirect URL")
            })?,
        ))
        .request_async(async_http_client)
        .await
        .map_err(|err| {
            error!("exchange_code {:?}", err);
            (Status::BadGateway, "unable to exchange code for token")
        })?;

    let api = Http::new(&format!("Bearer {}", token.access_token().secret()));
    let user = api.get_current_user().await.map_err(|err| {
        error!("get_current_user {:?}", err);
        (Status::BadGateway, "unable to get current user information")
    })?;
    let user_string = serde_json::to_string(&user).map_err(|err| {
        error!("to_string(user) {:?}", err);
        (
            Status::InternalServerError,
            "unable to stringify current user",
        )
    })?;

    trace!("{} ({}) logged in", user.tag(), user.id);
    cookies.remove_private(csrf_token);
    cookies.add_private(
        Cookie::build("user", user_string)
            .same_site(SameSite::Strict)
            .secure(origin.starts_with("https://"))
            .finish(),
    );

    Ok(Redirect::to(uri!(index)))
}

#[post("/api/auth/clear")]
fn clear(cookies: &CookieJar<'_>) -> HeaderResponder<Redirect> {
    cookies.remove_private(Cookie::named("user"));
    HeaderResponder::from(Redirect::to(uri!(index))).set_header(Header::new("Clear-Site-Data", "*"))
}
