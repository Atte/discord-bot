use super::{
    get_data, limits::ACTIVITY_LENGTH, log_channel, rules_check, sticky_roles, ActivityKey,
    ConfigKey,
};
use crate::util::ellipsis_string;
use log::error;
use serenity::{
    async_trait,
    client::{Context, EventHandler},
    http::CacheHttp,
    model::{
        channel::{Message, Reaction},
        gateway::{Activity, Ready},
        guild::Member,
        id::{ChannelId, GuildId, MessageId},
        user::User,
    },
};

#[cfg(feature = "openai")]
use crate::openai::{OpenAiKey, OpenAiMessage, OpenAiMessageRole, OpenAiRequest};

#[derive(Debug)]
pub struct Handler;

// #[async_trait] seems to mess with unused parameter detection,
// so need to use #[allow(unused_variables)] instead of underscore prefix
#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, _ready: Ready) {
        if let Some(activity) = {
            let data = ctx.data.read().await;
            data.get::<ActivityKey>()
                .map(|a| ellipsis_string(a, ACTIVITY_LENGTH))
        } {
            ctx.set_activity(Activity::playing(&activity)).await;
        }
    }

    async fn message(&self, ctx: Context, message: Message) {
        if let Ok(config) = get_data::<ConfigKey>(&ctx).await {
            if config.discord.clean_channels.contains(&message.channel_id) && !message.is_own(&ctx)
            {
                if let Err(err) = message.delete(&ctx).await {
                    error!("Unable to delete clean channel spam: {err:?}");
                }
            }

            #[cfg(feature = "openai")]
            if config
                .discord
                .command_channels
                .contains(&message.channel_id)
                && matches!(message.mentions_me(&ctx).await, Ok(true))
            {
                use crate::word_chunks::WordChunks;
                use serenity::{
                    constants::MESSAGE_CODE_LIMIT,
                    utils::{content_safe, ContentSafeOptions},
                };

                if let Ok(openai) = get_data::<OpenAiKey>(&ctx).await {
                    message.channel_id.start_typing(&ctx.http).ok();

                    let mut safe_opts = ContentSafeOptions::default().show_discriminator(false);
                    let mut my_nick = ctx.cache.current_user().name;
                    if let Some(guild_id) = message.guild_id {
                        safe_opts = safe_opts.display_as_member_from(guild_id);
                        if let Ok(member) = guild_id.member(&ctx, ctx.cache.current_user_id()).await
                        {
                            my_nick = member.display_name().into_owned();
                        }
                    }

                    let mut request = OpenAiRequest::new(message.author.tag());

                    let mut reply = message.clone();
                    loop {
                        let text = content_safe(&ctx, &reply.content, &safe_opts, &reply.mentions);
                        let text = text
                            .trim_start()
                            .strip_prefix(&format!("@{}", my_nick))
                            .unwrap_or_else(|| text.as_ref())
                            .trim();

                        if request
                            .try_unshift_message(OpenAiMessage::new(
                                if reply.is_own(&ctx) {
                                    OpenAiMessageRole::Assistant
                                } else {
                                    OpenAiMessageRole::User
                                },
                                text,
                            ))
                            .is_err()
                        {
                            break;
                        }

                        if let Some((channel_id, message_id)) = reply
                            .message_reference
                            .and_then(|r| r.message_id.map(|id| (r.channel_id, id)))
                        {
                            if let Some(referenced) = ctx.cache.message(channel_id, message_id) {
                                reply = referenced;
                                continue;
                            }
                            if let Ok(referenced) =
                                ctx.http().get_message(channel_id.0, message_id.0).await
                            {
                                reply = referenced;
                                continue;
                            }
                        }

                        break;
                    }

                    message.channel_id.start_typing(&ctx.http).ok();
                    let response = openai.chat(request, my_nick).await.unwrap_or_else(|err| {
                        log::error!("OpenAI error: {}", err);
                        err.to_string()
                    });

                    let response = content_safe(&ctx, response, &safe_opts, &message.mentions);
                    let response: Vec<_> =
                        WordChunks::from_str(&response, MESSAGE_CODE_LIMIT).collect();

                    let mut reply_to = message;
                    for chunk in response {
                        match reply_to.reply(&ctx, chunk).await {
                            Ok(reply) => {
                                reply_to = reply;
                            }
                            Err(err) => {
                                log::error!("error sending response: {:?}", err);
                            }
                        }
                    }
                }
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
            if let Some(message) = ctx.cache.message(channel_id, message_id) {
                if let Err(err) =
                    log_channel::message_deleted(&ctx, channel_id, guild_id, message).await
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
                if let Some(message) = ctx.cache.message(channel_id, message_id) {
                    if let Err(err) =
                        log_channel::message_deleted(&ctx, channel_id, guild_id, message).await
                    {
                        error!("Unable to log message deletion: {err:?}");
                    }
                }
            }
        }
    }

    async fn guild_member_addition(&self, ctx: Context, member: Member) {
        if let Err(err) = log_channel::member_added(&ctx, member.guild_id, &member.user).await {
            error!("Unable to log member addition: {err:?}");
        }
        match sticky_roles::apply_stickies(&ctx, &member).await {
            Ok(true) => { /* user has been here before */ }
            Ok(false) => {
                if let Err(err) = rules_check::post_welcome(ctx, member).await {
                    error!("Unable to send welcome message: {err:?}");
                }
            }
            Err(err) => {
                error!("Unable to apply stickies: {err:?}");
            }
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
        new_member: Member,
    ) {
        if let Err(err) = log_channel::member_updated(&ctx, old_member.as_ref(), &new_member).await
        {
            error!("Unable to log member update: {err:?}");
        }
        if let Err(err) = sticky_roles::save_stickies(&ctx, &new_member).await {
            error!("Unable to save stickies: {err:?}");
        }
    }

    async fn reaction_add(&self, ctx: Context, reaction: Reaction) {
        if let Err(err) = rules_check::handle_reaction(ctx, reaction).await {
            error!("Error handling rule reaction: {err:?}");
        }
    }
}
