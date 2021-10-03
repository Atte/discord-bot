use rocket::{
    http::Header,
    request::{FromRequest, Outcome, Request},
    response::Responder,
};

#[derive(Responder)]
pub struct HeaderResponder<T> {
    inner: T,
    header: Header<'static>,
}

impl<T> HeaderResponder<T> {
    #[inline]
    pub const fn new(header: Header<'static>, inner: T) -> Self {
        Self { inner, header }
    }
}

pub struct SecureRequest;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for &'r SecureRequest {
    type Error = ();

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
