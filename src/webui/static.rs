use super::{
    graphql::{Context, Schema},
    util::HeaderResponder,
};
use lazy_static::lazy_static;
use regex::{Captures, Regex};
use rocket::{
    get,
    http::{ContentType, Header, Status},
    routes, Build, Rocket, State,
};
use std::{
    borrow::Cow,
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
};

// defines const WEBUI_FILES
include!(concat!(env!("OUT_DIR"), "/webui.rs"));

pub fn init(vega: Rocket<Build>) -> Rocket<Build> {
    #[allow(clippy::no_effect_underscore_binding)] // within `routes!`
    vega.mount("/", routes![index, path, robots])
}

fn serve(path: &str) -> Option<(ContentType, Vec<u8>)> {
    let file = if env::var_os("WEBUI_PASSTHROUGH").is_some() {
        let base = Path::new("./webui/dist/").canonicalize().ok()?;
        let full = base.join(path).canonicalize().ok()?;
        if !full.starts_with(base) {
            return None;
        }
        fs::read(full).ok()?
    } else {
        WEBUI_FILES
            .get(&format!("{}/{}", env!("OUT_DIR"), path))
            .ok()
            .map(Cow::into_owned)?
    };
    let ext = Path::new(&path)
        .extension()
        .and_then(std::ffi::OsStr::to_str)?;
    Some((
        if ext == "map" {
            ContentType::JSON
        } else {
            ContentType::from_extension(ext)?
        },
        file,
    ))
}

#[get("/robots.txt")]
pub fn robots() -> &'static str {
    "User-agent: *\nDisallow: /\n"
}

#[allow(clippy::needless_pass_by_value)]
#[get("/static/<path..>")]
pub fn path(path: PathBuf) -> Option<HeaderResponder<(ContentType, Vec<u8>)>> {
    path.to_str().and_then(serve).map(|inner| {
        HeaderResponder::from(inner)
            // 1 year
            .set_header(Header::new("Cache-Control", "public, max-age=31536000"))
    })
}

#[get("/")]
pub async fn index(
    schema: &State<Schema>,
    context: Context,
) -> Result<HeaderResponder<(ContentType, Vec<u8>)>, (Status, &'static str)> {
    let (bot, errors) = juniper::execute(
        "
        query GetBot {
            bot {
                id
                name
                discriminator
                tag
                avatar
            }
        }
        ",
        Some("GetBot"),
        &*schema,
        &HashMap::new(),
        &context,
    )
    .await
    .map_err(|_| (Status::InternalServerError, "unable to get bot user"))?;

    if !errors.is_empty() {
        return Err((Status::InternalServerError, "error getting bot user"));
    }

    let bot = bot.into_object().unwrap();
    let bot = bot
        .get_field_value("bot")
        .unwrap()
        .as_object_value()
        .unwrap();

    lazy_static! {
        static ref RE: Regex = Regex::new(r"\(BOT_([A-Z]+)\)").unwrap();
    }

    let (mime, source) = serve("index.html").ok_or((Status::NotFound, "no index.html"))?;
    let source = String::from_utf8(source).expect("invalid UTF-8 in index.html");
    let source = RE.replace_all(&source, |caps: &Captures<'_>| {
        if &caps[1] == "JSON" {
            format!(
                r#"<script type="application/x-bot-user+json">{}</script>"#,
                serde_json::to_string(&bot).unwrap()
            )
        } else if let Some(field) = bot.get_field_value(caps[1].to_lowercase()) {
            field
                .as_scalar()
                .expect("non-scalar (BOT_PLACEHOLDER)")
                .to_string()
        } else {
            caps[0].to_string()
        }
    });

    Ok(
        HeaderResponder::from((mime, source.into_owned().into_bytes())).set_header(Header::new(
            "Link",
            r#"<https://cdn.discordapp.com>; rel="preconnect"; crossorigin="anonymous""#,
        )),
    )
}
