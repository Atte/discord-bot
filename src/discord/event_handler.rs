use super::InitialActivityKey;
use crate::util::ellipsis_string;
use log::info;
use serenity::{
    async_trait,
    client::{Context, EventHandler},
    model::{
        gateway::{Activity, Ready},
        id::{ChannelId, MessageId},
        user::OnlineStatus,
    },
};

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        let activity = {
            let data = ctx.data.read().await;
            data.get::<InitialActivityKey>().cloned()
        };
        ctx.set_presence(
            activity.map(|a| Activity::playing(&ellipsis_string(a, 128))),
            OnlineStatus::Online,
        )
        .await;
    }

    async fn message_delete(&self, ctx: Context, channel_id: ChannelId, message_id: MessageId) {
        if let Some(msg) = ctx.cache.message(channel_id, message_id).await {
            info!("Message deleted: {:?}", msg);
        }
    }

    async fn message_delete_bulk(
        &self,
        ctx: Context,
        channel_id: ChannelId,
        messages_ids: Vec<MessageId>,
    ) {
        for message_id in messages_ids {
            if let Some(msg) = ctx.cache.message(channel_id, message_id).await {
                info!("Message deleted: {:?}", msg);
            }
        }
    }
}
