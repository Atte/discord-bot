use super::{log_channel, ActivityKey, MAX_ACTIVITY_LENGTH};
use crate::util::ellipsis_string;
use log::error;
use serenity::{
    async_trait,
    client::{Context, EventHandler},
    model::{
        channel::Channel,
        gateway::{Activity, Ready},
        guild::Member,
        id::{ChannelId, GuildId, MessageId},
        user::User,
    },
};

pub struct Handler;

// #[async_trait] seems to mess with unused parameter detection,
// so need to use #[allow(unused_variables)] instead of underscore prefix
#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, #[allow(unused_variables)] ready: Ready) {
        if let Some(activity) = {
            let data = ctx.data.read().await;
            data.get::<ActivityKey>()
                .map(|a| ellipsis_string(a, MAX_ACTIVITY_LENGTH))
        } {
            ctx.set_activity(Activity::playing(&activity)).await;
        }
    }

    async fn message_delete(&self, ctx: Context, channel_id: ChannelId, message_id: MessageId) {
        match channel_id.to_channel(&ctx).await {
            Ok(Channel::Guild(channel)) => {
                if let Some(message) = ctx.cache.message(channel_id, message_id).await {
                    if let Err(err) = log_channel::message_deleted(&ctx, &channel, message).await {
                        error!("Unable to log message deletion: {}", err);
                    }
                }
            }
            Ok(_) => {} // ignore deletions outside guilds
            Err(err) => error!(
                "Unable to log message deletion due to channel lookup failure: {}",
                err
            ),
        }
    }

    async fn message_delete_bulk(
        &self,
        ctx: Context,
        channel_id: ChannelId,
        messages_ids: Vec<MessageId>,
    ) {
        match channel_id.to_channel(&ctx).await {
            Ok(Channel::Guild(channel)) => {
                for message_id in messages_ids {
                    if let Some(message) = ctx.cache.message(channel_id, message_id).await {
                        if let Err(err) =
                            log_channel::message_deleted(&ctx, &channel, message).await
                        {
                            error!("Unable to log message deletion: {}", err);
                        }
                    }
                }
            }
            Ok(_) => {} // ignore deletions outside guilds
            Err(err) => error!(
                "Unable to log message deletion due to channel lookup failure: {}",
                err
            ),
        }
    }

    async fn guild_member_addition(&self, ctx: Context, guild_id: GuildId, member: Member) {
        if let Err(err) = log_channel::member_added(&ctx, guild_id, &member.user).await {
            error!("Unable to log member addition: {}", err);
        }
    }

    async fn guild_member_removal(
        &self,
        ctx: Context,
        guild_id: GuildId,
        user: User,
        #[allow(unused_variables)] member: Option<Member>,
    ) {
        if let Err(err) = log_channel::member_removed(&ctx, guild_id, &user).await {
            error!("Unable to log member removal: {}", err);
        }
    }

    async fn guild_member_update(
        &self,
        ctx: Context,
        old_member: Option<Member>,
        new_member: Member,
    ) {
        if let Err(err) = log_channel::member_updated(&ctx, old_member.as_ref(), &new_member).await
        {
            error!("Unable to log member update: {}", err);
        }
    }
}
