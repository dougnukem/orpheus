"""Balance lookup across blockchain.info / blockstream.info / mock.

All providers return `{address: BalanceInfo}`. Network providers batch up to
``MAX_BATCH`` addresses per request; unknown addresses return zero balances.
"""

from __future__ import annotations

import json
import os
from abc import ABC, abstractmethod
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable

import requests

from orpheus.models import BalanceInfo

MAX_BATCH = 20


class BalanceProvider(ABC):
    name: str

    @abstractmethod
    def fetch(self, addresses: Iterable[str]) -> dict[str, BalanceInfo]: ...

    def enrich(self, balances: dict[str, BalanceInfo], addresses: Iterable[str]) -> None:
        for addr in addresses:
            balances.setdefault(addr, BalanceInfo(addr, 0, 0, 0))


@dataclass
class BlockchainInfoProvider(BalanceProvider):
    name: str = "blockchain.info"
    base_url: str = "https://blockchain.info"
    timeout: float = 30.0

    def fetch(self, addresses: Iterable[str]) -> dict[str, BalanceInfo]:
        addr_list = list(addresses)
        out: dict[str, BalanceInfo] = {}
        for i in range(0, len(addr_list), MAX_BATCH):
            batch = addr_list[i:i + MAX_BATCH]
            params = {"active": "|".join(batch)}
            resp = requests.get(f"{self.base_url}/balance", params=params, timeout=self.timeout)
            resp.raise_for_status()
            for addr, info in resp.json().items():
                out[addr] = BalanceInfo(
                    address=addr,
                    balance_sat=info.get("final_balance", 0),
                    total_received_sat=info.get("total_received", 0),
                    tx_count=info.get("n_tx", 0),
                )
        self.enrich(out, addr_list)
        return out


@dataclass
class BlockstreamProvider(BalanceProvider):
    name: str = "blockstream.info"
    base_url: str = "https://blockstream.info/api"
    timeout: float = 30.0

    def fetch(self, addresses: Iterable[str]) -> dict[str, BalanceInfo]:
        addr_list = list(addresses)
        out: dict[str, BalanceInfo] = {}
        for addr in addr_list:
            try:
                resp = requests.get(
                    f"{self.base_url}/address/{addr}", timeout=self.timeout
                )
                resp.raise_for_status()
                j = resp.json()
                chain = j.get("chain_stats", {})
                mem = j.get("mempool_stats", {})
                funded = chain.get("funded_txo_sum", 0) + mem.get("funded_txo_sum", 0)
                spent = chain.get("spent_txo_sum", 0) + mem.get("spent_txo_sum", 0)
                out[addr] = BalanceInfo(
                    address=addr,
                    balance_sat=funded - spent,
                    total_received_sat=funded,
                    tx_count=chain.get("tx_count", 0) + mem.get("tx_count", 0),
                )
            except requests.RequestException:
                out[addr] = BalanceInfo(addr, 0, 0, 0)
        self.enrich(out, addr_list)
        return out


@dataclass
class MockProvider(BalanceProvider):
    """Offline provider driven by a JSON file keyed on address."""

    name: str = "mock"
    path: Path | None = None

    def fetch(self, addresses: Iterable[str]) -> dict[str, BalanceInfo]:
        data: dict[str, dict] = {}
        if self.path and self.path.exists():
            data = json.loads(self.path.read_text())
        out: dict[str, BalanceInfo] = {}
        for addr in addresses:
            entry = data.get(addr)
            if entry is None:
                out[addr] = BalanceInfo(addr, 0, 0, 0)
            else:
                out[addr] = BalanceInfo(
                    address=addr,
                    balance_sat=int(entry.get("balance_sat", 0)),
                    total_received_sat=int(entry.get("total_received_sat", 0)),
                    tx_count=int(entry.get("tx_count", 0)),
                )
        return out


def get_provider(name: str, mock_path: Path | None = None) -> BalanceProvider | None:
    name = name.lower()
    if name == "none":
        return None
    if name in ("blockchain", "blockchain.info"):
        return BlockchainInfoProvider()
    if name in ("blockstream", "blockstream.info"):
        return BlockstreamProvider()
    if name == "mock":
        default = Path(__file__).parent / "data" / "mock_balances.json"
        return MockProvider(path=mock_path or default)
    raise ValueError(f"unknown balance provider: {name}")


def attach_balances(
    keys: list,  # list[ExtractedKey] — avoid circular import at module load
    provider: BalanceProvider,
    *,
    use_bech32: bool = False,
) -> None:
    """Mutate each key in-place with balance_sat / total_received_sat / tx_count."""
    addresses: list[str] = []
    index: dict[str, list] = {}
    for key in keys:
        addr = key.address_bech32 if use_bech32 and key.address_bech32 else key.address_compressed
        if not addr:
            continue
        addresses.append(addr)
        index.setdefault(addr, []).append(key)
    if not addresses:
        return
    balances = provider.fetch(sorted(set(addresses)))
    for addr, info in balances.items():
        for k in index.get(addr, []):
            k.balance_sat = info.balance_sat
            k.total_received_sat = info.total_received_sat
            k.tx_count = info.tx_count
