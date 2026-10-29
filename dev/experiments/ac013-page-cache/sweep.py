#!/usr/bin/env python3
"""Page-cache sweep driver for the AC gates (dev/experiments/ac013-page-cache/).

Runs a cell table (page-cache build mode x per-connection PRAGMAs) against
one or more measurement arms, interleaving cells across repeats and waiting
for a quiet host before each run. Appends one JSON record per run to
<out>/results.jsonl and keeps each run's raw log next to it.

  python3 dev/experiments/ac013-page-cache/sweep.py run \
      --cells dev/experiments/ac013-page-cache/cells.json \
      --arms gate,probe --repeats 2 --out /tmp/ac013-sweep \
      --restored-target /tmp/target-mm [--mcp-store /tmp/ac013-bge.sqlite] \
      [--tc5-inputs ~/.cache/fathomdb-comp/tc5-10000] [--only A-r2000,R-r2000]

  python3 dev/experiments/ac013-page-cache/sweep.py summarize \
      --out /tmp/ac013-sweep > summary.json

Cells file: a JSON list of {"id", "mode": "absent"|"restored", "reader":
"name=value,...", "writer": "name=value,..."}. "restored" runs cargo with
LIBSQLITE3_FLAGS=-DSQLITE_ENABLE_MEMORY_MANAGEMENT and
CARGO_TARGET_DIR=<--restored-target>; build that target first (README).
Non-empty PRAGMA lists set FATHOMDB_PERF_EXPERIMENTS=1 and
FATHOMDB_PERF_READER_PRAGMAS / FATHOMDB_PERF_WRITER_PRAGMAS.

Arms (each runs once per cell per repeat):
  gate   scripts/perf-experiments/run-ac013.sh, AC013_SCALE_TREATMENT=warm;
         records p50/p99/seed and seed write vs drain.
  probe  tests/exp_ac013_page_cache.rs; records its AC013_PROBE JSON.
  mcp    examples/ac013_mcp mcp <store> at concurrency 1 only
         (FATHOMDB_MCP_LOG=warn); records the c1 phase.
  i6     slice40 reader_page_cache_timing_report (REQ40-I6 clause i) on the
         tc5-10000 pack; records per-route after/before p50 ratios.
  iso    slice40 AC40-96/AC40-97 counter tests; records pass/fail.

Env passed through: AC013_CORPUS_N, AC013_VECTOR_DIM, AC013_SAMPLES and
anything else in the caller's environment. --max-load (default 2.0) is the
1-minute load average a run waits for.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
RESTORE_FLAG = "-DSQLITE_ENABLE_MEMORY_MANAGEMENT"


def cell_order(cells: list[dict], repeat: int) -> list[dict]:
    """Even repeats run the table forward, odd repeats backward."""
    return list(cells) if repeat % 2 == 0 else list(reversed(cells))


def cell_env(cell: dict, base: dict, restored_target: str | None) -> dict:
    env = dict(base)
    for key in ("FATHOMDB_PERF_EXPERIMENTS", "FATHOMDB_PERF_READER_PRAGMAS",
                "FATHOMDB_PERF_WRITER_PRAGMAS", "LIBSQLITE3_FLAGS"):
        env.pop(key, None)
    if cell.get("reader") or cell.get("writer"):
        env["FATHOMDB_PERF_EXPERIMENTS"] = "1"
        env["FATHOMDB_PERF_READER_PRAGMAS"] = cell.get("reader", "")
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


def target_dir(env: dict) -> Path:
    return Path(env.get("CARGO_TARGET_DIR", REPO / "target"))


# --- Parsers (tested in test_sweep.py). ----------------------------------------

def parse_gate(log: str) -> dict:
    out: dict = {}
    m = re.search(r"^.*AC013_NUMBERS n=(\d+) samples=(\d+) seed_ms=(\d+) p50_ms=(\d+) p99_ms=(\d+)",
                  log, re.M)
    if m:
        out.update(n=int(m[1]), samples=int(m[2]), seed_ms=int(m[3]),
                   p50_ms=int(m[4]), p99_ms=int(m[5]))
    m = re.search(r"AC013_TREATMENT_RECORD .*?seed_write_ms=(\d+) .*?projection_drain_ms=(\d+)"
                  r".*?samples_us=([\d,]+)", log)
    if m:
        samples = sorted(int(s) for s in m[3].split(","))
        out.update(seed_write_ms=int(m[1]), drain_ms=int(m[2]),
                   p50_us=percentile(samples, 50), p99_us=percentile(samples, 99),
                   mean_us=sum(samples) / len(samples),
                   samples_us=[int(s) for s in m[3].split(",")])
    return out


def percentile(sorted_samples: list[int], pct: int) -> int:
    """The gate's percentile: the ceil(len * pct / 100)-th smallest sample."""
    index = max(-(-len(sorted_samples) * pct // 100) - 1, 0)
    return sorted_samples[index]


def parse_probe(log: str) -> dict:
    m = re.search(r"AC013_PROBE (\{.*\})", log)
    return json.loads(m[1]) if m else {}


def parse_mcp(stdout: str) -> dict:
    for line in stdout.splitlines():
        if line.startswith("{") and '"command":"mcp"' in line.replace(" ", ""):
            record = json.loads(line)
            for phase in record["phases"]:
                if phase["phase"] == "c1":
                    return phase
    return {}


I6_LINE = re.compile(r"unit-i route=(\S+) pass=\"([^\"]+)\" p50_ms=([\d.]+) p95_ms=([\d.]+)")


def parse_i6(log: str) -> dict:
    """REQ40-I6 (i): each after-c01 pass p50 over the second before pass p50."""
    passes: dict[str, dict[str, float]] = {}
    for route, label, p50, _ in I6_LINE.findall(log):
        passes.setdefault(route, {})[label] = float(p50)
    out = {}
    for route, by_label in passes.items():
        before = by_label.get("before 2")
        after = [v for k, v in by_label.items() if k.startswith("after c01")]
        if before and after:
            out[route] = {"before2_p50_ms": before, "after_p50_ms": after,
                          "max_ratio": max(after) / before}
    return out


def parse_iso(log: str) -> dict:
    return {name: status for name, status in
            re.findall(r"^test (\S+) \.\.\. (ok|FAILED)", log, re.M)}


# --- Runner. -------------------------------------------------------------------

def load1() -> float:
    return float(Path("/proc/loadavg").read_text().split()[0])


def wait_quiet(max_load: float, log) -> float:
    while (load := load1()) > max_load:
        print(f"  load {load:.2f} > {max_load}; waiting", file=log, flush=True)
        time.sleep(20)
    return load


def run(cmd: list[str], env: dict, log_path: Path, timeout: int = 3600) -> tuple[int, str, str]:
    proc = subprocess.run(cmd, cwd=REPO, env=env, capture_output=True, text=True, timeout=timeout)
    log_path.write_text(f"$ {' '.join(cmd)}\n--- stdout\n{proc.stdout}\n--- stderr\n{proc.stderr}")
    return proc.returncode, proc.stdout, proc.stderr


def arm_command(arm: str, env: dict, args, run_dir: Path) -> list[str]:
    cargo_test = ["cargo", "test", "--release", "-p", "fathomdb-engine", "--features", "test-hooks"]
    if arm == "gate":
        env["AC013_SCALE_TREATMENT"] = "warm"
        env["LOG_PATH"] = str(run_dir / "gate-tee.log")
        return ["bash", "scripts/perf-experiments/run-ac013.sh"]
    if arm in ("probe", "probeiso"):
        if arm == "probeiso":
            env["AC013_ISO"] = "1"
        return cargo_test + ["--test", "exp_ac013_page_cache", "--", "--ignored", "--nocapture"]
    if arm == "mcp":
        if not args.mcp_store:
            raise SystemExit("arm mcp needs --mcp-store")
        env["FATHOMDB_MCP_LOG"] = "warn"
        bin_dir = target_dir(env) / "release"
        return [str(bin_dir / "examples" / "ac013_mcp"), "mcp", args.mcp_store,
                str(bin_dir / "fathomdb-mcp"), str(run_dir / "mcp"), "-"]
    if arm == "i6":
        if not args.tc5_inputs:
            raise SystemExit("arm i6 needs --tc5-inputs")
        env["FATHOMDB_TC5_INPUTS"] = os.path.expanduser(args.tc5_inputs)
        return cargo_test + ["--test", "slice40_reader_page_cache", "--", "--ignored",
                             "--exact", "reader_page_cache_timing_report", "--nocapture"]
    if arm == "iso":
        return cargo_test + ["--test", "slice40_reader_page_cache", "--",
                             "a_heavy_read_stays_inside_its_own_page_cache",
                             "other_readers_keep_their_page_caches_after_a_heavy_read",
                             "--test-threads=1"]
    raise SystemExit(f"unknown arm {arm}")


PARSERS = {"gate": lambda o, e: parse_gate(o + e), "probe": lambda o, e: parse_probe(o + e),
           "probeiso": lambda o, e: parse_probe(o + e),
           "mcp": lambda o, e: parse_mcp(o), "i6": lambda o, e: parse_i6(o + e),
           "iso": lambda o, e: parse_iso(o + e)}


def cmd_run(args) -> None:
    cells = json.loads(Path(args.cells).read_text())
    if args.only:
        keep = set(args.only.split(","))
        cells = [c for c in cells if c["id"] in keep]
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    arms = args.arms.split(",")
    progress = sys.stderr
    for repeat in range(args.repeats):
        for cell in cell_order(cells, repeat + args.repeat_offset):
            for arm in arms:
                env = cell_env(cell, os.environ, args.restored_target)
                rep = repeat + args.repeat_offset
                run_dir = out / f"{cell['id']}.{arm}.r{rep}"
                run_dir.mkdir(exist_ok=True)
                cmd = arm_command(arm, env, args, run_dir)
                load_before = wait_quiet(args.max_load, progress)
                started = time.time()
                print(f"[{time.strftime('%H:%M:%S')}] {cell['id']} {arm} r{rep} "
                      f"load={load_before:.2f}", file=progress, flush=True)
                code, stdout, stderr = run(cmd, env, run_dir / "run.log")
                record = {"cell": cell["id"], "mode": cell["mode"],
                          "reader": cell.get("reader", ""), "writer": cell.get("writer", ""),
                          "arm": arm, "repeat": rep, "exit": code,
                          "load_before": load_before, "load_after": load1(),
                          "wall_s": time.time() - started, "started": started,
                          "result": PARSERS[arm](stdout, stderr)}
                with (out / "results.jsonl").open("a") as f:
                    f.write(json.dumps(record) + "\n")
                print(f"    -> exit={code} {json.dumps(record['result'])[:200]}",
                      file=progress, flush=True)


def summarize(records: list[dict]) -> dict:
    """Group records by cell and arm, keeping every repeat."""
    cells: dict = {}
    for r in records:
        cell = cells.setdefault(r["cell"], {"mode": r["mode"], "reader": r["reader"],
                                            "writer": r["writer"]})
        res = r["result"]
        if r["arm"] == "gate":
            keep = {k: res.get(k) for k in ("p50_ms", "p99_ms", "p50_us", "p99_us",
                                            "seed_ms", "seed_write_ms", "drain_ms")}
        elif r["arm"] == "probeiso":
            iso = res.get("isolation") or {}
            keep = {"before_p50_ms": iso.get("before", {}).get("p50_ms"),
                    "after_p50_ms": iso.get("after", {}).get("p50_ms"),
                    "p50_ratio": iso.get("p50_ratio"),
                    "after_miss_per_reader": iso.get("after", {}).get("cache_miss_per_reader"),
                    "heavy_reader": iso.get("heavy_reader"),
                    "vm_rss_kib": iso.get("after", {}).get("vm_rss_kib")}
        elif r["arm"] == "probe":
            keep = {"hybrid_p50_ms": res.get("hybrid", {}).get("p50_ms"),
                    "hybrid_miss_per_search": res.get("hybrid", {}).get("cache_miss_per_search"),
                    "hybrid_hit_per_search": res.get("hybrid", {}).get("cache_hit_per_search"),
                    "text_p50_ms": res.get("text_only", {}).get("p50_ms"),
                    "text_miss_per_search": res.get("text_only", {}).get("cache_miss_per_search"),
                    "vm_rss_kib": res.get("hybrid", {}).get("vm_rss_kib"),
                    "db_bytes": res.get("store", {}).get("db_bytes"),
                    "wal_frames": res.get("store_before_warmup", {}).get("wal_frames_approx"),
                    "cache_used_bytes_max": max(res.get("hybrid", {}).get(
                        "cache_used_bytes_per_reader", [0]) or [0]),
                    "seed_write_ms": res.get("seed_write_ms"), "drain_ms": res.get("drain_ms")}
        elif r["arm"] == "mcp":
            keep = {"p50_ms": res.get("ok", {}).get("p50_ms"),
                    "p99_ms": res.get("ok", {}).get("p99_ms"),
                    "rejections": res.get("rejections")}
        elif r["arm"] == "i6":
            keep = {route: round(v["max_ratio"], 3) for route, v in res.items()}
        else:
            keep = res
        keep.update(repeat=r["repeat"], load_before=r["load_before"], exit=r["exit"])
        cell.setdefault(r["arm"], []).append(keep)
    paired_against(records, cells, control="A-r2000")
    return cells


def per_query_median(records: list[dict], cell: str) -> list[float] | None:
    """Per-query median latency (µs) over a cell's gate repeats, in query order."""
    runs = [r["result"]["samples_us"] for r in records
            if r["cell"] == cell and r["arm"] == "gate" and r["result"].get("samples_us")]
    if not runs:
        return None
    return [sorted(col)[len(col) // 2] if len(col) % 2 else
            (sorted(col)[len(col) // 2 - 1] + sorted(col)[len(col) // 2]) / 2
            for col in zip(*runs)]


def paired_against(records: list[dict], cells: dict, control: str) -> None:
    """Median over queries of (cell - control) per-query medians, in ms."""
    base = per_query_median(records, control)
    if base is None:
        return
    for name, cell in cells.items():
        mine = per_query_median(records, name)
        if mine is None or len(mine) != len(base):
            continue
        diffs = sorted(a - b for a, b in zip(mine, base))
        cell[f"paired_median_diff_vs_{control}_ms"] = diffs[len(diffs) // 2] / 1000.0


def cmd_summarize(args) -> None:
    lines = Path(args.out, "results.jsonl").read_text().splitlines()
    print(json.dumps(summarize([json.loads(l) for l in lines if l.strip()]), indent=1))


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = parser.add_subparsers(dest="command", required=True)
    r = sub.add_parser("run")
    r.add_argument("--cells", required=True)
    r.add_argument("--arms", default="gate")
    r.add_argument("--repeats", type=int, default=2)
    r.add_argument("--repeat-offset", type=int, default=0,
                   help="number the first repeat N (resumes a sweep; sets its order)")
    r.add_argument("--out", required=True)
    r.add_argument("--only", help="comma-separated cell ids")
    r.add_argument("--restored-target")
    r.add_argument("--mcp-store")
    r.add_argument("--tc5-inputs")
    r.add_argument("--max-load", type=float, default=2.0)
    r.set_defaults(func=cmd_run)
    s = sub.add_parser("summarize")
    s.add_argument("--out", required=True)
    s.set_defaults(func=cmd_summarize)
    args = parser.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
