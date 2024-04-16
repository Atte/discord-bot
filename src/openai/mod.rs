use crate::{
    config::OpenAiConfig,
    discord::{get_data, DbKey},
};
use chrono::{DateTime, Utc};
use color_eyre::eyre::{bail, eyre, Result};
use conv::{UnwrapOrSaturate, ValueFrom};
use futures::future::join_all;
use itertools::Itertools;
use lazy_static::lazy_static;
use maplit::{convert_args, hashmap};
use mongodb::bson::doc;
use openai_dive::v1::{
    api::Client,
    helpers::format_response,
    resources::{
        assistant::{
            assistant::{AssistantParameters, ToolOutput, ToolOutputsParameters},
            message::{CreateMessageParameters, ListMessagesResponse, MessageContent, MessageRole},
            run::{CreateRunParameters, Run, RunAction, RunStatus},
            thread::{CreateThreadParameters, Thread},
        },
        shared::Usage,
    },
};
use regex::Regex;
use serde::Serialize;
use serenity::{
    model::prelude::Message,
    prelude::{Context, TypeMapKey},
};
use std::{sync::Arc, time::Duration};

mod functions;

pub mod event_handler;

lazy_static! {
    static ref CLEANUP_REGEX: Regex = Regex::new(r"\bhttps?:\/\/\S+").unwrap();
}

pub const THREADS_COLLECTION_NAME: &str = "openai-threads";
pub const USER_LOG_COLLECTION_NAME: &str = "openai-user-log";

#[derive(Debug, Clone, Serialize)]
struct RunIdQuery<'a> {
    run_id: &'a str,
}

#[derive(Debug, Clone, Serialize)]
struct UserLog {
    time: DateTime<Utc>,
    user_id: String,
    model: String,
    prompt_tokens: i64,
    completion_tokens: i64,
    total_tokens: i64,
}

#[derive(Debug)]
pub struct OpenAiKey;

impl TypeMapKey for OpenAiKey {
    type Value = Arc<OpenAi>;
}

pub struct OpenAi {
    client: Client,
    assistant_id: String,
    temperature: Option<f32>,
}

impl OpenAi {
    #[inline]
    pub fn new(config: &OpenAiConfig) -> Self {
        Self {
            client: Client::new(config.api_key.to_string()),
            assistant_id: config.assistant_id.to_string(),
            temperature: config.temperature,
        }
    }

    pub async fn init(&self) -> Result<()> {
        let assistant = self
            .client
            .assistants()
            .retrieve(&self.assistant_id)
            .await
            .map_err(|err| eyre!("assistant retrieve {err:?}"))?;

        self.client
            .assistants()
            .modify(
                &self.assistant_id,
                AssistantParameters {
                    model: assistant.model,
                    name: assistant.name,
                    description: assistant.description,
                    instructions: assistant.instructions,
                    tools: Some(functions::as_tools()?),
                    file_ids: assistant.file_ids,
                    metadata: assistant.metadata,
                },
            )
            .await
            .map_err(|err| eyre!("assistant modify {err:?}"))?;

        Ok(())
    }

    pub async fn chat(&self, ctx: &Context, msg: &Message) -> Result<Vec<MessageContent>> {
        let metadata = Some(convert_args!(
            keys = String::from,
            values = ToString::to_string,
            hashmap!(
                "message_id" => &msg.id,
                "author_id" => &msg.author.id,
                "author_username" => &msg.author.name,
                "channel_id" => &msg.channel_id,
            )
        ));

        let thread = match Self::resolve_and_update_thread(ctx, msg.clone()).await? {
            Some(thread) => thread,
            None => {
                let thread = self
                    .client
                    .assistants()
                    .threads()
                    .create(CreateThreadParameters {
                        messages: None,
                        metadata: metadata.clone(),
                    })
                    .await
                    .map_err(|err| eyre!("thread create {err:?}"))?;
                Self::update_thread(ctx, &[msg.id.to_string()], &thread).await?;
                thread
            }
        };

        self.client
            .assistants()
            .messages()
            .create(
                &thread.id,
                CreateMessageParameters {
                    role: MessageRole::User,
                    content: msg.content_safe(ctx),
                    file_ids: None,
                    metadata,
                },
            )
            .await
            .map_err(|err| eyre!("message create {err:?}"))?;

        let mut run = self
            .client
            .assistants()
            .runs()
            .create(
                &thread.id,
                CreateRunParameters {
                    assistant_id: self.assistant_id.clone(),
                    model: None,
                    instructions: None,
                    tools: None,
                    // TODO: temperature not supported by library
                },
            )
            .await
            .map_err(|err| eyre!("run create {err:?}"))?;

        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            run = self
                .client
                .assistants()
                .runs()
                .retrieve(&thread.id, &run.id)
                .await
                .map_err(|err| eyre!("run create {err:?}"))?;
            match run.status {
                RunStatus::Queued | RunStatus::InProgress => {}
                RunStatus::Cancelling | RunStatus::Cancelled => bail!("OpenAI run cancelled"),
                RunStatus::Failed => bail!("OpenAI run failed"),
                RunStatus::Expired => bail!("OpenAI run timed out"),
                RunStatus::Completed => {
                    break;
                }
                RunStatus::RequiresAction => match run.required_action {
                    Some(action) => {
                        assert_eq!(action.r#type, "submit_tool_outputs");

                        let tool_outputs =
                            join_all(action.submit_tool_outputs.tool_calls.into_iter().map(
                                |call| async move {
                                    ToolOutput {
                                        tool_call_id: call.id,
                                        output: Some(
                                            match functions::call(ctx, msg, &call.function).await {
                                                Ok(result) => result,
                                                Err(err) => {
                                                    log::warn!("OpenAI tool failed: {err:?}");
                                                    err.to_string()
                                                }
                                            },
                                        ),
                                    }
                                },
                            ))
                            .await;

                        run = self
                            .client
                            .assistants()
                            .runs()
                            .submit_tool_outputs(
                                &thread.id,
                                &run.id,
                                ToolOutputsParameters { tool_outputs },
                            )
                            .await
                            .map_err(|err| eyre!("run create {err:?}"))?;
                    }
                    None => bail!("OpenAI requires an action, but didn't include any in response"),
                },
            }
        }

        Self::update_stats(ctx, msg, &run).await?;

        // TODO library doesn't support `run_id` parametert
        let response = self
            .client
            .get_with_query(
                &format!("/threads/{}/messages", thread.id),
                &RunIdQuery { run_id: &run.id },
            )
            .await
            .map_err(|err| eyre!("response list {err:?}"))?;
        let messages: ListMessagesResponse =
            format_response(response).map_err(|err| eyre!("response list format {err:?}"))?;

        Ok(messages
            .data
            .into_iter()
            .flat_map(|message| message.content)
            .collect_vec())
    }

    async fn resolve_and_update_thread(
        ctx: &Context,
        mut message: Message,
    ) -> Result<Option<Thread>> {
        let collection = get_data::<DbKey>(ctx)
            .await?
            .collection::<Thread>(THREADS_COLLECTION_NAME);

        let mut message_ids = Vec::new();
        loop {
            message_ids.push(message.id.to_string());

            if let Some(thread) = collection
                .find_one(
                    doc! {
                        "message_ids": {
                            "$in": &message_ids
                        }
                    },
                    None,
                )
                .await?
            {
                Self::update_thread(ctx, &message_ids, &thread).await?;
                return Ok(Some(thread));
            }

            if let Some((channel_id, message_id)) = message
                .message_reference
                .as_ref()
                .and_then(|r| r.message_id.map(|id| (r.channel_id, id)))
            {
                if let Some(referenced) = ctx.cache.message(channel_id, message_id) {
                    message = referenced.clone();
                    continue;
                }
                if let Ok(referenced) = ctx.http.get_message(channel_id, message_id).await {
                    message = referenced;
                    continue;
                }
                return Ok(None);
            }
        }
    }

    async fn update_thread(ctx: &Context, message_ids: &[String], thread: &Thread) -> Result<()> {
        let collection = get_data::<DbKey>(ctx)
            .await?
            .collection::<Thread>(THREADS_COLLECTION_NAME);

        collection
            .update_one(
                doc! {
                    "id": &thread.id,
                },
                doc! {
                    "$addToSet": {
                        "message_ids": message_ids
                    }
                },
                None,
            )
            .await?;

        Ok(())
    }

    async fn update_stats(ctx: &Context, message: &Message, run: &Run) -> Result<()> {
        let collection = get_data::<DbKey>(ctx)
            .await?
            .collection::<UserLog>(USER_LOG_COLLECTION_NAME);

        let usage = run.usage.as_ref().unwrap_or(&Usage {
            prompt_tokens: 0,
            completion_tokens: None,
            total_tokens: 0,
        });

        collection
            .insert_one(
                UserLog {
                    time: Utc::now(),
                    user_id: message.author.id.to_string(),
                    model: run.model.clone(),
                    prompt_tokens: i64::value_from(usage.prompt_tokens).unwrap_or_saturate(),
                    completion_tokens: i64::value_from(usage.completion_tokens.unwrap_or_default())
                        .unwrap_or_saturate(),
                    total_tokens: i64::value_from(usage.total_tokens).unwrap_or_saturate(),
                },
                None,
            )
            .await?;

        Ok(())
    }
}
