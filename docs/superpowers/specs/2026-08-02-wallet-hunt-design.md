# `orpheus hunt` — machine-wide wallet artifact hunt

> Design spec — 2026-08-02
>
> Sweep a whole machine (not one hand-picked directory) for Bitcoin wallet
> artifacts, extract offline, enrich with balances and transaction history,
> and keep a durable ledger of every recovery attempt.

## Problem

`orpheus scan <dir>` assumes you already know where the wallet is. Real
recovery doesn't work that way: the wallet is somewhere in fifteen years of
Dropbox, in a folder you don't remember, under a filename that doesn't end
in `.dat`.

Two concrete failures of `scan` when pointed at a real machine:

1. **Detection is filename-driven.** `Extractor::can_handle` gates on
   extension before it looks at bytes — `.dat`, `.wallet`, `.bak`, `.txt`,
   `.json`, `.dump`, `.aes.json`, or no extension. A real `$HOME` contains
   Bitcoin Core wallet backups named `bitcoin_1776391129.legacy.bak`,
   `wallet.dat.backup`, `wallet.dat-journal`, and `__db.001`. Every one of
   those is skipped today. `.legacy.bak` reaches the MultiBit extractor,
   which then rejects it for lacking the `org.bitcoin` magic, and no other
   extractor is offered the file.
2. **Cost is unbounded.** `bip39_mnemonic::can_handle` and
   `wallet_dump::can_handle` call `read_to_string` on *every* `.txt` and
   `.json` under the root. Pointed at `$HOME` that is millions of files and
   hundreds of gigabytes of JSON.

Neither is a bug in `scan` — `scan` is a directory tool. Hunting a machine
is a different job and needs a different front end.

## Approach

Add a discovery-and-triage layer in front of the existing extractors, and a
`hunt` subcommand that drives it as a five-stage pipeline. The extractors,
crypto, and balance code are untouched sources of truth; `hunt` only changes
*which files reach them* and *what gets recorded*.

```text
discover ──► triage ──► extract ──► enrich ──► report
(offline)   (offline)   (offline)   (network)  (offline)
```

The `discover → triage → extract` prefix never opens a socket. `enrich` is
the only networked stage and is separately invocable, so the "extract on an
air-gapped box, check balances elsewhere" workflow from `docs/security.md`
stays available.

### Why not just widen `can_handle`?

Because `can_handle` is a *cheap* predicate that runs per-extractor
per-file, and the fix requires reading magic bytes and maintaining a prune
list. Pushing that into every extractor would make `scan` slower and couple
the extractors to filesystem policy. Discovery does the sniffing once, in
one place, and hands each extractor a file it already knows the format of.

## Stage 1 — discover

New module `orpheus-core::discovery`.

Walks a set of roots and emits `Candidate` records. Three independent
signals promote a file to candidate:

**Filename patterns** — `wallet.dat*`, `*.wallet`, `*.aes.json`,
`*.legacy.bak`, `default_wallet`, `*mnemonic*`, `passwords*.txt`, and
friends.

**Content magic** — the discriminator that filename matching misses:

| Bytes | Format |
|---|---|
| `0x00053162` at offset 12 (either endianness) | Berkeley DB — Bitcoin Core legacy wallet |
| `SQLite format 3\0` at offset 0, with a `main` table | Bitcoin Core descriptor wallet |
| `0x0a` + `org.bitcoin.production` | MultiBit Classic protobuf |
| `# Wallet dump created by Bitcoin` | Bitcoin Core `dumpwallet` |
| `BITCOIN_CORE_WALLET_DUMP,` | prior-session dump format (see Prior art) |
| JSON with `payload` + `pbkdf2_iterations` | blockchain.com `wallet.aes.json` |
| JSON with `wallets[].desc` | `listdescriptors` output |

**Text content signatures** — for files under 8 MB with text-ish
extensions: mainnet WIF (`[5KL][1-9A-HJ-NP-Za-km-z]{50,51}`), extended
private keys (`xprv`/`yprv`/`zprv`/`tprv`), extended public keys, and a
valid BIP39 word sequence of 12/15/18/21/24 words checked against the
English wordlist.

Each candidate is assigned a tier that determines what happens next:

| Tier | Meaning | Next stage |
|---|---|---|
| `A` | Known wallet container, format identified | extract |
| `B` | Text carrying key or seed material | extract |
| `C` | Archive or encrypted container | reported, not opened |
| `D` | Contextual — lives in a crypto-named directory but is not itself identified | reported for human review |

Tier C is deliberately not unpacked. Unpacking archives writes decrypted
wallet material to scratch disk, and the scope decision for this run was to
flag them instead. They appear in the report with a suggested command.

**Pruning.** `node_modules`, `.git`, `target`, `Caches`, `DerivedData`,
`CoreSimulator`, `.rustup`, `.cargo/registry`, `OrbStack`, and friends.
`.Trash` is deliberately *not* pruned — a deleted wallet is still a wallet.

Output: `inventory.jsonl`, one `Candidate` per line — path, size, mtime,
birthtime, sha256, tier, detected format, and the signals that fired.

### Deduplication

Wallets get copied. `~/Dropbox/bitcoin/wallet.dat` and
`~/Dropbox/BACKUP bitcoin/wallet.dat` may be byte-identical. Candidates are
grouped by sha256; extraction runs once per distinct digest and the report
lists every path that shares it. This is what keeps the report honest about
"how many wallets do I actually have" versus "how many copies".

## Stage 2 — triage

Resolve each Tier A/B candidate to a concrete `SourceType` plus an
`encrypted: bool`, and pick the extractor by *detected format* rather than
by asking each extractor's `can_handle`. This is the step that lets
`bitcoin_1776391129.legacy.bak` reach the Bitcoin Core extractor.

`can_handle` stays exactly as it is and `scan` keeps using it. `hunt`
dispatches through a new `extractor_for_format(DetectedFormat) -> &dyn
Extractor` map.

## Stage 3 — extract (offline)

Run the selected extractor per distinct digest with `--provider none`.
Password-protected candidates get the supplied `--passwords` list.

Every attempt — success or failure — appends to the ledger (below).

Output: `keys.jsonl` in the vault. Contains WIFs. Mode 0600.

## Stage 4 — enrich (network)

For every unique address across all recovered keys:

- balance / total received / total sent / tx count via the chosen provider
- for any address with `tx_count > 0`, the full transaction list

Transaction history is new. `BalanceProvider` gains an optional
`transactions(&self, address) -> Result<Vec<TxRecord>>` with a default
implementation returning `Ok(vec![])`, so existing providers and the
`MockProvider` test seam keep compiling. `BlockstreamProvider` implements it
against `/address/:addr/txs`.

`TxRecord`: txid, block height, block time, confirmed, net value to this
address in sats, and fee.

Output: `balances.jsonl`, `transactions.jsonl`.

## Stage 5 — report

A redacted Markdown report plus the raw JSON.

- Inventory: candidate counts by tier, by detected format, by root
- Duplicates collapsed, with every path listed per digest
- Extraction outcomes per wallet, including failures and their reasons
- Addresses split funded / spent-but-empty / never-used
- Per-address transaction history with dates and amounts
- Totals: ever received, current balance
- Recovery-attempt ledger: what was tried, when, and what happened
- Follow-ups: Tier C archives and encrypted wallets that need passwords

**Redaction is the default.** WIFs render as `L1aW…8xQz`. Full keys live
only in `keys.jsonl` inside the vault. `--unredact` prints them for the
operator's own eyes.

## The recovery-attempt ledger

`attempts.jsonl`, append-only, one record per (digest, extractor, password
set) attempt:

```json
{"ts":"2026-08-02T19:51:00Z","digest":"ab12…","path":"~/Dropbox/…","format":"bitcoin_core_bdb",
 "extractor":"bitcoin_core","passwords_tried":5,"outcome":"success","keys_found":102,"error":null}
```

This is what makes the workflow *repeatable* rather than a one-shot script.
A second run skips digests that already succeeded, retries the ones that
failed with a new password list, and the report can answer "what have I
already tried on this file?" — which is the question that actually matters
when a recovery spans months.

## Vault layout

Everything the hunt produces lands outside the repo, in
`~/.orpheus/hunt/<run-id>/` (dir 0700, files 0600):

```text
inventory.jsonl     candidates + tiers + digests
attempts.jsonl      append-only recovery-attempt ledger
keys.jsonl          extracted keys (SENSITIVE — contains WIFs)
balances.jsonl      per-address balance
transactions.jsonl  per-address tx history
report.md           redacted human report
```

Nothing from a hunt is ever written into the repo. `.gitignore` gains
`.orpheus/` as a belt-and-braces guard against a vault created with a
relative path.

## CLI

```bash
orpheus hunt all       --roots ~ --roots /Volumes  --passwords pw.txt
orpheus hunt discover  --roots ~/Dropbox --json
orpheus hunt extract   --run-id <id> --passwords pw.txt
orpheus hunt enrich    --run-id <id> --provider blockstream
orpheus hunt report    --run-id <id> [--unredact]
```

`--roots` repeats. Defaults to `$HOME`. `hunt all` runs the five stages in
order and is the normal entry point.

## Prior art — the 2015/2016 recovery

`~/Dropbox/bitcoin/` is the working directory of the recovery that seeded
this project, and the hunt should build on it rather than rediscover it:

- `passwords.txt` — 5 candidate passwords that were tried before. `hunt`
  should be pointed at this file by default when it exists.
- `dump.txt` — a `BITCOIN_CORE_WALLET_DUMP,1` export. Custom format from
  that session; discovery recognises the header.
- `balance_results.json` — the prior report. Its schema (`summary`,
  `funded_addresses`, `used_addresses`) is the shape this report supersedes.
- Five Python extractors that encode what worked: BDB key extraction,
  MultiBit protobuf, MultiBit v3 scrypt+AES decryption, blockchain.com
  mnemonic decoding.

The Rust extractors already reimplement all of these. What was missing is
the *finding* step, which is this spec.

## Testing

Per `CLAUDE.md`, anything touching crypto/extractors/balance needs a test
pinned to a known value.

- Magic-byte detection: one test per format, using byte literals for the
  headers. BDB magic asserted in both endiannesses.
- WIF regex: true-positive against a fixed known WIF, false-positive
  against a same-length base58 string that isn't a WIF.
- BIP39 text detection: true-positive on the standard all-`abandon` test
  vector, false-positive on 12 English words that aren't in the wordlist.
- Prune list: a temp tree with `node_modules/wallet.dat` asserts zero
  candidates; the same file outside `node_modules` asserts one.
- Dedup: two byte-identical files, one digest, one extraction.
- Ledger: append two attempts, read back two records, assert ordering.
- `TxRecord` parsing: a recorded Blockstream `/txs` JSON payload as a
  fixture, asserting parsed txid / height / net value. No network in tests.

## Out of scope

- Unpacking archives and disk images (Tier C is flagged, not opened)
- OCR of paper-wallet photos and PDF recovery sheets
- Keychain, browser-extension, and Apple Notes extraction
- Password cracking beyond a supplied list — `docs/password-recovery.md`
  already points at btcrecover for that
- Altcoins
