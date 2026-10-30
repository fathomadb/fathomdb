#!/usr/bin/env python3
"""Print one AC013_DRAIN record (or every record in a drain_sweep results.jsonl).

  python3 dev/experiments/ac013-seed-drain/show.py <run.log | results.jsonl> [--top N]

Shows timing, phase counters per vector, per-role page requests and the top
statements by exclusive page requests (role, runs, requests per run, misses,
exclusive ms).
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path


def records(path: Path):
    text = path.read_text()
    if path.suffix == ".jsonl":
        for line in text.splitlines():
            if line.strip():
                rec = json.loads(line)
                yield rec.get("cell", "?"), rec.get("result", {})
        return
    for m in re.finditer(r"AC013_DRAIN (\{.*\})", text):
        yield path.name, json.loads(m[1])


def show(name: str, d: dict, top: int) -> None:
    if not d:
        print(f"== {name}: no record")
        return
    p = d["phase"]
    print(f"== {name} n={d['n']} mode={d['mode']} threads={d['runtime_threads']} "
          f"write={d['write_ms']:.0f} drain={d['drain_ms']:.0f} seed={d['seed_ms']:.0f} ms")
    print(f"   scans={p['dispatcher_scans']} scan_ms={p['dispatcher_scan_ms']:.0f} "
          f"rows_seen/scan={p['rows_seen_per_scan']} batches={p['worker_batches']} "
          f"embed_ms={p['worker_embed_ms']:.0f} commit_ms={p['worker_commit_ms']:.0f} "
          f"gate_wait_ms={p['worker_gate_wait_ms']:.0f} probes={p['drain_probes']} "
          f"probe_ms={p['drain_probe_ms']:.0f} probe_pending={p['drain_probe_pending']}")
    print(f"   writer drain miss={d['writer']['drain_miss']} hit={d['writer']['drain_hit']}")
    for r in d.get("trace_by_role", []):
        print(f"   {r['role']:<22} req/vec={r['requests_per_vector']:.1f} "
              f"miss/vec={r['miss_per_vector']:.1f} excl_ms={r['exclusive_ms']:.0f}")
    for r in d.get("trace", [])[:top]:
        req = r["exclusive_hit"] + r["exclusive_miss"]
        print(f"   {r['role'][:14]:<14} runs={r['runs']:<7} req={req:<9} req/run={r['requests_per_run']:<9.1f}"
              f" miss={r['exclusive_miss']:<8} ms={r['exclusive_ms']:<7.0f} {r['sql'][:90]}")
    if "digest" in d:
        dg = d["digest"]
        cur = dict(dg.get("open_state", [])).get("projection_cursor")
        print(f"   digest rows={dg['vector_rows']} vec={dg['vector_default']} "
              f"term={dg['terminal']} cursor={cur}")


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("path")
    ap.add_argument("--top", type=int, default=12)
    args = ap.parse_args()
    for name, d in records(Path(args.path)):
        show(name, d, args.top)


if __name__ == "__main__":
    sys.exit(main())
