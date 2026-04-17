//! Orpheus Tauri v2 desktop entrypoint.
//!
//! Frontend is the same React app that axum serves; the Tauri shell exposes
//! `#[tauri::command]` handlers that call directly into `orpheus-core`
//! — no HTTP sidecar.

use std::path::PathBuf;

use orpheus_core::{
    ExtractedKey, WalletScanResult,
    balance::{BalanceProvider, MockProvider},
    extractors::bip39_mnemonic::{DEFAULT_SPECS, derive_bip39},
    extractors::blockchain_com::decode_mnemonic,
    scanner::scan_path,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct ScanReply {
    results: Vec<WalletScanResult>,
}

#[tauri::command]
async fn scan_paths(
    paths: Vec<String>,
    passwords: Vec<String>,
    provider: Option<String>,
    mock_file: Option<String>,
) -> Result<ScanReply, String> {
    let provider_name = provider.unwrap_or_else(|| "none".into());
    let mock_path = mock_file.map(PathBuf::from);

    let mut all = Vec::new();
    for p in paths {
        let root = PathBuf::from(p);
        let passwords = passwords.clone();
        let provider_name = provider_name.clone();
        let mock_path = mock_path.clone();
        let partial =
            tokio::task::spawn_blocking(move || -> Result<Vec<WalletScanResult>, String> {
                let provider: Option<Box<dyn BalanceProvider>> = match provider_name.as_str() {
                    "none" => None,
                    "mock" => Some(Box::new(MockProvider { path: mock_path })),
                    other => return Err(format!("provider {other} not supported in desktop mode")),
                };
                Ok(scan_path(&root, &passwords, provider.as_deref()))
            })
            .await
            .map_err(|e| e.to_string())??;
        all.extend(partial);
    }
    Ok(ScanReply { results: all })
}

#[derive(Debug, Deserialize)]
pub struct MnemonicInput {
    phrase: String,
    #[serde(default = "default_kind")]
    kind: String,
    #[serde(default)]
    passphrase: String,
    #[serde(default = "default_gap")]
    gap_limit: u32,
    #[serde(default)]
    wordlist: Option<String>,
}

fn default_kind() -> String {
    "bip39".into()
}
fn default_gap() -> u32 {
    20
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum MnemonicReply {
    Bip39 { keys: Vec<ExtractedKey> },
    Blockchain { decoded: DecodedReply },
}

#[derive(Debug, Serialize)]
pub struct DecodedReply {
    password: String,
    word_count: usize,
    version: String,
}

#[tauri::command]
fn mnemonic(input: MnemonicInput) -> Result<MnemonicReply, String> {
    if input.phrase.trim().is_empty() {
        return Err("phrase is required".into());
    }
    match input.kind.as_str() {
        "bip39" => {
            let keys = derive_bip39(
                input.phrase.trim(),
                &input.passphrase,
                input.gap_limit,
                DEFAULT_SPECS,
                "(mnemonic)",
            )?;
            Ok(MnemonicReply::Bip39 { keys })
        }
        "blockchain" => {
            let path = input
                .wordlist
                .ok_or_else(|| "blockchain.com mnemonics require a wordlist path".to_string())?;
            let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
            let words: Vec<String> = text
                .lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect();
            let decoded =
                decode_mnemonic(input.phrase.trim(), &words).map_err(|e| e.to_string())?;
            Ok(MnemonicReply::Blockchain {
                decoded: DecodedReply {
                    password: decoded.password,
                    word_count: decoded.word_count,
                    version: decoded.version_guess,
                },
            })
        }
        other => Err(format!("unknown kind: {other}")),
    }
}

#[tauri::command]
async fn demo(app_data_dir: Option<String>) -> Result<ScanReply, String> {
    let fixtures = match app_data_dir {
        Some(dir) => PathBuf::from(dir),
        None => workspace_root().join("fixtures"),
    };
    let provider = MockProvider {
        path: Some(fixtures.join("mock_balances.json")),
    };
    let demo_dir = fixtures.join("demo-wallets");
    let results = tokio::task::spawn_blocking(move || {
        scan_path(
            &demo_dir,
            &["orpheus-demo".to_string()],
            Some(&provider as &dyn BalanceProvider),
        )
    })
    .await
    .map_err(|e| e.to_string())?;
    Ok(ScanReply { results })
}

fn workspace_root() -> PathBuf {
    let here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for ancestor in here.ancestors() {
        let cargo = ancestor.join("Cargo.toml");
        if cargo.exists()
            && std::fs::read_to_string(&cargo)
                .map(|t| t.contains("[workspace]"))
                .unwrap_or(false)
        {
            return ancestor.to_path_buf();
        }
    }
    here
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![scan_paths, mnemonic, demo])
        .run(tauri::generate_context!())
        .expect("error while running Orpheus");
}
