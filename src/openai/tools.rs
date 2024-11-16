use async_openai::types::{AssistantTools, AssistantToolsFunction, FunctionCall, FunctionObject};
use chrono::{Datelike, Utc, Weekday};

pub fn get_specs() -> Vec<AssistantTools> {
    let funcs = vec![FunctionObject {
        name: "get_day_of_week".to_owned(),
        description: Some("Get the current day of the week".to_owned()),
        parameters: None,
        strict: None,
    }];
    funcs
        .into_iter()
        .map(|function| AssistantTools::Function(AssistantToolsFunction { function }))
        .collect()
}

pub async fn run(call: FunctionCall) -> String {
    match call.name.as_str() {
        "get_day_of_week" => match Utc::now().weekday() {
            Weekday::Mon => "Monday",
            Weekday::Tue => "Tuesday",
            Weekday::Wed => "Wednesday",
            Weekday::Thu => "Thursday",
            Weekday::Fri => "Friday",
            Weekday::Sat => "Saturday",
            Weekday::Sun => "Sunday",
        }
        .to_owned(),
        _ => "No such function!".to_owned(),
    }
}
