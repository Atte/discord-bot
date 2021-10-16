use super::RequestScheme;
use rocket::{
    http::{uri::Absolute, Status},
    outcome::IntoOutcome,
    request::{FromRequest, Outcome, Request},
};
use std::ops::Deref;

#[derive(Debug)]
pub struct RequestOrigin(Absolute<'static>);

impl Deref for RequestOrigin {
    type Target = Absolute<'static>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for &'r RequestOrigin {
    type Error = &'static str;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        request
            .local_cache_async(async {
                let mut uri = Absolute::parse_owned(format!(
                    "{}://{}",
                    request.guard::<RequestScheme>().await.succeeded()?,
                    request.headers().get_one("Host")?
                ))
                .ok()?;
                uri.normalize();
                Some(RequestOrigin(uri))
            })
            .await
            .as_ref()
            .into_outcome((Status::BadRequest, "can't determine request origin"))
    }
}
