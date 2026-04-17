#!/usr/bin/env bash
# Build the `orpheus` CLI and stage it for the Tauri bundler as an
# externalBin resource. Tauri expects one file per target triple named
# `binaries/orpheus-<triple>[.exe]`; the shell then ships it at
#   macOS:   Orpheus.app/Contents/MacOS/orpheus
#   Linux:   /usr/lib/Orpheus/orpheus (deb/rpm) or inside the AppImage
#   Windows: next to Orpheus.exe
#
# The Homebrew cask uses a `binary` stanza pointing to the macOS path so
# `brew install --cask orpheus` also exposes `orpheus` on $PATH.
#
# Usage:
#   build-cli-for-tauri.sh                       # auto-detect host triple
#   build-cli-for-tauri.sh <triple>              # single target
#   build-cli-for-tauri.sh <triple1> <triple2>   # multiple (macOS universal)
#
# Called from mise (tauri:dev, tauri:build) and from the build-tauri CI
# jobs in latest.yml / release.yml / nightly.yml.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

OUT_DIR="$ROOT/crates/orpheus-tauri/src-tauri/binaries"
mkdir -p "$OUT_DIR"

if [[ $# -eq 0 ]]; then
  HOST_TRIPLE=$(rustc -vV | sed -n 's/host: //p')
  set -- "$HOST_TRIPLE"
fi

for TRIPLE in "$@"; do
  echo "==> building orpheus CLI for $TRIPLE"
  rustup target add "$TRIPLE" >/dev/null 2>&1 || true
  cargo build --release --locked -p orpheus-cli --target "$TRIPLE"

  EXT=""
  [[ "$TRIPLE" == *windows* ]] && EXT=".exe"
  SRC="$ROOT/target/$TRIPLE/release/orpheus$EXT"
  DST="$OUT_DIR/orpheus-$TRIPLE$EXT"

  cp "$SRC" "$DST"
  # `cp` preserves mode on macOS/Linux, but Dropbox and similar file-sync
  # tools silently strip the executable bit on files under their watch.
  # Tauri's bundler copies externalBin resources bit-for-bit into
  # `Orpheus.app/Contents/MacOS/`, so a non-executable source leaks into
  # the installed bundle and the Homebrew cask `binary` stanza ends up
  # symlinking an unrunnable file onto $PATH. Force +x here so the staged
  # source is always executable before Tauri sees it.
  chmod +x "$DST"
  echo "    staged $DST"
done
