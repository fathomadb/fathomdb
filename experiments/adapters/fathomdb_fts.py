"""Reusable CPU FTS latency cell for the competitive experiment runner."""

from __future__ import annotations

import hashlib
import json
import math
import os
import platform
import random
import resource
import time
from pathlib import Path
from typing import Any

import fathomdb
from fathomdb import Engine

from experiments import _lib
from experiments.fathomdb_test_setup import prepare_test_database


def _digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _percentile(sorted_values: list[float], fraction: float) -> float:
    return sorted_values[max(0, math.ceil(len(sorted_values) * fraction) - 1)]


def _hardware() -> dict[str, Any]:
    cpu = platform.processor() or platform.machine()
    cpuinfo = Path("/proc/cpuinfo")
    if cpuinfo.is_file():
        for line in cpuinfo.read_text(encoding="utf-8").splitlines():
            if line.startswith("model name") and ":" in line:
                cpu = line.split(":", 1)[1].strip()
                break
    ram_gib = 1.0
    meminfo = Path("/proc/meminfo")
    if meminfo.is_file():
        for line in meminfo.read_text(encoding="utf-8").splitlines():
            if line.startswith("MemTotal:"):
                ram_gib = round(int(line.split()[1]) / (1024 * 1024), 3)
                break
    return {
        "cpu": cpu,
        "cores": os.cpu_count() or 1,
        "ram_gib": ram_gib,
        "storage": "local filesystem",
        "gpu": "none",
    }


def run(
    definition: dict[str, Any], inputs: dict[str, Path], artifact_root: Path
) -> dict[str, Any]:
    """Measure a fresh FathomDB FTS cell and return its aggregate result.

    The corpus is a JSON array of ``{id, body}`` objects and queries are a
    JSON array of strings. Raw inputs and the database remain external. The
    boundary covers ``Engine.search`` through materialized result objects.
    """
    workload = definition["workload"]
    measurement = definition["measurement"]
    settings = definition["settings"]
    if (
        definition["scoreboard"] != "latency_capacity"
        or workload["search_mode"] != "fts"
        or workload["filter"] != "none"
        or workload["top_k"] != 10
        or measurement["boundary"] != "Python Engine.search through materialized hits"
        or measurement["concurrency"] != 1
        or measurement["cache_state"] != "steady"
        or set(settings) != {"cli_path", "cli_sha256", "embed_device", "rerank_device"}
        or settings["embed_device"] != "cpu"
        or settings["rerank_device"] != "cpu"
    ):
        raise ValueError("unsupported FTS latency cell configuration")
    cli = Path(settings["cli_path"]).resolve(strict=True)
    if _digest(cli) != settings["cli_sha256"]:
        raise ValueError("FathomDB CLI digest drift")
    corpus = json.loads(inputs["corpus"].read_text(encoding="utf-8"))
    queries = json.loads(inputs["queries"].read_text(encoding="utf-8"))
    if (
        not isinstance(corpus, list)
        or len(corpus) != workload["items"]
        or not all(
            isinstance(row, dict)
            and set(row) == {"id", "body"}
            and isinstance(row["id"], str)
            and isinstance(row["body"], str)
            for row in corpus
        )
        or not isinstance(queries, list)
        or len(queries) != workload["queries"]
        or not all(isinstance(query, str) and query for query in queries)
    ):
        raise ValueError("corpus or query shape differs from the run definition")
    prepared = prepare_test_database(
        artifact_root,
        test_id="fathomdb-cell",
        embed_device="cpu",
        rerank_device="cpu",
        embedder="none",
        check_reranker=False,
        fathomdb_bin=str(cli),
    )
    engine = Engine.open(str(prepared.database_path), use_default_embedder=False)
    timings: list[float] = []
    hit_counts: list[int] = []
    query_indices: list[int] = []
    try:
        rows = [
            {
                "kind": "doc",
                "body": row["body"],
                "logical_id": row["id"],
                "source_id": f"competitive:{row['id']}",
            }
            for row in corpus
        ]
        ingest_started = time.perf_counter()
        engine.write(rows)
        engine.drain(timeout_s=30)
        ingest_s = time.perf_counter() - ingest_started
        for query in queries:
            _ = len(engine.search(query).results)
        rng = random.Random(measurement["seed"])
        measured_started = time.perf_counter()
        for _ in range(measurement["repetitions"]):
            order = list(range(len(queries)))
            rng.shuffle(order)
            for index in order:
                started = time.perf_counter_ns()
                hits = engine.search(queries[index]).results
                timings.append((time.perf_counter_ns() - started) / 1_000_000)
                hit_counts.append(len(hits))
                query_indices.append(index)
        measured_s = time.perf_counter() - measured_started
    finally:
        engine.close()
    ordered = sorted(timings)
    metrics = {
        "p50_ms": _percentile(ordered, 0.50),
        "p95_ms": _percentile(ordered, 0.95),
        "p99_ms": _percentile(ordered, 0.99),
        "queries_per_s": len(timings) / measured_s,
        "ingest_items_per_s": len(corpus) / ingest_s,
        "ready_to_search_ms": ingest_s * 1000,
        "peak_rss_mib": resource.getrusage(resource.RUSAGE_SELF).ru_maxrss / 1024,
        "storage_mib": sum(
            path.stat().st_size
            for path in prepared.database_path.parent.glob("fathomdb.sqlite*")
        )
        / (1024 * 1024),
    }
    summary = artifact_root / "summary.json"
    summary.write_text(
        json.dumps(
            {
                "schema_version": "competitive.fts-detail.v1",
                "latency_ms": timings,
                "hit_counts": hit_counts,
                "query_indices": query_indices,
                "percentile_method": "nearest_rank",
                "warmup_queries": len(queries),
                "measured_wall_s": measured_s,
                "cli_sha256": _digest(cli),
                "binding_sha256": _digest(Path(fathomdb._fathomdb.__file__)),
                "doctor_sha256": _digest(prepared.doctor_path),
            },
            indent=2,
        )
        + "\n",
        encoding="utf-8",
    )
    return {
        "schema_version": "competitive.result.v1",
        "case_id": definition["case_id"],
        "config_sha256": _lib.config_sha256(definition),
        "status": "complete",
        "observed": {
            "product_version": fathomdb.__version__,
            **_hardware(),
            "settings": settings,
            "workload": workload,
            "measurement": measurement,
            "queries_completed": len(timings),
            "errors": 0,
        },
        "metrics": metrics,
        "source": {"url": None, "locator": str(summary), "sha256": _digest(summary)},
        "comparability": "context_only",
        "reason": None,
    }
