use crate::CONFIG;
use error_chain::error_chain;
use log::trace;
use serde::Deserialize;
use serenity::{
    client::bridge::gateway::{ShardManager, ShardMessenger},
    model::gateway::Activity,
    prelude::*,
};
use std::sync::Arc;
use sse_client::EventSource;

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

#[derive(Debug, Deserialize)]
struct Video {
    id: String,
    length: isize,
    title: String,
    #[serde(rename = "type")]
    videotype: String,
    volat: bool
}

pub fn spawn(shard_manager: Arc<Mutex<ShardManager>>) -> Result<EventSource> {
    if !CONFIG.berrytube.enabled {
        return Err(ErrorKind::DisabledInConfig.into());
    }

    trace!("Spawning BerryTube thread...");
    let previous_title = Mutex::new(String::new());
    let source = EventSource::new(&format!("{}/sse", CONFIG.berrytube.origin)).map_err(|_| ErrorKind::InvalidUrl)?;

    source.add_event_listener("videoChange", move |event| {
        if let Ok(video) = serde_json::from_str::<Video>(&event.data) {
            let mut prev = previous_title.lock();
            if video.title != *prev {
                let manager = shard_manager.lock();
                for id in manager.shards_instantiated() {
                    if let Some(shard) = manager.runners.lock().get(&id) {
                        let messenger = ShardMessenger::new(shard.runner_tx.clone());
                        messenger.set_activity(Some(Activity::playing(&video.title)));
                    }
                }

                *prev = video.title;
            }
        }
    });

    Ok(source)
}
