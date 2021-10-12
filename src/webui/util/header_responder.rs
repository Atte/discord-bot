use rocket::{
    http::{Header, HeaderMap},
    request::Request,
    response::Responder,
};

#[derive(Debug)]
pub struct HeaderResponder<T> {
    inner: T,
    headers: HeaderMap<'static>,
}

impl<T> HeaderResponder<T> {
    #[inline]
    pub fn from(inner: T) -> Self {
        Self {
            inner,
            headers: HeaderMap::new(),
        }
    }

    #[inline]
    pub fn set_header<H>(mut self, header: H) -> Self
    where
        H: Into<Header<'static>>,
    {
        self.headers.replace(header);
        self
    }
}

impl<'r, T> Responder<'r, 'static> for HeaderResponder<T>
where
    T: Responder<'r, 'static>,
{
    fn respond_to(self, request: &'r Request<'_>) -> rocket::response::Result<'static> {
        let mut response = self.inner.respond_to(request)?;
        for header in self.headers.into_iter() {
            response.set_header(header);
        }
        Ok(response)
    }
}
