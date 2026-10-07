#!/usr/bin/env python3
"""Reject generated Python bytecode in a release wheel."""

from __future__ import annotations

import argparse
from pathlib import Path
from zipfile import ZipFile


def validate_archive(path: Path) -> int:
    """Return member count, or reject stale Python bytecode in the wheel."""
    with ZipFile(path) as archive:
        names = archive.namelist()
    stale = sorted(
        name
        for name in names
        if "__pycache__" in Path(name).parts or name.endswith((".pyc", ".pyo"))
    )
    if stale:
        raise ValueError(f"stale Python bytecode in wheel: {stale[0]}")
    return len(names)


def main() -> None:
    """Check one built wheel from the command line."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("wheel", type=Path)
    args = parser.parse_args()
    try:
        count = validate_archive(args.wheel)
    except ValueError as error:
        parser.error(str(error))
    print(f"wheel bytecode hygiene: ok ({count} members)")


if __name__ == "__main__":
    main()
