use crate::config::TeamupConfig;
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use log::info;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Url,
};
use serde::Deserialize;
use serenity::{
    model::id::{GuildId, MessageId},
    CacheAndHttp,
};
use std::{collections::HashSet, sync::Arc};

#[derive(Debug, Clone, Deserialize)]
struct EventsResponse {
    timestamp: u64,
    events: Vec<EventsResponseEvent>,
}

#[derive(Debug, Clone, Deserialize)]
struct EventsResponseEvent {
    id: String,
    series_id: Option<u64>,
    start_dt: DateTime<Utc>,
    end_dt: DateTime<Utc>,
    title: String,
    notes: String,
}

pub struct Teamup {
    guild_id: GuildId,
    discord: Arc<CacheAndHttp>,
    client: reqwest::Client,
    url: Url,
    recurring_subcalendars: HashSet<u64>,
    oneoff_subcalendars: HashSet<u64>,
    location: String,
}

impl Teamup {
    #[inline]
    pub fn try_new(
        guild_id: GuildId,
        config: TeamupConfig,
        discord: Arc<CacheAndHttp>,
    ) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Teamup-Token",
            HeaderValue::from_str(config.api_key.as_ref())?,
        );
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        let mut url = Url::parse("https://api.teamup.com/")?;
        url.path_segments_mut()
            .unwrap()
            .push(config.calendar_key.as_ref())
            .push("events");

        Ok(Self {
            guild_id,
            discord,
            client,
            url,
            recurring_subcalendars: config.recurring_subcalendars,
            oneoff_subcalendars: config.oneoff_subcalendars,
            location: config.location.to_string(),
        })
    }

    async fn fetch_calendar_events(
        &self,
        range: Duration,
        subcalendars: impl Iterator<Item = u64>,
    ) -> Result<Vec<EventsResponseEvent>> {
        let now = Utc::now();
        let response: EventsResponse = self
            .client
            .get(self.url.clone())
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
            .send()
            .await?
            .json()
            .await?;
        Ok(response.events)
    }

    async fn fetch_discord_events(&self) -> Result<()> {
        Ok(())
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut calendar_events = self
            .fetch_calendar_events(
                Duration::days(9),
                self.recurring_subcalendars.iter().copied(),
            )
            .await?;
        calendar_events.extend(
            self.fetch_calendar_events(
                Duration::days(365),
                self.oneoff_subcalendars.iter().copied(),
            )
            .await?,
        );

        let now = Utc::now();
        for calendar_event in calendar_events {
            if calendar_event.end_dt < now
                || (calendar_event.series_id.is_some()
                    && (calendar_event.start_dt - now) > Duration::days(7))
            {
                continue;
            }
        }

        Ok(())
    }
}
