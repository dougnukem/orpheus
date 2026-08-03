# `orpheus hunt` — sweeping a whole machine

`orpheus scan <dir>` assumes you know where the wallet is. `orpheus hunt`
is for when you don't: fifteen years of Dropbox, a folder you don't
remember, a filename that doesn't end in `.dat`.

```bash
orpheus hunt all --root ~ --passwords ~/passwords.txt
```

That runs five stages and leaves a report behind. Everything lands in
`~/.orpheus/hunt/<run-id>/`, mode 0700 — **never inside a repository**,
because it contains private keys.

## Why not just `scan ~`?

Two reasons, both of which cost you wallets.

**Detection is filename-driven.** `Extractor::can_handle` checks the
extension before it looks at any bytes. A real machine holds Bitcoin Core
backups named `bitcoin_1776391129.legacy.bak`, `wallet.dat.backup`, and
`wallet.dat-journal`. None of them reach the Bitcoin Core extractor.
`hunt` sniffs magic bytes first and dispatches on what a file *is*.

**Cost is unbounded.** `can_handle` calls `read_to_string` on every `.txt`
and `.json` under the root. Pointed at `$HOME` that is millions of files.
`hunt` prunes build noise and caches before it opens anything.

## The five stages

```text
discover ──► triage ──► extract ──► enrich ──► report
(offline)   (offline)   (offline)   (network)  (offline)
```

`discover`, `triage`, and `extract` never open a socket. Only `enrich`
talks to the network, and it is separately invocable — so you can extract
on an air-gapped machine, copy the addresses off, and check balances
somewhere else.

```bash
orpheus hunt discover --root ~/Dropbox      # inventory candidates
orpheus hunt extract  --passwords pw.txt    # offline key recovery
orpheus hunt enrich   --provider blockstream
orpheus hunt report   [--unredact]
orpheus hunt runs                           # list past runs
```

Each stage defaults to the most recent run; pass `--run-id` to pick one.

## What discovery looks for

| Signal | Example |
|---|---|
| Magic bytes | BDB btree `0x00053162` at offset 12 → Bitcoin Core legacy wallet |
| | `SQLite format 3` + a `main` table → descriptor wallet |
| | `org.bitcoin.production` protobuf → MultiBit Classic |
| | `mkey`/`ckey` records → **encrypted** Bitcoin Core wallet |
| Filename | `wallet.dat*`, `*.wallet`, `*.aes.json`, `*mnemonic*`, `passwords*.txt` |
| Text content | a WIF that passes base58check, `xprv`/`zprv`, a valid BIP39 phrase |

Candidates are sorted into tiers:

| Tier | Meaning | What happens |
|---|---|---|
| A | identified wallet container | extracted |
| B | text carrying key or seed material | extracted |
| C | archive or encrypted container | reported, **not opened** |
| D | contextual — sits in a crypto-named folder | reported for review |

`.Trash` is deliberately **not** pruned. A deleted wallet is still a wallet.

**Cloud storage is pruned by default.** On macOS `~/Library/CloudStorage/`
(Google Drive, OneDrive, the modern Dropbox client) holds *online-only
placeholders*. Reading a file's head to sniff it forces the provider to
download the whole file, so an unguarded sweep would quietly pull your
entire cloud drive over the network. "This computer" means bytes on local
disk. To scan a specific cloud folder anyway, point `--root` straight at
it and accept the download cost. The **classic local `~/Dropbox` folder is
not affected** — it isn't under `CloudStorage` and its files are real on
disk, so it's swept normally.

### How long it takes

Discovery reads the head of every plausibly-relevant file and fully reads
every text file under 8 MB to check it for key material. On a developer's
`$HOME` with a large `~/.cargo`, `~/go/pkg/mod`, and years of documents,
**a first full sweep takes tens of minutes**. Run it in the background.

The prune list stays deliberately conservative — a wallet in an odd
directory is exactly what this tool exists to find, so breadth wins over
speed. Narrow the scope with `--root` when you know roughly where to look:

```bash
orpheus hunt discover --root ~/Dropbox --root ~/Documents
```

## Deduplication

Wallets get copied. `~/Dropbox/bitcoin/wallet.dat` and
`~/Dropbox/BACKUP bitcoin/wallet.dat` are frequently byte-identical.
Candidates are grouped by SHA-256, extraction runs once per distinct
digest, and the report lists every path sharing it. That is what keeps
"how many wallets do I have" honest.

## The recovery-attempt ledger

`attempts.jsonl` records every attempt — success or not — keyed on content
digest:

```json
{"ts":"2026-08-02T19:51:00Z","digest":"ab12…","format":"bitcoin_core_encrypted",
 "passwords_tried":6,"outcome":"needs_password","keys_found":0}
```

This is what makes the hunt repeatable rather than a one-shot script. A
rerun skips digests that already gave up their keys and retries the ones
that didn't. When you come back in six months with two more passwords you
remember, you can see exactly what you already tried.

Force a retry of solved artifacts with `--retry-solved`.

## Encrypted wallets

Orpheus opens two encryption schemes:

- **Bitcoin Core** (`mkey`/`ckey`) — iterated SHA-512 key stretching, then
  AES-256-CBC. Works against both BDB and SQLite wallets. Every decrypted
  key is verified by re-deriving its public key, so a wrong passphrase
  fails loudly instead of producing plausible garbage.
- **MultiBit Classic v3** — scrypt + AES-256-CBC.

An encrypted wallet that no supplied password opens is reported as
`needs_password`, **not** as empty. That distinction is the whole point:
a locked wallet with funds and an empty wallet look identical if you
collapse them.

If the password list doesn't work, extend it and rerun `extract`. For
serious password search, see [`password-recovery.md`](password-recovery.md)
and point btcrecover at the wallet.

## Unknown versus empty

A balance lookup that fails — rate limit, network error — is recorded as a
**failed lookup**, not as a zero balance. Failed lookups get their own
section in the report. Rerun `enrich` before concluding anything about
those addresses. Reporting "we don't know" as "0.00000000 BTC" is how a
funded address gets written off.

## The vault

```text
~/.orpheus/hunt/<run-id>/
├── inventory.jsonl        candidates: path, tier, format, digest
├── attempts.jsonl         append-only attempt ledger
├── keys.jsonl             SENSITIVE — recovered private keys
├── balances.jsonl         per-address balance
├── transactions.jsonl     per-address transaction history
├── lookup_failures.jsonl  addresses we could not check
└── report.md              redacted human report
```

The report redacts WIFs to `L1aW…8xQz` by default. Full keys live in
`keys.jsonl`, or render with `--unredact` when you're ready to sweep.

## Safety

- Read-only against everything it scans. Nothing is written under a root.
- The vault is 0700, its files 0600, and `.orpheus/` is gitignored.
- Balance lookups transmit **addresses only**, never keys. Use
  `--provider none` to stay fully offline.
- If you find funds: sweep them immediately. A key that sat in Dropbox for
  a decade should be treated as compromised.
- Clear your scrollback after running with `--unredact`.

## Known limitations

- Archives and disk images (Tier C) are flagged, not opened. Unpack them
  yourself and rerun the hunt against the extracted directory.
- Bitcoin Core **descriptor** wallets yield keys only via their
  `listdescriptors` output or their `ckey` records; descriptor parsing
  from a SQLite wallet is not implemented.
- No OCR of paper-wallet photos or PDF recovery sheets.
- No Keychain, browser-extension, or Apple Notes extraction.
- Password search is limited to the list you supply.
