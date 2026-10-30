#!/usr/bin/env python3
"""AC-013 drain sweep driver (dev/experiments/ac013-seed-drain/).

Runs tests/exp_ac013_seed_drain.rs for every (repeat, corpus N, cell) of a
cell table. Each run takes an exclusive flock(2) on --lock (shared with other
agents measuring on this host), waits inside the lock for the 1-minute load to
fall to --max-load, records the load before, the peak sampled each second
while running, and the load after, and flags the run when the peak or the
load after exceeds --flag-load. Cells interleave: even repeats run the table
forward, odd repeats backward. One JSON record per run is appended to
<out>/results.jsonl; each run's raw log is kept under <out>.

  python3 dev/experiments/ac013-seed-drain/drain_sweep.py run \\
      --cells dev/experiments/ac013-seed-drain/cells.json --out $S/drain/sweep \\
      --lock $S/measure.lock --n 10000,30000,100000 --repeats 2 \\
      [--only CS64,CS64-AL] [--repeat-offset 2] [--trace]

  python3 dev/experiments/ac013-seed-drain/drain_sweep.py summarize \\
      --out $S/drain/sweep > dev/experiments/ac013-seed-drain/results.json

  python3 dev/experiments/ac013-seed-drain/drain_sweep.py table results.json

Cells file: a JSON list of {"id", "env": {...}}. `env` is applied over the
caller's environment after clearing every FATHOMDB_PERF_* variable, so a cell
names all of its knobs. --trace sets FATHOMDB_PERF_PAGE_TRACE=all and marks
the record `traced` (attribution only: the tracer's callback costs time, so
timing comes from untraced runs). Build first so no run compiles:
`cargo test --release -p fathomdb-engine --features test-hooks --test
exp_ac013_seed_drain --no-run`.
"""

from __future__ import annotations

import argparse
import fcntl
import json
import os
import re
import statistics
import subprocess
import sys
import threading
import time
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]


def cell_order(cells: list[dict], repeat: int) -> list[dict]:
    """Even repeats run the table forward, odd repeats backward."""
    return list(cells) if repeat % 2 == 0 else list(reversed(cells))


def cell_env(cell: dict, base: dict, n: int, traced: bool) -> dict:
    env = {k: v for k, v in base.items() if not k.startswith("FATHOMDB_PERF_")}
    env.update(cell.get("env", {}))
    env["AC013_CORPUS_N"] = str(n)
    if traced:
        env["FATHOMDB_PERF_PAGE_TRACE"] = "all"
    return env


def parse_drain(log: str) -> dict:
    m = re.search(r"AC013_DRAIN (\{.*\})", log)
    return json.loads(m[1]) if m else {}


def load1() -> float:
    return float(Path("/proc/loadavg").read_text().split()[0])


def wait_quiet(max_load: float, log) -> float:
    while (load := load1()) > max_load:
        print(f"  load {load:.2f} > {max_load}; waiting", file=log, flush=True)
        time.sleep(15)
    return load


def run_sampled(cmd: list[str], env: dict, log_path: Path, timeout: int) -> tuple[int, str, float]:
    peak = [0.0]
    done = threading.Event()

    def sample() -> None:
        while not done.wait(1.0):
            peak[0] = max(peak[0], load1())

    sampler = threading.Thread(target=sample, daemon=True)
    sampler.start()
    try:
        proc = subprocess.run(cmd, cwd=REPO, env=env, capture_output=True, text=True,
                              timeout=timeout)
    finally:
        done.set()
        sampler.join()
    log_path.write_text(f"$ {' '.join(cmd)}\n--- stdout\n{proc.stdout}\n--- stderr\n{proc.stderr}")
    return proc.returncode, proc.stdout + proc.stderr, peak[0]


PROBE = ["cargo", "test", "--release", "-p", "fathomdb-engine", "--features", "test-hooks",
         "--test", "exp_ac013_seed_drain", "--", "--ignored", "--nocapture", "--exact",
         "ac013_seed_drain_probe"]


def cmd_run(args) -> None:
    cells = json.loads(Path(args.cells).read_text())
    if args.only:
        keep = args.only.split(",")
        unknown = set(keep) - {c["id"] for c in cells}
        if unknown:
            raise SystemExit(f"unknown cells: {sorted(unknown)}")
        cells = [c for c in cells if c["id"] in keep]
    ns = [int(x) for x in args.n.split(",")]
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    progress = sys.stderr
    for repeat in range(args.repeat_offset, args.repeat_offset + args.repeats):
        for n in ns:
            for cell in cell_order(cells, repeat):
                env = cell_env(cell, os.environ, n, args.trace)
                tag = "trace" if args.trace else "time"
                run_dir = out / f"{cell['id']}.n{n}.{tag}.r{repeat}"
                run_dir.mkdir(exist_ok=True)
                with open(args.lock, "a") as lock:
                    fcntl.flock(lock, fcntl.LOCK_EX)
                    load_before = wait_quiet(args.max_load, progress)
                    started = time.time()
                    print(f"[{time.strftime('%H:%M:%S')}] {cell['id']} n={n} {tag} r{repeat} "
                          f"load={load_before:.2f}", file=progress, flush=True)
                    code, text, peak = run_sampled(PROBE, env, run_dir / "run.log", args.timeout)
                    load_after = load1()
                    fcntl.flock(lock, fcntl.LOCK_UN)
                result = parse_drain(text)
                record = {"cell": cell["id"], "env": cell.get("env", {}), "n": n,
                          "traced": args.trace, "repeat": repeat, "exit": code,
                          "load_before": load_before, "load_peak": peak, "load_after": load_after,
                          "load_flag": max(peak, load_after) > args.flag_load,
                          "wall_s": time.time() - started, "started": started, "result": result}
                with (out / "results.jsonl").open("a") as f:
                    f.write(json.dumps(record) + "\n")
                print(f"    -> exit={code} peak={peak:.2f} after={load_after:.2f} "
                      f"drain_ms={result.get('drain_ms')} seed_ms={result.get('seed_ms')}",
                      file=progress, flush=True)


def keep_fields(rec: dict) -> dict:
    res = rec["result"]
    phase = res.get("phase", {})
    digest = res.get("digest", {})
    keep = {
        "repeat": rec["repeat"], "traced": rec["traced"], "exit": rec["exit"],
        "load_before": rec["load_before"], "load_peak": rec["load_peak"],
        "load_after": rec["load_after"], "load_flag": rec["load_flag"],
        "write_ms": res.get("write_ms"), "drain_ms": res.get("drain_ms"),
        "seed_ms": res.get("seed_ms"),
        "drain_us_per_vector": (res["drain_ms"] * 1000 / res["n"]) if res.get("n") else None,
        "phase": {k: phase.get(k) for k in (
            "dispatcher_scans", "dispatcher_scan_ms", "dispatcher_rows_seen", "worker_batches",
            "worker_embed_ms", "worker_commit_ms", "worker_gate_wait_ms", "drain_probes",
            "drain_probe_ms", "drain_probe_pending")},
        "writer_drain_miss": res.get("writer", {}).get("drain_miss"),
        "writer_drain_hit": res.get("writer", {}).get("drain_hit"),
        "runtime": res.get("runtime"),
        "digest": {
            "vector_rows": digest.get("vector_rows"),
            "vector_default": digest.get("vector_default"),
            "terminal": digest.get("terminal"),
            "projection_cursor": dict(digest.get("open_state", [])).get("projection_cursor"),
        },
        "corpus_digest": res.get("corpus_digest"),
    }
    if rec["traced"]:
        keep["trace_by_role"] = res.get("trace_by_role")
        keep["trace_top"] = [
            {k: row[k] for k in ("role", "sql", "runs", "exclusive_hit", "exclusive_miss",
                                 "exclusive_ms", "requests_per_run")}
            for row in res.get("trace", [])[:12]]
        keep["query_plans"] = res.get("query_plans")
    if res.get("timeline"):
        keep["timeline"] = res["timeline"]
    return keep


def summarize(records: list[dict]) -> dict:
    """Group by cell@n, keeping every run in order."""
    out: dict = {}
    for rec in records:
        key = f"{rec['cell']}@n{rec['n']}"
        cell = out.setdefault(key, {"cell": rec["cell"], "n": rec["n"], "env": rec["env"],
                                    "runs": []})
        cell["runs"].append(keep_fields(rec))
    return out


def cmd_summarize(args) -> None:
    lines = Path(args.out, "results.jsonl").read_text().splitlines()
    print(json.dumps(summarize([json.loads(line) for line in lines if line.strip()]), indent=1))


def median(values: list[float]) -> float | None:
    values = [v for v in values if v is not None]
    return statistics.median(values) if values else None


def cmd_table(args) -> None:
    data = json.loads(Path(args.results).read_text())
    print("| Cell | N | Write ms | Drain ms (each timed run) | Drain us/vector (median) "
          "| Dispatcher scan ms | Drain probes | Load before/peak |")
    print("| --- | --- | --- | --- | --- | --- | --- | --- |")
    for key, cell in data.items():
        timed = [r for r in cell["runs"] if not r["traced"] and r["exit"] == 0]
        if not timed:
            continue
        drains = " / ".join(f"{r['drain_ms']:,.0f}{'*' if r['load_flag'] else ''}" for r in timed)
        writes = " / ".join(f"{r['write_ms']:,.0f}" for r in timed)
        per = median([r["drain_us_per_vector"] for r in timed])
        scan = " / ".join(f"{r['phase']['dispatcher_scan_ms']:,.0f}" for r in timed)
        probes = " / ".join(str(r["phase"]["drain_probes"]) for r in timed)
        loads = " / ".join(f"{r['load_before']:.1f}-{r['load_peak']:.1f}" for r in timed)
        print(f"| {cell['cell']} | {cell['n']:,} | {writes} | {drains} | {per:,.1f} | {scan} "
              f"| {probes} | {loads} |")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = parser.add_subparsers(dest="command", required=True)
    r = sub.add_parser("run")
    r.add_argument("--cells", required=True)
    r.add_argument("--out", required=True)
    r.add_argument("--lock", required=True, help="flock(2) file shared with other measurers")
    r.add_argument("--n", default="10000", help="comma-separated corpus sizes")
    r.add_argument("--repeats", type=int, default=2)
    r.add_argument("--repeat-offset", type=int, default=0,
                   help="number the first repeat N (resumes a sweep; sets its order)")
    r.add_argument("--only", help="comma-separated cell ids")
    r.add_argument("--trace", action="store_true", help="FATHOMDB_PERF_PAGE_TRACE=all")
    r.add_argument("--max-load", type=float, default=2.0)
    r.add_argument("--flag-load", type=float, default=2.5)
    r.add_argument("--timeout", type=int, default=3600)
    r.set_defaults(func=cmd_run)
    s = sub.add_parser("summarize")
    s.add_argument("--out", required=True)
    s.set_defaults(func=cmd_summarize)
    t = sub.add_parser("table")
    t.add_argument("results")
    t.set_defaults(func=cmd_table)
    args = parser.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
