use super::{CACHE, CONFIG};
use reqwest::{self, header};
use serde_json;
use std::time::{Duration, Instant};
use std::{io, thread};

error_chain! {
    links {
        Cache(super::cache::Error, super::cache::ErrorKind);
    }

    foreign_links {
        Io(::std::io::Error);
        Discord(::serenity::Error);
        Reddit(::reqwest::Error);
        Json(::serde_json::Error);
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct AccessTokenResponse {
    access_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct RedditObject<T> {
    kind: String,
    data: T,
}

#[derive(Debug, Serialize, Deserialize)]
struct RedditListing<T> {
    after: Option<String>,
    before: Option<String>,
    children: Vec<RedditObject<T>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct RedditMessageish {
    id: String,
}

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
    headers.set(header::Accept::json());
    headers.set(auth);

    Ok(reqwest::Client::builder()
        .referer(false)
        .default_headers(headers)
        .build()?)
}

fn make_login_client() -> Result<reqwest::Client> {
    trace!("Making login client...");

    make_client(header::Authorization(header::Basic {
        username: CONFIG.reddit.client_id.to_string(),
        password: Some(CONFIG.reddit.client_secret.to_string()),
    }))
}

// TODO: cache results
fn make_user_client() -> Result<reqwest::Client> {
    trace!("Making user client...");

    let mut resp = make_login_client()?
        .post("https://www.reddit.com/api/v1/access_token")
        .form(&hashmap!{
            "grant_type" => "password".to_owned(),
            "username" => CONFIG.reddit.username.to_string(),
            "password" => CONFIG.reddit.password.to_string(),
        })
        .send()?
        .error_for_status()?;

    let data: AccessTokenResponse = resp.json()?;
    let auth = header::Authorization(header::Bearer {
        token: data.access_token,
    });
    Ok(make_client(auth)?)
}

fn check_sub<S>(client: &reqwest::Client, sub: S) -> Result<Option<NotificationClass>>
where
    S: AsRef<str>,
{
    trace!("Checking /r/{}", sub.as_ref());

    {
        let mut resp = client
            .get(&format!(
                "https://oauth.reddit.com/r/{}/about/modqueue",
                sub.as_ref()
            ))
            .send()?
            .error_for_status()?;
        let data: RedditObject<RedditListing<RedditMessageish>> = resp.json()?;
        trace!("data: {}", serde_json::to_string(&data)?);

        let all_seen = CACHE.with(|cache| {
            let all_seen = data.data
                .children
                .iter()
                .all(|obj| cache.seen.contains(&obj.data.id));
            cache
                .seen
                .extend(data.data.children.into_iter().map(|obj| obj.data.id));
            all_seen
        })?;
        if !all_seen {
            return Ok(Some(NotificationClass::Modqueue));
        }
    }

    // TODO: check modmail

    Ok(None)
}

fn main() -> Result<()> {
    trace!("Time for a Reddit check!");

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
    trace!("Spawning Reddit thread...");

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
