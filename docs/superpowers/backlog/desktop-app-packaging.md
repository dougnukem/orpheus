# Desktop App Packaging — Strategy & Plan

**Status:** backlog / needs-decision
**Created:** 2026-04-17
**Related:** [../plans/2026-04-17-orpheus-implementation.md](../plans/2026-04-17-orpheus-implementation.md), [../specs/2026-04-17-bitcoin-recovery-tool-design.md](../specs/2026-04-17-bitcoin-recovery-tool-design.md)

## Goal

Ship Orpheus as a signed, cross-platform desktop application (macOS, Windows, Linux) that a non-technical wallet-recovery user can double-click to run — in addition to the existing `pip install orpheus` / CLI path.

Decision needed: **Tauri + PyInstaller Python sidecar** vs **full Rust rewrite with Tauri UI**. A TypeScript rewrite is also considered but deprioritized.

## Current state (as of 2026-04-17)

- ~1,700 lines of Python across `src/orpheus/`
- **Crypto primitives** (`crypto/`): secp256k1 via `ecdsa`, BIP32 HD derivation, bech32, address helpers (~300 LOC)
- **Extractors** (`extractors/`): Bitcoin Core BDB byte-scan (DER pattern), MultiBit protobuf + IV/ciphertext pairs, blockchain.com V2/V3/V4/V5 decoders, BIP39 (incl. Breadwallet path regression), `dumpwallet` text parser, AES-256-CBC + scrypt / `EVP_BytesToKey` envelope decryption (~690 LOC)
- **Balance checker** (`balance.py`): Blockchain.info + Blockstream + Mock providers
- **CLI** (`cli.py`): Click-based; subcommands `scan`, `extract`, `mnemonic`, `demo`, `web`
- **Web** (`web/`): empty Flask scaffolding — desktop UI would replace this surface
- External deps: `click`, `flask`, `ecdsa`, `base58`, `requests`, `cryptography`, `mnemonic`, `rich`

## Option A — Tauri + PyInstaller sidecar (keep Python)

Tauri app ships a WebView UI (NextJS/React/TS) and spawns a PyInstaller-built `orpheusd` binary as a sidecar. Frontend talks to it over stdio JSON-RPC or a loopback HTTP server (already partially scaffolded via Flask).

### Pros
- **Ship in days, not weeks.** Every extractor and crypto path already works; no rewrite risk against BIP test vectors or the Breadwallet regression case.
- Keeps the `pip install orpheus` / CLI distribution working unchanged — the desktop app becomes one more shipping target, not a fork.
- Python is a better prototyping environment for the *next* extractors (Electrum, Armory, Samourai, etc.) — easier to iterate than Rust.
- Single source of truth for recovery logic across CLI + desktop.

### Cons (and they are significant for this product)
- **Antivirus false positives.** PyInstaller-wrapped binaries routinely trigger Windows Defender and consumer AV heuristics. For a wallet-recovery tool — exactly the kind of software a paranoid user is inspecting — a SmartScreen warning or AV quarantine is a trust-killer. Mitigable with EV code signing ($$) and reputation-building, but never fully solved.
- **Bundle size.** ~50–100 MB per platform (CPython + stdlib + `cryptography` native wheels + `ecdsa` + deps). Tauri's whole pitch is ~10 MB single-binary; adding Python throws that away.
- **Code-signing complexity.** Must notarize both the Tauri shell *and* the sidecar on macOS. PyInstaller's single-file extraction to temp dirs fights Gatekeeper; `--onedir` mode is safer but means packaging a whole Python tree inside the `.app`.
- **Cold-start + IPC overhead.** PyInstaller single-file unpacks to `$TMPDIR` on every launch (visible UX lag). JSON-RPC over stdio/loopback is fine for correctness but adds surface area.
- **Security posture.** Python processes are easier to introspect/hook than a statically-linked Rust binary. For a key-handling tool this is a narrative disadvantage even if not a practical one.
- **Dependency on `cryptography` native build** complicates the reproducible-build story and `cargo tauri build`'s caching.

### Ship plan (if chosen)
1. Extract Python CLI into a daemon mode (`orpheus serve --stdio` or `--socket`) returning JSON — roughly the shape already in [src/orpheus/cli.py](../../../src/orpheus/cli.py).
2. PyInstaller spec: `--onedir`, hidden imports for `cryptography.hazmat` backends, `--collect-all mnemonic`.
3. Tauri scaffold with NextJS/React frontend; sidecar config in `tauri.conf.json` `bundle.externalBin`.
4. macOS notarization: codesign both binaries, entitlements with `com.apple.security.cs.allow-jit` disabled.
5. Windows EV cert for SmartScreen reputation (lead time: weeks).
6. Linux: AppImage + `.deb` + Flatpak.
7. CI: GitHub Actions matrix build on `macos-14`, `windows-2022`, `ubuntu-22.04`.

**Estimated effort:** 1–2 weeks to first shippable build; 2–4 weeks to clean signed releases on all three platforms.

## Option B — Full Rust rewrite, Tauri UI (NextJS/React/TS)

Replace the Python backend with a Rust crate exposing the same operations. Tauri commands call into the crate directly (in-process) — no sidecar, no IPC. CLI becomes a thin Rust binary wrapping the same crate.

### Ecosystem match (Rust has excellent Bitcoin coverage)
| Current Python                              | Rust replacement                                  |
| ------------------------------------------- | ------------------------------------------------- |
| `ecdsa` secp256k1                           | `secp256k1` (libsecp256k1 bindings — gold standard) |
| `mnemonic` / BIP39                          | `bip39`                                           |
| Manual BIP32 derivation                     | `bitcoin::bip32` / `bdk`                          |
| Manual bech32 / addresses                   | `bech32`, `bitcoin::Address`                      |
| `cryptography` (AES-256-CBC, scrypt)        | `aes`, `block-modes`, `scrypt`                    |
| `base58`                                    | `bs58`                                            |
| MultiBit protobuf                           | `prost` + a hand-rolled `.proto` from the MultiBit source |
| Bitcoin Core BDB DER byte-scan              | pure bytes — trivial port (the pattern `\x30\x81\xd3\x02\x01\x01\x04\x20` is language-agnostic) |
| blockchain.com V2/V3/V4/V5 decoders         | hand port of the bit-unpacking / wordlist logic — 100% deterministic against the committed test vectors |
| HTTP balance providers (`requests`)         | `reqwest`                                         |
| Click CLI                                   | `clap`                                            |
| Rich tables                                 | `comfy-table` / `tabled`                          |

The **Bitcoin Dev Kit (`bdk`)** covers a huge fraction of what we do — HD, address derivation, descriptor handling — so the Rust version may end up *smaller* than the Python one.

### Pros
- **~10 MB single signed binary.** Clean AV story. Clean Gatekeeper/SmartScreen story. Clean narrative for a wallet-recovery tool.
- **No IPC**, no sidecar, no PyInstaller quirks. One process, one supply chain.
- **Faster crypto.** BIP39 seed derivation + scanning dozens of derivation paths per mnemonic is noticeably snappier (libsecp256k1 native vs Python `ecdsa`).
- **Better long-term security story.** Statically-linked, memory-safe, auditable.
- **CLI + desktop share the same crate** — no duplication, no drift.
- **Attracts contributors** in the Bitcoin open-source community, which is Rust-dominant (bdk, rust-bitcoin, LDK, …).

### Cons
- **Rewrite cost: real.** Estimate 2–4 weeks focused work for parity with the current extractor set, plus time to re-establish every test vector and the Breadwallet-path regression.
- **BDB handling is the one place Python was arguably easier** — but we don't actually parse BDB; we byte-scan for a DER prefix, which is the same in any language.
- **MultiBit protobuf** needs a `.proto` definition recovered from upstream; a half-hour task but a task.
- Loses the "it's just Python, I can read it" accessibility for security-minded users auditing the recovery tool. (Counter: Rust is auditable and more and more Bitcoin users read it fluently.)
- CLI distribution story changes: no more `pip install orpheus`. Instead: `brew install orpheus`, `cargo install orpheus`, `.deb` / `.rpm`, GitHub Releases binaries. Probably a net positive for non-Python users, a step down for Python users.

### Ship plan (if chosen)
1. New `rust/` workspace: `orpheus-core` (library), `orpheus-cli` (`clap`), `orpheus-desktop` (Tauri).
2. **Port in the order the tests already prove correctness:**
   1. `crypto::keys` / `bip32` / `bech32` — BIP test vectors are language-agnostic; copy `tests/` assertions into Rust `#[test]`s.
   2. `extractors::bip39` — including the Breadwallet `m/0'/0/x` regression vector.
   3. `extractors::wallet_dump`, `bitcoin_core`, `multibit`.
   4. `extractors::blockchain_com` V2 → V5 — the hardest port; do one sub-version at a time, each against the committed hand-crafted vectors.
   5. `extractors::encrypted` — AES/scrypt/EVP_BytesToKey.
   6. `balance::*` providers — trivial with `reqwest`.
3. Tauri commands as a thin layer over `orpheus-core` (`scan`, `extract`, `mnemonic_derive`, `check_balances`).
4. Frontend: NextJS static export (`next export`) served by Tauri — matches the team's existing NextJS/React/TS fluency.
5. Keep the current Python codebase on a `legacy-python` branch until Rust parity is verified by running both against the same fixture directory and diffing JSON output.
6. Delete Python `src/` only after N successful real-wallet recoveries on the Rust version.

**Estimated effort:** 3–5 weeks to shippable parity; 5–7 weeks to signed, notarized releases on all platforms.

## Option C — TypeScript rewrite (mentioned, not recommended)

Porting to Node/Deno/Bun with `@noble/secp256k1`, `@scure/bip32`, `bitcoinjs-lib`.

- **Pros:** single language across frontend + backend. Reasonable crypto libs (`@noble/*` is audited).
- **Cons:**
  - `@noble/secp256k1` is ~10–30× slower than libsecp256k1 for batch derivation; felt when sweeping dozens of paths.
  - Handling **raw bytes from BDB / MultiBit / AES-CBC envelopes** in JS is painful vs Rust's `&[u8]`. Lots of `Buffer` / `Uint8Array` gymnastics.
  - Wallet-recovery tool written in Node has a worse security narrative than Rust (supply-chain anxiety is higher in npm than crates.io, fairly or not).
  - No meaningful CLI distribution advantage over Rust.

Only revisit if the team decides the frontend can absorb the backend entirely and we're willing to accept the perf + auditability hit.

## Recommendation

**Pursue Option B (full Rust rewrite) as the target end state, but consider Option A as an optional bridge.**

Rationale:
1. For a **wallet-recovery tool**, the AV-false-positive + bundle-size + code-signing drag of PyInstaller is not a minor ergonomic issue — it's a product-level trust issue. Users who find this tool will google it and find AV warnings; that's fatal.
2. Rust's Bitcoin ecosystem (`rust-bitcoin`, `bdk`, `secp256k1`, `bip39`) covers essentially 100% of what we do and is *higher-quality* than the Python equivalents we're using.
3. The existing Python codebase is only ~1,700 LOC and is **already heavily test-covered with deterministic vectors** (BIP32, BIP39, BIP173, BIP350, blockchain.com hand-crafted vectors, Breadwallet regression). That is exactly the codebase where a rewrite is low-risk — every line has a passing oracle.
4. CLI + desktop sharing one crate is a cleaner forever-architecture than CLI-in-Python + desktop-in-Tauri-wrapping-Python.

**The case for doing A first** (as a 2-week bridge) is only compelling if: (a) there is time pressure to put *something* desktop-shaped in front of users this quarter, and (b) we accept throwing that work away. Given the tool is pre-1.0 and unreleased, the time pressure is low; better to spend the 3–5 weeks once and ship the architecture we want to keep.

## Open questions (to resolve before writing the execution plan)

1. **Timeline pressure?** Are we demoing this to anyone before Rust parity is realistic (~4 weeks)? If yes → A-then-B. If no → B direct.
2. **Is `pip install orpheus` a supported distribution forever, or is desktop + signed binary releases the new story?** Affects whether we maintain Python on a branch or delete it.
3. **Who will audit the Rust port?** Recommend at least one outside review pass before shipping, given the "it handles your keys" surface.
4. **Windows EV code-signing cert:** get the procurement started now regardless of which option — 2–4 week lead time and needed for both.
5. **Frontend scope:** is v1 just the CLI surfaces wrapped in a UI, or do we want richer desktop-native features (drag-drop wallet file, scheduled re-scans, password-list manager UI)? Impacts Tauri command design.

## Deliverable when decision is made

Move the chosen option into [../plans/](../plans/) as a numbered implementation plan with phases mirroring the existing `2026-04-17-orpheus-implementation.md` structure (TDD, BIP vectors first, commit per step, Breadwallet regression preserved end-to-end).
