"""Core data models for Orpheus."""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum


class SourceType(str, Enum):
    BITCOIN_CORE = "bitcoin_core"
    MULTIBIT = "multibit"
    BIP39 = "bip39"
    BLOCKCHAIN_COM = "blockchain_com"
    WALLET_DUMP = "wallet_dump"
    ENCRYPTED = "encrypted"
    UNKNOWN = "unknown"


@dataclass
class ExtractedKey:
    wif: str
    address_compressed: str
    source_file: str
    source_type: SourceType
    address_uncompressed: str | None = None
    address_p2sh_segwit: str | None = None
    address_bech32: str | None = None
    derivation_path: str | None = None
    balance_sat: int | None = None
    total_received_sat: int | None = None
    tx_count: int | None = None
    notes: str | None = None


@dataclass
class BalanceInfo:
    address: str
    balance_sat: int
    total_received_sat: int
    tx_count: int


@dataclass
class WalletScanResult:
    source_file: str
    source_type: SourceType
    keys: list[ExtractedKey] = field(default_factory=list)
    error: str | None = None

    @property
    def key_count(self) -> int:
        return len(self.keys)

    @property
    def total_balance_sat(self) -> int:
        return sum(k.balance_sat or 0 for k in self.keys)
