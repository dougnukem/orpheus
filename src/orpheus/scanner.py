"""Scan a path (file or directory) by dispatching to registered extractors."""

from __future__ import annotations

from pathlib import Path

from orpheus.balance import BalanceProvider, attach_balances
from orpheus.extractors.base import list_extractors
from orpheus.models import WalletScanResult

# Import for side effect — populates the registry
from orpheus.extractors import (  # noqa: F401
    bip39,
    bitcoin_core,
    encrypted,
    multibit,
    wallet_dump,
)


SKIP_DIRS = {".git", "__pycache__", "node_modules", ".venv"}


def iter_candidate_files(root: Path) -> list[Path]:
    if root.is_file():
        return [root]
    files: list[Path] = []
    for p in root.rglob("*"):
        if p.is_dir():
            continue
        if any(part in SKIP_DIRS for part in p.parts):
            continue
        if p.stat().st_size > 64 * 1024 * 1024:  # skip files > 64 MiB
            continue
        files.append(p)
    return files


def scan_path(
    root: Path,
    *,
    passwords: list[str] | None = None,
    provider: BalanceProvider | None = None,
) -> list[WalletScanResult]:
    results: list[WalletScanResult] = []
    extractors = list_extractors()
    for path in iter_candidate_files(root):
        for ex in extractors:
            try:
                handles = ex.can_handle(path)
            except Exception:  # noqa: BLE001
                handles = False
            if not handles:
                continue
            result = ex.scan(path, passwords=passwords)
            if result.keys:
                results.append(result)
                break
            if result.error:
                results.append(result)
                # Try another extractor only if no error surfaced yet
                continue

    if provider is not None:
        flat_keys = [k for r in results for k in r.keys]
        if flat_keys:
            attach_balances(flat_keys, provider)
    return results
