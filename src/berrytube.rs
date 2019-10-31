use crate::{
    socketio::{self, SocketIOClient},
    CONFIG,
};
use error_chain::error_chain;
use log::{error, trace};
use percent_encoding::percent_decode_str;
use serde::Deserialize;
use serenity::{
    client::bridge::gateway::{ShardManager, ShardMessenger},
    model::gateway::Activity,
    prelude::*,
};
use std::{io, sync::Arc, thread, time::Duration};

error_chain! {
    links {
        SocketIO(socketio::Error, socketio::ErrorKind);
    }

    foreign_links {
        Io(::std::io::Error);
        Discord(::serenity::Error);
        Json(::serde_json::Error);
        ParseUrl(::websocket::url::ParseError);
    }
}

#[derive(Debug, Deserialize)]
struct VideoDetails {
    videotitle: String,
}

#[derive(Debug, Deserialize)]
struct VideoDetailMessage {
    video: VideoDetails,
}

fn main(shard_manager: &Arc<Mutex<ShardManager>>) -> Result<()> {
    let mut previous_title = String::new();

    let mut sock = SocketIOClient::new(CONFIG.berrytube.origin.to_string().parse()?);
    sock.run(move |event| {
        if event.name == "hbVideoDetail" || event.name == "forceVideoChange" {
            if let Some(VideoDetailMessage { video }) = event
                .args
                .first()
                .cloned()
                .and_then(|arg| serde_json::from_value::<VideoDetailMessage>(arg).ok())
            {
                let title = percent_decode_str(video.videotitle.as_ref())
                    .decode_utf8_lossy()
                    .to_string();

                if title != previous_title {
                    trace!("video change: {}", title);

                    let manager = shard_manager.lock();
                    for id in manager.shards_instantiated() {
                        if let Some(shard) = manager.runners.lock().get(&id) {
                            let messenger = ShardMessenger::new(shard.runner_tx.clone());
                            messenger.set_activity(Some(Activity::playing(&title)));
                        }
                    }

                    previous_title = title;
                }
            }
        }
        None
    })?;
    Ok(())
}

pub fn spawn(shard_manager: Arc<Mutex<ShardManager>>) -> io::Result<thread::JoinHandle<()>> {
    if !CONFIG.berrytube.enabled {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "BerryTube functionality is disabled in config",
        ));
    }

    trace!("Spawning BerryTube thread...");

    thread::Builder::new()
        .name("berrytube".to_owned())
        .spawn(move || loop {
            if let Err(err) = main(&shard_manager) {
                error!("{}", err);
            }
            thread::sleep(Duration::from_secs(60));
        })
}
