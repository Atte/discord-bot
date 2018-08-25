use super::serialization::string_or_struct;
use super::{CACHE, CONFIG};
use reqwest::{self, header};
use serenity::builder::CreateEmbed;
use serenity::utils::Colour;
use std::collections::HashSet;
use std::str::FromStr;
use std::time::{Duration, Instant};
use std::{io, thread};
use void::Void;

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
    #[serde(default, deserialize_with = "string_or_struct")]
    replies: RedditObject<RedditListing<RedditMessageish>>,
}

#[derive(Debug, PartialEq, Eq, Hash)]
enum NotificationClass {
    Modqueue,
    Modmail,
    ModmailReply,
}

impl NotificationClass {
    fn title(&self) -> &'static str {
        match *self {
            NotificationClass::Modqueue => "New stuff in the modqueue",
            NotificationClass::Modmail => "New modmail",
            NotificationClass::ModmailReply => "New reply to modmail",
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
            NotificationClass::Modmail | NotificationClass::ModmailReply => format!(
                "https://old.reddit.com/r/{}/about/message/inbox/",
                sub.as_ref()
            ),
        }
    }

    fn colour(&self) -> Colour {
        match *self {
            NotificationClass::Modqueue => Colour::BLUE,
            NotificationClass::Modmail | NotificationClass::ModmailReply => Colour::RED,
        }
    }
}

impl<T> Default for RedditObject<RedditListing<T>> {
    fn default() -> Self {
        RedditObject {
            kind: "Listing".to_owned(),
            data: RedditListing {
                after: None,
                before: None,
                children: Vec::new(),
            },
        }
    }
}

impl<T> FromStr for RedditObject<RedditListing<T>> {
    type Err = Void;

    fn from_str(_s: &str) -> ::std::result::Result<Self, Self::Err> {
        Ok(Self::default())
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
        }).send()?
        .error_for_status()?;

    let data: AccessTokenResponse = resp.json()?;
    let auth = header::Authorization(header::Bearer {
        token: data.access_token,
    });
    Ok(make_client(auth)?)
}

fn contains_unseen(data: &RedditListing<RedditMessageish>) -> Result<bool> {
    Ok(CACHE.with(|cache| {
        let has_unseen = data
            .children
            .iter()
            .any(|obj| !cache.reddit_seen.contains(&obj.data.id));
        cache
            .reddit_seen
            .extend(data.children.iter().map(|obj| obj.data.id.clone()));
        has_unseen
    })?)
}

fn check_sub(client: &reqwest::Client, sub: &str) -> Result<HashSet<NotificationClass>> {
    debug!("Checking /r/{}", sub);

    let mut out = HashSet::new();
    {
        let data: RedditObject<RedditListing<RedditMessageish>> = client
            .get(&format!(
                "https://oauth.reddit.com/r/{}/about/modqueue",
                sub
            )).send()?
            .error_for_status()?
            .json()?;
        if contains_unseen(&data.data)? {
            out.insert(NotificationClass::Modqueue);
        }
    }
    {
        let data: RedditObject<RedditListing<RedditMessageish>> = client
            .get(&format!(
                "https://oauth.reddit.com/r/{}/about/message/inbox",
                sub
            )).send()?
            .error_for_status()?
            .json()?;
        if contains_unseen(&data.data)? {
            out.insert(NotificationClass::Modmail);
        }
        for msg in data.data.children.into_iter() {
            if contains_unseen(&msg.data.replies.data)? {
                out.insert(NotificationClass::ModmailReply);
            }
        }
    }
    Ok(out)
}

fn apply_embed(
    e: CreateEmbed,
    reddit_type: &NotificationClass,
    sub: &str,
    new: bool,
) -> CreateEmbed {
    let e = e
        .colour(reddit_type.colour())
        .title(reddit_type.title())
        .url(reddit_type.url(sub))
        .author(|a| a.name(&format!("/r/{}", sub)));
    if new {
        e
    } else {
        e.description("(has been resolved)")
    }
}

fn main() -> Result<()> {
    trace!("Time for a Reddit check!");

    let client = make_user_client()?;
    for (sub, sub_config) in &CONFIG.subreddits {
        let sub = sub.as_ref();
        let reddit_types = check_sub(&client, sub)?;
        for reddit_type in &reddit_types {
            for channel_id in &sub_config.notify_channels {
                channel_id
                    .send_message(|msg| msg.embed(|e| apply_embed(e, reddit_type, sub, true)))?;
            }
        }
        /*
        if !reddit_types.contains(&NotificationClass::Modqueue) {
            for channel_id in &sub_config.notify_channels {
                if let Some(mut msg) = channel_id
                    .messages(|req| req.limit(10))?
                    .into_iter()
                    .filter(|msg| msg.author.id == util::uid())
                    .last()
                {
                    msg.edit(|msg| {
                        msg.embed(|e| apply_embed(e, &NotificationClass::Modqueue, sub, false))
                    })?;
                }
            }
        }
        */
    }
    Ok(())
}

pub fn spawn() -> io::Result<thread::JoinHandle<()>> {
    if !CONFIG.reddit.enabled {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Reddit functionality is disabled in config",
        ));
    }

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
