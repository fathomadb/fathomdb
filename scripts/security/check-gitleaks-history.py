#!/usr/bin/env python3
"""Compare a safe Gitleaks history report with the reviewed tuple multiset."""

from __future__ import annotations

import hashlib
import json
import re
import sys
from collections import Counter
from pathlib import Path


SCHEMA = 1
SCANNER_VERSION = "8.30.1"
TUPLE_ENCODING = "fathomdb-gitleaks-history-v1 NUL commit NUL rule NUL path NUL start_line"
PREFIX = b"fathomdb-gitleaks-history-v1\0"
EXPECTED_RECORDS = 57
EXPECTED_UNIQUE = 41
FINGERPRINT = re.compile(r"[0-9a-f]{32}\.[0-9a-f]{32}\Z")
COMMIT = re.compile(r"[0-9a-f]{40}\Z")
RULES = frozenset({"generic-api-key", "sourcegraph-access-token"})


def fail(reason: str) -> int:
    print(f"gitleaks-history: {reason}", file=sys.stderr)
    return 1


def tuple_fingerprint(parts: list[str]) -> str:
    encoded = PREFIX + b"\0".join(part.encode("utf-8") for part in parts)
    digest = hashlib.sha256(encoded).hexdigest()
    return f"{digest[:32]}.{digest[32:]}"


def load_manifest(path: Path) -> Counter[str] | None:
    try:
        value = json.loads(path.read_text())
    except (OSError, json.JSONDecodeError):
        return None
    if not isinstance(value, dict) or set(value) != {
        "schema",
        "scanner_version",
        "tuple_encoding",
        "expected_records",
        "expected_unique_fingerprints",
        "entries",
    }:
        return None
    if (
        value["schema"] != SCHEMA
        or value["scanner_version"] != SCANNER_VERSION
        or value["tuple_encoding"] != TUPLE_ENCODING
        or value["expected_records"] != EXPECTED_RECORDS
        or value["expected_unique_fingerprints"] != EXPECTED_UNIQUE
        or not isinstance(value["entries"], list)
    ):
        return None
    entries: Counter[str] = Counter()
    for entry in value["entries"]:
        if not isinstance(entry, dict) or set(entry) != {"fingerprint", "count"}:
            return None
        fingerprint = entry["fingerprint"]
        count = entry["count"]
        if not isinstance(fingerprint, str) or not FINGERPRINT.fullmatch(fingerprint):
            return None
        if not isinstance(count, int) or isinstance(count, bool) or count < 1:
            return None
        if fingerprint in entries:
            return None
        entries[fingerprint] = count
    if len(entries) != EXPECTED_UNIQUE or sum(entries.values()) != EXPECTED_RECORDS:
        return None
    return entries


def load_report(path: Path) -> Counter[str] | None:
    try:
        lines = path.read_text().splitlines()
    except OSError:
        return None
    if not lines:
        return None
    records: Counter[str] = Counter()
    for line in lines:
        parts = line.split("|")
        if len(parts) != 4:
            return None
        commit, rule, path_value, line_number = parts
        if (
            not COMMIT.fullmatch(commit)
            or rule not in RULES
            or not path_value
            or path_value.startswith("/")
            or "\\" in path_value
            or any(part in {"", ".", ".."} for part in path_value.split("/"))
            or not line_number.isdecimal()
            or int(line_number) < 1
        ):
            return None
        records[tuple_fingerprint(parts)] += 1
    return records


def main() -> int:
    if len(sys.argv) != 3:
        return fail("usage requires MANIFEST and SAFE_REPORT")
    expected = load_manifest(Path(sys.argv[1]))
    if expected is None:
        return fail("invalid reviewed redacted manifest")
    observed = load_report(Path(sys.argv[2]))
    if observed is None:
        return fail("invalid safe scanner report")
    unknown = sum((observed - expected).values())
    missing = sum((expected - observed).values())
    count_mismatch = sum(
        expected[key] != observed[key] for key in expected.keys() & observed.keys()
    )
    if unknown or missing or count_mismatch:
        return fail(
            "safe multiset mismatch "
            f"expected_records={sum(expected.values())} observed_records={sum(observed.values())} "
            f"unknown={unknown} missing={missing} count_mismatch={count_mismatch}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
