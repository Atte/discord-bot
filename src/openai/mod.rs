use std::{ops::Deref, sync::Arc};

use crate::{Result, config::OpenAiConfig};
use async_openai::{
    Client,
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
        ChatCompletionRequestMessageContentPartImageArgs,
        ChatCompletionRequestMessageContentPartTextArgs, ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestToolMessageArgs, ChatCompletionRequestToolMessageContent,
        ChatCompletionRequestUserMessageArgs, ChatCompletionResponseMessage,
        ChatCompletionToolChoiceOption, CreateChatCompletionRequestArgs, ImageDetail, ImageUrlArgs,
        ResponseFormat,
    },
};
use chrono::{Datelike, Utc, Weekday};
use color_eyre::eyre::eyre;
use lazy_regex::regex_replace;
use maplit::hashmap;
use serenity::{
    all::{Context, CreateAllowedMentions, CreateMessage, MESSAGE_CODE_LIMIT, Message},
    prelude::TypeMapKey,
};
use tokio::task::JoinSet;
use word_chunks::WordChunks;

mod tools;
mod word_chunks;

const MAX_TRIES: usize = 3;

#[derive(Debug)]
pub struct OpenAiKey;

impl TypeMapKey for OpenAiKey {
    type Value = Arc<OpenAi>;
}

pub struct OpenAi {
    client: Client<OpenAIConfig>,
    config: OpenAiConfig,
}

impl OpenAi {
    pub fn new(config: OpenAiConfig) -> Self {
        let mut client_config = OpenAIConfig::new().with_api_key(config.api_key.to_string());
        if let Some(ref url) = config.api_url {
            client_config = client_config.with_api_base(url.to_string());
        }
        Self {
            client: Client::with_config(client_config),
            config,
        }
    }

    async fn reply(
        &self,
        ctx: &Context,
        mut reply_to: Message,
        response: &ChatCompletionResponseMessage,
    ) -> Result<Message> {
        if let Some(ref content) = response.content {
            for chunk in WordChunks::from_str(
                &regex_replace!(r"^.*</think>\s*"s, content, ""),
                MESSAGE_CODE_LIMIT,
            ) {
                reply_to = reply_to
                    .channel_id
                    .send_message(
                        ctx,
                        CreateMessage::new()
                            .reference_message(&reply_to)
                            .allowed_mentions(CreateAllowedMentions::new().replied_user(false))
                            .content(chunk),
                    )
                    .await?;
            }
        }

        Ok(reply_to)
    }

    async fn user_message_to_api(
        ctx: &Context,
        msg: &Message,
    ) -> Result<ChatCompletionRequestMessage> {
        let mut parts = vec![
            ChatCompletionRequestMessageContentPartTextArgs::default()
                .text(regex_replace!(r"^<@!?[0-9]+>\s*", &msg.content, "").to_string())
                .build()?
                .into(),
        ];

        for attachment in &msg.attachments {
            parts.push(
                ChatCompletionRequestMessageContentPartImageArgs::default()
                    .image_url(
                        ImageUrlArgs::default()
                            .url(&attachment.url)
                            .detail(ImageDetail::Low)
                            .build()?,
                    )
                    .build()?
                    .into(),
            );
        }

        let mut builder = ChatCompletionRequestUserMessageArgs::default();
        if let Some(nick) = msg.author_nick(ctx).await {
            builder.name(nick);
        }
        Ok(builder.content(parts).build()?.into())
    }

    #[allow(clippy::too_many_lines)] // TODO
    pub async fn handle_message(&self, ctx: &Context, msg: Message) -> Result<()> {
        let _typing = msg.channel_id.start_typing(&ctx.http);

        let mut messages: Vec<ChatCompletionRequestMessage> =
            vec![Self::user_message_to_api(ctx, &msg).await?];

        let mut historical = msg.clone();
        while let Some(reference) = historical.message_reference {
            let Some(message_id) = reference.message_id else {
                break;
            };

            if let Some(hist) = ctx.cache.message(reference.channel_id, message_id) {
                historical = hist.deref().clone();
            } else {
                historical = ctx
                    .http
                    .get_message(reference.channel_id, message_id)
                    .await?;
            }

            if historical.author.id == ctx.cache.current_user().id {
                messages.insert(
                    0,
                    ChatCompletionRequestAssistantMessageArgs::default()
                        .content(historical.content)
                        .build()?
                        .into(),
                );
            } else {
                messages.insert(0, Self::user_message_to_api(ctx, &historical).await?);
            }
        }

        messages.insert(
            0,
            ChatCompletionRequestSystemMessageArgs::default()
                .content(
                    self.config
                        .prompt
                        .to_string()
                        .replace(
                            "{WEEKDAY}",
                            match Utc::now().weekday() {
                                Weekday::Mon => "Monday",
                                Weekday::Tue => "Tuesday",
                                Weekday::Wed => "Wednesday",
                                Weekday::Thu => "Thursday",
                                Weekday::Fri => "Friday",
                                Weekday::Sat => "Saturday",
                                Weekday::Sun => "Sunday",
                            },
                        )
                        .trim(),
                )
                .build()?
                .into(),
        );

        let mut metadata = hashmap! {
            "user_id" => msg.author.id.to_string(),
            "user_name" => msg.author.name.clone(),
            "message_id" => msg.id.to_string(),
            "channel_id" => msg.channel_id.to_string(),
        };
        if let Some(nick) = msg.author_nick(ctx).await {
            metadata.insert("user_nick", nick);
        }
        if let Some(guild_id) = msg.guild_id {
            metadata.insert("guild_id", guild_id.to_string());
        }

        for i in 1..=MAX_TRIES {
            metadata.insert("try", i.to_string());
            let mut args = CreateChatCompletionRequestArgs::default();
            args.model(self.config.model.to_string())
                // .metadata(json!(metadata))
                // .store(true)
                .parallel_tool_calls(true)
                .response_format(ResponseFormat::Text)
                .temperature(self.config.temperature)
                .top_p(self.config.top_p)
                .messages(messages.clone());
            if self.config.tools {
                args.tools(tools::get_specs()?)
                    .tool_choice(if i == MAX_TRIES {
                        ChatCompletionToolChoiceOption::None
                    } else {
                        ChatCompletionToolChoiceOption::Auto
                    });
            }
            let args = args.build()?;
            // println!("{args:?}");
            let result = self.client.chat().create(args).await?;
            // println!("{result:?}");

            let response = result
                .choices
                .first()
                .ok_or_else(|| eyre!("No choices in API response"))?;

            if let Some(content) = response.message.content.clone() {
                messages.push(
                    ChatCompletionRequestAssistantMessageArgs::default()
                        .content(content)
                        .build()?
                        .into(),
                );
            }

            if let Some(calls) = response.message.tool_calls.clone() {
                if !calls.is_empty() {
                    let mut joinset = JoinSet::new();
                    for call in calls {
                        joinset.spawn(async move {
                            let text = tools::run(&call).await;
                            ChatCompletionRequestToolMessageArgs::default()
                                .tool_call_id(call.id)
                                .content(ChatCompletionRequestToolMessageContent::Text(text))
                                .build()
                        });
                    }
                    while let Some(response) = joinset.join_next().await {
                        messages.push(response??.into());
                    }
                    continue;
                }
            }

            self.reply(ctx, msg, &response.message).await?;
            break;
        }

        Ok(())
    }
}
