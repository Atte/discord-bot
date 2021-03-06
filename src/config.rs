use crate::{Result, SubstitutingString};
use serde::Deserialize;
use serenity::model::id::{ChannelId, RoleId, UserId};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub mongodb: MongodbConfig,
    pub discord: DiscordConfig,
    pub cron: CronConfig,
    pub berrytube: BerrytubeConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MongodbConfig {
    pub uri: SubstitutingString,
    pub database: SubstitutingString,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DiscordConfig {
    pub command_prefix: SubstitutingString,
    pub token: SubstitutingString,
    pub owners: HashSet<UserId>,
    pub blocked_users: HashSet<UserId>,
    pub command_channels: HashSet<ChannelId>,
    pub log_channels: HashSet<ChannelId>,
    pub sticky_roles: HashSet<RoleId>,
    pub gib: GibConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GibConfig {
    pub endpoint: SubstitutingString,
    pub user_agent: SubstitutingString,
    pub shy_artists: HashSet<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CronConfig {
    pub rate: u64,
    pub delete_old_messages: HashMap<ChannelId, i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BerrytubeConfig {
    pub enabled: bool,
    pub url: SubstitutingString,
}

impl Config {
    #[inline]
    pub fn from_str(source: &str) -> Result<Config> {
        Ok(toml::from_str(source)?)
    }

    pub async fn from_file(path: impl AsRef<Path>) -> Result<Config> {
        let source = tokio::fs::read_to_string(path).await?;
        Ok(Self::from_str(&source)?)
    }
}
