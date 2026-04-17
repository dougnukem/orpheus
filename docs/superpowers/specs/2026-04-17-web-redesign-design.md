# Orpheus web app redesign — design spec

- **Date:** 2026-04-17
- **Branch:** `feat/web-redesign`
- **Scope:** `apps/web/**`, plus targeted extensions in `orpheus-core`, `orpheus-server`, `orpheus-tauri/src-tauri` to support the new UI.
- **Out of scope:** extractor logic, crypto code, CLI (except kept in sync per project rules).

## 1. Goals

1. **Fix broken CSS.** Every component references `--color-bronze`, `--color-verdigris`, `--color-rust`, `--color-fg`, `--color-fg-dim`, `--color-fg-faint`, `--color-rule`, `--color-bg-inset`, `--color-bronze-lit`. **None are defined** — `apps/web/src/index.css` contains only unrelated starter tokens (`--text`, `--bg`, `--border`, `--accent`). The app renders on browser defaults. This is the first thing to fix.
2. **Strip the Greek-mythology theming.** The name **Orpheus** stays. Everything else goes: Ω glyph, `καταβαίνω`, "to descend", "Descent" navigation, Roman numerals (I / II / III / IV) as panel numbers, "Cast the phrase", "Begin descent", "silence in the hall", "Retrieved from the underworld", "a warning from the guide", serif display type, bronze/verdigris/rust palette, `grain` overlay, `text-shadow` glow halos, `panel-enter`/`glyph-enter`/`descent-marker` animations.
3. **Turn the app into a navigable explorer.** Today: "form → static result list." Target: bookmarkable routes for results / wallet / address / import-instructions with clear CTAs that end on *"I have my key and I know how to use it."*

## 2. Non-goals

- No changes to extractor logic, crypto routines, or `orpheus-core::balance::VALID_PROVIDERS` membership.
- No Tauri shell redesign beyond the new `#[tauri::command]` endpoints needed for folder-picking and transaction lookups.
- No keyboard shortcut palette (⌘K).
- No sweep/sign/broadcast in-app. Orpheus extracts and explains. It does not transact. That boundary is non-negotiable.
- No fiat price fetching. Render a USD estimate only when the balance provider response happens to include a price; otherwise omit. No new price feed.
- No server-side persistence of scan results. Session-scoped only.

## 3. Visual system

### Style

Linear/Vercel-adjacent: flat, dark-default, sans-serif, one accent color, generous-but-not-wasteful spacing. Auto-switches to light when `prefers-color-scheme: light`. Both themes are first-class (not a quick inversion).

### Palette

Defined in `apps/web/src/index.css` under Tailwind v4 `@theme`:

| Token                | Dark      | Light     | Semantic               |
|----------------------|-----------|-----------|------------------------|
| `--color-bg`         | `#0b0d10` | `#ffffff` | page background        |
| `--color-surface`    | `#11141a` | `#f6f8fa` | cards, panels          |
| `--color-border`     | `#1d2125` | `#d0d7de` | hairline dividers      |
| `--color-text`       | `#e6e8eb` | `#1f2328` | body text              |
| `--color-text-dim`   | `#9ba1a6` | `#57606a` | secondary text         |
| `--color-text-faint` | `#7a8087` | `#8c959f` | tertiary / labels      |
| `--color-accent`     | `#3b82f6` | `#0969da` | focus, primary CTA     |
| `--color-success`    | `#22c55e` | `#1a7f37` | balance > 0 (reserved) |
| `--color-warn`       | `#f59e0b` | `#9a6700` | caution banners        |
| `--color-danger`     | `#f87171` | `#d1242f` | errors, locked state   |

`--color-success` is reserved strictly for "balance found" and "key unlocked" moments — never for UI chrome like "action succeeded" toasts.

### Type

- **Sans:** `ui-sans-serif, system-ui, -apple-system, "Segoe UI", Roboto, sans-serif` — whole UI.
- **Mono:** `ui-monospace, "SF Mono", Menlo, monospace` — addresses, WIFs, derivation paths, tx hashes.
- **No serif anywhere.**
- Base size **14px** (was 18px). Labels 10–11px, `text-transform: uppercase`, `letter-spacing: 0.08em`. Headings 13 / 16 / 20 / 24px.
- All numeric cells use `font-variant-numeric: tabular-nums` so decimal points line up.

### Spacing & shape

- 4px base grid.
- Cards: 6px radius, 1px border, **no shadow**.
- Buttons: 5px radius.
- Focus ring: 2px solid `--color-accent`, no glow.

### Motion

- Remove existing `panel-enter`, `glyph-enter`, `descent-marker`, `grain`, `text-shadow` halo.
- Add: 120ms ease-out on hover backgrounds; 200ms fade on route/panel swap. Nothing else animates.
- Respect `prefers-reduced-motion: reduce` — disable the fade.

## 4. Information architecture

### Top-level navigation

Three tabs at top of page (the sidebar `Descent` nav is removed):

- **Wallet files** (`/scan`)
- **Mnemonic** (`/mnemonic`)
- **Results** (`/results`)

### Routes (React Router, client-side)

| Route                         | Purpose                                                                |
|-------------------------------|------------------------------------------------------------------------|
| `/`                           | redirects to `/scan`                                                   |
| `/scan`                       | file/folder input, password list, provider, submit                     |
| `/mnemonic`                   | phrase input, BIP39 vs blockchain.com, derive                          |
| `/results`                    | summary across all scans this session                                  |
| `/results/:walletId`          | one source file — addresses table, tabs for `All`/`With value`/`Empty` |
| `/results/:walletId/:address` | one address — balance, tx history, CTAs                                |
| `/import/:keyId`              | per-wallet import instructions (Electrum/Sparrow/Core/Hardware)        |

- `walletId = sha256(source_file + '|' + source_type).slice(0, 16)`. Deterministic per scan input.
- `keyId = sha256(address).slice(0, 16)`. We never put WIFs in URLs.

### Persistence

Results cache in `sessionStorage` under key `orpheus:v1:results`. Cleared when the tab closes. No `localStorage`. No IndexedDB. No server-side storage.

Schema is versioned. On schema-version mismatch, the cache is discarded without migration.

### Global header

On every route: brand mark · "Orpheus" wordmark · breadcrumbs for current route · right-aligned **active-provider indicator**.

The indicator is functional, not decorative:

- Green dot + name (e.g. `● blockstream.info`) when a network provider is selected.
- Amber dot when a non-default network provider is selected (`blockchain.info`).
- Grey dot + "offline" when provider = `mock` or `none`.
- Clicking the indicator opens a small provider-switcher popover.

Goal: a user recovering a sensitive wallet can see at a glance whether balance lookups are hitting the network.

## 5. Scan input methods

Priority order, picked by runtime feature detection:

1. **Tauri — native OS folder picker.** `@tauri-apps/plugin-dialog` `open({ directory: true, multiple: false })`. Path goes via a new `#[tauri::command] scan_directory(path, passwords, provider)` straight into `orpheus-core`. **No file transfer.** Highest privacy, best UX.
2. **Chrome/Edge — File System Access API.** `window.showDirectoryPicker({ startIn: 'documents' })` returns a `FileSystemDirectoryHandle`. We walk it recursively, collect candidate wallet files (`*.dat`, `*.wallet`, `*.aes.json`, `*.json`, `*.txt`), stream them to `POST /api/scan` as multipart. Server (running on `127.0.0.1`) does the extraction.
3. **Firefox / Safari / no FS Access — dropzone fallback.** Current multi-file drag-and-drop. Only rendered when the two above are unavailable.

Feature detection runtime check, in order:

```ts
if (window.__TAURI_INTERNALS__) return 'tauri';
if (typeof window.showDirectoryPicker === 'function') return 'fs-access';
return 'dropzone';
```

### Scan page UX

- Big primary button: **Pick a folder** (label: "Choose folder" in Tauri, "Pick folder" in Chrome). Expands into a multi-line help card showing common wallet paths by OS:
  - macOS: `~/Library/Application Support/Bitcoin/wallets/`
  - Linux: `~/.bitcoin/wallets/`
  - Windows: `%APPDATA%\Bitcoin\wallets\`
  - Plus `~/Documents` for legacy `.aes.json`, `.wallet` files.
- Thin link below: **"Or select individual files"** — reveals the dropzone fallback for any browser.
- Settings row (two columns on desktop): Provider `<select>` · Passwords `<textarea>` (monospace, placeholder `one per line`).
- Submit: **Scan**. Status line to the right: idle / `scanning… 47 of 103 files` / `done: 3 keys with value` / error message.
- Recent scans list (sessionStorage-backed) beneath the form, click to jump to `/results/:walletId`.

## 6. Screens

### `/mnemonic`

- `Type` select: BIP39 / blockchain.com legacy.
- Phrase `<textarea>` (mono, 3 rows). Word-count indicator: `12 / 12 words` (green when 12/15/18/21/24; amber otherwise).
- Row: `BIP39 passphrase` (optional) · `Gap limit` (number, default 20, min 1, max 200).
- `wordlist` path input — rendered **only** when type = blockchain.com legacy.
- Submit: **Derive keys**. Status line.
- Decoded-password result card (blockchain.com legacy only): copyable, warning line "This password unlocks the `wallet.aes.json` payload."

### `/results`

- Sticky 4-stat bar at top: `Wallets` · `Keys` · `With value` · `Recovered BTC`. Last two coloured `--color-success` when non-zero.
- Per-wallet cards (clickable, navigate to `/results/:walletId`): filename, source-type pill (`bitcoin_core`, `multibit`, `encrypted`, `wallet_dump`, `bip39_mnemonic`, `blockchain_com`), subtotal row, top 3 address preview (with-balance first, else top 3 by path order).
- Empty state: "No scans this session. Start with [Wallet files] or [Mnemonic]." Links are real CTAs.

### `/results/:walletId`

- Breadcrumb: `Orpheus / Results / wallet.dat`.
- Header card: filename, source-type pill, total keys, keys-with-value, subtotal BTC.
- Inner tabs: `All (n)` · `With value (n)` · `Empty (n)`. Default to `With value` if any, else `All`.
- Table columns: Derivation pill (`BIP44` / `BIP49` / `BIP84` / `Bread`) · Address (mono, truncated with middle ellipsis) · BTC (right-aligned, `tabular-nums`, green when >0) · Txs (right-aligned) · `Open →` link.
- Sort by each column (click header). Search input filters over address + derivation path.

### `/results/:walletId/:address`

Two-column: detail (2fr) · actions (1fr).

**Detail column:**

- Label `Address`, full address in monospace + inline copy button.
- Big balance: 24px `tabular-nums`, green if >0, dim grey if 0.
- Fiat estimate line — rendered only when provider response includes a price; otherwise omitted. No fallback.
- 3-cell stat bar: `Derivation` (mono) · `Total received` · `Transactions`.
- Transactions card: list, each row = `txid` (mono, truncated, click → Blockstream) + date + direction (+/–) + value (green/red, `tabular-nums`) + confirmations. Empty state: "No transactions for this address."

**Actions column (top-to-bottom):**

- QR code card: SVG QR of the address (no network; library: `qrcode-generator`, ~2KB).
- WIF row (sensitive): locked by default, `••••••••••` placeholder. **Reveal** button shows the WIF and starts a 20-second clipboard-auto-clear timer if the user clicks **Copy**. No hover-to-reveal. No auto-reveal on navigation. Uses `@copied-at` timer, not page-level timer (survives re-render).
- CTA stack:
  - `Copy address`
  - `Open in Blockstream ↗` (external link, `rel="noopener noreferrer"`)
  - **`Import this key →`** (primary CTA, green) → navigates to `/import/:keyId`
  - `Export all keys (CSV / descriptor)` — secondary, downloads a file containing every derived key for the current wallet

### `/import/:keyId`

- Amber banner at top: "Handle on an air-gapped machine. Clear clipboard after use. Prefer sweeping funds to a new address once the key is imported."
- Tabs: `Electrum` · `Sparrow` · `Bitcoin Core` · `Hardware wallet` (Coldcard / Jade / Passport).
- Each tab: numbered steps as ordered list, with copyable commands/field values. Each tab ends with the WIF reveal row (same component as `/results/.../:address`).
- Bottom link: "Having trouble? See [`docs/RECOVERY.md`]" (the doc is a follow-up deliverable).

### Global states

- **Loading** (scan running): inline spinner + `Scanned 47 of 103 files`. No blocking modal.
- **Error**: inline banner above the relevant form or section. Red border, neutral surface bg. Includes a `Try again` action where recoverable.
- **Empty after scan**: "No keys extracted. The files may not be supported wallet formats — see the supported list →." Link to docs.

## 7. Backend & core deltas

### `orpheus-core` — balance provider trait

Current method:

```rust
fn balances(&self, addresses: &[&str]) -> Result<Vec<BalanceInfo>>
```

Add:

```rust
fn transactions(&self, address: &str, limit: usize) -> Result<Vec<Tx>>

pub struct Tx {
    pub txid: String,
    pub time: i64,               // unix seconds, confirmed block time when available
    pub value_sat: i64,          // net effect on the query address:
                                 //   positive = received, negative = sent,
                                 //   zero = self-transfer (fee delta only)
    pub fee_sat: Option<u64>,
    pub confirmations: Option<u32>,
    pub block_height: Option<u64>,
}
```

UI renders direction from `value_sat.signum()`; no separate `direction` field on the wire.

Implementations:

- **`BlockstreamProvider`:** `GET /address/:addr/txs` (Esplora). Paginated via `/chain/:last_seen_txid`. Native support.
- **`BlockchainInfoProvider`:** `GET /rawaddr/:addr?limit=50`. Parse into `Tx`.
- **`MockProvider`:** extend `fixtures/mock_balances.json` to include `transactions: [{ txid, time, value_sat, ... }]` per address. Deterministic test values.
- **`NoneProvider`:** returns `Ok(vec![])`.

**Pinned tests per provider** using recorded HTTP fixtures (no live network): recorded response → parsed `Vec<Tx>` → asserted exact values. Same discipline as the rest of the core per `CLAUDE.md`.

### `orpheus-server` — endpoints

| Endpoint                                 | Status              | Notes                                                                         |
|------------------------------------------|---------------------|-------------------------------------------------------------------------------|
| `POST /api/scan`                         | unchanged           | multipart upload, current behavior                                            |
| `POST /api/mnemonic`                     | unchanged           |                                                                               |
| `POST /api/demo`                         | unchanged           |                                                                               |
| `GET /api/address/:address/transactions` | **new**             | query params: `provider`, `limit` (default 50, max 200)                       |
| `POST /api/scan-directory`               | **new, flag-gated** | body `{ path, passwords, provider }`. See `--allow-local-paths` gate below.   |

The `GET /api/address/:address/transactions` endpoint is an auth-free thin wrapper over the provider trait — acceptable because the server binds to `127.0.0.1` only.

`POST /api/scan-directory` is **only mounted** when the server is started with the `--allow-local-paths` flag. When mounted, the path validator enforces:

- Path must be absolute (rejects relative paths, rejects `~`).
- Path must not contain `..` segments (rejects traversal).
- Resolved path must be inside `$HOME` (rejects symlinks that escape home).

There is no opt-out of the home restriction. A user who needs to scan outside `$HOME` uses the CLI directly.

### `orpheus-tauri` — commands

All call directly into `orpheus-core` without HTTP:

```rust
#[tauri::command] async fn scan_directory(path: String, passwords: String, provider: String)
    -> Result<ScanResponse, String>
#[tauri::command] async fn mnemonic(payload: MnemonicRequest)
    -> Result<MnemonicResponse, String>
#[tauri::command] async fn address_transactions(address: String, provider: String, limit: usize)
    -> Result<Vec<Tx>, String>
```

Web client detects Tauri via `window.__TAURI_INTERNALS__` and routes through these. A single adapter layer in `apps/web/src/lib/api.ts` hides the HTTP-vs-Tauri split so route components stay agnostic.

### Provider list in sync

Per `CLAUDE.md`: any provider change must land in the same PR across **three surfaces** — `orpheus-core::VALID_PROVIDERS`, clap `CliProvider`, frontend `<select>`. This redesign does not add or remove providers, but the `tauri-invoke` boundary becomes a fourth sync point for future changes.

## 8. File & component structure

```text
apps/web/src/
├── App.tsx                   — router shell
├── main.tsx                  — unchanged
├── index.css                 — @theme tokens + base reset only (no component CSS)
├── types.ts                  — extended with Tx, KeyId, WalletId
│
├── routes/
│   ├── ScanPage.tsx
│   ├── MnemonicPage.tsx
│   ├── ResultsPage.tsx
│   ├── WalletPage.tsx
│   ├── AddressPage.tsx
│   └── ImportPage.tsx
│
├── components/
│   ├── layout/
│   │   ├── Header.tsx        — brand + breadcrumbs + provider indicator
│   │   ├── Tabs.tsx
│   │   └── Breadcrumbs.tsx
│   ├── ui/                   — primitives (source of truth for visual style)
│   │   ├── Button.tsx
│   │   ├── Input.tsx
│   │   ├── Textarea.tsx
│   │   ├── Select.tsx
│   │   ├── Card.tsx
│   │   ├── Table.tsx
│   │   ├── Pill.tsx          — BIP44/49/84/Bread + source-type badges
│   │   ├── StatBar.tsx
│   │   ├── CopyButton.tsx
│   │   ├── WifReveal.tsx     — locked → reveal + auto-clear clipboard
│   │   ├── QRCode.tsx        — SVG, no network
│   │   ├── Spinner.tsx
│   │   └── Banner.tsx
│   ├── scan/
│   │   ├── FolderPicker.tsx  — runtime-detects Tauri / FS Access / fallback
│   │   ├── FileDropzone.tsx  — fallback
│   │   └── RecentScans.tsx
│   ├── results/
│   │   ├── WalletCard.tsx
│   │   ├── KeyTable.tsx
│   │   ├── TxTable.tsx
│   │   └── AddressDetail.tsx
│   └── import/
│       ├── ImportTabs.tsx
│       ├── ElectrumSteps.tsx
│       ├── SparrowSteps.tsx
│       ├── BitcoinCoreSteps.tsx
│       └── HardwareSteps.tsx
│
├── lib/
│   ├── api.ts                — Tauri/HTTP adapter (single interface)
│   ├── storage.ts            — sessionStorage wrapper, schema version
│   ├── ids.ts                — walletId/keyId derivation
│   ├── clipboard.ts          — copy + auto-clear timer
│   ├── fs-access.ts          — File System Access API walker
│   ├── tauri.ts              — feature detect + invoke wrappers
│   └── utils.ts              — cn, satToBtc, truncateMiddle, formatBytes
```

### Deletions

Rewritten structurally, so the following files are deleted rather than edited:

- `apps/web/src/components/Descent.tsx`
- `apps/web/src/components/Masthead.tsx`
- `apps/web/src/components/Dropzone.tsx`
- `apps/web/src/components/Field.tsx`
- `apps/web/src/components/ResultsView.tsx`

### New dependencies (web)

- `react-router-dom` — client-side routing.
- `qrcode-generator` — 2KB, SVG output, no network.
- `@tauri-apps/plugin-dialog` — Tauri only, folder picker.
- `@testing-library/react`, `@testing-library/user-event`, `@testing-library/jest-dom` — dev-only, component tests.

No component library. All UI primitives hand-rolled against Tailwind v4.

## 9. Testing

- **UI primitives** (`components/ui/`): unit tests with Testing Library. Focus: `WifReveal` (clipboard + auto-clear timing), `CopyButton`, `QRCode` (renders correct SVG for a known address), `Table` (sort / filter behavior), `Banner` (variants).
- **Route-level integration**: one test per route rendering with a mocked `api.ts`, asserts key content and CTAs. Not visual/screenshot tests.
- **`lib/`**: `ids.ts` (hash stability + known-input pinned values), `storage.ts` (schema version + mismatch discard), `fs-access.ts` (walker given a mock directory handle), `tauri.ts` (feature-detect matrix), `clipboard.ts` (auto-clear timing).
- **Provider tx-parsing** (Rust): pinned fixture per provider. Recorded HTTP → `Vec<Tx>` → exact-value assertions. `MockProvider` fixture extension covered by extending the existing mock-balances test.
- **`scan-directory` path validator** (Rust): unit tests for absolute-path requirement, `..` traversal rejection, symlink-to-outside-home rejection.

Per project rules: no real wallets, real mnemonics, or real passwords in any new test. Fixtures via `orpheus-demo-fixtures` or inline generation.

## 10. Accessibility

- All interactive elements keyboard-reachable. Visible 2px focus ring on every focusable control.
- Tabs: `role="tablist"` / `role="tab"` / `aria-selected` / arrow-key navigation.
- Tables: `<th scope="col">`; sort controls announce state with `aria-sort`.
- WIF row hidden state: `aria-label="Private key (hidden — activate reveal to show)"`.
- Color contrast: both themes hit WCAG AA for body text, UI text, and CTA buttons. Verified in CI via a contrast check (added to `mise run ci`).
- Respect `prefers-reduced-motion`.
- Route transitions: `<main>` receives focus on route change; breadcrumb changes are announced via `aria-live="polite"`.

## 11. Explicit punts (follow-ups, not this PR)

- **Fiat price feed.** Render-only when already present in a provider response.
- **Per-transaction detail view inside Orpheus.** Click a tx → Blockstream.
- **Command palette (⌘K).**
- **Settings page.** Provider lives in the header popover.
- **Sweep / sign / broadcast.** Hard line — out of scope forever, not just this PR.
- **`docs/RECOVERY.md`.** Referenced by the Import page footer; a placeholder link is acceptable in the first merge.

## 12. Ship order

The implementation plan (next artifact) will be a checklist; this is the dependency order:

1. Worktree + branch + `.gitignore` entry for `.superpowers/`. *(done — this branch)*
2. Tokens + base CSS reset — fixes the broken-vars bug; everything else depends on this.
3. UI primitives (Button / Input / Textarea / Select / Card / Table / Pill / StatBar / CopyButton / WifReveal / QRCode / Spinner / Banner).
4. Router + layout shell (Header / Tabs / Breadcrumbs).
5. `ScanPage` + `FolderPicker` + FS Access walker + Tauri adapter layer.
6. `MnemonicPage`.
7. **Backend work:** provider trait `transactions()` + server endpoints + Tauri commands + pinned fixture tests.
8. `ResultsPage` / `WalletPage` / `AddressPage`.
9. `ImportPage` + per-wallet instruction components.
10. A11y + contrast audit + test coverage + delete old components.
11. Manual smoke: `mise run server:dev` in Chrome + Firefox; `mise run tauri:dev` on macOS.

## 13. Open questions

None blocking. Follow-ups tracked in §11.
