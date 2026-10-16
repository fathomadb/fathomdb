"""10k TC-5 projected and body search latency, one FathomDB store (COMP-02b shape).

Builds one projected store (``text`` SEARCHABLE + FTS, ``bench_cohort``
FILTERABLE) from the COMP-02b inputs, then times each cell: one warm pass over
the queries, then one pass in a seeded shuffled order. Percentiles are the
diagnostic script's: p50 the median, p95 ``sorted[int(n * 0.95) - 1]``. Writes
``summary.json`` and an ``experiments.record.v1`` receipt with the engine
identity. No SurrealDB arm.

    python -m experiments.comp02b_latency run --inputs DIR --attempt-root DIR \\
        --engine-commit SHA [--base-dir DIR]
"""

from __future__ import annotations

import argparse
import hashlib
import json
import random
import statistics
import time
from datetime import UTC, datetime
from pathlib import Path
from typing import Any, Callable, Sequence

from experiments import _lib, comp_record

EXPERIMENT = "comp-02b-latency"
SHUFFLE_SEED = 7
CELLS = [
    {"id": "projected-unfiltered", "method": "search_projected_text", "cohort": None},
    {"id": "projected-c01", "method": "search_projected_text", "cohort": "c01"},
    {"id": "projected-c10", "method": "search_projected_text", "cohort": "c10"},
    {"id": "projected-c33", "method": "search_projected_text", "cohort": "c33"},
    {"id": "body-text-only", "method": "search_text_only", "cohort": None},
    {"id": "projected-single-term", "method": "search_projected_text", "cohort": None,
     "query": "meeting", "queries": 20},
]


def percentiles(timings_ms: Sequence[float]) -> dict[str, float]:
    ordered = sorted(timings_ms)
    return {"p50_ms": statistics.median(ordered), "p95_ms": ordered[int(len(ordered) * 0.95) - 1]}


def build(database: Path, corpus: list[dict[str, Any]]) -> float:
    from fathomdb import Engine, ProjectionRole, ProjectionSpec

    started = time.perf_counter()
    engine = Engine.open(str(database), use_default_embedder=False)
    try:
        engine.configure_projections([
            ProjectionSpec(name="text", roles=frozenset({ProjectionRole.SEARCHABLE}), fts=True),
            ProjectionSpec(name="bench_cohort", roles=frozenset({ProjectionRole.FILTERABLE})),
        ])
        engine.write([{"kind": "doc", "body": json.dumps({"text": d["body"], "bench_cohort": d["cohort"]}),
                       "logical_id": d["id"], "source_id": f"c:{d['id']}"} for d in corpus])
        engine.drain(timeout_s=1200)
    finally:
        engine.close()
    return time.perf_counter() - started


def _call(engine: Any, cell: dict[str, Any]) -> Callable[[str], Any]:
    from fathomdb import SearchFilter

    if cell["method"] == "search_text_only":
        return lambda q: engine.search_text_only(q, limit=10)
    flt = SearchFilter(attributes=(("bench_cohort", cell["cohort"]),)) if cell["cohort"] else None
    fixed = cell.get("query")
    return lambda q: engine.search_projected_text(fixed or q, "text", flt, limit=10)


def time_cell(engine: Any, cell: dict[str, Any], queries: list[str]) -> dict[str, Any]:
    texts = queries[: cell["queries"]] if "queries" in cell else list(queries)
    fn = _call(engine, cell)
    for text in texts:
        fn(text)
    order = list(texts)
    random.Random(SHUFFLE_SEED).shuffle(order)
    timings, short = [], 0
    for text in order:
        started = time.perf_counter()
        result = fn(text)
        timings.append((time.perf_counter() - started) * 1e3)
        short += len(result.results) < 10
    return {"id": cell["id"], "n": len(timings), **percentiles(timings), "short_results": short}


def run(inputs: Path, attempt_root: Path) -> dict[str, Any]:
    corpus = json.loads((inputs / "corpus.json").read_text(encoding="utf-8"))
    queries = [q["text"] for q in json.loads((inputs / "queries.json").read_text(encoding="utf-8"))]
    attempt_root.mkdir(parents=True, exist_ok=True)
    database = attempt_root / "fathomdb.sqlite"
    if database.exists():
        raise FileExistsError(f"store already exists: {database}")
    build_s = build(database, corpus)
    from fathomdb import Engine

    engine = Engine.open(str(database), use_default_embedder=False)
    try:
        cells = [time_cell(engine, cell, queries) for cell in CELLS]
    finally:
        engine.close()
    summary = {
        "schema_version": "comp-02b.latency.summary.v1",
        "documents": len(corpus), "queries": len(queries), "build_s": build_s,
        "inputs": {name: hashlib.sha256((inputs / name).read_bytes()).hexdigest()
                   for name in ("corpus.json", "queries.json")},
        "cells": {cell.pop("id"): cell for cell in cells},
    }
    (attempt_root / "summary.json").write_text(json.dumps(summary, indent=1, sort_keys=True) + "\n", encoding="utf-8")
    return summary


def write_receipt(summary: dict[str, Any], *, attempt_root: Path, engine: dict[str, str],
                  base_dir: Path | None) -> tuple[str, Path]:
    base = base_dir if base_dir is not None else _lib.EXPERIMENTS_DIR
    git = _lib.git_info()
    config = {"experiment": EXPERIMENT, "cells": CELLS, "shuffle_seed": SHUFFLE_SEED, "limit": 10,
              "inputs": summary["inputs"]}
    read = "; ".join(f"{cid} p50 {c['p50_ms']:.2f} ms p95 {c['p95_ms']:.2f} ms" for cid, c in summary["cells"].items())
    run_id, run_dir = _lib.write_record(
        EXPERIMENT, ts=datetime.now(UTC), config_obj=config, metrics=summary, verdict="complete", read=read,
        title="10k TC-5 projected and body search latency", n=summary["queries"],
        headline={cid: round(c["p50_ms"], 3) for cid, c in summary["cells"].items()},
        code={"git_sha": git.get("git_sha"), "dirty": git.get("dirty"), "branch": git.get("branch"),
              "baseline_commit": None},
        corpus={"source": "tc5-10000 (COMP-02b inputs)", "manifest_sha256": None,
                "datasets": [{"name": k, "sha256": v} for k, v in sorted(summary["inputs"].items())]},
        seeds={"shuffle": SHUFFLE_SEED},
        env=_lib.env_info(key_deps={"fathomdb": engine["version"]} | comp_record.engine_key_deps(engine)),
        cost_usd=0.0, config_path=None,
        artifacts=[{"kind": "external_attempt",
                    "summary_sha256": hashlib.sha256((attempt_root / "summary.json").read_bytes()).hexdigest()}],
        base_dir=base,
    )
    _lib.regen_index_md(index_path=base / "index.jsonl", md_path=base / "INDEX.md")
    return run_id, run_dir


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    cmd = sub.add_parser("run")
    cmd.add_argument("--inputs", type=Path, required=True)
    cmd.add_argument("--attempt-root", type=Path, required=True)
    cmd.add_argument("--engine-commit", required=True)
    cmd.add_argument("--base-dir", type=Path)
    args = parser.parse_args(argv)
    engine = comp_record.engine_identity(args.engine_commit)
    summary = run(args.inputs, args.attempt_root)
    run_id, run_dir = write_receipt(summary, attempt_root=args.attempt_root, engine=engine, base_dir=args.base_dir)
    print(json.dumps({"run_id": run_id, "receipt": str(run_dir / "record.json")}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
