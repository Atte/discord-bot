use super::reqwest;
use reqwest::header;
use serenity::model::prelude::*;
use std::time::{Duration, Instant};
use std::{env, thread};

#[derive(Debug, Deserialize)]
struct AccessTokenResponse {
    access_token: String,
}

#[derive(Debug)]
enum RedditType {
    Modqueue,
    Modmail,
}

impl RedditType {
    fn title(&self) -> &'static str {
        match *self {
            RedditType::Modqueue => "New stuff in the modqueue",
            RedditType::Modmail => "New modmail",
        }
    }

    fn url<S>(&self, sub: S) -> String
    where
        S: AsRef<str>,
    {
        match *self {
            RedditType::Modqueue => {
                format!("https://old.reddit.com/r/{}/about/modqueue/", sub.as_ref())
            }
            RedditType::Modmail => "https://old.reddit.com/message/moderator/".to_owned(),
        }
    }
}

fn make_client<H>(auth: H) -> reqwest::Client
where
    H: header::Header,
{
    let mut headers = header::Headers::new();
    headers.set(header::UserAgent::new(concat!(
        "bot:fi.atte.",
        env!("CARGO_PKG_NAME"),
        ":v",
        env!("CARGO_PKG_VERSION"),
        " (by /u/AtteLynx)"
    )));
    headers.set(auth);

    reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .expect("Error making Reddit client")
}

fn make_login_client() -> reqwest::Client {
    let reddit_id: String = env::var("REDDIT_ID").expect("REDDIT_ID missing from env");
    let reddit_secret: String = env::var("REDDIT_SECRET").expect("REDDIT_SECRET missing from env");
    make_client(header::Authorization(header::Basic {
        username: reddit_id,
        password: Some(reddit_secret),
    }))
}

// TODO: cache results
fn make_user_client() -> Option<reqwest::Client> {
    let username: String = env::var("REDDIT_USERNAME").expect("REDDIT_USERNAME missing from env");
    let password: String = env::var("REDDIT_PASSWORD").expect("REDDIT_PASSWORD missing from env");
    if let Ok(resp) = make_login_client()
        .get("https://www.reddit.com/api/v1/access_token")
        .query(&[
            ("grant_type", "password".to_owned()),
            ("username", username),
            ("password", password),
        ])
        .send()
        .and_then(|resp| resp.json::<AccessTokenResponse>())
    {
        Some(make_client(header::Authorization(header::Bearer {
            token: resp.access_token,
        })))
    } else {
        error!("Error getting Reddit access_token");
        None
    }
}

fn check<S>(sub: S) -> Option<RedditType>
where
    S: AsRef<str>,
{
    None
}

pub fn spawn() -> thread::JoinHandle<()> {
    let check_interval = Duration::from_secs(5);

    let notify_channel: u64 = env::var("NOTIFY_CHANNEL")
        .expect("NOTIFY_CHANNEL missing from env")
        .parse()
        .expect("NOTIFY_CHANNEL is not a number");

    let sub: String = env::var("REDDIT_SUB").expect("REDDIT_SUB missing from env");

    thread::Builder::new()
        .name("reddit".to_owned())
        .spawn(move || {
            let mut start = Instant::now();
            loop {
                thread::sleep(Duration::from_secs(1));
                if start.elapsed() < check_interval {
                    continue;
                }
                start = Instant::now();

                if let Some(reddit_type) = check(&sub) {
                    if let Err(err) = ChannelId(notify_channel).send_message(|msg| {
                        msg.embed(|e| e.title(reddit_type.title()).url(reddit_type.url(&sub)))
                    }) {
                        error!("Error sending Reddit notification: {}", err);
                    }
                }
            }
        })
        .expect("Error spawning Reddit thread")
}
