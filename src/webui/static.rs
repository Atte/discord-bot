use rocket::{get, http::ContentType, routes, Build, Rocket};
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
pub fn index() -> ServeResponse {
    serve("index.html")
}

#[allow(clippy::needless_pass_by_value)]
#[get("/<path..>")]
pub fn path(path: PathBuf) -> ServeResponse {
    path.to_str().and_then(serve)
}
