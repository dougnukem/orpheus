//! Tauri build script.
//!
//! `tauri.conf.json` declares `bundle.externalBin = ["binaries/orpheus"]`
//! so the packaged desktop app ships the unified `orpheus` CLI alongside
//! the GUI (see `scripts/build-cli-for-tauri.sh`). `tauri-build` validates
//! that the expected per-triple file exists at compile time — which would
//! break plain `cargo check`/`cargo build` in a fresh clone that hasn't
//! staged the real binary yet.
//!
//! To keep workspace-level builds ergonomic we write an empty placeholder
//! here if none exists. `mise run tauri:cli` (auto-invoked by `tauri:dev`
//! / `tauri:build`) and the `build-tauri` CI jobs overwrite it with the
//! real binary before Tauri bundles the `.app` / `.dmg` / `.AppImage` /
//! `.msi`, so release artifacts always ship the genuine article.
use std::{env, fs, path::PathBuf};

fn main() {
    ensure_external_bin_placeholder();
    tauri_build::build()
}

fn ensure_external_bin_placeholder() {
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR set by cargo"));
    let target = env::var("TARGET").expect("TARGET set by cargo");
    let ext = if target.contains("windows") {
        ".exe"
    } else {
        ""
    };

    let bin_dir = manifest_dir.join("binaries");
    if let Err(e) = fs::create_dir_all(&bin_dir) {
        println!("cargo:warning=failed to create {}: {e}", bin_dir.display());
        return;
    }

    let bin_path = bin_dir.join(format!("orpheus-{target}{ext}"));
    if !bin_path.exists() {
        if let Err(e) = fs::write(&bin_path, b"") {
            println!(
                "cargo:warning=failed to stage placeholder {}: {e}",
                bin_path.display()
            );
        } else {
            println!(
                "cargo:warning=staged empty placeholder at {} — run `mise run tauri:cli` before `cargo tauri build` to ship the real orpheus CLI",
                bin_path.display()
            );
        }
    }
    println!("cargo:rerun-if-changed={}", bin_path.display());
}
