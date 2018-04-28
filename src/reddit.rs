use super::{CACHE, CONFIG};
use reqwest::{self, header};
use serenity::utils::Colour;
use std::collections::HashSet;
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

#[derive(Debug, PartialEq, Eq, Hash)]
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
            NotificationClass::Modmail => format!(
                "https://old.reddit.com/r/{}/about/message/inbox/",
                sub.as_ref()
            ),
        }
    }

    fn colour(&self) -> Colour {
        match *self {
            NotificationClass::Modqueue => Colour::blue(),
            NotificationClass::Modmail => Colour::red(),
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

fn contains_unseen(data: RedditObject<RedditListing<RedditMessageish>>) -> Result<bool> {
    Ok(CACHE.with(|cache| {
        let has_unseen = data.data
            .children
            .iter()
            .any(|obj| !cache.seen.contains(&obj.data.id));
        cache
            .seen
            .extend(data.data.children.into_iter().map(|obj| obj.data.id));
        has_unseen
    })?)
}

fn check_sub(client: &reqwest::Client, sub: &str) -> Result<HashSet<NotificationClass>> {
    trace!("Checking /r/{}", sub);

    let mut out = HashSet::new();
    {
        let data: RedditObject<RedditListing<RedditMessageish>> = client
            .get(&format!(
                "https://oauth.reddit.com/r/{}/about/modqueue",
                sub
            ))
            .send()?
            .error_for_status()?
            .json()?;
        if contains_unseen(data)? {
            out.insert(NotificationClass::Modqueue);
        }
    }
    {
        let data: RedditObject<RedditListing<RedditMessageish>> = client
            .get(&format!(
                "https://oauth.reddit.com/r/{}/about/message/inbox",
                sub
            ))
            .send()?
            .error_for_status()?
            .json()?;
        if contains_unseen(data)? {
            out.insert(NotificationClass::Modmail);
        }
    }
    Ok(out)
}

fn main() -> Result<()> {
    trace!("Time for a Reddit check!");

    let client = make_user_client()?;
    for (sub, sub_config) in &CONFIG.subreddits {
        let sub = sub.as_ref();
        for reddit_type in check_sub(&client, sub)? {
            for channel_id in &sub_config.notify_channels {
                channel_id.send_message(|msg| {
                    msg.embed(|e| {
                        e.colour(reddit_type.colour())
                            .title(reddit_type.title())
                            .url(reddit_type.url(sub))
                            .author(|a| a.name(&format!("/r/{}", sub)))
                    })
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
            if cfg!(feature = "reddit-debug") {
                if let Err(err) = main() {
                    error!("{}", err);
                }
                return;
            }

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
