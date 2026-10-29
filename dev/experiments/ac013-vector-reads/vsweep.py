#!/usr/bin/env python3
"""Vector-read sweep driver (dev/experiments/ac013-vector-reads/).

Extends the page-cache sweep driver (../ac013-page-cache/sweep.py) with:

- a shared measurement lock (--lock PATH): each run takes an exclusive
  flock on PATH, then waits for the 1-minute load to fall to --max-load
  inside the lock, so runs never overlap another agent's timed runs;
- the arms `vreads` (tests/exp_ac013_vector_reads.rs, untraced) and
  `vtrace` (the same probe with FATHOMDB_PERF_PAGE_TRACE=1);
- `load_flag` on each record: true when the load after the run exceeds
  --flag-load (default 2.5).

Every arm of sweep.py (gate, probe, probeiso, mcp, mcpconc, sdk, i6, iso)
is available. Cells use sweep.py's format; a candidate is a cell whose
"env" sets FATHOMDB_PERF_EXPERIMENTS=1 and its knob (README lists the
knobs). A cell may set "mcp_store" to use its own MCP store.

  python3 dev/experiments/ac013-vector-reads/vsweep.py run \
      --cells dev/experiments/ac013-vector-reads/cells.json --arms gate,vreads \
      --repeats 2 --out /tmp/vreads --lock /tmp/measure.lock \
      [--mcp-store S] [--tc5-inputs ~/.cache/fathomdb-comp/tc5-10000] [--only ...]

  python3 dev/experiments/ac013-vector-reads/vsweep.py summarize --out /tmp/vreads
"""

from __future__ import annotations

import argparse
import fcntl
import json
import os
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "ac013-page-cache"))
import sweep  # noqa: E402


def parse_vreads(log: str) -> dict:
    import re
    m = re.search(r"AC013_VREADS (\{.*\})", log)
    return json.loads(m[1]) if m else {}


def arm_command(arm: str, env: dict, args, run_dir: Path) -> list[str]:
    cargo_test = ["cargo", "test", "--release", "-p", "fathomdb-engine", "--features", "test-hooks"]
    if arm in ("vreads", "vtrace"):
        if arm == "vtrace":
            env["FATHOMDB_PERF_PAGE_TRACE"] = "1"
        else:
            env.pop("FATHOMDB_PERF_PAGE_TRACE", None)
        return cargo_test + ["--test", "exp_ac013_vector_reads", "--", "--ignored", "--nocapture"]
    return sweep.arm_command(arm, env, args, run_dir)


PARSERS = dict(sweep.PARSERS)
PARSERS["vreads"] = lambda o, e: parse_vreads(o + e)
PARSERS["vtrace"] = lambda o, e: parse_vreads(o + e)


def cmd_run(args) -> None:
    cells = json.loads(Path(args.cells).read_text())
    if args.only:
        keep = args.only.split(",")
        cells = [c for c in cells if c["id"] in keep]
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    arms = args.arms.split(",")
    progress = sys.stderr
    default_store = args.mcp_store
    for repeat in range(args.repeats):
        rep = repeat + args.repeat_offset
        for cell in sweep.cell_order(cells, rep):
            for arm in arms:
                env = sweep.cell_env(cell, os.environ, args.restored_target)
                args.mcp_store = cell.get("mcp_store", default_store)
                run_dir = out / f"{cell['id']}.{arm}.r{rep}"
                run_dir.mkdir(exist_ok=True)
                cmd = arm_command(arm, env, args, run_dir)
                lock = open(args.lock, "a") if args.lock else None
                if lock:
                    fcntl.flock(lock, fcntl.LOCK_EX)
                try:
                    load_before = sweep.wait_quiet(args.max_load, progress)
                    started = time.time()
                    print(f"[{time.strftime('%H:%M:%S')}] {cell['id']} {arm} r{rep} "
                          f"load={load_before:.2f}", file=progress, flush=True)
                    code, stdout, stderr = sweep.run(cmd, env, run_dir / "run.log",
                                                     timeout=args.timeout)
                    load_after = sweep.load1()
                finally:
                    if lock:
                        fcntl.flock(lock, fcntl.LOCK_UN)
                        lock.close()
                record = {"cell": cell["id"], "mode": cell["mode"],
                          "reader": cell.get("reader", ""), "writer": cell.get("writer", ""),
                          "env": cell.get("env", {}),
                          "arm": arm, "repeat": rep, "exit": code,
                          "load_before": load_before, "load_after": load_after,
                          "load_flag": load_after > args.flag_load,
                          "wall_s": time.time() - started, "started": started,
                          "result": PARSERS[arm](stdout, stderr)}
                with (out / "results.jsonl").open("a") as f:
                    f.write(json.dumps(record) + "\n")
                print(f"    -> exit={code} load_after={load_after:.2f} "
                      f"{json.dumps(record['result'])[:200]}", file=progress, flush=True)


def summarize(records: list[dict], control: str) -> dict:
    """Group by cell and arm, every repeat kept; vreads/vtrace condensed."""
    plain = [r for r in records if r["arm"] not in ("vreads", "vtrace")]
    cells = sweep.summarize(plain) if plain else {}
    sweep.paired_against(plain, cells, control=control)
    for r in records:
        cell = cells.setdefault(r["cell"], {"mode": r["mode"], "reader": r["reader"],
                                            "writer": r["writer"]})
        cell["env"] = r.get("env", {})
        if r["arm"] in ("vreads", "vtrace"):
            res = r["result"]
            keep = {"hybrid": res.get("hybrid"), "text_only": res.get("text_only"),
                    "results_digest": res.get("results_digest"),
                    "ids_digest": res.get("ids_digest"),
                    "seed_write_ms": res.get("seed_write_ms"), "drain_ms": res.get("drain_ms"),
                    "db_bytes": res.get("store", {}).get("db_bytes"),
                    "file_bytes_checkpointed": res.get("file_bytes_checkpointed"),
                    "n": res.get("n")}
            if r["arm"] == "vtrace":
                keep["trace"] = res.get("trace")
                keep["trace_text_only"] = res.get("trace_text_only")
            keep.update(repeat=r["repeat"], load_before=r["load_before"],
                        load_after=r.get("load_after"), exit=r["exit"])
            cell.setdefault(r["arm"], []).append(keep)
        else:
            # attach load_after / flag to the sweep.py-condensed entries
            for entry in cell.get(r["arm"], []):
                if entry.get("repeat") == r["repeat"]:
                    entry["load_after"] = r.get("load_after")
                    entry["load_flag"] = r.get("load_flag")
    return cells


def cmd_summarize(args) -> None:
    lines = Path(args.out, "results.jsonl").read_text().splitlines()
    print(json.dumps(summarize([json.loads(l) for l in lines if l.strip()], args.control),
                     indent=1))


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = parser.add_subparsers(dest="command", required=True)
    r = sub.add_parser("run")
    r.add_argument("--cells", required=True)
    r.add_argument("--arms", default="gate")
    r.add_argument("--repeats", type=int, default=2)
    r.add_argument("--repeat-offset", type=int, default=0)
    r.add_argument("--out", required=True)
    r.add_argument("--only", help="comma-separated cell ids, run in this order")
    r.add_argument("--restored-target")
    r.add_argument("--mcp-store")
    r.add_argument("--tc5-inputs")
    r.add_argument("--mcp-levels", default="4,8,16,32")
    r.add_argument("--mcp-requests", type=int, default=1000)
    r.add_argument("--max-load", type=float, default=2.0)
    r.add_argument("--flag-load", type=float, default=2.5)
    r.add_argument("--lock", help="flock file shared with other measuring agents")
    r.add_argument("--timeout", type=int, default=7200)
    r.set_defaults(func=cmd_run)
    s = sub.add_parser("summarize")
    s.add_argument("--out", required=True)
    s.add_argument("--control", default="BASE")
    s.set_defaults(func=cmd_summarize)
    args = parser.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
