use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=assets");
    // only "fragments" are supposed to contain maud templates that would affect
    // tailwind
    println!("cargo:rerun-if-changed=src/fragments");
    println!("cargo:rerun-if-changed=src/fragments.rs");

    // This should make it possible for distros to override default location.
    let out_dir = PathBuf::from(
        std::env::var_os("PERFIT_BUILD_OUT_DIR").unwrap_or_else(|| env::var_os("OUT_DIR").unwrap()),
    );
    println!("cargo::rustc-env=PERFIT_SHARE_DIR={}", out_dir.display());

    let assets_out_dir = out_dir.join("assets");

    std::fs::create_dir_all(&assets_out_dir).expect("Create out assets dir");

    copy_files(&PathBuf::from("assets"), &assets_out_dir);

    let mut cmd = Command::new("tailwindcss");
    cmd.args([
        "-c",
        "tailwind.config.js",
        "-i",
        "assets/style.css",
        "-o",
        assets_out_dir
            .join("style.css")
            .to_str()
            .expect("Invalid out_dir"),
    ]);
    if cfg!(not(debug_assertions)) {
        cmd.arg("--minify");
    }
    if !cmd.status().expect("failed to run tailwindcss").success() {
        panic!("tailwindcss failed");
    }
}

fn copy_files(src_dir: &Path, dst_dir: &Path) {
    for entry in std::fs::read_dir(src_dir).expect("failed to read dir") {
        let entry = entry.expect("failed to read entry");
        let path = entry.path();
        let src = path.clone();
        let src_rel = path.strip_prefix(src_dir).expect("Must have prefix");
        let dst = dst_dir.join(src_rel);

        println!("Copying {} to {}", src.display(), dst.display());
        if entry.file_type().unwrap().is_dir() {
            copy_files(&src, &dst);
        } else {
            std::fs::copy(src, dst).expect("failed to copy file");
        }
    }
}
