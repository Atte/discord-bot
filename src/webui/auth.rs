use super::{r#static::rocket_uri_macro_index, util::HeaderResponder};
use crate::config::WebUIConfig;
use log::{error, trace};
use oauth2::{
    basic::BasicClient, reqwest::async_http_client, AuthUrl, AuthorizationCode, ClientId,
    ClientSecret, CsrfToken, Scope, TokenResponse, TokenUrl,
};
use rocket::{
    get, head,
    http::{Cookie, CookieJar, Header, SameSite, Status},
    request::{FromRequest, Outcome, Request},
    response::Redirect,
    routes, uri, Build, Rocket, State,
};
use serenity::{http::Http, model::oauth2::OAuth2Scope, model::user::CurrentUser};

pub struct SessionUser(pub CurrentUser);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for &'r SessionUser {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        request
            .local_cache(|| {
                if let Some(cookie) = request.cookies().get_private("user") {
                    if let Ok(user) = serde_json::from_str(cookie.value()) {
                        return Some(SessionUser(user));
                    }
                }
                None
            })
            .as_ref()
            .map_or(Outcome::Forward(()), |user| Outcome::Success(user))
    }
}

pub fn init(vega: Rocket<Build>, config: &WebUIConfig) -> crate::Result<Rocket<Build>> {
    let client = BasicClient::new(
        ClientId::new(config.discord_client_id.to_string()),
        Some(ClientSecret::new(config.discord_client_secret.to_string())),
        AuthUrl::new("https://discord.com/api/oauth2/authorize".to_string())?,
        Some(TokenUrl::new(
            "https://discord.com/api/oauth2/token".to_string(),
        )?),
    );
    Ok(vega
        .manage(client)
        .mount("/", routes![redirect, callback, callback_head, clear]))
}

#[get("/auth/redirect")]
fn redirect(client: &State<BasicClient>, cookies: &CookieJar<'_>) -> Redirect {
    let (auth_url, csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new(OAuth2Scope::Identify.to_string()))
        .url();
    cookies.add_private(
        Cookie::build("csrf_token", csrf_token.secret().to_string())
            .http_only(true)
            .same_site(SameSite::Lax)
            .permanent()
            .finish(),
    );
    Redirect::to(auth_url.to_string())
}

#[head("/auth/callback")]
fn callback_head() {}

#[get("/auth/callback?<state>&<code>")]
async fn callback(
    state: &str,
    code: &str,
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

    let token = client
        .exchange_code(AuthorizationCode::new(code.to_owned()))
        .request_async(async_http_client)
        .await
        .map_err(|err| {
            error!("exchange_code {:#?}", err);
            (Status::BadGateway, "unable to exchange code for token")
        })?;

    let api = Http::new_with_token(&format!("Bearer {}", token.access_token().secret()));
    let user = api.get_current_user().await.map_err(|err| {
        error!("get_current_user {:#?}", err);
        (Status::BadGateway, "unable to get current user information")
    })?;
    let user_string = serde_json::to_string(&user).map_err(|err| {
        error!("to_string(user) {:#?}", err);
        (
            Status::InternalServerError,
            "unable to stringify current user",
        )
    })?;

    trace!("{} ({}) logged in", user.id, user.tag());
    cookies.add_private(
        Cookie::build("user", user_string)
            .http_only(true)
            .permanent()
            .finish(),
    );

    Ok(Redirect::to(uri!(index)))
}

#[get("/auth/clear")]
fn clear() -> HeaderResponder<Redirect> {
    HeaderResponder::new(
        Redirect::to(uri!(redirect)),
        Header::new("Clear-Site-Data", "*"),
    )
}
