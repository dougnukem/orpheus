"""Bech32 / Bech32m encoding for segwit v0/v1 addresses.

Reference: BIP173 (bech32) and BIP350 (bech32m).
Implementation translated from the BIP173 reference appendix.
"""

from __future__ import annotations

from enum import Enum

CHARSET = "qpzry9x8gf2tvdw0s3jn54khce6mua7l"


class Encoding(Enum):
    BECH32 = 1
    BECH32M = 2


BECH32M_CONST = 0x2BC830A3


def _polymod(values: list[int]) -> int:
    gen = [0x3B6A57B2, 0x26508E6D, 0x1EA119FA, 0x3D4233DD, 0x2A1462B3]
    chk = 1
    for v in values:
        b = chk >> 25
        chk = ((chk & 0x1FFFFFF) << 5) ^ v
        for i in range(5):
            if (b >> i) & 1:
                chk ^= gen[i]
    return chk


def _hrp_expand(hrp: str) -> list[int]:
    return [ord(x) >> 5 for x in hrp] + [0] + [ord(x) & 31 for x in hrp]


def _create_checksum(hrp: str, data: list[int], enc: Encoding) -> list[int]:
    const = 1 if enc == Encoding.BECH32 else BECH32M_CONST
    values = _hrp_expand(hrp) + data
    polymod = _polymod(values + [0, 0, 0, 0, 0, 0]) ^ const
    return [(polymod >> 5 * (5 - i)) & 31 for i in range(6)]


def _verify_checksum(hrp: str, data: list[int]) -> Encoding | None:
    p = _polymod(_hrp_expand(hrp) + data)
    if p == 1:
        return Encoding.BECH32
    if p == BECH32M_CONST:
        return Encoding.BECH32M
    return None


def _convertbits(data: list[int], frombits: int, tobits: int, pad: bool = True) -> list[int]:
    acc = 0
    bits = 0
    ret: list[int] = []
    maxv = (1 << tobits) - 1
    max_acc = (1 << (frombits + tobits - 1)) - 1
    for value in data:
        if value < 0 or (value >> frombits):
            raise ValueError("invalid data for base conversion")
        acc = ((acc << frombits) | value) & max_acc
        bits += frombits
        while bits >= tobits:
            bits -= tobits
            ret.append((acc >> bits) & maxv)
    if pad:
        if bits:
            ret.append((acc << (tobits - bits)) & maxv)
    elif bits >= frombits or ((acc << (tobits - bits)) & maxv):
        raise ValueError("invalid padding in base conversion")
    return ret


def encode_segwit_address(hrp: str, witver: int, witprog: bytes) -> str:
    enc = Encoding.BECH32 if witver == 0 else Encoding.BECH32M
    data = [witver] + _convertbits(list(witprog), 8, 5)
    combined = data + _create_checksum(hrp, data, enc)
    return hrp + "1" + "".join(CHARSET[d] for d in combined)


def decode_segwit_address(hrp: str, addr: str) -> tuple[int, bytes]:
    if not (addr.lower() == addr or addr.upper() == addr):
        raise ValueError("mixed case")
    addr = addr.lower()
    if not addr.startswith(hrp + "1"):
        raise ValueError("wrong hrp")
    data_part = addr[len(hrp) + 1:]
    if any(c not in CHARSET for c in data_part):
        raise ValueError("invalid character")
    data = [CHARSET.index(c) for c in data_part]
    if len(data) < 6:
        raise ValueError("too short")
    enc = _verify_checksum(hrp, data)
    if enc is None:
        raise ValueError("bad checksum")
    witver = data[0]
    witprog = bytes(_convertbits(data[1:-6], 5, 8, pad=False))
    if witver == 0 and enc != Encoding.BECH32:
        raise ValueError("v0 requires bech32")
    if witver > 0 and enc != Encoding.BECH32M:
        raise ValueError("v>=1 requires bech32m")
    return witver, witprog


def p2wpkh_address(pubkey: bytes, hrp: str = "bc") -> str:
    from orpheus.crypto.keys import hash160

    return encode_segwit_address(hrp, 0, hash160(pubkey))
