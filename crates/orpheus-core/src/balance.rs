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

use crate::models::{BalanceInfo, ExtractedKey, TxRecord};

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
    total_sent_sat: Option<u64>,
    #[serde(default)]
    tx_count: u64,
}

pub trait BalanceProvider: Send + Sync {
    fn fetch(&self, addresses: &[String]) -> HashMap<String, BalanceInfo>;
    fn name(&self) -> &'static str;

    /// Full transaction history for one address.
    ///
    /// Defaults to empty so existing providers — and the [`MockProvider`] test
    /// seam — keep working unchanged. Only providers that can serve real
    /// history override it.
    fn transactions(&self, _address: &str) -> Vec<TxRecord> {
        Vec::new()
    }

    /// Whether [`BalanceProvider::transactions`] returns real data.
    fn supports_transactions(&self) -> bool {
        false
    }
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
                        total_sent_sat: e
                            .total_sent_sat
                            .unwrap_or_else(|| e.total_received_sat.saturating_sub(e.balance_sat)),
                        tx_count: e.tx_count,
                    })
                    .unwrap_or_else(|| BalanceInfo::zero(addr.clone()));
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
            .map(|a| (a.clone(), BalanceInfo::zero(a.clone())))
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

    /// Esplora exposes a per-address endpoint with no batching, so requests go
    /// out sequentially to stay friendly to the public rate limit.
    ///
    /// An address whose lookup fails after retries is **omitted** from the
    /// result rather than recorded as zero. Zero means "we asked and this
    /// address is empty"; a rate-limited request means "we do not know". For a
    /// tool whose whole job is finding forgotten money, conflating those is how
    /// a funded address gets written off as empty.
    fn fetch(&self, addresses: &[String]) -> HashMap<String, BalanceInfo> {
        const ATTEMPTS: u32 = 3;
        let mut out = HashMap::new();
        for addr in addresses {
            let url = format!("{}/address/{addr}", self.base);
            for attempt in 0..ATTEMPTS {
                match self
                    .client
                    .get(&url)
                    .send()
                    .and_then(reqwest::blocking::Response::error_for_status)
                    .and_then(reqwest::blocking::Response::json::<serde_json::Value>)
                {
                    Ok(json) => {
                        out.insert(addr.clone(), blockstream_info_from_json(addr, &json));
                        break;
                    }
                    Err(e) => {
                        if attempt + 1 == ATTEMPTS {
                            tracing::warn!(
                                address = %addr,
                                error = %e,
                                "balance lookup failed after {ATTEMPTS} attempts; \
                                 reporting as unknown rather than zero"
                            );
                        } else {
                            // Linear backoff is enough for a public esplora;
                            // the failure mode here is a 429, not congestion.
                            std::thread::sleep(std::time::Duration::from_millis(
                                500 * u64::from(attempt + 1),
                            ));
                        }
                    }
                }
            }
        }
        out
    }

    fn supports_transactions(&self) -> bool {
        true
    }

    /// Walk `/address/{addr}/txs`, then page backwards through
    /// `/address/{addr}/txs/chain/{last_seen}`. Esplora returns 25 confirmed
    /// transactions per page, so an address with a long history needs several
    /// round trips.
    fn transactions(&self, address: &str) -> Vec<TxRecord> {
        const MAX_PAGES: usize = 40; // 40 * 25 = 1000 transactions, plenty here.
        let mut out: Vec<TxRecord> = Vec::new();
        let mut url = format!("{}/address/{address}/txs", self.base);

        for _ in 0..MAX_PAGES {
            let Ok(resp) = self
                .client
                .get(&url)
                .send()
                .and_then(reqwest::blocking::Response::error_for_status)
            else {
                break;
            };
            let Ok(json) = resp.json::<serde_json::Value>() else {
                break;
            };
            let page = blockstream_txs_from_json(address, &json);
            if page.is_empty() {
                break;
            }
            let last_txid = page[page.len() - 1].txid.clone();
            out.extend(page);
            url = format!("{}/address/{address}/txs/chain/{last_txid}", self.base);
        }
        out
    }
}

/// Parse a Blockstream `/address/{addr}/txs` array into [`TxRecord`]s.
///
/// Net value is computed as (outputs paying this address) minus (inputs spent
/// from this address), so a sweep shows up as a single negative entry rather
/// than as a confusing pair.
///
/// Kept pure so it can be pinned in tests without a network round trip.
#[cfg(feature = "network")]
#[must_use]
pub fn blockstream_txs_from_json(address: &str, json: &serde_json::Value) -> Vec<TxRecord> {
    let Some(txs) = json.as_array() else {
        return vec![];
    };
    txs.iter()
        .filter_map(|tx| {
            let txid = tx.get("txid")?.as_str()?.to_string();
            let status = tx.get("status");
            let confirmed = status
                .and_then(|s| s.get("confirmed"))
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);

            let sum_for = |key: &str, unwrap_prevout: bool| -> u64 {
                tx.get(key)
                    .and_then(serde_json::Value::as_array)
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(|item| {
                                let target = if unwrap_prevout {
                                    item.get("prevout")?
                                } else {
                                    item
                                };
                                let addr = target.get("scriptpubkey_address")?.as_str()?;
                                if addr != address {
                                    return None;
                                }
                                target.get("value")?.as_u64()
                            })
                            .sum()
                    })
                    .unwrap_or(0)
            };

            let received = sum_for("vout", false);
            let spent = sum_for("vin", true);

            Some(TxRecord {
                txid,
                address: address.to_string(),
                confirmed,
                block_height: status
                    .and_then(|s| s.get("block_height"))
                    .and_then(serde_json::Value::as_u64),
                block_time: status
                    .and_then(|s| s.get("block_time"))
                    .and_then(serde_json::Value::as_u64),
                net_value_sat: received as i64 - spent as i64,
                fee_sat: tx.get("fee").and_then(serde_json::Value::as_u64),
            })
        })
        .collect()
}

/// Parse a Blockstream `/address/{addr}` response into a [`BalanceInfo`].
/// Kept pure so it can be unit-tested without a network round trip.
#[cfg(feature = "network")]
pub fn blockstream_info_from_json(address: &str, j: &serde_json::Value) -> BalanceInfo {
    let chain = j.get("chain_stats").cloned().unwrap_or_default();
    let mem = j.get("mempool_stats").cloned().unwrap_or_default();
    let get = |v: &serde_json::Value, k: &str| -> u64 {
        v.get(k).and_then(serde_json::Value::as_u64).unwrap_or(0)
    };
    let funded = get(&chain, "funded_txo_sum") + get(&mem, "funded_txo_sum");
    let spent = get(&chain, "spent_txo_sum") + get(&mem, "spent_txo_sum");
    BalanceInfo {
        address: address.to_string(),
        balance_sat: funded.saturating_sub(spent),
        total_received_sat: funded,
        total_sent_sat: spent,
        tx_count: get(&chain, "tx_count") + get(&mem, "tx_count"),
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
                let balance = info
                    .get("final_balance")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0);
                let received = info
                    .get("total_received")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0);
                out.insert(
                    addr.clone(),
                    BalanceInfo {
                        address: addr,
                        balance_sat: balance,
                        total_received_sat: received,
                        total_sent_sat: received.saturating_sub(balance),
                        tx_count: info
                            .get("n_tx")
                            .and_then(serde_json::Value::as_u64)
                            .unwrap_or(0),
                    },
                );
            }
        }
        for addr in addresses {
            out.entry(addr.clone())
                .or_insert_with(|| BalanceInfo::zero(addr.clone()));
        }
        out
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
            k.total_sent_sat = Some(info.total_sent_sat);
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
            total_sent_sat: None,
            tx_count: None,
            notes: None,
        }];
        attach_balances(&mut keys, &provider);
        assert_eq!(keys[0].balance_sat, Some(7));
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn mock_derives_sent_when_not_provided() {
        let tmp = std::env::temp_dir().join("orpheus-mock-sent.json");
        std::fs::write(
            &tmp,
            r#"{"1abc":{"balance_sat":100,"total_received_sat":500,"tx_count":3}}"#,
        )
        .unwrap();
        let provider = MockProvider {
            path: Some(tmp.clone()),
        };
        let r = provider.fetch(&["1abc".into()]);
        // sent = received - balance = 500 - 100 = 400 when not explicit
        assert_eq!(r["1abc"].total_sent_sat, 400);
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn mock_respects_explicit_sent() {
        let tmp = std::env::temp_dir().join("orpheus-mock-sent-explicit.json");
        std::fs::write(
            &tmp,
            r#"{"1abc":{"balance_sat":100,"total_received_sat":500,"total_sent_sat":999,"tx_count":3}}"#,
        )
        .unwrap();
        let provider = MockProvider {
            path: Some(tmp.clone()),
        };
        let r = provider.fetch(&["1abc".into()]);
        assert_eq!(r["1abc"].total_sent_sat, 999);
        std::fs::remove_file(&tmp).ok();
    }

    #[cfg(feature = "network")]
    #[test]
    fn blockstream_json_parse_pins_fields() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{
                "chain_stats": {
                    "funded_txo_sum": 5000000,
                    "spent_txo_sum": 1134948,
                    "tx_count": 4
                },
                "mempool_stats": {
                    "funded_txo_sum": 0,
                    "spent_txo_sum": 0,
                    "tx_count": 0
                }
            }"#,
        )
        .unwrap();
        let info = blockstream_info_from_json("1EBuf21icKTE5m3HWVndKx2bTxvqrWCqV6", &json);
        assert_eq!(info.balance_sat, 3_865_052);
        assert_eq!(info.total_received_sat, 5_000_000);
        assert_eq!(info.total_sent_sat, 1_134_948);
        assert_eq!(info.tx_count, 4);
    }

    /// A recorded two-transaction history: a 0.05 BTC receive, then a sweep
    /// that spends it. Pinned so the signed net-value arithmetic cannot drift.
    #[cfg(feature = "network")]
    #[test]
    fn blockstream_tx_parse_pins_signed_net_values() {
        const ADDR: &str = "1EBuf21icKTE5m3HWVndKx2bTxvqrWCqV6";
        let json: serde_json::Value = serde_json::from_str(
            r#"[
              {
                "txid": "aa11",
                "status": {"confirmed": true, "block_height": 300000, "block_time": 1399000000},
                "fee": 10000,
                "vin": [
                  {"prevout": {"scriptpubkey_address": "1EBuf21icKTE5m3HWVndKx2bTxvqrWCqV6", "value": 5000000}}
                ],
                "vout": [
                  {"scriptpubkey_address": "1SomeoneElse", "value": 4990000}
                ]
              },
              {
                "txid": "bb22",
                "status": {"confirmed": true, "block_height": 290000, "block_time": 1390000000},
                "fee": 5000,
                "vin": [
                  {"prevout": {"scriptpubkey_address": "1Funder", "value": 5100000}}
                ],
                "vout": [
                  {"scriptpubkey_address": "1EBuf21icKTE5m3HWVndKx2bTxvqrWCqV6", "value": 5000000},
                  {"scriptpubkey_address": "1Change", "value": 95000}
                ]
              }
            ]"#,
        )
        .unwrap();

        let txs = blockstream_txs_from_json(ADDR, &json);
        assert_eq!(txs.len(), 2);

        // The spend: this address funded 5_000_000 and received nothing back.
        assert_eq!(txs[0].txid, "aa11");
        assert_eq!(txs[0].net_value_sat, -5_000_000);
        assert_eq!(txs[0].block_height, Some(300_000));
        assert_eq!(txs[0].block_time, Some(1_399_000_000));
        assert_eq!(txs[0].fee_sat, Some(10_000));
        assert!(txs[0].confirmed);

        // The receive: only the output paying this address counts.
        assert_eq!(txs[1].txid, "bb22");
        assert_eq!(
            txs[1].net_value_sat, 5_000_000,
            "change paying a different address must not be credited here"
        );
    }

    #[cfg(feature = "network")]
    #[test]
    fn blockstream_tx_parse_handles_unconfirmed_and_empty() {
        let json: serde_json::Value = serde_json::from_str(
            r#"[{"txid":"cc33","status":{"confirmed":false},"vin":[],"vout":[{"scriptpubkey_address":"1Me","value":700}]}]"#,
        )
        .unwrap();
        let txs = blockstream_txs_from_json("1Me", &json);
        assert_eq!(txs.len(), 1);
        assert!(!txs[0].confirmed);
        assert_eq!(txs[0].block_height, None);
        assert_eq!(txs[0].net_value_sat, 700);

        let empty: serde_json::Value = serde_json::from_str("[]").unwrap();
        assert!(blockstream_txs_from_json("1Me", &empty).is_empty());
    }

    /// Providers that cannot serve history must say so rather than silently
    /// returning an empty list that reads like "this address has no history".
    #[test]
    fn non_network_providers_declare_no_tx_support() {
        assert!(!NoopProvider.supports_transactions());
        assert!(NoopProvider.transactions("1abc").is_empty());
        assert!(!MockProvider { path: None }.supports_transactions());
    }
}
