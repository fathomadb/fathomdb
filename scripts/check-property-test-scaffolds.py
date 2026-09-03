#!/usr/bin/env python3
"""Reject the known no-op property-test scaffolds."""

from __future__ import annotations

import argparse
import pathlib
import sys


TARGETS = (
    "src/rust/crates/fathomdb-engine/tests/property_template.rs",
    "src/rust/crates/fathomdb-schema/tests/property_template.rs",
    "src/python/tests/test_property_template.py",
)
FORBIDDEN = ("placeholder_identity", "placeholder_round_trip", "assert x == x", "_x in any")


def find_trivial_properties(root: pathlib.Path) -> list[str]:
    failures: list[str] = []
    for relative in TARGETS:
        path = root / relative
        if not path.is_file():
            failures.append(f"missing property test: {relative}")
            continue
        text = path.read_text(encoding="utf-8")
        for token in FORBIDDEN:
            if token in text:
                failures.append(f"{relative}: trivial scaffold token {token!r}")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=pathlib.Path, default=pathlib.Path.cwd())
    args = parser.parse_args()
    failures = find_trivial_properties(args.root.resolve())
    for failure in failures:
        print(f"FAIL property-scaffolds: {failure}", file=sys.stderr)
    if failures:
        return 1
    print("ok    property-scaffolds")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
