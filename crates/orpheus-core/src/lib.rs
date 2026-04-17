//! Orpheus core — wallet extraction, crypto primitives, balance lookup.
//!
//! This crate is deliberately kept framework-agnostic so it can be driven by
//! `orpheus-cli` (which also hosts `orpheus serve` for the axum + embedded
//! web UI) and `orpheus-tauri` (desktop) without any of them dragging in the
//! others' dependencies.

#![forbid(unsafe_code)]

pub mod balance;
pub mod crypto;
pub mod extractors;
pub mod models;
pub mod scanner;

pub use models::{
    BalanceInfo, ExtractedKey, ScanSummary, SourceType, SourceTypeStats, WalletScanResult,
};
