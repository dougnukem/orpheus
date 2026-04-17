# Orpheus - Design Spec

> **Orpheus** — Recover lost cryptocurrency from forgotten wallets
>
> *Descended into the underworld to recover what was lost.*

## Overview

A pip-installable Python CLI + local web UI tool for scanning, extracting, and checking balances of old cryptocurrency wallets. Starts with Bitcoin wallet support (2012-2016 era), designed to extend to other cryptocurrencies.

**Key principle:** All processing happens locally. Private keys never leave the user's machine. Network calls are limited to public address balance lookups (which only transmit public addresses, not keys).

## Target Users

People who have old Bitcoin wallet files, seed phrases, or recovery mnemonics from early Bitcoin (2012-2016) and want to determine if any associated addresses hold funds, then recover those funds using modern wallet software.

## Supported Wallet Types

| Type | File/Input | Description |
|---|---|---|
| Bitcoin Core / Qt | `.dat` (Berkeley DB) | Extract EC private keys from BDB key-value store |
| MultiBit Classic | `.wallet` (protobuf) | Extract keys from bitcoinj serialized wallets |
| Blockchain.com legacy | Mnemonic phrase (6-25 words) | Decode V2/V3/V4 mnemonics to wallet GUID + password |
| BIP39 seeds | 12/15/18/21/24 word mnemonic | Derive keys via BIP32/44 paths (Breadwallet, Electrum v2+, etc.) |
| Bitcoin Core dumps | JSON descriptor dumps | Parse WIF keys from `listdescriptors` output |
| Encrypted wallets | Any of the above + password list | Attempt decryption using user-provided passwords |

## Architecture

```
orpheus/
├── pyproject.toml
├── Dockerfile
├── README.md
├── docs/
│   ├── security.md             # Security best practices
│   └── password-recovery.md    # Password recovery techniques guide
├── src/
│   └── orpheus/
│       ├── __init__.py
│       ├── __main__.py         # python -m orpheus
│       ├── cli.py              # CLI entry point (click)
│       ├── scanner.py          # Directory scanning - finds wallet files
│       ├── extractors/
│       │   ├── __init__.py
│       │   ├── base.py         # Abstract extractor interface
│       │   ├── bitcoin_core.py # BDB wallet.dat key extraction
│       │   ├── multibit.py     # MultiBit Classic protobuf wallets
│       │   ├── blockchain_com.py # Legacy mnemonic decoder (V2/V3/V4)
│       │   ├── bip39.py        # BIP39 mnemonic + BIP32/44 derivation
│       │   ├── wallet_dump.py  # Bitcoin Core JSON descriptor dumps
│       │   └── encrypted.py    # Password-list decryption coordinator
│       ├── crypto/
│       │   ├── __init__.py
│       │   ├── keys.py         # WIF encode/decode, pubkey derivation, address generation
│       │   ├── bip32.py        # HD key derivation (BIP32)
│       │   └── bech32.py       # Bech32/Bech32m address encoding
│       ├── balance.py          # Public API balance checker
│       ├── models.py           # Data classes
│       └── web/
│           ├── __init__.py
│           ├── app.py          # Flask app + API routes
│           ├── templates/
│           │   └── index.html  # Single-page UI
│           └── static/
│               ├── style.css
│               └── app.js
└── tests/
    ├── test_extractors/
    ├── test_crypto/
    └── fixtures/               # Non-sensitive test wallet files
```

## CLI Interface

```bash
# Install
uvx orpheus

# Or install into project
uv pip install orpheus

# Scan a directory for wallet files, extract keys, check balances
orpheus scan /path/to/wallets

# Extract keys from a specific wallet file
orpheus extract wallet.dat

# Extract with password list for encrypted wallets
orpheus extract wallet.dat --passwords passwords.txt

# Decode a blockchain.com legacy mnemonic
orpheus decode-mnemonic "bought purple insane contended ..."

# Derive keys from BIP39 seed phrase
orpheus derive-bip39 "excuse abandon plug ..." --path "m/0'/0/0"

# Derive with common Breadwallet paths
orpheus derive-bip39 "excuse abandon plug ..." --wallet-type breadwallet

# Check balances for previously extracted keys
orpheus check-balance results.json

# Launch local web UI
orpheus serve --port 8080

# Offline mode - extract only, no network calls
orpheus scan /path --offline

# Ephemeral Docker mode
orpheus docker-run /path/to/wallets
```

## Extractor Interface

Each wallet type extractor implements a common interface:

```python
from dataclasses import dataclass
from pathlib import Path
from abc import ABC, abstractmethod

class BaseExtractor(ABC):
    """Base class for all wallet extractors."""

    @staticmethod
    @abstractmethod
    def can_handle(file_path: Path) -> bool:
        """Return True if this extractor can process the given file."""
        ...

    @abstractmethod
    def extract_keys(
        self,
        file_path: Path,
        passwords: list[str] | None = None,
    ) -> list[ExtractedKey]:
        """Extract private keys from the wallet file."""
        ...
```

The scanner auto-detects wallet types by calling `can_handle()` on each extractor. For mnemonic-based recovery (blockchain.com, BIP39), the CLI and web UI provide direct entry points since there's no file to scan.

## Data Models

```python
@dataclass
class ExtractedKey:
    wif: str
    address_compressed: str
    address_uncompressed: str
    address_p2sh_segwit: str | None
    address_bech32: str | None
    source_file: str
    source_type: str  # "bitcoin_core", "multibit", "bip39", etc.
    derivation_path: str | None
    balance_sat: int | None
    total_received_sat: int | None
    tx_count: int | None

@dataclass
class WalletFile:
    path: str
    wallet_type: str
    encrypted: bool
    keys_extracted: int
    keys: list[ExtractedKey]

@dataclass
class ScanResult:
    directory: str
    wallets_found: list[WalletFile]
    total_keys: int
    total_balance_sat: int
    funded_addresses: list[ExtractedKey]
```

## Balance Checking

- **Providers:** blockchain.info (primary), blockstream.info (fallback)
- **Batching:** Up to 20 addresses per API request
- **Rate limiting:** 1-second delay between batches, exponential backoff on 429 responses
- **Caching:** Results cached in-memory during a session to avoid redundant queries
- **Offline mode:** `--offline` flag skips all balance checks; user exports keys and checks separately
- **Address types checked:** For each key, check P2PKH compressed, P2PKH uncompressed, P2SH-P2WPKH, and Bech32 addresses

## Web UI

Single-page Flask application bound to `127.0.0.1` only. Four tabs:

1. **Scan** - Select a directory path or drag-drop wallet files. Shows table of discovered wallet files with type, encrypted status, and key count.
2. **Extract** - Shows all extracted keys with addresses. Optional password list input for encrypted wallets. Per-key balance status (checking/found/empty).
3. **Mnemonic** - Text input for seed phrases. Dropdown for wallet type (BIP39 generic, Breadwallet, Electrum, Blockchain.com legacy). Shows derived addresses and balances.
4. **Results** - Summary dashboard: total balance found, list of funded addresses with WIF keys, copy-to-clipboard buttons, and step-by-step instructions for importing into Electrum/Sparrow/Blue Wallet.

No authentication required (localhost only). No data persisted server-side between sessions.

## Password / Passphrase Handling

### Phase 1: Password List

Users provide a text file with one password per line. Each extractor tries all passwords against encrypted wallets. Supports:

- Bitcoin Core wallet encryption (AES-256-CBC with key derived from passphrase via EVP_BytesToKey)
- MultiBit Classic v3 encrypted keys (scrypt + AES-256-CBC at the protobuf key level)
- Blockchain.com legacy mnemonic decoding (password embedded in mnemonic)
- OpenSSL-encrypted key backup files (AES-256-CBC with MD5/SHA256 key derivation)

### Phase 2: Advanced Password Recovery (documented, not built)

The `docs/password-recovery.md` guide covers techniques and tools for users who need more sophisticated password recovery:

**Rule-based mutation:**
Take known/likely passwords and apply transformations:
- Capitalization variants (`monkey` -> `Monkey`, `MONKEY`, `mONKEY`)
- Common suffixes (`!`, `1`, `123`, `2015`, `!1`, `#1`)
- Leet speak (`a->@`, `e->3`, `o->0`, `s->$`)
- Keyboard patterns, doubled words, reversed strings
- Tools: Hashcat rules, John the Ripper rules

**Dictionary + hybrid attacks:**
- Common password wordlists (rockyou, etc.)
- Combined with user-provided hint tokens ("I know it had Monkey and some numbers")
- Token-based approach: provide word fragments, tool tries all combinations

**btcrecover integration:**
The best existing tool for Bitcoin wallet password recovery. Our guide documents:
- How to export the encrypted wallet data our tool identifies
- How to feed it to btcrecover with token lists
- GPU acceleration setup for faster cracking
- Typo-tolerant recovery (btcrecover's `--typos` flag)
- Example commands for each wallet type we support

**Pattern generation:**
For users who remember partial passwords:
- `orpheus generate-passwords --pattern "Monkey{4digits}{punct}"` outputs a wordlist
- Feed output to `--passwords` flag or to btcrecover

## Security Best Practices

### Ephemeral Docker Mode

```dockerfile
FROM python:3.12-slim
RUN pip install orpheus
ENTRYPOINT ["orpheus"]
```

```bash
# Wallets mounted read-only, container destroyed on exit
docker run --rm -it \
  -v /path/to/wallets:/wallets:ro \
  -p 127.0.0.1:8080:8080 \
  orpheus serve /wallets

# Offline mode in Docker (no network except localhost web UI)
docker run --rm -it \
  --network none \
  -v /path/to/wallets:/wallets:ro \
  -v /tmp/results:/results \
  orpheus scan /wallets --offline --output /results/keys.json
```

Properties:
- Wallet directory mounted **read-only** (`:ro`)
- Container destroyed on exit (`--rm`)
- Web UI bound to localhost only (`127.0.0.1:8080`)
- Optional `--network none` for fully air-gapped operation
- Results explicitly exported or lost on container exit

### Documented Best Practices (in `docs/security.md`)

1. **Air-gapped machine** - For maximum security, run on a machine with no network. Use `--offline` to extract keys, then transfer only the addresses (not keys) to a networked machine to check balances.
2. **Ephemeral environment** - Use Docker mode or a live USB OS (Tails) so no traces remain after shutdown.
3. **Never paste keys into web services** - Import WIF keys only into local wallet software (Electrum, Sparrow).
4. **Clear clipboard** - After copying a WIF key, clear clipboard immediately.
5. **Secure deletion** - After importing keys into a wallet and moving funds, securely delete any output files (`shred -u results.json` on Linux, Secure Empty Trash on macOS).
6. **Verify tool integrity** - Check git signatures or published checksums before running.
7. **Move funds immediately** - Once you've confirmed a balance and have the private key, sweep funds to a new wallet you control. Old keys may be compromised.
8. **Separate balance checking** - Use `--offline` mode, export only public addresses, check balances on a separate device. This prevents any possible key leakage through network calls.

## Recovery Instructions (shown in UI)

For each funded address found, the Results tab shows wallet-specific import instructions:

**Electrum (recommended):**
1. Install Electrum from electrum.org
2. Create new wallet > Import Bitcoin addresses or private keys
3. Paste the WIF private key
4. Send funds to your own wallet

**Sparrow:**
1. Install Sparrow from sparrowwallet.com
2. File > New Wallet > Import wallet from private key
3. Paste WIF key, sweep funds

**For BIP39 seed phrases:**
1. Install Electrum or Sparrow
2. Restore from seed phrase
3. Set derivation path to match original wallet (documented per wallet type)

## Dependencies

```toml
[project]
requires-python = ">=3.10"
dependencies = [
    "click>=8.0",
    "flask>=3.0",
    "ecdsa>=0.19",
    "base58>=2.1",
    "requests>=2.31",
    "cryptography>=42.0",
    "mnemonic>=0.21",
    "rich>=13.0",        # Pretty terminal output
]
```

## Testing

- Unit tests for each extractor using fixture wallet files (generated, not real wallets)
- Unit tests for all crypto primitives (known test vectors from BIP32/39/44 specs)
- Integration test for the scan → extract → balance-check pipeline with mocked API
- No real private keys or wallet data in the test suite

## Out of Scope (for now)

- Altcoin support (Ethereum, Litecoin, etc.)
- GPU-accelerated password cracking (documented, not built)
- Full blockchain validation / UTXO scanning
- Wallet repair or transaction signing
- Cloud deployment (this is a local-only tool)
