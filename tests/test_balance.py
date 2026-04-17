import json
from pathlib import Path

import requests_mock

from orpheus.balance import (
    BlockchainInfoProvider,
    BlockstreamProvider,
    MockProvider,
    attach_balances,
    get_provider,
)
from orpheus.models import ExtractedKey, SourceType


def test_mock_provider_returns_file_data(tmp_path: Path) -> None:
    path = tmp_path / "mock.json"
    path.write_text(
        json.dumps(
            {
                "1abc": {"balance_sat": 1000, "total_received_sat": 5000, "tx_count": 3},
            }
        )
    )
    provider = MockProvider(path=path)
    result = provider.fetch(["1abc", "1missing"])
    assert result["1abc"].balance_sat == 1000
    assert result["1abc"].total_received_sat == 5000
    assert result["1missing"].balance_sat == 0


def test_attach_balances_mutates_keys(tmp_path: Path) -> None:
    path = tmp_path / "mock.json"
    path.write_text(json.dumps({"addr1": {"balance_sat": 42}}))
    provider = MockProvider(path=path)
    key = ExtractedKey(
        wif="w",
        address_compressed="addr1",
        source_file="f",
        source_type=SourceType.BIP39,
    )
    attach_balances([key], provider)
    assert key.balance_sat == 42


def test_blockchain_info_batches() -> None:
    with requests_mock.Mocker() as m:
        m.get(
            "https://blockchain.info/balance",
            json={
                "1a": {"final_balance": 100, "total_received": 200, "n_tx": 1},
                "1b": {"final_balance": 0, "total_received": 0, "n_tx": 0},
            },
        )
        result = BlockchainInfoProvider().fetch(["1a", "1b"])
        assert result["1a"].balance_sat == 100


def test_blockstream_per_address_calls() -> None:
    with requests_mock.Mocker() as m:
        m.get(
            "https://blockstream.info/api/address/1z",
            json={
                "chain_stats": {"funded_txo_sum": 500, "spent_txo_sum": 200, "tx_count": 2},
                "mempool_stats": {"funded_txo_sum": 0, "spent_txo_sum": 0, "tx_count": 0},
            },
        )
        result = BlockstreamProvider().fetch(["1z"])
        assert result["1z"].balance_sat == 300
        assert result["1z"].total_received_sat == 500


def test_get_provider_none() -> None:
    assert get_provider("none") is None


def test_get_provider_unknown_raises() -> None:
    import pytest

    with pytest.raises(ValueError):
        get_provider("neverheardofit")
