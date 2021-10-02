use super::json::to_safe_string;
use log::error;
use rocket::{get, http::ContentType, routes, Build, Rocket, State};
use serenity::CacheAndHttp;
use static_assertions::const_assert;
use std::sync::Arc;
use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

// defines const WEBUI_FILES
include!(concat!(env!("OUT_DIR"), "/webui.rs"));

type ServeResponse = Option<(ContentType, Vec<u8>)>;

pub fn init(vega: Rocket<Build>) -> Rocket<Build> {
    vega.mount("/", routes![index, path])
}

fn serve(path: &str) -> ServeResponse {
    let path = format!("{}/{}", env!("OUT_DIR"), path);
    let file = WEBUI_FILES.get(&path).ok().map(Cow::into_owned)?;
    let mime = Path::new(&path)
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .and_then(ContentType::from_extension)?;
    Some((mime, file))
}

#[get("/")]
pub async fn index(discord: &State<Arc<CacheAndHttp>>) -> ServeResponse {
    let bot = discord.cache.current_user().await;

    let mut extra: Vec<String> = Vec::new();
    extra.push(format!("<title>{}</title>", bot.name));

    // https://ogp.me/
    extra.push(format!(
        r#"<meta property="og:title" content="{}" />"#,
        bot.name
    ));
    if let Some(ref avatar) = bot.avatar {
        const SIZE: u16 = 64;
        const_assert!(SIZE >= 16);
        const_assert!(SIZE <= 4096);
        const_assert!(SIZE.is_power_of_two());

        let url = format!(
            "https://cdn.discordapp.com/avatars/{}/{}.png?size={}",
            bot.id, avatar, SIZE
        );

        extra.push(format!(
            r#"<link rel="icon" type="image/png" href="{}" />"#,
            url
        ));
        extra.push(format!(r#"<meta property="og:image" content="{}" />"#, url));
        extra.push(r#"<meta property="og:image:type" content="image/png" />"#.to_string());
        extra.push(format!(
            r#"<meta property="og:image:width" content="{}" />"#,
            SIZE
        ));
        extra.push(format!(
            r#"<meta property="og:image:height" content="{}" />"#,
            SIZE
        ));
    }

    match to_safe_string(&bot) {
        Ok(string) => extra.push(format!(
            r#"<script type="application/x-bot-user+json">{}</script>"#,
            string
        )),
        Err(err) => error!("Bot user JSON serialization failed: {:#?}", err),
    }

    let (mime, source) = serve("index.html")?;
    Some((
        mime,
        String::from_utf8_lossy(&source)
            .replace("</head>", &format!("{}</head>", extra.join("")))
            .into_bytes(),
    ))
}

#[allow(clippy::needless_pass_by_value)]
#[get("/static/<path..>")]
pub fn path(path: PathBuf) -> ServeResponse {
    path.to_str().and_then(serve)
}
