//! MultiBit Classic .wallet extractor — protobuf-ish scan.

use std::path::Path;

use crate::{
    crypto::aes_cbc_decrypt,
    extractors::{
        Extractor,
        bitcoin_core::{make_key, memmem, read_head},
    },
    models::{ExtractedKey, SourceType, WalletScanResult},
};

pub const UNENCRYPTED_TAG: &[u8] = b"\x12\x20";
pub const ENCRYPTED_IV_TAG: &[u8] = b"\x0a\x10";
pub const ENCRYPTED_DATA_TAG: &[u8] = b"\x12\x30";

pub struct MultibitExtractor;

impl Extractor for MultibitExtractor {
    fn source_type(&self) -> SourceType {
        SourceType::Multibit
    }

    fn can_handle(&self, path: &Path) -> bool {
        if !path.is_file() {
            return false;
        }
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        if ext != "wallet" && ext != "bak" {
            return false;
        }
        let Ok(head) = read_head(path, 512) else {
            return false;
        };
        memmem(&head, b"org.bitcoin") || memmem(&head, UNENCRYPTED_TAG)
    }

    fn extract(&self, path: &Path, _passwords: &[String]) -> WalletScanResult {
        let data = match std::fs::read(path) {
            Ok(d) => d,
            Err(e) => return super::scan_result_error(path, self.source_type(), e),
        };
        let keys = scan_unencrypted(&data, path);
        WalletScanResult {
            source_file: path.display().to_string(),
            source_type: self.source_type(),
            keys,
            error: None,
        }
    }
}

pub fn scan_unencrypted(data: &[u8], source_file: &Path) -> Vec<ExtractedKey> {
    let mut keys = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut pos = 0usize;
    while pos + UNENCRYPTED_TAG.len() + 32 <= data.len() {
        if &data[pos..pos + UNENCRYPTED_TAG.len()] == UNENCRYPTED_TAG {
            let start = pos + UNENCRYPTED_TAG.len();
            if let Ok(priv_bytes) = <[u8; 32]>::try_from(&data[start..start + 32])
                && seen.insert(priv_bytes)
                && let Some(k) = make_key(&priv_bytes, source_file, SourceType::Multibit)
            {
                keys.push(k);
            }
        }
        pos += 1;
    }
    keys
}

pub fn find_encrypted_entries(data: &[u8]) -> Vec<([u8; 16], [u8; 48])> {
    let mut entries = Vec::new();
    let mut pos = 0usize;
    while pos + 68 <= data.len() {
        if &data[pos..pos + 2] == ENCRYPTED_IV_TAG
            && &data[pos + 18..pos + 20] == ENCRYPTED_DATA_TAG
            && let (Ok(iv), Ok(ct)) = (
                <[u8; 16]>::try_from(&data[pos + 2..pos + 18]),
                <[u8; 48]>::try_from(&data[pos + 20..pos + 68]),
            )
        {
            entries.push((iv, ct));
        }
        pos += 1;
    }
    entries
}

/// Locate the MultiBit v3 scrypt salt (tag 1, len 8, followed by scrypt params).
pub fn find_scrypt_salt(data: &[u8]) -> Option<[u8; 8]> {
    let tag = b"\x0a\x08";
    let mut start = 0usize;
    while start + 11 < data.len() {
        if let Some(rel) = data[start..].windows(2).position(|w| w == tag) {
            let idx = start + rel;
            if idx + 10 < data.len() && matches!(data[idx + 10], 0x10 | 0x18 | 0x20) {
                let salt: [u8; 8] = data[idx + 2..idx + 10].try_into().ok()?;
                return Some(salt);
            }
            start = idx + 1;
        } else {
            break;
        }
    }
    None
}

/// Try to decrypt a single MultiBit v3 entry with the given password.
pub fn try_decrypt_multibit(
    password: &str,
    salt: &[u8; 8],
    iv: &[u8; 16],
    ciphertext: &[u8; 48],
) -> Option<[u8; 32]> {
    let aes_key = crate::crypto::scrypt_aes_key(password.as_bytes(), salt).ok()?;
    let plaintext = aes_cbc_decrypt(&aes_key, iv, ciphertext)?;
    if plaintext.len() != 32 {
        return None;
    }
    <[u8; 32]>::try_from(&plaintext[..]).ok().filter(|b| {
        // Reject obvious garbage — non-zero and under secp256k1 order
        let n = bitcoin::secp256k1::constants::CURVE_ORDER;
        let zero = [0u8; 32];
        *b != zero && b.as_slice() < n.as_slice()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn priv_hex(s: &str) -> [u8; 32] {
        let mut out = [0u8; 32];
        hex::decode_to_slice(s, &mut out).unwrap();
        out
    }

    #[test]
    fn scan_unencrypted_finds_planted_key() {
        let priv_bytes =
            priv_hex("0c28fca386c7a227600b2fe50b7cae11ec86d3bf1fbe471be89827e19d72aa1d");
        let mut blob = Vec::new();
        blob.extend_from_slice(b"org.bitcoin.production");
        blob.extend_from_slice(&[0u8; 16]);
        blob.extend_from_slice(UNENCRYPTED_TAG);
        blob.extend_from_slice(&priv_bytes);
        let keys = scan_unencrypted(&blob, std::path::Path::new("x"));
        assert_eq!(keys.len(), 1);
        assert_eq!(
            keys[0].wif,
            "KwdMAjGmerYanjeui5SHS7JkmpZvVipYvB2LJGU1ZxJwYvP98617"
        );
    }

    #[test]
    fn find_encrypted_entries_locates_pairs() {
        let mut blob = Vec::from(b"org.bitcoin.production".as_slice());
        let iv = [0x42u8; 16];
        let ct = [0x55u8; 48];
        for _ in 0..2 {
            blob.extend_from_slice(ENCRYPTED_IV_TAG);
            blob.extend_from_slice(&iv);
            blob.extend_from_slice(ENCRYPTED_DATA_TAG);
            blob.extend_from_slice(&ct);
        }
        let found = find_encrypted_entries(&blob);
        assert_eq!(found.len(), 2);
    }
}
