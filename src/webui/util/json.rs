use log::error;
use rocket::{
    http::Status,
    request::Request,
    response::{self, Responder},
};
use serde::Serialize;
use serde_json::Value;

// https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Number/MAX_SAFE_INTEGER#description
const JAVASCRIPT_MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;
// https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Number/MIN_SAFE_INTEGER#description
const JAVASCRIPT_MIN_SAFE_INTEGER: i64 = -9_007_199_254_740_991;

fn stringify_u64(value: Value) -> Value {
    match value {
        Value::Number(num) => {
            if num
                .as_u64()
                .map_or(false, |n| n > JAVASCRIPT_MAX_SAFE_INTEGER)
                || num
                    .as_i64()
                    .map_or(false, |n| n < JAVASCRIPT_MIN_SAFE_INTEGER)
            {
                Value::String(num.to_string())
            } else {
                Value::Number(num)
            }
        }
        Value::Array(vec) => Value::Array(vec.into_iter().map(stringify_u64).collect()),
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(key, val)| (key, stringify_u64(val)))
                .collect(),
        ),
        val => val,
    }
}

/// Serializes numeric values outside the range of `[Number.MIN_SAFE_INTEGER, Number.MAX_SAFE_INTEGER]` as strings.
pub fn to_safe_string<T>(input: T) -> serde_json::Result<String>
where
    T: Serialize,
{
    let value = serde_json::to_value(input)?;
    let value = stringify_u64(value);
    serde_json::to_string(&value)
}

/// Responder for JSON data.
/// Serializes numeric values outside the range of `[Number.MIN_SAFE_INTEGER, Number.MAX_SAFE_INTEGER]` as strings.
pub struct Json<T>(pub T);

impl<'r, T> Responder<'r, 'static> for Json<T>
where
    T: Serialize,
{
    fn respond_to(self, request: &'r Request<'_>) -> response::Result<'static> {
        let string = to_safe_string(self.0).map_err(|err| {
            error!("JSON serialization failed: {:#?}", err);
            Status::InternalServerError
        })?;
        response::content::Json(string).respond_to(request)
    }
}
