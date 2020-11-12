use crate::{
    config::BerrytubeConfig,
    discord::limits::ACTIVITY_LENGTH,
    discord::ActivityKey,
    util::{ellipsis_string, format_duration_short},
    Result,
};
use futures::StreamExt;
use log::{debug, warn};
use reqwest::Url;
use serde::Deserialize;
use serenity::{
    client::bridge::gateway::ShardManager,
    model::gateway::Activity,
    prelude::{Mutex, RwLock, TypeMap},
};
use std::{convert::TryInto, sync::Arc, time::Duration};

mod sse;
use sse::{stream_sse_events, SseEvent};

#[derive(Debug, Clone, Deserialize)]
struct VideoChangeEvent {
    title: String,
    length: i64,
}

#[derive(Debug, Clone, Copy, Deserialize)]
struct VideoStatusEvent {
    time: i64,
}

pub struct Berrytube {
    url: Url,
    shard_manager: Arc<Mutex<ShardManager>>,
    data: Arc<RwLock<TypeMap>>,
    latest_change: Option<VideoChangeEvent>,
    latest_status: Option<VideoStatusEvent>,
}

impl Berrytube {
    pub fn try_new(
        config: &BerrytubeConfig,
        shard_manager: Arc<Mutex<ShardManager>>,
        data: Arc<RwLock<TypeMap>>,
    ) -> Result<Self> {
        Ok(Self {
            url: Url::parse(config.url.as_ref())?,
            shard_manager,
            data,
            latest_change: None,
            latest_status: None,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut stream = stream_sse_events(self.url.clone()).await?;
        loop {
            match stream.next().await {
                Some(Ok(SseEvent {
                    event: Some(event),
                    data: Some(data),
                    ..
                })) => {
                    debug!("BT event: {:?}", event);
                    if event == "videoChange" {
                        if let Ok(video_change) = serde_json::from_str::<VideoChangeEvent>(&data) {
                            // videoChange might come after videoStatus when receiving the backlog
                            if self.latest_change.is_some() {
                                self.latest_status = None;
                            }
                            self.latest_change = Some(video_change);
                            if let Err(err) = self.update_title().await {
                                warn!("Error updating BT title after videoChange: {}", err);
                            }
                        }
                    } else if event == "videoStatus" {
                        if let Ok(video_status) = serde_json::from_str::<VideoStatusEvent>(&data) {
                            self.latest_status = Some(video_status);
                            if let Err(err) = self.update_title().await {
                                warn!("Error updating BT title after videoStatus: {}", err);
                            }
                        }
                    }
                }
                Some(Ok(_)) => {} // ignore events with incomplete content
                Some(Err(err)) => return Err(err),
                None => return Ok(()),
            }
        }
    }

    async fn update_title(&self) -> Result<()> {
        match (self.latest_change.as_ref(), self.latest_status) {
            (Some(change), status) => {
                let time_string = format!(
                    " ({}/{})",
                    match status {
                        Some(VideoStatusEvent { time }) if time > 0 =>
                            format_duration_short(&Duration::from_secs(time.try_into()?)),
                        _ => "00:00".to_owned(),
                    },
                    if change.length > 0 {
                        format_duration_short(&Duration::from_secs(change.length.try_into()?))
                    } else {
                        "live".to_owned()
                    }
                );
                self.set_title(format!(
                    "{}{}",
                    ellipsis_string(&change.title, ACTIVITY_LENGTH - time_string.len()),
                    time_string
                ))
                .await;
            }
            _ => {
                self.set_title("").await;
            }
        }
        Ok(())
    }

    async fn set_title(&self, title: impl AsRef<str>) {
        let title = title.as_ref();
        {
            let mut data = self.data.write().await;
            if data
                .get::<ActivityKey>()
                .map_or(false, |current| current == title)
            {
                return;
            }
            data.insert::<ActivityKey>(title.to_owned());
        }

        let shard_manager = self.shard_manager.lock().await;
        for runner in shard_manager.runners.lock().await.values() {
            runner.runner_tx.set_activity(if title.is_empty() {
                None
            } else {
                Some(Activity::playing(&title))
            });
        }
    }
}
