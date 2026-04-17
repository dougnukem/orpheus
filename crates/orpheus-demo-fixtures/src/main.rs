//! Generator for Orpheus demo fixtures (fake wallets + mock balances).
//!
//! Run with `cargo run -p orpheus-demo-fixtures`. Output lands in
//! `fixtures/demo-wallets/` and `fixtures/mock_balances.json` at workspace root.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use aes::cipher::{BlockEncryptMut, KeyIvInit, block_padding::Pkcs7};
use anyhow::Result;
use bip39::{Language, Mnemonic};
use orpheus_core::crypto::{
    addresses_for_privkey, derive_from_seed, privkey_to_wif, scrypt_aes_key,
};

const DEMO_MNEMONIC: &str =
    "legal winner thank year wave sausage worth useful legal winner thank yellow";

const DER_PATTERN: &[u8] = b"\x30\x81\xd3\x02\x01\x01\x04\x20";
const UNENCRYPTED_TAG: &[u8] = b"\x12\x20";
const IV_TAG: &[u8] = b"\x0a\x10";
const DATA_TAG: &[u8] = b"\x12\x30";

fn main() -> Result<()> {
    let root = locate_workspace_root()?;
    let fixtures = root.join("fixtures");
    let demo_dir = fixtures.join("demo-wallets");
    std::fs::create_dir_all(&demo_dir)?;

    let mnemonic = Mnemonic::parse_in(Language::English, DEMO_MNEMONIC)?;
    let seed = mnemonic.to_seed("");
    let mut privs: Vec<[u8; 32]> = Vec::with_capacity(5);
    for i in 0..5 {
        let child = derive_from_seed(&seed, &format!("m/44'/0'/0'/0/{i}"))
            .map_err(|e| anyhow::anyhow!(e))?;
        privs.push(child.private_key.secret_bytes());
    }

    std::fs::write(
        demo_dir.join("demo_bitcoin_core.wallet.dat"),
        build_bdb(&privs[..2]),
    )?;
    std::fs::write(
        demo_dir.join("demo_multibit.wallet"),
        build_multibit_unencrypted(&privs[2..3]),
    )?;
    std::fs::write(
        demo_dir.join("demo_multibit_protected.wallet"),
        build_multibit_encrypted(&privs[3..4], "orpheus-demo"),
    )?;
    std::fs::write(
        demo_dir.join("demo_descriptor_dump.json"),
        build_descriptor_dump(&privs[4..])?,
    )?;
    std::fs::write(demo_dir.join("demo_seed.txt"), DEMO_MNEMONIC)?;

    // Build mock balances — include a 0.03865052 BTC homage to the original
    // recovery session on the first key's compressed P2PKH address.
    let mut balances: BTreeMap<String, serde_json::Value> = BTreeMap::new();
    for (i, priv_bytes) in privs.iter().enumerate() {
        let a = addresses_for_privkey(priv_bytes).map_err(|e| anyhow::anyhow!(e))?;
        let addrs = [
            ("p2pkh_compressed", a.p2pkh_compressed),
            ("p2pkh_uncompressed", a.p2pkh_uncompressed),
            ("p2sh_p2wpkh", a.p2sh_p2wpkh),
            ("bech32", a.bech32),
        ];
        for (label, addr) in addrs {
            let entry = if i == 0 && label == "p2pkh_compressed" {
                // 0.03865052 BTC homage: 0.05 received, 0.01134948 spent,
                // 0.03865052 remaining.
                serde_json::json!({
                    "balance_sat": 3_865_052u64,
                    "total_received_sat": 5_000_000u64,
                    "total_sent_sat": 1_134_948u64,
                    "tx_count": 4u64
                })
            } else if i == 2 && label == "bech32" {
                serde_json::json!({
                    "balance_sat": 1_000_000u64,
                    "total_received_sat": 1_000_000u64,
                    "total_sent_sat": 0u64,
                    "tx_count": 1u64
                })
            } else if i == 3 && label == "p2pkh_compressed" {
                serde_json::json!({
                    "balance_sat": 500_000u64,
                    "total_received_sat": 500_000u64,
                    "total_sent_sat": 0u64,
                    "tx_count": 2u64
                })
            } else if i == 4 && label == "p2pkh_compressed" {
                // History but empty — exercises the "spent (empty)" counter.
                serde_json::json!({
                    "balance_sat": 0u64,
                    "total_received_sat": 250_000u64,
                    "total_sent_sat": 250_000u64,
                    "tx_count": 3u64
                })
            } else {
                serde_json::json!({
                    "balance_sat": 0u64,
                    "total_received_sat": 0u64,
                    "total_sent_sat": 0u64,
                    "tx_count": 0u64
                })
            };
            balances.insert(addr, entry);
        }
    }
    std::fs::write(
        fixtures.join("mock_balances.json"),
        serde_json::to_string_pretty(&balances)?,
    )?;

    println!("wrote demo fixtures to {}", demo_dir.display());
    println!(
        "wrote mock balances to {}",
        fixtures.join("mock_balances.json").display()
    );
    Ok(())
}

fn build_bdb(privs: &[[u8; 32]]) -> Vec<u8> {
    let mut blob = Vec::new();
    blob.extend_from_slice(b"\x62\x31\x05\x00\x00\x00\x00\x00");
    blob.extend_from_slice(&[0u8; 120]);
    blob.extend_from_slice(b"main\0");
    for p in privs {
        blob.extend_from_slice(&[0u8; 16]);
        blob.extend_from_slice(DER_PATTERN);
        blob.extend_from_slice(p);
        blob.extend_from_slice(&[0u8; 32]);
    }
    blob
}

fn build_multibit_unencrypted(privs: &[[u8; 32]]) -> Vec<u8> {
    let mut blob = Vec::new();
    blob.extend_from_slice(b"org.bitcoin.production");
    blob.extend_from_slice(&[0u8; 8]);
    for p in privs {
        blob.extend_from_slice(b"\x0a\x21\x02");
        blob.extend_from_slice(&[0u8; 32]);
        blob.extend_from_slice(UNENCRYPTED_TAG);
        blob.extend_from_slice(p);
    }
    blob
}

fn build_multibit_encrypted(privs: &[[u8; 32]], password: &str) -> Vec<u8> {
    let salt = [0x11u8; 8];
    let aes_key = scrypt_aes_key(password.as_bytes(), &salt).unwrap();
    let mut blob = Vec::new();
    blob.extend_from_slice(b"org.bitcoin.production");
    blob.extend_from_slice(&[0u8; 4]);
    blob.extend_from_slice(b"\x0a\x08");
    blob.extend_from_slice(&salt);
    blob.extend_from_slice(b"\x10\x80\x80\x01");
    for (i, p) in privs.iter().enumerate() {
        let iv = [(0x22u8 ^ i as u8); 16];
        type Cbc = cbc::Encryptor<aes::Aes256>;
        let ct: Vec<u8> =
            Cbc::new((&aes_key).into(), (&iv).into()).encrypt_padded_vec_mut::<Pkcs7>(p);
        blob.extend_from_slice(IV_TAG);
        blob.extend_from_slice(&iv);
        blob.extend_from_slice(DATA_TAG);
        blob.extend_from_slice(&ct[..48]);
    }
    blob
}

fn build_descriptor_dump(privs: &[[u8; 32]]) -> Result<String> {
    let mut arr = Vec::new();
    for (i, p) in privs.iter().enumerate() {
        let wif = privkey_to_wif(p, true).map_err(|e| anyhow::anyhow!(e))?;
        arr.push(serde_json::json!({
            "desc": format!("wpkh({wif})#checksum"),
            "timestamp": 1_700_000_000u64 + i as u64,
        }));
    }
    Ok(serde_json::to_string_pretty(&serde_json::json!({
        "descriptors": arr,
    }))?)
}

fn locate_workspace_root() -> Result<PathBuf> {
    let here = Path::new(env!("CARGO_MANIFEST_DIR"));
    for ancestor in here.ancestors() {
        if ancestor.join("Cargo.toml").exists()
            && std::fs::read_to_string(ancestor.join("Cargo.toml"))
                .unwrap_or_default()
                .contains("[workspace]")
        {
            return Ok(ancestor.to_path_buf());
        }
    }
    anyhow::bail!("workspace root not found from {}", here.display())
}

// silence unused-imports warning when hmac isn't touched
#[allow(dead_code)]
fn _unused(_: &dyn std::io::Write) {}
