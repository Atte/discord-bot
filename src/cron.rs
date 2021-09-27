use crate::{config::CronConfig, Result};
use chrono::{Duration, Utc};
use log::info;
use serenity::{http::client::Http, model::id::ChannelId};
use std::{collections::HashMap, sync::Arc};

pub struct Cron {
    http: Arc<Http>,
    delete_old_messages: HashMap<ChannelId, i64>,
    pub rate: u64,
}

impl Cron {
    pub fn new(config: CronConfig, http: Arc<Http>) -> Self {
        Self {
            http,
            delete_old_messages: config.delete_old_messages,
            rate: config.rate,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        let now = Utc::now();
        for (channel_id, seconds) in &self.delete_old_messages {
            let max_age = Duration::seconds(*seconds);

            let messages = self.http.get_messages(channel_id.0, "limit=100").await?;
            let delete_message_ids: Vec<u64> = messages
                .iter()
                .filter_map(|msg| {
                    if msg.timestamp < now && (now - msg.timestamp) > max_age {
                        Some(msg.id.0)
                    } else {
                        None
                    }
                })
                .collect();

            match delete_message_ids.len() {
                0 => { /* nothing to delete */ }
                1 => {
                    info!("Deleting an obsolete message from {}", channel_id);
                    self.http
                        .delete_message(channel_id.0, delete_message_ids[0])
                        .await?;
                }
                len => {
                    info!("Deleting {} obsolete messages from {}", len, channel_id);
                    self.http
                        .delete_messages(channel_id.0, &serde_json::to_value(&delete_message_ids)?)
                        .await?;
                }
            }
        }
        Ok(())
    }
}
