use crate::Result;
use async_openai::types::{
    ChatCompletionMessageToolCall, ChatCompletionTool, ChatCompletionToolArgs,
    ChatCompletionToolType, FunctionObjectArgs,
};
use chrono::{Datelike, Utc, Weekday};
use serde_json::{Value, json};
use serenity::async_trait;

pub fn get_specs() -> Result<Vec<ChatCompletionTool>> {
    get_tools()
        .into_iter()
        .map(|tool| {
            Ok(ChatCompletionToolArgs::default()
                .r#type(ChatCompletionToolType::Function)
                .function(
                    FunctionObjectArgs::default()
                        .name(tool.get_name().to_owned())
                        .description(tool.get_description().to_owned())
                        .parameters(tool.get_parameters())
                        .strict(true)
                        .build()?,
                )
                .build()?)
        })
        .collect()
}

pub async fn run(call: &ChatCompletionMessageToolCall) -> String {
    let Ok(args) = serde_json::from_str::<Value>(&call.function.arguments) else {
        return "Invalid arguments!".to_owned();
    };

    for tool in get_tools() {
        if tool.get_name() == call.function.name {
            return tool.run(args).await.unwrap_or_else(|err| err.to_string());
        }
    }

    "No such function!".to_owned()
}

#[inline]
fn get_tools() -> [Box<dyn Tool + Send>; 1] {
    [Box::new(GetDayOfWeek)]
}

#[async_trait]
trait Tool {
    fn get_name(&self) -> &'static str;
    fn get_description(&self) -> &'static str;
    fn get_parameters(&self) -> Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {},
            "required": []
        })
    }

    async fn run(&self, args: Value) -> Result<String>;
}

#[derive(Debug)]
struct GetDayOfWeek;

#[async_trait]
impl Tool for GetDayOfWeek {
    fn get_name(&self) -> &'static str {
        "GetDayOfWeek"
    }

    fn get_description(&self) -> &'static str {
        "Get the current day of the week"
    }

    async fn run(&self, _args: Value) -> Result<String> {
        Ok(match Utc::now().weekday() {
            Weekday::Mon => "Monday",
            Weekday::Tue => "Tuesday",
            Weekday::Wed => "Wednesday",
            Weekday::Thu => "Thursday",
            Weekday::Fri => "Friday",
            Weekday::Sat => "Saturday",
            Weekday::Sun => "Sunday",
        }
        .to_owned())
    }
}
