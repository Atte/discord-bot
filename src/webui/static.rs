use super::{json::to_safe_string, util::HeaderResponder};
use indoc::formatdoc;
use log::error;
use rocket::{
    get,
    http::{ContentType, Header},
    routes, Build, Rocket, State,
};
use serenity::CacheAndHttp;
use static_assertions::const_assert;
use std::{
    borrow::Cow,
    path::{Path, PathBuf},
    sync::Arc,
};

// defines const WEBUI_FILES
include!(concat!(env!("OUT_DIR"), "/webui.rs"));

pub fn init(vega: Rocket<Build>) -> Rocket<Build> {
    vega.mount("/", routes![index, path])
}

fn serve(path: &str) -> Option<(ContentType, Vec<u8>)> {
    let path = format!("{}/{}", env!("OUT_DIR"), path);
    let file = WEBUI_FILES.get(&path).ok().map(Cow::into_owned)?;
    let mime = Path::new(&path)
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .and_then(ContentType::from_extension)?;
    Some((mime, file))
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
    discord: &State<Arc<CacheAndHttp>>,
) -> Option<HeaderResponder<(ContentType, Vec<u8>)>> {
    let bot = discord.cache.current_user().await;

    let mut extra: Vec<String> = Vec::new();
    extra.push(formatdoc!(
        r#"
            <title>{name}</title>
            <meta property="og:title" content="{name}" />
        "#,
        name = bot.name
    ));

    if let Some(ref avatar) = bot.avatar {
        const SIZE: u16 = 64;
        const_assert!(SIZE >= 16);
        const_assert!(SIZE <= 4096);
        const_assert!(SIZE.is_power_of_two());

        let url = format!(
            "https://cdn.discordapp.com/avatars/{}/{}.png?size={size}",
            bot.id,
            avatar,
            size = SIZE
        );

        extra.push(formatdoc!(r#"
            <link rel="icon" type="image/png" href="{url}" sizes="{size}x{size}" crossorigin="anonymous" />
            <meta property="og:image" content="{url}" />
            <meta property="og:image:type" content="image/png" />
            <meta property="og:image:width" content="{size}" />
            <meta property="og:image:height" content="{size}" />
        "#, url=url, size=SIZE));
    }

    match to_safe_string(&bot) {
        Ok(string) => extra.push(format!(
            r#"<script type="application/x-bot-user+json">{}</script>"#,
            string
        )),
        Err(err) => error!("Bot user JSON serialization failed: {:#?}", err),
    }

    let (mime, source) = serve("index.html")?;
    Some(
        HeaderResponder::from((
            mime,
            String::from_utf8_lossy(&source)
                .replace("</head>", &format!("{}</head>", extra.join("")))
                .into_bytes(),
        ))
        .set_header(Header::new(
            "Link",
            r#"<https://cdn.discordapp.com>; rel="preconnect"; crossorigin="anonymous""#,
        )),
    )
}
