"""Encrypted wallet coordinator.

Tries each candidate password against known encryption schemes:

  - MultiBit Classic v3: scrypt(password, salt=wallet_salt, N=16384, r=8, p=1)
    → AES-256-CBC with IV from protobuf entry
  - Blockchain.com wallet.aes.json: AES-256-CBC with IV=first 16 bytes of
    ciphertext; key = PBKDF2(password, iv, iter, 32) (iter defaults to 5000
    for legacy wallets, 10000+ for newer)
  - Bitcoin Core crypted_key: out of scope for v0.1 (requires BDB record parsing)

The coordinator delegates raw key scanning to the MultiBit / wallet_dump
extractors once the ciphertext is decrypted.
"""

from __future__ import annotations

import hashlib
import json
from dataclasses import dataclass
from pathlib import Path

from cryptography.hazmat.primitives.ciphers import Cipher, algorithms, modes

from orpheus.crypto.addresses import all_addresses_for_privkey
from orpheus.crypto.keys import privkey_to_wif, validate_privkey
from orpheus.extractors.base import Extractor
from orpheus.models import ExtractedKey, SourceType


@dataclass
class DecryptionAttempt:
    scheme: str
    ok: bool
    error: str | None = None


def _aes_cbc_decrypt(key: bytes, iv: bytes, ciphertext: bytes) -> bytes | None:
    try:
        cipher = Cipher(algorithms.AES(key), modes.CBC(iv))
        decryptor = cipher.decryptor()
        out = decryptor.update(ciphertext) + decryptor.finalize()
        pad_len = out[-1]
        if 1 <= pad_len <= 16 and all(b == pad_len for b in out[-pad_len:]):
            return out[:-pad_len]
        return out
    except Exception:  # noqa: BLE001
        return None


def try_multibit_v3(
    password: str,
    salt: bytes,
    iv: bytes,
    ciphertext: bytes,
    *,
    n: int = 16384,
    r: int = 8,
    p: int = 1,
) -> bytes | None:
    aes_key = hashlib.scrypt(
        password.encode("utf-8"), salt=salt, n=n, r=r, p=p, dklen=32
    )
    plaintext = _aes_cbc_decrypt(aes_key, iv, ciphertext)
    if plaintext and len(plaintext) == 32 and validate_privkey(plaintext):
        return plaintext
    return None


def try_blockchain_com_aes_json(
    password: str,
    wallet_json_text: str,
    *,
    iterations: int = 5000,
) -> str | None:
    """Decrypt a blockchain.com wallet.aes.json payload; returns plaintext JSON."""
    import base64

    data = json.loads(wallet_json_text)
    payload = data["payload"] if isinstance(data, dict) and "payload" in data else data
    if isinstance(payload, dict) and "payload" in payload:
        payload = payload["payload"]
    if not isinstance(payload, str):
        return None
    raw = base64.b64decode(payload)
    iv, ct = raw[:16], raw[16:]
    key = hashlib.pbkdf2_hmac("sha1", password.encode("utf-8"), iv, iterations, 32)
    plaintext = _aes_cbc_decrypt(key, iv, ct)
    if plaintext is None:
        return None
    try:
        text = plaintext.decode("utf-8")
    except UnicodeDecodeError:
        return None
    if '"' in text and "{" in text:
        return text
    return None


class EncryptedWalletExtractor(Extractor):
    """Try each password against recognized encrypted wallet formats."""

    source_type = SourceType.ENCRYPTED

    def can_handle(self, path: Path) -> bool:
        name = path.name.lower()
        return name.endswith(".aes.json") or name.endswith(".wallet")

    def extract(self, path: Path, passwords: list[str] | None = None) -> list[ExtractedKey]:
        if not passwords:
            return []
        data = path.read_bytes()
        name = path.name.lower()
        if name.endswith(".aes.json"):
            return self._try_blockchain_com(path, data.decode(errors="ignore"), passwords)
        # MultiBit: delegate to multibit.find_encrypted_entries + try_multibit_v3
        from orpheus.extractors.multibit import find_encrypted_entries

        entries = find_encrypted_entries(data)
        if not entries:
            return []
        salt = _find_scrypt_salt(data)
        if not salt:
            return []
        found: list[ExtractedKey] = []
        for pw in passwords:
            for iv, ct in entries:
                priv = try_multibit_v3(pw, salt, iv, ct)
                if priv:
                    addrs = all_addresses_for_privkey(priv)
                    found.append(
                        ExtractedKey(
                            wif=privkey_to_wif(priv, compressed=True),
                            address_compressed=addrs["p2pkh_compressed"],
                            address_uncompressed=addrs["p2pkh_uncompressed"],
                            address_p2sh_segwit=addrs["p2sh_p2wpkh"],
                            address_bech32=addrs["bech32"],
                            source_file=str(path),
                            source_type=SourceType.ENCRYPTED,
                            notes=f"multibit-v3 decrypted with password length {len(pw)}",
                        )
                    )
            if found:
                break
        return found

    def _try_blockchain_com(
        self, path: Path, text: str, passwords: list[str]
    ) -> list[ExtractedKey]:
        from orpheus.extractors.wallet_dump import _from_json

        for pw in passwords:
            for iterations in (5000, 10000, 25000):
                plaintext = try_blockchain_com_aes_json(pw, text, iterations=iterations)
                if plaintext is None:
                    continue
                keys = _from_json(plaintext, str(path))
                if keys:
                    return keys
        return []


def _find_scrypt_salt(data: bytes) -> bytes | None:
    # MultiBit v3 stores salt as a protobuf field tag 1, length 8.
    idx = data.find(b"\x0a\x08")
    while idx != -1 and idx + 10 < len(data):
        salt = data[idx + 2:idx + 10]
        # Must be followed by a varint-looking byte for scrypt params (n/r/p)
        if data[idx + 10] in (0x10, 0x18, 0x20):
            return salt
        idx = data.find(b"\x0a\x08", idx + 1)
    return None
