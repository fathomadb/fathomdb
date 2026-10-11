#!/usr/bin/env python3
"""Tabulate slice110-alloc-split step results per process (temporary diagnostic)."""

import collections
import glob
import os
import re
import sys

d = sys.argv[1]
rows = []
for log in sorted(glob.glob(os.path.join(d, "run-*.log"))):
    txt = open(log).read()
    r = {"run": os.path.basename(log)[4:-4]}
    r["outcome"] = "pass" if re.search(r"^pass$", txt, re.M) else "FAIL"
    for line in txt.splitlines():
        m = re.match(r"slice110-alloc-split (\S+?):? (.*)", line)
        if not m:
            continue
        k, v = m.groups()
        r[k] = v
    pid = re.search(r"pid=(\d+)", r.get("P0_thread", ""))
    r["pid"] = pid.group(1) if pid else ""
    rows.append(r)


def short(v):
    if v is None:
        return "-"
    if "OUT_OF_MEMORY" in v:
        return "OOM"
    if v.startswith(("ok", "CUDA_SUCCESS", "Ok(")):
        return "ok"
    if v.startswith("skipped"):
        return "skip"
    return v[:20]


steps = [
    "S2_cuMemAlloc_v2_4",
    "S3_stream_alloc_u8_4",
    "S3r_cuMemAllocAsync_4_raw",
    "S3r_cuEventCreate_disable_timing",
    "S3r_cuMemAllocFromPoolAsync_default_4",
    "S3r_cuMemPoolCreate",
    "S3r_cuMemAllocFromPoolAsync_newpool_4",
    "S3v_cuMemAddressReserve_2MiB",
    "S3v_cuMemAddressReserve_1024MiB",
    "S3v_cuMemAddressReserve_32768MiB",
    "S4_memset_zeros",
    "S4_stream_synchronize",
    "S5a_alloc_zeros_u8_4",
    "S5b_tensor_zeros_f32_1",
]


def pools(r):
    v = r.get("P0_pools", "")
    m = re.search(r"default=(\w+)", v)
    return (
        "ok"
        if m and m.group(1) == "CUDA_SUCCESS"
        else ("OOM" if "OUT_OF_MEMORY" in v else v[:12])
    )


def maps_feats(r):
    path = os.path.join(d, "maps", f"maps-{r['pid']}-start.txt")
    if not os.path.exists(path):
        return {}
    segs = []
    for line in open(path):
        a, b = (int(x, 16) for x in line.split()[0].split("-"))
        segs.append((a, b, line.split()[5] if len(line.split()) > 5 else ""))
    segs.sort()

    # largest free gap below 2^40 (1 TiB) and below 2^39
    def max_gap(limit, lo=0x10000):
        best, prev = 0, lo
        for a, b, _ in segs:
            if a >= limit:
                break
            if a > prev:
                best = max(best, a - prev)
            prev = max(prev, b)
        return max(best, limit - prev if prev < limit else 0)

    big = sorted([s for s in segs if s[1] - s[0] >= (1 << 30)], key=lambda s: s[0])
    return {
        "gap_lt_2^36_GiB": max_gap(1 << 36) >> 30,
        "gap_lt_2^39_GiB": max_gap(1 << 39) >> 30,
        "gap_lt_2^40_GiB": max_gap(1 << 40) >> 30,
        "big_maps": " ".join(f"{a:#x}+{(b - a) >> 30}G" for a, b, _ in big),
    }


print(
    "| run | outcome | default pool | "
    + " | ".join(s.split("_", 1)[1] for s in steps)
    + " |"
)
print("|" + "---|" * (3 + len(steps)))
counts = collections.defaultdict(collections.Counter)
for r in rows:
    vals = [short(r.get(s)) for s in steps]
    for s, v in zip(["pool"] + steps, [pools(r)] + vals):
        counts[s][(r["outcome"], v)] += 1
    print(f"| {r['run']} | {r['outcome']} | {pools(r)} | " + " | ".join(vals) + " |")
print("\nCounts (outcome, result):")
for s, c in counts.items():
    print(f"  {s}: " + ", ".join(f"{o}/{v}={n}" for (o, v), n in sorted(c.items())))
print("\nPer-process properties:")


def grab(pat, text, default="-"):
    m = re.search(pat, text or "")
    return m.group(1) if m else default


for r in rows:
    f = maps_feats(r)
    st = r.get("P0_status", "")
    vms = grab(r"VmSize: (\d+)", st, "0")
    fields = {
        "is_main": grab(r"is_main=(\w+)", r.get("P0_thread")),
        "thread_name": grab(r"thread_name=(\S+)", r.get("P0_thread")),
        "threads": grab(r"Threads: (\d+)", st),
        "VmSize_GiB": int(vms) >> 20,
        "free": grab(r"free=(\d+)", r.get("P0_meminfo_start")),
        "S2ptr": grab(r"ptr=(0x[0-9a-f]+)", r.get("S2_cuMemAlloc_v2_4")),
        "S3ptr": grab(r"ptr=(0x[0-9a-f]+)", r.get("S3_stream_alloc_u8_4")),
        "S5aptr": grab(r"ptr=(0x[0-9a-f]+)", r.get("S5a_alloc_zeros_u8_4")),
        "va1G": grab(r"va=(0x[0-9a-f]+)", r.get("S3v_cuMemAddressReserve_1024MiB")),
        "va32G": grab(r"va=(0x[0-9a-f]+)", r.get("S3v_cuMemAddressReserve_32768MiB")),
    }
    fields.update(f)
    print(r["run"], r["outcome"], " ".join(f"{k}={v}" for k, v in fields.items()))
