//! Bitcoin Core `dumpwallet` text output + descriptor JSON dumps.

use std::path::Path;

use regex::Regex;
use serde_json::Value;

use crate::{
    crypto::{addresses_for_privkey, wif_to_privkey},
    extractors::{Extractor, scan_result_error},
    models::{ExtractedKey, SourceType, WalletScanResult},
};

pub struct WalletDumpExtractor;

fn wif_regex() -> Regex {
    // Compressed K/L/c or uncompressed 5/9; 51–52 chars base58 body.
    Regex::new(r"\b([5KL9c][1-9A-HJ-NP-Za-km-z]{50,51})\b").unwrap()
}

impl Extractor for WalletDumpExtractor {
    fn source_type(&self) -> SourceType {
        SourceType::WalletDump
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
        matches!(ext.as_str(), "txt" | "dump" | "json")
    }

    fn extract(&self, path: &Path, _passwords: &[String]) -> WalletScanResult {
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) => return scan_result_error(path, self.source_type(), e),
        };
        let trimmed = text.trim_start();
        let keys = if trimmed.starts_with('{') || trimmed.starts_with('[') {
            extract_json(&text, path).unwrap_or_else(|| extract_text(&text, path))
        } else {
            extract_text(&text, path)
        };
        WalletScanResult {
            source_file: path.display().to_string(),
            source_type: self.source_type(),
            keys,
            error: None,
        }
    }
}

fn extract_text(text: &str, source_file: &Path) -> Vec<ExtractedKey> {
    let re = wif_regex();
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for cap in re.captures_iter(text) {
        let wif = cap[1].to_string();
        if !seen.insert(wif.clone()) {
            continue;
        }
        if let Some(k) = key_from_wif(&wif, source_file, None) {
            out.push(k);
        }
    }
    out
}

fn extract_json(text: &str, source_file: &Path) -> Option<Vec<ExtractedKey>> {
    let value: Value = serde_json::from_str(text).ok()?;
    let mut candidates: Vec<(String, Option<String>)> = Vec::new();
    match &value {
        Value::Object(obj) if obj.contains_key("descriptors") => {
            if let Some(Value::Array(arr)) = obj.get("descriptors") {
                for d in arr {
                    if let Some(Value::String(desc)) = d.get("desc") {
                        let re = wif_regex();
                        for cap in re.captures_iter(desc) {
                            candidates.push((
                                cap[1].to_string(),
                                d.get("timestamp").map(|v| v.to_string()),
                            ));
                        }
                    }
                }
            }
        }
        Value::Array(arr) => {
            for entry in arr {
                if let (Some(Value::String(wif)), path) =
                    (entry.get("wif"), entry.get("path").and_then(Value::as_str))
                {
                    candidates.push((wif.clone(), path.map(str::to_string)));
                }
            }
        }
        Value::Object(obj) if obj.contains_key("wif") => {
            if let Some(Value::String(wif)) = obj.get("wif") {
                candidates.push((
                    wif.clone(),
                    obj.get("path").and_then(Value::as_str).map(str::to_string),
                ));
            }
        }
        _ => {}
    }
    if candidates.is_empty() {
        return None;
    }
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for (wif, path) in candidates {
        if !seen.insert(wif.clone()) {
            continue;
        }
        if let Some(k) = key_from_wif(&wif, source_file, path) {
            out.push(k);
        }
    }
    Some(out)
}

fn key_from_wif(
    wif: &str,
    source_file: &Path,
    derivation_path: Option<String>,
) -> Option<ExtractedKey> {
    let (priv_bytes, compressed) = wif_to_privkey(wif).ok()?;
    let a = addresses_for_privkey(&priv_bytes).ok()?;
    let primary = if compressed {
        a.p2pkh_compressed.clone()
    } else {
        a.p2pkh_uncompressed.clone()
    };
    Some(ExtractedKey {
        wif: wif.to_string(),
        address_compressed: primary,
        address_uncompressed: Some(a.p2pkh_uncompressed),
        address_p2sh_segwit: Some(a.p2sh_p2wpkh),
        address_bech32: Some(a.bech32),
        source_file: source_file.display().to_string(),
        source_type: SourceType::WalletDump,
        derivation_path,
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

    const KNOWN_WIF: &str = "KwdMAjGmerYanjeui5SHS7JkmpZvVipYvB2LJGU1ZxJwYvP98617";

    fn tempfile_path(name: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("orpheus-wd-{}-{}", std::process::id(), name));
        p
    }

    #[test]
    fn extracts_from_text_dump() {
        let path = tempfile_path("dump.txt");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "# Wallet dump").unwrap();
        writeln!(f, "{KNOWN_WIF} 2024-01-01 label=foo # addr=abc").unwrap();
        drop(f);
        let r = WalletDumpExtractor.extract(&path, &[]);
        assert_eq!(r.keys.len(), 1);
        assert_eq!(r.keys[0].wif, KNOWN_WIF);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn extracts_from_json_list_with_path() {
        let path = tempfile_path("dump.json");
        std::fs::write(
            &path,
            format!(r#"[{{"wif":"{KNOWN_WIF}","path":"m/44'/0'/0'/0/0"}}]"#),
        )
        .unwrap();
        let r = WalletDumpExtractor.extract(&path, &[]);
        assert_eq!(r.keys.len(), 1);
        assert_eq!(r.keys[0].derivation_path.as_deref(), Some("m/44'/0'/0'/0/0"));
        std::fs::remove_file(&path).ok();
    }
}
