//! Balance lookup.
//!
//! Four providers, all implementing [`BalanceProvider`]:
//!
//!   * [`BlockstreamProvider`] — public esplora at blockstream.info/api.
//!     This is the default in the CLI / server because it has the kindest
//!     rate limit of the free public options and requires no API key.
//!   * [`BlockchainInfoProvider`] — blockchain.info /balance endpoint. Good
//!     fallback; supports batching of up to 20 addresses per request.
//!   * [`MockProvider`] — reads a JSON file keyed on address. Used by
//!     `orpheus demo` and the test suite so nothing hits the network.
//!   * [`NoopProvider`] — returns all-zero balances. Used when
//!     `--provider none` is requested explicitly.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::models::{BalanceInfo, ExtractedKey};

/// Addresses per batch request when the provider supports batching.
pub const MAX_BATCH: usize = 20;

/// String identifiers accepted by [`ProviderKind::parse`] and the CLI/server
/// `--provider` flag. Keep this list in sync with `clap::ValueEnum` in the
/// CLI and the frontend `<select>` values.
pub const VALID_PROVIDERS: &[&str] = &["blockstream", "blockchain", "mock", "none"];

/// Which balance provider the user requested.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKind {
    Blockstream,
    BlockchainInfo,
    Mock,
    None,
}

impl ProviderKind {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.to_ascii_lowercase().as_str() {
            "blockstream" | "blockstream.info" => Ok(Self::Blockstream),
            "blockchain" | "blockchain.info" => Ok(Self::BlockchainInfo),
            "mock" => Ok(Self::Mock),
            "none" | "off" => Ok(Self::None),
            other => Err(format!(
                "unknown provider {other:?}; expected one of {}",
                VALID_PROVIDERS.join(", ")
            )),
        }
    }
}

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

/// Returns all-zero balances. For when the user explicitly opts out.
pub struct NoopProvider;

impl BalanceProvider for NoopProvider {
    fn name(&self) -> &'static str {
        "none"
    }

    fn fetch(&self, addresses: &[String]) -> HashMap<String, BalanceInfo> {
        addresses
            .iter()
            .map(|a| {
                (
                    a.clone(),
                    BalanceInfo {
                        address: a.clone(),
                        balance_sat: 0,
                        total_received_sat: 0,
                        tx_count: 0,
                    },
                )
            })
            .collect()
    }
}

#[cfg(feature = "network")]
pub struct BlockstreamProvider {
    client: reqwest::blocking::Client,
    base: String,
}

#[cfg(feature = "network")]
impl BlockstreamProvider {
    pub fn new() -> Self {
        Self {
            client: reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .user_agent("orpheus/0.1")
                .build()
                .expect("reqwest client"),
            base: "https://blockstream.info/api".into(),
        }
    }
}

#[cfg(feature = "network")]
impl Default for BlockstreamProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "network")]
impl BalanceProvider for BlockstreamProvider {
    fn name(&self) -> &'static str {
        "blockstream.info"
    }

    fn fetch(&self, addresses: &[String]) -> HashMap<String, BalanceInfo> {
        // Esplora exposes a per-address endpoint; no batch. We issue them
        // sequentially to play nice with the public rate limit.
        let mut out = HashMap::new();
        for addr in addresses {
            let url = format!("{}/address/{addr}", self.base);
            let info = match self
                .client
                .get(&url)
                .send()
                .and_then(|r| r.error_for_status())
            {
                Ok(resp) => match resp.json::<serde_json::Value>() {
                    Ok(j) => {
                        let chain = j.get("chain_stats").cloned().unwrap_or_default();
                        let mem = j.get("mempool_stats").cloned().unwrap_or_default();
                        let get = |v: &serde_json::Value, k: &str| -> u64 {
                            v.get(k).and_then(|x| x.as_u64()).unwrap_or(0)
                        };
                        let funded = get(&chain, "funded_txo_sum") + get(&mem, "funded_txo_sum");
                        let spent = get(&chain, "spent_txo_sum") + get(&mem, "spent_txo_sum");
                        BalanceInfo {
                            address: addr.clone(),
                            balance_sat: funded.saturating_sub(spent),
                            total_received_sat: funded,
                            tx_count: get(&chain, "tx_count") + get(&mem, "tx_count"),
                        }
                    }
                    Err(_) => zero(addr),
                },
                Err(_) => zero(addr),
            };
            out.insert(addr.clone(), info);
        }
        out
    }
}

#[cfg(feature = "network")]
pub struct BlockchainInfoProvider {
    client: reqwest::blocking::Client,
    base: String,
}

#[cfg(feature = "network")]
impl BlockchainInfoProvider {
    pub fn new() -> Self {
        Self {
            client: reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .user_agent("orpheus/0.1")
                .build()
                .expect("reqwest client"),
            base: "https://blockchain.info".into(),
        }
    }
}

#[cfg(feature = "network")]
impl Default for BlockchainInfoProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "network")]
impl BalanceProvider for BlockchainInfoProvider {
    fn name(&self) -> &'static str {
        "blockchain.info"
    }

    fn fetch(&self, addresses: &[String]) -> HashMap<String, BalanceInfo> {
        let mut out: HashMap<String, BalanceInfo> = HashMap::new();
        for chunk in addresses.chunks(MAX_BATCH) {
            let active = chunk.join("|");
            let url = format!("{}/balance?active={active}", self.base);
            let Ok(resp) = self
                .client
                .get(url)
                .send()
                .and_then(|r| r.error_for_status())
            else {
                continue;
            };
            let Ok(map) = resp.json::<HashMap<String, serde_json::Value>>() else {
                continue;
            };
            for (addr, info) in map {
                out.insert(
                    addr.clone(),
                    BalanceInfo {
                        address: addr,
                        balance_sat: info
                            .get("final_balance")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0),
                        total_received_sat: info
                            .get("total_received")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0),
                        tx_count: info.get("n_tx").and_then(|v| v.as_u64()).unwrap_or(0),
                    },
                );
            }
        }
        for addr in addresses {
            out.entry(addr.clone()).or_insert_with(|| zero(addr));
        }
        out
    }
}

fn zero(addr: &str) -> BalanceInfo {
    BalanceInfo {
        address: addr.to_string(),
        balance_sat: 0,
        total_received_sat: 0,
        tx_count: 0,
    }
}

/// Build a provider from a [`ProviderKind`]. `mock_file` is only consulted for
/// [`ProviderKind::Mock`]. Returns `None` for [`ProviderKind::None`], meaning
/// the caller should skip the balance lookup step entirely.
pub fn provider_from_kind(
    kind: ProviderKind,
    mock_file: Option<PathBuf>,
) -> Option<Box<dyn BalanceProvider>> {
    match kind {
        ProviderKind::None => None,
        ProviderKind::Mock => Some(Box::new(MockProvider { path: mock_file })),
        #[cfg(feature = "network")]
        ProviderKind::Blockstream => Some(Box::new(BlockstreamProvider::new())),
        #[cfg(feature = "network")]
        ProviderKind::BlockchainInfo => Some(Box::new(BlockchainInfoProvider::new())),
        #[cfg(not(feature = "network"))]
        ProviderKind::Blockstream | ProviderKind::BlockchainInfo => {
            tracing::warn!(
                "network providers requested but the `network` feature is disabled; \
                 falling back to NoopProvider"
            );
            Some(Box::new(NoopProvider))
        }
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
        let provider = MockProvider {
            path: Some(tmp.clone()),
        };
        let r = provider.fetch(&["1abc".into(), "1missing".into()]);
        assert_eq!(r["1abc"].balance_sat, 42);
        assert_eq!(r["1missing"].balance_sat, 0);
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn attach_mutates_keys() {
        let tmp = std::env::temp_dir().join("orpheus-mock2.json");
        std::fs::write(&tmp, r#"{"1x":{"balance_sat":7}}"#).unwrap();
        let provider = MockProvider {
            path: Some(tmp.clone()),
        };
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
