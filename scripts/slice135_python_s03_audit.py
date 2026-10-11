#!/usr/bin/env python3
"""Independently inspect S03 pilot samples and reopened SQLite state."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
from pathlib import Path
import sqlite3
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
RUNNER = ROOT / "scripts/slice135_python_s03.py"
S01 = ROOT / "scripts/slice135_python_s01.py"
S02 = ROOT / "scripts/slice135_python_s02.py"
FIXTURE = ROOT / "src/python/tests/functional_search_fixture.json"
SOURCE_BODY = '{"summary": "s02 canonical source evidence bytes"}'


def digest(path: Path) -> str:
    """Hash exact bytes of an independently reopened input."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def check_attempt(name: str, attempt: dict[str, Any], size: int) -> int:
    """Reject a semantic mismatch even when the producer claims success."""
    elapsed = attempt.get("latency_ns")
    if (
        type(elapsed) is not int
        or elapsed <= 0
        or attempt.get("semantic_ok") is not True
    ):
        raise ValueError(f"{name}: invalid elapsed time or semantic claim")
    observed = attempt.get("observed")
    fixed = {
        "filter": ["alpha structured retrieval document"],
        "temporal_early": ["epoch alpha record"],
        "temporal_boundary": [],
        "temporal_late": ["epoch beta record"],
        "graph": ["s02-claim"],
        "evidence": [SOURCE_BODY],
    }
    if name in fixed:
        if observed != fixed[name]:
            raise ValueError(f"{name}: observed result differs from fixture")
    elif name == f"memory_{size}":
        if observed != [f"F{index:04}" for index in range(10)]:
            raise ValueError(f"{name}: memory corpus hits differ from fixture")
    else:
        raise ValueError(f"{name}: unrecognized case")
    return elapsed


def summarize_durations(durations: list[int]) -> dict[str, int | str]:
    """Recompute total cost and supported nearest-rank statistics."""
    if not durations or any(
        type(value) is not int or value <= 0 for value in durations
    ):
        raise ValueError("nonempty positive integer durations required")
    ordered = sorted(durations)

    def percentile(fraction: float) -> int:
        return ordered[math.ceil(fraction * len(ordered)) - 1]

    summary: dict[str, int | str] = {
        "count": len(ordered),
        "total_elapsed_ns": sum(ordered),
        "minimum_ns": ordered[0],
        "p50_ns": percentile(0.50),
        "p95_ns": percentile(0.95),
        "maximum_ns": ordered[-1],
        "p99": "unsupported_below_1000_samples",
    }
    if len(ordered) >= 1000:
        summary["p99"] = "measured"
        summary["p99_ns"] = percentile(0.99)
    return summary


def _check_database(path: Path, size: int) -> dict[str, Any]:
    if not path.is_file():
        raise ValueError("retained SQLite database missing")
    with sqlite3.connect(f"file:{path}?mode=ro", uri=True) as connection:
        integrity = connection.execute("PRAGMA integrity_check").fetchone()[0]
        nodes = connection.execute("SELECT count(*) FROM canonical_nodes").fetchone()[0]
        edges = connection.execute("SELECT count(*) FROM canonical_edges").fetchone()[0]
        memory = connection.execute(
            "SELECT count(*) FROM canonical_nodes WHERE source_id=?",
            ("slice135-s01-python",),
        ).fetchone()[0]
        rows = dict(
            connection.execute(
                "SELECT logical_id, body FROM canonical_nodes WHERE logical_id IN"
                " ('s03-x1-0','s03-x1-1','s03-x1-2','s03-x1-3','s02-source','s02-claim')"
            )
        )
        windows = dict(
            connection.execute(
                "SELECT logical_id, json_array(valid_from,valid_until) FROM canonical_nodes"
                " WHERE logical_id IN ('s03-early','s03-late')"
            )
        )
        edge = connection.execute(
            "SELECT kind,from_id,to_id FROM canonical_edges WHERE logical_id='s02-edge'"
        ).fetchone()
    fixture = json.loads(FIXTURE.read_text())["corpus"]
    expected = {f"s03-x1-{index}": row["body"] for index, row in enumerate(fixture)}
    expected.update(
        {
            "s02-source": SOURCE_BODY,
            "s02-claim": '{"summary": "slice135s02evidenceneedle"}',
        }
    )
    if integrity != "ok" or nodes != size + 9 or edges != 1 or memory != size:
        raise ValueError("reopened SQLite counts or integrity differ from fixture")
    if rows != expected or windows != {
        "s03-early": "[1000,2000]",
        "s03-late": "[3000,null]",
    }:
        raise ValueError(
            "reopened SQLite bodies or validity windows differ from fixture"
        )
    if edge != ("supports", "s02-root", "s02-claim"):
        raise ValueError("reopened SQLite graph edge differs from fixture")
    return {
        "integrity_check": integrity,
        "nodes": nodes,
        "edges": edges,
        "memory_nodes": memory,
    }


def audit(
    directory: Path, *, size: int, source_sha: str, wheel_sha256: str
) -> dict[str, Any]:
    """Recompute one pilot receipt from raw samples and the retained database."""
    raw_path = directory / "raw.json"
    raw = json.loads(raw_path.read_text())
    if (
        raw.get("schema_version") != 1
        or raw.get("status") != "S03_FUNCTIONAL_PILOT_NOT_FROZEN_COMPARISON"
    ):
        raise ValueError("S03 pilot schema or status mismatch")
    if (
        raw.get("source_sha") != source_sha
        or raw.get("artifact", {}).get("wheel_sha256") != wheel_sha256
    ):
        raise ValueError("source or wheel identity mismatch")
    if raw.get("size") != size:
        raise ValueError("corpus size mismatch")
    expected_hashes = {
        str(path.relative_to(ROOT)): digest(path)
        for path in (RUNNER, FIXTURE, S01, S02)
    }
    if raw.get("input_sha256") != expected_hashes:
        raise ValueError("runner or source fixture bytes changed")
    database = _check_database(directory / "s03.sqlite", size)
    if raw.get("reopened_sqlite") != {
        key: database[key] for key in ("integrity_check", "nodes", "edges")
    }:
        raise ValueError("producer reopened-state claim differs from database")
    expected_cases = {
        "filter",
        "temporal_early",
        "temporal_boundary",
        "temporal_late",
        "graph",
        "evidence",
        f"memory_{size}",
    }
    if set(raw.get("cases", {})) != expected_cases:
        raise ValueError("S03 case set differs from protocol draft")
    repetitions = raw.get("repetitions")
    if type(repetitions) is not int or repetitions < 1:
        raise ValueError("invalid repetition count")
    case_summary: dict[str, dict[str, int | str]] = {}
    for name, attempts in raw["cases"].items():
        if not isinstance(attempts, list) or len(attempts) != repetitions:
            raise ValueError(f"{name}: attempt count differs from declaration")
        durations = [check_attempt(name, attempt, size) for attempt in attempts]
        case_summary[name] = summarize_durations(durations)
    return {
        "status": "S03_FUNCTIONAL_FEASIBILITY_AUDITED_NOT_TIMING_COMPARISON",
        "source_sha": source_sha,
        "wheel_sha256": wheel_sha256,
        "size": size,
        "raw_sha256": digest(raw_path),
        "database_sha256": digest(directory / "s03.sqlite"),
        "reopened_sqlite": database,
        "cases": case_summary,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--size", type=int, choices=(32, 256), required=True)
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--wheel-sha256", required=True)
    args = parser.parse_args()
    print(
        json.dumps(
            audit(
                args.input,
                size=args.size,
                source_sha=args.source_sha,
                wheel_sha256=args.wheel_sha256,
            ),
            indent=2,
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
