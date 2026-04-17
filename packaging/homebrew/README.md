# Homebrew cask templates

These templates are rendered into the external tap repo
`dougnukem/homebrew-orpheus` by two workflows:

- `.github/workflows/publish-cask.yml` — fires on tagged releases and
  writes `Casks/orpheus.rb` from one of the stable templates below.
- `.github/workflows/publish-cask-head.yml` — fires from `latest.yml`
  on every push to `main` and writes `Casks/orpheus-head.rb` from the
  HEAD template.

Templates:

- `orpheus.rb.universal.template` — stable cask, used when a single
  universal DMG (`Orpheus_<ver>_universal.dmg`) is present on the
  release. This is the output of `tauri build --target
  universal-apple-darwin`.
- `orpheus.rb.perarch.template` — stable cask fallback when
  per-architecture DMGs (`Orpheus_<ver>_aarch64.dmg`,
  `Orpheus_<ver>_x64.dmg`) are present instead. Useful if the universal
  target is disabled.
- `orpheus-head.rb.template` — rolling HEAD cask. Uses `version :latest`
  with `sha256 :no_check` because the DMG contents change on every push to
  `main` without a version bump. Users opt in explicitly via the
  `-head` suffix and re-pull with `brew reinstall --cask orpheus-head`.
  `conflicts_with` prevents co-installing the stable cask.

Token substitution:

| Token | Meaning |
| --- | --- |
| `%%VERSION%%` | Release version without the leading `v` (e.g. `0.1.0`) |
| `%%REPO%%` | `<owner>/<repo>` of the app repo |
| `%%UNIVERSAL_SHA%%` | SHA-256 of the universal DMG |
| `%%ARM_SHA%%` | SHA-256 of the arm64 DMG |
| `%%INTEL_SHA%%` | SHA-256 of the x86_64 DMG |
| `%%DMG_FILENAME%%` | Actual universal DMG filename on the `latest` release (detected at publish time) |
| `%%TAP_OWNER%%` | Tap GitHub org/user (e.g. `dougnukem`) |
| `%%TAP_NAME%%` | Tap short name (e.g. `orpheus` — without `homebrew-` prefix) |

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

Users then install with:

```bash
brew tap dougnukem/orpheus
brew install --cask orpheus
```

## Removing the Gatekeeper caveat

Once Apple Developer signing is wired up in `release.yml`, delete the
`caveats <<~EOS ... EOS` block in both templates. The cask will then
install and launch cleanly with no manual `xattr` step.
