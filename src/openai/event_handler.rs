use crate::word_chunks::WordChunks;
use crate::{discord::get_data, openai::OpenAiKey};
use openai_dive::v1::resources::assistant::message::{MessageContent, Text, TextContent};
use serenity::all::{Context, CreateAllowedMentions, CreateMessage, Message};
use serenity::{
    constants::MESSAGE_CODE_LIMIT,
    model::channel::MessageFlags,
    utils::{content_safe, ContentSafeOptions},
};

pub async fn message(ctx: &Context, message: &Message) {
    if let Ok(openai) = get_data::<OpenAiKey>(&ctx).await {
        let typing = message.channel_id.start_typing(&ctx.http);

        let mut safe_opts = ContentSafeOptions::default().show_discriminator(false);
        if let Some(guild_id) = message.guild_id {
            safe_opts = safe_opts.display_as_member_from(guild_id);
        }

        let responses = openai.chat(&ctx, &message).await.unwrap_or_else(|err| {
            log::error!("OpenAI error: {}", err);
            vec![MessageContent::Text(TextContent {
                r#type: String::from("text"),
                text: Text {
                    value: err.to_string(),
                    annotations: Vec::new(),
                },
            })]
        });

        typing.stop();
        for response in responses {
            match response {
                MessageContent::ImageFile(_) => {
                    // TODO
                }
                MessageContent::Text(content) => {
                    let response =
                        content_safe(&ctx, content.text.value, &safe_opts, &message.mentions);
                    let response: Vec<_> =
                        WordChunks::from_str(&response, MESSAGE_CODE_LIMIT).collect();

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
        }
    }
}
