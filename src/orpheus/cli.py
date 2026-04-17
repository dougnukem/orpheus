"""CLI entry point — implemented in later phases."""

import click


@click.group()
@click.version_option()
def main() -> None:
    """Orpheus — recover lost cryptocurrency from forgotten wallets."""


@main.command()
def version() -> None:
    """Print the Orpheus version."""
    from orpheus import __version__

    click.echo(__version__)
