"""Key primitive tests against BIP test vectors and WIF round-trips."""

from orpheus.crypto.keys import (
    hash160,
    privkey_to_p2sh_p2wpkh,
    privkey_to_pubkey,
    privkey_to_wif,
    pubkey_to_p2pkh,
    validate_privkey,
    wif_to_privkey,
)


def test_validate_privkey_rejects_zero_and_n() -> None:
    assert not validate_privkey(b"\x00" * 32)
    assert not validate_privkey(b"\xff" * 32)  # > n
    assert validate_privkey((1).to_bytes(32, "big"))


def test_privkey_to_pubkey_bitcoin_wiki_vector() -> None:
    # From https://en.bitcoin.it/wiki/Private_key — canonical vector
    # priv = 0x18e14a7b6a307f426a94f8114701e7c8e774e7f9a47e2c2035db29a206321725
    priv = bytes.fromhex(
        "18e14a7b6a307f426a94f8114701e7c8e774e7f9a47e2c2035db29a206321725"
    )
    pub_uncomp = privkey_to_pubkey(priv, compressed=False)
    assert pub_uncomp.hex() == (
        "0450863ad64a87ae8a2fe83c1af1a8403cb53f53e486d8511dad8a04887e5b2352"
        "2cd470243453a299fa9e77237716103abc11a1df38855ed6f2ee187e9c582ba6"
    )
    addr = pubkey_to_p2pkh(pub_uncomp)
    assert addr == "16UwLL9Risc3QfPqBUvKofHmBQ7wMtjvM"


def test_privkey_to_pubkey_compressed_produces_33_bytes() -> None:
    priv = (2).to_bytes(32, "big")
    pub = privkey_to_pubkey(priv, compressed=True)
    assert len(pub) == 33
    assert pub[0] in (0x02, 0x03)


def test_wif_roundtrip_compressed() -> None:
    priv = bytes.fromhex(
        "0c28fca386c7a227600b2fe50b7cae11ec86d3bf1fbe471be89827e19d72aa1d"
    )
    wif = privkey_to_wif(priv, compressed=True)
    assert wif == "KwdMAjGmerYanjeui5SHS7JkmpZvVipYvB2LJGU1ZxJwYvP98617"
    recovered, comp = wif_to_privkey(wif)
    assert recovered == priv
    assert comp is True


def test_wif_roundtrip_uncompressed() -> None:
    priv = bytes.fromhex(
        "0c28fca386c7a227600b2fe50b7cae11ec86d3bf1fbe471be89827e19d72aa1d"
    )
    wif = privkey_to_wif(priv, compressed=False)
    assert wif == "5HueCGU8rMjxEXxiPuD5BDku4MkFqeZyd4dZ1jvhTVqvbTLvyTJ"
    recovered, comp = wif_to_privkey(wif)
    assert recovered == priv
    assert comp is False


def test_hash160_known_vector() -> None:
    # hash160(empty) = b472a266d0bd89c13706a4132ccfb16f7c3b9fcb
    assert hash160(b"").hex() == "b472a266d0bd89c13706a4132ccfb16f7c3b9fcb"


def test_p2sh_p2wpkh_format() -> None:
    """P2SH-P2WPKH mainnet addresses always start with '3' and are 34 chars."""
    priv = (1).to_bytes(32, "big")
    addr = privkey_to_p2sh_p2wpkh(priv)
    assert addr.startswith("3")
    assert 33 <= len(addr) <= 35
