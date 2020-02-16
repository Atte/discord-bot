use crate::CONFIG;
use error_chain::error_chain;
use log::trace;
use serde::Deserialize;
use serenity::{
    client::bridge::gateway::{ShardManager, ShardMessenger},
    model::gateway::Activity,
    prelude::*,
};
use sse_client::EventSource;
use std::{sync::Arc, time::Duration};

error_chain! {
    foreign_links {
        Io(::std::io::Error);
        Discord(::serenity::Error);
        Json(::serde_json::Error);
    }

    errors {
        DisabledInConfig {
            description("BerryTube functionality is disabled in config")
        }

        InvalidUrl {
            description("invalid BerryTube URL")
        }
    }
}

const READ_TIMEOUT: Duration = Duration::from_secs(5);
const WRITE_TIMEOUT: Duration = Duration::from_secs(5);

pub struct NowPlayingKey;

impl TypeMapKey for NowPlayingKey {
    type Value = String;
}

#[derive(Debug, Deserialize)]
struct Video {
    id: String,
    length: isize,
    title: String,
    #[serde(rename = "type")]
    videotype: String,
    volat: bool,
}

pub fn spawn(
    client_data: Arc<RwLock<ShareMap>>,
    shard_manager: Arc<Mutex<ShardManager>>,
) -> Result<EventSource> {
    if !CONFIG.berrytube.enabled {
        return Err(ErrorKind::DisabledInConfig.into());
    }

    trace!("Spawning BerryTube thread...");
    let source = EventSource::new(&format!("{}/sse", CONFIG.berrytube.origin))
        .map_err(|_| ErrorKind::InvalidUrl)?;

    source.add_event_listener("videoChange", move |event| {
        if let Ok(video) = serde_json::from_str::<Video>(&event.data) {
            if let Some(previous_title) = client_data
                .try_read_for(READ_TIMEOUT)
                .map(|data| data.get::<NowPlayingKey>().cloned())
            {
                if previous_title.map_or(true, |prev| video.title != prev) {
                    for runner in shard_manager.lock().runners.lock().values() {
                        let messenger = ShardMessenger::new(runner.runner_tx.clone());
                        messenger.set_activity(Some(Activity::playing(&video.title)));
                    }

                    if let Some(mut data) = client_data.try_write_for(WRITE_TIMEOUT) {
                        data.insert::<NowPlayingKey>(video.title);
                    }
                }
            }
        }
    });

    Ok(source)
}
