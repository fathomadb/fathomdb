#!/usr/bin/env python3
"""Independently audit paired Phase 2 filter, page and typed-error receipts."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import sqlite3
from typing import Any
import zipfile


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _exact(left: object, right: object) -> bool:
    return json.dumps(left, sort_keys=True, separators=(",", ":")) == json.dumps(
        right, sort_keys=True, separators=(",", ":")
    )


def audit_cases(
    cases: object, requests: list[dict[str, Any]], gold: dict[str, Any],
) -> dict[str, int]:
    """Reject every omitted or changed outcome against predeclared human gold."""
    ids = [request.get("id") for request in requests]
    if (not isinstance(cases, list) or not ids or len(ids) != len(set(ids))
            or set(ids) != set(gold) or len(cases) != len(ids)):
        raise ValueError("matrix case set missing or changed")
    for case, expected_id in zip(cases, ids, strict=True):
        if not isinstance(case, dict) or set(case) != {"id", "before", "after"} or case["id"] != expected_id:
            raise ValueError("matrix case ID or stages changed")
        for stage in ("before", "after"):
            if not _exact(case[stage], gold[expected_id]):
                raise ValueError(f"{expected_id} {stage}: gold outcome mismatch")
    return {"cases": len(ids), "reopened_cases": len(ids)}


def _check_wheel(path: Path, specification: dict[str, str]) -> None:
    if _sha256(path) != specification["wheel_sha256"]:
        raise ValueError("wheel SHA-256 mismatch")
    try:
        with zipfile.ZipFile(path) as archive:
            native = archive.read("fathomdb/_fathomdb.abi3.so")
    except (OSError, KeyError, zipfile.BadZipFile) as error:
        raise ValueError("wheel native member unavailable") from error
    if hashlib.sha256(native).hexdigest() != specification["native_sha256"]:
        raise ValueError("wheel native SHA-256 mismatch")


def _check_database(path: Path, fixture: dict, source_id: str, prefix: str) -> int:
    expected = sorted(
        (f"{prefix}{index}", row["kind"], row["body"], source_id, "active", None)
        for index, row in enumerate(fixture["corpus"])
    )
    try:
        with sqlite3.connect(f"file:{path}?mode=ro", uri=True) as connection:
            integrity_row = connection.execute("PRAGMA integrity_check").fetchone()
            observed = connection.execute(
                "SELECT logical_id, kind, body, source_id, state, superseded_at "
                "FROM canonical_nodes ORDER BY logical_id"
            ).fetchall()
    except sqlite3.Error as error:
        raise ValueError("persisted matrix database inspection failed") from error
    if not integrity_row or integrity_row[0] != "ok" or observed != expected:
        raise ValueError("persisted matrix database differs from fixture")
    return len(observed)


def audit_campaign(
    *, protocol_path: Path, fixture_path: Path, runner_path: Path,
    authored_sources: dict[str, Path], wheels: dict[str, Path], version_dirs: dict[str, Path],
) -> dict[str, Any]:
    """Recompute both version outcomes, artifact identities and persisted state."""
    versions = {"baseline", "candidate"}
    if set(wheels) != versions or set(version_dirs) != versions:
        raise ValueError("paired version files missing")
    if version_dirs["baseline"].resolve() == version_dirs["candidate"].resolve():
        raise ValueError("paired versions share one database directory")
    protocol_hash = _sha256(protocol_path)
    fixture_hash = _sha256(fixture_path)
    runner_hash = _sha256(runner_path)
    protocol = json.loads(protocol_path.read_text())
    fixture = json.loads(fixture_path.read_text())
    if protocol.get("schema_version") != 1 or set(protocol.get("versions", {})) != versions:
        raise ValueError("protocol schema or version set mismatch")
    if protocol.get("fixture_sha256") != fixture_hash or protocol.get("runner_sha256") != runner_hash:
        raise ValueError("fixture or runner SHA-256 mismatch")
    declared_sources = protocol.get("authored_sources")
    if not isinstance(declared_sources, dict) or set(declared_sources) != set(authored_sources):
        raise ValueError("authored source set mismatch")
    for name, path in authored_sources.items():
        if _sha256(path) != declared_sources[name]["sha256"]:
            raise ValueError(f"authored source SHA-256 mismatch: {name}")
    requests = protocol["requests"]
    gold = protocol["gold"]
    source_id = protocol["source_id"]
    prefix = protocol["logical_id_prefix"]
    result: dict[str, Any] = {"protocol_sha256": protocol_hash, "versions": {}}
    for version in ("baseline", "candidate"):
        specification = protocol["versions"][version]
        _check_wheel(wheels[version], specification)
        directory = version_dirs[version]
        raw = json.loads((directory / "raw.json").read_text())
        database = directory / "matrix.sqlite"
        identity = raw.get("identity")
        environment = raw.get("environment")
        if (not isinstance(identity, dict) or not isinstance(environment, dict)
                or any(not isinstance(identity.get(key), str) or not identity[key]
                       for key in ("module_path", "native_path"))
                or any(not isinstance(environment.get(key), str) or not environment[key]
                       for key in ("python_executable", "python_version", "platform"))):
            raise ValueError(f"{version}: installed SDK identity missing")
        for key, expected in (
            ("schema_version", 1), ("version", version),
            ("source_sha", specification["source_sha"]),
            ("wheel_sha256", specification["wheel_sha256"]),
            ("native_sha256", specification["native_sha256"]),
            ("protocol_sha256", protocol_hash), ("fixture_sha256", fixture_hash),
            ("runner_sha256", runner_hash), ("database_sha256", _sha256(database)),
            ("sqlite_integrity_check", "ok"),
        ):
            if raw.get(key) != expected:
                raise ValueError(f"{version}: {key} mismatch")
        scored = audit_cases(raw.get("cases"), requests, gold)
        scored["canonical_rows"] = _check_database(database, fixture, source_id, prefix)
        result["versions"][version] = scored
    return result


def main() -> None:
    """Audit a complete paired installed-wheel campaign from explicit paths."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--protocol", type=Path, required=True)
    parser.add_argument("--fixture", type=Path, required=True)
    parser.add_argument("--runner", type=Path, required=True)
    parser.add_argument("--search-test", type=Path, required=True)
    parser.add_argument("--page-test", type=Path, required=True)
    parser.add_argument("--baseline-wheel", type=Path, required=True)
    parser.add_argument("--candidate-wheel", type=Path, required=True)
    parser.add_argument("--baseline-dir", type=Path, required=True)
    parser.add_argument("--candidate-dir", type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(audit_campaign(
        protocol_path=args.protocol,
        fixture_path=args.fixture,
        runner_path=args.runner,
        authored_sources={"search_test": args.search_test, "page_test": args.page_test},
        wheels={"baseline": args.baseline_wheel, "candidate": args.candidate_wheel},
        version_dirs={"baseline": args.baseline_dir, "candidate": args.candidate_dir},
    ), indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
