use std::{iter::once, sync::Arc};

use crate::{word_chunks::WordChunks, Result};
use async_openai::{
    config::OpenAIConfig,
    types::{
        AssistantStreamEvent, AssistantsApiResponseFormatOption, CreateMessageRequestArgs,
        CreateMessageRequestContent, CreateRunRequestArgs, CreateThreadRequestArgs, ImageDetail,
        ImageUrlArgs, MessageContent, MessageContentImageUrlObject, MessageContentInput,
        MessageRequestContentTextObject, MessageRole, ResponseFormat, RunObject,
        SubmitToolOutputsRunRequest, ToolsOutputsArgs,
    },
    Client,
};
use bson::{doc, Bson};
use futures::StreamExt;
use lazy_static::lazy_static;
use maplit::{convert_args, hashmap};
use mongodb::{Collection, Database};
use regex::Regex;
use serenity::{
    all::{
        Context, CreateAttachment, CreateEmbed, CreateMessage, Message, MessageBuilder,
        MESSAGE_CODE_LIMIT,
    },
    prelude::TypeMapKey,
};
use tokio::task::JoinSet;

mod models;
mod tools;
use models::*;

lazy_static! {
    static ref MESSAGE_CLEANUP_RE: Regex =
        Regex::new(r"^<@[0-9]+>\s*").expect("invalid message cleanup regex");
}

#[derive(Debug)]
pub struct OpenAiKey;

impl TypeMapKey for OpenAiKey {
    type Value = Arc<OpenAi>;
}

pub struct OpenAi {
    client: Client<OpenAIConfig>,
    assistant_id: String,
    log: Collection<LogEntry>,
}

impl OpenAi {
    pub fn new(config: &crate::config::OpenAiConfig, db: Database) -> Self {
        Self {
            client: Client::with_config(
                OpenAIConfig::new().with_api_key(config.api_key.to_string()),
            ),
            assistant_id: config.assistant_id.to_string(),
            log: db.collection("openai-log"),
        }
    }

    async fn find_thread_id(&self, msg: &Message) -> Result<Option<String>> {
        let mut ids = Vec::with_capacity(2);
        ids.push(msg.id.to_string());

        if let Some(ref msgref) = msg.message_reference {
            ids.extend(msgref.message_id.map(|r| r.to_string()));
        }

        let entry = self
            .log
            .find_one(doc! {
                "$or": [
                    { "message.id": &ids },
                    { "responses": { "id": ids } }
                ]
            })
            .sort(doc! { "time": -1 })
            .await?;
        Ok(entry.map(|e| e.thread.id))
    }

    async fn after_run(&self, entry_id: &Bson, run: RunObject) -> Result<()> {
        self.log
            .update_one(
                doc! { "_id": &entry_id },
                doc! { "$set": { "usage": bson::to_bson(&run.usage)? } },
            )
            .await?;

        if let Some(err) = run.last_error {
            self.log
                .update_one(
                    doc! { "_id": &entry_id },
                    doc! { "$push": { "errors": err.message } },
                )
                .await?;
        }

        Ok(())
    }

    async fn reply(
        &self,
        ctx: &Context,
        entry_id: &Bson,
        reply_to: &Message,
        content: CreateMessage,
        attachment: Option<CreateAttachment>,
    ) -> Result<Message> {
        let msg = if let Some(attachment) = attachment {
            reply_to
                .channel_id
                .send_files(ctx, once(attachment), content.reference_message(reply_to))
                .await?
        } else {
            reply_to
                .channel_id
                .send_message(ctx, content.reference_message(reply_to))
                .await?
        };

        self.log
            .update_one(
                doc! { "_id": &entry_id },
                doc! { "$push": { "responses": { "id": msg.id.to_string(), "length": msg.content.len() as u32 } } },
            )
            .await?;

        Ok(msg)
    }

    pub async fn handle_message(&self, ctx: &Context, msg: &Message) -> Result<()> {
        let _typing = msg.channel_id.start_typing(&ctx.http);

        let content = MESSAGE_CLEANUP_RE.replace_all(&msg.content, "");

        let thread_id = if let Some(thread_id) = self.find_thread_id(msg).await? {
            thread_id
        } else {
            self.client
                .threads()
                .create(CreateThreadRequestArgs::default().build()?)
                .await?
                .id
        };

        let log_entry = LogEntry {
            time: bson::DateTime::now(),
            user: LogEntryUser {
                id: msg.author.id,
                name: msg.author.name.clone(),
                nick: msg.author_nick(ctx).await,
            },
            message: LogEntryMessage {
                id: msg.id,
                length: content.len(),
            },
            channel: LogEntryChannel { id: msg.channel_id },
            guild: LogEntryGuild {
                id: msg.guild_id.unwrap_or_default(),
            },
            responses: Vec::new(),
            errors: Vec::new(),
            thread: LogEntryThread { id: thread_id },
            usage: None,
        };
        let entry_id = self.log.insert_one(&log_entry).await?.inserted_id;

        let mut openai_content = vec![MessageContentInput::Text(MessageRequestContentTextObject {
            text: content.to_string(),
        })];
        for attachment in &msg.attachments {
            openai_content.push(MessageContentInput::ImageUrl(
                MessageContentImageUrlObject {
                    image_url: ImageUrlArgs::default()
                        .url(&attachment.url)
                        .detail(ImageDetail::Low)
                        .build()?,
                },
            ));
        }

        self.client
            .threads()
            .messages(&log_entry.thread.id)
            .create(
                CreateMessageRequestArgs::default()
                    .role(MessageRole::User)
                    .content(CreateMessageRequestContent::ContentArray(openai_content))
                    .metadata(convert_args!(hashmap!(
                        "user_id" => log_entry.user.id.to_string(),
                        "user_name" => log_entry.user.name.clone(),
                        "user_nick" => log_entry.user.nick.unwrap_or_else(|| log_entry.user.name.clone()),
                        "message_id" => log_entry.message.id.to_string(),
                        "channel_id" => log_entry.channel.id.to_string(),
                        "guild_id" => log_entry.guild.id.to_string(),
                    )))
                    .build()?,
            )
            .await?;

        let mut stream = self
            .client
            .threads()
            .runs(&log_entry.thread.id)
            .create_stream(
                CreateRunRequestArgs::default()
                    .assistant_id(&self.assistant_id)
                    .parallel_tool_calls(true)
                    .tools(tools::get_specs())
                    .stream(true)
                    .response_format(AssistantsApiResponseFormatOption::Format(
                        ResponseFormat::Text,
                    ))
                    .build()?,
            )
            .await?;

        let mut reply_to = msg.clone();
        while let Some(event) = stream.next().await {
            let event = event?;
            // log::trace!("{event:?}");
            match event {
                AssistantStreamEvent::ThreadRunRequiresAction(run) => {
                    if let Some(action) = run.required_action {
                        let mut tasks = JoinSet::new();
                        for call in action.submit_tool_outputs.tool_calls {
                            tasks.spawn(async move {
                                ToolsOutputsArgs::default()
                                    .tool_call_id(call.id)
                                    .output(tools::run(call.function).await)
                                    .build()
                            });
                        }

                        let mut tool_outputs = Vec::new();
                        while let Some(result) = tasks.join_next().await {
                            tool_outputs.push(result??);
                        }

                        stream = self
                            .client
                            .threads()
                            .runs(&log_entry.thread.id)
                            .submit_tool_outputs_stream(
                                &run.id,
                                SubmitToolOutputsRunRequest {
                                    tool_outputs,
                                    stream: None,
                                },
                            )
                            .await?;
                    }
                }

                AssistantStreamEvent::ThreadRunCompleted(run)
                | AssistantStreamEvent::ThreadRunIncomplete(run)
                | AssistantStreamEvent::ThreadRunFailed(run)
                | AssistantStreamEvent::ThreadRunCancelled(run) => {
                    self.after_run(&entry_id, run).await?;
                }

                AssistantStreamEvent::ThreadMessageIncomplete(message)
                | AssistantStreamEvent::ThreadMessageCompleted(message) => {
                    for content in &message.content {
                        match content {
                            MessageContent::Text(content) => {
                                for chunk in
                                    WordChunks::from_str(&content.text.value, MESSAGE_CODE_LIMIT)
                                {
                                    reply_to = self
                                        .reply(
                                            ctx,
                                            &entry_id,
                                            &reply_to,
                                            CreateMessage::new().content(chunk),
                                            None,
                                        )
                                        .await?;
                                }
                            }
                            MessageContent::ImageFile(content) => {
                                let file = self
                                    .client
                                    .files()
                                    .content(&content.image_file.file_id)
                                    .await?;
                                reply_to = self
                                    .reply(
                                        ctx,
                                        &entry_id,
                                        &reply_to,
                                        CreateMessage::new(),
                                        Some(CreateAttachment::bytes(file, "image.png")),
                                    )
                                    .await?;
                            }
                            MessageContent::ImageUrl(content) => {
                                reply_to = self
                                    .reply(
                                        ctx,
                                        &entry_id,
                                        &reply_to,
                                        CreateMessage::new().add_embed(
                                            CreateEmbed::new().image(&content.image_url.url),
                                        ),
                                        None,
                                    )
                                    .await?;
                            }
                            MessageContent::Refusal(content) => {
                                reply_to = self
                                    .reply(
                                        ctx,
                                        &entry_id,
                                        &reply_to,
                                        CreateMessage::new().content(&content.refusal),
                                        None,
                                    )
                                    .await?;
                            }
                        }
                    }
                }

                AssistantStreamEvent::ErrorEvent(err) => {
                    reply_to = reply_to
                        .reply(
                            ctx,
                            MessageBuilder::new()
                                .push_codeblock_safe(&err.message, None)
                                .build(),
                        )
                        .await?;

                    self.log
                        .update_one(
                            doc! { "_id": &entry_id },
                            doc! { "$push": { "errors": err.message } },
                        )
                        .await?;
                }

                _ => {
                    // ignore other events
                }
            }
        }

        Ok(())
    }
}
