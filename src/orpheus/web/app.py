"""Flask web UI for Orpheus.

All operations happen server-side in the same Python process — uploaded files
are held in memory only (never written to disk), decoded, returned, discarded.
Designed to run behind ``orpheus web`` bound to 127.0.0.1 by default.
"""

from __future__ import annotations

import json
import tempfile
from dataclasses import asdict
from pathlib import Path

from flask import Flask, jsonify, render_template, request

from orpheus import __version__
from orpheus.balance import get_provider
from orpheus.extractors.bip39 import derive_bip39
from orpheus.extractors.blockchain_com import (
    BlockchainComMnemonicError,
    decode_mnemonic,
    load_wordlist,
)
from orpheus.models import WalletScanResult
from orpheus.scanner import scan_path


def create_app() -> Flask:
    app = Flask(__name__)
    app.config["MAX_CONTENT_LENGTH"] = 64 * 1024 * 1024  # 64 MiB per upload

    @app.get("/")
    def index() -> str:
        return render_template("index.html", version=__version__)

    @app.post("/api/scan")
    def api_scan():
        files = request.files.getlist("wallet")
        if not files:
            return jsonify(error="no files uploaded"), 400
        passwords = _parse_passwords(request.form.get("passwords", ""))
        provider_name = request.form.get("provider", "mock")
        mock_file = request.form.get("mock_file", "").strip()
        provider = get_provider(
            provider_name,
            mock_path=Path(mock_file) if mock_file else None,
        )

        results: list[WalletScanResult] = []
        with tempfile.TemporaryDirectory(prefix="orpheus_") as tmp:
            tmp_path = Path(tmp)
            for upload in files:
                if not upload.filename:
                    continue
                dest = tmp_path / Path(upload.filename).name
                upload.save(dest)
            results = scan_path(tmp_path, passwords=passwords, provider=provider)
        return jsonify(results=_serialize(results))

    @app.post("/api/mnemonic")
    def api_mnemonic():
        data = request.get_json(force=True) or {}
        phrase = (data.get("phrase") or "").strip()
        mtype = data.get("type", "bip39")
        passphrase = data.get("passphrase", "") or ""
        gap_limit = int(data.get("gap_limit", 20))

        if not phrase:
            return jsonify(error="phrase is required"), 400

        if mtype == "bip39":
            try:
                keys = derive_bip39(
                    phrase, passphrase=passphrase, gap_limit=gap_limit
                )
            except ValueError as exc:
                return jsonify(error=str(exc)), 400
            return jsonify(keys=[_key_dict(k) for k in keys])

        # blockchain.com legacy
        wordlist_path = data.get("wordlist", "")
        if not wordlist_path:
            return jsonify(
                error="blockchain.com mnemonics require a wordlist path",
            ), 400
        try:
            decoded = decode_mnemonic(phrase, load_wordlist(Path(wordlist_path)))
        except (BlockchainComMnemonicError, FileNotFoundError) as exc:
            return jsonify(error=str(exc)), 400
        return jsonify(
            decoded=dict(
                password=decoded.password,
                word_count=decoded.word_count,
                version=decoded.version_guess,
            ),
        )

    @app.post("/api/demo")
    def api_demo():
        """Run the bundled demo scan (fully offline, mock balances)."""
        demo_dir = Path(__file__).parent.parent / "data" / "demo-wallets"
        mock_file = Path(__file__).parent.parent / "data" / "mock_balances.json"
        provider = get_provider("mock", mock_path=mock_file)
        results = scan_path(demo_dir, provider=provider, passwords=["orpheus-demo"])
        return jsonify(results=_serialize(results))

    return app


def _parse_passwords(raw: str) -> list[str] | None:
    if not raw.strip():
        return None
    return [line.strip() for line in raw.splitlines() if line.strip()]


def _key_dict(k) -> dict:
    d = asdict(k)
    d["source_type"] = k.source_type.value
    return d


def _serialize(results: list[WalletScanResult]) -> list[dict]:
    out = []
    for r in results:
        out.append(
            dict(
                source_file=r.source_file,
                source_type=r.source_type.value,
                error=r.error,
                key_count=r.key_count,
                total_balance_sat=r.total_balance_sat,
                keys=[_key_dict(k) for k in r.keys],
            )
        )
    return out
