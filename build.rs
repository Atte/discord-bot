use includedir_codegen::Compression;
use std::{env, ffi::OsStr, io, path::Path, process::Command};

const SOURCE_DIR: &str = "webui";
const ENV_SWITCH: &str = "CARGO_FEATURE_WEBUI";

fn yarn<I, S>(args: I) -> io::Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let status = Command::new("yarn")
        .args(args)
        .current_dir(SOURCE_DIR)
        .spawn()?
        .wait()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::new(io::ErrorKind::Other, status.to_string()))
    }
}

fn main() -> io::Result<()> {
    println!("cargo:rerun-if-env-changed={}", ENV_SWITCH);
    if env::var_os(ENV_SWITCH).is_none() {
        return Ok(());
    }

    println!("cargo:rerun-if-changed={}", SOURCE_DIR);

    if !Path::new(SOURCE_DIR).join("node_modules").exists() {
        yarn(["install", "--frozen-lockfile"])?;
    }

    let webui_dist = env::var("OUT_DIR").expect("missing env OUT_DIR");
    yarn(["run", "build", "--dist-dir", &webui_dist])?;

    includedir_codegen::start("WEBUI_FILES")
        .dir(webui_dist, Compression::None)
        .build("webui.rs")?;

    Ok(())
}
