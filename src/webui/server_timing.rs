use itertools::Itertools;
use rocket::{
    data::Data,
    fairing::{Fairing, Info, Kind},
    http::Header,
    request::{FromRequest, Outcome, Request},
    response::Response,
};
use serenity::prelude::RwLock;
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    time::{Duration, Instant},
};

#[derive(Default)]
pub struct Metric {
    desc: Option<String>,
    duration: Option<Duration>,
    start: Option<Instant>,
}

impl Metric {
    #[inline]
    pub fn start(&mut self) {
        self.stop();
        self.start = Some(Instant::now());
    }

    pub fn stop(&mut self) {
        if let Some(start) = self.start.take() {
            self.duration = Some(self.duration.unwrap_or_default() + start.elapsed());
        }
    }

    pub fn duration(&self) -> Option<Duration> {
        self.start.map_or(self.duration, |start| {
            Some(self.duration.unwrap_or_default() + start.elapsed())
        })
    }

    #[inline]
    pub fn desc(&self) -> Option<&String> {
        self.desc.as_ref()
    }

    #[inline]
    pub fn set_desc(&mut self, desc: String) {
        self.desc = Some(desc);
    }
}

pub struct Metrics(RwLock<HashMap<String, Metric>>);

impl Deref for Metrics {
    type Target = RwLock<HashMap<String, Metric>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Metrics {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for &'r Metrics {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        Outcome::Success(request.local_cache(|| Metrics(RwLock::new(HashMap::new()))))
    }
}

pub struct ServerTiming;

#[rocket::async_trait]
impl Fairing for ServerTiming {
    fn info(&self) -> Info {
        Info {
            name: "Server-Timing",
            kind: Kind::Request | Kind::Response,
        }
    }

    async fn on_request(&self, request: &mut Request<'_>, _data: &mut Data<'_>) {
        if let Outcome::Success(metrics) = request.guard::<&Metrics>().await {
            metrics
                .write()
                .await
                .entry("total".to_string())
                .or_default()
                .start();
        }
    }

    async fn on_response<'r>(&self, request: &'r Request<'_>, response: &mut Response<'r>) {
        if let Outcome::Success(metrics) = request.guard::<&Metrics>().await {
            let header = metrics
                .read()
                .await
                .iter()
                .map(|(key, metric)| {
                    let mut value = key.clone();
                    if let Some(ref desc) = metric.desc {
                        value.push_str(&format!(
                            ";desc=\"{}\"",
                            desc.replace('\\', "\\\\").replace('"', "\\\"")
                        ));
                    }
                    if let Some(duration) = metric.duration() {
                        value.push_str(&format!(";dur={}", duration.as_millis()));
                    }
                    value
                })
                .join(", ");
            if !header.is_empty() {
                response.adjoin_header(Header::new("Server-Timing", header));
            }
        }
    }
}
