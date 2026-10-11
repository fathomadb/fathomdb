"""Read the candidate wheel's Maturin feature set on Python 3.10 and newer."""

import ast
import importlib
from pathlib import Path


def _checked_features(value: object) -> tuple[str, ...]:
    if not isinstance(value, list) or not value or any(
        not isinstance(feature, str) or not feature for feature in value
    ):
        raise ValueError("tool.maturin.features must be a nonempty string list")
    return tuple(value)


def read_maturin_features(pyproject: Path) -> tuple[str, ...]:
    """Return declared features; a strict single-line fallback needs no TOML package."""
    for module_name in ("tomllib", "tomli"):
        try:
            parser = importlib.import_module(module_name)
        except ModuleNotFoundError:
            continue
        with pyproject.open("rb") as config_file:
            return _checked_features(parser.load(config_file)["tool"]["maturin"]["features"])

    in_maturin_section = False
    for line in pyproject.read_text(encoding="utf-8").splitlines():
        stripped = line.strip()
        if stripped.startswith("[") and stripped.endswith("]"):
            in_maturin_section = stripped == "[tool.maturin]"
        elif in_maturin_section:
            key, separator, literal = stripped.partition("=")
            if separator and key.strip() == "features":
                try:
                    return _checked_features(ast.literal_eval(literal.strip()))
                except (ValueError, SyntaxError) as error:
                    raise ValueError(
                        "tool.maturin.features needs tomli for multiline or complex TOML"
                    ) from error
    raise ValueError("tool.maturin.features is missing")
