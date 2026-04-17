import json
from pathlib import Path

from orpheus.extractors.wallet_dump import WalletDumpExtractor
from orpheus.models import SourceType

KNOWN_WIF = "KwdMAjGmerYanjeui5SHS7JkmpZvVipYvB2LJGU1ZxJwYvP98617"


def test_extracts_wif_from_dumpwallet_text(tmp_path: Path) -> None:
    dump = tmp_path / "wallet.dump"
    dump.write_text(
        "# Wallet dump created by Bitcoin Core\n"
        "# \n"
        f"{KNOWN_WIF} 2024-01-01T00:00:00Z label=foo # addr=abc\n"
        "# End of dump\n"
    )
    ex = WalletDumpExtractor()
    assert ex.can_handle(dump)
    keys = ex.extract(dump)
    assert len(keys) == 1
    assert keys[0].wif == KNOWN_WIF
    assert keys[0].source_type == SourceType.WALLET_DUMP
    assert keys[0].address_compressed  # compressed address populated
    assert keys[0].address_bech32.startswith("bc1q")


def test_extracts_from_json_list(tmp_path: Path) -> None:
    dump = tmp_path / "wallet.json"
    dump.write_text(json.dumps([{"wif": KNOWN_WIF, "path": "m/44'/0'/0'/0/0"}]))
    ex = WalletDumpExtractor()
    keys = ex.extract(dump)
    assert len(keys) == 1
    assert keys[0].derivation_path == "m/44'/0'/0'/0/0"


def test_deduplicates_repeated_wifs(tmp_path: Path) -> None:
    dump = tmp_path / "wallet.txt"
    dump.write_text(f"{KNOWN_WIF}\n{KNOWN_WIF}\n")
    keys = WalletDumpExtractor().extract(dump)
    assert len(keys) == 1


def test_scan_wraps_errors(tmp_path: Path) -> None:
    missing = tmp_path / "nope.txt"
    result = WalletDumpExtractor().scan(missing)
    assert result.error is not None
    assert result.keys == []
