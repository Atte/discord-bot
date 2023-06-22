use crate::{
    discord::commands::{derpibooru_embed, derpibooru_search},
    Result,
};
use chrono::Utc;
use chrono_tz::Tz;
use color_eyre::eyre::{bail, eyre};
use itertools::Itertools;
use schemars::{gen::SchemaSettings, schema::SchemaObject, JsonSchema};
use serde::{Deserialize, Serialize};
use serenity::{http::CacheHttp, model::prelude::Message, prelude::Context};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FunctionName {
    GetTime,
    GetDate,
    GetEvents,
    ShowImage,
}

impl From<&FunctionName> for String {
    fn from(value: &FunctionName) -> Self {
        serde_json::to_value(value).unwrap().to_string()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Function {
    name: FunctionName,
    description: &'static str,
    parameters: SchemaObject,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: FunctionName,
    pub arguments: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct TimeParameters {
    #[schemars(description = "Timezone to return the current time for.")]
    timezone: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct DateParameters {
    #[schemars(description = "Timezone to return the current date for.")]
    timezone: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct EventsParameters {
    #[schemars(description = "Timezone to return the event times in.")]
    timezone: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct GibParameters {
    #[schemars(
        description = "List of keywords to search for. Should be as short as possible, but not empty."
    )]
    keywords: Vec<String>,
}

fn parameters<T>() -> Result<SchemaObject>
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
        bail!("Generated schema contains definitions");
    }
    Ok(schema.schema)
}

pub fn all() -> Result<Vec<Function>> {
    Ok(vec![
        Function {
            name: FunctionName::GetTime,
            description: "Get the time in a given timezone.",
            parameters: parameters::<TimeParameters>()?,
        },
        Function {
            name: FunctionName::GetDate,
            description: "Get the date in a given timezone.",
            parameters: parameters::<DateParameters>()?,
        },
        Function {
            name: FunctionName::GetEvents,
            description: "Get a list of upcoming events.",
            parameters: parameters::<EventsParameters>()?,
        },
        Function {
            name: FunctionName::ShowImage,
            description: "Search for an image on Derpibooru. If one is found, show it to the user.",
            parameters: parameters::<GibParameters>()?,
        },
    ])
}

pub async fn call(ctx: &Context, msg: &Message, call: &FunctionCall) -> Result<String> {
    match call.name {
        FunctionName::GetTime => {
            let params: TimeParameters = serde_json::from_str(&call.arguments)
                .map_err(|_| eyre!("Invalid function arguments"))?;

            let timezone: Tz = params
                .timezone
                .parse()
                .map_err(|_| eyre!("Invalid timezone"))?;
            Ok(Utc::now()
                .with_timezone(&timezone)
                .format("%H:%M:%S")
                .to_string())
        }

        FunctionName::GetDate => {
            let params: DateParameters = serde_json::from_str(&call.arguments)
                .map_err(|_| eyre!("Invalid function arguments"))?;

            let timezone: Tz = params
                .timezone
                .parse()
                .map_err(|_| eyre!("Invalid timezone"))?;
            Ok(Utc::now()
                .with_timezone(&timezone)
                .format("%A %Y-%m-%d")
                .to_string())
        }

        FunctionName::GetEvents => {
            let params: EventsParameters = serde_json::from_str(&call.arguments)
                .map_err(|_| eyre!("Invalid function arguments"))?;
            let Some(guild_id) = msg.guild_id else {
                bail!("Function not available in current context");
            };

            let timezone: Tz = params
                .timezone
                .parse()
                .map_err(|_| eyre!("Invalid timezone"))?;

            let events = guild_id
                .scheduled_events(ctx.http(), false)
                .await
                .map_err(|_| eyre!("Failed to fetch events"))?;
            if events.is_empty() {
                bail!("No events found");
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
            let params: GibParameters = serde_json::from_str(&call.arguments)
                .map_err(|_| eyre!("Invalid function arguments"))?;
            if params.keywords.is_empty() {
                bail!("No keywords provided");
            }

            if let Some((image, total)) = derpibooru_search(ctx, &params.keywords.join(","))
                .await
                .map_err(|err| eyre!(err))?
            {
                derpibooru_embed(ctx, msg, image, total)
                    .await
                    .map_err(|err| eyre!(err))?;
                Ok("Image shown to user".to_owned())
            } else {
                Ok("No images found".to_owned())
            }
        }
    }
}
