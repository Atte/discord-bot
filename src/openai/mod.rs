use crate::{
    config::OpenAiConfig,
    discord::{get_data, DbKey},
};
use chrono::{DateTime, Datelike, Timelike, Utc};
use color_eyre::eyre::{bail, Result};
use conv::{UnwrapOrSaturate, ValueFrom};
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serenity::{
    model::prelude::Message,
    prelude::{Context, TypeMapKey},
};
use std::{sync::Arc, time::Duration};

#[cfg(feature = "openai-functions")]
mod functions;
#[cfg(feature = "openai-functions")]
use self::functions::{Function, FunctionCall, FunctionCallType};

#[cfg(not(feature = "openai-vision"))]
const MODEL: OpenAiModel = OpenAiModel::Gpt4Turbo;
#[cfg(feature = "openai-vision")]
const MODEL: OpenAiModel = OpenAiModel::Gpt4Vision;
const MAX_TOKENS: usize = 1024 * 8;
const MAX_RESULT_TOKENS: usize = 1024 * 4;

lazy_static! {
    static ref CLEANUP_REGEX: Regex = Regex::new(r"\bhttps?:\/\/\S+").unwrap();
}

const USER_LOG_COLLECTION_NAME: &str = "openai-user-log";

#[derive(Debug, Clone, Serialize)]
struct UserLog {
    time: DateTime<Utc>,
    user_id: String,
    model: String,
    prompt_tokens: i64,
    completion_tokens: i64,
    total_tokens: i64,
    #[cfg(feature = "openai-functions")]
    function_call: Option<FunctionCall>,
}

#[derive(Debug)]
pub struct OpenAiKey;

impl TypeMapKey for OpenAiKey {
    type Value = Arc<OpenAi>;
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenAiRequest {
    model: OpenAiModel,
    messages: Vec<OpenAiMessage>,
    #[cfg(feature = "openai-functions")]
    function_call: FunctionCallType,
    #[cfg(feature = "openai-functions")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    functions: Vec<Function>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    user: Option<String>,
    max_tokens: usize,
}

impl OpenAiRequest {
    pub fn new(user: Option<impl Into<String>>) -> Self {
        #[cfg(feature = "openai-functions")]
        let functions = match functions::all() {
            Ok(funs) => funs,
            Err(err) => {
                log::error!("Unable to define OpenAI functions: {:?}", err);
                Vec::new()
            }
        };
        OpenAiRequest {
            model: MODEL,
            messages: Vec::new(),
            #[cfg(feature = "openai-functions")]
            function_call: if functions.is_empty() {
                FunctionCallType::None
            } else {
                FunctionCallType::Auto
            },
            #[cfg(feature = "openai-functions")]
            functions,
            temperature: None,
            user: user.map(Into::into),
            max_tokens: MAX_RESULT_TOKENS,
        }
    }

    fn shift_message(&mut self) -> Option<OpenAiMessage> {
        if self.messages.is_empty() {
            None
        } else {
            Some(self.messages.remove(0))
        }
    }

    #[inline]
    fn push_message(&mut self, message: OpenAiMessage) {
        self.messages.push(message);
    }

    #[inline]
    fn unshift_message(&mut self, message: OpenAiMessage) {
        self.messages.insert(0, message);
    }

    pub fn try_unshift_message(&mut self, message: OpenAiMessage) -> Result<()> {
        self.unshift_message(message);

        if self.approximate_num_tokens() > MAX_TOKENS / 2 {
            self.shift_message();
            bail!("too many tokens");
        }

        Ok(())
    }

    fn approximate_num_tokens(&self) -> usize {
        let words: usize = self
            .messages
            .iter()
            .filter_map(|msg| {
                msg.content()
                    .map(|content| content.split_whitespace().count())
            })
            .sum();
        words * 4 / 3
    }

    #[cfg(feature = "openai-vision")]
    pub fn expand_vision(&mut self) {
        let mut finder = linkify::LinkFinder::new();
        finder.kinds(&[linkify::LinkKind::Url]);

        for msg in &mut self.messages {
            if let OpenAiMessage::User { content } = msg {
                let urls = if let Some(OpenAiUserMessage::Text { text }) = content.first_mut() {
                    let urls: Vec<_> = finder
                        .links(text)
                        .map(|link| link.as_str().to_owned())
                        .collect();
                    for url in &urls {
                        *text = text.replace(url.as_str(), " ");
                    }
                    urls
                } else {
                    continue;
                };

                for url in urls {
                    if url.ends_with(".png")
                        || url.ends_with(".jpg")
                        || url.ends_with(".jpeg")
                        || url.ends_with(".webp")
                        || url.ends_with(".gif")
                    {
                        content.push(OpenAiUserMessage::ImageUrl {
                            image_url: OpenAiImageUrl {
                                url,
                                detail: OpenAiImageDetail::Low,
                            },
                        });
                    }
                }
            }
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
enum OpenAiModel {
    #[serde(rename = "gpt-3.5-turbo")]
    Gpt35Turbo,
    #[serde(rename = "gpt-3.5-turbo-16k")]
    Gpt35Turbo16k,
    #[serde(rename = "gpt-4")]
    Gpt4,
    #[serde(rename = "gpt-4-32k")]
    Gpt432k,
    #[serde(rename = "gpt-4-1106-preview")]
    Gpt4Turbo,
    #[serde(rename = "gpt-4-vision-preview")]
    Gpt4Vision,
}

#[derive(Debug, Clone, Deserialize)]
struct OpenAiResponse {
    model: String,
    choices: Vec<OpenAiResponseChoice>,
    usage: OpenAiResponseUsage,
}

#[derive(Debug, Clone, Deserialize)]
struct OpenAiResponseChoice {
    message: OpenAiMessage,
}

#[derive(Debug, Clone, Deserialize)]
struct OpenAiResponseUsage {
    prompt_tokens: usize,
    completion_tokens: usize,
    total_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiImageUrl {
    url: String,
    detail: OpenAiImageDetail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpenAiImageDetail {
    Low,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OpenAiUserMessage {
    Text {
        text: String,
    },
    #[cfg(feature = "openai-vision")]
    ImageUrl {
        image_url: OpenAiImageUrl,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "role", rename_all = "snake_case")]
pub enum OpenAiMessage {
    System {
        content: String,
    },
    User {
        content: Vec<OpenAiUserMessage>,
    },
    Assistant {
        content: Option<String>,
        #[cfg(feature = "openai-functions")]
        function_call: Option<FunctionCall>,
    },
    Function {
        content: String,
        name: String,
    },
}

impl OpenAiMessage {
    #[inline]
    pub fn content(&self) -> Option<&String> {
        match self {
            OpenAiMessage::System { content } => Some(content),
            OpenAiMessage::User { content } => content
                .iter()
                .filter_map(|msg| match msg {
                    OpenAiUserMessage::Text { text } => Some(text),
                    #[cfg(feature = "openai-vision")]
                    OpenAiUserMessage::ImageUrl { .. } => None,
                })
                .next(),
            OpenAiMessage::Assistant { content, .. } => content.as_ref(),
            OpenAiMessage::Function { content, .. } => Some(content),
        }
    }

    #[inline]
    pub fn content_mut(&mut self) -> Option<&mut String> {
        match self {
            OpenAiMessage::System { content } => Some(content),
            OpenAiMessage::User { content } => content
                .iter_mut()
                .filter_map(|msg| match msg {
                    OpenAiUserMessage::Text { text } => Some(text),
                    #[cfg(feature = "openai-vision")]
                    OpenAiUserMessage::ImageUrl { .. } => None,
                })
                .next(),
            OpenAiMessage::Assistant { content, .. } => content.as_mut(),
            OpenAiMessage::Function { content, .. } => Some(content),
        }
    }
}

pub struct OpenAi {
    client: reqwest::Client,
    api_key: String,
    temperature: Option<f32>,
    prompt: String,
    examples: Vec<(String, String)>,
    bot_replacements: Vec<(Regex, String)>,
    user_replacements: Vec<(Regex, String)>,
}

fn parse_replacements(
    config: impl Iterator<Item = (impl AsRef<str>, impl Into<String>)>,
) -> Vec<(Regex, String)> {
    config
        .filter_map(|(key, value)| match Regex::new(key.as_ref()) {
            Ok(re) => Some((re, value.into())),
            Err(err) => {
                log::error!("Invalid OpenAI replacement regex: {}", err);
                None
            }
        })
        .collect()
}

impl OpenAi {
    #[inline]
    pub fn new(config: &OpenAiConfig) -> Self {
        Self {
            client: reqwest::ClientBuilder::new()
                .timeout(Duration::from_secs(55))
                .build()
                .expect("invalid static reqwest client config"),
            api_key: config.api_key.to_string(),
            temperature: config.temperature,
            prompt: config.prompt.to_string().trim().to_owned(),
            examples: config
                .examples
                .iter()
                .map(|(user, bot)| (user.to_string(), bot.to_string()))
                .collect(),
            bot_replacements: parse_replacements(config.bot_replacements.iter()),
            user_replacements: parse_replacements(config.user_replacements.iter()),
        }
    }

    async fn request(&self, request: &OpenAiRequest) -> Result<OpenAiResponse> {
        log::debug!("OpenAI request: {}", serde_json::to_string_pretty(request)?);

        let response = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .bearer_auth(&self.api_key)
            .json(request)
            .send()
            .await?;

        match response.error_for_status_ref() {
            Ok(_) => {
                let text = response.text().await?;
                log::debug!("OpenAI response: {}", text);
                Ok(serde_json::from_str(&text)?)
            }
            Err(err) => {
                if let Ok(text) = response.text().await {
                    log::debug!("OpenAI error response: {}", text);
                }
                Err(err.into())
            }
        }
    }

    pub async fn chat(
        &self,
        ctx: &Context,
        msg: &Message,
        mut request: OpenAiRequest,
        botname: impl AsRef<str>,
    ) -> Result<String> {
        for message in &mut request.messages {
            let replacements = match message {
                OpenAiMessage::User { .. } => &self.user_replacements,
                OpenAiMessage::Assistant { .. } => &self.bot_replacements,
                OpenAiMessage::System { .. } | OpenAiMessage::Function { .. } => continue,
            };

            for (from, to) in replacements {
                if let Some(content) = message.content_mut() {
                    *content = from.replace_all(content, to).to_string();
                }
            }
        }

        request.temperature = request.temperature.or(self.temperature);

        for (user, bot) in self.examples.iter().rev() {
            request.unshift_message(OpenAiMessage::Assistant {
                content: Some(bot.clone()),
                #[cfg(feature = "openai-functions")]
                function_call: None,
            });
            request.unshift_message(OpenAiMessage::User {
                content: vec![OpenAiUserMessage::Text { text: user.clone() }],
            });
        }
        request.unshift_message(OpenAiMessage::System {
            content: self
                .prompt
                .replace("{botname}", botname.as_ref())
                .replace("{date}", &Utc::now().format("%A, %B %d, %Y").to_string())
                .replace("{time}", &Utc::now().format("%I:%M %p").to_string())
                .replace(
                    "{is_weekend}",
                    if Utc::now().weekday().number_from_monday() >= 6
                        || (Utc::now().weekday().number_from_monday() == 5
                            && Utc::now().hour() >= 16)
                    {
                        "is"
                    } else {
                        "is not"
                    },
                ),
        });

        request.model = MODEL;

        #[cfg(feature = "openai-vision")]
        request.expand_vision();

        let response = self.request(&request).await?;
        Self::update_stats(
            ctx,
            msg,
            &response,
            #[cfg(feature = "openai-functions")]
            None,
        )
        .await?;

        #[cfg(feature = "openai-functions")]
        let response = if let Some(call) = response.choices.get(0).and_then(|choice| match &choice
            .message
        {
            OpenAiMessage::Assistant { function_call, .. } => function_call.as_ref(),
            _ => None,
        }) {
            request.push_message(response.choices.get(0).unwrap().message.clone());
            request.push_message(OpenAiMessage::Function {
                name: (&call.name).into(),
                content: functions::call(ctx, msg, &call)
                    .await
                    .unwrap_or_else(|err| err.to_string()),
            });

            request.function_call = FunctionCallType::None;
            request.model = MODEL;

            let response = self.request(&request).await?;
            Self::update_stats(ctx, msg, &response, Some(call.clone())).await?;
            response
        } else {
            response
        };

        if let Some(content) = response
            .choices
            .get(0)
            .and_then(|choice| choice.message.content())
        {
            Ok(CLEANUP_REGEX.replace_all(content, "").to_string())
        } else {
            bail!("No content in OpenAI response")
        }
    }

    async fn update_stats(
        ctx: &Context,
        message: &Message,
        response: &OpenAiResponse,
        #[cfg(feature = "openai-functions")] function_call: Option<FunctionCall>,
    ) -> Result<()> {
        let collection = get_data::<DbKey>(ctx)
            .await?
            .collection::<UserLog>(USER_LOG_COLLECTION_NAME);

        collection
            .insert_one(
                UserLog {
                    time: Utc::now(),
                    user_id: message.author.id.to_string(),
                    model: response.model.clone(),
                    prompt_tokens: i64::value_from(response.usage.prompt_tokens)
                        .unwrap_or_saturate(),
                    completion_tokens: i64::value_from(response.usage.completion_tokens)
                        .unwrap_or_saturate(),
                    total_tokens: i64::value_from(response.usage.total_tokens).unwrap_or_saturate(),
                    #[cfg(feature = "openai-functions")]
                    function_call,
                },
                None,
            )
            .await?;

        Ok(())
    }
}
