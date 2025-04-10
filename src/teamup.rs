use crate::{config::TeamupConfig, substituting_string::SubstitutingString};
use chrono::{DateTime, Duration, Utc};
use color_eyre::{
    Section, SectionExt,
    eyre::{Result, eyre},
};
use log::info;
use reqwest::{Method, header::HeaderValue};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use serde_with::{DefaultOnError, NoneAsEmptyString, serde_as};
use serenity::all::{Cache, Http};
use std::time::Duration as StdDuration;
use std::{collections::HashMap, sync::Arc};
use tokio::time::sleep;
use tokio::try_join;

const RATE_LIMIT: StdDuration = StdDuration::from_secs(15);

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum TeamupId {
    String(#[allow(unused)] String),
    Number(#[allow(unused)] u64),
}

#[derive(Debug, Clone, Deserialize)]
struct TeamupEventsResponse {
    // timestamp: u64,
    events: Vec<TeamupEvent>,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize)]
pub struct TeamupEvent {
    // id: TeamupId,
    series_id: Option<TeamupId>,
    pub start_dt: DateTime<Utc>,
    pub end_dt: DateTime<Utc>,
    pub title: String,
    #[serde_as(as = "NoneAsEmptyString")]
    pub notes: Option<String>,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DiscordEvent {
    #[serde(skip_serializing)]
    id: String,
    #[serde(skip_serializing)]
    creator_id: String,
    entity_metadata: Option<DiscordEventEntityMetadata>,
    name: String,
    #[serde_as(as = "DefaultOnError")]
    privacy_level: Option<DiscordEventPrivacyLevel>,
    scheduled_start_time: DateTime<Utc>,
    scheduled_end_time: Option<DateTime<Utc>>,
    #[serde_as(as = "NoneAsEmptyString")]
    description: Option<String>,
    #[serde_as(as = "DefaultOnError")]
    entity_type: Option<DiscordEventEntityType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DiscordEventEntityMetadata {
    location: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
enum DiscordEventPrivacyLevel {
    GuildOnly = 2,
}

#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
enum DiscordEventEntityType {
    StageInstance = 1,
    Voice = 2,
    External = 3,
}

pub struct Teamup {
    config: TeamupConfig,
    cache: Arc<Cache>,
    http: Arc<Http>,
    client: reqwest::Client,
}

impl Teamup {
    #[inline]
    pub fn new(config: TeamupConfig, cache: Arc<Cache>, http: Arc<Http>) -> Self {
        Self {
            config,
            cache,
            http,
            client: reqwest::Client::new(),
        }
    }

    async fn fetch_calendar_events<T: Iterator<Item = u64>>(
        &self,
        range: Duration,
        subcalendars: T,
    ) -> Result<impl Iterator<Item = TeamupEvent> + use<T>> {
        let now = Utc::now();
        let response = self
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
            .error_for_status()?
            .text()
            .await?;

        let response: TeamupEventsResponse = serde_json::from_str(&response)
            .map_err(|err| eyre!(err).with_section(|| response.header("Response:")))?;

        Ok(response.events.into_iter().filter(move |event| {
            event.end_dt > now
                && (event.series_id.is_none() || event.start_dt < (now + Duration::days(6)))
        }))
    }

    async fn discord_request(
        &self,
        method: Method,
        event_id: Option<&str>,
        f: impl FnOnce(reqwest::RequestBuilder) -> reqwest::RequestBuilder,
    ) -> Result<String> {
        let url = if let Some(event_id) = event_id {
            format!(
                "https://discord.com/api/v9/guilds/{}/scheduled-events/{}",
                self.config.guild, event_id
            )
        } else {
            format!(
                "https://discord.com/api/v9/guilds/{}/scheduled-events",
                self.config.guild
            )
        };

        let request = self
            .client
            .request(method, url)
            .header("Authorization", HeaderValue::from_str(self.http.token())?);

        let response = f(request).send().await?;

        if let Err(err) = response.error_for_status_ref() {
            let text = response.text().await?;
            Err(eyre!(err).with_section(|| text.header("Response:")))
        } else {
            Ok(response.text().await?)
        }
    }

    async fn fetch_discord_events(&self) -> Result<impl Iterator<Item = DiscordEvent> + use<>> {
        let response = self.discord_request(Method::GET, None, |r| r).await?;

        let response: Vec<DiscordEvent> = serde_json::from_str(&response)
            .map_err(|err| eyre!(err).with_section(|| response.header("Response:")))?;

        let bot_id = self.cache.current_user().id.to_string();
        Ok(response
            .into_iter()
            .filter(move |event| event.creator_id == bot_id))
    }

    async fn create_discord_event(&self, event: &DiscordEvent) -> Result<()> {
        self.discord_request(Method::POST, None, |r| r.json(event))
            .await?;
        Ok(())
    }

    async fn delete_discord_event(&self, event_id: &str) -> Result<()> {
        self.discord_request(Method::DELETE, Some(event_id), |r| r)
            .await?;
        Ok(())
    }

    pub async fn fetch_recurring_events(&self) -> Result<impl Iterator<Item = TeamupEvent> + '_> {
        self.fetch_calendar_events(
            Duration::days(9),
            self.config.recurring_subcalendars.iter().copied(),
        )
        .await
    }

    pub async fn fetch_oneoff_events(&self) -> Result<impl Iterator<Item = TeamupEvent> + '_> {
        self.fetch_calendar_events(
            Duration::days(365),
            self.config.oneoff_subcalendars.iter().copied(),
        )
        .await
    }

    pub async fn run(&mut self) -> Result<()> {
        let (discord_events, recurring_calendar_events, oneoff_calendar_events) = try_join!(
            self.fetch_discord_events(),
            self.fetch_recurring_events(),
            self.fetch_oneoff_events(),
        )?;

        let mut discord_events: HashMap<String, DiscordEvent> = discord_events
            .map(|event| (event.id.clone(), event))
            .collect();

        for calendar_event in recurring_calendar_events.chain(oneoff_calendar_events) {
            if let Some(existing_event) = discord_events
                .values()
                .find(|event| {
                    event.name == calendar_event.title
                        && event.scheduled_start_time == calendar_event.start_dt
                })
                .cloned()
            {
                discord_events.remove(&existing_event.id);
            } else if calendar_event.start_dt > Utc::now() + Duration::minutes(1) {
                sleep(RATE_LIMIT).await;
                info!("Creating event: {}", calendar_event.title);
                self.create_discord_event(&DiscordEvent {
                    id: "serialization skipped".to_owned(),
                    creator_id: "serialization skipped".to_owned(),
                    entity_metadata: Some(DiscordEventEntityMetadata {
                        location: self
                            .config
                            .location
                            .as_ref()
                            .map(SubstitutingString::to_string),
                    }),
                    name: calendar_event.title,
                    privacy_level: Some(DiscordEventPrivacyLevel::GuildOnly),
                    scheduled_start_time: calendar_event.start_dt,
                    scheduled_end_time: Some(calendar_event.end_dt),
                    description: calendar_event.notes,
                    entity_type: Some(DiscordEventEntityType::External),
                })
                .await?;
            }
        }

        for discord_event in discord_events.into_values() {
            sleep(RATE_LIMIT).await;
            info!("Deleting event: {}", discord_event.name);
            self.delete_discord_event(&discord_event.id).await?;
        }

        Ok(())
    }
}
