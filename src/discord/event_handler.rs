use super::{limits::ACTIVITY_LENGTH, log_channel, sticky_roles, ActivityKey};
use crate::util::ellipsis_string;
use log::error;
use serenity::{
    async_trait,
    client::{Context, EventHandler},
    model::{
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
                .map(|a| ellipsis_string(a, ACTIVITY_LENGTH))
        } {
            ctx.set_activity(Activity::playing(&activity)).await;
        }
    }

    async fn message_delete(
        &self,
        ctx: Context,
        channel_id: ChannelId,
        message_id: MessageId,
        guild_id: Option<GuildId>,
    ) {
        if let Some(guild_id) = guild_id {
            if let Some(message) = ctx.cache.message(channel_id, message_id).await {
                if let Err(err) =
                    log_channel::message_deleted(&ctx, channel_id, guild_id, message).await
                {
                    error!("Unable to log message deletion: {}", err);
                }
            }
        }
    }

    async fn message_delete_bulk(
        &self,
        ctx: Context,
        channel_id: ChannelId,
        messages_ids: Vec<MessageId>,
        guild_id: Option<GuildId>,
    ) {
        if let Some(guild_id) = guild_id {
            for message_id in messages_ids {
                if let Some(message) = ctx.cache.message(channel_id, message_id).await {
                    if let Err(err) =
                        log_channel::message_deleted(&ctx, channel_id, guild_id, message).await
                    {
                        error!("Unable to log message deletion: {}", err);
                    }
                }
            }
        }
    }

    async fn guild_member_addition(&self, ctx: Context, guild_id: GuildId, member: Member) {
        if let Err(err) = log_channel::member_added(&ctx, guild_id, &member.user).await {
            error!("Unable to log member addition: {}", err);
        }
        if let Err(err) = sticky_roles::apply_stickies(&ctx, &member).await {
            error!("Unable to apply stickies: {}", err);
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
        if let Err(err) = sticky_roles::save_stickies(&ctx, &new_member).await {
            error!("Unable to save stickies: {}", err);
        }
    }
}
