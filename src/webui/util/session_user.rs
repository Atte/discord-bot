use super::RateLimiter;
use derive_more::Deref;
use rocket::{
    outcome::{try_outcome, IntoOutcome},
    request::{FromRequest, Outcome, Request},
    State,
};
use serenity::model::user::CurrentUser;

#[derive(Clone, Copy, Debug, Deref)]
pub struct SessionUser<'r>(&'r CurrentUser);

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

        Outcome::Success(SessionUser(user))
    }
}
