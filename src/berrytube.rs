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
use std::sync::Arc;

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
            let mut data = client_data.write();
            let previous_title = data.get::<NowPlayingKey>();

            if previous_title.map_or(true, |prev| &video.title != prev) {
                let manager = shard_manager.lock();
                for id in manager.shards_instantiated() {
                    if let Some(shard) = manager.runners.lock().get(&id) {
                        let messenger = ShardMessenger::new(shard.runner_tx.clone());
                        messenger.set_activity(Some(Activity::playing(&video.title)));
                    }
                }

                data.insert::<NowPlayingKey>(video.title);
            }
        }
    });

    Ok(source)
}
