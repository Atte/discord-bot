use itertools::Itertools;
use maplit::hashmap;
use rocket::{
    data::Data,
    fairing::{Fairing, Info, Kind},
    http::Header,
    request::{FromRequest, Outcome, Request},
    response::Response,
};
use std::{collections::HashMap, sync::Mutex, time::Duration};

#[cfg(test)]
use mock_instant::Instant;
#[cfg(not(test))]
use std::time::Instant;

#[derive(Debug)]
struct Timer {
    duration: Duration,
    started: Option<Instant>,
    desc: Option<String>,
}

impl Timer {
    #[inline]
    fn new_running(desc: Option<String>) -> Self {
        Self {
            duration: Duration::ZERO,
            started: Some(Instant::now()),
            desc,
        }
    }

    #[inline]
    const fn new_stopped() -> Self {
        Self {
            duration: Duration::ZERO,
            started: None,
            desc: None,
        }
    }

    #[inline]
    fn start(&mut self) {
        self.started.get_or_insert_with(Instant::now);
    }

    #[inline]
    fn stop(&mut self) {
        if let Some(started) = self.started.take() {
            self.duration += Instant::now() - started;
        }
    }

    #[inline]
    fn duration(&self) -> Duration {
        self.started.map_or(self.duration, |started| {
            self.duration + (Instant::now() - started)
        })
    }
}

#[derive(Debug)]
pub struct ServerTimings(Mutex<HashMap<String, Timer>>);

impl ServerTimings {
    fn with_req() -> Self {
        Self(Mutex::new(
            hashmap! {"req".to_owned() => Timer::new_running(None)},
        ))
    }

    #[inline]
    fn empty() -> Self {
        Self(Mutex::new(HashMap::new()))
    }

    pub fn start(&self, name: impl Into<String>) {
        if let Ok(mut timers) = self.0.lock() {
            timers
                .entry(name.into())
                .and_modify(|timer| {
                    timer.start();
                })
                .or_insert_with(|| Timer::new_running(None));
        }
    }

    pub fn start_with_desc(&self, name: impl Into<String>, desc: impl Into<String>) {
        if let Ok(mut timers) = self.0.lock() {
            timers
                .entry(name.into())
                .and_modify(|timer| {
                    timer.start();
                })
                .or_insert_with(|| Timer::new_running(Some(desc.into())));
        }
    }

    pub fn stop(&self, name: impl Into<String>) {
        if let Ok(mut timers) = self.0.lock() {
            timers
                .entry(name.into())
                .and_modify(|timer| {
                    timer.stop();
                })
                .or_insert_with(Timer::new_stopped);
        }
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for &'r ServerTimings {
    type Error = ();

    #[inline]
    async fn from_request(request: &'r Request<'_>) -> Outcome<&'r ServerTimings, Self::Error> {
        Outcome::Success(request.local_cache(ServerTimings::empty))
    }
}

#[derive(Debug)]
pub struct ServerTimingFairing;

#[rocket::async_trait]
impl Fairing for ServerTimingFairing {
    #[inline]
    fn info(&self) -> Info {
        Info {
            name: "Server-Timing",
            kind: Kind::Request | Kind::Response | Kind::Singleton,
        }
    }

    #[inline]
    async fn on_request(&self, request: &mut Request<'_>, _data: &mut Data<'_>) {
        request.local_cache(ServerTimings::with_req);
    }

    async fn on_response<'r>(&self, request: &'r Request<'_>, response: &mut Response<'r>) {
        if let Ok(timers) = request.local_cache(ServerTimings::empty).0.lock() {
            let header = timers
                .iter()
                .map(|(name, timer)| {
                    let millis = timer.duration().as_millis();
                    if let Some(ref desc) = timer.desc {
                        format!(
                            "{};desc=\"{}\";dur={}",
                            name,
                            desc.replace('"', "\""),
                            millis
                        )
                    } else {
                        format!("{};dur={}", name, millis)
                    }
                })
                .join(", ");
            if !header.is_empty() {
                response.adjoin_header(Header::new("Server-timing", header));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ServerTimingFairing, ServerTimings};
    use itertools::Itertools;
    use mock_instant::MockClock;
    use rocket::{get, local::blocking::Client, routes};
    use std::time::Duration;

    #[get("/")]
    fn index(timings: &ServerTimings) -> &'static str {
        MockClock::advance(Duration::from_secs(1));
        timings.start("internal");
        timings.start_with_desc("leaking", "not stopped in handler");
        MockClock::advance(Duration::from_millis(100));
        timings.stop("internal");
        MockClock::advance(Duration::from_millis(10));
        ""
    }

    #[test]
    fn server_timings() {
        let timings = ServerTimings::with_req(); // implicitly starts the "req" timer
        MockClock::advance(Duration::from_secs(1));
        timings.start("req"); // noop
        MockClock::advance(Duration::from_secs(1));
        timings.stop("req"); // stops the timer
        MockClock::advance(Duration::from_secs(10));
        timings.stop("req"); // noop

        timings.start("req"); // restarts the timer
        MockClock::advance(Duration::from_secs(100));
        timings.stop("req"); // stops the timer

        let duration = timings.0.lock().unwrap().get("req").unwrap().duration();
        assert_eq!(duration, Duration::from_secs(102));
    }

    #[test]
    fn server_timing_fairing() {
        let client = Client::untracked(
            rocket::build()
                .attach(ServerTimingFairing)
                .mount("/", routes![index]),
        )
        .unwrap();

        let response = client.get("/").dispatch();
        let parts: Vec<_> = response
            .headers()
            .get_one("Server-Timing")
            .unwrap()
            .split(", ")
            .sorted()
            .collect();
        assert_eq!(
            parts,
            vec![
                "internal;dur=100",
                "leaking;desc=\"not stopped in handler\";dur=110",
                "req;dur=1110",
            ]
        );
    }
}
