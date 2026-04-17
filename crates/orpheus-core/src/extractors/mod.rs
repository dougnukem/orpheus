//! Wallet extractors — each converts a file into a `WalletScanResult`.

use std::path::Path;

use crate::models::{SourceType, WalletScanResult};

pub mod bip39_mnemonic;
pub mod bitcoin_core;
pub mod blockchain_com;
pub mod encrypted;
pub mod multibit;
pub mod wallet_dump;

/// Trait implemented by every extractor.
pub trait Extractor: Send + Sync {
    fn source_type(&self) -> SourceType;
    fn can_handle(&self, path: &Path) -> bool;
    fn extract(&self, path: &Path, passwords: &[String]) -> WalletScanResult;
}

pub fn registry() -> Vec<Box<dyn Extractor>> {
    vec![
        Box::new(bitcoin_core::BitcoinCoreExtractor),
        Box::new(multibit::MultibitExtractor),
        Box::new(wallet_dump::WalletDumpExtractor),
        Box::new(bip39_mnemonic::Bip39TextExtractor),
        Box::new(encrypted::EncryptedWalletExtractor),
    ]
}

pub fn scan_result_error(
    path: &Path,
    source_type: SourceType,
    err: impl std::fmt::Display,
) -> WalletScanResult {
    WalletScanResult {
        source_file: path.display().to_string(),
        source_type,
        keys: vec![],
        error: Some(err.to_string()),
    }
}
