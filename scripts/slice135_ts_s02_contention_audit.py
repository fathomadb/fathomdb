#!/usr/bin/env python3
"""Independently audit installed TypeScript S02 shared-engine contention."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import sqlite3


EXPECTED_SDK_STATE = {
    "anchor_retained": True,
    "erased_writer_absent": True,
    "erased_graph_absent": True,
    "evidence_query_empty": True,
    "graph_empty": True,
}
EXPECTED_DATABASE = {
    "corpus_nodes": 32,
    "graph_nodes": 0,
    "graph_edges": 0,
    "contended_nodes": 0,
    "integrity_check": "ok",
}
EXPECTED_ERASURE = {"graph_nodes": 3, "graph_edges": 1, "contended_nodes": 8}
EXPECTED_EVIDENCE = {
    "logical_id": "s02-claim",
    "source_body": '{"summary": "s02 canonical source evidence bytes"}',
}


def sha(path: Path) -> str:
    """Hash exact retained file bytes."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def audit_timeline(writer: list[dict], reader: list[dict], *, claimed: int, expected: int) -> int:
    """Recompute actual shared-engine call-interval overlap."""
    if len(writer) != expected:
        raise ValueError("writer count differs from declared workload")
    if len(reader) != expected:
        raise ValueError("reader count differs from declared workload")
    if [row.get("logical_id") for row in writer] != [f"contend-{number:02d}" for number in range(expected)]:
        raise ValueError("writer identity or order")
    for calls in (writer, reader):
        if any(
            type(row.get("start_ns")) is not int
            or type(row.get("end_ns")) is not int
            or row["start_ns"] >= row["end_ns"]
            for row in calls
        ):
            raise ValueError("call interval invalid")
        if any(left["start_ns"] >= right["start_ns"] for left, right in zip(calls, calls[1:])):
            raise ValueError("call order invalid")
    if any(type(row.get("vector_hits")) is not int or row["vector_hits"] < 1 for row in reader):
        raise ValueError("reader vector result missing")
    overlap = sum(
        any(
            write["start_ns"] < read["end_ns"]
            and write["end_ns"] > read["start_ns"]
            for write in writer
        )
        for read in reader
    )
    if overlap < 1 or claimed != overlap:
        raise ValueError("call overlap claim differs from timeline")
    return overlap


def reopened_state(database: Path) -> dict:
    """Inspect real persisted rows without using the installed SDK."""
    with sqlite3.connect(f"file:{database}?mode=ro", uri=True) as connection:
        counts = {
            "corpus_nodes": connection.execute(
                "SELECT COUNT(*) FROM canonical_nodes WHERE source_id='slice135-s01-python'"
            ).fetchone()[0],
            "graph_nodes": connection.execute(
                "SELECT COUNT(*) FROM canonical_nodes WHERE source_id='slice135-s02-graph'"
            ).fetchone()[0],
            "graph_edges": connection.execute(
                "SELECT COUNT(*) FROM canonical_edges WHERE source_id='slice135-s02-graph'"
            ).fetchone()[0],
            "contended_nodes": connection.execute(
                "SELECT COUNT(*) FROM canonical_nodes WHERE source_id='slice135-contended'"
            ).fetchone()[0],
            "integrity_check": connection.execute("PRAGMA integrity_check").fetchone()[0],
        }
    return counts


def audit_run(
    *, raw_path: Path, database: Path, runner: Path, s01_helper: Path,
    s02_helper: Path, comparison_protocol: Path, reference_manifest: Path,
    install_root: Path, main_archive: Path, platform_archive: Path,
) -> dict:
    """Reject false timing, source, package, overlap and reopened-state claims."""
    raw = json.loads(raw_path.read_text())
    protocol = json.loads(comparison_protocol.read_text())
    reference = json.loads(reference_manifest.read_text())
    role = reference.get("role")
    if role not in {"baseline", "candidate"} or protocol.get("status") != "FROZEN_TS_S02_PAIRED":
        raise ValueError("reference role or comparison protocol invalid")
    pin = protocol[role]
    if (
        raw.get("schema_version") != 1
        or raw.get("status") != "S02_TS_CONTENTION_FUNCTIONAL_OK"
        or raw.get("source_sha") != pin["source_sha"]
        or reference.get("source_sha") != pin["source_sha"]
        or raw.get("runner_sha256") != sha(runner)
        or raw.get("s01_helper_sha256") != sha(s01_helper)
        or raw.get("s02_helper_sha256") != sha(s02_helper)
    ):
        raise ValueError("source or runner identity invalid")
    if (
        sha(main_archive) != pin["main_archive_sha256"]
        or sha(platform_archive) != pin["platform_archive_sha256"]
        or reference["artifacts"]["main_archive_sha256"] != pin["main_archive_sha256"]
        or reference["artifacts"]["platform_archive_sha256"] != pin["platform_archive_sha256"]
    ):
        raise ValueError("npm archive identity invalid")
    artifact = raw.get("artifact", {})
    expected_paths = {
        "module_path": install_root / "node_modules/fathomdb/dist/index.js",
        "native_path": install_root / "node_modules/fathomdb-linux-x64-gnu/fathomdb.linux-x64-gnu.node",
        "package_path": install_root / "node_modules/fathomdb/package.json",
    }
    if Path(artifact.get("module_path", "")).resolve() != expected_paths["module_path"].resolve():
        raise ValueError("installed module path invalid")
    for key, hash_key in (("module_path", "module_sha256"), ("native_path", "native_sha256"),
                          ("package_path", "package_sha256")):
        actual = sha(expected_paths[key])
        if artifact.get(hash_key) != actual or reference.get(hash_key) != actual:
            raise ValueError(f"installed {hash_key} differs from archive receipt")
    if artifact.get("node_version") != protocol["node_version"]:
        raise ValueError("Node version invalid")
    observed = raw.get("observed", {})
    timeline = raw.get("timeline", {})
    if (
        observed.get("status") != "S02_TS_CONTENTION_FUNCTIONAL_OK"
        or observed.get("writer_ids") != [f"contend-{number:02d}" for number in range(8)]
        or observed.get("reader_cycles") != 8
        or observed.get("bounded_seconds") != 60
    ):
        raise ValueError("observed contention sequence invalid")
    overlap = audit_timeline(timeline.get("writer_calls", []), timeline.get("reader_calls", []),
                             claimed=observed.get("overlap_cycles"), expected=8)
    if (
        observed.get("readiness") != "ready"
        or observed.get("reopened_readiness") != "ready"
        or observed.get("evidence") != EXPECTED_EVIDENCE
        or observed.get("erasure") != EXPECTED_ERASURE
        or any(observed.get(key) != EXPECTED_SDK_STATE for key in ("before_reopen", "after_reopen"))
    ):
        raise ValueError("SDK semantic or reopened state invalid")
    elapsed = observed.get("elapsed_ns")
    verification = observed.get("verification_ns")
    if (
        type(elapsed) is not int or not 0 < elapsed < 60_000_000_000
        or type(verification) is not int or verification <= 0
    ):
        raise ValueError("product or verification timer invalid")
    state = reopened_state(database)
    if observed.get("final_state") != state or state != EXPECTED_DATABASE:
        raise ValueError("reopened SQLite state invalid")
    return {
        "status": "ACCEPTED_S02_TS_CONTENTION_FUNCTIONAL",
        "role": role,
        "source_sha": pin["source_sha"],
        "actual_call_overlap_cycles": overlap,
        "elapsed_ns": elapsed,
        "verification_ns": verification,
        "raw_sha256": sha(raw_path),
        "database_sha256": sha(database),
        "runner_sha256": sha(runner),
        "installed_native_sha256": artifact["native_sha256"],
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    for name in (
        "raw", "database", "runner", "s01-helper", "s02-helper",
        "comparison-protocol", "reference-manifest", "install-root",
        "main-archive", "platform-archive", "output",
    ):
        parser.add_argument("--" + name, required=True, type=Path)
    args = parser.parse_args()
    result = audit_run(
        raw_path=args.raw, database=args.database, runner=args.runner,
        s01_helper=args.s01_helper, s02_helper=args.s02_helper,
        comparison_protocol=args.comparison_protocol,
        reference_manifest=args.reference_manifest, install_root=args.install_root,
        main_archive=args.main_archive, platform_archive=args.platform_archive,
    )
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(result["status"])


if __name__ == "__main__":
    main()
