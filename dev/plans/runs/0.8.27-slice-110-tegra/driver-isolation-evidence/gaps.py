#!/usr/bin/env python3
"""gaps.py [SERIES...]: for every run, from the after-context maps snapshot compute
  - driver reservation pieces: anonymous ---p mappings >= 1 GiB absent before cuInit
  - free gaps: unmapped holes inside the window [8 GiB, 128 GiB)
and print outcome vs largest piece / largest free gap, plus a separation summary."""

import glob
import os
import re
import sys

GiB = 1 << 30
LO, HI = 8 * GiB, 128 * GiB
base = os.path.join(os.path.dirname(os.path.abspath(__file__)), "logs")


def load(p):
    out = []
    for line in open(p):
        f = line.split()
        a, b = (int(x, 16) for x in f[0].split("-"))
        out.append((a, b, f[1], int(f[4]), " ".join(f[5:])))
    return out


def analyse(series):
    rows = []
    for log in sorted(glob.glob(os.path.join(base, series, "run-*.log"))):
        t = open(log).read()
        m = re.search(r"INFO pid=(\d+)", t)
        r = re.search(r"outcome=(\w+)", t)
        if not m or not r:
            continue
        pid = m.group(1)
        pre = os.path.join(base, series, "maps", f"maps-{pid}-0-before-init.txt")
        ctx = os.path.join(base, series, "maps", f"maps-{pid}-2-after-ctx.txt")
        if not (os.path.exists(pre) and os.path.exists(ctx)):
            continue
        prekeys = {(a, b) for a, b, *_ in load(pre)}
        maps = load(ctx)
        pieces = [
            (a, b)
            for a, b, p, ino, n in maps
            if p == "---p"
            and ino == 0
            and not n
            and b - a >= GiB
            and (a, b) not in prekeys
            and a < (1 << 40)
        ]
        # free holes in [LO, HI)
        cur, gaps = LO, []
        for a, b, *_ in sorted(maps):
            if b <= cur:
                continue
            if a >= HI:
                break
            if a > cur:
                gaps.append((cur, min(a, HI)))
            cur = max(cur, b)
        if cur < HI:
            gaps.append((cur, HI))
        mp = max((b - a for a, b in pieces), default=0) / GiB
        mg = max((b - a for a, b in gaps), default=0) / GiB
        rows.append((os.path.basename(log), r.group(1), mp, mg, len(pieces)))
    return rows


series = sys.argv[1:] or sorted(
    os.path.basename(d) for d in glob.glob(os.path.join(base, "*"))
)
allrows = []
for s in series:
    rows = analyse(s)
    allrows += [(s,) + r for r in rows]
    for out in ("pass", "FAIL"):
        sel = [r for r in rows if r[1] == out]
        if sel:
            print(
                f"{s:34} {out:4} n={len(sel):4} maxpiece[min..max]={min(r[2] for r in sel):6.2f}..{max(r[2] for r in sel):6.2f}"
                f"  maxfreegap[min..max]={min(r[3] for r in sel):6.2f}..{max(r[3] for r in sel):6.2f}"
            )

# rule: pass iff (largest reservation VMA >= 30.6 GiB, i.e. a full- or half-size
# driver reservation exists) or (largest free hole in [8,128) GiB >= 20960 MiB).
# Series using --pre-reserve (s4d/s4e) fragment the inside of the reservation,
# which /proc/self/maps cannot show, so they are excluded from this check.
NEED = 20960 * (1 << 20) / GiB
print(
    "\nviolations of rule 'pass iff max_piece >= 30.6 GiB or max_free_gap >= 20960 MiB' (excluding --pre-reserve series):"
)
bad = 0
allrows = [r for r in allrows if not r[0].startswith(("s4d-", "s4e-"))]
for s, log, out, mp, mg, n in allrows:
    pred = "pass" if (mp >= 30.6 or mg >= NEED - 1e-9) else "FAIL"
    if pred != out:
        bad += 1
        print(
            f"  {s} {log} actual={out} max_piece={mp:.2f} max_free_gap={mg:.2f} pieces={n}"
        )
print(f"  total violations: {bad} of {len(allrows)} runs")
