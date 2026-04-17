# Rust + Tauri Desktop Rewrite — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the Python backend of Orpheus with a Rust crate, wrap it in a Tauri desktop app with a NextJS/React/TS frontend, and ship signed single-binary releases for macOS / Windows / Linux — preserving every BIP test vector and the Breadwallet regression on the way through.

**Architecture:** A Cargo workspace with three crates: `orpheus-core` (pure library — all crypto + extractors + balance providers + scanner), `orpheus-cli` (thin `clap` wrapper reproducing the Python CLI surface), and `orpheus-desktop` (Tauri app exposing the same operations as Tauri commands to a NextJS frontend). The current Python tree stays on a `legacy-python` branch as ground truth until parity is verified against the shared fixture set.

**Tech Stack:** Rust 1.75+, `rust-bitcoin` ecosystem (`bitcoin`, `secp256k1`, `bip39`, `bech32`), `reqwest`, `tokio`, `clap`, `prost` (MultiBit protobuf), `aes` + `block-modes` + `scrypt` (encrypted extractor), `serde` + `serde_json`, Tauri 2.x, NextJS 15 + React 19 + TypeScript, Tailwind + shadcn/ui, GitHub Actions for CI, `cargo-dist` or native Tauri bundler for releases.

**Source of truth:** the existing Python modules under [src/orpheus/](../../../src/orpheus/) and their tests under [tests/](../../../tests/) define correct behaviour. Every Rust port copies assertions from the matching Python test file; Rust tests must pass on the same inputs before the Python version is retired.

---

## Phase 0 — Workspace scaffolding

### Task 0.1: Add `rust/` workspace with three crates

**Files:**
- Create: `rust/Cargo.toml` (workspace manifest)
- Create: `rust/orpheus-core/Cargo.toml`, `rust/orpheus-core/src/lib.rs`
- Create: `rust/orpheus-cli/Cargo.toml`, `rust/orpheus-cli/src/main.rs`
- Create: `rust/orpheus-desktop/Cargo.toml` (scaffolded in Phase 7)
- Modify: `.gitignore` (add `rust/target/`)

- [ ] **Step 1: Create workspace manifest**

`rust/Cargo.toml`:

```toml
[workspace]
resolver = "2"
members = ["orpheus-core", "orpheus-cli"]
# orpheus-desktop added in Phase 7

[workspace.package]
version = "0.1.0"
edition = "2021"
rust-version = "1.75"
license = "MIT"
repository = "https://github.com/dougnukem/orpheus"

[workspace.dependencies]
bitcoin = "0.32"
secp256k1 = { version = "0.29", features = ["rand", "global-context"] }
bip39 = "2.0"
bech32 = "0.11"
bs58 = { version = "0.5", features = ["check"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"
anyhow = "1"
hex = "0.4"
sha2 = "0.10"
ripemd = "0.1"
hmac = "0.12"
pbkdf2 = "0.12"
scrypt = "0.11"
aes = "0.8"
cbc = "0.1"
prost = "0.13"
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
clap = { version = "4", features = ["derive"] }
comfy-table = "7"
```

- [ ] **Step 2: Create `orpheus-core` crate**

`rust/orpheus-core/Cargo.toml`:

```toml
[package]
name = "orpheus-core"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
bitcoin.workspace = true
secp256k1.workspace = true
bip39.workspace = true
bech32.workspace = true
bs58.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
hex.workspace = true
sha2.workspace = true
ripemd.workspace = true
hmac.workspace = true
pbkdf2.workspace = true
scrypt.workspace = true
aes.workspace = true
cbc.workspace = true
prost.workspace = true
reqwest = { workspace = true, optional = true }
tokio = { workspace = true, optional = true }

[features]
default = ["balance-http"]
balance-http = ["dep:reqwest", "dep:tokio"]
```

`rust/orpheus-core/src/lib.rs`:

```rust
//! Core library for Orpheus wallet recovery.
pub mod crypto;
pub mod extractors;
pub mod balance;
pub mod scanner;
pub mod types;
```

- [ ] **Step 3: Create `orpheus-cli` crate stub**

`rust/orpheus-cli/Cargo.toml`:

```toml
[package]
name = "orpheus-cli"
version.workspace = true
edition.workspace = true
license.workspace = true

[[bin]]
name = "orpheus"
path = "src/main.rs"

[dependencies]
orpheus-core = { path = "../orpheus-core" }
clap.workspace = true
anyhow.workspace = true
comfy-table.workspace = true
serde_json.workspace = true
tokio.workspace = true
```

`rust/orpheus-cli/src/main.rs`:

```rust
fn main() -> anyhow::Result<()> {
    println!("orpheus 0.1.0 (Rust build)");
    Ok(())
}
```

- [ ] **Step 4: Update `.gitignore`**

Append to [.gitignore](../../../.gitignore):

```
# Rust build artifacts
rust/target/
rust/**/*.rs.bk
```

- [ ] **Step 5: Verify workspace builds**

Run: `cd rust && cargo build`
Expected: clean build of both crates, no warnings.

Run: `cd rust && cargo run --bin orpheus`
Expected output: `orpheus 0.1.0 (Rust build)`

- [ ] **Step 6: Commit**

```bash
git add rust/ .gitignore
git commit -m "orpheus: scaffold Rust workspace (core + cli)"
```

### Task 0.2: CI for the Rust workspace

**Files:**
- Create: `.github/workflows/rust.yml`

- [ ] **Step 1: Write workflow**

```yaml
name: rust
on:
  pull_request:
    paths: ['rust/**', '.github/workflows/rust.yml']
  push:
    branches: [main]
    paths: ['rust/**', '.github/workflows/rust.yml']

jobs:
  check:
    runs-on: ubuntu-latest
    defaults: { run: { working-directory: rust } }
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { components: rustfmt, clippy }
      - uses: Swatinem/rust-cache@v2
        with: { workspaces: rust }
      - run: cargo fmt --all -- --check
      - run: cargo clippy --all-targets --all-features -- -D warnings
      - run: cargo test --all-features
```

- [ ] **Step 2: Verify locally**

Run: `cd rust && cargo fmt --all -- --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test --all-features`
Expected: all three commands exit 0.

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/rust.yml
git commit -m "orpheus: add Rust CI (fmt, clippy, test)"
```

---

## Phase 1 — Core types

Reference: [src/orpheus/models.py](../../../src/orpheus/models.py), [tests/test_models.py](../../../tests/test_models.py).

### Task 1.1: Port data models

**Files:**
- Create: `rust/orpheus-core/src/types.rs`
- Test: inline `#[cfg(test)] mod tests` in the same file

- [ ] **Step 1: Write failing test**

`rust/orpheus-core/src/types.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    BitcoinCore,
    Multibit,
    Bip39,
    BlockchainCom,
    WalletDump,
    Encrypted,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedKey {
    pub wif: String,
    pub address_compressed: String,
    pub source_file: String,
    pub source_type: SourceType,
    #[serde(default)] pub address_uncompressed: Option<String>,
    #[serde(default)] pub address_p2sh_segwit: Option<String>,
    #[serde(default)] pub address_bech32: Option<String>,
    #[serde(default)] pub derivation_path: Option<String>,
    #[serde(default)] pub balance_sat: Option<u64>,
    #[serde(default)] pub total_received_sat: Option<u64>,
    #[serde(default)] pub tx_count: Option<u64>,
    #[serde(default)] pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceInfo {
    pub address: String,
    pub balance_sat: u64,
    pub total_received_sat: u64,
    pub tx_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletScanResult {
    pub source_file: String,
    pub source_type: SourceType,
    #[serde(default)] pub keys: Vec<ExtractedKey>,
    #[serde(default)] pub error: Option<String>,
}

impl WalletScanResult {
    pub fn key_count(&self) -> usize { self.keys.len() }
    pub fn total_balance_sat(&self) -> u64 {
        self.keys.iter().filter_map(|k| k.balance_sat).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracted_key_serializes_with_snake_case_source_type() {
        let k = ExtractedKey {
            wif: "KxFC…".into(),
            address_compressed: "1A…".into(),
            source_file: "wallet.dat".into(),
            source_type: SourceType::BitcoinCore,
            address_uncompressed: None,
            address_p2sh_segwit: None,
            address_bech32: None,
            derivation_path: None,
            balance_sat: None,
            total_received_sat: None,
            tx_count: None,
            notes: None,
        };
        let j = serde_json::to_value(&k).unwrap();
        assert_eq!(j["source_type"], "bitcoin_core");
    }

    #[test]
    fn wallet_scan_result_totals() {
        let r = WalletScanResult {
            source_file: "w.dat".into(),
            source_type: SourceType::BitcoinCore,
            keys: vec![],
            error: None,
        };
        assert_eq!(r.key_count(), 0);
        assert_eq!(r.total_balance_sat(), 0);
    }
}
```

- [ ] **Step 2: Run tests, expect them to pass**

Run: `cd rust && cargo test -p orpheus-core types::`
Expected: 2 passed.

- [ ] **Step 3: Commit**

```bash
git add rust/orpheus-core/src/types.rs
git commit -m "orpheus-core: add data types (ExtractedKey, WalletScanResult, BalanceInfo)"
```

---

## Phase 2 — Crypto primitives

Reference files map 1:1. Each Rust module copies its tests from the matching Python test file. **Use the `bitcoin` crate rather than reimplementing — the whole point of the rewrite is that the Rust ecosystem is higher-quality than what we have.**

### Task 2.1: Keys — secp256k1 + WIF + P2PKH

**Files:**
- Create: `rust/orpheus-core/src/crypto/mod.rs`, `rust/orpheus-core/src/crypto/keys.rs`
- Reference Python: [src/orpheus/crypto/keys.rs](../../../src/orpheus/crypto/keys.py), [tests/test_crypto_keys.py](../../../tests/test_crypto_keys.py)

- [ ] **Step 1: Create the module skeleton**

`rust/orpheus-core/src/crypto/mod.rs`:

```rust
pub mod keys;
pub mod bip32;
pub mod bech32;
pub mod addresses;
```

- [ ] **Step 2: Write the API + port tests from Python**

`rust/orpheus-core/src/crypto/keys.rs`:

```rust
use bitcoin::{Address, Network, PrivateKey, PublicKey};
use bitcoin::secp256k1::{Secp256k1, SecretKey};

#[derive(Debug, thiserror::Error)]
pub enum KeyError {
    #[error("invalid secp256k1 scalar")]
    InvalidScalar,
    #[error("invalid WIF: {0}")]
    InvalidWif(String),
}

pub fn validate_secp256k1_scalar(bytes: &[u8; 32]) -> Result<(), KeyError> {
    SecretKey::from_slice(bytes).map(|_| ()).map_err(|_| KeyError::InvalidScalar)
}

pub fn privkey_to_pubkey(priv_bytes: &[u8; 32], compressed: bool) -> Result<Vec<u8>, KeyError> {
    let sk = SecretKey::from_slice(priv_bytes).map_err(|_| KeyError::InvalidScalar)?;
    let secp = Secp256k1::new();
    let pk = bitcoin::secp256k1::PublicKey::from_secret_key(&secp, &sk);
    Ok(if compressed { pk.serialize().to_vec() } else { pk.serialize_uncompressed().to_vec() })
}

pub fn pubkey_to_p2pkh_address(pub_bytes: &[u8]) -> Result<String, KeyError> {
    let pk = PublicKey::from_slice(pub_bytes).map_err(|e| KeyError::InvalidWif(e.to_string()))?;
    Ok(Address::p2pkh(&pk, Network::Bitcoin).to_string())
}

pub fn privkey_to_wif(priv_bytes: &[u8; 32], compressed: bool) -> Result<String, KeyError> {
    let sk = SecretKey::from_slice(priv_bytes).map_err(|_| KeyError::InvalidScalar)?;
    Ok(PrivateKey { compressed, network: Network::Bitcoin.into(), inner: sk }.to_wif())
}

pub fn wif_to_privkey(wif: &str) -> Result<([u8; 32], bool), KeyError> {
    let pk = PrivateKey::from_wif(wif).map_err(|e| KeyError::InvalidWif(e.to_string()))?;
    Ok((pk.inner.secret_bytes(), pk.compressed))
}

#[cfg(test)]
mod tests {
    use super::*;

    // BIP32 test vector 1 master key secret: derived from seed "000102...0f"
    // WIF + addresses copied verbatim from tests/test_crypto_keys.py.

    #[test]
    fn wif_roundtrip_compressed() {
        let priv_hex = "e8f32e723decf4051aefac8e2c93c9c5b214313817cdb01a1494b917c8436b35";
        let priv_bytes: [u8; 32] = hex::decode(priv_hex).unwrap().try_into().unwrap();
        let wif = privkey_to_wif(&priv_bytes, true).unwrap();
        let (back, compressed) = wif_to_privkey(&wif).unwrap();
        assert_eq!(back, priv_bytes);
        assert!(compressed);
    }

    #[test]
    fn pubkey_and_address_match_reference() {
        // Copy the exact (priv, expected_addr) pair from tests/test_crypto_keys.py.
        // Replace the expected_addr string with whatever that test asserts.
        let priv_hex = "e8f32e723decf4051aefac8e2c93c9c5b214313817cdb01a1494b917c8436b35";
        let priv_bytes: [u8; 32] = hex::decode(priv_hex).unwrap().try_into().unwrap();
        let pk = privkey_to_pubkey(&priv_bytes, true).unwrap();
        let addr = pubkey_to_p2pkh_address(&pk).unwrap();
        let expected = std::fs::read_to_string(
            concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/fixtures/keys_expected_p2pkh.txt")
        ).unwrap_or_else(|_| "15mKKb2eos1hWa6tisdPwwDC1a5J1y9nma".into());
        assert_eq!(addr.trim(), expected.trim());
    }

    #[test]
    fn reject_invalid_scalar_zero() {
        assert!(validate_secp256k1_scalar(&[0u8; 32]).is_err());
    }
}
```

> **Porting note:** open [tests/test_crypto_keys.py](../../../tests/test_crypto_keys.py) and copy each `(priv_hex, expected_wif, expected_addr)` tuple into a `#[test]`. Do not invent vectors — copy the ones the Python tests already prove.

- [ ] **Step 3: Run tests**

Run: `cd rust && cargo test -p orpheus-core crypto::keys`
Expected: all pass.

- [ ] **Step 4: Commit**

```bash
git add rust/orpheus-core/src/crypto/
git commit -m "orpheus-core: port crypto::keys with WIF + P2PKH (BIP vectors)"
```

### Task 2.2: BIP32 HD derivation

**Files:**
- Create: `rust/orpheus-core/src/crypto/bip32.rs`
- Reference Python: [src/orpheus/crypto/bip32.py](../../../src/orpheus/crypto/bip32.py), [tests/test_crypto_bip32.py](../../../tests/test_crypto_bip32.py)

- [ ] **Step 1: Implement `derive_path` using `bitcoin::bip32`**

```rust
use bitcoin::bip32::{DerivationPath, Xpriv};
use bitcoin::secp256k1::Secp256k1;
use bitcoin::Network;
use std::str::FromStr;

#[derive(Debug, thiserror::Error)]
pub enum Bip32Error {
    #[error("bad path: {0}")] BadPath(String),
    #[error("bad seed")] BadSeed,
    #[error("derive failed: {0}")] Derive(String),
}

pub fn derive_path(seed: &[u8], path: &str) -> Result<[u8; 32], Bip32Error> {
    let secp = Secp256k1::new();
    let root = Xpriv::new_master(Network::Bitcoin, seed).map_err(|_| Bip32Error::BadSeed)?;
    let dp = DerivationPath::from_str(path).map_err(|e| Bip32Error::BadPath(e.to_string()))?;
    let child = root.derive_priv(&secp, &dp).map_err(|e| Bip32Error::Derive(e.to_string()))?;
    Ok(child.private_key.secret_bytes())
}
```

- [ ] **Step 2: Port BIP32 test vectors 1 & 2 from the Python test file**

Copy every `(seed_hex, path, expected_priv_hex)` tuple from [tests/test_crypto_bip32.py](../../../tests/test_crypto_bip32.py) into Rust `#[test]`s. Include at minimum the master node, `m/0'`, `m/0'/1`, `m/0'/1/2'`, `m/0'/1/2'/2`, `m/0'/1/2'/2/1000000000` from vector 1, and at least two entries from vector 2.

- [ ] **Step 3: Run, commit**

Run: `cd rust && cargo test -p orpheus-core crypto::bip32`

```bash
git add rust/orpheus-core/src/crypto/bip32.rs
git commit -m "orpheus-core: port BIP32 HD derivation (vectors 1 & 2)"
```

### Task 2.3: Bech32

**Files:**
- Create: `rust/orpheus-core/src/crypto/bech32.rs`
- Reference: [src/orpheus/crypto/bech32.py](../../../src/orpheus/crypto/bech32.py), [tests/test_crypto_bech32.py](../../../tests/test_crypto_bech32.py)

- [ ] **Step 1: Use the `bech32` crate directly; wrap BIP173/BIP350 encode + decode**

```rust
use bech32::{Bech32, Bech32m, Hrp};

pub fn encode_segwit_v0(hrp: &str, program: &[u8]) -> Result<String, String> {
    let hrp = Hrp::parse(hrp).map_err(|e| e.to_string())?;
    let mut data = vec![bech32::u5::try_from_u8(0).unwrap()];
    data.extend(bech32::convert_bits(program, 8, 5, true).map_err(|e| e.to_string())?
        .into_iter().map(|b| bech32::u5::try_from_u8(b).unwrap()));
    bech32::encode::<Bech32>(hrp, &data).map_err(|e| e.to_string())
}

pub fn encode_segwit_v1(hrp: &str, program: &[u8]) -> Result<String, String> {
    let hrp = Hrp::parse(hrp).map_err(|e| e.to_string())?;
    let mut data = vec![bech32::u5::try_from_u8(1).unwrap()];
    data.extend(bech32::convert_bits(program, 8, 5, true).map_err(|e| e.to_string())?
        .into_iter().map(|b| bech32::u5::try_from_u8(b).unwrap()));
    bech32::encode::<Bech32m>(hrp, &data).map_err(|e| e.to_string())
}
```

> Match the exact API the Python module exposes. If the `bech32` crate's current version has a different shape, adapt — the test vectors are the contract, not the function signatures.

- [ ] **Step 2: Port BIP173 + BIP350 vectors from the Python test file as `#[test]`s**

- [ ] **Step 3: Run, commit**

```bash
git add rust/orpheus-core/src/crypto/bech32.rs
git commit -m "orpheus-core: port bech32 encode/decode (BIP173 + BIP350 vectors)"
```

### Task 2.4: Address helpers

**Files:**
- Create: `rust/orpheus-core/src/crypto/addresses.rs`
- Reference: [src/orpheus/crypto/addresses.py](../../../src/orpheus/crypto/addresses.py)

- [ ] **Step 1: Implement `all_addresses_for_privkey`**

```rust
use crate::crypto::keys;

#[derive(Debug, Clone)]
pub struct AllAddresses {
    pub p2pkh_compressed: String,
    pub p2pkh_uncompressed: String,
    pub p2sh_p2wpkh: String,
    pub bech32: String,
}

pub fn all_addresses_for_privkey(priv_bytes: &[u8; 32]) -> anyhow::Result<AllAddresses> {
    use bitcoin::{Address, Network, PublicKey};
    use bitcoin::secp256k1::{Secp256k1, SecretKey};
    let secp = Secp256k1::new();
    let sk = SecretKey::from_slice(priv_bytes)?;
    let pk_compressed = PublicKey { compressed: true,
        inner: bitcoin::secp256k1::PublicKey::from_secret_key(&secp, &sk) };
    let pk_uncompressed = PublicKey { compressed: false, inner: pk_compressed.inner };
    Ok(AllAddresses {
        p2pkh_compressed: Address::p2pkh(&pk_compressed, Network::Bitcoin).to_string(),
        p2pkh_uncompressed: Address::p2pkh(&pk_uncompressed, Network::Bitcoin).to_string(),
        p2sh_p2wpkh: Address::p2shwpkh(&pk_compressed, Network::Bitcoin)?.to_string(),
        bech32: Address::p2wpkh(&pk_compressed, Network::Bitcoin)?.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn known_priv_yields_all_four_formats() {
        // Copy the (priv_hex, expected_*) tuple from the matching Python test.
        let priv_bytes: [u8; 32] = hex::decode(
            "e8f32e723decf4051aefac8e2c93c9c5b214313817cdb01a1494b917c8436b35"
        ).unwrap().try_into().unwrap();
        let a = all_addresses_for_privkey(&priv_bytes).unwrap();
        assert!(a.p2pkh_compressed.starts_with('1'));
        assert!(a.p2sh_p2wpkh.starts_with('3'));
        assert!(a.bech32.starts_with("bc1q"));
    }
}
```

- [ ] **Step 2: Run, commit**

```bash
git add rust/orpheus-core/src/crypto/addresses.rs
git commit -m "orpheus-core: port all_addresses_for_privkey helper"
```

---

## Phase 3 — Extractors

Each extractor: base trait, module, unit tests ported from the matching `tests/test_extractors_*.py`, one commit per extractor.

### Task 3.1: Extractor trait + registry

**Files:**
- Create: `rust/orpheus-core/src/extractors/mod.rs`
- Reference: [src/orpheus/extractors/base.py](../../../src/orpheus/extractors/base.py)

- [ ] **Step 1: Define the trait**

```rust
use crate::types::{ExtractedKey, SourceType};
use std::path::Path;

pub mod wallet_dump;
pub mod bip39;
pub mod bitcoin_core;
pub mod multibit;
pub mod blockchain_com;
pub mod encrypted;

pub trait Extractor: Send + Sync {
    fn name(&self) -> &'static str;
    fn source_type(&self) -> SourceType;
    fn can_handle(&self, path: &Path) -> bool;
    fn extract(&self, path: &Path, passwords: &[String]) -> anyhow::Result<Vec<ExtractedKey>>;
}

pub fn all_extractors() -> Vec<Box<dyn Extractor>> {
    vec![
        Box::new(wallet_dump::WalletDumpExtractor),
        Box::new(bitcoin_core::BitcoinCoreExtractor),
        Box::new(multibit::MultibitExtractor),
        Box::new(blockchain_com::BlockchainComExtractor),
        Box::new(bip39::Bip39Extractor),
        Box::new(encrypted::EncryptedExtractor),
    ]
}

pub fn find_extractor(path: &Path) -> Option<Box<dyn Extractor>> {
    all_extractors().into_iter().find(|e| e.can_handle(path))
}
```

- [ ] **Step 2: Commit (will fail to build until module stubs exist — do step 3 first)**

- [ ] **Step 3: Add empty module stubs for each extractor so the trait file compiles**

For each module (`wallet_dump`, `bip39`, `bitcoin_core`, `multibit`, `blockchain_com`, `encrypted`), create `rust/orpheus-core/src/extractors/<name>.rs` with a `pub struct NameExtractor;` + a trait impl returning empty vec and `can_handle = false`. These are filled in in subsequent tasks.

- [ ] **Step 4: `cargo build`, commit**

```bash
git add rust/orpheus-core/src/extractors/
git commit -m "orpheus-core: add Extractor trait + empty module stubs"
```

### Task 3.2: `wallet_dump` (Bitcoin Core `dumpwallet` text)

**Files:** `rust/orpheus-core/src/extractors/wallet_dump.rs`
**Reference:** [src/orpheus/extractors/wallet_dump.py](../../../src/orpheus/extractors/wallet_dump.py), [tests/test_extractors_wallet_dump.py](../../../tests/test_extractors_wallet_dump.py)

- [ ] **Step 1: Implement `can_handle` (file is UTF-8, starts with `# Wallet dump`) and `extract`**

Port the line-by-line parser exactly. Each line looks like `<wif> <timestamp> <reserve|change|label=...> # ...`. Build one `ExtractedKey` per line via `keys::wif_to_privkey` + `addresses::all_addresses_for_privkey`.

- [ ] **Step 2: Copy every test case from `tests/test_extractors_wallet_dump.py`**

Use `include_str!("../../../../tests/fixtures/wallet_dump_sample.txt")` or similar if fixtures exist; otherwise inline the small hand-crafted dump the Python test uses.

- [ ] **Step 3: Run, commit**

```bash
git add rust/orpheus-core/src/extractors/wallet_dump.rs
git commit -m "orpheus-core: port wallet_dump extractor"
```

### Task 3.3: `bip39` — seed → derive common paths (Breadwallet regression included)

**Files:** `rust/orpheus-core/src/extractors/bip39.rs`
**Reference:** [src/orpheus/extractors/bip39.py](../../../src/orpheus/extractors/bip39.py), [tests/test_extractors_bip39.py](../../../tests/test_extractors_bip39.py)

This is one of the **highest-value tests** in the repo: the Breadwallet `m/0'/0/x`, `m/0'/1/x` paths anchor a real 2026-04-17 recovery. Port every assertion from the Python test verbatim.

- [ ] **Step 1: Implement `extract_from_mnemonic(mnemonic, paths_per_type, passphrase) -> Vec<ExtractedKey>`**

```rust
use bip39::Mnemonic;
use crate::crypto::{bip32, addresses, keys};
use crate::types::{ExtractedKey, SourceType};

pub const DEFAULT_PATHS: &[&str] = &[
    // BIP44 / 49 / 84 receive chain, first 20 addresses
    "m/44'/0'/0'/0/{i}", "m/49'/0'/0'/0/{i}", "m/84'/0'/0'/0/{i}",
    // Breadwallet — first-class regression case
    "m/0'/0/{i}", "m/0'/1/{i}",
];

pub fn derive_paths(mnemonic: &str, passphrase: &str, count_per_path: u32)
    -> anyhow::Result<Vec<ExtractedKey>> {
    let m = Mnemonic::parse(mnemonic)?;
    let seed = m.to_seed(passphrase);
    let mut out = Vec::new();
    for template in DEFAULT_PATHS {
        for i in 0..count_per_path {
            let path = template.replace("{i}", &i.to_string());
            let priv_bytes = bip32::derive_path(&seed, &path)?;
            let addrs = addresses::all_addresses_for_privkey(&priv_bytes)?;
            out.push(ExtractedKey {
                wif: keys::privkey_to_wif(&priv_bytes, true)?,
                address_compressed: addrs.p2pkh_compressed,
                source_file: mnemonic.into(),
                source_type: SourceType::Bip39,
                address_uncompressed: Some(addrs.p2pkh_uncompressed),
                address_p2sh_segwit: Some(addrs.p2sh_p2wpkh),
                address_bech32: Some(addrs.bech32),
                derivation_path: Some(path),
                balance_sat: None, total_received_sat: None, tx_count: None, notes: None,
            });
        }
    }
    Ok(out)
}
```

- [ ] **Step 2: Port BIP39 reference test — mnemonic `"abandon abandon ... about"` → known addresses**

- [ ] **Step 3: Port the Breadwallet regression test**

Copy the exact `(mnemonic, path, expected_address)` tuple from `tests/test_extractors_bip39.py` that covers `m/0'/0/x`. **If this test fails, stop and diagnose before continuing.**

- [ ] **Step 4: Run, commit**

```bash
git add rust/orpheus-core/src/extractors/bip39.rs
git commit -m "orpheus-core: port BIP39 extractor + Breadwallet regression"
```

### Task 3.4: `bitcoin_core` — BDB + SQLite DER byte-scan

**Files:** `rust/orpheus-core/src/extractors/bitcoin_core.rs`
**Reference:** [src/orpheus/extractors/bitcoin_core.py](../../../src/orpheus/extractors/bitcoin_core.py), [tests/test_extractors_bitcoin_core.py](../../../tests/test_extractors_bitcoin_core.py)

- [ ] **Step 1: Implement DER pattern byte-scan**

```rust
const DER_PATTERN: &[u8] = &[0x30, 0x81, 0xD3, 0x02, 0x01, 0x01, 0x04, 0x20];

pub fn scan_for_keys(bytes: &[u8]) -> Vec<[u8; 32]> {
    let mut out = Vec::new();
    let mut i = 0;
    while i + DER_PATTERN.len() + 32 <= bytes.len() {
        if &bytes[i..i + DER_PATTERN.len()] == DER_PATTERN {
            let start = i + DER_PATTERN.len();
            let mut k = [0u8; 32];
            k.copy_from_slice(&bytes[start..start + 32]);
            out.push(k);
            i = start + 32;
        } else { i += 1; }
    }
    out
}
```

- [ ] **Step 2: `can_handle`: extension is `.dat` OR starts with BDB magic `0x00053162` OR SQLite magic `"SQLite format 3\0"`**

- [ ] **Step 3: `extract`: read file → scan_for_keys → map to ExtractedKey**

- [ ] **Step 4: Port tests, incl. the synthesized BDB fixture from `tests/fixtures/`. `include_bytes!` the fixture.**

- [ ] **Step 5: Run, commit**

```bash
git add rust/orpheus-core/src/extractors/bitcoin_core.rs
git commit -m "orpheus-core: port bitcoin_core BDB/SQLite extractor"
```

### Task 3.5: `multibit` — protobuf + IV/ciphertext pairs

**Files:**
- Create: `rust/orpheus-core/build.rs`, `rust/orpheus-core/proto/multibit.proto`, `rust/orpheus-core/src/extractors/multibit.rs`
- Add build-dep: `prost-build = "0.13"` in `orpheus-core/Cargo.toml` `[build-dependencies]`
- Reference: [src/orpheus/extractors/multibit.py](../../../src/orpheus/extractors/multibit.py), [tests/test_extractors_multibit.py](../../../tests/test_extractors_multibit.py)

- [ ] **Step 1: Extract the `.proto` used by MultiBit HD / Classic**

Source the relevant messages (`Wallet`, `Key`, `EncryptedData`) from the MultiBit Classic repo (`org.multibit.hd.brit.protobuf.MBHDProtos` / `WalletProtos.proto`). Commit the `.proto` under `rust/orpheus-core/proto/multibit.proto`.

- [ ] **Step 2: Wire `build.rs`**

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    prost_build::compile_protos(&["proto/multibit.proto"], &["proto/"])?;
    Ok(())
}
```

- [ ] **Step 3: Implement the two paths (encrypted vs plaintext)**

Encrypted path: scan the protobuf bytes for `IV(16) + encrypted(48)` pairs (same heuristic the Python does). Plaintext path: parse the protobuf with prost, read raw privkeys.

- [ ] **Step 4: Port all tests from the Python test file**

- [ ] **Step 5: Commit**

```bash
git add rust/orpheus-core/build.rs rust/orpheus-core/proto/ rust/orpheus-core/src/extractors/multibit.rs rust/orpheus-core/Cargo.toml
git commit -m "orpheus-core: port MultiBit extractor (protobuf + IV pair scan)"
```

### Task 3.6: `blockchain_com` — V2/V3/V4/V5 decoders

**Files:** `rust/orpheus-core/src/extractors/blockchain_com.rs`
**Reference:** [src/orpheus/extractors/blockchain_com.py](../../../src/orpheus/extractors/blockchain_com.py), [tests/test_extractors_blockchain_com.py](../../../tests/test_extractors_blockchain_com.py)

This is the hardest port. Do it one version at a time, commit per version, run all Python test vectors against each.

- [ ] **Step 1: V2 (1626-word decoder)**

Port the wordlist + bit-unpacking logic. Commit the wordlist checksum test (from the Python test) as a compile-time assertion.

- [ ] **Step 2: V3 (65591-word)**
- [ ] **Step 3: V4 (GUID-based)**
- [ ] **Step 4: V5 (timestamp-based)**
- [ ] **Step 5: Dispatcher: try V5 → V4 → V3 → V2 in order; first to decode wins**
- [ ] **Step 6: Port every hand-crafted vector from `tests/test_extractors_blockchain_com.py`**
- [ ] **Step 7: Commit each sub-version separately**

```bash
git commit -m "orpheus-core: port blockchain.com V2 decoder"
# ... repeat per version
```

### Task 3.7: `encrypted` — AES-256-CBC + scrypt / EVP_BytesToKey

**Files:** `rust/orpheus-core/src/extractors/encrypted.rs`
**Reference:** [src/orpheus/extractors/encrypted.py](../../../src/orpheus/extractors/encrypted.py), [tests/test_extractors_encrypted.py](../../../tests/test_extractors_encrypted.py)

- [ ] **Step 1: Implement scrypt KDF path**

```rust
use scrypt::{scrypt, Params};
use aes::Aes256;
use cbc::Decryptor;
use cbc::cipher::{BlockDecryptMut, KeyIvInit, block_padding::Pkcs7};

pub fn decrypt_scrypt(ct: &[u8], iv: &[u8; 16], salt: &[u8], password: &str)
    -> anyhow::Result<Vec<u8>> {
    let mut key = [0u8; 32];
    scrypt(password.as_bytes(), salt, &Params::new(14, 8, 1, 32)?, &mut key)?;
    let mut buf = ct.to_vec();
    let pt = Decryptor::<Aes256>::new(&key.into(), iv.into())
        .decrypt_padded_mut::<Pkcs7>(&mut buf)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(pt.to_vec())
}
```

- [ ] **Step 2: Implement EVP_BytesToKey path** (OpenSSL-compatible MD5-based KDF)

- [ ] **Step 3: Password-list iteration** — try each password against both KDFs, return on first successful decrypt whose plaintext parses as a privkey / WIF / mnemonic.

- [ ] **Step 4: Port all test vectors**

- [ ] **Step 5: Commit**

```bash
git add rust/orpheus-core/src/extractors/encrypted.rs
git commit -m "orpheus-core: port encrypted extractor (scrypt + EVP_BytesToKey)"
```

---

## Phase 4 — Balance providers

Reference: [src/orpheus/balance.py](../../../src/orpheus/balance.py), [tests/test_balance.py](../../../tests/test_balance.py).

### Task 4.1: `BalanceProvider` trait + Mock

**Files:** `rust/orpheus-core/src/balance/mod.rs`, `rust/orpheus-core/src/balance/mock.rs`

- [ ] **Step 1: Define async trait**

```rust
use crate::types::BalanceInfo;
use async_trait::async_trait;

#[async_trait]
pub trait BalanceProvider: Send + Sync {
    async fn get_balances(&self, addresses: &[String]) -> anyhow::Result<Vec<BalanceInfo>>;
}

pub mod mock;
#[cfg(feature = "balance-http")] pub mod blockchain_info;
#[cfg(feature = "balance-http")] pub mod blockstream;
```

Add `async-trait = "0.1"` to workspace deps.

- [ ] **Step 2: Mock provider reads `tests/fixtures/mock_balances.json`**

```rust
pub struct MockProvider { entries: std::collections::HashMap<String, BalanceInfo> }

impl MockProvider {
    pub fn from_path(path: &std::path::Path) -> anyhow::Result<Self> {
        let raw: std::collections::HashMap<String, BalanceInfo> =
            serde_json::from_str(&std::fs::read_to_string(path)?)?;
        Ok(Self { entries: raw })
    }
}

#[async_trait]
impl BalanceProvider for MockProvider {
    async fn get_balances(&self, addresses: &[String]) -> anyhow::Result<Vec<BalanceInfo>> {
        Ok(addresses.iter()
            .filter_map(|a| self.entries.get(a).cloned())
            .collect())
    }
}
```

- [ ] **Step 3: Test against the same `tests/fixtures/mock_balances.json` the Python uses**
- [ ] **Step 4: Commit**

### Task 4.2: HTTP providers (Blockchain.info + Blockstream)

- [ ] **Step 1: Implement both, batching up to 20 addresses per call (matching Python)**
- [ ] **Step 2: Tests use `wiremock` crate to stub HTTP — no live network in tests**
- [ ] **Step 3: Commit one provider per step**

```bash
git commit -m "orpheus-core: add Blockchain.info balance provider"
git commit -m "orpheus-core: add Blockstream balance provider"
```

---

## Phase 5 — Scanner

Reference: [src/orpheus/scanner.py](../../../src/orpheus/scanner.py).

### Task 5.1: Directory walker + extractor dispatch

**Files:** `rust/orpheus-core/src/scanner.rs`

- [ ] **Step 1: Implement**

```rust
use crate::extractors::find_extractor;
use crate::types::{SourceType, WalletScanResult};
use crate::balance::BalanceProvider;
use std::path::Path;

pub async fn scan_dir(
    root: &Path,
    passwords: &[String],
    provider: Option<&dyn BalanceProvider>,
) -> anyhow::Result<Vec<WalletScanResult>> {
    let mut out = Vec::new();
    for entry in walkdir::WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() { continue; }
        let path = entry.path();
        let Some(ext) = find_extractor(path) else { continue; };
        let mut result = WalletScanResult {
            source_file: path.display().to_string(),
            source_type: ext.source_type(),
            keys: vec![], error: None,
        };
        match ext.extract(path, passwords) {
            Ok(keys) => result.keys = keys,
            Err(e) => { result.error = Some(e.to_string()); out.push(result); continue; }
        }
        if let Some(p) = provider {
            let addrs: Vec<String> = result.keys.iter().map(|k| k.address_compressed.clone()).collect();
            if let Ok(balances) = p.get_balances(&addrs).await {
                let map: std::collections::HashMap<_, _> = balances.into_iter()
                    .map(|b| (b.address.clone(), b)).collect();
                for k in result.keys.iter_mut() {
                    if let Some(b) = map.get(&k.address_compressed) {
                        k.balance_sat = Some(b.balance_sat);
                        k.total_received_sat = Some(b.total_received_sat);
                        k.tx_count = Some(b.tx_count);
                    }
                }
            }
        }
        out.push(result);
    }
    Ok(out)
}
```

Add `walkdir = "2"` to workspace deps.

- [ ] **Step 2: Test: run `scan_dir` against `tests/fixtures/demo-wallets/` with `MockProvider`. Assert same result count + same total balance as the Python `orpheus demo` output (capture that output first into `tests/fixtures/demo_expected.json`).**

- [ ] **Step 3: Commit**

```bash
git add rust/orpheus-core/src/scanner.rs
git commit -m "orpheus-core: add scanner with extractor dispatch + balance resolution"
```

---

## Phase 6 — CLI (`orpheus-cli` with `clap`)

Reference: [src/orpheus/cli.py](../../../src/orpheus/cli.py).

### Task 6.1: Subcommand scaffolding

**Files:** `rust/orpheus-cli/src/main.rs`, `rust/orpheus-cli/src/cmd/mod.rs` (+ one file per subcommand)

- [ ] **Step 1: `clap` derives**

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "orpheus", version, about = "Bitcoin wallet recovery tool")]
struct Cli { #[command(subcommand)] cmd: Cmd }

#[derive(Subcommand)]
enum Cmd {
    Scan { path: std::path::PathBuf,
           #[arg(long)] passwords: Option<std::path::PathBuf>,
           #[arg(long, default_value = "mock")] provider: String,
           #[arg(long, default_value = "table")] output: String },
    Extract { wallet: std::path::PathBuf,
              #[arg(long)] passwords: Option<std::path::PathBuf> },
    Mnemonic { words: String,
               #[arg(long, default_value = "bip39")] kind: String,
               #[arg(long)] paths: Option<String> },
    Demo,
    Serve { #[arg(long, default_value = "stdio")] transport: String },
}
```

- [ ] **Step 2: Route to module-level `run(args) -> anyhow::Result<()>` per subcommand**
- [ ] **Step 3: Table output using `comfy-table`; JSON via `serde_json::to_string_pretty`; CSV via `csv` crate**
- [ ] **Step 4: End-to-end test: `orpheus demo` produces JSON identical to Python `orpheus demo --output json` (golden-file test using `insta` crate)**

Add `insta = "1"` to dev-deps.

- [ ] **Step 5: Commit per subcommand**

```bash
git commit -m "orpheus-cli: add scan subcommand"
# ... one per subcommand
```

### Task 6.2: Retire Flask `web` → `orpheus serve --transport stdio|http`

The desktop app needs a stable JSON-RPC surface that *doesn't* include the Python Flask scaffolding.

- [ ] **Step 1: Define the JSON-RPC method set** (scan, extract, mnemonic_derive, check_balances) as a single Rust module used by both the CLI `serve` subcommand and Tauri commands in Phase 8.
- [ ] **Step 2: stdio transport — read line-delimited JSON, write line-delimited JSON**
- [ ] **Step 3: Tests: round-trip a `scan` request against `tests/fixtures/demo-wallets/`**
- [ ] **Step 4: Commit**

---

## Phase 7 — Tauri desktop app

### Task 7.1: Scaffold `orpheus-desktop`

**Files:**
- Run: `cargo install create-tauri-app --locked`
- Run: `cd rust && cargo create-tauri-app` — answer: project name `orpheus-desktop`, frontend `Next.js` (TS), package manager `npm`.
- Modify: `rust/Cargo.toml` — add `"orpheus-desktop/src-tauri"` to workspace `members`.
- Modify: `rust/orpheus-desktop/src-tauri/Cargo.toml` — add `orpheus-core = { path = "../../orpheus-core" }`.

- [ ] **Step 1: Scaffold**
- [ ] **Step 2: Verify `cd rust/orpheus-desktop && npm install && npm run tauri dev` launches the default Tauri window**
- [ ] **Step 3: Commit**

```bash
git add rust/orpheus-desktop/ rust/Cargo.toml
git commit -m "orpheus-desktop: scaffold Tauri + Next.js app"
```

### Task 7.2: Expose Tauri commands wrapping `orpheus-core`

**Files:** `rust/orpheus-desktop/src-tauri/src/lib.rs`

- [ ] **Step 1: Define the command surface (mirrors Phase 6.2 JSON-RPC methods)**

```rust
use orpheus_core::{scanner, types::WalletScanResult, balance::mock::MockProvider};
use std::path::PathBuf;

#[tauri::command]
async fn scan_path(path: String, passwords: Vec<String>) -> Result<Vec<WalletScanResult>, String> {
    let provider = None::<&dyn orpheus_core::balance::BalanceProvider>;
    scanner::scan_dir(&PathBuf::from(path), &passwords, provider).await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn derive_mnemonic(mnemonic: String, passphrase: String, count: u32)
    -> Result<Vec<orpheus_core::types::ExtractedKey>, String> {
    orpheus_core::extractors::bip39::derive_paths(&mnemonic, &passphrase, count)
        .map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![scan_path, derive_mnemonic])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 2: Rust-side integration test using `tauri::test`**
- [ ] **Step 3: Commit**

---

## Phase 8 — Frontend (Next.js + React + Tailwind + shadcn)

### Task 8.1: Next.js static export + Tauri wiring

**Files:**
- Modify: `rust/orpheus-desktop/next.config.js` — `output: "export"`, `images.unoptimized: true`
- Create: `rust/orpheus-desktop/src/lib/tauri.ts`

- [ ] **Step 1: Typed wrappers for Tauri commands**

```ts
import { invoke } from "@tauri-apps/api/core";

export interface ExtractedKey { /* mirror Rust type */ }
export interface WalletScanResult { /* mirror Rust type */ }

export const scanPath = (path: string, passwords: string[] = []) =>
  invoke<WalletScanResult[]>("scan_path", { path, passwords });

export const deriveMnemonic = (mnemonic: string, passphrase = "", count = 20) =>
  invoke<ExtractedKey[]>("derive_mnemonic", { mnemonic, passphrase, count });
```

- [ ] **Step 2: Install shadcn/ui + Tailwind**

Run: `cd rust/orpheus-desktop && npx shadcn@latest init` and add `button`, `card`, `table`, `input`, `dialog`.

- [ ] **Step 3: Commit**

### Task 8.2: Core UI screens (MVP: scan → results table)

**Files:**
- Create: `rust/orpheus-desktop/src/app/page.tsx` (scan home)
- Create: `rust/orpheus-desktop/src/app/results/page.tsx` (results table)
- Create: `rust/orpheus-desktop/src/app/mnemonic/page.tsx` (BIP39 derivation)

- [ ] **Step 1: Home page — drag-and-drop wallet folder, password list input, "Scan" button**
- [ ] **Step 2: Results page — Rich-style table of `ExtractedKey`s grouped by source file, with balance column**
- [ ] **Step 3: Mnemonic page — textarea for mnemonic, passphrase field, derivation count, results table**
- [ ] **Step 4: Smoke test: `npm run tauri dev`, scan the demo fixture directory, verify results render**
- [ ] **Step 5: Commit per screen**

---

## Phase 9 — Packaging + signing

### Task 9.1: GitHub Actions release workflow

**Files:** `.github/workflows/release.yml`

- [ ] **Step 1: Matrix build (`macos-14`, `windows-2022`, `ubuntu-22.04`) via Tauri GitHub Action**

Use `tauri-apps/tauri-action@v0`. Triggered on `v*` tags. Produces `.dmg`, `.msi`, `.AppImage`, `.deb`.

- [ ] **Step 2: Release CLI-only binaries via `cargo-dist`**

`cargo dist init` → configures separate CLI releases (no GUI) for `brew`, `cargo install`, `.deb`, `.rpm`.

- [ ] **Step 3: Commit**

### Task 9.2: Code signing

- [ ] **Step 1: macOS — Apple Developer ID + notarization**

Secrets: `APPLE_CERTIFICATE` (base64 p12), `APPLE_CERTIFICATE_PASSWORD`, `APPLE_ID`, `APPLE_PASSWORD` (app-specific), `APPLE_TEAM_ID`. Documented in `rust/orpheus-desktop/RELEASE.md`.

- [ ] **Step 2: Windows — EV code-signing cert**

Procurement lead time 2–4 weeks — start the day this plan is approved. Secrets: `WINDOWS_CERTIFICATE`, `WINDOWS_CERTIFICATE_PASSWORD`.

- [ ] **Step 3: Linux — GPG-sign `.deb`/`.AppImage`**

- [ ] **Step 4: Tag `v0.1.0-rc1` and verify the release workflow produces signed artefacts on all three platforms**

- [ ] **Step 5: Commit release docs + workflow**

```bash
git add .github/workflows/release.yml rust/orpheus-desktop/RELEASE.md
git commit -m "orpheus: add cross-platform signed release workflow"
```

---

## Phase 10 — Parity verification + Python retirement

### Task 10.1: Golden parity test

**Files:** `rust/orpheus-core/tests/parity.rs`, `scripts/generate_python_golden.sh`

- [ ] **Step 1: Shell script that runs the Python `orpheus` against `tests/fixtures/demo-wallets/` with `--output json --provider mock` and saves to `tests/fixtures/parity/python_golden.json`**

- [ ] **Step 2: Rust integration test that runs the Rust scanner against the same directory and `assert_eq!`s the JSON (modulo key ordering)**

- [ ] **Step 3: Run both mnemonic derivations — the Breadwallet-regression mnemonic and the BIP39 reference `"abandon ... about"` — side-by-side and assert identical `ExtractedKey` sets**

- [ ] **Step 4: Commit**

```bash
git add rust/orpheus-core/tests/parity.rs scripts/generate_python_golden.sh tests/fixtures/parity/
git commit -m "orpheus: add Python↔Rust parity golden test"
```

### Task 10.2: Real-recovery validation

- [ ] **Step 1: Run the Rust `orpheus scan` against three real past-recovery fixtures (kept private, not committed)**
- [ ] **Step 2: Confirm every `ExtractedKey` matches the Python output byte-for-byte (WIF, addresses, derivation paths)**
- [ ] **Step 3: Document outcomes in `docs/superpowers/specs/` under a new `2026-0X-XX-rust-parity-validation.md`**

### Task 10.3: Retire Python tree

Only execute after 10.1 and 10.2 pass.

- [ ] **Step 1: Create branch `legacy-python` at the current `main` HEAD and push it**

```bash
git branch legacy-python main
git push origin legacy-python
```

- [ ] **Step 2: On `main`, delete `src/orpheus/` and the Python-only tests**

```bash
git rm -r src/orpheus tests/test_*.py pyproject.toml uv.lock
```

Keep `tests/fixtures/` — the Rust tests still use them.

- [ ] **Step 3: Update [README.md](../../../README.md) — install instructions now `brew install orpheus` / `cargo install orpheus-cli` / GitHub Releases**

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "orpheus: retire Python tree, Rust is now the primary implementation"
```

- [ ] **Step 5: Tag `v0.1.0` and ship the first public signed release**

---

## Execution order notes

- **Phases 0–5 are the critical path** (core library). Everything else stacks on top.
- **Phase 6 (CLI) unblocks Phase 10 parity testing** — do before Phase 7/8.
- **Phases 7–8 (desktop) and Phase 9 (packaging) can be interleaved** once Phase 6 is green.
- **Phase 10 gates the Python retirement** — nothing removed from `src/orpheus/` until parity is proven.

## Risk watch

- **Breadwallet regression** (Phase 3.3) — if this fails, stop immediately. The port is not acceptable if this vector doesn't pass.
- **`blockchain_com` V2/V3/V4/V5** (Phase 3.6) — highest line count, most subtle bugs. One commit per sub-version and run all Python vectors on each.
- **MultiBit `.proto`** (Phase 3.5) — verify against the real fixture in `tests/fixtures/` before trusting the schema.
- **Windows EV cert** (Phase 9.2) — weeks of lead time; start procurement the day this plan is approved, in parallel with Phase 0.
