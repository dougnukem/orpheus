# Releasing Orpheus

This doc is the runbook for cutting a release, the signing setup behind
it, and the placeholders that turn on platform-specific signing when
their accounts become available.

## Release channels

| Channel                 | Workflow              | Trigger                       | Tag shape            | GitHub "latest"? |
|-------------------------|-----------------------|-------------------------------|----------------------|------------------|
| Stable                  | `release.yml`         | push `v*` tag / manual        | `v0.1.0`             | ✅ yes           |
| Rolling pre-release     | `latest.yml`          | push to `main`                | `latest`             | ❌ pre-release   |
| Nightly (dated)         | `nightly.yml`         | cron `0 7 * * *` / manual     | `nightly-YYYY-MM-DD` | ❌ pre-release   |

`latest` rebuilds on every push, `nightly-*` rebuilds once a day. Both
are pre-releases so GitHub's "Latest release" pointer only ever moves
when a stable `v*` tag is cut.

## Cutting a stable release

```bash
# 1. Bump versions + run CI locally.
#    The single source of truth is `version = "X.Y.Z"` in Cargo.toml
#    under [workspace.package]. All crates inherit it via
#    `version.workspace = true`, so one edit covers every crate.
#    Use cargo-release for the tag+commit in one go:
cargo install cargo-release   # one-time
cargo release 0.1.1 --workspace --execute --no-publish --no-push

# 2. Push the commit + tag. The tag push triggers release.yml.
git push origin main --follow-tags
```

The `release.yml` workflow will:

1. Create a draft GitHub release.
2. Build CLI + server binaries for 5 targets; build Tauri bundles for
   macOS (universal), Linux x86_64, Windows x86_64.
3. GPG-sign every artifact and produce a clearsigned `SHA256SUMS`.
4. Attach artifacts to the draft and flip it to published + latest.
5. Publish `orpheus-core`, `orpheus-cli`, `orpheus-server` to crates.io.
6. Update the Homebrew cask in `dougnukem/homebrew-orpheus`.

Failure anywhere in steps 1–4 leaves the draft untouched — re-run the
workflow. Failure in 5 or 6 does not block the GitHub release; rerun the
specific sub-workflow (`publish-crates` / `publish-cask`) manually via
`gh workflow run`.

## Linux self-signing (GPG)

All Linux artifacts — plus the tarballs, zips, DMGs, and MSIs — are
signed with a long-lived GPG signing key. Users verify with:

```bash
curl -L https://github.com/dougnukem/orpheus/raw/main/packaging/gpg/orpheus-release-pubkey.asc \
  | gpg --import

# Per-artifact
gpg --verify orpheus_0.1.0_amd64.deb.asc orpheus_0.1.0_amd64.deb

# Or one-shot via the signed manifest
gpg --verify SHA256SUMS.asc SHA256SUMS
sha256sum -c SHA256SUMS
```

### One-time key setup

```bash
# 1. Generate a 4096-bit RSA master + signing subkey. On the interactive
#    prompts pick: RSA+RSA, 4096 bits, expiry 2y.
gpg --full-generate-key
#    UID: "Orpheus Release Signing <releases@orpheus.dev>"

# 2. Identify the key IDs.
gpg --list-secret-keys --keyid-format=LONG

# 3. Export the public key into the repo.
gpg --armor --export <MASTER_KEY_ID> \
  > packaging/gpg/orpheus-release-pubkey.asc

# 4. Export ONLY the signing subkey (never the master) for CI. The `!`
#    after the subkey ID tells gpg to export that subkey only.
gpg --armor --export-secret-subkeys <SUBKEY_ID>! | base64 > ci-key.b64

# 5. Load into GitHub secrets:
gh secret set GPG_PRIVATE_KEY < ci-key.b64
gh secret set GPG_PASSPHRASE --body "<subkey passphrase>"
gh secret set GPG_KEY_ID     --body "<SUBKEY_ID>"
rm ci-key.b64

# 6. Publish the public key to a keyserver so casual users can find it.
gpg --keyserver keys.openpgp.org --send-keys <MASTER_KEY_ID>
```

### Key storage best practices

- Keep the **master key offline** (paper backup + air-gapped USB stick).
  Rotate the subkey yearly; the master stays stable for 2+ years.
- CI only ever holds the signing subkey. Losing it means revoking the
  subkey and issuing a new one — not rotating the whole identity.
- For the highest-value stable releases, consider moving the master key
  onto a YubiKey and signing manually from a local machine before
  uploading. The rolling `latest`/`nightly` CI signatures stay automated.

## crates.io publishing

The tagged `release.yml` calls `publish-crates.yml`. Bootstrap once:

```bash
# 1. Generate a token scoped to publishing the three crates (NOT account-wide).
#    Visit https://crates.io/settings/tokens -> "New token".
#    Scopes: publish-new, publish-update
#    Crate filter: orpheus-core, orpheus-cli, orpheus-server

gh secret set CARGO_REGISTRY_TOKEN --body "cio_..."

# 2. Enable 2FA on your crates.io account.
```

Publishing order (handled automatically by the workflow):

1. `orpheus-core`
2. `orpheus-cli`   (depends on core; `--no-verify` to skip redundant rebuild)
3. `orpheus-server` (same)

`orpheus-tauri` and `orpheus-demo-fixtures` are marked `publish = false`
and skipped.

Users can install without compiling via
[`cargo-binstall`](https://github.com/cargo-bins/cargo-binstall), which
downloads the matching archive from GitHub Releases:

```bash
cargo binstall orpheus-cli
```

The `[package.metadata.binstall]` block in each crate's `Cargo.toml`
points at the release artifact naming scheme.

## Homebrew cask

The first release must bootstrap the tap by hand; afterwards
`publish-cask.yml` keeps it current automatically.

```bash
# 1. Create the tap repo.
gh repo create dougnukem/homebrew-orpheus --public --add-readme \
  --description "Homebrew tap for Orpheus"
cd /tmp && git clone git@github.com:dougnukem/homebrew-orpheus.git
cd homebrew-orpheus && mkdir Casks
printf 'cask "orpheus" do\n  version "0.0.0"\nend\n' > Casks/orpheus.rb
git add . && git commit -m "init tap" && git push

# 2. Create a PAT scoped to the tap repo and store as HOMEBREW_TAP_TOKEN.
#    Fine-grained token, repo: dougnukem/homebrew-orpheus, permissions:
#    Contents: Read and write.
gh secret set HOMEBREW_TAP_TOKEN --body "<pat>"
```

Users then install with:

```bash
brew tap dougnukem/orpheus
brew install --cask orpheus
```

The cask template ships a `caveats` block explaining the one-time
`xattr -cr /Applications/Orpheus.app` required on unsigned builds.
Delete that block from the two templates in
[packaging/homebrew/](../packaging/homebrew/) once Apple signing goes
live.

## Windows signing — SignPath Foundation (placeholder)

The release workflow contains a commented-out SignPath block ready to
enable once the Foundation application is approved.

### Application checklist

- MIT/Apache-2.0 license (✅ already MIT)
- Public repo with non-trivial history (✅)
- Tagged release + changelog (✅ once the first v* tag is cut)
- Reproducible build via GitHub Actions (✅)
- Homepage describing the app (✅ the README)

Apply at [signpath.io/foundation](https://about.signpath.io/foundation).
Review takes ~1–2 weeks.

### After approval

1. On signpath.io: create project `orpheus`, signing policy
   `release-signing`, and link it to this repo.
2. Create an API token (`Settings → API Tokens → Add`) and store it:
   ```bash
   gh secret set  SIGNPATH_API_TOKEN --body "<token>"
   gh variable set SIGNPATH_ORG_ID   --body "<org-uuid>"
   ```
3. Uncomment the `signpath/github-action-submit-signing-request@v1` steps
   in `release.yml` (both `build-cli` and `build-tauri` jobs).
4. Remove any Windows-signing disclaimers from the README.

SignPath signs unsigned artifacts uploaded to GitHub Actions, then
returns signed versions. Our build steps already produce unsigned
artifacts, so only the signing call needs to be enabled.

## macOS signing — Apple Developer ID (placeholder)

Tauri's bundler already reads a full set of `APPLE_*` env vars. The
`build-tauri` job in `release.yml` passes them through; they are simply
unset today, so bundling proceeds unsigned. CLI binaries have a matching
commented-out block in `build-cli`.

### Once enrolled ($99/yr)

1. Create a **Developer ID Application** certificate at
   [developer.apple.com/account/resources/certificates](https://developer.apple.com/account/resources/certificates).
2. Export the cert + private key as a `.p12` with a password.
3. Create an App Store Connect API key at
   [appstoreconnect.apple.com/access/integrations/api](https://appstoreconnect.apple.com/access/integrations/api).
   Download the `.p8`, note the Key ID and Issuer ID.
4. Populate secrets:
   ```bash
   base64 -i cert.p12 | gh secret set APPLE_CERTIFICATE
   gh secret set APPLE_CERTIFICATE_PASSWORD --body "<p12 password>"
   gh secret set APPLE_SIGNING_IDENTITY     --body "Developer ID Application: <Your Name> (<TEAM>)"
   gh secret set APPLE_TEAM_ID              --body "<TEAM>"
   base64 -i AuthKey_XXX.p8 | gh secret set APPLE_API_KEY
   gh secret set APPLE_API_KEY_ID           --body "<KEY_ID>"
   gh secret set APPLE_API_ISSUER           --body "<ISSUER_UUID>"
   ```
5. Uncomment the CLI signing/notarization blocks in `release.yml`
   (`build-cli`), and remove the Gatekeeper `caveats` block from both
   Homebrew cask templates in `packaging/homebrew/`.
6. Rerun `release.yml` via `workflow_dispatch` on the existing tag to
   replace the unsigned artifacts with signed ones.

## Tauri updater signing (minisign)

Separate from OS signing. Used only if we enable the Tauri auto-updater.

```bash
cargo install tauri-cli --version '^2'
cargo tauri signer generate -w ~/.tauri/orpheus-updater.key
# Store the private key + passphrase:
base64 -i ~/.tauri/orpheus-updater.key \
  | gh secret set TAURI_SIGNING_PRIVATE_KEY
gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD --body "<passphrase>"
# The matching public key should be embedded in tauri.conf.json -> bundle.pubkey
# before enabling the updater. See https://v2.tauri.app/plugin/updater/
```

The env vars are already plumbed through `release.yml`; the updater
itself is off until `tauri.conf.json` references the public key.

## Secret + variable cheat-sheet

| Name                                 | Kind     | Required for         | Notes                                   |
|--------------------------------------|----------|----------------------|-----------------------------------------|
| `GPG_PRIVATE_KEY`                    | secret   | all sign steps       | base64-encoded ASCII armor of subkey    |
| `GPG_PASSPHRASE`                     | secret   | all sign steps       | passphrase for the subkey               |
| `GPG_KEY_ID`                         | secret   | all sign steps       | long key ID or fingerprint of subkey    |
| `CARGO_REGISTRY_TOKEN`               | secret   | publish-crates.yml   | scoped crates.io token                  |
| `HOMEBREW_TAP_TOKEN`                 | secret   | publish-cask.yml     | PAT with write on the tap repo          |
| `SIGNPATH_API_TOKEN`                 | secret   | Windows signing      | add when SignPath is live               |
| `SIGNPATH_ORG_ID`                    | variable | Windows signing      | org UUID from signpath.io               |
| `APPLE_CERTIFICATE`                  | secret   | Apple signing        | base64 .p12                             |
| `APPLE_CERTIFICATE_PASSWORD`         | secret   | Apple signing        |                                         |
| `APPLE_SIGNING_IDENTITY`             | secret   | Apple signing        | e.g. `Developer ID Application: Foo (...)`|
| `APPLE_TEAM_ID`                      | secret   | Apple signing        |                                         |
| `APPLE_ID` / `APPLE_PASSWORD`        | secret   | Apple legacy notary  | prefer API key below                    |
| `APPLE_API_KEY`                      | secret   | Apple notarization   | base64 .p8                              |
| `APPLE_API_KEY_ID`                   | secret   | Apple notarization   |                                         |
| `APPLE_API_ISSUER`                   | secret   | Apple notarization   |                                         |
| `TAURI_SIGNING_PRIVATE_KEY`          | secret   | Tauri updater        |                                         |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | secret   | Tauri updater        |                                         |

## Troubleshooting

- **Tag push didn't trigger `release.yml`** — check the tag matches
  `v*`, and that the workflow is enabled. Re-run via
  `gh workflow run release.yml -f tag=v0.1.1` as a fallback.
- **GPG step fails with "no secret key"** — the `GPG_PRIVATE_KEY`
  secret must be the base64 of a private key that matches `GPG_KEY_ID`.
  Regenerate from the source key with the exact `<SUBKEY_ID>!` syntax.
- **cargo publish: "crate already uploaded"** — idempotent; the workflow
  detects this and skips. A real version bump fixes it.
- **Homebrew cask fails to download** — the `sha256` mismatched. Either
  the release artifact naming changed (check `tauri.conf.json` product
  name) or the artifact wasn't uploaded. Re-run `publish-cask.yml`.
- **Nightly skipped with "no commits in last 24h"** — by design. Use
  `workflow_dispatch` to force a build.
