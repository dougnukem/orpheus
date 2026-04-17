"""BIP39 mnemonic extractor.

Derives keys across all common derivation paths by default:
  - BIP44:  m/44'/0'/0'/{0,1}/x       (legacy P2PKH)
  - BIP49:  m/49'/0'/0'/{0,1}/x       (wrapped SegWit)
  - BIP84:  m/84'/0'/0'/{0,1}/x       (native SegWit)
  - Breadwallet: m/0'/{0,1}/x         (legacy iOS wallet; critical path)

Breadwallet's m/0'/{0,1}/x is non-standard and deserves first-class support:
the 2026-04-17 recovery session unlocked ~0.04 BTC from a 2013-era Breadwallet
seed at m/0'/1/2 — the address was nowhere else.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

from mnemonic import Mnemonic

from orpheus.crypto.addresses import all_addresses_for_privkey
from orpheus.crypto.bip32 import ExtendedKey, derive_path, master_from_seed, mnemonic_to_seed
from orpheus.crypto.keys import privkey_to_wif
from orpheus.extractors.base import Extractor, register
from orpheus.models import ExtractedKey, SourceType


@dataclass(frozen=True)
class DerivationSpec:
    name: str
    account_path: str  # e.g., "m/44'/0'/0'"
    include_change: bool = True


DEFAULT_SPECS: tuple[DerivationSpec, ...] = (
    DerivationSpec("BIP44 (P2PKH)", "m/44'/0'/0'"),
    DerivationSpec("BIP49 (P2SH-P2WPKH)", "m/49'/0'/0'"),
    DerivationSpec("BIP84 (P2WPKH)", "m/84'/0'/0'"),
    DerivationSpec("Breadwallet", "m/0'"),
)


def derive_bip39(
    mnemonic: str,
    *,
    passphrase: str = "",
    gap_limit: int = 20,
    specs: tuple[DerivationSpec, ...] = DEFAULT_SPECS,
    source_file: str = "(mnemonic)",
) -> list[ExtractedKey]:
    if not Mnemonic("english").check(mnemonic):
        raise ValueError("mnemonic is not a valid BIP39 phrase")

    seed = mnemonic_to_seed(mnemonic, passphrase=passphrase)
    master = master_from_seed(seed)

    keys: list[ExtractedKey] = []
    for spec in specs:
        account = derive_path(master, spec.account_path)
        chains = (0, 1) if spec.include_change else (0,)
        for chain in chains:
            chain_key = _derive_child(account, chain)
            for i in range(gap_limit):
                child = _derive_child(chain_key, i)
                keys.append(_to_extracted_key(child, spec, chain, i, source_file))
    return keys


def _derive_child(parent: ExtendedKey, index: int) -> ExtendedKey:
    # Reuse derive_path to keep BIP32 logic in one place
    return derive_path(parent, f"m/{index}")


def _to_extracted_key(
    child: ExtendedKey,
    spec: DerivationSpec,
    chain: int,
    index: int,
    source_file: str,
) -> ExtractedKey:
    addrs = all_addresses_for_privkey(child.privkey)
    # Pick the canonical compressed address for this spec
    if "44" in spec.name:
        primary = addrs["p2pkh_compressed"]
    elif "49" in spec.name:
        primary = addrs["p2sh_p2wpkh"]
    elif "84" in spec.name:
        primary = addrs["bech32"]
    else:
        primary = addrs["p2pkh_compressed"]  # Breadwallet uses P2PKH

    return ExtractedKey(
        wif=privkey_to_wif(child.privkey, compressed=True),
        address_compressed=primary,
        address_uncompressed=addrs["p2pkh_uncompressed"],
        address_p2sh_segwit=addrs["p2sh_p2wpkh"],
        address_bech32=addrs["bech32"],
        source_file=source_file,
        source_type=SourceType.BIP39,
        derivation_path=f"{spec.account_path}/{chain}/{index}",
        notes=spec.name,
    )


@register
class BIP39TextFileExtractor(Extractor):
    """Extractor for .txt files whose contents are a BIP39 mnemonic phrase."""

    source_type = SourceType.BIP39

    def can_handle(self, path: Path) -> bool:
        if not path.is_file() or path.suffix.lower() not in (".txt", ".mnemonic", ""):
            return False
        try:
            text = path.read_text(errors="ignore").strip()
        except OSError:
            return False
        words = text.split()
        if len(words) not in (12, 15, 18, 21, 24):
            return False
        return Mnemonic("english").check(text)

    def extract(self, path: Path, passwords: list[str] | None = None) -> list[ExtractedKey]:
        mnemonic = path.read_text(errors="ignore").strip()
        # If a password file is provided, treat each entry as a BIP39 passphrase.
        if passwords:
            keys: list[ExtractedKey] = []
            for pw in [""] + passwords:
                keys.extend(derive_bip39(mnemonic, passphrase=pw, source_file=str(path)))
            return keys
        return derive_bip39(mnemonic, source_file=str(path))
