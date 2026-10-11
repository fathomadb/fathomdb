#!/usr/bin/env python3
"""Independently score the first Slice 135 Phase 2 installed-SDK fixture."""

from __future__ import annotations

import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import sqlite3
from typing import Any
import zipfile


HIT_FIELDS = ("id_space", "id_value", "kind", "body", "branch", "source_id")


def _bound(name: str, observed: object, expected: object) -> None:
    if observed != expected:
        raise ValueError(f"{name} mismatch")


def _fixture_index(fixture: dict[str, Any]) -> tuple[dict[str, tuple[int, dict]], dict[str, list[str]]]:
    corpus = fixture.get("corpus")
    queries = fixture.get("queries")
    if not isinstance(corpus, list) or not corpus or not isinstance(queries, list) or not queries:
        raise ValueError("fixture corpus or queries missing")
    by_body: dict[str, tuple[int, dict]] = {}
    for index, row in enumerate(corpus):
        if not isinstance(row, dict) or not isinstance(row.get("body"), str) or not isinstance(row.get("kind"), str):
            raise ValueError("fixture corpus row malformed")
        body = row["body"]
        if not body or body in by_body:
            raise ValueError("fixture corpus body empty or duplicate")
        by_body[body] = (index, row)
    expected: dict[str, list[str]] = {}
    for case in queries:
        if not isinstance(case, dict):
            raise ValueError("fixture query malformed")
        query = case.get("query")
        bodies = case.get("expected_bodies")
        if (not isinstance(query, str) or not query or query in expected
                or not isinstance(bodies, list)
                or any(not isinstance(body, str) or body not in by_body for body in bodies)
                or len(bodies) != len(set(bodies))):
            raise ValueError("fixture query or expected bodies malformed")
        expected[query] = bodies
    return by_body, expected


def _check_hits(
    hits: object,
    *,
    query: str,
    stage: str,
    bodies: list[str],
    by_body: dict[str, tuple[int, dict]],
    source_id: str,
    id_prefix: str,
    ordered_bodies: list[str] | None,
) -> None:
    if not isinstance(hits, list):
        raise ValueError(f"{query} {stage}: hits missing")
    seen_ids: set[str] = set()
    observed_bodies: list[str] = []
    for hit in hits:
        if not isinstance(hit, dict) or any(not isinstance(hit.get(field), str) for field in HIT_FIELDS):
            raise ValueError(f"{query} {stage}: hit malformed")
        body = hit["body"]
        if body not in by_body:
            raise ValueError(f"{query} {stage}: unknown body")
        index, row = by_body[body]
        _bound(f"{query} {stage}: id space", hit["id_space"], "logical")
        _bound(f"{query} {stage}: id", hit["id_value"], f"{id_prefix}{index}")
        _bound(f"{query} {stage}: kind", hit["kind"], row["kind"])
        _bound(f"{query} {stage}: branch", hit["branch"], "text")
        _bound(f"{query} {stage}: source", hit["source_id"], source_id)
        if hit["id_value"] in seen_ids:
            raise ValueError(f"{query} {stage}: duplicate hit")
        seen_ids.add(hit["id_value"])
        observed_bodies.append(body)
    if Counter(observed_bodies) != Counter(bodies):
        raise ValueError(f"{query} {stage}: gold bodies mismatch")
    if ordered_bodies is not None and observed_bodies != ordered_bodies:
        raise ValueError(f"{query} {stage}: order mismatch")


def audit(
    raw: dict[str, Any],
    protocol: dict[str, Any],
    fixture: dict[str, Any],
    *,
    version: str,
    protocol_sha256: str,
    fixture_sha256: str,
    runner_sha256: str,
) -> dict[str, int]:
    """Check one installed version against authored fixture truth and pinned identities.

    The caller must hash the protocol, fixture, runner and installed artifacts
    from disk before passing their digests. This function never treats another
    product version's result as a correctness oracle.
    """
    if not isinstance(raw, dict) or not isinstance(protocol, dict) or not isinstance(fixture, dict):
        raise ValueError("raw, protocol and fixture must be objects")
    _bound("schema_version", protocol.get("schema_version"), 1)
    _bound("schema_version", raw.get("schema_version"), 1)
    _bound("protocol_sha256", raw.get("protocol_sha256"), protocol_sha256)
    _bound("fixture_sha256", protocol.get("fixture_sha256"), fixture_sha256)
    _bound("fixture_sha256", raw.get("fixture_sha256"), fixture_sha256)
    _bound("runner_sha256", protocol.get("runner_sha256"), runner_sha256)
    _bound("runner_sha256", raw.get("runner_sha256"), runner_sha256)
    source_id = protocol.get("source_id")
    id_prefix = protocol.get("logical_id_prefix")
    if not isinstance(source_id, str) or not source_id or not isinstance(id_prefix, str) or not id_prefix:
        raise ValueError("protocol seed identity missing")
    versions = protocol.get("versions")
    if not isinstance(versions, dict) or version not in versions or not isinstance(versions[version], dict):
        raise ValueError("version absent from protocol")
    _bound("version", raw.get("version"), version)
    for name in ("source_sha", "wheel_sha256", "native_sha256"):
        _bound(name, raw.get(name), versions[version].get(name))
    by_body, expected = _fixture_index(fixture)
    ordered = protocol.get("ordered_case")
    if not isinstance(ordered, dict) or ordered.get("query") not in expected:
        raise ValueError("ordered case missing from fixture")
    ordered_query = ordered["query"]
    ordered_bodies = ordered.get("expected_bodies")
    if not isinstance(ordered_bodies, list) or Counter(ordered_bodies) != Counter(expected[ordered_query]):
        raise ValueError("ordered case contradicts fixture")
    cases = raw.get("cases")
    if not isinstance(cases, list) or len(cases) != len(expected):
        raise ValueError("cases missing or changed")
    seen_queries: set[str] = set()
    for case in cases:
        if not isinstance(case, dict):
            raise ValueError("case malformed")
        query = case.get("query")
        if not isinstance(query, str) or query not in expected or query in seen_queries:
            raise ValueError("cases missing, repeated or changed")
        seen_queries.add(query)
        for stage in ("before", "after"):
            _check_hits(
                case.get(stage),
                query=query,
                stage=stage,
                bodies=expected[query],
                by_body=by_body,
                source_id=source_id,
                id_prefix=id_prefix,
                ordered_bodies=ordered_bodies if query == ordered_query else None,
            )
    if seen_queries != set(expected):
        raise ValueError("cases missing or changed")
    return {"queries": len(expected), "reopened_queries": len(expected), "exact_order_queries": 1}


def audit_database(
    path: Path,
    fixture: dict[str, Any],
    *,
    source_id: str,
    id_prefix: str,
) -> dict[str, int | str]:
    """Check retained SQLite integrity and every canonical row against fixture inputs."""
    by_body, _ = _fixture_index(fixture)
    expected = sorted(
        (f"{id_prefix}{index}", row["kind"], body, source_id, "active", None)
        for body, (index, row) in by_body.items()
    )
    try:
        with sqlite3.connect(f"file:{path}?mode=ro", uri=True) as connection:
            integrity_row = connection.execute("PRAGMA integrity_check").fetchone()
            observed = connection.execute(
                "SELECT logical_id, kind, body, source_id, state, superseded_at "
                "FROM canonical_nodes ORDER BY logical_id"
            ).fetchall()
    except sqlite3.Error as error:
        raise ValueError("persisted database inspection failed") from error
    integrity = integrity_row[0] if integrity_row else None
    if integrity != "ok":
        raise ValueError("persisted database integrity failed")
    if observed != expected:
        raise ValueError("persisted canonical rows differ from authored fixture")
    return {"canonical_rows": len(observed), "integrity": integrity}


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _wheel_identity(path: Path, expected: dict[str, str]) -> None:
    if _sha256(path) != expected.get("wheel_sha256"):
        raise ValueError("wheel file SHA-256 mismatch")
    try:
        with zipfile.ZipFile(path) as archive:
            native = archive.read("fathomdb/_fathomdb.abi3.so")
    except (OSError, KeyError, zipfile.BadZipFile) as error:
        raise ValueError("wheel native member missing or malformed") from error
    if hashlib.sha256(native).hexdigest() != expected.get("native_sha256"):
        raise ValueError("wheel native member SHA-256 mismatch")


def audit_campaign(
    *,
    protocol_path: Path,
    fixture_path: Path,
    runner_path: Path,
    oracle_source_path: Path,
    wheels: dict[str, Path],
    version_dirs: dict[str, Path],
) -> dict[str, Any]:
    """Recompute a paired fixture verdict from files and real persisted state.

    Both versions are independently compared with authored fixture truth.
    Input paths are explicit so untrusted raw JSON cannot redirect the audit.
    """
    expected_versions = {"baseline", "candidate"}
    if set(wheels) != expected_versions or set(version_dirs) != expected_versions:
        raise ValueError("paired version files missing or changed")
    if version_dirs["baseline"].resolve() == version_dirs["candidate"].resolve():
        raise ValueError("paired versions share one database directory")
    protocol_hash = _sha256(protocol_path)
    fixture_hash = _sha256(fixture_path)
    runner_hash = _sha256(runner_path)
    oracle_hash = _sha256(oracle_source_path)
    protocol = json.loads(protocol_path.read_text())
    fixture = json.loads(fixture_path.read_text())
    if protocol.get("ordered_case_source_sha256") != oracle_hash:
        raise ValueError("ordered oracle source SHA-256 mismatch")
    if set(protocol.get("versions", {})) != expected_versions:
        raise ValueError("protocol version set mismatch")
    results: dict[str, Any] = {}
    for version in ("baseline", "candidate"):
        specification = protocol["versions"][version]
        _wheel_identity(wheels[version], specification)
        directory = version_dirs[version]
        raw_path = directory / "raw.json"
        database = directory / "contract.sqlite"
        raw = json.loads(raw_path.read_text())
        environment = raw.get("environment")
        identity = raw.get("identity")
        if (not isinstance(environment, dict)
                or any(not isinstance(environment.get(key), str) or not environment[key]
                       for key in ("python_executable", "python_version", "platform"))
                or not isinstance(identity, dict)
                or any(not isinstance(identity.get(key), str) or not identity[key]
                       for key in ("module_path", "native_path"))):
            raise ValueError(f"{version}: installed environment identity incomplete")
        scored = audit(
            raw,
            protocol,
            fixture,
            version=version,
            protocol_sha256=protocol_hash,
            fixture_sha256=fixture_hash,
            runner_sha256=runner_hash,
        )
        database_hash = _sha256(database)
        if raw.get("database_sha256") != database_hash:
            raise ValueError(f"{version}: retained database SHA-256 mismatch")
        persisted = audit_database(
            database,
            fixture,
            source_id=protocol["source_id"],
            id_prefix=protocol["logical_id_prefix"],
        )
        if raw.get("sqlite_integrity_check") != persisted["integrity"]:
            raise ValueError(f"{version}: SQLite integrity report mismatch")
        results[version] = {
            **scored,
            **persisted,
            "raw_sha256": _sha256(raw_path),
            "database_sha256": database_hash,
            "wheel_sha256": _sha256(wheels[version]),
        }
    return {
        "verdict": "both_versions_match_independent_fixture_gold",
        "protocol_sha256": protocol_hash,
        "fixture_sha256": fixture_hash,
        "runner_sha256": runner_hash,
        "ordered_oracle_source_sha256": oracle_hash,
        "versions": results,
    }


def main() -> None:
    """Audit one retained baseline/candidate pair and save its recomputation."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--protocol", type=Path, required=True)
    parser.add_argument("--fixture", type=Path, required=True)
    parser.add_argument("--runner", type=Path, required=True)
    parser.add_argument("--oracle-source", type=Path, required=True)
    parser.add_argument("--baseline-wheel", type=Path, required=True)
    parser.add_argument("--candidate-wheel", type=Path, required=True)
    parser.add_argument("--baseline-dir", type=Path, required=True)
    parser.add_argument("--candidate-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    result = audit_campaign(
        protocol_path=args.protocol,
        fixture_path=args.fixture,
        runner_path=args.runner,
        oracle_source_path=args.oracle_source,
        wheels={"baseline": args.baseline_wheel, "candidate": args.candidate_wheel},
        version_dirs={"baseline": args.baseline_dir, "candidate": args.candidate_dir},
    )
    with args.output.open("x") as destination:
        json.dump(result, destination, indent=2, sort_keys=True)
        destination.write("\n")
    print(args.output)


if __name__ == "__main__":
    main()
