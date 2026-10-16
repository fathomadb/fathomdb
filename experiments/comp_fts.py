"""Paired local FTS comparison cells for COMP-01 / COMP-02 (side plan).

One cell is one product arm and one fresh-database repetition, executed in its
own subprocess so peak RSS belongs to that cell alone. A cell writes ordered
top-k IDs and per-query timings from the same calls; scoring never re-queries.

    python -m experiments.comp_fts run --config CONFIG --inputs DIR --out DIR
        [--cell ID] [--max-cells N]
    python -m experiments.comp_fts score --out DIR
"""

from __future__ import annotations

import argparse
import json
import math
import os
import platform
import random
import resource
import shutil
import statistics
import subprocess
import sys
import time
from pathlib import Path
from typing import Any

ORDERS = ("AB", "BA", "BA", "AB")
TOP_K = 10


def roster(arms: tuple[str, str], repetitions: int = 4) -> list[str]:
    """Cell IDs in execution order; arm A first in AB repetitions."""
    cells = []
    for rep in range(1, repetitions + 1):
        order = ORDERS[(rep - 1) % len(ORDERS)]
        for letter in order:
            cells.append(f"r{rep}-{arms[0] if letter == 'A' else arms[1]}")
    return cells


def pending(cells: list[str], out: Path) -> list[str]:
    return [cell for cell in cells if not (out / "cells" / f"{cell}.json").exists()]


# --- arms ------------------------------------------------------------------


class FathomArm:
    name = "fathomdb"

    def __init__(self) -> None:
        self.filtered = False

    def version(self) -> str:
        import fathomdb

        return fathomdb.__version__

    def settings(self) -> dict[str, Any]:
        if self.filtered:
            return {
                "method": "Engine.search_projected_text('text', SearchFilter(attributes=bench_cohort))",
                "projections": "text: SEARCHABLE+fts; bench_cohort: FILTERABLE",
                "embedder": "none",
            }
        return {
            "method": "Engine.search_text_only",
            "embedder": "none",
            "query_translation": "native (short-token/stopword removal, OR terms)",
            "dedup": "native body dedup before limit",
            "tokenizer": "FTS5 porter unicode61 remove_diacritics 2",
        }

    def create(self, root: Path) -> Any:
        from fathomdb import Engine, ProjectionRole, ProjectionSpec

        engine = Engine.open(str(root / "fathomdb.sqlite"), use_default_embedder=False)
        if self.filtered:
            engine.configure_projections([
                ProjectionSpec(name="text", roles=frozenset({ProjectionRole.SEARCHABLE}), fts=True),
                ProjectionSpec(name="bench_cohort", roles=frozenset({ProjectionRole.FILTERABLE})),
            ])
        return engine

    def insert(self, handle: Any, docs: list[dict[str, str]]) -> None:
        if self.filtered:
            bodies = [json.dumps({"text": d["body"], "bench_cohort": d["cohort"]}) for d in docs]
        else:
            bodies = [d["body"] for d in docs]
        handle.write([
            {"kind": "doc", "body": body, "logical_id": d["id"], "source_id": f"c:{d['id']}"}
            for d, body in zip(docs, bodies)
        ])

    def finish(self, handle: Any) -> None:
        handle.drain(timeout_s=1200)

    def build(self, root: Path, docs: list[dict[str, str]]) -> dict[str, float]:
        started = time.perf_counter()
        handle = self.create(root)
        self.insert(handle, docs)
        ack = time.perf_counter()
        self.finish(handle)
        built = time.perf_counter()
        self.close(handle)
        return {"ingest_ack_s": ack - started, "build_return_s": built - started}

    def open(self, root: Path) -> Any:
        from fathomdb import Engine

        return Engine.open(str(root / "fathomdb.sqlite"), use_default_embedder=False)

    def query(self, handle: Any, text: str, cohort: str | None = None, limit: int = TOP_K) -> list[str]:
        if self.filtered:
            from fathomdb import SearchFilter

            flt = SearchFilter(attributes=(("bench_cohort", cohort),)) if cohort else None
            hits = handle.search_projected_text(text, "text", flt, limit=limit).results
        else:
            hits = handle.search_text_only(text, limit=limit).results
        return [hit.id.value for hit in hits]

    def close(self, handle: Any) -> None:
        handle.close()

    def rss_mib(self, handle: Any) -> float:
        return resource.getrusage(resource.RUSAGE_SELF).ru_maxrss / 1024


class LanceArm:
    name = "lancedb"
    filtered = False

    def version(self) -> str:
        import lancedb

        return lancedb.__version__

    def settings(self) -> dict[str, Any]:
        return {
            "method": "table.search(query_type='fts')",
            "index": "native FTS (use_tantivy=False), library defaults",
            "tokenizer": "simple, lower_case, stem, remove_stop_words, ascii_folding",
        }

    def build(self, root: Path, docs: list[dict[str, str]]) -> dict[str, float]:
        import lancedb
        import pyarrow as pa

        started = time.perf_counter()
        db = lancedb.connect(str(root / "lance"))
        table = db.create_table(
            "docs",
            pa.table({"id": [d["id"] for d in docs], "body": [d["body"] for d in docs]}),
        )
        ack = time.perf_counter()
        table.create_fts_index("body", use_tantivy=False)
        table.wait_for_index(["body_idx"])
        built = time.perf_counter()
        return {"ingest_ack_s": ack - started, "build_return_s": built - started}

    def open(self, root: Path) -> Any:
        import lancedb

        return lancedb.connect(str(root / "lance")).open_table("docs")

    def query(self, handle: Any, text: str, cohort: str | None = None, limit: int = TOP_K) -> list[str]:
        rows = handle.search(text, query_type="fts").limit(limit).select(["id"]).to_list()
        return [row["id"] for row in rows]

    def close(self, handle: Any) -> None:
        pass

    def rss_mib(self, handle: Any) -> float:
        return resource.getrusage(resource.RUSAGE_SELF).ru_maxrss / 1024


SURREAL_DRIVER = Path(__file__).resolve().parent / "comp_drivers/surrealdb/target/release/comp-surrealdb-driver"


class _SurrealProcess:
    """One embedded SurrealDB process; ``ns`` is the engine-side call time."""

    def __init__(self, endpoint: str) -> None:
        self.proc = subprocess.Popen(
            [str(SURREAL_DRIVER)], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True, encoding="utf-8", bufsize=1
        )
        self.last_ns = 0
        self.call({"op": "open", "endpoint": endpoint})

    def call(self, request: dict[str, Any]) -> Any:
        assert self.proc.stdin and self.proc.stdout
        self.proc.stdin.write(json.dumps(request, ensure_ascii=False) + "\n")
        reply = json.loads(self.proc.stdout.readline())
        if not reply["ok"]:
            raise RuntimeError(f"surrealdb: {reply['error']}")
        self.last_ns = reply["ns"]
        return reply.get("result")

    def sql(self, sql: str, **vars: str) -> Any:
        return self.call({"op": "query", "sql": sql, "vars": vars})

    def hwm_mib(self) -> float:
        for line in Path(f"/proc/{self.proc.pid}/status").read_text().splitlines():
            if line.startswith("VmHWM:"):
                return int(line.split()[1]) / 1024
        return 0.0

    def close(self) -> None:
        self.call({"op": "close"})
        assert self.proc.stdin
        self.proc.stdin.close()
        self.proc.wait(timeout=120)


class SurrealArm:
    name = "surrealdb"
    # crud-bench's analyzer (src/adapters/surrealdb.rs), BM25, native OR match.
    SCHEMA = (
        "DEFINE ANALYZER bench TOKENIZERS blank,class FILTERS lowercase,ascii;"
        " DEFINE INDEX body_ft ON doc FIELDS body FULLTEXT ANALYZER bench BM25;"
    )

    def __init__(self) -> None:
        self.filtered = False

    def version(self) -> str:
        return "3.3.0 (embedded rocksdb, Rust crate)"

    def settings(self) -> dict[str, Any]:
        return {
            "method": "SELECT ... WHERE body @0,OR@ $q ORDER BY search::score(0) DESC LIMIT 10"
            + (" with AND cohort = $c (btree index)" if self.filtered else ""),
            "analyzer": "crud-bench: TOKENIZERS blank,class FILTERS lowercase,ascii; BM25",
            "storage": "rocksdb embedded, library defaults",
            "timing": "engine-side db.query call through materialized results",
        }

    def _endpoint(self, root: Path) -> str:
        return f"rocksdb:{root / 'surreal'}"

    def create(self, root: Path) -> Any:
        handle = _SurrealProcess(self._endpoint(root))
        handle.sql(self.SCHEMA)
        if self.filtered:
            handle.sql("DEFINE INDEX cohort_idx ON doc FIELDS cohort;")
        return handle

    def insert(self, handle: Any, docs: list[dict[str, str]]) -> None:
        rows = [{"id": d["id"], "body": d["body"], "cohort": d.get("cohort", "")} for d in docs]
        handle.sql(f"INSERT INTO doc {json.dumps(rows, ensure_ascii=False)} RETURN NONE;")

    def finish(self, handle: Any) -> None:
        deadline = time.monotonic() + 1200
        while time.monotonic() < deadline:
            info = handle.sql("INFO FOR INDEX body_ft ON doc;")[0]
            if info["building"]["status"] == "ready":
                return
            time.sleep(0.01)
        raise TimeoutError("surrealdb fulltext index not ready")

    def build(self, root: Path, docs: list[dict[str, str]]) -> dict[str, float]:
        started = time.perf_counter()
        handle = self.create(root)
        for index in range(0, len(docs), 256):
            self.insert(handle, docs[index:index + 256])
        ack = time.perf_counter()
        self.finish(handle)
        built = time.perf_counter()
        self.close(handle)
        return {"ingest_ack_s": ack - started, "build_return_s": built - started}

    def open(self, root: Path) -> Any:
        return _SurrealProcess(self._endpoint(root))

    def query(self, handle: Any, text: str, cohort: str | None = None, limit: int = TOP_K) -> list[str]:
        where = "body @0,OR@ $q" + (" AND cohort = $c" if cohort else "")
        vars = {"q": text, **({"c": cohort} if cohort else {})}
        rows = handle.sql(
            f"SELECT record::id(id) AS id, search::score(0) AS s FROM doc WHERE {where}"
            f" ORDER BY s DESC LIMIT {limit};",
            **vars,
        )[0]
        return [row["id"] for row in rows]

    def close(self, handle: Any) -> None:
        handle.close()

    def rss_mib(self, handle: Any) -> float:
        return handle.hwm_mib()


ARMS = {arm.name: arm for arm in (FathomArm(), LanceArm(), SurrealArm())}


# --- one cell (child process) ---------------------------------------------


def _disk_bytes(root: Path) -> int:
    return sum(p.stat().st_size for p in root.rglob("*") if p.is_file())


def run_cell(arm_name: str, rep: int, inputs: Path, workdir: Path) -> dict[str, Any]:
    """Build a fresh database, reopen it, and time one steady query pass."""
    arm = ARMS[arm_name]
    docs = json.loads((inputs / "corpus.json").read_text(encoding="utf-8"))
    queries = json.loads((inputs / "queries.json").read_text(encoding="utf-8"))
    if workdir.exists():
        shutil.rmtree(workdir)
    workdir.mkdir(parents=True)
    build = arm.build(workdir, docs)
    opened = time.perf_counter()
    handle = arm.open(workdir)
    reopen_s = time.perf_counter() - opened
    try:
        started = time.perf_counter_ns()
        first_ids = arm.query(handle, queries[0]["text"])
        first_ms = (time.perf_counter_ns() - started) / 1e6
        for q in queries:  # explicit untimed warm pass
            arm.query(handle, q["text"])
        order = list(range(len(queries)))
        random.Random(1000 + rep).shuffle(order)
        results = []
        for index in order:
            q = queries[index]
            started = time.perf_counter_ns()
            ids = arm.query(handle, q["text"])
            latency_ms = (time.perf_counter_ns() - started) / 1e6
            results.append({"query_id": q["id"], "ids": ids, "latency_ms": latency_ms})
    finally:
        arm.close(handle)
    disk = _disk_bytes(workdir)
    shutil.rmtree(workdir)
    return {
        "arm": arm_name,
        "version": arm.version(),
        "settings": arm.settings(),
        "repetition": rep,
        "seed": 1000 + rep,
        "top_k": TOP_K,
        "documents": len(docs),
        **build,
        "reopen_s": reopen_s,
        "first_query_ms": first_ms,
        "first_query_ids": first_ids,
        "disk_bytes": disk,
        "peak_rss_mib": resource.getrusage(resource.RUSAGE_SELF).ru_maxrss / 1024,
        "results": results,
    }


def _timed_pass(arm: Any, handle: Any, queries: list[dict[str, str]], order: list[int],
                cohort: str | None = None) -> list[dict[str, Any]]:
    out = []
    for index in order:
        q = queries[index]
        started = time.perf_counter_ns()
        ids = arm.query(handle, q["text"], cohort) if cohort else arm.query(handle, q["text"])
        wall_ms = (time.perf_counter_ns() - started) / 1e6
        engine_ms = handle.last_ns / 1e6 if isinstance(handle, _SurrealProcess) else wall_ms
        out.append({"query_id": q["id"], "ids": ids, "latency_ms": engine_ms, "wall_ms": wall_ms})
    return out


def _probe_until_ready(arm: Any, handle: Any, probes: list[dict[str, Any]], started: float,
                       timeout_s: float = 600) -> dict[str, Any]:
    attempts = 0
    while True:
        attempts += 1
        failing = [p["id"] for p in probes if p["id"] not in arm.query(handle, p["text"])]
        if not failing:
            return {"query_ready_s": time.perf_counter() - started, "probe_attempts": attempts, "probe_failing": []}
        if time.perf_counter() - started > timeout_s:
            return {"query_ready_s": None, "probe_attempts": attempts, "probe_failing": failing}
        time.sleep(0.005)


def run_scale_cell(arm_name: str, rep: int, inputs: Path, workdir: Path,
                   batch: int = 256) -> dict[str, Any]:
    """COMP-02a: batched ingest, true query-ready, reopen, cold/warm/steady passes."""
    arm = ARMS[arm_name]
    docs = json.loads((inputs / "corpus.json").read_text(encoding="utf-8"))
    queries = json.loads((inputs / "queries.json").read_text(encoding="utf-8"))
    probes = json.loads((inputs / "probes.json").read_text(encoding="utf-8"))
    if workdir.exists():
        shutil.rmtree(workdir)
    workdir.mkdir(parents=True)
    handle = arm.create(workdir)
    started = time.perf_counter()
    for index in range(0, len(docs), batch):
        arm.insert(handle, docs[index:index + batch])
    ingest_ack_s = time.perf_counter() - started
    arm.finish(handle)
    build_return_s = time.perf_counter() - started
    ready = _probe_until_ready(arm, handle, probes, started)
    build_rss = arm.rss_mib(handle)
    arm.close(handle)
    opened = time.perf_counter()
    handle = arm.open(workdir)
    reopen_s = time.perf_counter() - opened
    rng = random.Random(2000 + rep)
    try:
        started_q = time.perf_counter()
        first_ids = arm.query(handle, probes[-1]["text"])
        first_query_after_reopen_s = time.perf_counter() - started_q
        cold_order = list(range(len(queries)))
        rng.shuffle(cold_order)
        cold = _timed_pass(arm, handle, queries, cold_order)
        _timed_pass(arm, handle, queries, list(range(len(queries))))  # declared warmup
        steady = []
        for _ in range(10):
            order = list(range(len(queries)))
            rng.shuffle(order)
            steady.extend(_timed_pass(arm, handle, queries, order))
        query_rss = arm.rss_mib(handle)
    finally:
        arm.close(handle)
    disk = _disk_bytes(workdir)
    shutil.rmtree(workdir)
    return {
        "arm": arm_name, "version": arm.version(), "settings": arm.settings(), "repetition": rep,
        "documents": len(docs), "batch_size": batch, "ingest_ack_s": ingest_ack_s,
        "build_return_s": build_return_s, **ready, "sentinel_id": probes[-1]["id"],
        "sentinel_found_after_reopen": probes[-1]["id"] in first_ids,
        "reopen_s": reopen_s, "first_query_after_reopen_s": first_query_after_reopen_s,
        "disk_bytes": disk, "peak_rss_mib": max(build_rss, query_rss), "cold": cold, "steady": steady,
    }


def run_filtered_cell(arm_name: str, rep: int, inputs: Path, workdir: Path) -> dict[str, Any]:
    """COMP-02b: one fresh DB; steady filtered FTS at each cohort plus membership check."""
    arm = ARMS[arm_name]
    arm.filtered = True
    docs = json.loads((inputs / "corpus.json").read_text(encoding="utf-8"))
    queries = json.loads((inputs / "queries.json").read_text(encoding="utf-8"))
    cohort_of = {d["id"]: d["cohort"] for d in docs}
    if workdir.exists():
        shutil.rmtree(workdir)
    workdir.mkdir(parents=True)
    build = arm.build(workdir, docs)
    handle = arm.open(workdir)
    rng = random.Random(3000 + rep)
    passes: dict[str, Any] = {}
    try:
        for cohort in ("c01", "c10", "c33"):
            _timed_pass(arm, handle, queries, list(range(len(queries))), cohort)  # warmup
            order = list(range(len(queries)))
            rng.shuffle(order)
            results = _timed_pass(arm, handle, queries, order, cohort)
            passes[cohort] = {
                "matching": sum(c == cohort for c in cohort_of.values()),
                "violations": sum(cohort_of.get(i) != cohort for r in results for i in r["ids"]),
                "short_results": sum(len(r["ids"]) < TOP_K for r in results),
                "results": results,
            }
        rss = arm.rss_mib(handle)
    finally:
        arm.close(handle)
        arm.filtered = False
    disk = _disk_bytes(workdir)
    shutil.rmtree(workdir)
    return {
        "arm": arm_name, "version": arm.version(), "repetition": rep, "documents": len(docs),
        **build, "disk_bytes": disk, "peak_rss_mib": rss, "passes": passes,
    }


# --- scoring ---------------------------------------------------------------


def score_query(ids: list[str], judgments: dict[str, int], k: int = TOP_K) -> dict[str, float]:
    """nDCG@k (linear gain, as pytrec_eval/BEIR), Recall@k and MRR@k (grade > 0)."""
    ranked = ids[:k]
    dcg = sum(judgments.get(d, 0) / math.log2(i + 2) for i, d in enumerate(ranked))
    ideal = sorted(judgments.values(), reverse=True)[:k]
    idcg = sum(g / math.log2(i + 2) for i, g in enumerate(ideal))
    relevant = sum(g > 0 for g in judgments.values())
    found = sum(judgments.get(d, 0) > 0 for d in ranked)
    first = next((i for i, d in enumerate(ranked, 1) if judgments.get(d, 0) > 0), None)
    return {
        "ndcg_at_10": dcg / idcg if idcg else 0.0,
        "recall_at_10": found / relevant if relevant else 0.0,
        "mrr_at_10": 1 / first if first else 0.0,
    }


def _pct(values: list[float], fraction: float) -> float:
    ordered = sorted(values)
    return ordered[max(0, math.ceil(len(ordered) * fraction) - 1)]


def summarize_cell(cell: dict[str, Any], gold: dict[str, dict[str, int]], affected: set[str]) -> dict[str, Any]:
    scored = [score_query(r["ids"], gold[r["query_id"]]) for r in cell["results"]]
    lat = [r["latency_ms"] for r in cell["results"]]
    out: dict[str, Any] = {
        "arm": cell["arm"],
        "repetition": cell["repetition"],
        "queries": len(scored),
        "empty_results": sum(not r["ids"] for r in cell["results"]),
        "p50_ms": _pct(lat, 0.50),
        "p95_ms": _pct(lat, 0.95),
        "p99_ms": _pct(lat, 0.99),
        "first_query_ms": cell["first_query_ms"],
        "ingest_ack_s": cell["ingest_ack_s"],
        "build_return_s": cell["build_return_s"],
        "reopen_s": cell["reopen_s"],
        "disk_mib": cell["disk_bytes"] / 2**20,
        "peak_rss_mib": cell["peak_rss_mib"],
    }
    for metric in ("ndcg_at_10", "recall_at_10", "mrr_at_10"):
        out[metric] = statistics.fmean(s[metric] for s in scored)
        out[f"{metric}_unaffected"] = statistics.fmean(
            s[metric] for s, r in zip(scored, cell["results"]) if r["query_id"] not in affected
        )
    return out


def score(out: Path, inputs: Path) -> dict[str, Any]:
    gold = json.loads((inputs / "gold.json").read_text(encoding="utf-8"))
    manifest = json.loads((inputs / "manifest.json").read_text(encoding="utf-8"))
    affected = set(manifest["queries_with_relevant_duplicate_body"])
    cells = [json.loads(p.read_text(encoding="utf-8")) for p in sorted((out / "cells").glob("*.json"))]
    summaries = [summarize_cell(c, gold, affected) for c in cells]
    arms = sorted({s["arm"] for s in summaries})
    medians = {
        arm: {
            key: statistics.median(s[key] for s in summaries if s["arm"] == arm)
            for key in summaries[0]
            if key not in {"arm", "repetition"}
        }
        for arm in arms
    }
    report = {"cells": summaries, "median_by_arm": medians, "affected_queries": len(affected)}
    (out / "report.json").write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return report


# --- coordinator -----------------------------------------------------------

CAMPAIGNS = {
    "comp01": (("fathomdb", "lancedb"), "fts"),
    "comp02a": (("fathomdb", "surrealdb"), "scale"),
    "comp02b": (("fathomdb", "surrealdb"), "filtered"),
}
CELL_RUNNERS = {"fts": run_cell, "scale": run_scale_cell, "filtered": run_filtered_cell}


def _host() -> dict[str, Any]:
    cpu = platform.processor()
    for line in Path("/proc/cpuinfo").read_text().splitlines():
        if line.startswith("model name"):
            cpu = line.split(":", 1)[1].strip()
            break
    return {"cpu": cpu, "cores": os.cpu_count(), "python": sys.version.split()[0], "platform": platform.platform()}


def run(campaign: str, inputs: Path, out: Path, cell: str | None, max_cells: int | None) -> list[str]:
    arms, mode = CAMPAIGNS[campaign]
    cells = roster(arms)
    todo = pending(cells, out)
    if cell is not None:
        if cell not in cells:
            raise SystemExit(f"unknown cell {cell}; roster: {cells}")
        todo = [c for c in todo if c == cell]
    if max_cells is not None:
        todo = todo[:max_cells]
    (out / "cells").mkdir(parents=True, exist_ok=True)
    manifest_path = out / "manifest.json"
    manifest = (
        json.loads(manifest_path.read_text())
        if manifest_path.exists()
        else {"campaign": campaign, "roster": cells, "inputs": str(inputs), "host": _host(), "attempts": []}
    )
    for cell_id in todo:
        rep, arm = cell_id.split("-", 1)
        started = time.time()
        proc = subprocess.run(
            [sys.executable, "-m", "experiments.comp_fts", "_cell", "--mode", mode, "--arm", arm, "--rep", rep[1:],
             "--inputs", str(inputs), "--out", str(out)],
            capture_output=True, text=True,
        )
        status = "complete" if proc.returncode == 0 else "failed"
        manifest["attempts"].append(
            {"cell": cell_id, "status": status, "started": started, "elapsed_s": time.time() - started,
             "stderr_tail": proc.stderr[-2000:] if proc.returncode else ""}
        )
        manifest_path.write_text(json.dumps(manifest, indent=2) + "\n")
        print(f"{cell_id}: {status} ({time.time() - started:.1f}s)", flush=True)
    return todo


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    r = sub.add_parser("run")
    r.add_argument("--campaign", choices=sorted(CAMPAIGNS))
    r.add_argument("--config", type=Path, help="comp-fts.campaign.v1 file; verifies input digests")
    r.add_argument("--inputs", type=Path, required=True)
    r.add_argument("--out", type=Path, required=True)
    r.add_argument("--cell")
    r.add_argument("--max-cells", type=int)
    s = sub.add_parser("score")
    s.add_argument("--inputs", type=Path, required=True)
    s.add_argument("--out", type=Path, required=True)
    c = sub.add_parser("_cell")
    c.add_argument("--mode", choices=sorted(CELL_RUNNERS), default="fts")
    c.add_argument("--arm", choices=sorted(ARMS), required=True)
    c.add_argument("--rep", type=int, required=True)
    c.add_argument("--inputs", type=Path, required=True)
    c.add_argument("--out", type=Path, required=True)
    args = parser.parse_args(argv)
    if args.command == "run":
        campaign = args.campaign
        if args.config is not None:
            from experiments import comp_record

            config = comp_record.load_config(args.config)
            comp_record.verify_inputs(config, args.inputs)
            if campaign not in (None, config["harness_campaign"]):
                parser.error("--campaign disagrees with --config")
            campaign = config["harness_campaign"]
        if campaign is None:
            parser.error("--campaign or --config is required")
        run(campaign, args.inputs, args.out, args.cell, args.max_cells)
    elif args.command == "score":
        report = score(args.out, args.inputs)
        print(json.dumps(report["median_by_arm"], indent=2, sort_keys=True))
    else:
        result = CELL_RUNNERS[args.mode](args.arm, args.rep, args.inputs, args.out / "work" / f"r{args.rep}-{args.arm}")
        path = args.out / "cells" / f"r{args.rep}-{args.arm}.json"
        path.write_text(json.dumps(result) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
