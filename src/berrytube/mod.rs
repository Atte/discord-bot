use crate::{
    config::BerrytubeConfig, discord::ActivityKey, discord::MAX_ACTIVITY_LENGTH,
    util::ellipsis_string, Result,
};
use futures::StreamExt;
use reqwest::Url;
use serde::Deserialize;
use serenity::{
    client::bridge::gateway::ShardManager,
    model::gateway::Activity,
    prelude::{Mutex, RwLock, TypeMap},
};
use std::sync::Arc;

mod sse;
use sse::{stream_sse_events, SseEvent};

#[derive(Debug, Clone, Deserialize)]
struct VideoChangeEvent {
    title: String,
}

pub struct Berrytube {
    url: Url,
    shard_manager: Arc<Mutex<ShardManager>>,
    data: Arc<RwLock<TypeMap>>,
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
        })
    }

    pub async fn run(&self) -> Result<()> {
        let mut stream = stream_sse_events(self.url.clone()).await?;
        loop {
            match stream.next().await {
                Some(Ok(SseEvent {
                    event: Some(event),
                    data: Some(ref data),
                    ..
                })) if event == "videoChange" => {
                    if let Ok(video_change) = serde_json::from_str::<VideoChangeEvent>(data) {
                        self.set_title(video_change.title).await
                    }
                }
                Some(Ok(_)) => {} // ignore other events
                Some(Err(err)) => return Err(err),
                None => return Ok(()),
            }
        }
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
            runner
                .runner_tx
                .set_activity(Some(Activity::playing(&ellipsis_string(
                    title,
                    MAX_ACTIVITY_LENGTH,
                ))));
        }
    }
}
