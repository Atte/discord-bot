use super::CONFIG;
use reqwest::{self, header};
use std::time::{Duration, Instant};
use std::{io, thread};

#[derive(Debug, Serialize, Deserialize)]
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
    make_client(header::Authorization(header::Basic {
        username: CONFIG.reddit.client_id.to_string(),
        password: Some(CONFIG.reddit.client_secret.to_string()),
    }))
}

// TODO: cache results
fn make_user_client() -> Option<reqwest::Client> {
    if let Ok(resp) = make_login_client()
        .get("https://www.reddit.com/api/v1/access_token")
        .query(&[
            ("grant_type", "password".to_owned()),
            ("username", CONFIG.reddit.username.to_string()),
            ("password", CONFIG.reddit.password.to_string()),
        ])
        .send()
        .and_then(|mut resp| resp.json::<AccessTokenResponse>())
    {
        Some(make_client(header::Authorization(header::Bearer {
            token: resp.access_token,
        })))
    } else {
        error!("Error getting Reddit access_token");
        None
    }
}

fn check_sub<S>(client: &reqwest::Client, sub: S) -> Option<RedditType>
where
    S: AsRef<str>,
{
    None
}

fn main() {
    if let Some(client) = make_user_client() {
        for (sub, sub_config) in &CONFIG.subreddits {
            let sub = sub.to_string();
            if let Some(reddit_type) = check_sub(&client, &sub) {
                for channel_id in &sub_config.notify_channels {
                    if let Err(err) = channel_id.send_message(|msg| {
                        msg.embed(|e| e.title(reddit_type.title()).url(reddit_type.url(&sub)))
                    }) {
                        error!("Error sending Reddit notification: {}", err);
                    }
                }
            }
        }
    } else {
        error!("Error creating Reddit client");
    }
}

pub fn spawn() -> io::Result<thread::JoinHandle<()>> {
    let check_interval = Duration::from_secs(60 * CONFIG.reddit.check_interval);
    if check_interval.as_secs() < 60 {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Reddit check interval is less than a minute; refusing",
        ));
    }

    thread::Builder::new()
        .name("reddit".to_owned())
        .spawn(move || {
            let mut start = Instant::now();
            loop {
                thread::sleep(Duration::from_secs(1));
                if start.elapsed() >= check_interval {
                    start = Instant::now();
                    main();
                }
            }
        })
}
