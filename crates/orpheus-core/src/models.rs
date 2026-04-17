//! Data types returned by extractors and consumed by every frontend.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    BitcoinCore,
    Multibit,
    Bip39,
    BlockchainCom,
    WalletDump,
    Encrypted,
    Unknown,
}

impl SourceType {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BitcoinCore => "bitcoin_core",
            Self::Multibit => "multibit",
            Self::Bip39 => "bip39",
            Self::BlockchainCom => "blockchain_com",
            Self::WalletDump => "wallet_dump",
            Self::Encrypted => "encrypted",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedKey {
    pub wif: String,
    pub address_compressed: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address_uncompressed: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address_p2sh_segwit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address_bech32: Option<String>,
    pub source_file: String,
    pub source_type: SourceType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub derivation_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub balance_sat: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_received_sat: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_sent_sat: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tx_count: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceInfo {
    pub address: String,
    pub balance_sat: u64,
    pub total_received_sat: u64,
    pub total_sent_sat: u64,
    pub tx_count: u64,
}

impl BalanceInfo {
    #[must_use]
    pub const fn zero(address: String) -> Self {
        Self {
            address,
            balance_sat: 0,
            total_received_sat: 0,
            total_sent_sat: 0,
            tx_count: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletScanResult {
    pub source_file: String,
    pub source_type: SourceType,
    pub keys: Vec<ExtractedKey>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl WalletScanResult {
    #[must_use]
    pub fn key_count(&self) -> usize {
        self.keys.len()
    }

    #[must_use]
    pub fn total_balance_sat(&self) -> u64 {
        self.keys.iter().filter_map(|k| k.balance_sat).sum()
    }

    #[must_use]
    pub fn total_received_sat(&self) -> u64 {
        self.keys.iter().filter_map(|k| k.total_received_sat).sum()
    }

    #[must_use]
    pub fn total_sent_sat(&self) -> u64 {
        self.keys.iter().filter_map(|k| k.total_sent_sat).sum()
    }
}

/// Aggregate rollup across a set of [`WalletScanResult`]s — shown as the
/// summary at the bottom of `orpheus scan` output.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScanSummary {
    pub wallets: usize,
    pub total_keys: usize,
    pub unique_addresses: usize,
    /// addresses with a current balance > 0
    pub funded_addresses: usize,
    /// addresses that have seen activity but are now empty
    pub spent_addresses: usize,
    /// addresses that never received anything
    pub unfunded_addresses: usize,
    pub total_received_sat: u64,
    pub total_sent_sat: u64,
    pub total_balance_sat: u64,
    pub by_source_type: Vec<SourceTypeStats>,
    pub provider: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceTypeStats {
    pub source_type: SourceType,
    pub wallets: usize,
    pub keys: usize,
    pub balance_sat: u64,
}

impl ScanSummary {
    #[must_use]
    pub fn from_results(results: &[WalletScanResult], provider: Option<&str>) -> Self {
        use std::collections::{BTreeMap, HashSet};

        let mut by_type: BTreeMap<SourceType, SourceTypeStats> = BTreeMap::new();
        let mut unique: HashSet<String> = HashSet::new();
        let mut funded = 0usize;
        let mut spent = 0usize;
        let mut unfunded = 0usize;

        for r in results {
            let e = by_type
                .entry(r.source_type)
                .or_insert_with(|| SourceTypeStats {
                    source_type: r.source_type,
                    wallets: 0,
                    keys: 0,
                    balance_sat: 0,
                });
            e.wallets += 1;
            e.keys += r.keys.len();
            e.balance_sat += r.total_balance_sat();
            for k in &r.keys {
                if unique.insert(k.address_compressed.clone()) {
                    let received = k.total_received_sat.unwrap_or(0);
                    let balance = k.balance_sat.unwrap_or(0);
                    if balance > 0 {
                        funded += 1;
                    } else if received > 0 {
                        spent += 1;
                    } else {
                        unfunded += 1;
                    }
                }
            }
        }

        Self {
            wallets: results.len(),
            total_keys: results.iter().map(WalletScanResult::key_count).sum(),
            unique_addresses: unique.len(),
            funded_addresses: funded,
            spent_addresses: spent,
            unfunded_addresses: unfunded,
            total_received_sat: results
                .iter()
                .map(WalletScanResult::total_received_sat)
                .sum(),
            total_sent_sat: results.iter().map(WalletScanResult::total_sent_sat).sum(),
            total_balance_sat: results
                .iter()
                .map(WalletScanResult::total_balance_sat)
                .sum(),
            by_source_type: by_type.into_values().collect(),
            provider: provider.map(str::to_string),
        }
    }
}
