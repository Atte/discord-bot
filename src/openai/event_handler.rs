use serenity::all::{Context, CreateAllowedMentions, CreateMessage, Message};

use crate::discord::get_data;

pub async fn message(ctx: &Context, message: &Message) {
    use crate::openai::{OpenAiKey, OpenAiMessage, OpenAiRequest, OpenAiUserMessage};
    use crate::word_chunks::WordChunks;
    use serenity::{
        constants::MESSAGE_CODE_LIMIT,
        model::channel::MessageFlags,
        utils::{content_safe, ContentSafeOptions},
    };

    if let Ok(openai) = get_data::<OpenAiKey>(&ctx).await {
        let typing = message.channel_id.start_typing(&ctx.http);

        let mut safe_opts = ContentSafeOptions::default().show_discriminator(false);
        let current_user_id = ctx.cache.current_user().id.clone();
        let mut my_nick = ctx.cache.current_user().name.to_owned();
        if let Some(guild_id) = message.guild_id {
            safe_opts = safe_opts.display_as_member_from(guild_id);
            if let Ok(member) = guild_id.member(&ctx, current_user_id).await {
                my_nick = member.display_name().to_owned();
            }
        }

        let mut request = OpenAiRequest::new(Some(message.author.tag()));

        // TODO: use safe_reply
        let mut reply = message.clone();
        for _ in 0..100 {
            let text = content_safe(&ctx, &reply.content, &safe_opts, &reply.mentions);
            let text = text
                .trim_start()
                .strip_prefix(&format!("@{my_nick}"))
                .unwrap_or_else(|| text.as_ref())
                .trim();

            if request
                .try_unshift_message(if reply.is_own(&ctx) {
                    OpenAiMessage::Assistant {
                        content: Some(text.to_owned()),
                        #[cfg(feature = "openai-functions")]
                        function_call: None,
                    }
                } else {
                    eprintln!("{reply:?}");
                    let mut text = text.to_owned();
                    #[cfg(feature = "openai-vision")]
                    {
                        for attach in reply.attachments {
                            if let Some("image/jpeg") | Some("image/png") | Some("image/webp") =
                                attach.content_type.as_deref()
                            {
                                text.push_str(&format!(" {}", attach.url));
                            }
                        }
                        for embed in reply.embeds {
                            if let Some(image) = embed.image {
                                text.push_str(&format!(" {}", image.url));
                            }
                        }
                    }
                    OpenAiMessage::User {
                        content: vec![OpenAiUserMessage::Text { text }],
                    }
                })
                .is_err()
            {
                break;
            }

            if let Some((channel_id, message_id)) = reply
                .message_reference
                .and_then(|r| r.message_id.map(|id| (r.channel_id, id)))
            {
                if let Some(referenced) = ctx.cache.message(channel_id, message_id) {
                    reply = referenced.clone();
                    continue;
                }
                if let Ok(referenced) = ctx.http.get_message(channel_id, message_id).await {
                    reply = referenced;
                    continue;
                }
            }

            break;
        }

        let response = openai
            .chat(&ctx, &message, request, my_nick)
            .await
            .unwrap_or_else(|err| {
                log::error!("OpenAI error: {}", err);
                err.to_string()
            });

        let response = content_safe(&ctx, response, &safe_opts, &message.mentions);
        let response: Vec<_> = WordChunks::from_str(&response, MESSAGE_CODE_LIMIT).collect();

        typing.stop();

        let mut reply_to = message.clone();
        for chunk in response {
            match message
                .channel_id
                .send_message(
                    &ctx,
                    CreateMessage::new()
                        .allowed_mentions(CreateAllowedMentions::new())
                        .reference_message(&reply_to)
                        .flags(MessageFlags::SUPPRESS_EMBEDS)
                        .content(chunk),
                )
                .await
            {
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
