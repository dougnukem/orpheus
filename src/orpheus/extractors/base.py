"""Extractor base class + registry."""

from __future__ import annotations

from abc import ABC, abstractmethod
from pathlib import Path

from orpheus.models import ExtractedKey, SourceType, WalletScanResult


class Extractor(ABC):
    source_type: SourceType = SourceType.UNKNOWN

    @abstractmethod
    def can_handle(self, path: Path) -> bool: ...

    @abstractmethod
    def extract(self, path: Path, passwords: list[str] | None = None) -> list[ExtractedKey]: ...

    def scan(self, path: Path, passwords: list[str] | None = None) -> WalletScanResult:
        try:
            keys = self.extract(path, passwords=passwords)
            return WalletScanResult(source_file=str(path), source_type=self.source_type, keys=keys)
        except Exception as exc:  # noqa: BLE001 — surface any extractor failure
            return WalletScanResult(
                source_file=str(path),
                source_type=self.source_type,
                error=f"{type(exc).__name__}: {exc}",
            )


_REGISTRY: list[type[Extractor]] = []


def register(cls: type[Extractor]) -> type[Extractor]:
    _REGISTRY.append(cls)
    return cls


def list_extractors() -> list[Extractor]:
    return [cls() for cls in _REGISTRY]


def find_extractor(path: Path) -> Extractor | None:
    for ex in list_extractors():
        if ex.can_handle(path):
            return ex
    return None
