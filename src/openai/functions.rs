use crate::{
    discord::commands::{derpibooru_embed, derpibooru_search},
    Result,
};
use chrono::Utc;
use chrono_tz::Tz;
use color_eyre::eyre::{bail, eyre};
use itertools::Itertools;
use openai_dive::v1::resources::{
    assistant::assistant::{AssistantFunction, AssistantFunctionTool, AssistantTools},
    chat::Function,
};
use schemars::{gen::SchemaSettings, JsonSchema};
use serde::Deserialize;
use serenity::{http::CacheHttp, model::prelude::Message, prelude::Context};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, strum::EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum FunctionName {
    GetTime,
    GetDate,
    GetDayOfWeek,
    GetEvents,
    ShowImage,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct GetTimeParameters {
    #[schemars(description = "Timezone to return the current time for.")]
    timezone: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct GetDateParameters {
    #[schemars(description = "Timezone to return the current date for.")]
    timezone: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct GetDayOfWeekParameters {
    #[schemars(description = "Timezone to return the current day of the week for.")]
    timezone: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct GetEventsParameters {
    #[schemars(description = "Timezone to return the event start times in.")]
    timezone: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct ShowImageParameters {
    #[schemars(
        description = "List of keywords to search for. Should be as short as possible, but not empty. The name of an artist should be prefixed with `artist:`."
    )]
    keywords: Vec<String>,
}

fn parameters<T>() -> Result<serde_json::Value>
where
    T: JsonSchema,
{
    let generator = SchemaSettings::openapi3()
        .with(|s| {
            s.inline_subschemas = true;
        })
        .into_generator();
    let schema = generator.into_root_schema_for::<T>();
    if !schema.definitions.is_empty() {
        bail!("Generated schema contains definitions.");
    }
    Ok(serde_json::to_value(schema.schema)?)
}

pub fn as_tools() -> Result<Vec<AssistantTools>> {
    let functions = [
        AssistantFunction {
            name: FunctionName::GetTime.to_string(),
            description: Some(String::from("Get the current time.")),
            parameters: parameters::<GetTimeParameters>()?,
        },
        AssistantFunction {
            name: FunctionName::GetDate.to_string(),
            description: Some(String::from("Get the current date.")),
            parameters: parameters::<GetDateParameters>()?,
        },
        AssistantFunction {
            name: FunctionName::GetDayOfWeek.to_string(),
            description: Some(String::from("Get the current day of the week.")),
            parameters: parameters::<GetDayOfWeekParameters>()?,
        },
        AssistantFunction {
            name: FunctionName::GetEvents.to_string(),
            description: Some(String::from("Get a list of upcoming events.")),
            parameters: parameters::<GetEventsParameters>()?,
        },
        AssistantFunction {
            name: FunctionName::ShowImage.to_string(),
            description: Some(String::from(
                "Search for an image on Derpibooru. If one is found, show it to the user.",
            )),
            parameters: parameters::<ShowImageParameters>()?,
        },
    ];
    Ok(functions
        .iter()
        .cloned()
        .map(|function| {
            AssistantTools::Function(AssistantFunctionTool {
                r#type: String::from("function"),
                function,
            })
        })
        .collect())
}

pub async fn call(ctx: &Context, msg: &Message, call: &Function) -> Result<String> {
    match call.name.parse()? {
        FunctionName::GetTime => {
            let params: GetTimeParameters = serde_json::from_str(&call.arguments)
                .map_err(|_| eyre!("Invalid function arguments."))?;

            let timezone: Tz = params
                .timezone
                .parse()
                .map_err(|_| eyre!("Invalid timezone."))?;
            Ok(Utc::now()
                .with_timezone(&timezone)
                .format("%H:%M:%S")
                .to_string())
        }

        FunctionName::GetDate => {
            let params: GetDateParameters = serde_json::from_str(&call.arguments)
                .map_err(|_| eyre!("Invalid function arguments."))?;

            let timezone: Tz = params
                .timezone
                .parse()
                .map_err(|_| eyre!("Invalid timezone."))?;
            Ok(Utc::now()
                .with_timezone(&timezone)
                .format("%A %Y-%m-%d")
                .to_string())
        }

        FunctionName::GetDayOfWeek => {
            let params: GetDayOfWeekParameters = serde_json::from_str(&call.arguments)
                .map_err(|_| eyre!("Invalid function arguments."))?;

            let timezone: Tz = params
                .timezone
                .parse()
                .map_err(|_| eyre!("Invalid timezone."))?;
            Ok(Utc::now().with_timezone(&timezone).format("%A").to_string())
        }

        FunctionName::GetEvents => {
            let params: GetEventsParameters = serde_json::from_str(&call.arguments)
                .map_err(|_| eyre!("Invalid function arguments."))?;
            let Some(guild_id) = msg.guild_id else {
                bail!("Function not available in current context.");
            };

            let timezone: Tz = params
                .timezone
                .parse()
                .map_err(|_| eyre!("Invalid timezone."))?;

            let events = guild_id
                .scheduled_events(ctx.http(), false)
                .await
                .map_err(|_| eyre!("Failed to fetch events."))?;
            if events.is_empty() {
                bail!("No events found.");
            }

            Ok(events
                .into_iter()
                .map(|event| {
                    format!(
                        "{}: {}",
                        event
                            .start_time
                            .with_timezone(&timezone)
                            .format("%A %Y-%m-%d %H:%M"),
                        event.name
                    )
                })
                .join("\n"))
        }

        FunctionName::ShowImage => {
            let params: ShowImageParameters = serde_json::from_str(&call.arguments)
                .map_err(|_| eyre!("Invalid function arguments."))?;
            if params.keywords.is_empty() {
                bail!("No keywords provided.");
            }

            if let Some((image, total)) = derpibooru_search(ctx, &params.keywords.join(","))
                .await
                .map_err(|err| eyre!(err))?
            {
                derpibooru_embed(ctx, msg, &image, total)
                    .await
                    .map_err(|err| eyre!(err))?;
                Ok(format!(
                    "Image found and shown to the user. It matches the following keywords: {}",
                    image.tags.join(", ")
                ))
            } else {
                Ok("No images found.".to_owned())
            }
        }
    }
}
