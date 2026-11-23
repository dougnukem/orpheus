---
name: wallet-hunt
description: Sweep a machine (or Dropbox, or a backup drive) for Bitcoin wallet artifacts, recover keys offline, then report balances and transaction history. Use when the user asks to find old or forgotten wallets, search a disk for wallet.dat / seed phrases / private keys, check whether any recovered address holds funds, or resume a previous recovery attempt. Triggers on "find my old bitcoin", "search for wallets", "did I have any BTC", "check these old backups", "resume the wallet recovery".
---

# Hunting for forgotten Bitcoin wallets

Drive `orpheus hunt`, which sweeps for wallet artifacts by **content**
rather than filename, recovers keys offline, and keeps an append-only
ledger so a recovery can span months.

Read [`docs/wallet-hunt.md`](../../../docs/wallet-hunt.md) for the full
reference. This skill is the operating procedure.

## Ground rules

These are not negotiable and they override any instinct to be helpful with
output:

1. **Never print a private key, WIF, seed phrase, or password** into the
   conversation, a commit, a PR, or a file inside the repo. Report counts,
   formats, addresses, and balances. The keys stay in the vault.
2. **Never commit anything a hunt produced.** The vault lives at
   `~/.orpheus/hunt/<run-id>/`. `.orpheus/` is gitignored; keep it that way.
3. **Extraction is offline. Enrichment is not.** Balance lookups send
   addresses to a third party. Confirm before the first networked run
   unless the user has already said to go ahead.
4. **"Locked" is not "empty" and "unknown" is not "zero".** When
   summarising, keep encrypted wallets and failed lookups visible.
   Collapsing them is how someone concludes they have nothing.

## Procedure

### 1. Build

```bash
mise run build:release      # or: cargo build --release -p orpheus-cli
```

### 2. Find the password list before you start

Old recoveries leave one behind. Check for `passwords*.txt` near any
wallet folder — a prior session's list is the highest-value input you
have, and rerunning without it wastes the whole extract stage.

```bash
ls ~/Dropbox/**/passwords*.txt 2>/dev/null
orpheus hunt runs           # is there already a run to resume?
```

If a previous run exists, prefer resuming it — the ledger will skip
everything already solved.

### 3. Discover (offline)

```bash
orpheus hunt discover --root ~ --root /Volumes/SomeDrive
```

Scope notes:
- `/Volumes/Macintosh HD` is a firmlink to the root volume and re-walks
  `$HOME`. Skip it.
- Paths with spaces need a bash array, not a flat `$VARS` string.
- Whole-machine sweeps of `/` need Full Disk Access for the terminal.

Report the inventory back as counts by tier and format — not as a file
listing.

### 4. Extract (offline)

```bash
orpheus hunt extract --passwords ~/Dropbox/bitcoin/passwords.txt
```

Add `--retry-solved` only when re-attempting artifacts the ledger already
records as solved.

### 5. Enrich (network — confirm first)

```bash
orpheus hunt enrich --provider blockstream
```

This is sequential and rate-limited; a few thousand addresses takes a
while. Run it in the background and check back rather than blocking.

Use `--no-transactions` to skip per-address history, or
`--provider none` to stay fully offline.

### 6. Report

```bash
orpheus hunt report          # redacted
orpheus hunt report --unredact   # only when the user is ready to sweep
```

Summarise for the user:
- wallets found, by format, deduped by content digest
- keys recovered, unique addresses
- **funded addresses and total balance**
- addresses with history but no remaining balance
- wallets still locked, and how many passwords were tried
- failed lookups that need a rerun
- Tier C archives worth unpacking by hand

## When a wallet stays locked

Say so plainly, with the number of encrypted keys inside it — that is the
measure of what is at stake. Then offer the actual next steps:

- extend the password list with variants and rerun `extract`
- point btcrecover at it (see `docs/password-recovery.md`) for rule-based
  mutation and GPU search

Do not attempt a brute-force sweep from inside this skill.

## If funds are found

Tell the user the balance and the address. Then, before anything else:
sweep the funds to a wallet they control. A key that sat in cloud storage
for a decade should be assumed compromised. Import instructions for
Electrum and Sparrow are in the generated report.
