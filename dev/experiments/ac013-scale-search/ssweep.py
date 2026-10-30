#!/usr/bin/env python3
"""Scale-search sweep driver (dev/experiments/ac013-scale-search/).

Runs a prebuilt `exp_ac013_scale_search` test binary (so source edits during a
sweep cannot change what is measured) once per cell x repeat, under a shared
measurement lock, and appends one record per run to `<out>/results.jsonl`.

- `--lock PATH`: an exclusive flock on PATH is held for the whole run; inside
  it the driver waits until the 1-minute load is <= --max-load (default 2.0,
  at most --max-wait seconds, then runs anyway). Records carry `load_before`,
  `load_after` and `load_flag` (load_after > --max-load).
- Workload flags map to the probe's env: `--corpus syn|lme` (lme reads
  `--lme-dir`/corpus.jsonl and queries.jsonl), `--n`, `--samples`,
  `--passes`, `--trace` (FATHOMDB_PERF_PAGE_TRACE=1), `--supersede`.
- Cells come from `--cells` (JSON list of {"id", "env"}); `--only` keeps a
  comma list of ids. Odd repeats run the cells in reverse order.
- `--bin` is the test binary (`cargo test --release -p fathomdb-engine
  --features test-hooks --test exp_ac013_scale_search --no-run` prints its
  path; copy it so later builds do not replace it). `--bin-sha` records the
  commit it was built from.

  python3 ssweep.py run --cells cells.json --out OUT --lock LOCK --bin BIN \
      --bin-sha SHA --corpus syn --n 100000 --samples 300 --passes hybrid,text \
      [--trace] [--repeats 2] [--repeat-offset 0] [--only CS64,CS64+ELIG]
  python3 ssweep.py summarize --out OUT > results.json
  python3 ssweep.py run-gate ...   (the perf_gates AC-013 gate; see --help)
"""

from __future__ import annotations

import argparse
import fcntl
import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path


def load1() -> float:
    return os.getloadavg()[0]


def wait_quiet(max_load: float, max_wait: float) -> float:
    started = time.time()
    while load1() > max_load and time.time() - started < max_wait:
        time.sleep(15)
    return load1()


def probe_env(cell: dict, args) -> dict:
    env = {k: v for k, v in os.environ.items() if not k.startswith(("FATHOMDB_PERF_", "AC013_"))}
    env.update(cell.get("env", {}))
    env["AC013_CORPUS_N"] = str(args.n)
    env["AC013_SAMPLES"] = str(args.samples)
    env["AC013_PASSES"] = args.passes
    if args.corpus == "lme":
        env["AC013_CORPUS_FILE"] = str(Path(args.lme_dir) / "corpus.jsonl")
        env["AC013_QUERY_FILE"] = str(Path(args.lme_dir) / "queries.jsonl")
    if args.trace:
        env["FATHOMDB_PERF_PAGE_TRACE"] = "1"
    if args.supersede:
        env["AC013_SUPERSEDE_N"] = str(args.supersede)
    if args.results_dir:
        env["AC013_RESULTS_OUT"] = str(
            Path(args.results_dir) / f"{cell['id']}.{args.corpus}.n{args.n}.hits.txt"
        )
    return env


def locked_run(cmd: list[str], env: dict, log: Path, args) -> tuple[int, str, float, float, float]:
    lock = open(args.lock, "a") if args.lock else None
    if lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
    try:
        load_before = wait_quiet(args.max_load, args.max_wait)
        started = time.time()
        with log.open("w") as f:
            proc = subprocess.run(cmd, env=env, stdout=f, stderr=subprocess.STDOUT,
                                  timeout=args.timeout)
        load_after = load1()
    finally:
        if lock:
            fcntl.flock(lock, fcntl.LOCK_UN)
            lock.close()
    return proc.returncode, log.read_text(errors="replace"), load_before, load_after, started


def cmd_run(args) -> None:
    cells = json.loads(Path(args.cells).read_text())
    if args.only:
        keep = args.only.split(",")
        cells = [c for c in cells if c["id"] in keep]
        cells.sort(key=lambda c: keep.index(c["id"]))
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    for repeat in range(args.repeats):
        rep = repeat + args.repeat_offset
        order = cells if rep % 2 == 0 else list(reversed(cells))
        for cell in order:
            env = probe_env(cell, args)
            tag = f"{cell['id']}.{args.corpus}.n{args.n}.{'trace' if args.trace else 'time'}.r{rep}"
            log = out / f"{tag}.log"
            cmd = [args.bin, "--ignored", "--nocapture", "--exact", "ac013_scale_search_probe"]
            print(f"[{time.strftime('%H:%M:%S')}] start {tag}", file=sys.stderr, flush=True)
            code, text, lb, la, started = locked_run(cmd, env, log, args)
            m = re.search(r"AC013_SCALE (\{.*\})", text)
            record = {
                "cell": cell["id"], "env": cell.get("env", {}), "corpus": args.corpus,
                "n": args.n, "samples": args.samples, "passes": args.passes,
                "trace": args.trace, "supersede": args.supersede, "repeat": rep,
                "bin_sha": args.bin_sha, "exit": code, "load_before": lb, "load_after": la,
                "load_flag": la > args.max_load, "wall_s": time.time() - started,
                "started": started, "log": str(log),
                "result": json.loads(m[1]) if m else None,
            }
            with (out / "results.jsonl").open("a") as f:
                f.write(json.dumps(record) + "\n")
            brief = {}
            if record["result"]:
                for p, v in record["result"]["passes"].items():
                    brief[p] = (round(v["p50_ms"], 1), v["digest"], v["soft_fallback_searches"])
            print(f"    exit={code} load {lb:.2f}->{la:.2f} {brief}", file=sys.stderr, flush=True)


def cmd_gate(args) -> None:
    """The perf_gates AC-013 gate (AGENT_LONG=1) once per cell x repeat."""
    cells = json.loads(Path(args.cells).read_text())
    if args.only:
        keep = args.only.split(",")
        cells = [c for c in cells if c["id"] in keep]
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    for repeat in range(args.repeats):
        rep = repeat + args.repeat_offset
        for cell in cells if rep % 2 == 0 else list(reversed(cells)):
            env = {k: v for k, v in os.environ.items()
                   if not k.startswith(("FATHOMDB_PERF_", "AC013_"))}
            env.update(cell.get("env", {}))
            env["AGENT_LONG"] = "1"
            env["AC013_CORPUS_N"] = str(args.n)
            tag = f"{cell['id']}.gate.n{args.n}.r{rep}"
            log = out / f"{tag}.log"
            cmd = [args.bin, "--exact", "ac_013_vector_retrieval_latency", "--nocapture",
                   "--test-threads=1"]
            print(f"[{time.strftime('%H:%M:%S')}] start {tag}", file=sys.stderr, flush=True)
            code, text, lb, la, started = locked_run(cmd, env, log, args)
            m = re.search(r"AC013_NUMBERS n=(\d+) samples=(\d+) seed_ms=(\d+) p50_ms=(\d+) "
                          r"p99_ms=(\d+)", text)
            record = {"cell": cell["id"], "env": cell.get("env", {}), "arm": "gate", "n": args.n,
                      "repeat": rep, "bin_sha": args.bin_sha, "exit": code, "load_before": lb,
                      "load_after": la, "load_flag": la > args.max_load, "log": str(log),
                      "result": ({"p50_ms": int(m[4]), "p99_ms": int(m[5]), "seed_ms": int(m[3])}
                                 if m else None)}
            with (out / "results.jsonl").open("a") as f:
                f.write(json.dumps(record) + "\n")
            print(f"    exit={code} load {lb:.2f}->{la:.2f} {record['result']}",
                  file=sys.stderr, flush=True)


def condense_trace(trace: dict | None, top: int) -> dict | None:
    if not trace:
        return None
    rows = []
    for s in trace["statements"][:top]:
        rows.append({k: (round(v, 3) if isinstance(v, float) else v) for k, v in s.items()})
    return {"exclusive_ms_per_search": round(trace["exclusive_ms_per_search"], 3),
            "exclusive_requests_per_search": round(trace["exclusive_requests_per_search"], 1),
            "exclusive_miss_per_search": round(trace["exclusive_miss_per_search"], 1),
            "statements": rows}


def cmd_summarize(args) -> None:
    records = [json.loads(line) for line in (Path(args.out) / "results.jsonl").open()]
    summary: dict = {}
    for r in records:
        if r.get("arm") == "gate":
            key = f"{r['cell']}|gate|n={r['n']}"
            summary.setdefault(key, {"env": r["env"], "runs": []})["runs"].append(
                {k: r[k] for k in ("repeat", "bin_sha", "exit", "load_before", "load_after",
                                   "load_flag")} | {"result": r["result"]})
            continue
        kind = "trace" if r["trace"] else "time"
        sup = f"|sup={r['supersede']}" if r.get("supersede") else ""
        key = f"{r['cell']}|{r['corpus']}|n={r['n']}|{kind}{sup}"
        entry = summary.setdefault(key, {"env": r["env"], "runs": []})
        res = r["result"] or {}
        run = {k: r[k] for k in ("repeat", "bin_sha", "exit", "load_before", "load_after",
                                 "load_flag", "samples")}
        if res:
            run["matches"] = res["matches"]
            run["seed_write_ms"] = round(res["seed_write_ms"], 1)
            run["drain_ms"] = round(res["drain_ms"], 1)
            run["page_count"] = res["page_count"]
            run["corpus_digest"] = res["corpus_digest"]
            run["query_digest"] = res["query_digest"]
            run["passes"] = {}
            for p, v in res["passes"].items():
                run["passes"][p] = {
                    k: (round(v[k], 3) if isinstance(v[k], float) else v[k])
                    for k in ("p50_ms", "p99_ms", "mean_ms", "cache_hit_per_search",
                              "cache_miss_per_search", "soft_fallback_searches",
                              "hits_per_search", "digest", "ids_digest")
                }
                run["passes"][p]["trace"] = condense_trace(v.get("trace"), args.top)
        entry["runs"].append(run)
    json.dump(summary, sys.stdout, indent=1, sort_keys=True)
    print()


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawTextHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    for name in ("run", "run-gate"):
        p = sub.add_parser(name)
        p.add_argument("--cells", required=True)
        p.add_argument("--out", required=True)
        p.add_argument("--lock")
        p.add_argument("--bin", required=True)
        p.add_argument("--bin-sha", required=True)
        p.add_argument("--n", type=int, default=10_000)
        p.add_argument("--repeats", type=int, default=1)
        p.add_argument("--repeat-offset", type=int, default=0)
        p.add_argument("--only")
        p.add_argument("--max-load", type=float, default=2.0)
        p.add_argument("--max-wait", type=float, default=1800)
        p.add_argument("--timeout", type=float, default=4 * 3600)
        if name == "run":
            p.add_argument("--corpus", choices=("syn", "lme"), default="syn")
            p.add_argument("--lme-dir")
            p.add_argument("--samples", type=int, default=1_000)
            p.add_argument("--passes", default="hybrid,text")
            p.add_argument("--trace", action="store_true")
            p.add_argument("--supersede", type=int, default=0)
            p.add_argument("--results-dir")
    s = sub.add_parser("summarize")
    s.add_argument("--out", required=True)
    s.add_argument("--top", type=int, default=12)
    args = ap.parse_args()
    {"run": cmd_run, "run-gate": cmd_gate, "summarize": cmd_summarize}[args.cmd](args)


if __name__ == "__main__":
    main()
