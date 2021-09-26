use includedir_codegen::Compression;
use std::{ffi::OsStr, path::Path, process::Command};

const SOURCE_DIR: &'static str = "webui";

fn npm<I, S>(args: I) -> std::io::Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    Command::new("npm")
        .args(args)
        .current_dir(SOURCE_DIR)
        .spawn()?
        .wait()?;
    Ok(())
}

fn main() -> std::io::Result<()> {
    println!("cargo:rerun-if-changed={}", SOURCE_DIR);

    if !Path::new(SOURCE_DIR).join("node_modules").exists() {
        npm(["ci"])?;
    }

    let webui_dist = std::env::var("OUT_DIR").expect("missing env OUT_DIR");
    npm(["run", "build", "--", "--dist-dir", &webui_dist])?;

    includedir_codegen::start("WEBUI_FILES")
        .dir(webui_dist, Compression::None)
        .build("webui.rs")?;

    Ok(())
}
