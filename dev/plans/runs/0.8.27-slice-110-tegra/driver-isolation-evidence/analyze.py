#!/usr/bin/env python3
"""Summarise logs/<series>/results.txt: outcome counts, per-call result patterns,
largest-reservation-piece distribution, and the pass/fail boundary on max piece."""

import collections
import glob
import os
import sys

base = (
    sys.argv[1]
    if len(sys.argv) > 1
    else os.path.join(os.path.dirname(os.path.abspath(__file__)), "logs")
)
series_filter = sys.argv[2:]

KEYS = [
    "attr_pools",
    "alloc_sync",
    "default_pool",
    "alloc_async",
    "pool_create",
    "pool_create_maxsize",
    "resv_2M",
    "resv_64M",
    "resv_1G",
    "resv_32G",
]


def parse(line):
    d = {}
    for tok in line.split()[1:]:
        if "=" in tok:
            k, v = tok.split("=", 1)
            d[k] = v
    return d


rows = []
for path in sorted(glob.glob(os.path.join(base, "*", "results.txt"))):
    s = os.path.basename(os.path.dirname(path))
    if series_filter and s not in series_filter:
        continue
    recs = [parse(line) for line in open(path) if line.startswith("RESULT")]
    if not recs:
        continue
    out = collections.Counter(r["outcome"] for r in recs)
    pats = collections.Counter(
        (r["outcome"],) + tuple(f"{k}={r.get(k)}" for k in KEYS) for r in recs
    )
    pass_max = [float(r["max_piece_gib"]) for r in recs if r["outcome"] == "pass"]
    fail_max = [float(r["max_piece_gib"]) for r in recs if r["outcome"] == "FAIL"]
    n = len(recs)
    print(
        f"## {s}: n={n} pass={out['pass']} FAIL={out['FAIL']} fail_rate={out['FAIL'] / n:.1%}"
        f"  placed_fail_runs={sum(1 for r in recs if r.get('place_fail', '0') != '0')}"
    )
    print(
        f"   max piece GiB  pass: min={min(pass_max) if pass_max else '-'} max={max(pass_max) if pass_max else '-'}"
        f" | FAIL: min={min(fail_max) if fail_max else '-'} max={max(fail_max) if fail_max else '-'}"
    )
    for p, c in pats.most_common():
        print(f"   x{c:<4} {' '.join(p)}")
    rows.append((s, n, out["pass"], out["FAIL"], pass_max, fail_max))

print(
    "\n| series | runs | pass | FAIL | fail rate | min max-piece (pass) | max max-piece (FAIL) |"
)
print("|---|---|---|---|---|---|---|")
for s, n, p, f, pm, fm in rows:
    print(
        f"| {s} | {n} | {p} | {f} | {f / n:.0%} | {min(pm) if pm else '-'} | {max(fm) if fm else '-'} |"
    )
