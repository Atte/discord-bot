use rocket::{http::Header, response::Responder};

#[derive(Responder)]
pub struct HeaderResponder<T> {
    inner: T,
    header: Header<'static>,
}

impl<T> HeaderResponder<T> {
    pub fn new(inner: T, header: Header<'static>) -> Self {
        Self { inner, header }
    }
}
