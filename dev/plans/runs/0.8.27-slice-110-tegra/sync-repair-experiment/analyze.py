#!/usr/bin/env python3
"""Summarize sync-repair experiment logs: per-run table + grouped timing stats."""

import json
import re
import statistics
import sys
from pathlib import Path

root = Path(sys.argv[1])
rows = []
for log in sorted(root.glob("*/run-*.log")):
    text = log.read_text()
    sel = re.findall(r"pool_available=(\w+) path=(\w+)", text)
    paths = sorted({p for _, p in sel})
    pools = sorted({a for a, _ in sel})
    js = next(
        (json.loads(line) for line in text.splitlines() if line.startswith("{")), None
    )
    wall = re.search(r"wall_ms=(\d+)", text)
    rows.append(
        {
            "series": log.parent.name,
            "run": log.stem,
            "contexts": len(sel),
            "pool": "/".join(pools) or "-",
            "path": "/".join(paths) or "-",
            "outcome": js["outcome"] if js else "nojson",
            "failedStep": js.get("failedStep") if js else None,
            "error": (js.get("error") or {}).get("message") if js else text[-400:],
            "t": js.get("timingsMs", {}) if js else {},
            "delta": (js.get("witness") or {}).get("deltaBytes") if js else None,
            "ctrlBlocks": (js.get("witness") or {}).get("controlBlockCount")
            if js
            else None,
            "wall": int(wall.group(1)) if wall else None,
        }
    )

print(
    f"{'series':22} {'run':18} ctx pool        path   outcome failStep   open   embed1 embedSt rerank1 rerank2  witnessDelta ctrlBlk wall"
)
for r in rows:
    t = r["t"]

    def f(k, t=t):
        return f"{t[k]:7.1f}" if k in t else "      -"

    print(
        f"{r['series']:22} {r['run']:18} {r['contexts']:3} {r['pool']:11} {r['path']:6} {r['outcome']:7} "
        f"{str(r['failedStep'] or ''):8} {f('open')} {f('embedFirst')} {f('embedSteadyMedian')} "
        f"{f('rerankFirst')} {f('rerankSecond')} {str(r['delta'] or '-'):>13} {str(r['ctrlBlocks'] or '-'):>7} {r['wall']}"
    )

print("\n# failures")
for r in rows:
    if r["outcome"] != "pass":
        print(
            f"{r['series']} {r['run']} pool={r['pool']} path={r['path']} step={r['failedStep']}: {r['error']}"
        )

print("\n# counts per series/outcome/path/pool")
counts = {}
for r in rows:
    k = (r["series"], r["outcome"], r["path"], r["pool"])
    counts[k] = counts.get(k, 0) + 1
for k, v in sorted(counts.items()):
    print(f"  {k}: {v}")


def stats(xs):
    if not xs:
        return "n=0"
    q = statistics.quantiles(xs, n=4) if len(xs) >= 4 else None
    s = f"n={len(xs):2} median={statistics.median(xs):8.1f} min={min(xs):8.1f} max={max(xs):8.1f}"
    if q:
        s += f" IQR=[{q[0]:.1f},{q[2]:.1f}]"
    return s


print("\n# timing (ms) for PASSING runs grouped by node series prefix + path taken")
groups = {}
for r in rows:
    if r["outcome"] != "pass" or "-" not in r["series"]:
        continue
    node = r["series"].split("-")[0]
    mode = r["series"].split("-", 1)[1]
    groups.setdefault((node, mode, r["path"]), []).append(r)
for k in sorted(groups):
    print(f"\n{k}")
    for metric in [
        "open",
        "embedFirst",
        "embedSteadyMedian",
        "rerankFirst",
        "rerankSecond",
    ]:
        print(
            f"  {metric:18} {stats([r['t'][metric] for r in groups[k] if metric in r['t']])}"
        )
    print(
        f"  {'witnessDeltaMiB':18} {stats([r['delta'] / 2**20 for r in groups[k] if r['delta']])}"
    )
    print(
        f"  {'controlBlocks':18} {stats([r['ctrlBlocks'] for r in groups[k] if r['ctrlBlocks']])}"
    )
