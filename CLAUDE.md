# Claude / Agent operating instructions for Orpheus

These notes teach a coding agent how to work productively in this repo.
They supplement any global `~/.claude/CLAUDE.md`.

## Mental model

Orpheus is a **Rust workspace** (`crates/`) plus a **React web app**
(`apps/web`) that the `orpheus` CLI and Tauri shell both embed. A single
extraction core powers every frontend:

```text
apps/web (Vite + React + TS)          user-facing UI — bundled into dist/
                                      consumed by `orpheus serve` and tauri

crates/
  orpheus-core           ← the one source of truth for crypto/extractors/
                           balance/scanner. Every other crate depends on it.
  orpheus-cli            ← clap: scan / extract / mnemonic / demo / serve.
                           The `serve` subcommand embeds apps/web/dist and
                           exposes the JSON API over axum on a local port.
  orpheus-tauri/src-tauri← Tauri v2 desktop shell; #[tauri::command]s call
                           directly into orpheus-core (no HTTP sidecar)
  orpheus-demo-fixtures  ← regenerates synthesized fake wallets +
                           fixtures/mock_balances.json
```

Every production code path in `orpheus-core` has a companion test that
**pins the output to a known value** — BIP test vector, fixed WIF,
decrypt-roundtrip with a hardcoded plaintext. That discipline is
non-negotiable; see "Security-sensitive changes" below.

## Toolchain

**`mise` is the only supported toolchain manager.** `mise.toml` pins:

- rust 1.95
- node 22
- pnpm 9

CI invokes `mise run <task>` exactly like a developer does; if you add a
build/test/lint step, add it to `mise.toml` and reference it from both
local hooks and `.github/workflows/*.yml`.

Never invoke `npm` directly — only `pnpm`. Never call the system
`rustc`/`cargo` before `$HOME/.cargo/bin` — `mise.toml` puts the rustup
toolchain on PATH for this reason.

## Workflow

**Every change ships via a feature branch and a pull request** — no
direct commits to `main`.

1. `git switch -c <type>/<short-name>` (e.g. `feat/bip84-accounts`,
   `fix/multibit-salt-scan`).
2. Commits use [Conventional Commits](https://www.conventionalcommits.org/).
   Types: `feat`, `fix`, `perf`, `refactor`, `docs`, `test`, `build`,
   `ci`, `chore`, `revert`. Optional `(scope)`: `core`, `cli`,
   `tauri`, `web`, `ci`, `deps`, `fixtures`. `!` before `:` for
   breaking changes.
3. Push early, open a draft PR so CI runs.
4. `mise run ci` must pass locally before requesting review.
5. A reviewer plus green CI are required before merging. Squash-merge
   keeps the PR title as the merge commit — keep it conventional.

## Tasks (single source of truth: `mise.toml`)

| Task                    | Use when                                      |
|-------------------------|-----------------------------------------------|
| `mise run setup`        | fresh clone — fetches all deps                |
| `mise run ci`           | full pre-PR check (what GitHub CI also runs)  |
| `mise run lint`         | fmt --check + clippy + web lint + typecheck   |
| `mise run fmt`          | auto-format everything                        |
| `mise run test`         | `cargo test --workspace --locked`             |
| `mise run demo:fixtures`| regenerate synthetic demo wallets             |
| `mise run cli:demo`     | offline scan of the demo fixtures             |
| `mise run server:dev`   | `orpheus serve` on 127.0.0.1:3000             |
| `mise run web:dev`      | Vite on :5173 proxying /api → :3000           |
| `mise run dev`          | `orpheus serve` + web together                |
| `mise run tauri:dev`    | desktop app in development                    |

## Security-sensitive changes

**Anything touching `crypto`, `extractors`, or `balance` requires a test
that pins the behaviour to a known vector.** Acceptable pinning:

- Published BIP test vectors (BIP32/39/49/84/173/350).
- Round-trip fixtures generated inside the same test (encrypt → assert →
  decrypt → assert equal).
- Decode-test payloads with asserted outputs.

**Never put real wallets, real mnemonics, or real passwords in the repo
— even in tests under `fixtures/`.** `orpheus-demo-fixtures` generates
everything from a throwaway BIP39 test mnemonic; use it as the template
for new fixture needs. `.gitignore` blocks `*.wallet`, `*.dat`,
`*.aes.json`, and `passwords*.txt` outside `fixtures/` — keep it that way.

## Wallet format support (as of v0.1)

| Format                                    | Extractor             |
|-------------------------------------------|-----------------------|
| Bitcoin Core `wallet.dat` (BDB / SQLite)  | `bitcoin_core`        |
| MultiBit Classic `.wallet` (unencrypted)  | `multibit`            |
| MultiBit Classic `.wallet` (v3 scrypt+AES)| `encrypted`           |
| Bitcoin Core `dumpwallet` text            | `wallet_dump`         |
| Bitcoin Core `listdescriptors` JSON       | `wallet_dump`         |
| BIP39 (12/15/18/21/24 words)              | `bip39_mnemonic`      |
| blockchain.com legacy 1626/65591 mnemonic | `blockchain_com`      |

BIP39 derivation always covers BIP44, BIP49, BIP84 **and Breadwallet**
`m/0'/{0,1}/x`. The Breadwallet path is load-bearing — it's anchored by
a regression test in `crates/orpheus-core/src/extractors/bip39_mnemonic.rs`
that traces back to the 2013-era iOS wallet recovery that seeded this
project. Don't remove it.

## Balance providers

Default is `blockstream`. Full list (kept in sync across
`orpheus-core::balance::VALID_PROVIDERS`, clap `CliProvider`, and the
frontend `<select>`):

- `blockstream` — public esplora, no API key
- `blockchain` — blockchain.info, batched up to 20 addresses
- `mock` — offline lookup against `--mock-file` JSON
- `none` — skip balance lookup (all zeros)

If you add a provider, add it to all three surfaces in the same PR.

## CI / release pipeline

Three workflows under `.github/workflows/`:

- **`ci.yml`** — pull requests + pushes to main. Matrix test across
  ubuntu/macos/windows plus a `demo-smoke` job that asserts the 0.03865052
  BTC homage still prints in the offline demo. Aggregated into an
  `all-green` job for branch-protection.
- **`latest.yml`** — push to main. Builds the CLI + server + Tauri
  desktop bundles across the native-runner matrix, atomically
  replaces the `latest` GitHub prerelease with every artifact. The
  `latest` release is `prerelease: true` so the canonical "Latest
  release" pointer stays on tagged versions.
- **`release.yml`** — tag `v*` or manual `workflow_dispatch`. Draft
  release → matrix builds → upload → `gh release edit --draft=false
  --latest`. Nothing is visible to users until the last step.

`contents: write` is scoped to the release/publish jobs only. All
`${{ github.* }}` and `inputs.*` values that flow into shell bodies are
routed through `env:` blocks per the workflow-injection guidance.

Dependabot keeps third-party actions, Cargo crates, and npm packages
current. Action-version bumps rewrite floating tags to full-SHA pins
automatically.

## Writing tests

- Unit tests live in the same file as the code (`#[cfg(test)] mod
  tests { ... }`).
- Don't hit the network. `MockProvider` is the correct test seam for
  balance code.
- Prefer a single failing pinned assertion to a wall of looser sanity
  checks — it makes regressions obvious.
- When you add a new extractor, add at least: `can_handle` true-positive,
  `can_handle` false-positive, `extract` success path, and
  `extract` failure path.

## Commit / PR etiquette for agents

- If you are spawned to do work, open a draft PR; do not push straight
  to `main`.
- Leave a one-sentence "Test plan" in the PR body noting which
  `mise run ...` you ran.
- Do **not** regenerate `apps/web/pnpm-lock.yaml` or `Cargo.lock` unless
  a dependency change actually requires it. CI uses `--locked` / 
  `--frozen-lockfile` to catch drift.
- Do **not** commit artifacts from the user's real wallet folders under
  `/Users/.../bitcoin/`, `/Users/.../Documents/personal/Bitcoin/`, or
  any similar path. If the user asks you to verify against real wallets,
  print the commands you ran and summarise the counts/types only — never
  paste WIFs, addresses with value, or passwords into conversation or
  commits.

## Things that should never happen

1. A commit on `main` that didn't go through a PR.
2. A test that prints / asserts on a real private key, mnemonic, or
   password.
3. `npm install` in any apps/web script. Use `pnpm`.
4. A new `--provider` value in one surface (CLI / server / web) without
   the other two.
5. Removing the Breadwallet derivation path without explicit discussion.
