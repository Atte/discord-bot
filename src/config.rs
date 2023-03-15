use crate::SubstitutingString;
use color_eyre::eyre::Result;
use serde::Deserialize;
use serenity::model::id::{ChannelId, GuildId, RoleId, UserId};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

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
    #[serde(default)]
    pub teamup: Vec<TeamupConfig>,
    #[cfg(feature = "openai")]
    pub openai: OpenAiConfig,
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
    #[cfg(feature = "webui")]
    pub client_id: SubstitutingString,
    #[cfg(feature = "webui")]
    pub client_secret: SubstitutingString,
    #[serde(default)]
    pub owners: HashSet<UserId>,
    pub blocked_users: HashSet<UserId>,
    #[serde(default)]
    pub command_channels: HashSet<ChannelId>,
    #[serde(default)]
    pub log_channels: HashSet<ChannelId>,
    #[serde(default)]
    pub clean_channels: HashSet<ChannelId>,
    #[serde(default)]
    pub rules_channels: HashSet<ChannelId>,
    #[serde(default)]
    pub rules_roles: HashSet<RoleId>,
    pub rules_url: Option<SubstitutingString>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GibConfig {
    pub endpoint: SubstitutingString,
    pub user_agent: SubstitutingString,
    #[serde(default)]
    pub shy_artists: HashSet<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WebUIConfig {
    pub url: SubstitutingString,
    #[serde(default)]
    pub guilds: HashSet<GuildId>,
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
    pub location: SubstitutingString,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpenAiConfig {
    pub api_key: SubstitutingString,
    pub prompt: SubstitutingString,
    pub temperature: Option<f32>,
    #[serde(default)]
    pub bot_replacements: HashMap<String, String>,
    #[serde(default)]
    pub user_replacements: HashMap<String, String>,
}

#[cfg(test)]
mod tests {
    #[ignore]
    #[tokio::test]
    async fn test_config() {
        super::Config::from_file("config.toml").await.unwrap();
    }
}
