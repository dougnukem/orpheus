# Homebrew cask templates

These templates are rendered by `.github/workflows/publish-cask.yml` into
the external tap repo `dougnukem/homebrew-orpheus` on each tagged release.

- `orpheus.rb.universal.template` — used when a single universal DMG
  (`Orpheus_<ver>_universal.dmg`) is present on the release. This is the
  output of `tauri build --target universal-apple-darwin`.
- `orpheus.rb.perarch.template` — fallback when per-architecture DMGs
  (`Orpheus_<ver>_aarch64.dmg`, `Orpheus_<ver>_x64.dmg`) are present
  instead. Useful if the universal target is disabled.

Token substitution:

| Token              | Meaning                                             |
|--------------------|-----------------------------------------------------|
| `%%VERSION%%`      | Release version without the leading `v` (e.g. `0.1.0`) |
| `%%REPO%%`         | `<owner>/<repo>` of the app repo                    |
| `%%UNIVERSAL_SHA%%`| SHA-256 of the universal DMG                        |
| `%%ARM_SHA%%`      | SHA-256 of the arm64 DMG                            |
| `%%INTEL_SHA%%`    | SHA-256 of the x86_64 DMG                           |

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
