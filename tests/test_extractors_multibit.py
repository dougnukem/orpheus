from pathlib import Path

from orpheus.extractors.multibit import (
    ENCRYPTED_DATA_TAG,
    ENCRYPTED_IV_TAG,
    UNENCRYPTED_TAG,
    MultiBitExtractor,
    find_encrypted_entries,
    scan_unencrypted,
)


def build_unencrypted_multibit(privs: list[bytes]) -> bytes:
    blob = b"org.bitcoin.production" + b"\x00" * 16
    for p in privs:
        blob += b"\x0a\x21\x00" + b"\x00" * 32  # fake pubkey field (ignored)
        blob += UNENCRYPTED_TAG + p
    return blob


def build_encrypted_multibit(entries: list[tuple[bytes, bytes]]) -> bytes:
    blob = b"org.bitcoin.production" + b"\x00" * 8
    for iv, ct in entries:
        blob += ENCRYPTED_IV_TAG + iv + ENCRYPTED_DATA_TAG + ct
    return blob


def test_extracts_unencrypted_privkeys(tmp_path: Path) -> None:
    priv = bytes.fromhex(
        "0c28fca386c7a227600b2fe50b7cae11ec86d3bf1fbe471be89827e19d72aa1d"
    )
    path = tmp_path / "sample.wallet"
    path.write_bytes(build_unencrypted_multibit([priv]))

    ex = MultiBitExtractor()
    assert ex.can_handle(path)
    keys = ex.extract(path)
    assert len(keys) == 1
    assert keys[0].wif == "KwdMAjGmerYanjeui5SHS7JkmpZvVipYvB2LJGU1ZxJwYvP98617"


def test_scan_unencrypted_skips_invalid() -> None:
    blob = b"org.bitcoin.production" + UNENCRYPTED_TAG + b"\xff" * 32
    assert scan_unencrypted(blob, "x") == []


def test_finds_encrypted_entries() -> None:
    iv = b"I" * 16
    ct = b"C" * 48
    blob = build_encrypted_multibit([(iv, ct), (iv, ct)])
    entries = find_encrypted_entries(blob)
    assert len(entries) == 2
    assert entries[0] == (iv, ct)
