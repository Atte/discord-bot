use crate::{
    discord::commands::{derpibooru_embed, derpibooru_search},
    Result,
};
use color_eyre::eyre::{bail, eyre};
use schemars::{gen::SchemaSettings, schema::SchemaObject, JsonSchema};
use serde::{Deserialize, Serialize};
use serenity::{model::prelude::Message, prelude::Context};

#[derive(Debug, Clone, Serialize)]
pub struct OpenAiFunction {
    name: &'static str,
    description: &'static str,
    parameters: SchemaObject,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiFunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct GibParameters {
    #[schemars(
        description = "List of keywords to search for. Should be as short as possible, but not empty."
    )]
    keywords: Vec<String>,
}

fn parameters<T>() -> Result<SchemaObject> {
    let generator = SchemaSettings::openapi3()
        .with(|s| {
            s.inline_subschemas = true;
        })
        .into_generator();
    let schema = generator.into_root_schema_for::<GibParameters>();
    if !schema.definitions.is_empty() {
        bail!("Generated schema contains definitions");
    }
    Ok(schema.schema)
}

pub fn all() -> Result<Vec<OpenAiFunction>> {
    Ok(vec![OpenAiFunction {
        name: "show_derpibooru_image",
        description: "Search for an image on Derpibooru. If one is found, show it to the user.",
        parameters: parameters::<GibParameters>()?,
    }])
}

pub async fn call(ctx: &Context, msg: &Message, call: &OpenAiFunctionCall) -> Result<String> {
    match call.name.as_str() {
        "show_derpibooru_image" => {
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
        _ => bail!("Invalid function name"),
    }
}
