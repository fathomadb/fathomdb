#!/usr/bin/env python3
"""Record installed-Python Phase 2 query observations without scoring them."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import platform
import sqlite3
import sys
from typing import Any
import zipfile


def sha256(path: Path) -> str:
    """Hash one input file without changing it."""
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def seed_records(fixture: dict, *, source_id: str, id_prefix: str) -> list[dict[str, str]]:
    """Make stable write inputs from corpus rows, excluding all fixture answers."""
    corpus = fixture.get("corpus")
    if not isinstance(corpus, list) or not corpus:
        raise ValueError("fixture corpus missing")
    records = []
    for index, row in enumerate(corpus):
        if (not isinstance(row, dict) or not isinstance(row.get("kind"), str)
                or not isinstance(row.get("body"), str)):
            raise ValueError("fixture corpus row malformed")
        records.append({
            "kind": row["kind"],
            "body": row["body"],
            "logical_id": f"{id_prefix}{index}",
            "source_id": source_id,
        })
    return records


def serialize_hits(hits: list[Any]) -> list[dict[str, str]]:
    """Retain each product hit exactly; leave semantic judgment to the auditor."""
    return [
        {
            "id_space": hit.id.space,
            "id_value": hit.id.value,
            "kind": hit.kind,
            "body": hit.body,
            "branch": hit.branch,
            "source_id": hit.source_id,
        }
        for hit in hits
    ]


def _installed_identity(wheel: Path, expected_wheel: str, expected_native: str) -> dict[str, str]:
    import fathomdb
    import fathomdb._fathomdb as native

    if sha256(wheel) != expected_wheel:
        raise ValueError("wheel SHA-256 mismatch")
    if fathomdb.__file__ is None or native.__file__ is None:
        raise ValueError("installed SDK path unavailable")
    package = Path(fathomdb.__file__).resolve()
    native_path = Path(native.__file__).resolve()
    prefix = Path(sys.prefix).resolve()
    if not package.is_relative_to(prefix) or not native_path.is_relative_to(prefix):
        raise ValueError("SDK import escaped isolated Python environment")
    with zipfile.ZipFile(wheel) as archive:
        for member, installed in (
            ("fathomdb/__init__.py", package),
            ("fathomdb/_fathomdb.abi3.so", native_path),
        ):
            if hashlib.sha256(archive.read(member)).hexdigest() != sha256(installed):
                raise ValueError(f"installed SDK differs from wheel: {member}")
    if sha256(native_path) != expected_native:
        raise ValueError("native module SHA-256 mismatch")
    return {"module_path": str(package), "native_path": str(native_path)}


def _observe(engine: Any, queries: list[dict]) -> list[dict[str, Any]]:
    return [
        {"query": case["query"], "hits": serialize_hits(list(engine.search(case["query"]).results))}
        for case in queries
    ]


def run(
    *,
    protocol_path: Path,
    fixture_path: Path,
    wheel: Path,
    version: str,
    output: Path,
) -> Path:
    """Run authored queries before and after reopen in a fresh real database."""
    import fathomdb

    protocol_bytes = protocol_path.read_bytes()
    protocol = json.loads(protocol_bytes)
    fixture = json.loads(fixture_path.read_bytes())
    if protocol.get("schema_version") != 1 or version not in protocol.get("versions", {}):
        raise ValueError("protocol schema or version mismatch")
    runner_hash = sha256(Path(__file__).resolve())
    fixture_hash = sha256(fixture_path)
    if protocol.get("runner_sha256") != runner_hash or protocol.get("fixture_sha256") != fixture_hash:
        raise ValueError("runner or fixture SHA-256 mismatch")
    version_spec = protocol["versions"][version]
    identity = _installed_identity(
        wheel, version_spec["wheel_sha256"], version_spec["native_sha256"]
    )
    source_id = protocol["source_id"]
    id_prefix = protocol["logical_id_prefix"]
    records = seed_records(fixture, source_id=source_id, id_prefix=id_prefix)
    queries = fixture["queries"]
    if not isinstance(queries, list) or not all(isinstance(q.get("query"), str) for q in queries):
        raise ValueError("fixture queries malformed")
    output.mkdir(parents=True, exist_ok=False)
    database = output / "contract.sqlite"
    engine = fathomdb.Engine.open(str(database), use_default_embedder=False)
    try:
        for row in records:
            engine.write([row])
        engine.drain(timeout_s=30)
        before = _observe(engine, queries)
    finally:
        engine.close()
    engine = fathomdb.Engine.open(str(database), use_default_embedder=False)
    try:
        after = _observe(engine, queries)
    finally:
        engine.close()
    cases = [
        {"query": first["query"], "before": first["hits"], "after": second["hits"]}
        for first, second in zip(before, after, strict=True)
    ]
    with sqlite3.connect(f"file:{database}?mode=ro", uri=True) as connection:
        integrity = connection.execute("PRAGMA integrity_check").fetchone()[0]
    raw = {
        "schema_version": 1,
        "version": version,
        "source_sha": version_spec["source_sha"],
        "wheel_sha256": version_spec["wheel_sha256"],
        "native_sha256": version_spec["native_sha256"],
        "protocol_sha256": hashlib.sha256(protocol_bytes).hexdigest(),
        "fixture_sha256": fixture_hash,
        "runner_sha256": runner_hash,
        "database_sha256": sha256(database),
        "sqlite_integrity_check": integrity,
        "identity": identity,
        "environment": {
            "python_executable": str(Path(sys.executable).resolve()),
            "python_version": sys.version,
            "platform": platform.platform(),
        },
        "cases": cases,
    }
    raw_path = output / "raw.json"
    raw_path.write_text(json.dumps(raw, indent=2, sort_keys=True) + "\n")
    return raw_path


def main() -> None:
    """Parse a pinned installed-wheel run request and write unscored raw data."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--protocol", type=Path, required=True)
    parser.add_argument("--fixture", type=Path, required=True)
    parser.add_argument("--wheel", type=Path, required=True)
    parser.add_argument("--version", choices=("baseline", "candidate"), required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    path = run(
        protocol_path=args.protocol,
        fixture_path=args.fixture,
        wheel=args.wheel,
        version=args.version,
        output=args.output,
    )
    print(path)


if __name__ == "__main__":
    main()
