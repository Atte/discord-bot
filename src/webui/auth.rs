use super::{
    r#static::rocket_uri_macro_index,
    server_timing::Metrics,
    util::{HeaderResponder, SecureRequest},
    RateLimiter,
};
use crate::config::Config;
use governor::Jitter;
use log::{error, trace};
use oauth2::{
    basic::BasicClient, reqwest::async_http_client, AuthUrl, AuthorizationCode, ClientId,
    ClientSecret, CsrfToken, Scope, TokenResponse, TokenUrl,
};
use rocket::{
    get, head,
    http::{Cookie, CookieJar, Header, SameSite, Status},
    post,
    request::{FromRequest, Outcome, Request},
    response::Redirect,
    routes, uri, Build, Rocket, State,
};
use serenity::{http::Http, model::oauth2::OAuth2Scope, model::user::CurrentUser};
use std::{
    ops::{Deref, DerefMut},
    time::Duration,
};

pub struct SessionUser(CurrentUser);

impl Deref for SessionUser {
    type Target = CurrentUser;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SessionUser {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for &'r SessionUser {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let mut first_time = false;
        match request
            .local_cache(|| {
                request
                    .cookies()
                    .get_private("user")
                    .and_then(|cookie| serde_json::from_str::<CurrentUser>(cookie.value()).ok())
                    .map(|user| {
                        first_time = true;
                        SessionUser(user)
                    })
            })
            .as_ref()
        {
            Some(user) => {
                if first_time {
                    let mut metrics = request
                        .guard::<&Metrics>()
                        .await
                        .expect("no Metrics in request state")
                        .write()
                        .await;
                    let metric = metrics.entry("ratelimit".to_string()).or_default();
                    metric.start();
                    request
                        .guard::<&State<RateLimiter>>()
                        .await
                        .expect("no RateLimiter in request state")
                        .until_key_ready_with_jitter(
                            &user.id.0,
                            Jitter::up_to(Duration::from_millis(100)),
                        )
                        .await;
                    metric.stop();
                }
                Outcome::Success(user)
            }
            None => Outcome::Forward(()),
        }
    }
}

pub fn init(vega: Rocket<Build>, config: &Config) -> crate::Result<Rocket<Build>> {
    let client = BasicClient::new(
        ClientId::new(config.discord.client_id.to_string()),
        Some(ClientSecret::new(config.discord.client_secret.to_string())),
        AuthUrl::new("https://discord.com/api/oauth2/authorize".to_string())?,
        Some(TokenUrl::new(
            "https://discord.com/api/oauth2/token".to_string(),
        )?),
    );
    Ok(vega.manage(client).mount(
        "/api/auth",
        routes![redirect, callback, callback_head, clear],
    ))
}

#[post("/redirect")]
fn redirect(
    secure: Option<&SecureRequest>,
    client: &State<BasicClient>,
    cookies: &CookieJar<'_>,
) -> Redirect {
    let (auth_url, csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new(OAuth2Scope::Identify.to_string()))
        .url();
    cookies.add_private(
        Cookie::build("csrf_token", csrf_token.secret().to_string())
            .same_site(SameSite::Lax)
            .secure(secure.is_some())
            .finish(),
    );
    Redirect::to(auth_url.to_string())
}

#[head("/callback")]
const fn callback_head() {}

#[get("/callback?<state>&<code>")]
async fn callback(
    state: &str,
    code: &str,
    secure: Option<&SecureRequest>,
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

    trace!("{} ({}) logged in", user.tag(), user.id);
    cookies.remove_private(csrf_token);
    cookies.add_private(
        Cookie::build("user", user_string)
            .same_site(SameSite::Strict)
            .secure(secure.is_some())
            .finish(),
    );

    Ok(Redirect::to(uri!(index)))
}

#[post("/clear")]
fn clear(cookies: &CookieJar<'_>) -> HeaderResponder<Redirect> {
    cookies.remove_private(Cookie::named("user"));
    HeaderResponder::new(
        Header::new("Clear-Site-Data", "*"),
        Redirect::to(uri!(index)),
    )
}
