//! Bitcoin Core passphrase-encrypted wallet extractor (`mkey` / `ckey`).
//!
//! When a Bitcoin Core wallet is encrypted, the private keys are no longer
//! stored as DER blobs — [`super::bitcoin_core`] finds nothing and the wallet
//! looks empty. It is not empty; it is locked. This module opens it.
//!
//! The scheme, from Bitcoin Core's `CCrypter`:
//!
//! 1. An `mkey` record holds a *master key* encrypted under the passphrase:
//!    `(vchCryptedKey, vchSalt, nDerivationMethod, nDeriveIterations, …)`.
//! 2. The passphrase is stretched with an iterated SHA-512:
//!    `buf = SHA512(passphrase || salt)`, then `buf = SHA512(buf)` a further
//!    `nDeriveIterations - 1` times. `key = buf[0..32]`, `iv = buf[32..48]`.
//! 3. AES-256-CBC/PKCS7 decrypts `vchCryptedKey` into the 32-byte master key.
//! 4. Each `ckey` record pairs a public key with its encrypted secret. The
//!    master key decrypts it with `iv = SHA256d(pubkey)[0..16]`.
//!
//! Every decrypted secret is verified by re-deriving its public key and
//! comparing against the `ckey`'s stored pubkey, so a wrong passphrase can
//! never produce a plausible-looking wrong answer.

use std::path::Path;

use secp256k1::{Secp256k1, SecretKey};
use sha2::{Digest, Sha256, Sha512};

use crate::{
    crypto::{addresses_for_privkey, aes_cbc_decrypt, privkey_to_wif},
    extractors::{Extractor, scan_result_error},
    models::{ExtractedKey, SourceType, WalletScanResult},
};

/// Length-prefixed record-key markers Bitcoin Core writes.
const MKEY_MARKER: &[u8] = b"\x04mkey";
const CKEY_MARKER: &[u8] = b"\x04ckey";

/// `vchCryptedKey` and `vchCryptedSecret` are both 32 bytes of plaintext under
/// PKCS7, so 48 bytes on disk, stored with a `0x30` compact-size prefix.
const CRYPTED_LEN: usize = 48;
const CRYPTED_PREFIX: u8 = 0x30;
/// `vchSalt` is 8 bytes, prefixed `0x08`.
const SALT_LEN: usize = 8;
const SALT_PREFIX: u8 = 0x08;

/// Guard rails on `nDeriveIterations`, which Bitcoin Core calibrates to about
/// 100ms of work. Anything outside this is a false structural match.
const MIN_ITERATIONS: u32 = 1_000;
const MAX_ITERATIONS: u32 = 50_000_000;

/// An `mkey` record: the wallet's master key, encrypted under the passphrase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MasterKeyRecord {
    pub encrypted_key: Vec<u8>,
    pub salt: Vec<u8>,
    pub derivation_method: u32,
    pub iterations: u32,
}

/// A `ckey` record: one public key and its encrypted private counterpart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CryptedKeyRecord {
    pub pubkey: Vec<u8>,
    pub ciphertext: Vec<u8>,
}

/// Locate every `mkey` value in a wallet image.
///
/// Matched structurally rather than by following the record key, because in a
/// Berkeley DB page the key and its value are separate items. The value layout
/// — `0x30 <48> 0x08 <8> <u32 method> <u32 iterations> 0x00` — is specific
/// enough to be self-validating.
#[must_use]
pub fn find_master_keys(data: &[u8]) -> Vec<MasterKeyRecord> {
    // 1 + 48 + 1 + 8 + 4 + 4 + 1
    const RECORD_LEN: usize = 67;
    let mut out = Vec::new();
    if data.len() < RECORD_LEN {
        return out;
    }
    for i in 0..=(data.len() - RECORD_LEN) {
        if data[i] != CRYPTED_PREFIX || data[i + 1 + CRYPTED_LEN] != SALT_PREFIX {
            continue;
        }
        let method_at = i + 1 + CRYPTED_LEN + 1 + SALT_LEN;
        let method = u32::from_le_bytes(
            data[method_at..method_at + 4]
                .try_into()
                .expect("4-byte slice"),
        );
        let iterations = u32::from_le_bytes(
            data[method_at + 4..method_at + 8]
                .try_into()
                .expect("4-byte slice"),
        );
        // nDerivationMethod 0 == EVP_sha512; it is the only one Core ever wrote.
        // The trailing 0x00 is the empty vchOtherDerivationParameters.
        if method != 0
            || !(MIN_ITERATIONS..=MAX_ITERATIONS).contains(&iterations)
            || data[method_at + 8] != 0
        {
            continue;
        }
        let rec = MasterKeyRecord {
            encrypted_key: data[i + 1..i + 1 + CRYPTED_LEN].to_vec(),
            salt: data[i + 2 + CRYPTED_LEN..i + 2 + CRYPTED_LEN + SALT_LEN].to_vec(),
            derivation_method: method,
            iterations,
        };
        if !out.contains(&rec) {
            out.push(rec);
        }
    }
    out
}

/// Locate every `ckey` record, using whichever layout the backing store uses.
///
/// SQLite descriptor wallets store a row's key and value contiguously, so the
/// pubkey and ciphertext sit next to the marker. Berkeley DB does not — its
/// leaf pages hold keys and values as separate indexed items — so that case
/// needs a page walk. Both are attempted and the results merged, because a
/// single hunt sees both kinds.
#[must_use]
pub fn find_crypted_keys(data: &[u8]) -> Vec<CryptedKeyRecord> {
    let mut out = scan_contiguous_ckeys(data);
    for rec in scan_bdb_ckeys(data) {
        if !out.iter().any(|r| r.pubkey == rec.pubkey) {
            out.push(rec);
        }
    }
    out
}

/// `\x04ckey <len> <pubkey> 0x30 <48-byte ciphertext>` laid out back to back.
fn scan_contiguous_ckeys(data: &[u8]) -> Vec<CryptedKeyRecord> {
    let mut out: Vec<CryptedKeyRecord> = Vec::new();
    let mut i = 0usize;
    while i + MKEY_MARKER.len() < data.len() {
        let Some(pos) = find_from(data, CKEY_MARKER, i) else {
            break;
        };
        i = pos + CKEY_MARKER.len();
        let Some(&len_byte) = data.get(i) else { break };
        let pub_len = match len_byte {
            0x21 => 33,
            0x41 => 65,
            _ => continue,
        };
        let pub_start = i + 1;
        let ct_prefix = pub_start + pub_len;
        if ct_prefix + 1 + CRYPTED_LEN > data.len() || data[ct_prefix] != CRYPTED_PREFIX {
            continue;
        }
        let rec = CryptedKeyRecord {
            pubkey: data[pub_start..ct_prefix].to_vec(),
            ciphertext: data[ct_prefix + 1..ct_prefix + 1 + CRYPTED_LEN].to_vec(),
        };
        if !out.iter().any(|r| r.pubkey == rec.pubkey) {
            out.push(rec);
        }
    }
    out
}

// -- Berkeley DB page walking ----------------------------------------------

/// Bytes of BDB page header before the item-offset array.
const BDB_PAGE_HEADER: usize = 26;
/// `P_LBTREE` — a btree leaf page, the only page type holding wallet records.
const BDB_P_LBTREE: u8 = 5;
/// Every Bitcoin Core wallet is written with 4 KiB pages.
const BDB_PAGE_SIZE: usize = 4096;
/// `BKEYDATA` item header: `u16 len`, `u8 type`.
const BDB_ITEM_HEADER: usize = 3;

/// Walk BDB leaf pages, pairing each key item with the data item that follows
/// it in the page's offset array.
fn scan_bdb_ckeys(data: &[u8]) -> Vec<CryptedKeyRecord> {
    let mut out: Vec<CryptedKeyRecord> = Vec::new();

    for page in data.chunks_exact(BDB_PAGE_SIZE) {
        if page[25] != BDB_P_LBTREE {
            continue;
        }
        let entries = u16::from_le_bytes([page[20], page[21]]) as usize;
        // Entries come in (key, data) pairs; a page claiming more entries than
        // could fit is corrupt or misidentified.
        if entries < 2
            || !entries.is_multiple_of(2)
            || BDB_PAGE_HEADER + entries * 2 > BDB_PAGE_SIZE
        {
            continue;
        }

        let item = |idx: usize| -> Option<&[u8]> {
            let off_at = BDB_PAGE_HEADER + idx * 2;
            let off = u16::from_le_bytes([page[off_at], page[off_at + 1]]) as usize;
            if off + BDB_ITEM_HEADER > BDB_PAGE_SIZE {
                return None;
            }
            let len = u16::from_le_bytes([page[off], page[off + 1]]) as usize;
            let start = off + BDB_ITEM_HEADER;
            if start + len > BDB_PAGE_SIZE {
                return None;
            }
            Some(&page[start..start + len])
        };

        for pair in 0..entries / 2 {
            let (Some(key), Some(value)) = (item(pair * 2), item(pair * 2 + 1)) else {
                continue;
            };
            if !key.starts_with(CKEY_MARKER) {
                continue;
            }
            let rest = &key[CKEY_MARKER.len()..];
            let Some((&len_byte, pubkey)) = rest.split_first() else {
                continue;
            };
            let pub_len = match len_byte {
                0x21 => 33,
                0x41 => 65,
                _ => continue,
            };
            if pubkey.len() < pub_len {
                continue;
            }
            // The value is the length-prefixed ciphertext.
            let ct = match value.split_first() {
                Some((&CRYPTED_PREFIX, tail)) if tail.len() >= CRYPTED_LEN => &tail[..CRYPTED_LEN],
                _ if value.len() == CRYPTED_LEN => value,
                _ => continue,
            };
            let rec = CryptedKeyRecord {
                pubkey: pubkey[..pub_len].to_vec(),
                ciphertext: ct.to_vec(),
            };
            if !out.iter().any(|r| r.pubkey == rec.pubkey) {
                out.push(rec);
            }
        }
    }
    out
}

fn find_from(haystack: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if from >= haystack.len() || needle.is_empty() {
        return None;
    }
    haystack[from..]
        .windows(needle.len())
        .position(|w| w == needle)
        .map(|p| p + from)
}

// -- the crypto ------------------------------------------------------------

/// Bitcoin Core's `BytesToKeySHA512AES`: one SHA-512 over `passphrase || salt`,
/// then `iterations - 1` further SHA-512 rounds over the digest itself.
#[must_use]
pub fn derive_key_iv(passphrase: &[u8], salt: &[u8], iterations: u32) -> ([u8; 32], [u8; 16]) {
    let mut hasher = Sha512::new();
    hasher.update(passphrase);
    hasher.update(salt);
    let mut buf = hasher.finalize();
    for _ in 0..iterations.saturating_sub(1) {
        buf = Sha512::digest(buf);
    }
    let mut key = [0u8; 32];
    let mut iv = [0u8; 16];
    key.copy_from_slice(&buf[..32]);
    iv.copy_from_slice(&buf[32..48]);
    (key, iv)
}

/// Recover the 32-byte wallet master key, or `None` if the passphrase is wrong.
#[must_use]
pub fn decrypt_master_key(record: &MasterKeyRecord, passphrase: &str) -> Option<[u8; 32]> {
    if record.derivation_method != 0 {
        return None;
    }
    let (key, iv) = derive_key_iv(passphrase.as_bytes(), &record.salt, record.iterations);
    let plain = aes_cbc_decrypt(&key, &iv, &record.encrypted_key)?;
    plain.try_into().ok()
}

/// SHA-256 applied twice — Bitcoin's `Hash()`.
fn sha256d(data: &[u8]) -> [u8; 32] {
    let once = Sha256::digest(data);
    Sha256::digest(once).into()
}

/// Decrypt one `ckey` under the master key, verifying the result really is the
/// private counterpart of the record's public key.
///
/// The verification is what makes a wrong passphrase fail loudly. AES-CBC with
/// PKCS7 will occasionally "succeed" on garbage; a secp256k1 pubkey that
/// matches the one Bitcoin Core stored will not.
#[must_use]
pub fn decrypt_crypted_key(master: &[u8; 32], record: &CryptedKeyRecord) -> Option<[u8; 32]> {
    let hash = sha256d(&record.pubkey);
    let mut iv = [0u8; 16];
    iv.copy_from_slice(&hash[..16]);

    let plain = aes_cbc_decrypt(master, &iv, &record.ciphertext)?;
    let secret: [u8; 32] = plain.try_into().ok()?;

    let sk = SecretKey::from_slice(&secret).ok()?;
    let secp = Secp256k1::new();
    let pk = secp256k1::PublicKey::from_secret_key(&secp, &sk);
    let matches = match record.pubkey.len() {
        33 => pk.serialize().as_slice() == record.pubkey.as_slice(),
        65 => pk.serialize_uncompressed().as_slice() == record.pubkey.as_slice(),
        _ => false,
    };
    matches.then_some(secret)
}

/// Try every passphrase against every master key, returning the first that
/// works along with the passphrase that opened it.
#[must_use]
pub fn unlock<'a>(
    masters: &[MasterKeyRecord],
    passwords: &'a [String],
) -> Option<([u8; 32], &'a str)> {
    for pw in passwords {
        for mk in masters {
            if let Some(master) = decrypt_master_key(mk, pw) {
                return Some((master, pw.as_str()));
            }
        }
    }
    None
}

pub struct EncryptedBitcoinCoreExtractor;

impl Extractor for EncryptedBitcoinCoreExtractor {
    fn source_type(&self) -> SourceType {
        SourceType::Encrypted
    }

    /// Never auto-detected by name. [`crate::discovery`] identifies these from
    /// their `mkey`/`ckey` records and dispatches on format, and `scan` reaches
    /// them through the ordinary Bitcoin Core path.
    fn can_handle(&self, _path: &Path) -> bool {
        false
    }

    fn extract(&self, path: &Path, passwords: &[String]) -> WalletScanResult {
        let data = match std::fs::read(path) {
            Ok(d) => d,
            Err(e) => return scan_result_error(path, self.source_type(), e),
        };
        let source = path.display().to_string();

        let masters = find_master_keys(&data);
        let crypted = find_crypted_keys(&data);

        if masters.is_empty() || crypted.is_empty() {
            return WalletScanResult {
                source_file: source,
                source_type: self.source_type(),
                keys: vec![],
                error: None,
            };
        }

        // A decrypted master key is useless without the passphrase that made
        // it, so report the shape of the problem when nothing opens it.
        let Some((master, _password)) = unlock(&masters, passwords) else {
            return WalletScanResult {
                source_file: source,
                source_type: self.source_type(),
                keys: vec![],
                error: Some(format!(
                    "encrypted Bitcoin Core wallet: {} master key(s), {} encrypted key(s); \
                     none of the {} supplied password(s) worked",
                    masters.len(),
                    crypted.len(),
                    passwords.len()
                )),
            };
        };

        let mut keys = Vec::new();
        for rec in &crypted {
            let Some(secret) = decrypt_crypted_key(&master, rec) else {
                continue;
            };
            let compressed = rec.pubkey.len() == 33;
            let (Ok(addrs), Ok(wif)) = (
                addresses_for_privkey(&secret),
                privkey_to_wif(&secret, compressed),
            ) else {
                continue;
            };
            keys.push(ExtractedKey {
                wif,
                address_compressed: addrs.p2pkh_compressed.clone(),
                address_uncompressed: Some(addrs.p2pkh_uncompressed.clone()),
                address_p2sh_segwit: Some(addrs.p2sh_p2wpkh.clone()),
                address_bech32: Some(addrs.bech32.clone()),
                source_file: source.clone(),
                source_type: SourceType::BitcoinCore,
                derivation_path: None,
                balance_sat: None,
                total_received_sat: None,
                total_sent_sat: None,
                tx_count: None,
                notes: Some("decrypted from Bitcoin Core ckey".into()),
            });
        }

        WalletScanResult {
            source_file: source,
            source_type: self.source_type(),
            keys,
            error: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aes::cipher::{BlockModeEncrypt, KeyIvInit, block_padding::Pkcs7};

    type CbcEnc = cbc::Encryptor<aes::Aes256>;

    fn aes_encrypt(key: &[u8; 32], iv: &[u8; 16], plain: &[u8]) -> Vec<u8> {
        CbcEnc::new(key.into(), iv.into()).encrypt_padded_vec::<Pkcs7>(plain)
    }

    /// The BIP32 test-vector private key, reused here as a stand-in secret.
    const SECRET_HEX: &str = "0c28fca386c7a227600b2fe50b7cae11ec86d3bf1fbe471be89827e19d72aa1d";

    fn secret_bytes() -> [u8; 32] {
        let mut b = [0u8; 32];
        hex::decode_to_slice(SECRET_HEX, &mut b).unwrap();
        b
    }

    /// Kept at the low end of what [`find_master_keys`] considers plausible so
    /// the fixtures stay fast. Real wallets sit far higher — the one this was
    /// built against used 233,252 — and each password attempt there costs a
    /// meaningful fraction of a second of SHA-512.
    const TEST_ITERS: u32 = MIN_ITERATIONS;

    fn build_master(password: &str, master_key: &[u8; 32]) -> MasterKeyRecord {
        let salt = [0x11u8; 8];
        let (kek, iv) = derive_key_iv(password.as_bytes(), &salt, TEST_ITERS);
        MasterKeyRecord {
            encrypted_key: aes_encrypt(&kek, &iv, master_key),
            salt: salt.to_vec(),
            derivation_method: 0,
            iterations: TEST_ITERS,
        }
    }

    fn build_ckey(master: &[u8; 32], secret: &[u8; 32], compressed: bool) -> CryptedKeyRecord {
        let secp = Secp256k1::new();
        let sk = SecretKey::from_slice(secret).unwrap();
        let pk = secp256k1::PublicKey::from_secret_key(&secp, &sk);
        let pubkey = if compressed {
            pk.serialize().to_vec()
        } else {
            pk.serialize_uncompressed().to_vec()
        };
        let hash = sha256d(&pubkey);
        let mut iv = [0u8; 16];
        iv.copy_from_slice(&hash[..16]);
        CryptedKeyRecord {
            pubkey,
            ciphertext: aes_encrypt(master, &iv, secret),
        }
    }

    /// Pins the KDF against a fixed input. If this drifts, every wallet this
    /// tool has ever opened stops opening.
    #[test]
    fn kdf_is_deterministic_and_salt_sensitive() {
        let (k1, iv1) = derive_key_iv(b"orpheus", &[0x11; 8], 16);
        let (k2, iv2) = derive_key_iv(b"orpheus", &[0x11; 8], 16);
        assert_eq!(k1, k2);
        assert_eq!(iv1, iv2);

        let (k3, _) = derive_key_iv(b"orpheus", &[0x22; 8], 16);
        assert_ne!(k1, k3, "a different salt must give a different key");

        let (k4, _) = derive_key_iv(b"orpheus", &[0x11; 8], 17);
        assert_ne!(
            k1, k4,
            "a different iteration count must give a different key"
        );
    }

    /// One SHA-512 round must equal `SHA512(passphrase || salt)` exactly —
    /// this is where an off-by-one against Core's loop would show up.
    #[test]
    fn kdf_single_round_matches_plain_sha512() {
        let mut h = Sha512::new();
        h.update(b"pw");
        h.update([0xAAu8; 8]);
        let expect = h.finalize();
        let (key, iv) = derive_key_iv(b"pw", &[0xAA; 8], 1);
        assert_eq!(&key[..], &expect[..32]);
        assert_eq!(&iv[..], &expect[32..48]);
    }

    #[test]
    fn master_key_round_trips_with_the_right_password() {
        let master = [0x42u8; 32];
        let rec = build_master("correct horse", &master);
        assert_eq!(decrypt_master_key(&rec, "correct horse"), Some(master));
    }

    #[test]
    fn wrong_password_does_not_yield_the_master_key() {
        let master = [0x42u8; 32];
        let rec = build_master("correct horse", &master);
        assert_ne!(
            decrypt_master_key(&rec, "wrong horse"),
            Some(master),
            "a wrong passphrase must not recover the master key"
        );
    }

    #[test]
    fn ckey_round_trips_and_verifies_against_its_pubkey() {
        let master = [0x42u8; 32];
        let secret = secret_bytes();
        for compressed in [true, false] {
            let rec = build_ckey(&master, &secret, compressed);
            assert_eq!(
                decrypt_crypted_key(&master, &rec),
                Some(secret),
                "compressed={compressed}"
            );
        }
    }

    /// The pubkey check is the guard against a wrong master key producing a
    /// plausible 32 bytes. Without it, a bad passphrase could look like a hit.
    #[test]
    fn ckey_rejects_a_wrong_master_key() {
        let secret = secret_bytes();
        let rec = build_ckey(&[0x42u8; 32], &secret, true);
        assert_eq!(
            decrypt_crypted_key(&[0x43u8; 32], &rec),
            None,
            "a wrong master key must be rejected by the pubkey check"
        );
    }

    #[test]
    fn finds_master_key_structure_in_surrounding_noise() {
        let master = [0x42u8; 32];
        let rec = build_master("pw", &master);

        let mut blob = vec![0xEEu8; 512];
        blob.extend_from_slice(b"\x04mkey\x01\x00\x00\x00");
        blob.push(CRYPTED_PREFIX);
        blob.extend_from_slice(&rec.encrypted_key);
        blob.push(SALT_PREFIX);
        blob.extend_from_slice(&rec.salt);
        blob.extend_from_slice(&rec.derivation_method.to_le_bytes());
        blob.extend_from_slice(&rec.iterations.to_le_bytes());
        blob.push(0x00);
        blob.extend_from_slice(&[0xEEu8; 512]);

        let found = find_master_keys(&blob);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0], rec);
        assert_eq!(decrypt_master_key(&found[0], "pw"), Some(master));
    }

    #[test]
    fn ignores_implausible_iteration_counts() {
        let mut blob = vec![CRYPTED_PREFIX];
        blob.extend_from_slice(&[0u8; CRYPTED_LEN]);
        blob.push(SALT_PREFIX);
        blob.extend_from_slice(&[0u8; SALT_LEN]);
        blob.extend_from_slice(&0u32.to_le_bytes()); // method 0
        blob.extend_from_slice(&7u32.to_le_bytes()); // 7 iterations — nonsense
        blob.push(0x00);
        assert!(
            find_master_keys(&blob).is_empty(),
            "structural noise must not be read as a master key"
        );
    }

    /// SQLite descriptor wallets lay the record key and value out contiguously.
    #[test]
    fn finds_contiguous_ckey_records() {
        let master = [0x42u8; 32];
        let rec = build_ckey(&master, &secret_bytes(), true);

        let mut blob = vec![0x00u8; 64];
        blob.extend_from_slice(CKEY_MARKER);
        blob.push(0x21);
        blob.extend_from_slice(&rec.pubkey);
        blob.push(CRYPTED_PREFIX);
        blob.extend_from_slice(&rec.ciphertext);
        blob.extend_from_slice(&[0x00u8; 64]);

        let found = find_crypted_keys(&blob);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0], rec);
        assert_eq!(
            decrypt_crypted_key(&master, &found[0]),
            Some(secret_bytes())
        );
    }

    /// Berkeley DB keeps keys and values as separate page items, so the
    /// contiguous scan finds nothing and the page walk has to do the work.
    #[test]
    fn finds_bdb_ckey_records_across_split_page_items() {
        let master = [0x42u8; 32];
        let rec = build_ckey(&master, &secret_bytes(), true);

        let mut key_item = CKEY_MARKER.to_vec();
        key_item.push(0x21);
        key_item.extend_from_slice(&rec.pubkey);
        let mut value_item = vec![CRYPTED_PREFIX];
        value_item.extend_from_slice(&rec.ciphertext);

        let mut page = vec![0u8; BDB_PAGE_SIZE];
        page[25] = BDB_P_LBTREE;
        page[20..22].copy_from_slice(&2u16.to_le_bytes()); // one key/data pair

        // Items live at the tail of the page; offsets are recorded in inp[].
        let key_off = 2048usize;
        let val_off = 3072usize;
        page[BDB_PAGE_HEADER..BDB_PAGE_HEADER + 2].copy_from_slice(&(key_off as u16).to_le_bytes());
        page[BDB_PAGE_HEADER + 2..BDB_PAGE_HEADER + 4]
            .copy_from_slice(&(val_off as u16).to_le_bytes());

        let put = |off: usize, item: &[u8], page: &mut Vec<u8>| {
            page[off..off + 2].copy_from_slice(&(item.len() as u16).to_le_bytes());
            page[off + 2] = 1; // B_KEYDATA
            page[off + 3..off + 3 + item.len()].copy_from_slice(item);
        };
        put(key_off, &key_item, &mut page);
        put(val_off, &value_item, &mut page);

        assert!(
            scan_contiguous_ckeys(&page).is_empty(),
            "precondition: the split layout defeats the contiguous scan"
        );
        let found = find_crypted_keys(&page);
        assert_eq!(found.len(), 1, "the BDB page walk must recover it");
        assert_eq!(found[0], rec);
        assert_eq!(
            decrypt_crypted_key(&master, &found[0]),
            Some(secret_bytes())
        );
    }

    /// End to end: build an encrypted wallet image, then open it with a
    /// password list that contains the right passphrase among wrong ones.
    #[test]
    fn extracts_from_a_synthetic_encrypted_wallet() {
        let master = [0x42u8; 32];
        let password = "orpheus-demo";
        let mrec = build_master(password, &master);
        let secret = secret_bytes();
        let crec = build_ckey(&master, &secret, true);

        let mut blob = Vec::new();
        blob.extend_from_slice(b"\x04mkey\x01\x00\x00\x00");
        blob.push(CRYPTED_PREFIX);
        blob.extend_from_slice(&mrec.encrypted_key);
        blob.push(SALT_PREFIX);
        blob.extend_from_slice(&mrec.salt);
        blob.extend_from_slice(&mrec.derivation_method.to_le_bytes());
        blob.extend_from_slice(&mrec.iterations.to_le_bytes());
        blob.push(0x00);
        blob.extend_from_slice(CKEY_MARKER);
        blob.push(0x21);
        blob.extend_from_slice(&crec.pubkey);
        blob.push(CRYPTED_PREFIX);
        blob.extend_from_slice(&crec.ciphertext);

        let path =
            std::env::temp_dir().join(format!("orpheus-enc-core-{}.dat", std::process::id()));
        std::fs::write(&path, &blob).unwrap();

        let ex = EncryptedBitcoinCoreExtractor;
        let wrong = vec!["nope".to_string(), "also-nope".to_string()];
        let locked = ex.extract(&path, &wrong);
        assert!(locked.keys.is_empty());
        assert!(
            locked
                .error
                .as_deref()
                .unwrap_or("")
                .contains("none of the 2"),
            "a locked wallet must say so, not report itself as empty"
        );

        let right = vec!["nope".to_string(), password.to_string()];
        let opened = ex.extract(&path, &right);
        assert_eq!(opened.keys.len(), 1);
        assert!(opened.error.is_none());
        let expected_wif = privkey_to_wif(&secret, true).unwrap();
        assert_eq!(opened.keys[0].wif, expected_wif);

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn unlock_reports_which_password_worked() {
        let master = [0x7u8; 32];
        let rec = build_master("second", &master);
        let pws = vec!["first".to_string(), "second".to_string()];
        let (recovered, used) = unlock(std::slice::from_ref(&rec), &pws).expect("unlocks");
        assert_eq!(recovered, master);
        assert_eq!(used, "second");

        assert!(unlock(&[rec], &["nope".to_string()]).is_none());
    }
}
