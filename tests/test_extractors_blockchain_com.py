"""Round-trip test for the blockchain.com legacy decoder.

We synthesise a tiny wordlist, encode a known payload with the same algorithm,
then decode and assert we get the payload back. This avoids shipping the real
1626/65591-word lists in the test suite while still proving the decoder math.
"""

from __future__ import annotations

from pathlib import Path

import pytest

from orpheus.extractors.blockchain_com import (
    BlockchainComMnemonicError,
    decode_mnemonic,
)


def _encode(payload: bytes, wordlist: list[str]) -> str:
    if len(payload) % 4 != 0:
        raise ValueError("payload must be multiple of 4 bytes")
    n = len(wordlist)
    words: list[str] = []
    for chunk_start in range(0, len(payload), 4):
        x = int.from_bytes(payload[chunk_start:chunk_start + 4], "big")
        a = x % n
        b = (a + (x // n) % n) % n
        c = (b + (x // (n * n)) % n) % n
        words.extend([wordlist[a], wordlist[b], wordlist[c]])
    return " ".join(words)


def make_wordlist(size: int) -> list[str]:
    # Use just enough unique words for the round-trip
    return [f"w{i:05d}" for i in range(size)]


def test_roundtrip_padded_payload() -> None:
    wordlist = make_wordlist(1626)
    payload = b"Monkey99"  # 8 bytes → 6 words
    phrase = _encode(payload, wordlist)
    decoded = decode_mnemonic(phrase, wordlist)
    assert decoded.password == "Monkey99"
    assert decoded.word_count == 6
    assert "1626" in decoded.version_guess or decoded.version_guess == "V2"


def test_rejects_bad_word_count() -> None:
    wordlist = make_wordlist(1626)
    with pytest.raises(BlockchainComMnemonicError):
        decode_mnemonic("w00000 w00001", wordlist)


def test_rejects_unknown_word() -> None:
    wordlist = make_wordlist(1626)
    with pytest.raises(BlockchainComMnemonicError):
        decode_mnemonic("w00000 w00001 notaword", wordlist)
