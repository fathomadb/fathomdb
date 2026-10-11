#!/usr/bin/env python3
"""Per process: outcome, default-pool result, and largest CUDA ---p VA reservation (temporary diagnostic)."""

import collections
import glob
import os
import re
import sys

base = sys.argv[1] if len(sys.argv) > 1 else "."
tab = collections.Counter()
patterns = collections.Counter()
for d in sorted(glob.glob(os.path.join(base, "logs-*"))):
    for log in sorted(glob.glob(os.path.join(d, "run-*.log"))):
        t = open(log).read()
        out = "pass" if re.search(r"^pass$", t, re.M) else "FAIL"
        pid = re.search(r"pid=(\d+)", t).group(1)
        steps = tuple(
            (
                m.group(1),
                "OOM"
                if "OUT_OF_MEMORY" in m.group(2)
                else ("skip" if "skipped" in m.group(2) else "ok"),
            )
            for m in re.finditer(r"slice110-alloc-split (S\d\w*?|P0_pools):? (.*)", t)
            if not m.group(1).endswith("stored_err_after")
        )
        patterns[(out, steps)] += 1
        sizes = []
        for line in open(os.path.join(d, "maps", f"maps-{pid}-start.txt")):
            f = line.split()
            a, b = (int(x, 16) for x in f[0].split("-"))
            if f[1] == "---p" and b - a >= (4 << 30) and a < (1 << 40):
                sizes.append((b - a) / 2**30)
        mx = max(sizes)
        bucket = ">=30.6GiB" if mx >= 30.6 else "<=23.1GiB" if mx <= 23.1 else "other"
        tab[(os.path.basename(d), out, bucket)] += 1
        print(
            f"{os.path.basename(d):18} {log[-6:-4]} {out:4} max_cuda_resv={mx:6.2f}GiB pieces={len(sizes)} sum={sum(sizes):.2f}"
        )
print("\nseries / outcome / largest reservation bucket:")
for k, v in sorted(tab.items()):
    print("  ", k, v)
print("\ndistinct step-result patterns:")
for (out, steps), n in patterns.items():
    print(f"  {out} x{n}: " + ", ".join(f"{s}={r}" for s, r in steps))
