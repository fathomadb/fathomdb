#!/usr/bin/env python3
"""Capture FathomDB 0.8.30 (schema 37) baseline latency and closed golden fixture DB.

Run from a bootstrapped checkout (.venv/bin/python) before schema 38 changes,
and re-run on the 0.8.31 candidate in Slice 60 for paired latency comparison.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import platform
import resource
import statistics
import subprocess
import tempfile
import time
from pathlib import Path

from fathomdb import (
    CurrentGraphReadContextV1,
    Engine,
    GraphExpandRequestV1,
    GraphExplicitSeedV1,
    IdSpace,
    ProjectionSpec,
    ReadContextV1,
    graph,
)


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def percentile(values: list[float], pct: float) -> float:
    ordered = sorted(values)
    idx = min(len(ordered) - 1, max(0, int(round((pct / 100.0) * (len(ordered) - 1)))))
    return ordered[idx]


def summarize(samples_ms: list[float]) -> dict[str, float]:
    return {
        "n": len(samples_ms),
        "p50_ms": round(statistics.median(samples_ms), 4),
        "p95_ms": round(percentile(samples_ms, 95.0), 4),
        "p99_ms": round(percentile(samples_ms, 99.0), 4),
        "min_ms": round(min(samples_ms), 4),
        "max_ms": round(max(samples_ms), 4),
    }


def build_golden_schema37(golden_db: Path, manifest_path: Path) -> dict[str, object]:
    golden_db.parent.mkdir(parents=True, exist_ok=True)
    for suffix in ("", "-wal", "-shm", ".lock"):
        p = Path(f"{golden_db}{suffix}")
        if p.exists():
            p.unlink()

    eng = Engine.open(str(golden_db))
    try:
        eng.configure_projections([
            ProjectionSpec(name="mms_tier", roles=frozenset({"filterable"})),
            ProjectionSpec(name="mms_scope", roles=frozenset({"filterable"})),
        ])
        eng.write([
            {
                "admin_schema": {
                    "name": "agent_checkpoints",
                    "kind": "latest_state",
                    "schema_json": '{"type":"object"}',
                    "retention_json": "{}",
                }
            },
            {
                "admin_schema": {
                    "name": "agent_audit",
                    "kind": "append_only_log",
                    "schema_json": '{"type":"object"}',
                    "retention_json": "{}",
                }
            },
        ])
        eng.write([
            {
                "kind": "note",
                "body": json.dumps({"content": "Initial design requirement R1", "mms_tier": "gold", "mms_scope": "core"}),
                "source_id": "src-keep-1",
                "logical_id": "req:1",
            },
            {
                "kind": "note",
                "body": json.dumps({"content": "Verification report V1 for R1", "mms_tier": "gold", "mms_scope": "core"}),
                "source_id": "src-keep-1",
                "logical_id": "ver:1",
            },
            {
                "kind": "note",
                "body": json.dumps({"content": "Transient secret note to erase", "mms_tier": "temp", "mms_scope": "private"}),
                "source_id": "src-erase-1",
                "logical_id": "secret:1",
            },
        ])
        # Supersede req:1 with v2 and add an active edge + superseded edge
        eng.write([
            {
                "kind": "note",
                "body": json.dumps({"content": "Updated design requirement R1 rev 2", "mms_tier": "gold", "mms_scope": "core"}),
                "source_id": "src-keep-2",
                "logical_id": "req:1",
            },
            {
                "edge": {
                    "kind": "verifies",
                    "from": "ver:1",
                    "to": "req:1",
                    "body": "Bench verification link v1",
                    "source_id": "src-keep-1",
                    "logical_id": "edge:verifies:1",
                    "confidence": 0.92,
                }
            },
            {
                "op_store": {
                    "collection": "agent_checkpoints",
                    "record_key": "step-1",
                    "body": '{"status":"done","revision":1}',
                }
            },
            {
                "op_store": {
                    "collection": "agent_audit",
                    "record_key": "evt-1",
                    "body": '{"event":"seeded"}',
                }
            },
        ])
        eng.write([
            {
                "edge": {
                    "kind": "verifies",
                    "from": "ver:1",
                    "to": "req:1",
                    "body": "Bench verification link v2 updated",
                    "source_id": "src-keep-2",
                    "logical_id": "edge:verifies:1",
                    "confidence": 0.98,
                }
            },
        ])
        erase_report = eng.erase_source("src-erase-1")
        open_rep = eng.open_report()
        schema_ver = open_rep.schema_version_after
    finally:
        eng.close()

    manifest = {
        "schema": "fathomdb.golden-fixture/v1",
        "producer_release": "0.8.30",
        "schema_version": schema_ver,
        "git_sha": subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(),
        "path": str(golden_db),
        "bytes": golden_db.stat().st_size,
        "sha256": sha256_file(golden_db),
        "expected": {
            "active_nodes": ["req:1", "ver:1"],
            "erased_source": "src-erase-1",
            "erased_logical_ids": ["secret:1"],
            "erased_nodes_count": erase_report.nodes_excised,
            "active_edge_logical_id": "edge:verifies:1",
            "operational_collections": {
                "agent_checkpoints": "latest_state",
                "agent_audit": "append_only_log",
            },
        },
    }
    manifest_path.parent.mkdir(parents=True, exist_ok=True)
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    return manifest


def run_latency_baseline(num_nodes: int, queries: int, out_path: Path, label: str) -> dict[str, object]:
    with tempfile.TemporaryDirectory(prefix="fathomdb-0831-bench-") as tmp:
        db_path = Path(tmp) / "bench.sqlite"
        t0 = time.perf_counter()
        eng = Engine.open(str(db_path))
        cold_open_ms = (time.perf_counter() - t0) * 1000.0
        try:
            eng.configure_projections([
                ProjectionSpec(name="tier", roles=frozenset({"filterable"})),
                ProjectionSpec(name="topic", roles=frozenset({"filterable"})),
            ])
            # Write nodes in batches of 200
            write_batch_ms: list[float] = []
            topics = ("alpha", "beta", "gamma", "delta", "epsilon")
            tiers = ("gold", "silver", "bronze")
            batch_size = 200
            for start in range(0, num_nodes, batch_size):
                batch = []
                for i in range(start, min(start + batch_size, num_nodes)):
                    topic = topics[i % len(topics)]
                    tier = tiers[i % len(tiers)]
                    batch.append({
                        "kind": "note",
                        "body": json.dumps({
                            "content": f"Document {i} discusses {topic} architecture verification metric_{i % 97} benchmark",
                            "tier": tier,
                            "topic": topic,
                        }),
                        "source_id": f"src-{i % 50}",
                        "logical_id": f"node:{i}",
                    })
                tw0 = time.perf_counter()
                eng.write(batch)
                write_batch_ms.append((time.perf_counter() - tw0) * 1000.0)

            # Write edges (1.5x nodes) referencing active endpoints
            edge_batch_ms: list[float] = []
            edge_items = []
            for i in range(num_nodes):
                edge_items.append({
                    "edge": {
                        "kind": "relates_to",
                        "from": f"node:{i}",
                        "to": f"node:{(i + 1) % num_nodes}",
                        "source_id": f"src-{i % 50}",
                        "logical_id": f"edge:a:{i}",
                        "confidence": round(0.5 + (i % 50) * 0.01, 2),
                    }
                })
                if i % 2 == 0:
                    edge_items.append({
                        "edge": {
                            "kind": "verifies",
                            "from": f"node:{i}",
                            "to": f"node:{(i + 7) % num_nodes}",
                            "source_id": f"src-{i % 50}",
                            "logical_id": f"edge:b:{i}",
                            "confidence": 0.95,
                        }
                    })
            for start in range(0, len(edge_items), batch_size):
                tw0 = time.perf_counter()
                eng.write(edge_items[start : start + batch_size])
                edge_batch_ms.append((time.perf_counter() - tw0) * 1000.0)
        finally:
            eng.close()

        # Warm reopen timing (5 iterations)
        warm_open_ms: list[float] = []
        for _ in range(5):
            to0 = time.perf_counter()
            e2 = Engine.open(str(db_path))
            warm_open_ms.append((time.perf_counter() - to0) * 1000.0)
            e2.close()

        eng = Engine.open(str(db_path))
        try:
            def make_expand_req(logical_id: str) -> GraphExpandRequestV1:
                return GraphExpandRequestV1(
                    schema_version=1,
                    seed=GraphExplicitSeedV1(
                        schema_version=1,
                        type="explicit",
                        logical_ids=(IdSpace(space="logical", value=logical_id),),
                    ),
                    direction="outgoing",
                    edge_kinds=("relates_to", "verifies"),
                    target_kinds=(),
                    context=CurrentGraphReadContextV1(
                        schema_version=1,
                        type="current",
                        context=ReadContextV1(),
                    ),
                    max_depth=2,
                    result_limit=20,
                    max_work_units="5000",
                    include_explanation=False,
                    include_evidence=False,
                )

            # Warmup queries
            for q in range(10):
                eng.search_text_only(f"metric_{q} benchmark", limit=10)
                eng.search(f"metric_{q} architecture", limit=10)
                graph.expand(eng, make_expand_req(f"node:{q}"))

            fts_ms: list[float] = []
            hybrid_ms: list[float] = []
            expand_ms: list[float] = []
            for q in range(queries):
                term = f"metric_{q % 97} benchmark"
                tf0 = time.perf_counter()
                res_fts = eng.search_text_only(term, limit=10)
                fts_ms.append((time.perf_counter() - tf0) * 1000.0)
                assert len(res_fts.results) > 0

                th0 = time.perf_counter()
                res_hyb = eng.search(term, limit=10)
                hybrid_ms.append((time.perf_counter() - th0) * 1000.0)
                assert len(res_hyb.results) > 0

                tg0 = time.perf_counter()
                res_exp = graph.expand(eng, make_expand_req(f"node:{q % num_nodes}"))
                expand_ms.append((time.perf_counter() - tg0) * 1000.0)
                assert len(res_exp.targets) > 0

            schema_ver = eng.open_report().schema_version_after
        finally:
            eng.close()

        db_bytes = db_path.stat().st_size
        peak_rss_kb = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss

    result = {
        "schema": "fathomdb.0831-latency-baseline/v1",
        "label": label,
        "git_sha": subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(),
        "schema_version": schema_ver,
        "host": {
            "platform": platform.platform(),
            "machine": platform.machine(),
            "python": platform.python_version(),
        },
        "workload": {
            "num_nodes": num_nodes,
            "num_edges": num_nodes + (num_nodes + 1) // 2,
            "queries": queries,
        },
        "metrics": {
            "cold_open_ms": round(cold_open_ms, 4),
            "warm_open": summarize(warm_open_ms),
            "write_node_batch_200": summarize(write_batch_ms),
            "write_edge_batch_200": summarize(edge_batch_ms),
            "search_text_only": summarize(fts_ms),
            "search_hybrid_cpu_no_embedder": summarize(hybrid_ms),
            "graph_expand_2hop": summarize(expand_ms),
            "db_bytes": db_bytes,
            "peak_rss_bytes": peak_rss_kb * 1024,
        },
    }
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    return result


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--label", default="0.8.30-baseline")
    parser.add_argument("--nodes", type=int, default=5000)
    parser.add_argument("--queries", type=int, default=100)
    parser.add_argument(
        "--golden-db",
        type=Path,
        default=Path("/mnt/data/worktrees/fathomdb-0.8.31-golden/golden-0.8.30-schema37.sqlite"),
    )
    parser.add_argument(
        "--golden-manifest",
        type=Path,
        default=Path("dev/plans/0.8.31/features/slice-0/golden-0.8.30-manifest.json"),
    )
    parser.add_argument(
        "--latency-out",
        type=Path,
        default=Path("dev/plans/0.8.31/features/slice-0/baseline-0.8.30-latency.json"),
    )
    parser.add_argument("--skip-golden", action="store_true")
    args = parser.parse_args()

    if not args.skip_golden:
        manifest = build_golden_schema37(args.golden_db, args.golden_manifest)
        print("GOLDEN:", json.dumps(manifest))
    res = run_latency_baseline(args.nodes, args.queries, args.latency_out, args.label)
    print("LATENCY:", json.dumps(res["metrics"]))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
