use super::InitialActivityKey;
use crate::util::ellipsis_string;
use log::info;
use serenity::{
    async_trait,
    client::{Context, EventHandler},
    model::{
        gateway::{Activity, Ready},
        id::{ChannelId, MessageId},
    },
};

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, #[allow(unused_variables)] ready: Ready) {
        if let Some(activity) = {
            let data = ctx.data.read().await;
            data.get::<InitialActivityKey>()
                .map(|a| ellipsis_string(a, 128))
        } {
            ctx.set_activity(Activity::playing(&activity)).await;
        }
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
