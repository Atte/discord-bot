use crate::config::TeamupConfig;
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use log::{info, trace};
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Method, Url,
};
use serde::{Deserialize, Serialize};
use serenity::{
    model::id::{GuildId, MessageId},
    CacheAndHttp,
};
use std::{collections::HashSet, io::Read, sync::Arc};
use tokio::try_join;

#[derive(Debug, Clone, Deserialize)]
struct TeamupEventsResponse {
    timestamp: u64,
    events: Vec<TeamupEvent>,
}

#[derive(Debug, Clone, Deserialize)]
struct TeamupEvent {
    id: String,
    series_id: Option<u64>,
    start_dt: DateTime<Utc>,
    end_dt: DateTime<Utc>,
    title: String,
    notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DiscordEvent {
    #[serde(skip_serializing)]
    id: String,
    #[serde(skip_serializing)]
    creator_id: String,
    entity_metadata: Option<DiscordEventEntityMetadata>,
    name: String,
    #[serde(deserialize_with = "serde_with::rust::default_on_error::deserialize")]
    privacy_level: Option<DiscordEventPrivacyLevel>,
    scheduled_start_time: DateTime<Utc>,
    scheduled_end_time: Option<DateTime<Utc>>,
    description: Option<String>,
    #[serde(deserialize_with = "serde_with::rust::default_on_error::deserialize")]
    entity_type: Option<DiscordEventEntityType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DiscordEventEntityMetadata {
    location: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
enum DiscordEventPrivacyLevel {
    GuildOnly = 2,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
enum DiscordEventEntityType {
    StageInstance = 1,
    Voice = 2,
    External = 3,
}

pub struct Teamup {
    guild_id: GuildId,
    config: TeamupConfig,
    discord: Arc<CacheAndHttp>,
    client: reqwest::Client,
}

impl Teamup {
    #[inline]
    pub fn new(guild_id: GuildId, config: TeamupConfig, discord: Arc<CacheAndHttp>) -> Self {
        Self {
            guild_id,
            discord,
            config,
            client: reqwest::Client::new(),
        }
    }

    async fn fetch_calendar_events(
        &self,
        range: Duration,
        subcalendars: impl Iterator<Item = u64>,
    ) -> Result<impl Iterator<Item = TeamupEvent>> {
        let now = Utc::now();
        let response: TeamupEventsResponse = self
            .client
            .get(format!(
                "https://api.teamup.com/{}/events",
                self.config.calendar_key
            ))
            .query(&[
                ("startDate", now.format("%Y-%m-%d").to_string()),
                ("endDate", (now + range).format("%Y-%m-%d").to_string()),
            ])
            .query(
                &subcalendars
                    .into_iter()
                    .map(|sub| ("subcalendarId[]", sub.to_string()))
                    .collect::<Vec<_>>(),
            )
            .header(
                "Teamup-Token",
                HeaderValue::from_str(self.config.api_key.as_ref())?,
            )
            .send()
            .await?
            .json()
            .await?;
        Ok(response.events.into_iter().filter(move |event| {
            event.end_dt < now
                || (event.series_id.is_some() && (event.start_dt - now) > Duration::days(7))
        }))
    }

    fn discord_request(
        &self,
        method: Method,
        event_id: Option<&str>,
    ) -> Result<reqwest::RequestBuilder> {
        let url = if let Some(event_id) = event_id {
            format!(
                "https://discord.com/api/v9/guilds/{}/scheduled-events/{}",
                self.guild_id, event_id
            )
        } else {
            format!(
                "https://discord.com/api/v9/guilds/{}/scheduled-events",
                self.guild_id
            )
        };
        let request = self.client.request(method, url).header(
            "Authorization",
            HeaderValue::from_str(&format!("Bot {}", self.discord.http.token))?,
        );
        Ok(request)
    }

    async fn fetch_discord_events(&self) -> Result<impl Iterator<Item = DiscordEvent>> {
        let response: Vec<DiscordEvent> = self
            .discord_request(Method::GET, None)?
            .send()
            .await?
            .json()
            .await?;

        let bot_id = self.discord.cache.current_user_id().await.to_string();
        Ok(response
            .into_iter()
            // TODO
            .filter(move |event| true || event.creator_id == bot_id))
    }

    async fn create_discord_event(&self, event: &DiscordEvent) -> Result<()> {
        self.discord_request(Method::POST, None)?
            .json(event)
            .send()
            .await?;
        Ok(())
    }

    async fn modify_discord_event(&self, event: &DiscordEvent) -> Result<()> {
        self.discord_request(Method::PATCH, Some(&event.id))?
            .json(event)
            .send()
            .await?;
        Ok(())
    }

    async fn delete_discord_event(&self, event_id: &str) -> Result<()> {
        self.discord_request(Method::DELETE, Some(&event_id))?
            .send()
            .await?;
        Ok(())
    }

    pub async fn run(&mut self) -> Result<()> {
        let (discord_events, recurring_calendar_events, oneoff_calendar_events) = try_join!(
            self.fetch_discord_events(),
            self.fetch_calendar_events(
                Duration::days(9),
                self.config.recurring_subcalendars.iter().copied(),
            ),
            self.fetch_calendar_events(
                Duration::days(365),
                self.config.oneoff_subcalendars.iter().copied(),
            ),
        )?;

        for discord_event in discord_events {
            trace!("{:#?}", discord_event);
        }

        for calendar_event in recurring_calendar_events.chain(oneoff_calendar_events) {
            trace!("{:#?}", calendar_event);
        }

        Ok(())
    }
}
