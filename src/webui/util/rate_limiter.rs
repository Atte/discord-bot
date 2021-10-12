use super::ServerTimings;
use governor::Jitter;
use rocket::{outcome::Outcome, request::Request};
use std::{hash::Hash, time::Duration};

type Governor<T> = governor::RateLimiter<
    T,
    governor::state::keyed::DefaultKeyedStateStore<T>,
    governor::clock::DefaultClock,
>;

#[derive(Debug)]
pub struct RateLimiter<T>(Governor<T>)
where
    T: Clone + Eq + Hash;

impl<T> RateLimiter<T>
where
    T: Clone + Eq + Hash,
{
    #[inline]
    pub fn new(quota: governor::Quota) -> Self {
        Self(Governor::<T>::keyed(quota))
    }

    pub async fn apply_to_request(&self, key: &T, request: &Request<'_>) {
        let timings = request.guard::<&ServerTimings>().await;
        if let Outcome::Success(timings) = timings {
            timings.start("ratelimit");
        }
        self.apply(key).await;
        if let Outcome::Success(timings) = timings {
            timings.stop("ratelimit");
        }
    }

    pub async fn apply(&self, key: &T) {
        self.0
            .until_key_ready_with_jitter(key, Jitter::up_to(Duration::from_millis(100)))
            .await;
    }
}
