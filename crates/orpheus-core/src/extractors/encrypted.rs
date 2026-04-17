//! Encrypted wallet coordinator — delegates to per-format decryptors.

use std::path::Path;

use crate::{
    crypto::{addresses_for_privkey, privkey_to_wif},
    extractors::{
        Extractor,
        multibit::{find_encrypted_entries, find_scrypt_salt, try_decrypt_multibit},
        scan_result_error,
    },
    models::{ExtractedKey, SourceType, WalletScanResult},
};

pub struct EncryptedWalletExtractor;

impl Extractor for EncryptedWalletExtractor {
    fn source_type(&self) -> SourceType {
        SourceType::Encrypted
    }

    fn can_handle(&self, path: &Path) -> bool {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_lowercase();
        name.ends_with(".aes.json") || name.ends_with(".wallet")
    }

    fn extract(&self, path: &Path, passwords: &[String]) -> WalletScanResult {
        if passwords.is_empty() {
            return WalletScanResult {
                source_file: path.display().to_string(),
                source_type: self.source_type(),
                keys: vec![],
                error: None,
            };
        }
        let data = match std::fs::read(path) {
            Ok(d) => d,
            Err(e) => return scan_result_error(path, self.source_type(), e),
        };
        let entries = find_encrypted_entries(&data);
        let salt = find_scrypt_salt(&data);
        let (Some(salt), false) = (salt, entries.is_empty()) else {
            return WalletScanResult {
                source_file: path.display().to_string(),
                source_type: self.source_type(),
                keys: vec![],
                error: None,
            };
        };
        let mut found = Vec::new();
        for pw in passwords {
            for (iv, ct) in &entries {
                if let Some(priv_bytes) = try_decrypt_multibit(pw, &salt, iv, ct)
                    && let Ok(a) = addresses_for_privkey(&priv_bytes)
                    && let Ok(wif) = privkey_to_wif(&priv_bytes, true)
                {
                    found.push(ExtractedKey {
                        wif,
                        address_compressed: a.p2pkh_compressed,
                        address_uncompressed: Some(a.p2pkh_uncompressed),
                        address_p2sh_segwit: Some(a.p2sh_p2wpkh),
                        address_bech32: Some(a.bech32),
                        source_file: path.display().to_string(),
                        source_type: SourceType::Encrypted,
                        derivation_path: None,
                        balance_sat: None,
                        total_received_sat: None,
                        total_sent_sat: None,
                        tx_count: None,
                        notes: Some(format!(
                            "multibit-v3 decrypted (password length {})",
                            pw.len()
                        )),
                    });
                }
            }
            if !found.is_empty() {
                break;
            }
        }
        WalletScanResult {
            source_file: path.display().to_string(),
            source_type: self.source_type(),
            keys: found,
            error: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aes::cipher::{BlockEncryptMut, KeyIvInit, block_padding::Pkcs7};
    use std::io::Write;

    fn tempfile_path(name: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("orpheus-enc-{}-{name}", std::process::id()));
        p
    }

    #[test]
    fn decrypts_multibit_v3_end_to_end() {
        let priv_bytes = {
            let mut b = [0u8; 32];
            hex::decode_to_slice(
                "0c28fca386c7a227600b2fe50b7cae11ec86d3bf1fbe471be89827e19d72aa1d",
                &mut b,
            )
            .unwrap();
            b
        };
        let password = "orpheus-demo";
        let salt = [0x11u8; 8];
        let aes_key = crate::crypto::scrypt_aes_key(password.as_bytes(), &salt).unwrap();
        let iv = [0x22u8; 16];
        type Cbc = cbc::Encryptor<aes::Aes256>;
        let ct: Vec<u8> =
            Cbc::new((&aes_key).into(), (&iv).into()).encrypt_padded_vec_mut::<Pkcs7>(&priv_bytes);

        let path = tempfile_path("protected.wallet");
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(b"org.bitcoin.production").unwrap();
        f.write_all(&[0u8; 4]).unwrap();
        // salt block
        f.write_all(b"\x0a\x08").unwrap();
        f.write_all(&salt).unwrap();
        f.write_all(b"\x10\x80\x80\x01").unwrap(); // scrypt param marker
        // encrypted entry
        f.write_all(b"\x0a\x10").unwrap();
        f.write_all(&iv).unwrap();
        f.write_all(b"\x12\x30").unwrap();
        f.write_all(&ct[..48]).unwrap();
        drop(f);

        let ex = EncryptedWalletExtractor;
        assert!(ex.can_handle(&path));
        let r = ex.extract(&path, &["wrong".into(), password.into()]);
        assert_eq!(r.keys.len(), 1);
        assert_eq!(
            r.keys[0].wif,
            "KwdMAjGmerYanjeui5SHS7JkmpZvVipYvB2LJGU1ZxJwYvP98617"
        );
        std::fs::remove_file(&path).ok();
    }
}
