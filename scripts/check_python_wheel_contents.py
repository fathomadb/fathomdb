#!/usr/bin/env python3
"""Validate release wheel contents for SDK, CLI, and MCP distributions."""

from __future__ import annotations

import argparse
from pathlib import Path
import re
from zipfile import ZipFile

_CLI_BINARY_RE = re.compile(r"^[^/]+\.data/scripts/fathomdb(?:\.exe)?$")
_MCP_BINARY_RE = re.compile(r"^[^/]+\.data/scripts/fathomdb-mcp(?:\.exe)?$")
_LICENSE_RE = re.compile(r"^[^/]+\.dist-info/licenses/LICENSE$")
_FORBIDDEN_CLI_BINARY_RE = re.compile(r"(?:^|/)perf-regression-check(?:\.exe)?$")
_FORBIDDEN_NATIVE_EXT_RE = re.compile(r"(?:^|/)_fathomdb[^/]*\.(?:so|pyd)$")


def validate_archive(path: Path, *, kind: str = "sdk") -> int:
    """Return member count, or reject stale bytecode / invalid CLI or MCP wheel contents."""
    if kind not in {"sdk", "cli", "mcp"}:
        raise ValueError(f"unsupported wheel kind: {kind!r}")
    with ZipFile(path) as archive:
        names = archive.namelist()
    stale = sorted(
        name
        for name in names
        if "__pycache__" in Path(name).parts or name.endswith((".pyc", ".pyo"))
    )
    if stale:
        raise ValueError(f"stale Python bytecode in wheel: {stale[0]}")

    if kind == "cli":
        cli_bins = [name for name in names if _CLI_BINARY_RE.match(name)]
        if not cli_bins:
            raise ValueError(
                "missing required CLI binary (.data/scripts/fathomdb or .data/scripts/fathomdb.exe)"
            )
        forbidden_bins = sorted(name for name in names if _FORBIDDEN_CLI_BINARY_RE.search(name))
        if forbidden_bins:
            raise ValueError(f"forbidden binary in CLI wheel: {forbidden_bins[0]}")
        forbidden_exts = sorted(name for name in names if _FORBIDDEN_NATIVE_EXT_RE.search(name))
        if forbidden_exts:
            raise ValueError(f"forbidden native Python extension in CLI wheel: {forbidden_exts[0]}")

    if kind == "mcp":
        mcp_bins = [name for name in names if _MCP_BINARY_RE.match(name)]
        if not mcp_bins:
            raise ValueError(
                "missing required MCP binary (.data/scripts/fathomdb-mcp or .data/scripts/fathomdb-mcp.exe)"
            )
        licenses = [name for name in names if _LICENSE_RE.match(name)]
        if not licenses:
            raise ValueError("missing required LICENSE (.dist-info/licenses/LICENSE)")
        forbidden_exts = sorted(name for name in names if _FORBIDDEN_NATIVE_EXT_RE.search(name))
        if forbidden_exts:
            raise ValueError(f"forbidden native Python extension in MCP wheel: {forbidden_exts[0]}")

    return len(names)


def main() -> None:
    """Check one or more built wheels from the command line."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--kind", choices=("sdk", "cli", "mcp"), default="sdk")
    parser.add_argument("wheels", nargs="+", type=Path)
    args = parser.parse_args()
    total = 0
    for wheel in args.wheels:
        try:
            count = validate_archive(wheel, kind=args.kind)
        except ValueError as error:
            parser.error(f"{wheel}: {error}")
        total += count
    print(f"wheel contents ({args.kind}): ok ({len(args.wheels)} wheel(s), {total} members)")


if __name__ == "__main__":
    main()
