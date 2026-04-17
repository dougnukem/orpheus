import orpheus


def test_version_exposed() -> None:
    assert isinstance(orpheus.__version__, str)
    assert orpheus.__version__.count(".") >= 1


def test_cli_version_command() -> None:
    from click.testing import CliRunner

    from orpheus.cli import main

    result = CliRunner().invoke(main, ["version"])
    assert result.exit_code == 0
    assert orpheus.__version__ in result.output
