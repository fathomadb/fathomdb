#!/usr/bin/env python3
"""Seed-time connection sweep driver (dev/experiments/ac013-seed-connections/).

Runs a cell table (build mode x runtime/writer PRAGMAs x extra env) against
the arms `seed` (tests/exp_ac013_seed_connections.rs) and `gate`
(scripts/perf-experiments/run-ac013.sh, AC013_SCALE_TREATMENT=warm).
Every run takes an exclusive flock(2) on --lock (the same lock `flock(1)`
takes, shared with other agents measuring on this host), then waits inside
the lock for the 1-minute load to fall to --max-load, runs, and samples the
load once a second while running. Cells interleave: even repeats run the
table forward, odd repeats backward. One JSON record per run is appended to
<out>/results.jsonl; each run's raw log is kept under <out>.

  python3 dev/experiments/ac013-seed-connections/seed_sweep.py run \
      --cells dev/experiments/ac013-seed-connections/cells.json \
      --arms seed --repeats 2 --out $SCRATCH/seed-sweep \
      --lock $SCRATCH/../measure.lock --restored-target /tmp/target-mm \
      [--only A-c2000,A-c32000] [--repeat-offset 2]

  python3 dev/experiments/ac013-seed-connections/seed_sweep.py summarize \
      --out $SCRATCH/seed-sweep > results.json

Cells file: a JSON list of {"id", "mode": "absent"|"restored", "runtime":
"name=value,...", "writer": "name=value,...", "env": {...}}. A non-empty
PRAGMA list sets FATHOMDB_PERF_EXPERIMENTS=1 with
FATHOMDB_PERF_RUNTIME_PRAGMAS / FATHOMDB_PERF_WRITER_PRAGMAS. "restored" runs
cargo with LIBSQLITE3_FLAGS=-DSQLITE_ENABLE_MEMORY_MANAGEMENT and
CARGO_TARGET_DIR=<--restored-target>; build both targets first (README).
`env` sets AC013_SEED_MODE, AC013_VECTOR_DIM, AC013_RUNTIME_THREADS and the
like; the caller's environment (e.g. AC013_CORPUS_N) passes through unless a
cell overrides it.

A run is flagged `load_flag` when the load sampled during it, or right
after it, exceeds --flag-load (default 2.5).
"""

from __future__ import annotations

import argparse
import fcntl
import json
import os
import re
import subprocess
import sys
import threading
import time
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
RESTORE_FLAG = "-DSQLITE_ENABLE_MEMORY_MANAGEMENT"
CLEARED = ("FATHOMDB_PERF_EXPERIMENTS", "FATHOMDB_PERF_READER_PRAGMAS",
           "FATHOMDB_PERF_WRITER_PRAGMAS", "FATHOMDB_PERF_RUNTIME_PRAGMAS", "LIBSQLITE3_FLAGS")


def cell_order(cells: list[dict], repeat: int) -> list[dict]:
    """Even repeats run the table forward, odd repeats backward."""
    return list(cells) if repeat % 2 == 0 else list(reversed(cells))


def cell_env(cell: dict, base: dict, restored_target: str | None) -> dict:
    env = {k: v for k, v in base.items() if k not in CLEARED}
    if cell.get("runtime") or cell.get("writer"):
        env["FATHOMDB_PERF_EXPERIMENTS"] = "1"
        env["FATHOMDB_PERF_RUNTIME_PRAGMAS"] = cell.get("runtime", "")
        env["FATHOMDB_PERF_WRITER_PRAGMAS"] = cell.get("writer", "")
    env.update(cell.get("env", {}))
    if cell["mode"] == "restored":
        if not restored_target:
            raise SystemExit(f"cell {cell['id']} needs --restored-target")
        env["LIBSQLITE3_FLAGS"] = RESTORE_FLAG
        env["CARGO_TARGET_DIR"] = restored_target
    elif cell["mode"] != "absent":
        raise SystemExit(f"cell {cell['id']}: unknown mode {cell['mode']}")
    return env


# --- Parsers (tested in test_seed_sweep.py). -----------------------------------

def parse_seed(log: str) -> dict:
    m = re.search(r"AC013_SEED (\{.*\})", log)
    return json.loads(m[1]) if m else {}


def percentile(sorted_samples: list[int], pct: int) -> int:
    """The gate's percentile: the ceil(len * pct / 100)-th smallest sample."""
    index = max(-(-len(sorted_samples) * pct // 100) - 1, 0)
    return sorted_samples[index]


def parse_gate(log: str) -> dict:
    out: dict = {}
    m = re.search(r"AC013_NUMBERS n=(\d+) samples=(\d+) seed_ms=(\d+) p50_ms=(\d+) p99_ms=(\d+)",
                  log)
    if m:
        out.update(n=int(m[1]), samples=int(m[2]), seed_ms=int(m[3]),
                   p50_ms=int(m[4]), p99_ms=int(m[5]))
    m = re.search(r"AC013_TREATMENT_RECORD .*?seed_write_ms=(\d+) .*?projection_drain_ms=(\d+)"
                  r".*?samples_us=([\d,]+)", log)
    if m:
        samples = [int(s) for s in m[3].split(",")]
        ordered = sorted(samples)
        out.update(seed_write_ms=int(m[1]), drain_ms=int(m[2]),
                   p50_us=percentile(ordered, 50), p99_us=percentile(ordered, 99),
                   mean_us=sum(samples) / len(samples), samples_us=samples)
    return out


# --- Runner. -------------------------------------------------------------------

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


def arm_command(arm: str, env: dict, run_dir: Path) -> list[str]:
    if arm == "seed":
        return ["cargo", "test", "--release", "-p", "fathomdb-engine", "--features", "test-hooks",
                "--test", "exp_ac013_seed_connections", "--", "--ignored", "--nocapture",
                "--exact", "ac013_seed_connections_probe"]
    if arm == "gate":
        env["AC013_SCALE_TREATMENT"] = "warm"
        env["LOG_PATH"] = str(run_dir / "gate-tee.log")
        return ["bash", "scripts/perf-experiments/run-ac013.sh"]
    raise SystemExit(f"unknown arm {arm}")


PARSERS = {"seed": parse_seed, "gate": parse_gate}


def cmd_run(args) -> None:
    cells = json.loads(Path(args.cells).read_text())
    if args.only:
        keep = args.only.split(",")
        unknown = set(keep) - {c["id"] for c in cells}
        if unknown:
            raise SystemExit(f"unknown cells: {sorted(unknown)}")
        cells = [c for c in cells if c["id"] in keep]
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    progress = sys.stderr
    for repeat in range(args.repeat_offset, args.repeat_offset + args.repeats):
        for cell in cell_order(cells, repeat):
            for arm in args.arms.split(","):
                env = cell_env(cell, os.environ, args.restored_target)
                run_dir = out / f"{cell['id']}.{arm}.r{repeat}.n{env.get('AC013_CORPUS_N', '10000')}"
                run_dir.mkdir(exist_ok=True)
                cmd = arm_command(arm, env, run_dir)
                with open(args.lock, "a") as lock:
                    fcntl.flock(lock, fcntl.LOCK_EX)
                    load_before = wait_quiet(args.max_load, progress)
                    started = time.time()
                    print(f"[{time.strftime('%H:%M:%S')}] {cell['id']} {arm} r{repeat} "
                          f"load={load_before:.2f}", file=progress, flush=True)
                    code, text, peak = run_sampled(cmd, env, run_dir / "run.log", args.timeout)
                    load_after = load1()
                    fcntl.flock(lock, fcntl.LOCK_UN)
                record = {"cell": cell["id"], "mode": cell["mode"],
                          "runtime": cell.get("runtime", ""), "writer": cell.get("writer", ""),
                          "env": cell.get("env", {}),
                          "corpus_n": int(env.get("AC013_CORPUS_N", "10000")),
                          "arm": arm, "repeat": repeat, "exit": code,
                          "load_before": load_before, "load_peak": peak, "load_after": load_after,
                          "load_flag": max(peak, load_after) > args.flag_load,
                          "wall_s": time.time() - started, "started": started,
                          "result": PARSERS[arm](text)}
                with (out / "results.jsonl").open("a") as f:
                    f.write(json.dumps(record) + "\n")
                print(f"    -> exit={code} peak={peak:.2f} "
                      f"{json.dumps({k: v for k, v in record['result'].items() if k != 'samples_us'})[:240]}",
                      file=progress, flush=True)


def seed_keep(res: dict) -> dict:
    workers = res.get("worker_totals", {})
    dispatcher = [r for r in res.get("runtime", []) if r["role"] == "projection_dispatcher"]
    writer = res.get("writer", {})
    return {
        "write_ms": res.get("write_ms"), "drain_ms": res.get("drain_ms"),
        "seed_ms": res.get("seed_ms"),
        "worker_miss_per_vector": workers.get("miss_per_job"),
        "worker_miss_per_batch": workers.get("miss_per_round"),
        "worker_requests_per_vector": workers.get("requests_per_job"),
        "worker_batches": workers.get("rounds"),
        "worker_used_bytes": [r["cache_used_bytes"] for r in res.get("runtime", [])
                              if r["role"] == "projection_worker"],
        "dispatcher_miss_per_vector": (dispatcher[0]["cache_miss"] / res["n"]) if dispatcher else None,
        "writer_miss_per_row": writer.get("miss_per_row"),
        "writer_write_miss": writer.get("write_miss"), "writer_drain_miss": writer.get("drain_miss"),
        "runtime_cache_size": res.get("runtime_cache_size"),
        "db_bytes": res.get("store", {}).get("db_bytes"),
        "memory_management_compiled": res.get("memory_management_compiled"),
    }


def summarize(records: list[dict]) -> dict:
    """Group records by (cell, corpus N) and arm, keeping every repeat."""
    cells: dict = {}
    for r in records:
        key = r["cell"] if r["corpus_n"] == 10000 else f"{r['cell']}@n{r['corpus_n']}"
        cell = cells.setdefault(key, {"mode": r["mode"], "runtime": r["runtime"],
                                      "writer": r["writer"], "env": r["env"],
                                      "corpus_n": r["corpus_n"]})
        res = r["result"]
        if r["arm"] == "seed":
            keep = seed_keep(res)
        else:
            keep = {k: res.get(k) for k in ("p50_ms", "p99_ms", "p50_us", "p99_us", "seed_ms",
                                            "seed_write_ms", "drain_ms")}
        keep.update(repeat=r["repeat"], exit=r["exit"], load_before=r["load_before"],
                    load_peak=r["load_peak"], load_flag=r["load_flag"])
        cell.setdefault(r["arm"], []).append(keep)
    return cells


def cmd_summarize(args) -> None:
    lines = Path(args.out, "results.jsonl").read_text().splitlines()
    print(json.dumps(summarize([json.loads(l) for l in lines if l.strip()]), indent=1))


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = parser.add_subparsers(dest="command", required=True)
    r = sub.add_parser("run")
    r.add_argument("--cells", required=True)
    r.add_argument("--arms", default="seed")
    r.add_argument("--repeats", type=int, default=2)
    r.add_argument("--repeat-offset", type=int, default=0,
                   help="number the first repeat N (resumes a sweep; sets its order)")
    r.add_argument("--out", required=True)
    r.add_argument("--lock", required=True, help="flock(2) file shared with other measurers")
    r.add_argument("--only", help="comma-separated cell ids")
    r.add_argument("--restored-target")
    r.add_argument("--max-load", type=float, default=2.0)
    r.add_argument("--flag-load", type=float, default=2.5)
    r.add_argument("--timeout", type=int, default=7200)
    r.set_defaults(func=cmd_run)
    s = sub.add_parser("summarize")
    s.add_argument("--out", required=True)
    s.set_defaults(func=cmd_summarize)
    args = parser.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
