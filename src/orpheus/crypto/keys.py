"""Secp256k1 key + address primitives.

Pure-Python on top of `ecdsa` + `base58`. Validated against BIP32 test vectors.
"""

from __future__ import annotations

import hashlib

import base58
import ecdsa

SECP256K1_ORDER = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141


def sha256(data: bytes) -> bytes:
    return hashlib.sha256(data).digest()


def ripemd160(data: bytes) -> bytes:
    h = hashlib.new("ripemd160")
    h.update(data)
    return h.digest()


def hash160(data: bytes) -> bytes:
    return ripemd160(sha256(data))


def validate_privkey(privkey: bytes) -> bool:
    if len(privkey) != 32:
        return False
    n = int.from_bytes(privkey, "big")
    return 0 < n < SECP256K1_ORDER


def privkey_to_pubkey(privkey: bytes, compressed: bool = True) -> bytes:
    if not validate_privkey(privkey):
        raise ValueError("invalid secp256k1 private key")
    sk = ecdsa.SigningKey.from_string(privkey, curve=ecdsa.SECP256k1)
    vk = sk.get_verifying_key()
    x = vk.pubkey.point.x()
    y = vk.pubkey.point.y()
    if compressed:
        prefix = b"\x02" if y % 2 == 0 else b"\x03"
        return prefix + x.to_bytes(32, "big")
    return b"\x04" + x.to_bytes(32, "big") + y.to_bytes(32, "big")


def pubkey_to_p2pkh(pubkey: bytes, version: int = 0x00) -> str:
    return base58.b58encode_check(bytes([version]) + hash160(pubkey)).decode()


def privkey_to_wif(privkey: bytes, compressed: bool = True, version: int = 0x80) -> str:
    payload = bytes([version]) + privkey + (b"\x01" if compressed else b"")
    return base58.b58encode_check(payload).decode()


def wif_to_privkey(wif: str) -> tuple[bytes, bool]:
    """Return (privkey_bytes, compressed)."""
    raw = base58.b58decode_check(wif)
    if len(raw) == 34 and raw[-1] == 0x01:
        return raw[1:33], True
    if len(raw) == 33:
        return raw[1:33], False
    raise ValueError(f"unexpected WIF length: {len(raw)}")


def privkey_to_p2sh_p2wpkh(privkey: bytes, version: int = 0x05) -> str:
    """Return BIP49 wrapped-SegWit address (P2SH-P2WPKH)."""
    pubkey = privkey_to_pubkey(privkey, compressed=True)
    redeem = b"\x00\x14" + hash160(pubkey)
    return base58.b58encode_check(bytes([version]) + hash160(redeem)).decode()
