use super::substituting_string::SubstitutingString;
use serenity::model::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use toml;

error_chain! {
    foreign_links {
        Io(::std::io::Error);
        Toml(::toml::de::Error);
    }
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub cache_path: SubstitutingString,
    pub discord: DiscordConfig,
    pub reddit: RedditConfig,
    pub subreddits: HashMap<SubstitutingString, SubredditConfig>,
    pub bulk: BulkConfig,
    pub gib: GibConfig,
}

#[derive(Debug, Deserialize)]
pub struct DiscordConfig {
    pub command_prefix: SubstitutingString,
    pub deleted_msg_cache: usize,
    pub long_msg_threshold: usize,
    pub username: SubstitutingString,
    pub token: SubstitutingString,
    pub owners: HashSet<UserId>,
    pub log_channels: HashSet<ChannelId>,
    pub channel_blacklist: HashSet<ChannelId>,
    pub channel_whitelist: HashSet<ChannelId>,
}

#[derive(Debug, Deserialize)]
pub struct RedditConfig {
    pub enabled: bool,
    pub client_id: SubstitutingString,
    pub client_secret: SubstitutingString,
    pub username: SubstitutingString,
    pub password: SubstitutingString,
    pub check_interval: u64,
}

#[derive(Debug, Deserialize)]
pub struct SubredditConfig {
    pub notify_channels: HashSet<ChannelId>,
}

#[derive(Debug, Deserialize)]
pub struct BulkConfig {
    pub insults: Vec<SubstitutingString>,
}

#[derive(Debug, Deserialize)]
pub struct GibConfig {
    pub filter: usize,
    pub not_found: Vec<String>,
    pub aliases: HashMap<String, HashSet<String>>,
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
