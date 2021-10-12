use rocket::request::{FromRequest, Outcome, Request};
use std::convert::Infallible;

#[derive(Debug)]
pub struct SecureRequest;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for &'r SecureRequest {
    type Error = Infallible;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        if request.rocket().config().tls_enabled()
            || request
                .headers()
                .get_one("X-Forwarded-Proto")
                .map_or(false, |val| val == "https")
        {
            Outcome::Success(&SecureRequest)
        } else {
            Outcome::Forward(())
        }
    }
}
