#!/usr/bin/env python3
"""Summarize allocator-fix verification series (logs/<series>/run-*.log).

The allocator path is not printed by the product, so it is inferred two ways:
from the steady-state embed median (sync about 25 ms, async about 10 ms on
this Orin, per ../sync-repair-experiment/analysis.txt) and from the
/proc/self/maps rule in ../driver-isolation-evidence/results.md (default pool
obtainable iff the largest anonymous PROT_NONE mapping in [8, 128) GiB is at
least 30.6 GiB or the largest hole there is at least 20960 MiB). The maps
snapshot is taken after open, when a created pool may already occupy a hole,
so the rule is only reported, not used to classify.
"""

import collections
import glob
import json
import os
import re
import statistics
import sys

base = sys.argv[1] if len(sys.argv) > 1 else "logs"
SYNC_MS = 18.0
rows, fails = [], []
counts = collections.Counter()
heads, timing = (
    collections.defaultdict(set),
    collections.defaultdict(lambda: collections.defaultdict(list)),
)
for series in sorted(os.listdir(base)):
    for log in sorted(glob.glob(os.path.join(base, series, "run-*.log"))):
        text = open(log).read()
        m = re.search(r"^\{.*\}$", text, re.M)
        rec = (
            json.loads(m.group(0))
            if m
            else {"outcome": "fail", "failedStep": "no-json", "error": text[-400:]}
        )
        t = rec.get("timingsMs", {})
        steady = t.get("embedSteadyMedian")
        device = "cpu" if "-cpu" in series else "cuda"
        path = (
            "-"
            if device == "cpu" or steady is None
            else ("sync" if steady > SYNC_MS else "async")
        )
        va = rec.get("va") or {}
        rule = "-"
        if va:
            rule = (
                "pool-fits"
                if va["maxNoneMappingGiB"] >= 30.6 or va["maxHoleGiB"] >= 20960 / 1024
                else "no-fit"
            )
        w = rec.get("witness") or {}
        counts[(series, rec["outcome"], path)] += 1
        if rec["outcome"] != "pass":
            fails.append(
                f"{series} {os.path.basename(log)} step={rec.get('failedStep')}: {rec.get('error')}"
            )
        else:
            heads[device].add(tuple(rec.get("embedHead", [])))
            for k in (
                "open",
                "embedFirst",
                "embedSteadyMedian",
                "rerankFirst",
                "rerankSecond",
            ):
                if k in t:
                    timing[(series.split("-")[0], device, path)][k].append(t[k])
            if w:
                timing[(series.split("-")[0], device, path)]["witnessDeltaMiB"].append(
                    w["deltaBytes"] / 2**20
                )
        rows.append(
            f"{series:24} {os.path.basename(log):11} {rec['outcome']:4} {path:5} "
            f"steady={steady if steady is None else round(steady, 1)!s:6} "
            f"va_piece={va.get('maxNoneMappingGiB', '-')!s:6} va_hole={va.get('maxHoleGiB', '-')!s:6} rule={rule:9} "
            f"witness_delta={w.get('deltaBytes', '-')}"
        )
print("\n".join(rows))
print("\n# failures")
print("\n".join(fails) if fails else "none")
print("\n# counts per series/outcome/inferred path")
for k, v in sorted(counts.items()):
    print(f"  {k}: {v}")
print("\n# distinct embedding heads (first three values) per device")
for device, hs in sorted(heads.items()):
    print(f"  {device}: {len(hs)} distinct: {sorted(hs)}")
print("\n# timing (ms) for passing runs by series prefix, device, inferred path")
for key, metrics in sorted(timing.items()):
    print(key)
    for k, xs in metrics.items():
        print(
            f"  {k:18} n={len(xs):2} median={statistics.median(xs):8.1f} min={min(xs):8.1f} max={max(xs):8.1f}"
        )
