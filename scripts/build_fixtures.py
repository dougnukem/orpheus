#!/usr/bin/env python3
"""Synthesize demo wallet fixtures and matching mock-balance JSON.

Outputs:
  src/orpheus/data/demo-wallets/
    ├── demo_bitcoin_core.wallet.dat  (fake BDB blob w/ DER keys)
    ├── demo_multibit.wallet          (unencrypted MultiBit protobuf-ish blob)
    ├── demo_multibit_protected.wallet (encrypted; password "orpheus-demo")
    ├── demo_descriptor_dump.json     (Bitcoin Core descriptor JSON dump)
    └── demo_seed.txt                 (BIP39 mnemonic — NOT a real seed)
  src/orpheus/data/mock_balances.json

All keys are derived from a throwaway BIP39 phrase hard-coded in this script.
They have zero real-world value — the corresponding addresses are never funded.
Mock balances are synthetic (fake BTC amounts) so the UI has something to show.
"""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path

from cryptography.hazmat.primitives.ciphers import Cipher, algorithms, modes

from orpheus.crypto.addresses import all_addresses_for_privkey
from orpheus.crypto.bip32 import derive_path, master_from_seed, mnemonic_to_seed
from orpheus.crypto.keys import privkey_to_wif

DEMO_MNEMONIC = (
    "legal winner thank year wave sausage worth useful legal winner thank yellow"
)
# ^ well-known BIP39 test mnemonic — *not* a funded seed.

DATA_DIR = Path(__file__).resolve().parent.parent / "src" / "orpheus" / "data"
DEMO_DIR = DATA_DIR / "demo-wallets"
MOCK_BALANCES = DATA_DIR / "mock_balances.json"

DER_PATTERN = b"\x30\x81\xd3\x02\x01\x01\x04\x20"
MULTIBIT_TAG = b"\x12\x20"
IV_TAG = b"\x0a\x10"
DATA_TAG = b"\x12\x30"


def derive_demo_keys(count: int = 5) -> list:
    seed = mnemonic_to_seed(DEMO_MNEMONIC)
    master = master_from_seed(seed)
    keys = []
    for i in range(count):
        child = derive_path(master, f"m/44'/0'/0'/0/{i}")
        keys.append(child.privkey)
    return keys


def build_bdb(privkeys: list[bytes]) -> bytes:
    blob = b"\x62\x31\x05\x00\x00\x00\x00\x00" + b"\x00" * 120 + b"main\x00"
    for priv in privkeys:
        blob += b"\x00" * 16 + DER_PATTERN + priv + b"\x00" * 32
    return blob


def build_multibit_unencrypted(privkeys: list[bytes]) -> bytes:
    blob = b"org.bitcoin.production" + b"\x00" * 8
    for p in privkeys:
        blob += b"\x0a\x21\x02" + b"\x00" * 32  # fake pubkey entry
        blob += MULTIBIT_TAG + p
    return blob


def build_multibit_encrypted(privkeys: list[bytes], password: str) -> bytes:
    salt = os.urandom(8)
    aes_key = hashlib.scrypt(
        password.encode("utf-8"), salt=salt, n=16384, r=8, p=1, dklen=32
    )
    blob = b"org.bitcoin.production" + b"\x00" * 4
    blob += b"\x0a\x08" + salt + b"\x10\x80\x80\x01"  # scrypt salt + N=16384 marker
    for priv in privkeys:
        iv = os.urandom(16)
        padded = priv + bytes([16]) * 16
        enc = Cipher(algorithms.AES(aes_key), modes.CBC(iv)).encryptor()
        ct = enc.update(padded) + enc.finalize()
        blob += IV_TAG + iv + DATA_TAG + ct
    return blob


def build_descriptor_dump(privkeys: list[bytes]) -> str:
    descriptors = []
    for i, priv in enumerate(privkeys):
        wif = privkey_to_wif(priv, compressed=True)
        descriptors.append(
            {
                "desc": f"wpkh({wif})#checksum",
                "timestamp": 1_700_000_000 + i,
            }
        )
    return json.dumps({"descriptors": descriptors}, indent=2)


def main() -> None:
    DEMO_DIR.mkdir(parents=True, exist_ok=True)
    keys = derive_demo_keys(5)

    (DEMO_DIR / "demo_bitcoin_core.wallet.dat").write_bytes(build_bdb(keys[:2]))
    (DEMO_DIR / "demo_multibit.wallet").write_bytes(build_multibit_unencrypted(keys[2:3]))
    (DEMO_DIR / "demo_multibit_protected.wallet").write_bytes(
        build_multibit_encrypted(keys[3:4], "orpheus-demo")
    )
    (DEMO_DIR / "demo_descriptor_dump.json").write_text(build_descriptor_dump(keys[4:]))
    (DEMO_DIR / "demo_seed.txt").write_text(DEMO_MNEMONIC + "\n")

    # Mock balances — invent plausible numbers for every address of every key
    balances: dict[str, dict[str, int]] = {}
    for i, priv in enumerate(keys):
        addrs = all_addresses_for_privkey(priv)
        for label, addr in addrs.items():
            if i == 0 and label == "p2pkh_compressed":
                balances[addr] = {
                    "balance_sat": 3_865_052,
                    "total_received_sat": 5_000_000,
                    "tx_count": 4,
                }
            elif i == 2 and label == "bech32":
                balances[addr] = {
                    "balance_sat": 1_000_000,
                    "total_received_sat": 1_000_000,
                    "tx_count": 1,
                }
            elif i == 3 and label == "p2pkh_compressed":
                balances[addr] = {
                    "balance_sat": 500_000,
                    "total_received_sat": 500_000,
                    "tx_count": 2,
                }
            else:
                balances[addr] = {
                    "balance_sat": 0,
                    "total_received_sat": 0,
                    "tx_count": 0,
                }

    MOCK_BALANCES.write_text(json.dumps(balances, indent=2))

    print(f"wrote {len(list(DEMO_DIR.iterdir()))} demo wallet fixtures to {DEMO_DIR}")
    print(f"wrote {len(balances)} mock balance entries to {MOCK_BALANCES}")


if __name__ == "__main__":
    main()
