//! Bitcoin Core wallet.dat (BDB or SQLite) — DER-pattern private-key scan.

use std::path::Path;

use crate::{
    crypto::{addresses_for_privkey, privkey_to_wif},
    extractors::{Extractor, scan_result_error},
    models::{ExtractedKey, SourceType, WalletScanResult},
};

pub const DER_PATTERN: &[u8] = b"\x30\x81\xd3\x02\x01\x01\x04\x20";
const PRIVKEY_LEN: usize = 32;

pub struct BitcoinCoreExtractor;

impl Extractor for BitcoinCoreExtractor {
    fn source_type(&self) -> SourceType {
        SourceType::BitcoinCore
    }

    fn can_handle(&self, path: &Path) -> bool {
        if !path.is_file() {
            return false;
        }
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_lowercase();
        if !(name.ends_with(".dat") || name == "wallet") {
            return false;
        }
        let Ok(head) = read_head(path, 4096) else { return false };
        memmem(&head, DER_PATTERN) || memmem(&head, b"main\0")
    }

    fn extract(&self, path: &Path, _passwords: &[String]) -> WalletScanResult {
        let data = match std::fs::read(path) {
            Ok(d) => d,
            Err(e) => return scan_result_error(path, self.source_type(), e),
        };
        let mut keys = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let mut pos = 0usize;
        while pos + DER_PATTERN.len() + PRIVKEY_LEN <= data.len() {
            if &data[pos..pos + DER_PATTERN.len()] == DER_PATTERN {
                let start = pos + DER_PATTERN.len();
                let priv_bytes: [u8; 32] = match data[start..start + PRIVKEY_LEN].try_into() {
                    Ok(b) => b,
                    Err(_) => {
                        pos += 1;
                        continue;
                    }
                };
                if seen.insert(priv_bytes) {
                    if let Some(k) = make_key(&priv_bytes, path, SourceType::BitcoinCore) {
                        keys.push(k);
                    }
                }
            }
            pos += 1;
        }
        WalletScanResult {
            source_file: path.display().to_string(),
            source_type: self.source_type(),
            keys,
            error: None,
        }
    }
}

pub(crate) fn read_head(path: &Path, max: usize) -> std::io::Result<Vec<u8>> {
    use std::io::Read;
    let mut f = std::fs::File::open(path)?;
    let mut buf = vec![0u8; max];
    let n = f.read(&mut buf)?;
    buf.truncate(n);
    Ok(buf)
}

pub(crate) fn memmem(hay: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() || needle.len() > hay.len() {
        return false;
    }
    hay.windows(needle.len()).any(|w| w == needle)
}

pub(crate) fn make_key(
    privkey: &[u8; 32],
    source_file: &Path,
    source_type: SourceType,
) -> Option<ExtractedKey> {
    let wif = privkey_to_wif(privkey, true).ok()?;
    let a = addresses_for_privkey(privkey).ok()?;
    Some(ExtractedKey {
        wif,
        address_compressed: a.p2pkh_compressed,
        address_uncompressed: Some(a.p2pkh_uncompressed),
        address_p2sh_segwit: Some(a.p2sh_p2wpkh),
        address_bech32: Some(a.bech32),
        source_file: source_file.display().to_string(),
        source_type,
        derivation_path: None,
        balance_sat: None,
        total_received_sat: None,
        tx_count: None,
        notes: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn priv_hex(s: &str) -> [u8; 32] {
        let mut out = [0u8; 32];
        hex::decode_to_slice(s, &mut out).unwrap();
        out
    }

    #[test]
    fn extracts_planted_der_keys() {
        let tmp = tempfile_path("demo_bc.dat");
        let mut f = std::fs::File::create(&tmp).unwrap();
        f.write_all(&[0u8; 128]).unwrap();
        f.write_all(b"main\0").unwrap();
        let p1 = priv_hex("0c28fca386c7a227600b2fe50b7cae11ec86d3bf1fbe471be89827e19d72aa1d");
        let p2 = priv_hex("18e14a7b6a307f426a94f8114701e7c8e774e7f9a47e2c2035db29a206321725");
        for p in [p1, p2] {
            f.write_all(&[0u8; 16]).unwrap();
            f.write_all(DER_PATTERN).unwrap();
            f.write_all(&p).unwrap();
            f.write_all(&[0u8; 32]).unwrap();
        }
        drop(f);

        let ex = BitcoinCoreExtractor;
        assert!(ex.can_handle(&tmp));
        let result = ex.extract(&tmp, &[]);
        assert_eq!(result.keys.len(), 2);
        let wifs: std::collections::HashSet<_> =
            result.keys.iter().map(|k| k.wif.clone()).collect();
        assert!(wifs.contains("KwdMAjGmerYanjeui5SHS7JkmpZvVipYvB2LJGU1ZxJwYvP98617"));
        std::fs::remove_file(&tmp).ok();
    }

    pub(crate) fn tempfile_path(name: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "orpheus-test-{}-{}",
            std::process::id(),
            name
        ));
        p
    }
}
