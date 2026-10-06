#!/usr/bin/env python3
"""Summarize explicit-pool experiment logs.

Per run: series, outcome, failed step/error, allocator path, pool source
(default / explicit-created / explicit-reused / sync-fallback), timings, witness
delta, embedding hash, rerank scores, explicit-pool high-water marks.
Then: counts, timing stats grouped by (node, effective allocator), output identity.
"""

import json
import re
import statistics
import sys
from pathlib import Path

root = Path(sys.argv[1])
rows = []
for log in sorted(root.glob("*/run-*.log")):
    if log.parent.name == "trial":
        continue
    text = log.read_text()
    js = next(
        (json.loads(line) for line in text.splitlines() if line.startswith("{")), None
    )
    paths = re.findall(r"alloc-select .*? path=(\w+)", text)
    srcs = re.findall(r"pool_source=([a-z()-]+)", text)
    avail = re.findall(r"pool_available=(\w+)", text)
    used_high = [int(x) for x in re.findall(r"pool-teardown .*?used_high=(\d+)", text)]
    resv_high = [
        int(x) for x in re.findall(r"pool-teardown .*?reserved_high=(\d+)", text)
    ]
    if not paths:
        eff = "none(no-ctx)"
    elif "explicit-created" in srcs:
        eff = "explicit-pool"
    elif srcs and all(s == "default" for s in srcs):
        eff = "default-pool"
    elif set(paths) == {"sync"}:
        eff = "sync"
    elif set(paths) == {"async"}:
        eff = "default-pool"  # stock / pool-mode with default pool
    else:
        eff = "mixed"
    t = (js or {}).get("timingsMs", {})
    rows.append(
        {
            "series": log.parent.name,
            "run": log.stem,
            "node": (js or {}).get("node"),
            "outcome": (js or {}).get("outcome", "nojson"),
            "step": (js or {}).get("failedStep"),
            "error": ((js or {}).get("error") or {}).get("message"),
            "ctx": len(paths),
            "eff": eff,
            "default_pool_available": avail[0] if avail else "-",
            "t": t,
            "delta": ((js or {}).get("witness") or {}).get("deltaBytes"),
            "sha": (js or {}).get("embedSha"),
            "scores": (js or {}).get("rerankScores"),
            "used_high": max(used_high) if used_high else None,
            "resv_high": max(resv_high) if resv_high else None,
        }
    )

print(
    f"{'series':30} {'run':16} {'outcome':7} {'step':7} ctx {'effective':13} defpool "
    f"{'open':>7} {'emb1':>6} {'embSt':>6} {'rr1':>6} {'rr2':>6} {'deltaMiB':>8} {'poolUsedHiMiB':>13} sha"
)
for r in rows:
    t = r["t"]

    def f(k, t=t):
        return f"{t[k]:6.1f}" if k in t else "     -"

    print(
        f"{r['series']:30} {r['run']:16} {r['outcome']:7} {str(r['step'] or ''):7} {r['ctx']:3} {r['eff']:13} "
        f"{r['default_pool_available']:7} {f('open'):>7} {f('embedFirst')} {f('embedSteadyMedian')} {f('rerankFirst')} "
        f"{f('rerankSecond')} {(r['delta'] or 0) / 2**20:8.1f} "
        f"{(r['used_high'] or 0) / 2**20:13.1f} {r['sha'] or '-'}"
    )

print("\n# failures (series run step: error)")
for r in rows:
    if r["outcome"] != "pass":
        print(
            f"  {r['series']} {r['run']} ctx={r['ctx']} eff={r['eff']} step={r['step']}: {r['error']}"
        )

print("\n# counts: series / outcome / effective allocator")
counts = {}
for r in rows:
    k = (r["series"], r["outcome"], r["eff"])
    counts[k] = counts.get(k, 0) + 1
for k, v in sorted(counts.items()):
    print(f"  {k}: {v}")

print(
    "\n# default-pool availability among runs that created >=1 context (from pool_available=)"
)
av = {}
for r in rows:
    if r["ctx"]:
        k = r["series"]
        a, n = av.get(k, (0, 0))
        av[k] = (a + (r["default_pool_available"] == "true"), n + 1)
for k, (a, n) in sorted(av.items()):
    print(f"  {k}: {a}/{n}")


def stats(xs):
    if not xs:
        return "n=0"
    s = f"n={len(xs):2} median={statistics.median(xs):7.1f} min={min(xs):7.1f} max={max(xs):7.1f}"
    if len(xs) >= 4:
        q = statistics.quantiles(xs, n=4)
        s += f" IQR=[{q[0]:.1f},{q[2]:.1f}]"
    return s


print(
    "\n# timings (ms), PASSING runs, grouped by node major + series config + effective allocator"
)
groups = {}
for r in rows:
    if r["outcome"] != "pass":
        continue
    cfg = re.sub(r"^node\d+-", "", r["series"])
    groups.setdefault(((r["node"] or "?").split(".")[0], cfg, r["eff"]), []).append(r)
for k in sorted(groups):
    g = groups[k]
    print(f"\n{k}")
    for m in ["open", "embedFirst", "embedSteadyMedian", "rerankFirst", "rerankSecond"]:
        print(f"  {m:18} {stats([r['t'][m] for r in g if m in r['t']])}")
    print(
        f"  {'witnessDeltaMiB':18} {stats([r['delta'] / 2**20 for r in g if r['delta']])}"
    )
    ph = [r["used_high"] / 2**20 for r in g if r["used_high"]]
    if ph:
        print(f"  {'poolUsedHighMiB':18} {stats(ph)}")
        print(
            f"  {'poolReservedHighMiB':18} {stats([r['resv_high'] / 2**20 for r in g if r['resv_high']])}"
        )

print(
    "\n# pooled timings across Node 25 configs by effective allocator (passing runs, all heap/maxSize/threshold configs pooled)"
)
pooled = {}
for r in rows:
    if r["outcome"] == "pass" and (r["node"] or "").startswith("v25"):
        pooled.setdefault(r["eff"], []).append(r)
for k, g in sorted(pooled.items()):
    print(f"\n{k}")
    for m in ["open", "embedFirst", "embedSteadyMedian", "rerankFirst", "rerankSecond"]:
        print(f"  {m:18} {stats([r['t'][m] for r in g if m in r['t']])}")

print("\n# output identity across allocators (passing runs)")
by = {}
for r in rows:
    if r["outcome"] == "pass":
        key = (r["sha"], json.dumps(r["scores"]))
        by.setdefault(key, {}).setdefault(r["eff"], 0)
        by[key][r["eff"]] += 1
for (sha, scores), effs in by.items():
    print(f"  embedSha={sha} rerankScores={scores}: {effs}")
