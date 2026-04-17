"""Bech32 tests using BIP173 reference vectors."""

import pytest

from orpheus.crypto.bech32 import (
    decode_segwit_address,
    encode_segwit_address,
    p2wpkh_address,
)


def test_bip173_vector_mainnet_v0_p2wpkh() -> None:
    # From BIP173: encode of p2wpkh with 20-byte program 751e76e8...
    witprog = bytes.fromhex("751e76e8199196d454941c45d1b3a323f1433bd6")
    addr = encode_segwit_address("bc", 0, witprog)
    assert addr == "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4"


def test_bip173_vector_mainnet_v0_p2wsh() -> None:
    witprog = bytes.fromhex(
        "1863143c14c5166804bd19203356da136c985678cd4d27a1b8c6329604903262"
    )
    addr = encode_segwit_address("bc", 0, witprog)
    assert (
        addr
        == "bc1qrp33g0q5c5txsp9arysrx4k6zdkfs4nce4xj0gdcccefvpysxf3qccfmv3"
    )


def test_roundtrip_v0() -> None:
    witprog = bytes(range(20))
    addr = encode_segwit_address("bc", 0, witprog)
    ver, prog = decode_segwit_address("bc", addr)
    assert ver == 0
    assert prog == witprog


def test_v1_bech32m_roundtrip() -> None:
    witprog = bytes(range(32))
    addr = encode_segwit_address("bc", 1, witprog)
    ver, prog = decode_segwit_address("bc", addr)
    assert ver == 1
    assert prog == witprog


def test_p2wpkh_from_pubkey() -> None:
    from orpheus.crypto.keys import privkey_to_pubkey

    pub = privkey_to_pubkey((1).to_bytes(32, "big"), compressed=True)
    addr = p2wpkh_address(pub)
    assert addr.startswith("bc1q")


def test_decode_rejects_mixed_case() -> None:
    with pytest.raises(ValueError):
        decode_segwit_address("bc", "bc1QW508D6QEJXTDG4Y5R3ZarVary0c5XW7KV8F3T4")
