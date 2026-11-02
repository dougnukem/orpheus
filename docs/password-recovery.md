# Recovering a wallet password

`orpheus hunt` tries the passwords you give it and stops. It does not
brute-force. When a wallet stays locked — `needs_password` in the report —
this is how to go further.

Orpheus already tells you exactly what you're up against. For each locked
wallet the report lists its format and how many encrypted keys are inside.
That number is the size of the prize; it's what's behind the passphrase.

## Step 1 — think before you compute

A passphrase you *almost* remember is worth a thousand GPU-hours. Before
reaching for a cracker, write down everything you recall:

- fragments you're sure of ("it had my usual base word and a year in it")
- your password habits from that era — a base word, a capital, a number, a
  symbol at the end
- the years the wallet was active (the hunt report's file timestamps and
  transaction dates bracket this)

Old Bitcoin passphrases are usually *human* passwords from 2011–2016, not
random strings. That is what makes them recoverable.

## Step 2 — extend the list and rerun

The cheapest thing that can work: add candidates to your password file and
run extract again. The ledger skips everything already solved, so this
costs only the locked wallets.

```bash
orpheus hunt extract --passwords ~/passwords-v2.txt
orpheus hunt report
```

Generate variants of a base word rather than typing them by hand:

```bash
# capitalisation, common suffixes, leet — a few thousand candidates
python3 - <<'PY' > ~/passwords-v2.txt
import itertools
bases = ["hunter2", "Hunter2"]  # your remembered base word(s), not committed
suffixes = ["", "1", "123", "!", "!1", "#1"] + [str(y) for y in range(2011, 2017)]
leet = str.maketrans({"a":"@","e":"3","o":"0","s":"$"})
for b in bases:
    for s in suffixes:
        for w in {b, b.translate(leet)}:
            print(w + s)
PY
```

## Step 3 — hand it to btcrecover

For anything beyond a small list, [btcrecover](https://github.com/gurnec/btcrecover)
is the right tool. It is purpose-built for Bitcoin wallet passwords, is
typo-tolerant, and drives the GPU. Orpheus finds and identifies the
wallet; btcrecover searches its passphrase.

btcrecover reads the same wallet files Orpheus does — point it at the exact
path from the hunt report:

```bash
# token list: fragments you remember, in any order, some optional
python3 btcrecover.py --wallet ~/Dropbox/bitcoin/wallet.dat \
  --tokenlist tokens.txt --typos 2 --typos-capslock --typos-case
```

A `tokens.txt` for "it had my base word and some numbers, maybe a bang"
(put your real base word here in the local file — never commit it):

```text
hunter2
+ 2013 2014 2015
+ ! 1 123
```

The `+` marks a line as required. btcrecover permutes the rest.

btcrecover speaks both formats a hunt surfaces as locked:

- **Bitcoin Core** (`bitcoin_core_encrypted`) — pass the `wallet.dat`
  directly. Its `--dsw` extract tooling also works if you'd rather not
  expose the whole file.
- **MultiBit Classic** (`multibit_encrypted`) — pass the `.wallet` (or its
  extracted key block).

GPU acceleration (`--enable-gpu` with OpenCL) turns hours into minutes for
a large keyspace. See btcrecover's docs for setup.

## Step 4 — when it opens

btcrecover prints the passphrase. Feed it back to Orpheus rather than
retyping keys by hand:

```bash
echo 'theRecoveredPassphrase' >> ~/passwords-v2.txt
orpheus hunt extract --passwords ~/passwords-v2.txt
orpheus hunt report --unredact
```

The report now shows the recovered WIFs. Then, immediately: sweep the
funds. A key that sat in cloud storage for a decade should be treated as
compromised — move the coins to a wallet you control before doing anything
else. Import instructions for Electrum and Sparrow are in the report.

## What not to do

- **Never paste the wallet or its passphrase into a website** offering to
  "recover" it. They keep your keys.
- **Never run an untrusted build against a real wallet.** Audit or pin the
  commit first (see the Security section of the README).
- **Don't burn GPU-days on random-string assumptions.** If the passphrase
  was human, a rule-based search over your own habits finds it far faster
  than raw brute force.
