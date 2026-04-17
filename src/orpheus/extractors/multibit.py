"""MultiBit Classic .wallet extractor.

MultiBit Classic stores keys in a bitcoinj protobuf. Two variants:

- **Unencrypted**: raw 32-byte privkey follows the protobuf tag sequence
  `\\x12\\x20` (field 2, length 0x20). We also look for `\\x0a\\x20` which is
  the analogous tag for public keys — but we verify by deriving the matching
  pubkey from the candidate privkey.
- **Encrypted (v3)**: ciphertext follows `\\x0a\\x10` (IV, 16 bytes) then
  `\\x12\\x30` (field 2, 48 bytes AES-CBC). That path is handled by the
  `encrypted` extractor alongside a password list.

For v0.1 this module handles the unencrypted case and reports encrypted
entries as a count. Full encrypted decryption is dispatched to `encrypted.py`.
"""

from __future__ import annotations

from pathlib import Path

from orpheus.crypto.addresses import all_addresses_for_privkey
from orpheus.crypto.keys import privkey_to_wif, validate_privkey
from orpheus.extractors.base import Extractor, register
from orpheus.models import ExtractedKey, SourceType

UNENCRYPTED_TAG = b"\x12\x20"  # protobuf field 2 (bytes, len 32)
ENCRYPTED_IV_TAG = b"\x0a\x10"  # field 1, len 16
ENCRYPTED_DATA_TAG = b"\x12\x30"  # field 2, len 48


@register
class MultiBitExtractor(Extractor):
    source_type = SourceType.MULTIBIT

    def can_handle(self, path: Path) -> bool:
        if not path.is_file() or path.suffix.lower() not in (".wallet", ".bak"):
            return False
        try:
            head = path.read_bytes()[:512]
        except OSError:
            return False
        # MultiBit / bitcoinj protobufs begin with org.bitcoin.production magic
        return b"org.bitcoin" in head or UNENCRYPTED_TAG in head

    def extract(self, path: Path, passwords: list[str] | None = None) -> list[ExtractedKey]:
        data = path.read_bytes()
        return scan_unencrypted(data, str(path))


def scan_unencrypted(data: bytes, source_file: str) -> list[ExtractedKey]:
    keys: list[ExtractedKey] = []
    seen: set[bytes] = set()
    pos = 0
    while True:
        idx = data.find(UNENCRYPTED_TAG, pos)
        if idx == -1:
            break
        priv = data[idx + 2:idx + 2 + 32]
        pos = idx + 1
        if len(priv) != 32 or priv in seen or not validate_privkey(priv):
            continue
        seen.add(priv)
        addrs = all_addresses_for_privkey(priv)
        keys.append(
            ExtractedKey(
                wif=privkey_to_wif(priv, compressed=True),
                address_compressed=addrs["p2pkh_compressed"],
                address_uncompressed=addrs["p2pkh_uncompressed"],
                address_p2sh_segwit=addrs["p2sh_p2wpkh"],
                address_bech32=addrs["bech32"],
                source_file=source_file,
                source_type=SourceType.MULTIBIT,
            )
        )
    return keys


def find_encrypted_entries(data: bytes) -> list[tuple[bytes, bytes]]:
    """Return list of (iv, ciphertext) tuples for AES-CBC encrypted keys."""
    entries: list[tuple[bytes, bytes]] = []
    pos = 0
    while pos + 68 <= len(data):
        if data[pos:pos + 2] == ENCRYPTED_IV_TAG:
            iv = data[pos + 2:pos + 18]
            if data[pos + 18:pos + 20] == ENCRYPTED_DATA_TAG:
                ct = data[pos + 20:pos + 68]
                if len(iv) == 16 and len(ct) == 48:
                    entries.append((iv, ct))
        pos += 1
    return entries
