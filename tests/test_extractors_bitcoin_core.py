from pathlib import Path

from orpheus.extractors.bitcoin_core import DER_PATTERN, BitcoinCoreExtractor


def make_fake_bdb_blob(privkeys: list[bytes]) -> bytes:
    """Build a byte blob that mimics the parts of a BDB wallet the extractor cares about."""
    blob = b"\x00" * 128 + b"main\x00"  # fake header
    for priv in privkeys:
        blob += b"\x00" * 16 + DER_PATTERN + priv + b"\x00" * 32
    return blob


def test_extracts_planted_der_keys(tmp_path: Path) -> None:
    privs = [
        bytes.fromhex(
            "0c28fca386c7a227600b2fe50b7cae11ec86d3bf1fbe471be89827e19d72aa1d"
        ),
        bytes.fromhex(
            "18e14a7b6a307f426a94f8114701e7c8e774e7f9a47e2c2035db29a206321725"
        ),
    ]
    path = tmp_path / "wallet.dat"
    path.write_bytes(make_fake_bdb_blob(privs))

    ex = BitcoinCoreExtractor()
    assert ex.can_handle(path)
    keys = ex.extract(path)
    assert len(keys) == 2
    assert {k.wif for k in keys} == {
        "KwdMAjGmerYanjeui5SHS7JkmpZvVipYvB2LJGU1ZxJwYvP98617",
        "Kx45GeUBSMPReYQwgXiKhG9FzNXrnCeutJp4yjTd5kKxCitadm3C",
    }


def test_rejects_invalid_privkey_bytes(tmp_path: Path) -> None:
    blob = b"\x00" * 128 + b"main\x00" + DER_PATTERN + b"\xff" * 32
    path = tmp_path / "wallet.dat"
    path.write_bytes(blob)
    assert BitcoinCoreExtractor().extract(path) == []


def test_deduplicates_repeated_keys(tmp_path: Path) -> None:
    priv = bytes.fromhex(
        "0c28fca386c7a227600b2fe50b7cae11ec86d3bf1fbe471be89827e19d72aa1d"
    )
    path = tmp_path / "wallet.dat"
    path.write_bytes(make_fake_bdb_blob([priv, priv, priv]))
    assert len(BitcoinCoreExtractor().extract(path)) == 1
