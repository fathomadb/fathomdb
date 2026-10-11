#!/usr/bin/env python3
"""Independently audit installed-Python S02 contention and reopened state."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import sqlite3
import zipfile


WRITER_IDS = [f"contend-{number:02d}" for number in range(8)]
EXPECTED_COUNTS = {
    "corpus_nodes": 32, "graph_nodes": 0,
    "graph_edges": 0, "contended_nodes": 0,
}
EXPECTED_EVIDENCE = {
    "logical_id": "s02-claim",
    "source_body": '{"summary": "s02 canonical source evidence bytes"}',
}
EXPECTED_ERASURE = {"graph_nodes": 3, "graph_edges": 1, "contended_nodes": 8}


def sha(path: Path) -> str:
    """Hash exact retained bytes."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def audit_timeline(timeline: dict, observed: dict) -> int:
    """Recompute overlap from actual writer and reader SDK call intervals."""
    writer = timeline.get("writer_calls")
    reader = timeline.get("reader_calls")
    if not isinstance(writer, list) or len(writer) != 8:
        raise ValueError("writer call count")
    if not isinstance(reader, list) or len(reader) != 8:
        raise ValueError("reader call count")
    if [item.get("logical_id") for item in writer] != WRITER_IDS:
        raise ValueError("writer identity or order")
    for items in (writer, reader):
        if any(
            type(item.get("start_ns")) is not int
            or type(item.get("end_ns")) is not int
            or item["start_ns"] >= item["end_ns"]
            for item in items
        ):
            raise ValueError("call interval invalid")
        if any(left["start_ns"] >= right["start_ns"] for left, right in zip(items, items[1:])):
            raise ValueError("call order invalid")
    if any(type(item.get("vector_hits")) is not int or item["vector_hits"] < 1 for item in reader):
        raise ValueError("reader vector result missing")
    overlap = sum(
        any(
            write["start_ns"] < read["end_ns"]
            and write["end_ns"] > read["start_ns"]
            for write in writer
        )
        for read in reader
    )
    if overlap < 1 or observed.get("overlap_cycles") != overlap:
        raise ValueError("call overlap claim differs from timeline")
    if observed.get("reader_cycles") != len(reader):
        raise ValueError("reader cycle count")
    return overlap


def reopened_state(database: Path) -> tuple[dict[str, int], str]:
    """Check retained SQLite bytes without using the installed SDK."""
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
        }
        integrity = connection.execute("PRAGMA integrity_check").fetchone()[0]
    return counts, integrity


def resource_report(path: Path) -> dict[str, float | int]:
    """Parse the retained GNU Time report independently."""
    fields = dict(line.split("=", 1) for line in path.read_text().splitlines() if "=" in line)
    if set(fields) != {
        "user_s", "system_s", "peak_rss_kib", "major_faults", "swap_events", "exit_code",
    }:
        raise ValueError("resource report fields")
    return {
        key: float(value) if key in {"user_s", "system_s"} else int(value)
        for key, value in fields.items()
    }


def audit_run(
    *, raw_path: Path, database: Path, wheel: Path, protocol: Path,
    runner: Path, stdout: Path, stderr: Path, resource: Path,
) -> dict:
    """Reject identity, semantic, timeline and persisted-state disagreement."""
    raw = json.loads(raw_path.read_text())
    frozen = json.loads(protocol.read_text())
    role = raw.get("role")
    if role not in {"baseline", "candidate"}:
        raise ValueError("role invalid")
    if frozen.get("status") != "FROZEN_S02_PYTHON_PAIRED":
        raise ValueError("source protocol not frozen")
    expected = frozen[role]
    if raw.get("schema_version") != 1 or raw.get("source_sha") != expected["source_sha"]:
        raise ValueError("source identity")
    if raw.get("comparison_protocol_sha256") != sha(protocol):
        raise ValueError("protocol identity")
    if raw.get("runner_sha256") != sha(runner):
        raise ValueError("runner identity")
    if raw.get("artifact", {}).get("wheel_sha256") != expected["wheel_sha256"] or sha(wheel) != expected["wheel_sha256"]:
        raise ValueError("installed wheel identity")
    native_sha = raw["artifact"].get("native_sha256")
    with zipfile.ZipFile(wheel) as archive:
        native_members = [
            name for name in archive.namelist()
            if name.startswith("fathomdb/") and name.endswith((".so", ".pyd"))
        ]
        if len(native_members) != 1 or hashlib.sha256(archive.read(native_members[0])).hexdigest() != native_sha:
            raise ValueError("native module differs from wheel")
    if stdout.read_text().splitlines() != ["S02_CONTENTION_FUNCTIONAL_OK"] or stderr.read_text():
        raise ValueError("process output")
    resources = resource_report(resource)
    if resources["exit_code"] != 0 or resources["swap_events"] != 0 or resources["peak_rss_kib"] < 1:
        raise ValueError("process resource or swap")
    observed = raw.get("observed")
    if not isinstance(observed, dict) or observed.get("status") != "S02_CONTENTION_FUNCTIONAL_OK":
        raise ValueError("observed status")
    if observed.get("writer_ids") != WRITER_IDS or observed.get("writer_errors") or observed.get("reader_errors"):
        raise ValueError("writer or reader result")
    overlap = audit_timeline(raw.get("timeline", {}), observed)
    if observed.get("readiness") != "ready":
        raise ValueError("projection readiness")
    if observed.get("evidence") != EXPECTED_EVIDENCE or observed.get("erasure") != EXPECTED_ERASURE:
        raise ValueError("evidence or erasure result")
    if observed.get("bounded_seconds") != 60 or type(observed.get("elapsed_ns")) is not int or not 0 < observed["elapsed_ns"] < 60_000_000_000:
        raise ValueError("bounded elapsed time")
    counts, integrity = reopened_state(database)
    if counts != EXPECTED_COUNTS or integrity != "ok":
        raise ValueError("reopened database state")
    if observed.get("before_reopen") != counts or observed.get("after_reopen") != counts or observed.get("integrity_check") != integrity:
        raise ValueError("recorded reopen state")
    return {
        "status": "ACCEPTED_S02_PYTHON_CONTENTION_FUNCTIONAL",
        "role": role,
        "source_sha": expected["source_sha"],
        "actual_call_overlap_cycles": overlap,
        "elapsed_ns": observed["elapsed_ns"],
        "peak_rss_kib": resources["peak_rss_kib"],
        "raw_sha256": sha(raw_path),
        "database_sha256": sha(database),
        "resource_sha256": sha(resource),
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("raw", "database", "wheel", "protocol", "runner", "stdout", "stderr", "resource", "output"):
        parser.add_argument("--" + name, required=True, type=Path)
    args = parser.parse_args()
    result = audit_run(
        raw_path=args.raw, database=args.database, wheel=args.wheel,
        protocol=args.protocol, runner=args.runner, stdout=args.stdout,
        stderr=args.stderr, resource=args.resource,
    )
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(result["status"])


if __name__ == "__main__":
    main()
