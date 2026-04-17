//! Ensure the embedded web asset directory exists at compile time.
//!
//! `src/main.rs` uses `#[derive(RustEmbed)]` with `#[folder = "../../apps/web/dist"]`.
//! rust-embed refuses to compile if that directory is missing, which breaks
//! `cargo test` on any checkout that hasn't run `mise run web:build` yet.
//! Creating an empty placeholder here lets rust crates build standalone; the
//! real production bundle still comes from `pnpm build` when present.
use std::{fs, path::PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR set by cargo"),
    );
    let dist = manifest_dir.join("../../apps/web/dist");
    if !dist.exists()
        && let Err(e) = fs::create_dir_all(&dist)
    {
        println!("cargo:warning=failed to create {}: {e}", dist.display());
    }
    println!("cargo:rerun-if-changed=../../apps/web/dist");
}
