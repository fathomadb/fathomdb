#!/usr/bin/env python3
"""Apply the refined rule to the earlier Node.js runs (read-only input dir given as argv[1]).
Uses each run's maps-<pid>-start.txt snapshot (taken after CUDA context creation)."""

import glob
import os
import re
import sys

GiB = 1 << 30
NEED = 20960 << 20
LO, HI = 8 * GiB, 128 * GiB
base = sys.argv[1]
bad = 0
n = 0
for log in sorted(glob.glob(os.path.join(base, "logs-*", "run-*.log"))):
    t = open(log).read()
    out = "pass" if re.search(r"^pass$", t, re.M) else "FAIL"
    pid = re.search(r"pid=(\d+)", t).group(1)
    maps = []
    for line in open(
        os.path.join(os.path.dirname(log), "maps", f"maps-{pid}-start.txt")
    ):
        f = line.split()
        a, b = (int(x, 16) for x in f[0].split("-"))
        maps.append((a, b, f[1], " ".join(f[5:])))
    pieces = [
        b - a
        for a, b, p, nm in maps
        if p == "---p" and not nm and b - a >= 4 * GiB and LO <= a < HI
    ]
    cur, mg = LO, 0
    for a, b, *_ in sorted(maps):
        if b <= cur:
            continue
        if a >= HI:
            break
        if a > cur:
            mg = max(mg, min(a, HI) - cur)
        cur = max(cur, b)
    mg = max(mg, HI - cur) if cur < HI else mg
    mp = max(pieces, default=0)
    pred = "pass" if (mp >= 30.6 * GiB or mg >= NEED) else "FAIL"
    n += 1
    bad += pred != out
    print(
        f"{os.path.basename(os.path.dirname(log)):22} {os.path.basename(log):10} {out:4} pred={pred:4} max_piece={mp / GiB:6.2f} max_free_gap={mg / GiB:6.2f}{'  <-- MISMATCH' if pred != out else ''}"
    )
print(f"mismatches: {bad} of {n}")
