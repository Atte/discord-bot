use crate::SubstitutingString;
use color_eyre::eyre::Result;
use serde::Deserialize;
use serenity::model::id::{ChannelId, GuildId, RoleId, UserId};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};
use serde_with::{serde_as, DefaultOnNull};

#[serde_as]
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub mongodb: MongodbConfig,
    pub discord: DiscordConfig,
    pub gib: GibConfig,
    #[cfg(feature = "webui")]
    pub webui: WebUIConfig,
    #[cfg(feature = "cron")]
    pub cron: CronConfig,
    #[cfg(feature = "berrytube")]
    pub berrytube: BerrytubeConfig,
    #[cfg(feature = "teamup")]
    #[serde_as(as = "DefaultOnNull")]
    pub teamup: Vec<TeamupConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MongodbConfig {
    pub uri: SubstitutingString,
    pub database: SubstitutingString,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize)]
pub struct DiscordConfig {
    pub command_prefix: SubstitutingString,
    pub token: SubstitutingString,
    #[cfg(feature = "webui")]
    pub client_id: SubstitutingString,
    #[cfg(feature = "webui")]
    pub client_secret: SubstitutingString,
    #[serde_as(as = "DefaultOnNull")]
    pub owners: HashSet<UserId>,
    pub blocked_users: HashSet<UserId>,
    #[serde_as(as = "DefaultOnNull")]
    pub command_channels: HashSet<ChannelId>,
    #[serde_as(as = "DefaultOnNull")]
    pub log_channels: HashSet<ChannelId>,
    #[serde_as(as = "DefaultOnNull")]
    pub clean_channels: HashSet<ChannelId>,
    #[serde_as(as = "DefaultOnNull")]
    pub rules_channels: HashSet<ChannelId>,
    #[serde_as(as = "DefaultOnNull")]
    pub rules_roles: HashSet<RoleId>,
    pub rules_url: Option<SubstitutingString>,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize)]
pub struct GibConfig {
    pub endpoint: SubstitutingString,
    pub user_agent: SubstitutingString,
    #[serde_as(as = "DefaultOnNull")]
    pub shy_artists: HashSet<String>,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize)]
pub struct WebUIConfig {
    pub url: SubstitutingString,
    #[serde_as(as = "DefaultOnNull")]
    pub guilds: HashSet<GuildId>,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize)]
pub struct CronConfig {
    pub rate: u64,
    #[serde_as(as = "DefaultOnNull")]
    pub delete_old_messages: HashMap<ChannelId, i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BerrytubeConfig {
    pub url: SubstitutingString,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize)]
pub struct TeamupConfig {
    pub guild: GuildId,
    pub api_key: SubstitutingString,
    pub calendar_key: SubstitutingString,
    #[serde_as(as = "DefaultOnNull")]
    pub recurring_subcalendars: HashSet<u64>,
    #[serde_as(as = "DefaultOnNull")]
    pub oneoff_subcalendars: HashSet<u64>,
    pub location: SubstitutingString,
}

impl Config {
    #[inline]
    pub fn from_str(source: &str) -> Result<Config> {
        Ok(toml::from_str(source)?)
    }

    pub async fn from_file(path: impl AsRef<Path>) -> Result<Config> {
        let source = tokio::fs::read_to_string(path).await?;
        Self::from_str(&source)
    }
}

#[cfg(test)]
mod tests {
    #[ignore]
    #[tokio::test]
    async fn test_config() {
        super::Config::from_file("config.toml").await.unwrap();
    }
}
