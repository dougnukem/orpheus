"""Minimal BIP32 HD derivation.

Supports `m/a/b/c` paths with hardened (`a'`) or normal children,
plus path-string parsing `derive_path(seed, "m/44'/0'/0'/0/0")`.
"""

from __future__ import annotations

import hashlib
import hmac
from dataclasses import dataclass

import ecdsa

from orpheus.crypto.keys import SECP256K1_ORDER, privkey_to_pubkey

HARDENED = 0x80000000


@dataclass(frozen=True)
class ExtendedKey:
    privkey: bytes
    chain_code: bytes
    depth: int = 0
    parent_fingerprint: bytes = b"\x00\x00\x00\x00"
    child_number: int = 0

    @property
    def pubkey(self) -> bytes:
        return privkey_to_pubkey(self.privkey, compressed=True)


def master_from_seed(seed: bytes) -> ExtendedKey:
    i = hmac.new(b"Bitcoin seed", seed, hashlib.sha512).digest()
    il, ir = i[:32], i[32:]
    if int.from_bytes(il, "big") == 0 or int.from_bytes(il, "big") >= SECP256K1_ORDER:
        raise ValueError("invalid master key from seed")
    return ExtendedKey(privkey=il, chain_code=ir)


def _ckd_priv(parent: ExtendedKey, index: int) -> ExtendedKey:
    if index < 0 or index > 0xFFFFFFFF:
        raise ValueError("child index out of range")
    if index & HARDENED:
        data = b"\x00" + parent.privkey + index.to_bytes(4, "big")
    else:
        data = parent.pubkey + index.to_bytes(4, "big")
    i = hmac.new(parent.chain_code, data, hashlib.sha512).digest()
    il, ir = i[:32], i[32:]
    il_int = int.from_bytes(il, "big")
    if il_int >= SECP256K1_ORDER:
        raise ValueError("derived IL out of range; try next index")
    child_int = (il_int + int.from_bytes(parent.privkey, "big")) % SECP256K1_ORDER
    if child_int == 0:
        raise ValueError("derived child key is zero; try next index")
    child_priv = child_int.to_bytes(32, "big")
    parent_fingerprint = _fingerprint(parent)
    return ExtendedKey(
        privkey=child_priv,
        chain_code=ir,
        depth=parent.depth + 1,
        parent_fingerprint=parent_fingerprint,
        child_number=index,
    )


def _fingerprint(key: ExtendedKey) -> bytes:
    from orpheus.crypto.keys import hash160

    return hash160(key.pubkey)[:4]


def parse_path(path: str) -> list[int]:
    parts = path.strip().split("/")
    if parts[0] != "m":
        raise ValueError(f"path must start with 'm': {path}")
    indices: list[int] = []
    for part in parts[1:]:
        if not part:
            continue
        hardened = part.endswith("'") or part.endswith("h")
        num_str = part.rstrip("'h")
        n = int(num_str)
        if hardened:
            n |= HARDENED
        indices.append(n)
    return indices


def derive_path(master: ExtendedKey, path: str) -> ExtendedKey:
    key = master
    for idx in parse_path(path):
        key = _ckd_priv(key, idx)
    return key


def mnemonic_to_seed(mnemonic: str, passphrase: str = "") -> bytes:
    from mnemonic import Mnemonic

    return Mnemonic.to_seed(mnemonic, passphrase=passphrase)
