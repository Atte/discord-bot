#![allow(unused)] // features

use crate::SubstitutingString;
use color_eyre::eyre::Result;
use serde::Deserialize;
use serde_inline_default::serde_inline_default;
use serenity::{
    all::RuleId,
    model::id::{ChannelId, GuildId, RoleId, UserId},
};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub mongodb: MongodbConfig,
    pub discord: DiscordConfig,
    pub gib: GibConfig,
    #[cfg(feature = "cron")]
    pub cron: CronConfig,
    #[cfg(feature = "berrytube")]
    pub berrytube: BerrytubeConfig,
    #[cfg(feature = "teamup")]
    #[serde(default)]
    pub teamup: Vec<TeamupConfig>,
    #[cfg(feature = "openai")]
    pub openai: OpenAiConfig,
    #[cfg(feature = "starboard")]
    pub starboard: StarboardConfig,
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

#[derive(Debug, Clone, Deserialize)]
pub struct MongodbConfig {
    pub uri: SubstitutingString,
    pub database: SubstitutingString,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DiscordConfig {
    pub command_prefix: SubstitutingString,
    pub token: SubstitutingString,
    #[serde(default)]
    pub owners: HashSet<UserId>,
    pub blocked_users: HashSet<UserId>,
    #[serde(default)]
    pub command_channels: HashSet<ChannelId>,
    #[serde(default)]
    pub log_channels: HashSet<ChannelId>,
    #[serde(default)]
    pub rank_start_roles: HashSet<RoleId>,
    #[serde(default)]
    pub rank_end_roles: HashSet<RoleId>,
    #[serde(default)]
    pub enforce_automods: HashSet<GuildId>,
    #[serde(default)]
    pub volatiles: Vec<VolatileConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VolatileConfig {
    pub channel: ChannelId,
    pub role: RoleId,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GibConfig {
    pub endpoint: SubstitutingString,
    pub user_agent: SubstitutingString,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CronConfig {
    pub rate: u64,
    #[serde(default)]
    pub delete_old_messages: HashMap<ChannelId, i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BerrytubeConfig {
    pub url: SubstitutingString,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TeamupConfig {
    pub guild: GuildId,
    pub api_key: SubstitutingString,
    pub calendar_key: SubstitutingString,
    #[serde(default)]
    pub recurring_subcalendars: HashSet<u64>,
    #[serde(default)]
    pub oneoff_subcalendars: HashSet<u64>,
    pub location: Option<SubstitutingString>,
}

#[serde_inline_default]
#[derive(Debug, Clone, Deserialize)]
pub struct OpenAiConfig {
    #[serde(default)]
    pub api_url: Option<SubstitutingString>,
    pub api_key: SubstitutingString,
    #[serde_inline_default(1.0)]
    pub temperature: f32,
    #[serde_inline_default(1.0)]
    pub top_p: f32,
    pub model: SubstitutingString,
    pub prompt: SubstitutingString,
    #[serde_inline_default(true)]
    pub tools: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StarboardConfig {
    pub emoji: SubstitutingString,
    pub threshold: u64,
    #[serde(default)]
    pub max_threshold: Option<u64>,
    pub channels: HashSet<ChannelId>,
    #[serde(default)]
    pub ignore_stars: HashSet<RoleId>,
    #[serde(default)]
    pub ignore_messages: HashSet<RoleId>,
    #[serde(default)]
    pub ignore_channels: HashSet<RoleId>,
}

#[cfg(test)]
mod tests {
    #[ignore = "Depends on the environment"]
    #[tokio::test]
    async fn test_config() {
        super::Config::from_file("config.toml").await.unwrap();
    }
}
