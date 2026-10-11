#!/usr/bin/env python3
"""Run a source-bound S03 fixture pilot through one installed Python wheel."""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import sqlite3
from time import perf_counter_ns
from typing import Any

import slice135_python_s01 as s01
import slice135_python_s02 as s02


MEMORY_SOURCE = s01.SOURCE_ID
SOURCE_BODY = s02.SOURCE_BODY
FIXTURE = (
    Path(__file__).resolve().parents[1]
    / "src/python/tests/functional_search_fixture.json"
)


def digest(path: Path) -> str:
    """Return the digest of one input's exact bytes."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def make_seed(size: int) -> list[dict[str, Any]]:
    """Combine existing source-grounded shapes on one fresh database."""
    if size not in (32, 256):
        raise ValueError("S03 size must be 32 or 256")
    fixture = json.loads(FIXTURE.read_text())
    search = [
        {
            "kind": row["kind"],
            "body": row["body"],
            "logical_id": f"s03-x1-{index}",
            "source_id": "slice135-s03-x1",
        }
        for index, row in enumerate(fixture["corpus"])
    ]
    temporal = [
        {
            "kind": "doc",
            "body": "epoch alpha record",
            "logical_id": "s03-early",
            "source_id": "slice135-s03-temporal",
            "valid_from": 1000,
            "valid_until": 2000,
        },
        {
            "kind": "doc",
            "body": "epoch beta record",
            "logical_id": "s03-late",
            "source_id": "slice135-s03-temporal",
            "valid_from": 3000,
        },
    ]
    return [*s01.make_corpus(size), *search, *temporal, *s02.make_graph_records()]


def check_case(name: str, observed: list[str], size: int) -> bool:
    """Check fixture intent rather than agreement between product versions."""
    fixed = {
        "filter": ["alpha structured retrieval document"],
        "temporal_early": ["epoch alpha record"],
        "temporal_boundary": [],
        "temporal_late": ["epoch beta record"],
        "graph": ["s02-claim"],
        "evidence": [SOURCE_BODY],
    }
    if name in fixed:
        return observed == fixed[name]
    if name == f"memory_{size}":
        return (
            bool(observed)
            and len(observed) <= 10
            and all(
                item.startswith("F") and item[1:].isdigit() and int(item[1:]) < size - 2
                for item in observed
            )
        )
    return False


def _observe(engine: Any, name: str, size: int, frozen: Any) -> list[str]:
    import fathomdb

    if name == "filter":
        return [
            hit.body
            for hit in engine.search(
                "retrieval", fathomdb.SearchFilter(kind="note")
            ).results
        ]
    if name.startswith("temporal_"):
        instant = {
            "temporal_early": 1500,
            "temporal_boundary": 2000,
            "temporal_late": 3000,
        }[name]
        return [
            hit.body
            for hit in engine.search_text_only(
                "epoch", view=fathomdb.ReadView(valid_as_of=instant)
            ).results
        ]
    if name == "graph":
        request = fathomdb.GraphExpandRequestV1(
            schema_version=1,
            seed=fathomdb.GraphExplicitSeedV1(
                schema_version=1,
                type="explicit",
                logical_ids=(fathomdb.IdSpace(space="logical", value="s02-root"),),
            ),
            direction="outgoing",
            edge_kinds=("supports",),
            target_kinds=("doc",),
            context=fathomdb.FrozenGraphReadContextV1(
                schema_version=1, type="frozen", context=frozen
            ),
            max_depth=1,
            result_limit=1,
            max_work_units="10",
            include_explanation=False,
            include_evidence=True,
        )
        return [
            target.logical_id
            for target in fathomdb.graph.expand(engine, request).targets
        ]
    if name == "evidence":
        result = engine.search_with_evidence(
            fathomdb.EvidenceSearchRequestV1(
                query=s02.CLAIM_TOKEN, context=frozen, limit=1
            )
        )
        if not result.evidence:
            return []
        resolved = engine.resolve_evidence(
            fathomdb.EvidenceResolveRequestV1(
                evidence_ref=result.evidence[0].evidence_ref, context=frozen
            )
        )
        return [resolved.canonical_source_body]
    if name == f"memory_{size}":
        return [
            hit.id.value
            for hit in engine.search_text_only("neutral archive", limit=10).results
        ]
    raise ValueError(f"unknown S03 cell {name}")


def run_pilot(
    *,
    size: int,
    repetitions: int,
    wheel: Path,
    wheel_sha256: str,
    source_sha: str,
    output: Path,
) -> dict[str, Any]:
    """Retain every materialized observation and an independently reopenable DB."""
    if repetitions < 1:
        raise ValueError("at least one repetition is required")
    import fathomdb

    identity = s01._installed_wheel_identity(wheel, wheel_sha256)
    records = make_seed(size)
    output.mkdir(parents=True, exist_ok=False)
    database = output / "s03.sqlite"
    engine = fathomdb.Engine.open(str(database), use_default_embedder=True)
    try:
        engine.write(records)
        engine.drain(timeout_s=120)
    finally:
        engine.close()
    names = (
        "filter",
        "temporal_early",
        "temporal_boundary",
        "temporal_late",
        "graph",
        "evidence",
        f"memory_{size}",
    )
    cases: dict[str, list[dict[str, Any]]] = {name: [] for name in names}
    engine = fathomdb.Engine.open(str(database), use_default_embedder=True)
    try:
        frozen = engine.freeze_read_context(fathomdb.ReadContextV1())
        for name in names:
            for _ in range(repetitions):
                started = perf_counter_ns()
                observed = _observe(engine, name, size, frozen)
                elapsed = perf_counter_ns() - started
                cases[name].append(
                    {
                        "latency_ns": elapsed,
                        "observed": observed,
                        "semantic_ok": check_case(name, observed, size),
                    }
                )
    finally:
        engine.close()
    with sqlite3.connect(f"file:{database}?mode=ro", uri=True) as connection:
        integrity = connection.execute("PRAGMA integrity_check").fetchone()[0]
        node_count = connection.execute(
            "SELECT count(*) FROM canonical_nodes"
        ).fetchone()[0]
        edge_count = connection.execute(
            "SELECT count(*) FROM canonical_edges"
        ).fetchone()[0]
    raw = {
        "schema_version": 1,
        "status": "S03_FUNCTIONAL_PILOT_NOT_FROZEN_COMPARISON",
        "started_utc": datetime.now(timezone.utc).isoformat(),
        "source_sha": source_sha,
        "artifact": identity,
        "size": size,
        "repetitions": repetitions,
        "input_sha256": {
            str(path.relative_to(Path(__file__).resolve().parents[1])): digest(path)
            for path in (
                Path(__file__),
                FIXTURE,
                Path(s01.__file__),
                Path(s02.__file__),
            )
        },
        "seed_sha256": hashlib.sha256(
            json.dumps(records, sort_keys=True).encode()
        ).hexdigest(),
        "cases": cases,
        "reopened_sqlite": {
            "integrity_check": integrity,
            "nodes": node_count,
            "edges": edge_count,
        },
    }
    (output / "raw.json").write_text(json.dumps(raw, indent=2, sort_keys=True) + "\n")
    return raw


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--size", type=int, choices=(32, 256), required=True)
    parser.add_argument("--repetitions", type=int, default=1)
    parser.add_argument("--wheel", type=Path, required=True)
    parser.add_argument("--wheel-sha256", required=True)
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    raw = run_pilot(
        size=args.size,
        repetitions=args.repetitions,
        wheel=args.wheel,
        wheel_sha256=args.wheel_sha256,
        source_sha=args.source_sha,
        output=args.output,
    )
    passed = all(
        attempt["semantic_ok"]
        for attempts in raw["cases"].values()
        for attempt in attempts
    )
    passed = passed and raw["reopened_sqlite"]["integrity_check"] == "ok"
    print(json.dumps({"passed": passed, "output": str(args.output), "size": args.size}))
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
