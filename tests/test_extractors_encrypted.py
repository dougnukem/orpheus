"""Round-trip tests for the encrypted coordinator.

Builds a synthetic MultiBit-v3-like blob whose single encrypted key we can
decrypt with a known password. Proves the scrypt + AES-CBC path end-to-end.
"""

from __future__ import annotations

import hashlib
import os
from pathlib import Path

from cryptography.hazmat.primitives.ciphers import Cipher, algorithms, modes

from orpheus.extractors.encrypted import EncryptedWalletExtractor, try_multibit_v3
from orpheus.extractors.multibit import ENCRYPTED_DATA_TAG, ENCRYPTED_IV_TAG


def build_encrypted_multibit(priv: bytes, password: str) -> bytes:
    salt = os.urandom(8)
    aes_key = hashlib.scrypt(
        password.encode("utf-8"), salt=salt, n=16384, r=8, p=1, dklen=32
    )
    iv = os.urandom(16)
    # PKCS7 pad 32 bytes to 48
    pad = 16
    plaintext = priv + bytes([pad]) * pad
    cipher = Cipher(algorithms.AES(aes_key), modes.CBC(iv))
    enc = cipher.encryptor()
    ct = enc.update(plaintext) + enc.finalize()

    # Lay out a minimal protobuf-ish blob:
    blob = b"org.bitcoin.production" + b"\x00" * 4
    # scrypt salt: tag 1 len 8, then a scrypt param varint byte 0x10
    blob += b"\x0a\x08" + salt + b"\x10\x80\x80\x01"  # N=16384
    # Encrypted entry: IV tag + iv + DATA tag + ct
    blob += ENCRYPTED_IV_TAG + iv + ENCRYPTED_DATA_TAG + ct
    return blob


def test_try_multibit_v3_roundtrip() -> None:
    priv = bytes.fromhex(
        "0c28fca386c7a227600b2fe50b7cae11ec86d3bf1fbe471be89827e19d72aa1d"
    )
    password = "orpheus-demo"
    salt = os.urandom(8)
    aes_key = hashlib.scrypt(
        password.encode("utf-8"), salt=salt, n=16384, r=8, p=1, dklen=32
    )
    iv = os.urandom(16)
    plaintext = priv + bytes([16]) * 16
    enc = Cipher(algorithms.AES(aes_key), modes.CBC(iv)).encryptor()
    ct = enc.update(plaintext) + enc.finalize()

    result = try_multibit_v3(password, salt, iv, ct)
    assert result == priv
    # Wrong password returns None
    assert try_multibit_v3("wrong-password", salt, iv, ct) is None


def test_encrypted_extractor_decrypts_multibit(tmp_path: Path) -> None:
    priv = bytes.fromhex(
        "0c28fca386c7a227600b2fe50b7cae11ec86d3bf1fbe471be89827e19d72aa1d"
    )
    path = tmp_path / "protected.wallet"
    path.write_bytes(build_encrypted_multibit(priv, "orpheus-demo"))

    ex = EncryptedWalletExtractor()
    assert ex.can_handle(path)
    keys = ex.extract(path, passwords=["wrong", "orpheus-demo", "also-wrong"])
    assert len(keys) == 1
    assert keys[0].wif == "KwdMAjGmerYanjeui5SHS7JkmpZvVipYvB2LJGU1ZxJwYvP98617"


def test_encrypted_extractor_returns_nothing_without_passwords(tmp_path: Path) -> None:
    path = tmp_path / "protected.wallet"
    path.write_bytes(build_encrypted_multibit(b"\x01" * 32, "pw"))
    assert EncryptedWalletExtractor().extract(path, passwords=None) == []
