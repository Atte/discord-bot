use super::{json::to_safe_string, util::HeaderResponder};
use rocket::{
    get,
    http::{ContentType, Header},
    routes, Build, Rocket, State,
};
use serenity::CacheAndHttp;
use std::{
    borrow::Cow,
    env, fs,
    path::{Path, PathBuf},
    sync::Arc,
};

// defines const WEBUI_FILES
include!(concat!(env!("OUT_DIR"), "/webui.rs"));

pub fn init(vega: Rocket<Build>) -> Rocket<Build> {
    vega.mount("/", routes![index, path])
}

fn serve(path: &str) -> Option<(ContentType, Vec<u8>)> {
    let file = if env::var_os("WEBUI_PASSTHROUGH").is_some() {
        let base = PathBuf::from("./webui/dist/").canonicalize().ok()?;
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
    let (mime, source) = serve("index.html")?;

    let mut source = String::from_utf8_lossy(&source)
        .replace("(BOT_ID)", &bot.id.to_string())
        .replace("(BOT_NAME)", &bot.name)
        .replace("(BOT_DISCRIMINATOR)", &bot.discriminator.to_string());

    if let Some(ref avatar) = bot.avatar {
        source = source.replace("(BOT_AVATAR)", avatar);
    }
    if let Ok(ref string) = to_safe_string(&bot) {
        source = source.replace(
            "(BOT_JSON)",
            &format!(
                r#"<script type="application/x-bot-user+json">{}</script>"#,
                string
            ),
        );
    }

    Some(
        HeaderResponder::from((mime, source.into_bytes())).set_header(Header::new(
            "Link",
            r#"<https://cdn.discordapp.com>; rel="preconnect"; crossorigin="anonymous""#,
        )),
    )
}
