# Homebrew tap templates

These templates are rendered into the external tap repo
`dougnukem/homebrew-orpheus` by two workflows:

- `.github/workflows/publish-cask.yml` — fires on tagged releases and
  writes `Casks/orpheus.rb` from one of the stable cask templates below.
- `.github/workflows/publish-cask-head.yml` — fires from `latest.yml`
  on every push to `main` and writes both
  `Casks/orpheus-head-app.rb` (rolling desktop GUI + CLI) and
  `Formula/orpheus-head.rb` (rolling CLI-only). It also deletes the
  legacy `Casks/orpheus-head.rb` from the earlier single-cask layout.

Templates:

- `orpheus.rb.universal.template` — stable cask, used when a single
  universal DMG (`Orpheus_<ver>_universal.dmg`) is present on the
  release. This is the output of `tauri build --target
  universal-apple-darwin`.
- `orpheus.rb.perarch.template` — stable cask fallback when
  per-architecture DMGs (`Orpheus_<ver>_aarch64.dmg`,
  `Orpheus_<ver>_x64.dmg`) are present instead. Useful if the universal
  target is disabled.
- `orpheus-head-app.rb.template` — rolling HEAD cask for the desktop
  GUI. Uses `version :latest` + `sha256 :no_check` because the DMG
  contents change on every push to `main` without a version bump. The
  `binary` stanza symlinks the externalBin-bundled `orpheus` CLI onto
  `$PATH`, so the cask installs GUI + CLI together. `conflicts_with`
  the formula prevents a PATH collision.
- `orpheus-head.rb.formula.template` — rolling HEAD formula for the
  CLI-only use case (Linux + headless macOS). Formulas require a real
  `sha256` per platform, so the workflow downloads each
  `orpheus-<os>-<arch>.tar.gz` artifact from the `latest` pre-release
  and pins the hash. The version string is
  `<workspace-version>.g<sha7>` so each publish mints a fresh formula
  revision.

Token substitution:

| Token | Meaning |
| --- | --- |
| `%%VERSION%%` | Release version (stable: no leading `v`; HEAD formula: `<cargo-version>.g<sha7>`) |
| `%%REPO%%` | `<owner>/<repo>` of the app repo |
| `%%UNIVERSAL_SHA%%` | SHA-256 of the universal DMG (stable cask) |
| `%%ARM_SHA%%` | SHA-256 of the arm64 DMG (stable cask) |
| `%%INTEL_SHA%%` | SHA-256 of the x86_64 DMG (stable cask) |
| `%%DMG_FILENAME%%` | Actual universal DMG filename on the `latest` release (head cask) |
| `%%MACOS_ARM_SHA%%` | SHA-256 of `orpheus-macos-aarch64.tar.gz` (head formula) |
| `%%MACOS_INTEL_SHA%%` | SHA-256 of `orpheus-macos-x86_64.tar.gz` (head formula) |
| `%%LINUX_ARM_SHA%%` | SHA-256 of `orpheus-linux-aarch64.tar.gz` (head formula) |
| `%%LINUX_INTEL_SHA%%` | SHA-256 of `orpheus-linux-x86_64.tar.gz` (head formula) |
| `%%TAP_OWNER%%` | Tap GitHub org/user (e.g. `dougnukem`) |
| `%%TAP_NAME%%` | Tap short name (e.g. `orpheus` — without `homebrew-` prefix) |

## Installing from the tap

Stable (tagged) desktop build:

```bash
brew tap dougnukem/orpheus
brew install --cask orpheus
```

Rolling HEAD desktop (GUI + CLI bundled together):

```bash
brew install --cask dougnukem/orpheus/orpheus-head-app
xattr -cr /Applications/Orpheus.app   # unsigned build — clear Gatekeeper
# later:
brew reinstall --cask orpheus-head-app
```

Rolling HEAD CLI-only (Linux + headless macOS):

```bash
brew install dougnukem/orpheus/orpheus-head
# later:
brew reinstall orpheus-head
```

The cask and formula `conflicts_with` each other — install one or the
other, not both.

## Bootstrapping the tap

The first time, create the tap repo by hand:

```bash
gh repo create dougnukem/homebrew-orpheus --public --add-readme \
  --description "Homebrew tap for Orpheus"
cd homebrew-orpheus
mkdir Casks
# seed with a placeholder the workflow will overwrite
printf 'cask "orpheus" do\n  version "0.0.0"\nend\n' > Casks/orpheus.rb
git add . && git commit -m "init tap" && git push
```

`Formula/` is created by the head workflow on first run; no bootstrap
step needed.

## Removing the Gatekeeper caveat

Once Apple Developer signing is wired up in `release.yml`, delete the
`caveats <<~EOS ... EOS` block in the cask templates and the
`xattr -cr` note in the formula. Installs will then be clean.
