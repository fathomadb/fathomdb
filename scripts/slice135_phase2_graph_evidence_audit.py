#!/usr/bin/env python3
"""Audit paired graph, evidence, erasure and reopened SQLite results."""

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


def audit_observations(observations: object, fixture: dict[str, Any]) -> dict[str, int]:
    """Compare every captured phase with predeclared fixture truth."""
    gold = fixture["gold"]
    phases = ("before", "erase", "after_erase", "after_reopen")
    if not isinstance(observations, dict) or set(observations) != set(phases):
        raise ValueError("graph/evidence phase set missing or changed")
    for phase in phases:
        if not _exact(observations[phase], gold[phase]):
            raise ValueError(f"{phase}: graph/evidence gold outcome mismatch")
    return {
        "phases": len(phases),
        "graph_depth1_nodes": len(gold["before"]["graph_depth1"]),
        "graph_depth2_nodes": len(gold["before"]["graph_depth2"]),
        "evidence_refs": len(gold["before"]["evidence_refs"]),
        "erased_nodes": gold["erase"]["nodes_excised"],
    }


def audit_database(path: Path, fixture: dict[str, Any]) -> dict[str, int | str]:
    """Check retained canonical rows, edges, content FTS residue and integrity."""
    try:
        with sqlite3.connect(f"file:{path}?mode=ro", uri=True) as connection:
            integrity_row = connection.execute("PRAGMA integrity_check").fetchone()
            nodes = connection.execute(
                "SELECT logical_id, kind, body, source_id FROM canonical_nodes ORDER BY logical_id"
            ).fetchall()
            edges = connection.execute(
                "SELECT logical_id, from_id, to_id, source_id FROM canonical_edges ORDER BY logical_id"
            ).fetchall()
            residue = connection.execute(
                "SELECT COUNT(*) FROM search_index_v2 WHERE body = ?",
                (fixture["evidence_query"],),
            ).fetchone()[0]
    except sqlite3.Error as error:
        raise ValueError("persisted graph/evidence database inspection failed") from error
    observed = {
        "canonical_nodes": [list(row) for row in nodes],
        "canonical_edges": [list(row) for row in edges],
        "evidence_fts_residue": residue,
        "integrity": integrity_row[0] if integrity_row else None,
    }
    if not _exact(observed, fixture["gold"]["persisted"]):
        raise ValueError("persisted graph/evidence rows differ from authored fixture")
    return {
        "canonical_nodes": len(nodes),
        "canonical_edges": len(edges),
        "evidence_fts_residue": residue,
        "integrity": observed["integrity"],
    }


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


def audit_campaign(
    *, protocol_path: Path, fixture_path: Path, runner_path: Path,
    authored_sources: dict[str, Path], wheels: dict[str, Path], version_dirs: dict[str, Path],
) -> dict[str, Any]:
    """Recompute paired semantic and persisted-state verdicts from files."""
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
    declared = protocol.get("authored_sources")
    if not isinstance(declared, dict) or set(declared) != set(authored_sources):
        raise ValueError("authored source set mismatch")
    for name, path in authored_sources.items():
        if _sha256(path) != declared[name]["sha256"]:
            raise ValueError(f"authored source SHA-256 mismatch: {name}")
    result: dict[str, Any] = {"protocol_sha256": protocol_hash, "versions": {}}
    for version in ("baseline", "candidate"):
        specification = protocol["versions"][version]
        _check_wheel(wheels[version], specification)
        directory = version_dirs[version]
        raw = json.loads((directory / "raw.json").read_text())
        database = directory / "graph-evidence.sqlite"
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
        scored = audit_observations(raw.get("observations"), fixture)
        scored["persisted"] = audit_database(database, fixture)
        result["versions"][version] = scored
    return result


def main() -> None:
    """Audit a complete paired campaign from explicit local input paths."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--protocol", type=Path, required=True)
    parser.add_argument("--fixture", type=Path, required=True)
    parser.add_argument("--runner", type=Path, required=True)
    parser.add_argument("--graph-test", type=Path, required=True)
    parser.add_argument("--evidence-test", type=Path, required=True)
    parser.add_argument("--erasure-test", type=Path, required=True)
    parser.add_argument("--source-test", type=Path, required=True)
    parser.add_argument("--baseline-wheel", type=Path, required=True)
    parser.add_argument("--candidate-wheel", type=Path, required=True)
    parser.add_argument("--baseline-dir", type=Path, required=True)
    parser.add_argument("--candidate-dir", type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(audit_campaign(
        protocol_path=args.protocol,
        fixture_path=args.fixture,
        runner_path=args.runner,
        authored_sources={
            "graph_test": args.graph_test,
            "evidence_test": args.evidence_test,
            "erasure_test": args.erasure_test,
            "source_test": args.source_test,
        },
        wheels={"baseline": args.baseline_wheel, "candidate": args.candidate_wheel},
        version_dirs={"baseline": args.baseline_dir, "candidate": args.candidate_dir},
    ), indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
