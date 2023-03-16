use crate::config::OpenAiConfig;
use chrono::Utc;
use color_eyre::{
    eyre::{bail, eyre, Result},
    Section, SectionExt,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serenity::prelude::TypeMapKey;
use std::sync::Arc;

const MAX_TOKENS: usize = 4096;

#[derive(Debug)]
pub struct OpenAiKey;

impl TypeMapKey for OpenAiKey {
    type Value = Arc<OpenAi>;
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenAiRequest {
    model: OpenAiModel,
    messages: Vec<OpenAiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    user: Option<String>,
}

impl OpenAiRequest {
    pub fn new(user: Option<impl Into<String>>) -> Self {
        OpenAiRequest {
            model: OpenAiModel::Gpt35Turbo,
            messages: Vec::new(),
            temperature: None,
            user: user.map(Into::into),
        }
    }

    fn unshift_message(&mut self, message: OpenAiMessage) {
        self.messages.insert(0, message);
    }

    pub fn try_unshift_message(&mut self, message: OpenAiMessage) -> Result<()> {
        let words_sofar: usize = self
            .messages
            .iter()
            .map(|msg| msg.content.split_whitespace().count())
            .sum();
        let new_words = message.content.split_whitespace().count();

        if (words_sofar + new_words) * 4 / 3 > MAX_TOKENS / 2 {
            bail!("too many tokens");
        }

        self.messages.insert(0, message);
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
enum OpenAiModel {
    #[serde(rename = "gpt-3.5-turbo")]
    Gpt35Turbo,
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
    content: String,
}

impl OpenAiMessage {
    pub fn new(role: OpenAiMessageRole, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OpenAiMessageRole {
    System,
    User,
    Assistant,
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
            client: reqwest::Client::new(),
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

    pub async fn chat(
        &self,
        mut request: OpenAiRequest,
        botname: impl AsRef<str>,
    ) -> Result<String> {
        for message in &mut request.messages {
            let replacements = match message.role {
                OpenAiMessageRole::User => &self.user_replacements,
                OpenAiMessageRole::Assistant => &self.bot_replacements,
                OpenAiMessageRole::System => continue,
            };

            for (from, to) in replacements {
                message.content = from.replace_all(&message.content, to).to_string();
            }
        }

        request.temperature = request.temperature.or(self.temperature);

        for (user, bot) in self.examples.iter().rev() {
            request.unshift_message(OpenAiMessage {
                role: OpenAiMessageRole::Assistant,
                content: bot.clone(),
            });
            request.unshift_message(OpenAiMessage {
                role: OpenAiMessageRole::User,
                content: user.clone(),
            });
        }
        request.unshift_message(OpenAiMessage {
            role: OpenAiMessageRole::System,
            content: self
                .prompt
                .replace("{botname}", botname.as_ref())
                .replace("{date}", &Utc::now().format("%A, %B %d, %Y").to_string())
                .replace("{time}", &Utc::now().format("%I:%M %p").to_string()),
        });

        log::debug!("OpenAI request: {:?}", request);

        let response = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .bearer_auth(&self.api_key)
            .json(&request)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        let response: OpenAiResponse = serde_json::from_str(&response)
            .map_err(|err| eyre!(err).with_section(|| response.header("Response:")))?;

        if let Some(choice) = response.choices.get(0) {
            log::debug!("OpenAI response: {}", choice.message.content);
            Ok(choice.message.content.clone())
        } else {
            bail!("No content in OpenAI response")
        }
    }
}
