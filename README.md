# Orpheus

![Orpheus](https://github.com/user-attachments/assets/33de7189-3eaf-4239-b696-e03cac7c078a)

> **καταβαίνω** — *to descend*
>
> Recover lost cryptocurrency from forgotten wallets.

Orpheus is a Rust + Tauri v2 tool for extracting private keys from old Bitcoin
wallet files (Bitcoin Core `wallet.dat`, MultiBit Classic `.wallet`, Bitcoin
Core `dumpwallet` output, BIP39 seed phrases, and blockchain.com legacy
mnemonics). It ships as a single-binary CLI, an HTTP server hosting an embedded
React UI, and a native desktop app — all sharing the same extraction core.

## Architecture

```text
orpheus/
├── Cargo.toml              # workspace
├── mise.toml               # toolchain + tasks + hooks
├── rust-toolchain.toml
├── crates/
│   ├── orpheus-core/       # extractors, crypto, balance, scanner
│   ├── orpheus-cli/        # clap CLI: scan / extract / mnemonic / demo
│   ├── orpheus-server/     # axum + rust-embed: hosts apps/web/dist + /api
│   ├── orpheus-tauri/      # Tauri v2 desktop shell; same API, no sidecar
│   └── orpheus-demo-fixtures/
└── apps/
    └── web/                # Vite + React 19 + TS + Tailwind v4 + shadcn
```

The `orpheus-core` crate is the single source of truth. Every frontend
(`cli`, `server`, `tauri`) delegates to it.

## Getting started

Orpheus is managed by **[mise](https://mise.jdx.dev)**. It's the single
source of truth for toolchain versions (see `[tools]` in `mise.toml`) and
for every scripted task — local development and CI run the same
`mise run <task>` commands.

```bash
# Install mise once: https://mise.jdx.dev
mise install              # fetch rust 1.95, node 22, pnpm 9
mise run setup            # cargo fetch + pnpm install
```

On `cd` into the repo mise auto-activates the toolchain and installs the
git pre-commit hook (`.githooks/pre-commit` → `mise run pre-commit` =
`cargo fmt --check` + `cargo clippy -D warnings`).

### Common tasks

```bash
mise run build           # cargo build --workspace
mise run test            # cargo test --workspace
mise run lint            # fmt --check + clippy + pnpm lint
mise run fmt             # cargo fmt + prettier
mise run cli:demo        # offline demo via CLI
mise run server:dev      # axum server on 127.0.0.1:3000
mise run web:dev         # Vite dev server on :5173 proxying /api → :3000
mise run dev             # both above together
mise run tauri:dev       # desktop app in development mode
mise run tauri:build     # package desktop app
```

## Try the offline demo

No wallets, no network. Runs against synthesized fixtures:

```bash
cargo run -p orpheus-demo-fixtures  # regenerate fixtures/demo-wallets/
cargo run -p orpheus-cli -- demo
```

Output: 5 wallets, 325 keys, a synthetic 0.08730104 BTC including a
homage to the 0.03865052 BTC recovered in the original session that
seeded this project.

## Supported wallet formats

| Format                                    | Extractor             |
|-------------------------------------------|-----------------------|
| Bitcoin Core `wallet.dat` (BDB or SQLite) | `bitcoin_core`        |
| MultiBit Classic `.wallet` (unencrypted)  | `multibit`            |
| MultiBit Classic `.wallet` (v3 scrypt+AES)| `encrypted`           |
| Bitcoin Core `dumpwallet` text            | `wallet_dump`         |
| Bitcoin Core `listdescriptors` JSON       | `wallet_dump`         |
| BIP39 mnemonic (12/15/18/21/24 words)     | `bip39_mnemonic`      |
| blockchain.com legacy 1626/65591 mnemonic | `blockchain_com`      |

BIP39 derivation runs BIP44, BIP49, BIP84, *and* Breadwallet's legacy
`m/0'/{0,1}/x` — a first-class path anchored by a regression test for
the 2013-era iOS wallet that motivated this project.

## Balance providers

Every recovered address can be enriched with balance + received-total +
tx-count via `--provider`:

| Value         | Source                                                        |
|---------------|---------------------------------------------------------------|
| `blockstream` | https://blockstream.info/api (public esplora, no API key) — **default** |
| `blockchain`  | https://blockchain.info/balance (public, batched up to 20 addrs) |
| `mock`        | offline lookup against `--mock-file` JSON (used by `orpheus demo`) |
| `none`        | skip balance lookup entirely                                  |

The same identifiers work in `--provider <name>` on the CLI, the
`ORPHEUS_PROVIDER` env var, and the `Balance provider` dropdown in the
web UI. Use `none` or `mock` on air-gapped machines; the default
deliberately calls out to the internet so first-run scans report real
balances without extra flags.

## Releases & versioning

This repo follows [Conventional Commits](https://www.conventionalcommits.org/).
Release notes for every tagged release are generated from the commit
range and grouped by type (Features / Bug Fixes / Performance / …). See
[CONTRIBUTING.md](CONTRIBUTING.md#commit-messages--conventional-commits)
for the conventions we enforce.

**Where to grab builds:**

- [**Latest release**](https://github.com/dougnukem/orpheus/releases/latest) —
  the most recent signed, tagged release. Use this.
- [**`latest` pre-release**](https://github.com/dougnukem/orpheus/releases/tag/latest) —
  rolling build of `main`; useful for bleeding-edge smoke-testing.

Artifacts published for every tagged release:

- `orpheus-vX.Y.Z-{linux,macos,windows}-{x86_64,aarch64}.{tar.gz,zip}` —
  standalone `orpheus` CLI + `orpheus-server` binary
- Tauri desktop bundles (`.dmg` universal macOS, `.AppImage` + `.deb` Linux,
  `.msi` Windows)
- `SHA256SUMS` for integrity verification

## Security

- **Never feed real wallets to an untrusted build.** Audit the code first
  or pin a commit you've reviewed.
- **Offline by default.** The balance provider defaults to `mock` (fully
  offline); only opt-in to blockchain.info / blockstream.info.
- **Ephemeral Docker mode** is recommended for high-value recovery —
  mount the wallet directory read-only, run with `--network none` when
  not fetching balances, shut down when done.
- **Clear your scrollback** after a successful extraction. WIF keys in
  your terminal history are keys in your terminal history.

## License

MIT. See `LICENSE`.
