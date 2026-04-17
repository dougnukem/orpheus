"""Bitcoin Core wallet.dat extractor.

Works on both BDB and SQLite backends by scanning raw bytes for the DER-encoded
secp256k1 private-key pattern used by Bitcoin Core:

    30 81 D3 02 01 01 04 20 <32 bytes>

Encrypted (password-protected) wallets won't yield raw keys from this scan —
they appear as AES-CBC ciphertext keyed by the wallet password. For those we
rely on the `encrypted` extractor path (which can wrap this one).
"""

from __future__ import annotations

from pathlib import Path

from orpheus.crypto.addresses import all_addresses_for_privkey
from orpheus.crypto.keys import privkey_to_wif, validate_privkey
from orpheus.extractors.base import Extractor, register
from orpheus.models import ExtractedKey, SourceType

DER_PATTERN = b"\x30\x81\xd3\x02\x01\x01\x04\x20"
PRIVKEY_LEN = 32


@register
class BitcoinCoreExtractor(Extractor):
    source_type = SourceType.BITCOIN_CORE

    def can_handle(self, path: Path) -> bool:
        if not path.is_file():
            return False
        name = path.name.lower()
        # Only claim files that look like Bitcoin Core wallets — prefix name checks
        # aren't enough because MultiBit also uses *.wallet.
        if not (name.endswith(".dat") or name == "wallet"):
            return False
        try:
            head = path.read_bytes()[:4096]
        except OSError:
            return False
        return DER_PATTERN in head or b"main\x00" in head

    def extract(self, path: Path, passwords: list[str] | None = None) -> list[ExtractedKey]:
        data = path.read_bytes()
        keys: list[ExtractedKey] = []
        seen: set[bytes] = set()
        pos = 0
        while True:
            idx = data.find(DER_PATTERN, pos)
            if idx == -1:
                break
            start = idx + len(DER_PATTERN)
            privkey = data[start:start + PRIVKEY_LEN]
            pos = idx + 1
            if len(privkey) != PRIVKEY_LEN or privkey in seen or not validate_privkey(privkey):
                continue
            seen.add(privkey)
            keys.append(_privkey_to_extracted(privkey, str(path)))
        return keys


def _privkey_to_extracted(privkey: bytes, source_file: str) -> ExtractedKey:
    addrs = all_addresses_for_privkey(privkey)
    return ExtractedKey(
        wif=privkey_to_wif(privkey, compressed=True),
        address_compressed=addrs["p2pkh_compressed"],
        address_uncompressed=addrs["p2pkh_uncompressed"],
        address_p2sh_segwit=addrs["p2sh_p2wpkh"],
        address_bech32=addrs["bech32"],
        source_file=source_file,
        source_type=SourceType.BITCOIN_CORE,
    )
