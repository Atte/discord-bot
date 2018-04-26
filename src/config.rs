use serde::{de, ser};
use serenity::model::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::result::Result as StdResult;
use std::{cmp, env, fmt, hash};
use toml;

error_chain! {
    foreign_links {
        Io(::std::io::Error);
        Toml(::toml::de::Error);
    }
}

#[derive(Debug)]
pub struct SubstitutingString(String);

impl fmt::Display for SubstitutingString {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.0.starts_with('$') {
            if let Ok(val) = env::var(&self.0[1..]) {
                f.write_str(&val)
            } else {
                error!(
                    "Configuration contains non-existent environment variable {}",
                    self.0
                );
                Err(fmt::Error)
            }
        } else {
            f.write_str(&self.0)
        }
    }
}

impl<S> cmp::PartialEq<S> for SubstitutingString
where
    S: ToString,
{
    #[inline]
    fn eq(&self, other: &S) -> bool {
        self.to_string().eq(&other.to_string())
    }
}

impl Eq for SubstitutingString {}

impl cmp::PartialEq<SubstitutingString> for String {
    #[inline]
    fn eq(&self, other: &SubstitutingString) -> bool {
        self.eq(&other.to_string())
    }
}

impl hash::Hash for SubstitutingString {
    #[inline]
    fn hash<H>(&self, mut state: &mut H)
    where
        H: hash::Hasher,
    {
        self.to_string().hash(&mut state)
    }
}

impl ser::Serialize for SubstitutingString {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> StdResult<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

struct SubstitutingStringVisitor;

impl<'de> de::Visitor<'de> for SubstitutingStringVisitor {
    type Value = SubstitutingString;

    #[inline]
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string optionally containing $ENV variables")
    }

    #[inline]
    fn visit_string<E>(self, s: String) -> StdResult<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(SubstitutingString(s))
    }

    #[inline]
    fn visit_str<E>(self, s: &str) -> StdResult<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(SubstitutingString(s.to_owned()))
    }
}

impl<'de> de::Deserialize<'de> for SubstitutingString {
    #[inline]
    fn deserialize<D>(deserializer: D) -> StdResult<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_str(SubstitutingStringVisitor)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub cache_path: SubstitutingString,
    pub discord: DiscordConfig,
    pub reddit: RedditConfig,
    pub subreddits: HashMap<SubstitutingString, SubredditConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiscordConfig {
    pub token: SubstitutingString,
    pub owners: HashSet<UserId>,
    pub log_channels: HashSet<ChannelId>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RedditConfig {
    pub client_id: SubstitutingString,
    pub client_secret: SubstitutingString,
    pub username: SubstitutingString,
    pub password: SubstitutingString,
    pub check_interval: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubredditConfig {
    pub notify_channels: HashSet<ChannelId>,
}

impl Config {
    pub fn from_file<P>(path: P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let mut source: Vec<u8> = Vec::new();
        {
            let mut fh = File::open(path)?;
            fh.read_to_end(&mut source)?;
        }
        Ok(toml::from_slice(&source)?)
    }
}
