"""Blockchain.com legacy mnemonic decoder.

Blockchain.com (nee blockchain.info) used a custom mnemonic scheme that
pre-dates BIP39 and is NOT BIP39-compatible. Five versions exist:

    V2: 1626-word list encodes a password string
    V3: 65591-word list encodes a password string (larger list, same idea)
    V4: encodes (GUID, password)
    V5: encodes (timestamp, password)

The decoder requires the word list file. We ship the V2 list (1626 words, ~15KB)
alongside the code; V3/V4/V5 can be enabled by pointing ``--wordlist`` at the
relevant list from ``blockchain/My-Wallet-V3/src/wordlist/``.

Decoding produces a *password*, not a key — you'll feed that password into
whatever encrypted wallet it unlocks (often `wallet.aes.json`).
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

from orpheus.extractors.base import Extractor
from orpheus.models import ExtractedKey, SourceType

WORDLIST_DIR = Path(__file__).parent.parent / "data" / "wordlists"


@dataclass
class DecodedMnemonic:
    password: str
    word_count: int
    version_guess: str


class BlockchainComMnemonicError(ValueError):
    pass


def decode_mnemonic(phrase: str, wordlist: list[str]) -> DecodedMnemonic:
    """Decode a blockchain.com legacy mnemonic into its encoded payload.

    Algorithm (from My-Wallet-V3 ``encodePhrase``):
      - split phrase into words
      - every group of 3 words encodes 4 bytes:
            x = idx0 + n*((idx1 - idx0) mod n) + n*n*((idx2 - idx1) mod n)
            emit 4 bytes big-endian
      - the reassembled bytes are the UTF-8 password string
    """
    words = phrase.strip().lower().split()
    if len(words) % 3 != 0:
        raise BlockchainComMnemonicError(
            f"word count must be a multiple of 3, got {len(words)}"
        )
    lookup = {w: i for i, w in enumerate(wordlist)}
    missing = [w for w in words if w not in lookup]
    if missing:
        raise BlockchainComMnemonicError(
            f"{len(missing)} words not in wordlist (e.g. {missing[:3]})"
        )
    n = len(wordlist)
    out = bytearray()
    for i in range(0, len(words), 3):
        a, b, c = (lookup[words[i + k]] for k in range(3))
        x = a + n * ((b - a) % n) + n * n * ((c - b) % n)
        out.extend(x.to_bytes(4, "big"))
    payload = bytes(out).rstrip(b"\x00")
    try:
        password = payload.decode("utf-8")
    except UnicodeDecodeError as exc:
        raise BlockchainComMnemonicError(
            f"decoded payload is not valid UTF-8 ({exc}) — likely wrong word list"
        ) from exc
    return DecodedMnemonic(
        password=password,
        word_count=len(words),
        version_guess="V2" if n == 1626 else f"V3+ ({n}-word list)",
    )


def load_wordlist(path: Path) -> list[str]:
    return [line.strip() for line in path.read_text().splitlines() if line.strip()]


class BlockchainComMnemonicExtractor(Extractor):
    """Extractor variant for .txt files containing legacy blockchain.com phrases."""

    source_type = SourceType.BLOCKCHAIN_COM

    def __init__(self, wordlist_path: Path | None = None) -> None:
        self.wordlist_path = wordlist_path or (WORDLIST_DIR / "blockchain_com_v2.txt")

    def can_handle(self, path: Path) -> bool:  # opt-in via CLI flag; no auto-detect
        return False

    def extract(self, path: Path, passwords: list[str] | None = None) -> list[ExtractedKey]:
        if not self.wordlist_path.exists():
            raise BlockchainComMnemonicError(
                f"word list not found at {self.wordlist_path}"
            )
        wordlist = load_wordlist(self.wordlist_path)
        decoded = decode_mnemonic(path.read_text(), wordlist)
        # The decoded value is a password, not a key. Surface it via notes.
        return [
            ExtractedKey(
                wif="",
                address_compressed="",
                source_file=str(path),
                source_type=SourceType.BLOCKCHAIN_COM,
                notes=(
                    f"decoded blockchain.com mnemonic ({decoded.version_guess}, "
                    f"{decoded.word_count} words) -> password: {decoded.password!r}"
                ),
            )
        ]
