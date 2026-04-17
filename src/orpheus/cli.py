"""Orpheus CLI."""

from __future__ import annotations

import json
import sys
from dataclasses import asdict
from pathlib import Path

import click
from rich.console import Console
from rich.table import Table

from orpheus import __version__
from orpheus.balance import get_provider
from orpheus.extractors.bip39 import derive_bip39
from orpheus.extractors.blockchain_com import (
    BlockchainComMnemonicError,
    decode_mnemonic,
    load_wordlist,
)
from orpheus.models import ExtractedKey, WalletScanResult
from orpheus.scanner import scan_path

console = Console()
err_console = Console(stderr=True)


@click.group(invoke_without_command=True)
@click.version_option(__version__)
@click.pass_context
def main(ctx: click.Context) -> None:
    """Orpheus — recover lost cryptocurrency from forgotten wallets."""
    if ctx.invoked_subcommand is None:
        click.echo(ctx.get_help())


@main.command()
@click.argument("path", type=click.Path(exists=True, path_type=Path))
@click.option("--passwords", type=click.Path(exists=True, path_type=Path))
@click.option(
    "--provider",
    type=click.Choice(["blockchain", "blockstream", "mock", "none"]),
    default="none",
    help="Balance lookup provider (default: none)",
)
@click.option(
    "--mock-file",
    type=click.Path(path_type=Path),
    help="JSON file for mock balance data",
)
@click.option(
    "--output",
    "output_format",
    type=click.Choice(["table", "json", "csv"]),
    default="table",
)
def scan(
    path: Path,
    passwords: Path | None,
    provider: str,
    mock_file: Path | None,
    output_format: str,
) -> None:
    """Scan a file or directory for wallets and extract keys."""
    pw_list = _load_passwords(passwords)
    prov = get_provider(provider, mock_path=mock_file) if provider != "none" else None
    results = scan_path(path, passwords=pw_list, provider=prov)
    _render(results, output_format)


@main.command()
@click.argument("wallet", type=click.Path(exists=True, path_type=Path))
@click.option("--passwords", type=click.Path(exists=True, path_type=Path))
def extract(wallet: Path, passwords: Path | None) -> None:
    """Extract keys from a single wallet file."""
    pw_list = _load_passwords(passwords)
    results = scan_path(wallet, passwords=pw_list)
    _render(results, "table")


@main.command()
@click.argument("phrase")
@click.option(
    "--type",
    "mtype",
    type=click.Choice(["bip39", "blockchain"]),
    default="bip39",
)
@click.option("--passphrase", default="", help="BIP39 optional passphrase")
@click.option("--gap-limit", default=20, type=int)
@click.option("--wordlist", type=click.Path(exists=True, path_type=Path))
def mnemonic(
    phrase: str,
    mtype: str,
    passphrase: str,
    gap_limit: int,
    wordlist: Path | None,
) -> None:
    """Derive keys from a mnemonic phrase."""
    if mtype == "bip39":
        keys = derive_bip39(phrase, passphrase=passphrase, gap_limit=gap_limit)
        _render_keys(keys)
        return

    if wordlist is None:
        err_console.print(
            "[red]blockchain.com mnemonics require --wordlist pointing at the "
            "appropriate word list file (V2: 1626 words, V3: 65591 words).[/red]"
        )
        sys.exit(2)
    try:
        decoded = decode_mnemonic(phrase, load_wordlist(wordlist))
    except BlockchainComMnemonicError as exc:
        err_console.print(f"[red]decode failed:[/red] {exc}")
        sys.exit(1)
    console.print(
        f"[green]decoded[/green] {decoded.version_guess} ({decoded.word_count} words)"
    )
    console.print(f"password: [yellow]{decoded.password}[/yellow]")


@main.command()
@click.option(
    "--mock-file",
    type=click.Path(path_type=Path),
    help="JSON file for mock balance data (defaults to bundled demo balances)",
)
def demo(mock_file: Path | None) -> None:
    """Run Orpheus against the bundled demo wallet fixtures (fully offline)."""
    demo_dir = Path(__file__).parent / "data" / "demo-wallets"
    if not demo_dir.exists():
        err_console.print(
            "[red]demo fixtures missing —[/red] run "
            "[bold]python scripts/build_fixtures.py[/bold] first."
        )
        sys.exit(1)
    provider = get_provider(
        "mock",
        mock_path=mock_file or Path(__file__).parent / "data" / "mock_balances.json",
    )
    results = scan_path(demo_dir, provider=provider)
    _render(results, "table")


@main.command()
@click.option("--host", default="127.0.0.1")
@click.option("--port", default=5000, type=int)
@click.option("--debug/--no-debug", default=False)
def web(host: str, port: int, debug: bool) -> None:
    """Run the Orpheus web UI."""
    from orpheus.web.app import create_app

    if host != "127.0.0.1":
        err_console.print(
            f"[bold red]WARNING:[/bold red] binding to {host} exposes Orpheus beyond "
            "localhost. Only do this inside a trusted/air-gapped network."
        )
    app = create_app()
    app.run(host=host, port=port, debug=debug)


def _load_passwords(path: Path | None) -> list[str] | None:
    if path is None:
        return None
    return [line.strip() for line in path.read_text().splitlines() if line.strip()]


def _render(results: list[WalletScanResult], fmt: str) -> None:
    if fmt == "json":
        click.echo(
            json.dumps(
                [
                    {
                        "source_file": r.source_file,
                        "source_type": r.source_type.value,
                        "error": r.error,
                        "keys": [asdict(k) for k in r.keys],
                    }
                    for r in results
                ],
                default=_json_default,
                indent=2,
            )
        )
        return
    if fmt == "csv":
        click.echo("source_file,source_type,path,address,wif,balance_sat")
        for r in results:
            for k in r.keys:
                click.echo(
                    f"{r.source_file},{r.source_type.value},"
                    f"{k.derivation_path or ''},{k.address_compressed},"
                    f"{k.wif},{k.balance_sat or 0}"
                )
        return
    _render_table(results)


def _render_table(results: list[WalletScanResult]) -> None:
    total_keys = sum(r.key_count for r in results)
    total_bal = sum(r.total_balance_sat for r in results)
    console.print(
        f"[bold]Orpheus scan:[/bold] {len(results)} wallet(s), "
        f"{total_keys} key(s), total balance {total_bal / 1e8:.8f} BTC"
    )
    for r in results:
        console.print(
            f"\n[cyan]◆ {r.source_file}[/cyan]  "
            f"[dim]{r.source_type.value}[/dim]"
        )
        if r.error:
            console.print(f"  [red]error:[/red] {r.error}")
            continue
        _render_keys(r.keys)


def _render_keys(keys: list[ExtractedKey]) -> None:
    if not keys:
        console.print("  [dim]no keys found[/dim]")
        return
    table = Table(show_header=True, header_style="bold magenta", box=None, pad_edge=False)
    table.add_column("path", style="dim", no_wrap=True)
    table.add_column("address")
    table.add_column("BTC", justify="right")
    table.add_column("txs", justify="right")
    table.add_column("WIF (truncated)", style="dim")
    nonzero_only = any(k.balance_sat for k in keys) or any(k.tx_count for k in keys)
    for k in keys:
        if nonzero_only and not (k.balance_sat or k.tx_count):
            continue
        table.add_row(
            k.derivation_path or "-",
            k.address_compressed,
            f"{(k.balance_sat or 0) / 1e8:.8f}",
            str(k.tx_count or 0),
            k.wif[:8] + "…" + k.wif[-4:] if k.wif else "-",
        )
    if table.row_count == 0:
        # Show first few anyway so user sees something
        for k in keys[:5]:
            table.add_row(
                k.derivation_path or "-",
                k.address_compressed,
                "0.00000000",
                "0",
                k.wif[:8] + "…" + k.wif[-4:] if k.wif else "-",
            )
    console.print(table)


def _json_default(obj):  # type: ignore[no-untyped-def]
    if hasattr(obj, "value"):
        return obj.value
    raise TypeError(f"not serializable: {type(obj).__name__}")
