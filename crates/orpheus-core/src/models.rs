//! Data types returned by extractors and consumed by every frontend.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
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
    pub tx_count: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceInfo {
    pub address: String,
    pub balance_sat: u64,
    pub total_received_sat: u64,
    pub tx_count: u64,
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
}
