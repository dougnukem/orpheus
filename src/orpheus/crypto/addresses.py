"""Address derivation helpers that combine keys + bech32 for every format."""

from __future__ import annotations

from orpheus.crypto.bech32 import p2wpkh_address
from orpheus.crypto.keys import (
    privkey_to_p2sh_p2wpkh,
    privkey_to_pubkey,
    pubkey_to_p2pkh,
)


def all_addresses_for_privkey(privkey: bytes) -> dict[str, str]:
    """Return every derivable address format for a secp256k1 private key."""
    pub_comp = privkey_to_pubkey(privkey, compressed=True)
    pub_uncomp = privkey_to_pubkey(privkey, compressed=False)
    return {
        "p2pkh_compressed": pubkey_to_p2pkh(pub_comp),
        "p2pkh_uncompressed": pubkey_to_p2pkh(pub_uncomp),
        "p2sh_p2wpkh": privkey_to_p2sh_p2wpkh(privkey),
        "bech32": p2wpkh_address(pub_comp),
    }
