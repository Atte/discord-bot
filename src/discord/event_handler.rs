use super::{
    ActivityKey, automod, limits::ACTIVITY_LENGTH, log_channel, stats::update_stats, sticky_roles,
    volatiles,
};
use crate::util::ellipsis_string;
use log::error;
use serenity::{
    all::{ActivityData, GuildMemberUpdateEvent},
    async_trait,
    client::{Context, EventHandler},
    model::{
        channel::Message,
        gateway::Ready,
        guild::Member,
        id::{ChannelId, GuildId, MessageId},
        user::User,
    },
};

#[derive(Debug)]
pub struct Handler;

#[allow(clippy::ignored_unit_patterns)] // async_trait
#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, _ready: Ready) {
        if let Some(activity) = {
            let data = ctx.data.read().await;
            data.get::<ActivityKey>()
                .map(|a| ellipsis_string(a, ACTIVITY_LENGTH))
        } {
            ctx.set_activity(Some(ActivityData::custom(&activity)));
        }
    }

    async fn message(&self, ctx: Context, message: Message) {
        if let Err(err) = update_stats(&ctx, &message).await {
            error!("Error in update_stats for normal_message: {err:?}");
        }

        if message.author.bot {
            return;
        }

        if let Err(err) = automod::enforce(&ctx, &message).await {
            error!("Error enforcing automod: {err:?}");
        }

        if let Err(err) = volatiles::enforce(&ctx, &message).await {
            error!("Error enforcing volatiles: {err:?}");
        }

        #[cfg(feature = "openai")]
        if let Ok(config) = super::get_data::<super::ConfigKey>(&ctx).await {
            if config
                .discord
                .command_channels
                .contains(&message.channel_id)
                && matches!(message.mentions_me(&ctx).await, Ok(true))
            {
                let _ = tokio::task::spawn(async move {
                    if let Ok(openai) = super::get_data::<crate::openai::OpenAiKey>(&ctx).await {
                        if let Err(err) = openai.handle_message(&ctx, message.clone()).await {
                            error!("OpenAI error: {err:?}");
                            let _ = message
                                .reply(
                                    ctx,
                                    serenity::all::MessageBuilder::new()
                                        .push_codeblock_safe(err.to_string(), None)
                                        .build(),
                                )
                                .await;
                        }
                    }
                })
                .await;
            }
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
            if let Some(message) = ctx
                .cache
                .message(channel_id, message_id)
                .map(|msg| msg.clone())
            {
                if let Err(err) =
                    log_channel::message_deleted(&ctx, channel_id, guild_id, message.clone()).await
                {
                    error!("Unable to log message deletion: {err:?}");
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
                if let Some(message) = ctx
                    .cache
                    .message(channel_id, message_id)
                    .map(|msg| msg.clone())
                {
                    if let Err(err) =
                        log_channel::message_deleted(&ctx, channel_id, guild_id, message).await
                    {
                        error!("Unable to log message deletion: {err:?}");
                    }
                }
            }
        }
    }

    async fn guild_member_addition(&self, ctx: Context, mut member: Member) {
        if let Err(err) = log_channel::member_added(&ctx, member.guild_id, &member.user).await {
            error!("Unable to log member addition: {err:?}");
        }
        if let Err(err) = sticky_roles::apply_stickies(&ctx, &mut member).await {
            error!("Unable to apply stickies: {err:?}");
        }
    }

    async fn guild_member_removal(
        &self,
        ctx: Context,
        guild_id: GuildId,
        user: User,
        _member: Option<Member>,
    ) {
        if let Err(err) = log_channel::member_removed(&ctx, guild_id, &user).await {
            error!("Unable to log member removal: {err:?}");
        }
    }

    async fn guild_member_update(
        &self,
        ctx: Context,
        old_member: Option<Member>,
        new_member: Option<Member>,
        _event: GuildMemberUpdateEvent,
    ) {
        let Some(new_member) = new_member else {
            return;
        };

        if let Err(err) = log_channel::member_updated(&ctx, old_member.as_ref(), &new_member).await
        {
            error!("Unable to log member update: {err:?}");
        }
        if let Err(err) = sticky_roles::save_stickies(&ctx, &new_member).await {
            error!("Unable to save stickies: {err:?}");
        }
    }
}
