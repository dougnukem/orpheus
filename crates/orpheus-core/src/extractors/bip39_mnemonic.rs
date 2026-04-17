//! BIP39 mnemonic extractor.
//!
//! Derives across BIP44, BIP49, BIP84 and the Breadwallet path `m/0'/{0,1}/x`.
//! Breadwallet support is first-class — it anchors the 2026-04-17 recovery.

use std::path::Path;

use bip39::{Language, Mnemonic};

use crate::{
    crypto::{addresses_for_privkey, derive_from_seed, privkey_to_wif},
    extractors::{Extractor, scan_result_error},
    models::{ExtractedKey, SourceType, WalletScanResult},
};

const VALID_WORD_COUNTS: &[usize] = &[12, 15, 18, 21, 24];

#[derive(Debug, Clone, Copy)]
pub struct DerivationSpec {
    pub name: &'static str,
    pub account_path: &'static str,
    pub include_change: bool,
    pub preferred: Preferred,
}

#[derive(Debug, Clone, Copy)]
pub enum Preferred {
    P2pkh,
    P2shP2wpkh,
    Bech32,
}

pub const DEFAULT_SPECS: &[DerivationSpec] = &[
    DerivationSpec {
        name: "BIP44 (P2PKH)",
        account_path: "m/44'/0'/0'",
        include_change: true,
        preferred: Preferred::P2pkh,
    },
    DerivationSpec {
        name: "BIP49 (P2SH-P2WPKH)",
        account_path: "m/49'/0'/0'",
        include_change: true,
        preferred: Preferred::P2shP2wpkh,
    },
    DerivationSpec {
        name: "BIP84 (P2WPKH)",
        account_path: "m/84'/0'/0'",
        include_change: true,
        preferred: Preferred::Bech32,
    },
    DerivationSpec {
        name: "Breadwallet",
        account_path: "m/0'",
        include_change: true,
        preferred: Preferred::P2pkh,
    },
];

pub fn derive_bip39(
    phrase: &str,
    passphrase: &str,
    gap_limit: u32,
    specs: &[DerivationSpec],
    source_file: &str,
) -> Result<Vec<ExtractedKey>, String> {
    let mnemonic = Mnemonic::parse_in(Language::English, phrase)
        .map_err(|e| format!("invalid BIP39 phrase: {e}"))?;
    let seed = mnemonic.to_seed(passphrase);
    let mut out = Vec::new();
    for spec in specs {
        let chains: &[u32] = if spec.include_change { &[0, 1] } else { &[0] };
        for &chain in chains {
            for i in 0..gap_limit {
                let path = format!("{}/{}/{}", spec.account_path, chain, i);
                let child = derive_from_seed(&seed, &path).map_err(|e| e.to_string())?;
                let priv_bytes = child.private_key.secret_bytes();
                let a = addresses_for_privkey(&priv_bytes).map_err(|e| e.to_string())?;
                let primary = match spec.preferred {
                    Preferred::P2pkh => a.p2pkh_compressed.clone(),
                    Preferred::P2shP2wpkh => a.p2sh_p2wpkh.clone(),
                    Preferred::Bech32 => a.bech32.clone(),
                };
                out.push(ExtractedKey {
                    wif: privkey_to_wif(&priv_bytes, true).map_err(|e| e.to_string())?,
                    address_compressed: primary,
                    address_uncompressed: Some(a.p2pkh_uncompressed),
                    address_p2sh_segwit: Some(a.p2sh_p2wpkh),
                    address_bech32: Some(a.bech32),
                    source_file: source_file.to_string(),
                    source_type: SourceType::Bip39,
                    derivation_path: Some(path),
                    balance_sat: None,
                    total_received_sat: None,
                    total_sent_sat: None,
                    tx_count: None,
                    notes: Some(spec.name.to_string()),
                });
            }
        }
    }
    Ok(out)
}

pub struct Bip39TextExtractor;

impl Extractor for Bip39TextExtractor {
    fn source_type(&self) -> SourceType {
        SourceType::Bip39
    }

    fn can_handle(&self, path: &Path) -> bool {
        if !path.is_file() {
            return false;
        }
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .map(str::to_lowercase);
        if !matches!(
            ext.as_deref(),
            Some("txt") | Some("mnemonic") | None | Some("")
        ) {
            return false;
        }
        let Ok(text) = std::fs::read_to_string(path) else {
            return false;
        };
        let trimmed = text.trim();
        let words: Vec<&str> = trimmed.split_whitespace().collect();
        if !VALID_WORD_COUNTS.contains(&words.len()) {
            return false;
        }
        Mnemonic::parse_in(Language::English, trimmed).is_ok()
    }

    fn extract(&self, path: &Path, passwords: &[String]) -> WalletScanResult {
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) => return scan_result_error(path, self.source_type(), e),
        };
        let source = path.display().to_string();
        let mut all_keys = Vec::new();
        let passes: Vec<String> = if passwords.is_empty() {
            vec![String::new()]
        } else {
            std::iter::once(String::new())
                .chain(passwords.iter().cloned())
                .collect()
        };
        for pass in passes {
            match derive_bip39(text.trim(), &pass, 20, DEFAULT_SPECS, &source) {
                Ok(mut ks) => all_keys.append(&mut ks),
                Err(e) => {
                    return scan_result_error(path, self.source_type(), e);
                }
            }
        }
        WalletScanResult {
            source_file: source,
            source_type: self.source_type(),
            keys: all_keys,
            error: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ABANDON: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    #[test]
    fn abandon_first_bip44_address_known_vector() {
        let keys = derive_bip39(ABANDON, "", 1, DEFAULT_SPECS, "x").unwrap();
        let bip44 = keys
            .iter()
            .find(|k| k.derivation_path.as_deref() == Some("m/44'/0'/0'/0/0"))
            .unwrap();
        assert_eq!(
            bip44.address_compressed,
            "1LqBGSKuX5yYUonjxT5qGfpUsXKYYWeabA"
        );
    }

    #[test]
    fn breadwallet_paths_generate_keys() {
        let keys = derive_bip39(ABANDON, "", 2, DEFAULT_SPECS, "x").unwrap();
        let bw: Vec<_> = keys
            .iter()
            .filter(|k| {
                k.derivation_path
                    .as_deref()
                    .unwrap_or("")
                    .starts_with("m/0'/")
            })
            .collect();
        assert_eq!(bw.len(), 4);
    }

    #[test]
    fn invalid_phrase_errors() {
        let result = derive_bip39("not real words here", "", 1, DEFAULT_SPECS, "x");
        assert!(result.is_err());
    }
}
