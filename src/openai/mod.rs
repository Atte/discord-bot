use std::sync::Arc;

use crate::Result;
use async_openai::{
    Client,
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
        ChatCompletionRequestMessageContentPartImageArgs,
        ChatCompletionRequestMessageContentPartTextArgs, ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestToolMessageArgs, ChatCompletionRequestToolMessageContent,
        ChatCompletionRequestUserMessageArgs, ChatCompletionResponseMessage,
        CreateChatCompletionRequestArgs, ImageDetail, ImageUrlArgs,
    },
};
use color_eyre::eyre::eyre;
use lazy_regex::regex_replace;
use serde_json::json;
use serenity::{
    all::{Context, CreateAllowedMentions, CreateMessage, MESSAGE_CODE_LIMIT, Message},
    prelude::TypeMapKey,
};
use tokio::task::JoinSet;
use word_chunks::WordChunks;

mod tools;
mod word_chunks;

#[derive(Debug)]
pub struct OpenAiKey;

impl TypeMapKey for OpenAiKey {
    type Value = Arc<OpenAi>;
}

pub struct OpenAi {
    client: Client<OpenAIConfig>,
    prompt: String,
}

impl OpenAi {
    pub fn new(config: &crate::config::OpenAiConfig) -> Self {
        let mut client_config = OpenAIConfig::new().with_api_key(config.api_key.to_string());
        if let Some(ref url) = config.api_url {
            client_config = client_config.with_api_base(url.to_string());
        }
        Self {
            client: Client::with_config(client_config),
            prompt: config.prompt.to_string(),
        }
    }

    async fn reply(
        &self,
        ctx: &Context,
        mut reply_to: Message,
        response: &ChatCompletionResponseMessage,
    ) -> Result<Message> {
        if let Some(ref content) = response.content {
            for chunk in WordChunks::from_str(content, MESSAGE_CODE_LIMIT) {
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

    pub async fn handle_message(&self, ctx: &Context, msg: Message) -> Result<()> {
        let _typing = msg.channel_id.start_typing(&ctx.http);

        let mut messages: Vec<ChatCompletionRequestMessage> =
            vec![Self::user_message_to_api(ctx, &msg).await?];

        let mut historical = Box::new(msg.clone());
        while let Some(hist) = historical.referenced_message.clone() {
            historical = hist;
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
                .content(self.prompt.clone())
                .build()?
                .into(),
        );

        loop {
            let result = self
                .client
                .chat()
                .create(
                    CreateChatCompletionRequestArgs::default()
                        .metadata(json!({
                                        "user_id": msg.author.id.to_string(),
                                        "user_name": msg.author.name.clone(),
                                        "user_nick": msg.author_nick(ctx).await,
                                        "message_id": msg.id.to_string(),
                                        "channel_id": msg.channel_id.to_string(),
                                        "guild_id": msg.guild_id.map(|id| id.to_string()),
                        }))
                        .parallel_tool_calls(true)
                        .tools(tools::get_specs()?)
                        .messages(messages.clone())
                        .build()?,
                )
                .await?;

            let response = result
                .choices
                .first()
                .ok_or_else(|| eyre!("No choices in API response"))?;

            messages.push(response.message.into());

            if let Some(calls) = response.message.tool_calls.clone() {
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

            self.reply(ctx, msg, &response.message).await?;
            break;
        }

        Ok(())
    }
}
