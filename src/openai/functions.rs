use schemars::{gen::SchemaSettings, schema::RootSchema, JsonSchema};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct OpenAiFunction {
    name: &'static str,
    description: &'static str,
    parameters: RootSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiFunctionCall {
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct GibParameters {
    #[schemars()]
    keywords: Vec<String>,
}

pub fn all() -> Vec<OpenAiFunction> {
    let generator = SchemaSettings::openapi3()
        .with(|s| {
            s.inline_subschemas = true;
        })
        .into_generator();
    vec![OpenAiFunction {
        name: "derpibooru_search",
        description: "Searches for an image on Derpibooru",
        parameters: generator.into_root_schema_for::<GibParameters>(),
    }]
}

pub fn call(call: &OpenAiFunctionCall) -> Result<String, String> {
    match call.name.as_str() {
        "derpibooru_search" => {
            let params: GibParameters = serde_json::from_value(call.arguments.clone())
                .map_err(|_| "Invalid arguments".to_owned())?;
            todo!()
        }
        _ => Err("No such function".to_owned()),
    }
}
