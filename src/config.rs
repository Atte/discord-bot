use crate::{Result, SubstitutingString};
use serde::Deserialize;
use serenity::model::id::{ChannelId, RoleId, UserId};
use std::{collections::HashSet, path::Path};

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub mongodb: MongodbConfig,
    pub discord: DiscordConfig,
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
    pub gib_endpoint: SubstitutingString,
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
