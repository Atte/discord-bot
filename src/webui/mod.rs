use crate::{config::WebUIConfig, Result};
use rocket::{routes, get, State, http::ContentType};
use std::{path::{Path, PathBuf}, borrow::Cow, str::FromStr};
use log::trace;

// defines const WEBUI_FILES
include!(concat!(env!("OUT_DIR"), "/webui.rs"));

struct WebUI {}

impl WebUI {
    pub fn new() -> Self {
        WebUI {}
    }
}

type ServeResponse = Option<(ContentType, Vec<u8>)>;

fn serve(path: &str) -> ServeResponse {
    let path = format!("{}/{}", env!("OUT_DIR"), path);
    let file = WEBUI_FILES.get(&path).ok().map(Cow::into_owned)?;
    let mime = Path::new(&path).extension().and_then(std::ffi::OsStr::to_str).and_then(ContentType::from_extension)?;
    Some((mime, file))
}

#[get("/")]
fn index() -> ServeResponse {
    serve("index.html")
}

#[allow(clippy::needless_pass_by_value)]
#[get("/<path..>")]
fn static_path(path: PathBuf) -> ServeResponse {
    path.to_str().and_then(serve)
}

pub async fn run(config: WebUIConfig) -> Result<()> {
    WEBUI_FILES.set_passthrough(std::env::var_os("WEBUI_PASSTHROUGH").is_some());
    let fnames: Vec<_> = WEBUI_FILES.file_names().collect();
    trace!("fnames {}", fnames.len());
    for fname in fnames {
        trace!("fname {}", fname);
    }
    rocket::build()
        .manage(WebUI::new())
        .manage(config)
        .mount("/", routes![
            index, static_path,
        ])
        .launch()
        .await?;
    Ok(())
}
