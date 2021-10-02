use crate::config::CronConfig;
use anyhow::Result;
use chrono::{Duration, Utc};
use log::info;
use serenity::{
    model::id::{ChannelId, MessageId},
    CacheAndHttp,
};
use std::{collections::HashMap, sync::Arc};

pub struct Cron {
    discord: Arc<CacheAndHttp>,
    delete_old_messages: HashMap<ChannelId, i64>,
    pub rate: u64,
}

impl Cron {
    #[inline]
    pub fn new(config: CronConfig, discord: Arc<CacheAndHttp>) -> Self {
        Self {
            discord,
            delete_old_messages: config.delete_old_messages,
            rate: config.rate,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        let now = Utc::now();
        for (channel_id, seconds) in &self.delete_old_messages {
            let max_age = Duration::seconds(*seconds);

            let messages = channel_id
                .messages(&self.discord.http, |c| c.limit(100))
                .await?;
            let delete_message_ids: Vec<MessageId> = messages
                .iter()
                .filter_map(|msg| {
                    if msg.timestamp < now && (now - msg.timestamp) > max_age {
                        Some(msg.id)
                    } else {
                        None
                    }
                })
                .collect();

            match delete_message_ids.len() {
                0 => { /* nothing to delete */ }
                1 => {
                    info!("Deleting an obsolete message from {}", channel_id);
                    channel_id
                        .delete_message(&self.discord.http, delete_message_ids[0])
                        .await?;
                }
                len => {
                    info!("Deleting {} obsolete messages from {}", len, channel_id);
                    channel_id
                        .delete_messages(&self.discord.http, delete_message_ids)
                        .await?;
                }
            }
        }
        Ok(())
    }
}
