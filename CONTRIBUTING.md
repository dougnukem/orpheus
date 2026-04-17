# Contributing to Orpheus

## Development environment

The project uses [mise](https://mise.jdx.dev) to pin every toolchain version.
Once mise is installed, `cd` into the repo and:

```bash
mise install        # fetch rust 1.95, node 22, pnpm 9
mise run setup      # cargo fetch + pnpm install
```

On `cd` into a fresh clone, mise automatically installs the git pre-commit hook
(`cargo fmt --check` + `cargo clippy -D warnings`). See `mise.toml` for every
task; common ones:

| Task                     | What it does                                    |
|--------------------------|-------------------------------------------------|
| `mise run build`         | `cargo build --workspace`                       |
| `mise run test`          | `cargo test --workspace`                        |
| `mise run lint`          | fmt --check + clippy + `pnpm lint`              |
| `mise run web:dev`       | Vite dev server on :5173                        |
| `mise run server:dev`    | axum on 127.0.0.1:3000                          |
| `mise run dev`           | both of the above together                      |
| `mise run tauri:dev`     | Tauri desktop shell in dev mode                 |
| `mise run cli:demo`      | offline demo scan                               |

## Commit messages — Conventional Commits

Orpheus uses [Conventional Commits](https://www.conventionalcommits.org/) so
that release notes can be generated automatically from the commit range
between tags.

Format:

```
<type>(<scope>)<!>: <short summary>

[optional body]

[optional footer(s)]
```

Recognised types and when to use them:

| Type       | When                                                                |
|------------|---------------------------------------------------------------------|
| `feat`     | a new user-visible feature (triggers a minor version bump)          |
| `fix`      | a bug fix (triggers a patch version bump)                           |
| `perf`     | a performance improvement with no new behaviour                     |
| `refactor` | a restructuring with no behaviour change                            |
| `docs`     | README, CONTRIBUTING, in-code docs                                  |
| `test`     | new or changed tests with no production code change                 |
| `build`    | build system, dependency bumps, lockfile churn                      |
| `ci`       | CI/CD configuration                                                 |
| `chore`    | housekeeping that doesn't fit anything else                         |
| `revert`   | a revert of a prior commit                                          |

Scope (optional, in parentheses) is the area of the codebase — typically one
of: `core`, `cli`, `server`, `tauri`, `web`, `ci`, `deps`, `docs`, `fixtures`.

A `!` before the colon marks a breaking change. A body footer
`BREAKING CHANGE: <description>` is also accepted.

### Examples

```
feat(core): add blockchain.com V4 GUID decoder
fix(cli): treat empty --passwords file as None, not an error
perf(extractors): skip files over 64 MiB before pattern scan
refactor(scanner)!: return Vec<WalletScanResult> instead of Stream<Item=…>

BREAKING CHANGE: scanner no longer supports incremental streaming; callers that
consumed the old Stream must collect into a Vec.
```

### Version bumps

Release notes for tagged releases are generated from the commit range by
`gh release create --generate-notes`. With Conventional Commits they roll up
into clean sections (Features, Bug Fixes, Performance, …).

When you're ready to cut a release, tag the commit with `vX.Y.Z`:

```
git tag -a v0.2.0 -m "v0.2.0"
git push origin v0.2.0
```

The [`release` workflow](.github/workflows/release.yml) takes it from there.

## Pull request workflow

1. Branch from `main`.
2. Keep commits small and conventional.
3. Open a draft PR early so CI runs on your changes.
4. Once green, flip to ready-for-review.
5. Squash-merge preserves the conventional-commit title; review-then-merge
   preserves the full history. Either is fine; the PR title itself should be
   conventional in case of a squash.

## Security-sensitive changes

Orpheus handles private keys and mnemonics. Anything touching the `crypto`,
`extractors`, or `balance` modules gets a mandatory second-pair-of-eyes review
and a test that pins the change to a known vector — either a published BIP
vector, a round-trip fixture, or a hand-crafted payload with an asserted
output.

Never commit real wallets, real mnemonics, or real passwords — even in tests
under `fixtures/`. The generator in `crates/orpheus-demo-fixtures/` produces
synthetic material from a well-known BIP39 test mnemonic; use it as the
template for any new fixture needs.
