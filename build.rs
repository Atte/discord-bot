use std::process::Command;
use std::ffi::OsStr;
use includedir_codegen::Compression;

fn npm<I, S>(args: I) -> std::io::Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>, {
    Command::new("npm").args(args).current_dir("webui").spawn()?.wait()?;
    Ok(())
}

fn main() -> std::io::Result<()> {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=webui");

    let webui_dist = std::env::var("OUT_DIR").expect("missing env OUT_DIR");
    npm(["install"])?;
    npm(["run", "build", "--", "--dist-dir", &webui_dist])?;

    includedir_codegen::start("WEBUI_FILES").dir(webui_dist, Compression::None).build("webui.rs")?;

    Ok(())
}
