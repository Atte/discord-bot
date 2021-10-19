use super::{
    super::{
        guilds::{guild_member, guild_roles},
        BotGuilds,
    },
    RateLimiter,
};
use rocket::{
    http::Status,
    outcome::{try_outcome, IntoOutcome},
    request::{FromRequest, Outcome, Request},
    State,
};
use serenity::{
    model::{guild::Member, id::GuildId, permissions::Permissions, user::CurrentUser},
    CacheAndHttp,
};
use std::{ops::Deref, sync::Arc};

pub type AuthError = (Status, &'static str);

#[derive(Clone, Copy)]
pub struct SessionUser<'r> {
    user: &'r CurrentUser,
    bot_guilds: &'r BotGuilds,
    discord: &'r CacheAndHttp,
}

impl<'r> SessionUser<'r> {
    #[inline]
    pub fn into_current_user(self) -> &'r CurrentUser {
        self.user
    }

    pub async fn member(&self, guild_id: GuildId) -> Result<Member, AuthError> {
        if !self.bot_guilds.contains_key(&guild_id) {
            return Err((Status::BadRequest, "invalid guild"));
        }

        Ok(guild_member(guild_id, self.user.id, self.discord)
            .await
            .ok_or((Status::BadGateway, "can't fetch member"))?)
    }

    pub async fn admin(&self, guild_id: GuildId) -> Result<Member, AuthError> {
        let member = self.member(guild_id).await?;
        if guild_roles(guild_id, self.discord)
            .await
            .ok_or((Status::BadGateway, "can't fetch roles"))?
            .into_values()
            .filter_map(|role| {
                if role.has_permission(Permissions::ADMINISTRATOR) {
                    Some(role.id)
                } else {
                    None
                }
            })
            .any(|role_id| member.roles.contains(&role_id))
        {
            Ok(member)
        } else {
            Err((Status::Forbidden, "not an admin"))
        }
    }
}

impl<'r> Deref for SessionUser<'r> {
    type Target = CurrentUser;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.user
    }
}

impl<'r> std::fmt::Debug for SessionUser<'r> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("SessionUser").field(self.user).finish()
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for SessionUser<'r> {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let user = try_outcome!(request
            .local_cache_async(async {
                let user = request
                    .cookies()
                    .get_private("user")
                    .and_then(|cookie| serde_json::from_str::<CurrentUser>(cookie.value()).ok());

                if let Some(ref user) = user {
                    request
                        .guard::<&State<RateLimiter<u64>>>()
                        .await
                        .expect("no RateLimiter in request state")
                        .apply_to_request(&user.id.0, request)
                        .await;
                }

                user
            })
            .await
            .as_ref()
            .or_forward(()));

        let bot_guilds = try_outcome!(request.guard::<&State<BotGuilds>>().await).inner();
        let discord = try_outcome!(request.guard::<&State<Arc<CacheAndHttp>>>().await)
            .inner()
            .as_ref();

        Outcome::Success(SessionUser {
            user,
            bot_guilds,
            discord,
        })
    }
}
