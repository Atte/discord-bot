use crate::config::OpenAiConfig;
use chrono::Utc;
use color_eyre::eyre::{bail, Result};
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
use self::functions::{OpenAiFunction, OpenAiFunctionCall};

const MODEL_SMALL: OpenAiModel = OpenAiModel::Gpt35Turbo0613;
const MAX_TOKENS_SMALL: usize = 4_000;

const MODEL_LARGE: OpenAiModel = OpenAiModel::Gpt35Turbo16k0613;
const MAX_TOKENS_LARGE: usize = 16_000;

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
    #[serde(skip_serializing_if = "Vec::is_empty")]
    functions: Vec<OpenAiFunction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    user: Option<String>,
}

impl OpenAiRequest {
    pub fn new(user: Option<impl Into<String>>) -> Self {
        OpenAiRequest {
            model: MODEL_LARGE,
            messages: Vec::new(),
            #[cfg(feature = "openai-functions")]
            functions: match functions::all() {
                Ok(funs) => funs,
                Err(err) => {
                    log::error!("Unable to define OpenAI functions: {:?}", err);
                    Vec::new()
                }
            },
            temperature: None,
            user: user.map(Into::into),
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

        if self.approximate_num_tokens() > MAX_TOKENS_LARGE / 2 {
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
                msg.content
                    .as_ref()
                    .map(|content| content.split_whitespace().count())
            })
            .sum();
        words * 4 / 3
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
enum OpenAiModel {
    #[serde(rename = "gpt-3.5-turbo")]
    Gpt35Turbo,
    #[serde(rename = "gpt-3.5-turbo-16k")]
    Gpt35Turbo16k,
    #[serde(rename = "gpt-3.5-turbo-0301")]
    Gpt35Turbo0301,
    #[serde(rename = "gpt-3.5-turbo-0613")]
    Gpt35Turbo0613,
    #[serde(rename = "gpt-3.5-turbo-16k-0613")]
    Gpt35Turbo16k0613,
}

#[derive(Debug, Clone, Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiResponseChoice>,
    // usage: OpenAiResponseUsage,
}

#[derive(Debug, Clone, Deserialize)]
struct OpenAiResponseChoice {
    message: OpenAiMessage,
}

#[derive(Debug, Clone, Deserialize)]
struct OpenAiResponseUsage {
    // prompt_tokens: usize,
    // completion_tokens: usize,
    // total_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiMessage {
    role: OpenAiMessageRole,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    content: Option<String>,
    #[cfg(feature = "openai-functions")]
    #[serde(skip_serializing_if = "Option::is_none")]
    function_call: Option<OpenAiFunctionCall>,
}

impl OpenAiMessage {
    pub fn new(role: OpenAiMessageRole, content: impl Into<String>) -> Self {
        Self {
            role,
            name: None,
            content: Some(content.into()),
            #[cfg(feature = "openai-functions")]
            function_call: None,
        }
    }

    pub fn new_with_name(
        role: OpenAiMessageRole,
        name: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            role,
            name: Some(name.into()),
            content: Some(content.into()),
            #[cfg(feature = "openai-functions")]
            function_call: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OpenAiMessageRole {
    System,
    User,
    Assistant,
    Function,
}

impl OpenAiMessageRole {
    #[inline]
    pub fn message(&self, content: impl Into<String>) -> OpenAiMessage {
        OpenAiMessage::new(self.clone(), content)
    }

    #[inline]
    pub fn message_with_name(
        &self,
        name: impl Into<String>,
        content: impl Into<String>,
    ) -> OpenAiMessage {
        OpenAiMessage::new_with_name(self.clone(), name, content)
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
                .timeout(Duration::from_secs(30))
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
            let replacements = match message.role {
                OpenAiMessageRole::User => &self.user_replacements,
                OpenAiMessageRole::Assistant => &self.bot_replacements,
                OpenAiMessageRole::System | OpenAiMessageRole::Function => continue,
            };

            for (from, to) in replacements {
                message.content = message
                    .content
                    .as_ref()
                    .map(|content| from.replace_all(content, to).to_string());
            }
        }

        request.temperature = request.temperature.or(self.temperature);

        for (user, bot) in self.examples.iter().rev() {
            request.unshift_message(OpenAiMessageRole::Assistant.message(bot.clone()));
            request.unshift_message(OpenAiMessageRole::User.message(user.clone()));
        }
        request.unshift_message(
            OpenAiMessageRole::System.message(
                self.prompt
                    .replace("{botname}", botname.as_ref())
                    .replace("{date}", &Utc::now().format("%A, %B %d, %Y").to_string())
                    .replace("{time}", &Utc::now().format("%I:%M %p").to_string()),
            ),
        );

        request.model = if request.approximate_num_tokens() > MAX_TOKENS_SMALL / 2 {
            MODEL_LARGE
        } else {
            MODEL_SMALL
        };
        let response = self.request(&request).await?;

        #[cfg(feature = "openai-functions")]
        let response = if let Some(call) = response
            .choices
            .get(0)
            .and_then(|choice| choice.message.function_call.as_ref())
        {
            request.push_message(response.choices.get(0).unwrap().message.clone());
            request.push_message(
                OpenAiMessageRole::Function.message_with_name(
                    &call.name,
                    functions::call(ctx, msg, call)
                        .await
                        .unwrap_or_else(|err| err.to_string()),
                ),
            );

            request.model = if request.approximate_num_tokens() > MAX_TOKENS_SMALL / 2 {
                MODEL_LARGE
            } else {
                MODEL_SMALL
            };
            self.request(&request).await?
        } else {
            response
        };

        if let Some(content) = response
            .choices
            .get(0)
            .and_then(|choice| choice.message.content.as_ref())
        {
            Ok(content.clone())
        } else {
            bail!("No content in OpenAI response")
        }
    }
}
