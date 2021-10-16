use rocket::request::{FromRequest, Outcome, Request};
use std::{
    convert::Infallible,
    fmt::{Display, Formatter},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RequestScheme {
    Http,
    Https,
}

impl RequestScheme {
    #[inline]
    pub fn is_secure(self) -> bool {
        self == Self::Https
    }
}

impl AsRef<str> for RequestScheme {
    #[inline]
    fn as_ref(&self) -> &str {
        match self {
            Self::Http => "http",
            Self::Https => "https",
        }
    }
}

impl Display for RequestScheme {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for RequestScheme {
    type Error = Infallible;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        Outcome::Success(*request.local_cache(|| {
            let mut tls = request.rocket().config().tls_enabled();

            // legacy header
            for proto in request
                .headers()
                .get("X-Forwarded-Proto")
                .flat_map(|forwarded| forwarded.split(','))
                .map(str::trim)
            {
                tls = proto == "https";
            }

            // modern header
            for forwarded in request
                .headers()
                .get("Forwarded")
                .flat_map(|forwarded| forwarded.split(','))
                .map(str::trim)
            {
                if forwarded
                    .split(';')
                    .any(|directive| directive.eq_ignore_ascii_case("proto=http"))
                {
                    tls = false;
                }
                if forwarded
                    .split(';')
                    .any(|directive| directive.eq_ignore_ascii_case("proto=https"))
                {
                    tls = true;
                }
            }

            if tls {
                RequestScheme::Https
            } else {
                RequestScheme::Http
            }
        }))
    }
}
