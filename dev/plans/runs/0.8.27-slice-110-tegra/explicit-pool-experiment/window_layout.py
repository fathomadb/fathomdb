#!/usr/bin/env python3
"""window_layout.py LOGDIR...: per run, outcome vs the layout of [8 GiB, 128 GiB) in the
maps snapshot taken just before Engine.open (i.e. before cuInit).
Reports: mappings inside the window, largest unmapped hole, and how much of the
61.36 GiB driver reservation could be placed using holes >= 4 GiB (the smallest
reservation piece observed)."""

import glob
import json
import os
import re
import sys

GiB = 1 << 30
LO, HI = 8 * GiB, 128 * GiB
RESV = 65879900160
for d in sys.argv[1:]:
    for log in sorted(glob.glob(os.path.join(d, "run-*.log"))):
        t = open(log).read()
        js = next(
            (json.loads(line) for line in t.splitlines() if line.startswith("{")), {}
        )
        pid = js.get("pid")
        mp = os.path.join(d, "maps", f"maps-{pid}-before-open.txt")
        if not pid or not os.path.exists(mp):
            continue
        ivs = []
        for line in open(mp):
            a, b = (int(x, 16) for x in line.split()[0].split("-"))
            if b > LO and a < HI:
                ivs.append((max(a, LO), min(b, HI)))
        ivs.sort()
        holes = []
        cur = LO
        for a, b in ivs:
            if a > cur:
                holes.append(a - cur)
            cur = max(cur, b)
        if cur < HI:
            holes.append(HI - cur)
        fit4 = sum(h // (4 * GiB) * 4 * GiB for h in holes)
        sel = re.findall(r"path=(\w+)", t)
        src = sorted(set(re.findall(r"pool_source=([a-z-]+)", t)))
        print(
            f"{os.path.basename(d):28} {os.path.basename(log):18} {js.get('outcome', '?'):4} ctx={len(sel)} "
            f"src={','.join(src) or '-':32} in_window_maps={len(ivs):3} largest_hole={max(holes) / GiB:6.2f}GiB "
            f"holes>=4GiB_capacity={fit4 / GiB:6.2f}GiB need={RESV / GiB:.2f}"
        )
