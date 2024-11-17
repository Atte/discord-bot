use crate::Result;
use async_openai::types::{AssistantTools, AssistantToolsFunction, FunctionCall, FunctionObject};
use chrono::{Datelike, Utc, Weekday};
use serde_json::{json, Value};
use serenity::async_trait;

pub fn get_specs() -> Vec<AssistantTools> {
    get_tools()
        .into_iter()
        .map(|tool| {
            AssistantTools::Function(AssistantToolsFunction {
                function: FunctionObject {
                    name: tool.get_name().to_owned(),
                    description: Some(tool.get_description().to_owned()),
                    parameters: Some(tool.get_parameters()),
                    strict: Some(true),
                },
            })
        })
        .collect()
}

pub async fn run(call: FunctionCall) -> String {
    let Ok(args) = serde_json::from_str::<Value>(&call.arguments) else {
        return "Invalid arguments!".to_owned();
    };

    for tool in get_tools() {
        if tool.get_name() == call.name {
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
