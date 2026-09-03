#!/usr/bin/env python3
"""Validate that every NEED identifier used by traceability resolves."""

from __future__ import annotations

import argparse
import pathlib
import re
import sys


VALID_NEED = re.compile(r"^NEED-[0-9]{3}[a-z]?$")
TOKEN = re.compile(r"NEED-[A-Za-z0-9_-]+")
DEFINITION = re.compile(r"^(NEED-[0-9]{3}[a-z]?):", re.MULTILINE)


def validate_need_references(root: pathlib.Path) -> list[str]:
    needs = (root / "dev/needs.md").read_text(encoding="utf-8")
    traceability = (root / "dev/traceability.md").read_text(encoding="utf-8")
    defined = set(DEFINITION.findall(needs))
    failures: list[str] = []
    for token in sorted(set(TOKEN.findall(traceability))):
        if not VALID_NEED.fullmatch(token):
            failures.append(f"malformed need reference {token!r}")
        elif token not in defined:
            failures.append(f"unresolved need reference {token}")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=pathlib.Path, default=pathlib.Path.cwd())
    args = parser.parse_args()
    failures = validate_need_references(args.root.resolve())
    for failure in failures:
        print(f"FAIL traceability-contracts: {failure}", file=sys.stderr)
    if failures:
        return 1
    print("ok    traceability-contracts")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
