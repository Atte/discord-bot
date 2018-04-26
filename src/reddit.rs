use super::CONFIG;
use reqwest::{self, header};
use std::time::{Duration, Instant};
use std::{io, thread};

error_chain! {
    foreign_links {
        Io(::std::io::Error);
        Discord(::serenity::Error);
        Reddit(::reqwest::Error);
    }
}

static API_BASE: &str = "https://oauth.reddit.com/api/v1";

#[derive(Debug, Serialize, Deserialize)]
struct AccessTokenResponse {
    access_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ModqueueResponse {}

#[derive(Debug)]
enum NotificationClass {
    Modqueue,
    Modmail,
}

impl NotificationClass {
    fn title(&self) -> &'static str {
        match *self {
            NotificationClass::Modqueue => "New stuff in the modqueue",
            NotificationClass::Modmail => "New modmail",
        }
    }

    fn url<S>(&self, sub: S) -> String
    where
        S: AsRef<str>,
    {
        match *self {
            NotificationClass::Modqueue => {
                format!("https://old.reddit.com/r/{}/about/modqueue/", sub.as_ref())
            }
            NotificationClass::Modmail => "https://old.reddit.com/message/moderator/".to_owned(),
        }
    }
}

fn make_client<H>(auth: H) -> Result<reqwest::Client>
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

    Ok(reqwest::Client::builder().default_headers(headers).build()?)
}

fn make_login_client() -> Result<reqwest::Client> {
    make_client(header::Authorization(header::Basic {
        username: CONFIG.reddit.client_id.to_string(),
        password: Some(CONFIG.reddit.client_secret.to_string()),
    }))
}

// TODO: cache results
fn make_user_client() -> Result<reqwest::Client> {
    let data: AccessTokenResponse = make_login_client()?
        .get("https://www.reddit.com/api/v1/access_token")
        .query(&[
            ("grant_type", "password".to_owned()),
            ("username", CONFIG.reddit.username.to_string()),
            ("password", CONFIG.reddit.password.to_string()),
        ])
        .send()?
        .json()?;
    let auth = header::Authorization(header::Bearer {
        token: data.access_token,
    });
    Ok(make_client(auth)?)
}

fn check_sub<S>(client: &reqwest::Client, sub: S) -> Result<Option<NotificationClass>>
where
    S: AsRef<str>,
{
    let data: ModqueueResponse = client
        .get(&format!("{}/r/{}/about/modqueue", API_BASE, sub.as_ref()))
        .send()?
        .json()?;
    Ok(None)
}

fn main() -> Result<()> {
    let client = make_user_client()?;
    for (sub, sub_config) in &CONFIG.subreddits {
        if let Some(reddit_type) = check_sub(&client, sub)? {
            for channel_id in &sub_config.notify_channels {
                channel_id.send_message(|msg| {
                    msg.embed(|e| e.title(reddit_type.title()).url(reddit_type.url(sub)))
                })?;
            }
        }
    }
    Ok(())
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
                    if let Err(err) = main() {
                        error!("{}", err);
                    }
                }
            }
        })
}
