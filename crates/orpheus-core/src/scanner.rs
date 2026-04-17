//! Directory scanner — dispatches to registered extractors.

use std::path::Path;

use walkdir::WalkDir;

use crate::{
    balance::{BalanceProvider, attach_balances},
    extractors::registry,
    models::WalletScanResult,
};

const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;
const SKIP_DIRS: &[&str] = &[".git", "__pycache__", "node_modules", ".venv", "target"];

pub fn scan_path(
    root: &Path,
    passwords: &[String],
    provider: Option<&dyn BalanceProvider>,
) -> Vec<WalletScanResult> {
    let extractors = registry();
    let mut results = Vec::new();
    for path in iter_files(root) {
        for ex in &extractors {
            if !ex.can_handle(&path) {
                continue;
            }
            let r = ex.extract(&path, passwords);
            if !r.keys.is_empty() || r.error.is_some() {
                results.push(r);
                break;
            }
        }
    }
    if let Some(p) = provider {
        for r in &mut results {
            attach_balances(&mut r.keys, p);
        }
    }
    results
}

fn iter_files(root: &Path) -> Vec<std::path::PathBuf> {
    if root.is_file() {
        return vec![root.to_path_buf()];
    }
    WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            !e.path()
                .components()
                .any(|c| SKIP_DIRS.iter().any(|s| c.as_os_str() == *s))
        })
        .filter(|e| {
            e.metadata()
                .map(|m| m.len() <= MAX_FILE_BYTES)
                .unwrap_or(false)
        })
        .map(|e| e.path().to_path_buf())
        .collect()
}
