from dataclasses import asdict

from orpheus.models import (
    BalanceInfo,
    ExtractedKey,
    SourceType,
    WalletScanResult,
)


def test_extracted_key_defaults() -> None:
    key = ExtractedKey(
        wif="L1aW4aubDFB7yfras2S1mN3bqg9aXvFfz4hzc4Rc4zk5xS9kFGCm",
        address_compressed="1Kb6dFQgV5DwW8X7eQhk5JQxWjz2bJ38rt",
        source_file="fake.wallet",
        source_type=SourceType.MULTIBIT,
    )
    assert key.address_uncompressed is None
    assert key.address_p2sh_segwit is None
    assert key.address_bech32 is None
    assert key.balance_sat is None
    assert key.derivation_path is None
    assert asdict(key)["source_type"] == SourceType.MULTIBIT


def test_wallet_scan_result_aggregates_keys() -> None:
    k1 = ExtractedKey(
        wif="w1",
        address_compressed="a1",
        source_file="f",
        source_type=SourceType.BITCOIN_CORE,
    )
    result = WalletScanResult(source_file="f", source_type=SourceType.BITCOIN_CORE, keys=[k1])
    assert result.key_count == 1
    assert result.total_balance_sat == 0


def test_wallet_scan_result_balance_sum() -> None:
    k1 = ExtractedKey(
        wif="w1",
        address_compressed="a1",
        source_file="f",
        source_type=SourceType.BIP39,
        balance_sat=100,
    )
    k2 = ExtractedKey(
        wif="w2",
        address_compressed="a2",
        source_file="f",
        source_type=SourceType.BIP39,
        balance_sat=250,
    )
    result = WalletScanResult(source_file="f", source_type=SourceType.BIP39, keys=[k1, k2])
    assert result.total_balance_sat == 350


def test_balance_info_roundtrip() -> None:
    b = BalanceInfo(address="a", balance_sat=1, total_received_sat=2, tx_count=3)
    assert asdict(b) == {
        "address": "a",
        "balance_sat": 1,
        "total_received_sat": 2,
        "tx_count": 3,
    }


def test_source_type_values_stable() -> None:
    # Downstream serializers rely on these string values
    assert SourceType.BITCOIN_CORE.value == "bitcoin_core"
    assert SourceType.MULTIBIT.value == "multibit"
    assert SourceType.BIP39.value == "bip39"
    assert SourceType.BLOCKCHAIN_COM.value == "blockchain_com"
    assert SourceType.WALLET_DUMP.value == "wallet_dump"
    assert SourceType.ENCRYPTED.value == "encrypted"
