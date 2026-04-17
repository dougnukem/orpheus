from pathlib import Path

import pytest


@pytest.fixture(scope="session")
def fixtures_dir() -> Path:
    return Path(__file__).parent / "fixtures"


@pytest.fixture(scope="session")
def demo_wallets_dir(fixtures_dir: Path) -> Path:
    return fixtures_dir / "demo-wallets"
