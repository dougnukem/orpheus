"""Flask API smoke tests."""

import json

from orpheus.web.app import create_app


def test_index_renders():
    client = create_app().test_client()
    r = client.get("/")
    assert r.status_code == 200
    assert b"Orpheus" in r.data
    assert b"descent" in r.data  # navigation aesthetic lives


def test_api_mnemonic_bip39_returns_keys():
    client = create_app().test_client()
    r = client.post(
        "/api/mnemonic",
        json={"phrase": "abandon " * 11 + "about", "gap_limit": 2},
    )
    assert r.status_code == 200
    body = r.get_json()
    assert "keys" in body
    assert any(k["derivation_path"] == "m/44'/0'/0'/0/0" for k in body["keys"])


def test_api_mnemonic_rejects_empty_phrase():
    client = create_app().test_client()
    r = client.post("/api/mnemonic", json={"phrase": ""})
    assert r.status_code == 400


def test_api_demo_returns_results():
    client = create_app().test_client()
    r = client.post("/api/demo")
    assert r.status_code == 200
    body = r.get_json()
    assert body["results"]  # non-empty
    # Should include the homage wallet
    total = sum(w["total_balance_sat"] for w in body["results"])
    assert total > 0
