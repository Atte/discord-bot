use std::{iter::once, sync::Arc};

use crate::Result;
use async_openai::{
    config::OpenAIConfig,
    types::{
        AssistantEventStream, AssistantStreamEvent, AssistantsApiResponseFormatOption,
        CreateMessageRequestArgs, CreateMessageRequestContent, CreateRunRequestArgs,
        CreateThreadRequestArgs, ImageDetail, ImageUrlArgs, MessageContent,
        MessageContentImageUrlObject, MessageContentInput, MessageContentRefusalObject,
        MessageContentTextObject, MessageRequestContentTextObject, MessageRole, RequiredAction,
        ResponseFormat, RunObject, SubmitToolOutputsRunRequest, TextData, ToolsOutputsArgs,
    },
    Client,
};
use bson::{doc, Bson};
use futures::StreamExt;
use lazy_regex::regex_replace;
use log_entry::LogEntry;
use maplit::{convert_args, hashmap};
use mongodb::{Collection, Database};
use serenity::{
    all::{
        Context, CreateAllowedMentions, CreateAttachment, CreateEmbed, CreateMessage, Message,
        MessageBuilder, MESSAGE_CODE_LIMIT,
    },
    prelude::TypeMapKey,
};
use tokio::task::JoinSet;
use word_chunks::WordChunks;

mod log_entry;
mod tools;
mod word_chunks;

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
    pub fn new(config: &crate::config::OpenAiConfig, db: &Database) -> Self {
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
                    { "message.id": { "$in": &ids } },
                    { "responses": { "$elemMatch": { "id": { "$in": ids } } } }
                ]
            })
            .sort(doc! { "time": -1 })
            .await?;

        Ok(entry.map(|e| e.thread.id))
    }

    async fn after_run(&self, entry_id: &Bson, run: RunObject) -> Result<()> {
        if let Some(usage) = run.usage {
            self.log
                .update_one(
                    doc! { "_id": &entry_id },
                    doc! { "$set": { "usage": bson::to_bson(&usage)? } },
                )
                .await?;
        }

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
        mut reply_to: Message,
        contents: impl IntoIterator<Item = MessageContent>,
    ) -> Result<Message> {
        // split long messages
        let contents = contents.into_iter().flat_map(|content| match content {
            MessageContent::Text(inner) => {
                WordChunks::from_str(&inner.text.value, MESSAGE_CODE_LIMIT)
                    .map(|chunk| {
                        MessageContent::Text(MessageContentTextObject {
                            text: TextData {
                                value: chunk.to_owned(),
                                annotations: Vec::new(),
                            },
                        })
                    })
                    .collect()
            }
            MessageContent::Refusal(inner) => {
                WordChunks::from_str(&inner.refusal, MESSAGE_CODE_LIMIT)
                    .map(|chunk| {
                        MessageContent::Refusal(MessageContentRefusalObject {
                            refusal: chunk.to_owned(),
                        })
                    })
                    .collect()
            }
            _ => vec![content],
        });

        for content in contents {
            let builder = CreateMessage::new()
                .reference_message(&reply_to)
                .allowed_mentions(CreateAllowedMentions::new().replied_user(false));
            let builder = match content {
                MessageContent::Text(content) => builder.content(content.text.value),
                MessageContent::Refusal(content) => builder.content(content.refusal),
                MessageContent::ImageUrl(content) => {
                    builder.add_embed(CreateEmbed::new().image(&content.image_url.url))
                }
                MessageContent::ImageFile(content) => {
                    let file = self
                        .client
                        .files()
                        .content(&content.image_file.file_id)
                        .await?;
                    builder.add_file(CreateAttachment::bytes(file, "image.png"))
                }
            };
            reply_to = reply_to.channel_id.send_message(ctx, builder).await?;

            self.log
                .update_one(
                    doc! { "_id": &entry_id },
                    doc! { "$push": { "responses": bson::to_bson(&log_entry::Message::from(&reply_to))? } },
                )
                .await?;
        }

        Ok(reply_to)
    }

    async fn run_tools(
        &self,
        run_id: &str,
        action: RequiredAction,
        thread_id: &str,
    ) -> Result<AssistantEventStream> {
        let mut tasks = JoinSet::new();
        for call in action.submit_tool_outputs.tool_calls {
            if call.r#type == "function" {
                tasks.spawn(async move {
                    ToolsOutputsArgs::default()
                        .tool_call_id(call.id)
                        .output(tools::run(call.function).await)
                        .build()
                });
            }
        }

        let mut tool_outputs = Vec::new();
        while let Some(result) = tasks.join_next().await {
            tool_outputs.push(result??);
        }

        Ok(self
            .client
            .threads()
            .runs(thread_id)
            .submit_tool_outputs_stream(
                run_id,
                SubmitToolOutputsRunRequest {
                    tool_outputs,
                    stream: None,
                },
            )
            .await?)
    }

    pub async fn handle_message(&self, ctx: &Context, mut msg: Message) -> Result<()> {
        let _typing = msg.channel_id.start_typing(&ctx.http);

        msg.content = regex_replace!(r"^<@[0-9]+>\s*", &msg.content, "").to_string();

        let thread_id = if let Some(thread_id) = self.find_thread_id(&msg).await? {
            thread_id
        } else {
            self.client
                .threads()
                .create(CreateThreadRequestArgs::default().build()?)
                .await?
                .id
        };

        let log_entry = LogEntry::new(ctx, &msg, thread_id.clone()).await;
        let entry_id = self.log.insert_one(&log_entry).await?.inserted_id;

        let mut openai_content = vec![MessageContentInput::Text(MessageRequestContentTextObject {
            text: msg.content.clone(),
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

        while let Some(event) = stream.next().await {
            match event? {
                AssistantStreamEvent::ThreadRunRequiresAction(run) => {
                    if let Some(action) = run.required_action {
                        stream = self.run_tools(&run.id, action, &thread_id).await?;
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
                    msg = self.reply(ctx, &entry_id, msg, message.content).await?;
                }

                AssistantStreamEvent::ErrorEvent(err) => {
                    msg = msg
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
