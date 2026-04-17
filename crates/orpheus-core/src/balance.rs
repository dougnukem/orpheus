//! Balance lookup — network-optional, mock-driven by default for tests.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::models::{BalanceInfo, ExtractedKey};

pub const MAX_BATCH: usize = 20;

#[derive(Debug, Clone, Deserialize)]
struct MockEntry {
    #[serde(default)]
    balance_sat: u64,
    #[serde(default)]
    total_received_sat: u64,
    #[serde(default)]
    tx_count: u64,
}

pub trait BalanceProvider: Send + Sync {
    fn fetch(&self, addresses: &[String]) -> HashMap<String, BalanceInfo>;
    fn name(&self) -> &'static str;
}

pub struct MockProvider {
    pub path: Option<PathBuf>,
}

impl BalanceProvider for MockProvider {
    fn name(&self) -> &'static str {
        "mock"
    }

    fn fetch(&self, addresses: &[String]) -> HashMap<String, BalanceInfo> {
        let data: HashMap<String, MockEntry> = self
            .path
            .as_ref()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        addresses
            .iter()
            .map(|addr| {
                let info = data
                    .get(addr)
                    .map(|e| BalanceInfo {
                        address: addr.clone(),
                        balance_sat: e.balance_sat,
                        total_received_sat: e.total_received_sat,
                        tx_count: e.tx_count,
                    })
                    .unwrap_or_else(|| BalanceInfo {
                        address: addr.clone(),
                        balance_sat: 0,
                        total_received_sat: 0,
                        tx_count: 0,
                    });
                (addr.clone(), info)
            })
            .collect()
    }
}

/// Apply balance lookups to an already-extracted list of keys in place.
pub fn attach_balances(keys: &mut [ExtractedKey], provider: &dyn BalanceProvider) {
    if keys.is_empty() {
        return;
    }
    let mut dedup: Vec<String> = keys.iter().map(|k| k.address_compressed.clone()).collect();
    dedup.sort();
    dedup.dedup();
    let balances = provider.fetch(&dedup);
    for k in keys.iter_mut() {
        if let Some(info) = balances.get(&k.address_compressed) {
            k.balance_sat = Some(info.balance_sat);
            k.total_received_sat = Some(info.total_received_sat);
            k.tx_count = Some(info.tx_count);
        }
    }
}

pub fn mock_provider_with_default_path(default: &Path) -> MockProvider {
    MockProvider {
        path: Some(default.to_path_buf()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_returns_entries_and_zero_fills() {
        let tmp = std::env::temp_dir().join("orpheus-mock.json");
        std::fs::write(
            &tmp,
            r#"{"1abc":{"balance_sat":42,"total_received_sat":100,"tx_count":2}}"#,
        )
        .unwrap();
        let provider = MockProvider { path: Some(tmp.clone()) };
        let r = provider.fetch(&["1abc".into(), "1missing".into()]);
        assert_eq!(r["1abc"].balance_sat, 42);
        assert_eq!(r["1missing"].balance_sat, 0);
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn attach_mutates_keys() {
        let tmp = std::env::temp_dir().join("orpheus-mock2.json");
        std::fs::write(&tmp, r#"{"1x":{"balance_sat":7}}"#).unwrap();
        let provider = MockProvider { path: Some(tmp.clone()) };
        let mut keys = vec![ExtractedKey {
            wif: "w".into(),
            address_compressed: "1x".into(),
            address_uncompressed: None,
            address_p2sh_segwit: None,
            address_bech32: None,
            source_file: "f".into(),
            source_type: crate::models::SourceType::Bip39,
            derivation_path: None,
            balance_sat: None,
            total_received_sat: None,
            tx_count: None,
            notes: None,
        }];
        attach_balances(&mut keys, &provider);
        assert_eq!(keys[0].balance_sat, Some(7));
        std::fs::remove_file(&tmp).ok();
    }
}
