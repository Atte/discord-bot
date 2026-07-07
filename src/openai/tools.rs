use crate::Result;
#[cfg(feature = "teamup")]
use crate::teamup::TeamupEvent;
use async_openai::types::{
    ChatCompletionMessageToolCall, ChatCompletionTool, ChatCompletionToolArgs,
    ChatCompletionToolType, FunctionObjectArgs,
};
use chrono::{Datelike, Utc, Weekday};
use serde_json::{Value, json};
use serenity::async_trait;

#[derive(Clone)]
pub struct ToolContext {
    #[cfg(feature = "teamup")]
    pub calendar: Vec<TeamupEvent>,
}

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

pub async fn run(call: &ChatCompletionMessageToolCall, context: ToolContext) -> String {
    let Ok(args) = serde_json::from_str::<Value>(&call.function.arguments) else {
        return "Invalid arguments!".to_owned();
    };

    for tool in get_tools() {
        if tool.get_name() == call.function.name {
            log::info!("calling tool: {}", call.function.name);
            return tool
                .run(args, context)
                .await
                .unwrap_or_else(|err| err.to_string());
        }
    }

    "No such function!".to_owned()
}

#[inline]
fn get_tools() -> Box<[Box<dyn Tool + Send>]> {
    Box::new([
        Box::new(GetDayOfWeek),
        Box::new(GetYear),
        Box::new(GetDate),
        // #[cfg(feature = "teamup")]
        // Box::new(GetSchedule),
    ])
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

    async fn run(&self, args: Value, context: ToolContext) -> Result<String>;
}

#[derive(Debug)]
struct GetDayOfWeek;

#[async_trait]
impl Tool for GetDayOfWeek {
    #[inline]
    fn get_name(&self) -> &'static str {
        "GetDayOfWeek"
    }

    #[inline]
    fn get_description(&self) -> &'static str {
        "Get the current day of the week"
    }

    async fn run(&self, _args: Value, _context: ToolContext) -> Result<String> {
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

#[derive(Debug)]
struct GetYear;

#[async_trait]
impl Tool for GetYear {
    #[inline]
    fn get_name(&self) -> &'static str {
        "GetYear"
    }

    #[inline]
    fn get_description(&self) -> &'static str {
        "Get the current year"
    }

    async fn run(&self, _args: Value, _context: ToolContext) -> Result<String> {
        Ok(Utc::now().year().to_string())
    }
}

#[derive(Debug)]
struct GetDate;

#[async_trait]
impl Tool for GetDate {
    #[inline]
    fn get_name(&self) -> &'static str {
        "GetDate"
    }

    #[inline]
    fn get_description(&self) -> &'static str {
        "Get the current full date"
    }

    async fn run(&self, _args: Value, _context: ToolContext) -> Result<String> {
        Ok(Utc::now().date_naive().to_string())
    }
}

#[derive(Debug)]
#[cfg(feature = "teamup")]
struct GetSchedule;

#[async_trait]
#[cfg(feature = "teamup")]
impl Tool for GetSchedule {
    #[inline]
    fn get_name(&self) -> &'static str {
        "GetSchedule"
    }

    #[inline]
    fn get_description(&self) -> &'static str {
        "Get the BerryTube schedule"
    }

    async fn run(&self, _args: Value, context: ToolContext) -> Result<String> {
        use std::fmt::Write;

        let mut output = String::new();
        for event in context.calendar {
            write!(
                &mut output,
                "{} from {} to {}",
                event.title, event.start_dt, event.end_dt
            )?;
            if let Some(notes) = event.notes {
                write!(&mut output, " ({notes})")?;
            }
            writeln!(&mut output)?;
        }
        Ok(output)
    }
}
