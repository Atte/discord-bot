use crate::config::CronConfig;
use chrono::{Duration, Utc};
use color_eyre::eyre::Result;
use log::info;
use serenity::{
    all::{GetMessages, Http},
    model::id::{ChannelId, MessageId},
};
use std::{collections::HashMap, sync::Arc};

pub struct Cron {
    http: Arc<Http>,
    delete_old_messages: HashMap<ChannelId, i64>,
    pub rate: u64,
}

impl Cron {
    #[inline]
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

            let messages = channel_id
                .messages(&self.http, GetMessages::new().limit(100))
                .await?;
            let delete_message_ids: Vec<MessageId> = messages
                .iter()
                .filter_map(|msg| {
                    if *msg.timestamp < now && (now - *msg.timestamp) > max_age {
                        Some(msg.id)
                    } else {
                        None
                    }
                })
                .collect();

            match delete_message_ids.len() {
                0 => { /* nothing to delete */ }
                1 => {
                    info!("Deleting an obsolete message from {channel_id}");
                    channel_id
                        .delete_message(&self.http, delete_message_ids[0])
                        .await?;
                }
                len => {
                    info!("Deleting {len} obsolete messages from {channel_id}");
                    channel_id
                        .delete_messages(&self.http, delete_message_ids)
                        .await?;
                }
            }
        }
        Ok(())
    }
}
