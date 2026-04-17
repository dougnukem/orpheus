"""Parse Bitcoin Core `dumpwallet` text output.

Accepted formats: the classic `dumpwallet` output (one WIF per line, commented lines OK)
and the `descriptors dump` JSON format.
"""

from __future__ import annotations

import json
import re
from pathlib import Path

from orpheus.crypto.addresses import all_addresses_for_privkey
from orpheus.crypto.keys import wif_to_privkey
from orpheus.extractors.base import Extractor, register
from orpheus.models import ExtractedKey, SourceType

WIF_RE = re.compile(r"\b([5KL9c][1-9A-HJ-NP-Za-km-z]{50,51})\b")


@register
class WalletDumpExtractor(Extractor):
    source_type = SourceType.WALLET_DUMP

    def can_handle(self, path: Path) -> bool:
        if not path.is_file():
            return False
        suffix = path.suffix.lower()
        if suffix in (".txt", ".dump", ".json"):
            return True
        # Fallback: peek at contents
        try:
            head = path.read_bytes()[:1024]
        except OSError:
            return False
        return b"dumpwallet" in head or WIF_RE.search(head.decode("utf-8", "ignore")) is not None

    def extract(self, path: Path, passwords: list[str] | None = None) -> list[ExtractedKey]:
        text = path.read_text(errors="ignore")
        stripped = text.lstrip()
        if stripped.startswith("{") or stripped.startswith("["):
            keys = _from_json(text, str(path))
            if keys:
                return keys
        return _from_text(text, str(path))


def _make_key(wif: str, source_file: str, derivation_path: str | None = None) -> ExtractedKey | None:
    try:
        priv, compressed = wif_to_privkey(wif)
    except Exception:  # noqa: BLE001
        return None
    addrs = all_addresses_for_privkey(priv)
    return ExtractedKey(
        wif=wif,
        address_compressed=addrs["p2pkh_compressed"] if compressed else addrs["p2pkh_uncompressed"],
        address_uncompressed=addrs["p2pkh_uncompressed"] if compressed else None,
        address_p2sh_segwit=addrs["p2sh_p2wpkh"],
        address_bech32=addrs["bech32"],
        source_file=source_file,
        source_type=SourceType.WALLET_DUMP,
        derivation_path=derivation_path,
    )


def _from_text(text: str, source_file: str) -> list[ExtractedKey]:
    keys: list[ExtractedKey] = []
    seen: set[str] = set()
    for match in WIF_RE.finditer(text):
        wif = match.group(1)
        if wif in seen:
            continue
        seen.add(wif)
        key = _make_key(wif, source_file)
        if key:
            keys.append(key)
    return keys


def _from_json(text: str, source_file: str) -> list[ExtractedKey]:
    try:
        data = json.loads(text)
    except json.JSONDecodeError:
        return []
    keys: list[ExtractedKey] = []
    # Common shapes: {"wif": ...}, [{"wif": ..., "path": ...}, ...],
    #                {"descriptors": [{"desc": "wpkh(WIF)..."}]}
    candidates: list[tuple[str, str | None]] = []
    if isinstance(data, dict) and "descriptors" in data:
        for d in data["descriptors"]:
            desc = d.get("desc", "")
            for m in WIF_RE.finditer(desc):
                candidates.append((m.group(1), d.get("timestamp") and str(d.get("timestamp"))))
    elif isinstance(data, list):
        for entry in data:
            if isinstance(entry, dict) and "wif" in entry:
                candidates.append((entry["wif"], entry.get("path")))
    elif isinstance(data, dict) and "wif" in data:
        candidates.append((data["wif"], data.get("path")))

    seen: set[str] = set()
    for wif, path in candidates:
        if wif in seen:
            continue
        seen.add(wif)
        k = _make_key(wif, source_file, path)
        if k:
            keys.append(k)
    return keys
