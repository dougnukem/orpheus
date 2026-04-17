"""BIP32 + BIP39 derivation tests.

Uses BIP32 test vector 1 seed and the canonical BIP39
"abandon x11 about" mnemonic to anchor correctness.
"""

from orpheus.crypto.bip32 import (
    derive_path,
    master_from_seed,
    mnemonic_to_seed,
    parse_path,
    HARDENED,
)
from orpheus.crypto.keys import privkey_to_wif, pubkey_to_p2pkh


BIP32_VECTOR1_SEED = bytes.fromhex("000102030405060708090a0b0c0d0e0f")


def test_parse_path_hardened_and_normal() -> None:
    assert parse_path("m") == []
    assert parse_path("m/0") == [0]
    assert parse_path("m/0'") == [HARDENED]
    assert parse_path("m/44'/0'/0'/0/0") == [
        44 | HARDENED,
        0 | HARDENED,
        0 | HARDENED,
        0,
        0,
    ]


def test_bip32_vector1_master_privkey() -> None:
    m = master_from_seed(BIP32_VECTOR1_SEED)
    # BIP32 spec: master key from seed 000102...0f
    assert m.privkey.hex() == (
        "e8f32e723decf4051aefac8e2c93c9c5b214313817cdb01a1494b917c8436b35"
    )
    assert m.chain_code.hex() == (
        "873dff81c02f525623fd1fe5167eac3a55a049de3d314bb42ee227ffed37d508"
    )


def test_bip32_vector1_m_0h() -> None:
    m = master_from_seed(BIP32_VECTOR1_SEED)
    child = derive_path(m, "m/0'")
    # BIP32 test vector 1 chain m/0': priv edb2e14f9ee77d26dd93b4ecede8d16ed408ce149b6cd80b0715a2d911a0afea
    assert child.privkey.hex() == (
        "edb2e14f9ee77d26dd93b4ecede8d16ed408ce149b6cd80b0715a2d911a0afea"
    )


def test_bip39_abandon_bip44_first_address() -> None:
    """Canonical BIP39 test mnemonic → known first BIP44 receive address."""
    mnemonic = "abandon " * 11 + "about"
    seed = mnemonic_to_seed(mnemonic.strip())
    master = master_from_seed(seed)
    child = derive_path(master, "m/44'/0'/0'/0/0")

    from orpheus.crypto.keys import privkey_to_pubkey

    pub = privkey_to_pubkey(child.privkey, compressed=True)
    addr = pubkey_to_p2pkh(pub)
    # This is the canonical first address for the "abandon about" mnemonic
    # per iancoleman.io/bip39 & many tools.
    assert addr == "1LqBGSKuX5yYUonjxT5qGfpUsXKYYWeabA"


def test_bip39_breadwallet_path_derivation_works() -> None:
    """Breadwallet used m/0'/0/x and m/0'/1/x — non-BIP44. Must derive cleanly."""
    mnemonic = "abandon " * 11 + "about"
    seed = mnemonic_to_seed(mnemonic.strip())
    master = master_from_seed(seed)
    receive = derive_path(master, "m/0'/0/0")
    change = derive_path(master, "m/0'/1/0")
    # We don't assert specific addresses; just that distinct paths derive distinct keys
    assert receive.privkey != change.privkey
    assert len(receive.privkey) == 32

    wif = privkey_to_wif(receive.privkey, compressed=True)
    assert wif.startswith("K") or wif.startswith("L")
