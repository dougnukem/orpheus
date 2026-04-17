from pathlib import Path

import pytest

from orpheus.extractors.bip39 import (
    BIP39TextFileExtractor,
    DEFAULT_SPECS,
    derive_bip39,
)

ABANDON = "abandon " * 11 + "about"


def test_abandon_bip44_first_address_matches_known_vector() -> None:
    keys = derive_bip39(ABANDON.strip(), gap_limit=1)
    bip44 = next(k for k in keys if k.derivation_path == "m/44'/0'/0'/0/0")
    assert bip44.address_compressed == "1LqBGSKuX5yYUonjxT5qGfpUsXKYYWeabA"


def test_abandon_bip84_first_address_matches_known_vector() -> None:
    keys = derive_bip39(ABANDON.strip(), gap_limit=1)
    bip84 = next(k for k in keys if k.derivation_path == "m/84'/0'/0'/0/0")
    # Known native-segwit first address for the canonical mnemonic
    assert bip84.address_bech32 == "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu"


def test_all_default_specs_produce_keys() -> None:
    keys = derive_bip39(ABANDON.strip(), gap_limit=2)
    # 4 specs * 2 chains (receive+change) * 2 indices = 16
    assert len(keys) == 16
    paths = {k.derivation_path for k in keys}
    assert "m/44'/0'/0'/0/0" in paths
    assert "m/49'/0'/0'/1/1" in paths
    assert "m/84'/0'/0'/0/1" in paths
    assert "m/0'/1/0" in paths  # Breadwallet change chain


def test_breadwallet_path_is_first_class() -> None:
    """Regression anchor: Breadwallet m/0'/1/x must derive keys."""
    keys = derive_bip39(ABANDON.strip(), gap_limit=5)
    bw_keys = [k for k in keys if k.derivation_path.startswith("m/0'/")]
    assert len(bw_keys) == 10  # 2 chains * 5 indices


def test_invalid_mnemonic_raises() -> None:
    with pytest.raises(ValueError):
        derive_bip39("not a real mnemonic phrase here at all")


def test_text_file_extractor_detects_mnemonic(tmp_path: Path) -> None:
    path = tmp_path / "seed.txt"
    path.write_text(ABANDON.strip())
    ex = BIP39TextFileExtractor()
    assert ex.can_handle(path)
    keys = ex.extract(path)
    assert any(k.derivation_path == "m/44'/0'/0'/0/0" for k in keys)


def test_text_file_extractor_rejects_non_mnemonic(tmp_path: Path) -> None:
    path = tmp_path / "random.txt"
    path.write_text("this is not a mnemonic")
    ex = BIP39TextFileExtractor()
    assert not ex.can_handle(path)


def test_default_specs_count() -> None:
    # Guard against accidental removal of Breadwallet path
    names = {s.name for s in DEFAULT_SPECS}
    assert "Breadwallet" in names
