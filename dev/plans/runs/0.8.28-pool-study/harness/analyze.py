#!/usr/bin/env python3
"""0.8.28 pool study analysis (protocol sections 4.9, 6 and 8.1), Phases 0-1.

Usage: analyze.py <command> <paths...>
  equivalence DIR   Phase 0 control: DIR/prod-S, DIR/exp-S (run-*.json).
  seeds DIR         Phase 0 V8 --random-seed layout check (maps per run).
  c3 DIR...         C3 capacity (results.txt RESULT lines of pool_capacity).
  gap DIR           Contiguous need (points.txt / results.txt of gap-sweep.sh).
  pilot ROOT        Heap pilot: largest hole before open per heap cell.
  r5b ROOT          R5 revision 3 (chunked, interleaved cells pooled by label).
  poolloc ROOT...   Where the pool's first pages land relative to the driver reservation.
  phase2 ROOT       Phase 2 rows (C1/C2/C8/C9/C9b, CB1-CB4, trim arm).
  r5 DIR            R5 lazy creation: DIR/<cell>/run-*.json (+ maps).
  p1 DIR            P1 import cost: DIR/<variant>/run-*.json.
  hw DIR...         Workload high-water marks from teardown events.
Statistics: per-process medians; median and IQR across processes; ratio of
cell medians with a 95 % percentile bootstrap interval (10 000 resamples of
processes, seed 1); proportions with Wilson 95 % intervals. Run 000 (the
warm-cache run) is always excluded.
"""

import glob
import json
import math
import os
import random
import re
import statistics
import sys
from collections import defaultdict

GiB = 1 << 30
MiB = 1 << 20
LO, HI = 8 * GiB, 128 * GiB


def wilson(k, n, z=1.96):
    if n == 0:
        return (float("nan"), float("nan"))
    p = k / n
    d = 1 + z * z / n
    c = p + z * z / (2 * n)
    r = z * math.sqrt(p * (1 - p) / n + z * z / (4 * n * n))
    return ((c - r) / d, (c + r) / d)


def fmt_wilson(k, n):
    lo, hi = wilson(k, n)
    return f"{k}/{n} [{lo * 100:.1f}, {hi * 100:.1f}] %"


def med(xs):
    return statistics.median(xs) if xs else float("nan")


def iqr(xs):
    if len(xs) < 2:
        return (float("nan"), float("nan"))
    q = statistics.quantiles(xs, n=4, method="inclusive")
    return (q[0], q[2])


def boot_ratio(a, b, n=10000, seed=1):
    """95 % percentile interval for median(a)/median(b), resampling processes."""
    if not a or not b:
        return (float("nan"), float("nan"))
    rng = random.Random(seed)
    rs = []
    for _ in range(n):
        ra = [a[rng.randrange(len(a))] for _ in a]
        rb = [b[rng.randrange(len(b))] for _ in b]
        mb = statistics.median(rb)
        if mb:
            rs.append(statistics.median(ra) / mb)
    rs.sort()
    return (rs[int(0.025 * len(rs))], rs[int(0.975 * len(rs)) - 1])


def boot_diff(a, b, n=10000, seed=1):
    """95 % percentile interval for median(a) - median(b)."""
    if not a or not b:
        return (float("nan"), float("nan"))
    rng = random.Random(seed)
    ds = []
    for _ in range(n):
        ra = [a[rng.randrange(len(a))] for _ in a]
        rb = [b[rng.randrange(len(b))] for _ in b]
        ds.append(statistics.median(ra) - statistics.median(rb))
    ds.sort()
    return (ds[int(0.025 * n)], ds[int(0.975 * n) - 1])


def runs(d):
    out = []
    for p in sorted(glob.glob(os.path.join(d, "run-*.json"))):
        if os.path.basename(p) in ("run-000.json",) or p.endswith(".host.json") or p.endswith("host-after.json"):
            continue
        try:
            r = json.load(open(p))
        except (OSError, json.JSONDecodeError):
            continue
        r["_file"] = os.path.basename(p)
        out.append(r)
    return out


def steady_median(r):
    xs = (r.get("timingsMs") or {}).get("embedSteady") or []
    return med(xs) if xs else None


def inferred_path(steady_ms):
    """Slice 110's inference: stream-ordered steady embeds were 9.2-14.6 ms,
    synchronous 24.7-28.2 ms; 18 ms separates them."""
    if steady_ms is None:
        return None
    return "stream" if steady_ms < 18.0 else "sync"


# ---------------------------------------------------------------- equivalence
def cmd_equivalence(d):
    print("## Phase 0 production equivalence (full mode, Node 25, unobstructed)\n")
    print("| Build | Passed | Embed hashes | Rerank score sets | Inferred stream / sync | allocMode read | Steady median ms by path (n) |")
    print("| --- | --- | --- | --- | --- | --- | --- |")
    by_build = {}
    for b in ("prod-S", "exp-S"):
        rs = runs(os.path.join(d, b))
        by_build[b] = rs
        passed = [r for r in rs if r.get("outcome") == "pass"]
        hashes = sorted({r.get("embedSha") for r in passed})
        scores = sorted({json.dumps(r.get("rerankScores")) for r in passed})
        inf = defaultdict(list)
        for r in passed:
            inf[inferred_path(steady_median(r))].append(steady_median(r))
        modes = defaultdict(int)
        for r in passed:
            modes[r.get("allocMode")] += 1
        paths = "; ".join(f"{k}: {med(v):.1f} ({len(v)})" for k, v in sorted(inf.items(), key=str))
        print(f"| {b} | {len(passed)}/{len(rs)} | {', '.join(map(str, hashes))} | {len(scores)} | "
              f"{len(inf.get('stream', []))} / {len(inf.get('sync', []))} | "
              f"{', '.join(f'{k}={v}' for k, v in sorted(modes.items(), key=str))} | {paths} |")
    exp = [r for r in by_build["exp-S"] if r.get("outcome") == "pass"]
    agree = sum(1 for r in exp if {"default": "stream", "explicit": "stream", "sync": "sync"}.get(r.get("allocMode")) == inferred_path(steady_median(r)))
    print(f"\nInferred-vs-read path agreement (exp-S): {agree}/{len(exp)}")
    for path in ("stream", "sync"):
        p = [steady_median(r) for r in by_build["prod-S"] if r.get("outcome") == "pass" and inferred_path(steady_median(r)) == path]
        e = [steady_median(r) for r in exp if inferred_path(steady_median(r)) == path]
        if p and e:
            lo, hi = boot_ratio(e, p)
            print(f"Steady median ratio exp/prod on the {path} path: {med(e) / med(p):.3f} [{lo:.3f}, {hi:.3f}] (n={len(e)}/{len(p)}); pass if within 10 %")
        else:
            print(f"Steady median ratio on the {path} path: UNMEASURED (n exp={len(e)}, prod={len(p)})")
    for b, rs in by_build.items():
        opens = [r["timingsMs"]["open"] for r in rs if r.get("outcome") == "pass" and r["timingsMs"].get("open")]
        print(f"{b}: open median {med(opens):.0f} ms IQR {tuple(round(x) for x in iqr(opens))}; "
              f"gpuIdleBefore all true: {all(r['host'].get('gpuIdleBefore') for r in rs)}")


# ---------------------------------------------------------------------- seeds
def window_starts(path):
    starts = []
    for line in open(path):
        a, b = (int(x, 16) for x in line.split()[0].split("-"))
        if b > LO and a < HI:
            starts.append(max(a, LO))
    return starts


def cmd_seeds(d):
    by_seed = defaultdict(list)
    for r in runs(d):
        m = re.search(r"--random-seed=(\d+)", r.get("nodeFlags", ""))
        if not m:
            continue
        run_dir = os.path.join(d, "maps", r["_file"].replace(".json", ""))
        mp = glob.glob(os.path.join(run_dir, "maps-*-before-open.txt"))
        if mp:
            by_seed[int(m.group(1))].append((r["_file"], window_starts(mp[0])))
    print("## Phase 0 V8 --random-seed layout check (import-only, heap-400k)\n")
    print("| Seed | Runs | Window mappings (each run) | Identical starts | Common starts |")
    print("| --- | --- | --- | --- | --- |")
    identical = 0
    pairs = 0
    for seed in sorted(by_seed):
        rs = by_seed[seed]
        sets = [set(s) for _, s in rs]
        same = len(rs) >= 2 and all(s == sets[0] for s in sets)
        if len(rs) >= 2:
            pairs += 1
            identical += same
        common = len(set.intersection(*sets)) if sets else 0
        print(f"| {seed} | {len(rs)} | {', '.join(str(len(s)) for s in sets)} | {'yes' if same else 'no'} | {common} |")
    print(f"\nIdentical pairs: {identical}/{pairs} (paired layouts adopted if >= 9/10)")


# ------------------------------------------------------------------------- C3
def parse_results(paths):
    rows = []
    for d in paths:
        for line in open(os.path.join(d, "results.txt")):
            if not line.startswith("RESULT"):
                continue
            kv = dict(t.split("=", 1) for t in line.split()[1:] if "=" in t)
            rows.append(kv)
    return rows


def cmd_c3(*dirs):
    rows = [r for r in parse_results(dirs) if "allocated_mib" in r]
    cells = defaultdict(list)
    for r in rows:
        cells[(r["layout"], int(r["maxsize_mib"]), r["threshold"], int(r["chunk_mib"]))].append(r)
    print("## C3 capacity (pool_capacity, 5 runs per cell; medians)\n")
    print("| Layout | maxSize MiB | Threshold | Chunk MiB | n | Allocated MiB | First error | used_high MiB | reserved_high MiB | single max MiB | recovered MiB | runs identical |")
    print("| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |")
    by_size = defaultdict(list)
    for key in sorted(cells):
        rs = cells[key]
        alloc = [int(r["allocated_mib"]) for r in rs]
        errs = sorted({r["first_error"] for r in rs})
        uh = [int(r["used_high"]) / MiB for r in rs]
        rh = [int(r["reserved_high"]) / MiB for r in rs]
        single = [int(r["single_max_mib"]) for r in rs]
        rec = [int(r["recovered_mib"]) for r in rs]
        by_size[key[1]].append((key, med(alloc), med(single)))
        same = len(set(alloc)) == 1 and len(set(single)) == 1
        print(f"| {key[0]} | {key[1]} | {key[2]} | {key[3]} | {len(rs)} | {med(alloc):.0f} | {','.join(errs)} | "
              f"{med(uh):.1f} | {med(rh):.1f} | {med(single):.0f} | {med(rec):.0f} | {'yes' if same else 'no'} |")
    # Capacity model: capacity = a*maxSize + b, fitted on the per-size maximum
    # over chunks of (allocated + chunk residue is not added; allocated is a
    # multiple of the chunk, so the fit uses chunk-1 cells, the finest).
    pts = []
    for (layout, size, thr, chunk), rs in cells.items():
        if chunk == 1:
            pts.append((size, med([int(r["allocated_mib"]) for r in rs])))
    if len(pts) >= 2:
        n = len(pts)
        sx = sum(x for x, _ in pts)
        sy = sum(y for _, y in pts)
        sxx = sum(x * x for x, _ in pts)
        sxy = sum(x * y for x, y in pts)
        a = (n * sxy - sx * sy) / (n * sxx - sx * sx)
        b = (sy - a * sx) / n
        print(f"\nFit on chunk-1 cells (all layouts/thresholds): capacity_MiB = {a:.5f} * maxSize_MiB + {b:.2f}")
        worst = 0.0
        for size in sorted({x for x, _ in pts}):
            ys = [y for x, y in pts if x == size]
            pred = a * size + b
            res = max(abs(y - pred) / pred for y in ys)
            worst = max(worst, res)
            print(f"  maxSize {size}: measured {sorted(set(ys))} predicted {pred:.1f} worst residual {res * 100:.2f} %")
        print(f"Worst relative residual: {worst * 100:.2f} % (C3 pass needs <= 5 % at every size)")
        print("Rule ceil32(maxSize/3) check:")
        for size in sorted({x for x, _ in pts}):
            rule = math.ceil(size / 3 / 32) * 32
            ys = sorted({y for x, y in pts if x == size})
            print(f"  maxSize {size}: ceil32(maxSize/3) = {rule}, measured chunk-1 capacity {ys}")
    print("\nChunk dependence (per layout/threshold/size: allocated MiB by chunk; single max):")
    for size in sorted(by_size):
        for key, alloc, single in sorted(by_size[size]):
            pass
    groups = defaultdict(dict)
    singles = defaultdict(set)
    for (layout, size, thr, chunk), rs in cells.items():
        groups[(layout, size, thr)][chunk] = med([int(r["allocated_mib"]) for r in rs])
        singles[(layout, size, thr)].update(int(r["single_max_mib"]) for r in rs)
    for k in sorted(groups):
        print(f"  {k}: {dict(sorted(groups[k].items()))} single={sorted(singles[k])}")
    print("\nHost memory (medians over cells with chunk 1): MemAvailable and VmRSS deltas vs pool reserved")
    print("| Layout | maxSize MiB | Threshold | reserved at cap MiB | dMemAvail at cap MiB | dRSS at cap MiB | reserved after free MiB | dMemAvail after free MiB | dRSS after free MiB | reserved after trim MiB | dMemAvail after trim MiB | dRSS after trim MiB |")
    print("| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |")
    for key in sorted(cells):
        if key[3] != 1:
            continue
        rs = [r for r in cells[key] if "memavail_before_kib" in r]
        if not rs:
            continue

        def mm(f):
            return med([f(r) for r in rs])
        print(f"| {key[0]} | {key[1]} | {key[2]} | {mm(lambda r: int(r['reserved_cur_at_cap']) / MiB):.0f} | "
              f"{mm(lambda r: (int(r['memavail_before_kib']) - int(r['memavail_at_cap_kib'])) / 1024):.0f} | "
              f"{mm(lambda r: (int(r['rss_at_cap_kib']) - int(r['rss_before_kib'])) / 1024):.0f} | "
              f"{mm(lambda r: int(r['reserved_after_free']) / MiB):.0f} | "
              f"{mm(lambda r: (int(r['memavail_before_kib']) - int(r['memavail_after_free_kib'])) / 1024):.0f} | "
              f"{mm(lambda r: (int(r['rss_after_free_kib']) - int(r['rss_before_kib'])) / 1024):.0f} | "
              f"{mm(lambda r: int(r['reserved_after_trim']) / MiB):.0f} | "
              f"{mm(lambda r: (int(r['memavail_before_kib']) - int(r['memavail_after_trim_kib'])) / 1024):.0f} | "
              f"{mm(lambda r: (int(r['rss_after_trim_kib']) - int(r['rss_before_kib'])) / 1024):.0f} |")


# ------------------------------------------------------------------------ gap
def cmd_gap(d):
    pts = defaultdict(dict)
    for line in open(os.path.join(d, "points.txt")):
        m = re.match(r"POINT (\S+) maxsize=(\d+)GiB fails=(\d+)/3", line)
        if m:
            pts[m.group(1)][int(m.group(2))] = int(m.group(3))
    rows = parse_results([d])
    newmap = defaultdict(list)
    for r in rows:
        tag = r.get("tag", "")
        m = re.match(r"(\S+)-m(\d+)G#", tag)
        if m and r.get("create_rc") == "CUDA_SUCCESS" and "new_mmap_bytes" in r:
            newmap[(m.group(1), int(m.group(2)))].append(int(r["new_mmap_bytes"]))
    print("## Contiguous need (pool_gap, 3 runs per point)\n")
    print("| Series | Largest passing maxSize GiB | First failing maxSize GiB | Gap / largest passing maxSize | new mapping at largest pass (MiB) |")
    print("| --- | --- | --- | --- | --- |")
    def gap_key(k: str) -> tuple[str, float]:
        gm = re.search(r"-g(\d+)G", k)
        return (k.split("-g")[0], float(gm.group(1)) if gm else 0.0)

    for s in sorted(pts, key=gap_key):
        p = pts[s]
        passing = [m for m, f in p.items() if f == 0]
        failing = [m for m, f in p.items() if f > 0]
        lp = max([m for m in passing if not failing or m < min(failing)], default=None)
        ff = min(failing, default=None)
        g = re.search(r"-g(\d+)G", s)
        ratio = f"{int(g.group(1)) / lp:.3f}" if g and lp else "-"
        nm = newmap.get((s, lp), [])
        print(f"| {s} | {lp} | {ff if ff is not None else 'none <= sweep end'} | {ratio} | {', '.join(f'{x / MiB:.0f}' for x in nm) or '-'} |")
    print("\nNegative controls and others:")
    for line in open(os.path.join(d, "points.txt")):
        if "g0" in line or "slice110" in line:
            print("  " + line.strip())


# ------------------------------------------------------------------------- R5
def largest_hole(path):
    ivs = []
    for line in open(path):
        a, b = (int(x, 16) for x in line.split()[0].split("-"))
        if b > LO and a < HI:
            ivs.append((max(a, LO), min(b, HI)))
    ivs.sort()
    cur, best = LO, 0
    for a, b in ivs:
        if a > cur:
            best = max(best, a - cur)
        cur = max(cur, b)
    return max(best, HI - cur)


def new_bytes(pre, post):
    def load(p):
        return [tuple(int(x, 16) for x in ln.split()[0].split("-")) for ln in open(p)]
    a, b = load(pre), load(post)
    total = 0
    for lo, hi in b:
        cov = sum(max(0, min(hi, y) - max(lo, x)) for x, y in a)
        total += (hi - lo) - cov
    return total


def cmd_pilot(root):
    """Heap pilot (protocol revision 3, section 7): largest unmapped hole in
    [8, 128) GiB before open, per heap size, from import-only runs with MAPS=1."""
    print("## Heap pilot (import only, maps before open)\n")
    print("| Cell | n | Pass | heapUsed MiB (median) | VmRSS MiB (median) | Largest hole GiB median | min | max |")
    print("| --- | --- | --- | --- | --- | --- | --- | --- |")
    cells = sorted(glob.glob(os.path.join(root, "*")), key=lambda c: (len(c), c))
    for cell in cells:
        rs = runs(cell)
        if not rs:
            continue
        holes = []
        for r in rs:
            run = r["_file"][:-5]
            snaps = glob.glob(os.path.join(cell, "maps", run, "maps-*-before-open.txt"))
            if snaps:
                holes.append(largest_hole(snaps[0]) / GiB)
        heap = [r.get("heapUsedMiB") or 0 for r in rs]
        rss = [((r.get("mem") or {}).get("points") or {}).get("beforeOpen", {}).get("rssMiB", 0) for r in rs]
        passed = sum(r.get("outcome") == "pass" for r in rs)
        print(f"| {os.path.basename(cell)} | {len(rs)} | {passed} | {med(heap):.0f} | {med(rss):.0f} | "
              f"{med(holes) if holes else float('nan'):.2f} | {min(holes) if holes else float('nan'):.2f} | "
              f"{max(holes) if holes else float('nan'):.2f} |")


def cmd_r5(root):
    print("## R5 lazy creation after heap growth\n")
    print("| Cell | n | Pass | Pool created (install create+probe) | explicit | default | sync | default pool OOM (B) | min largest hole pre-pool GiB | new mapping at creation MiB (median) |")
    print("| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |")
    pooled = defaultdict(lambda: [0, 0, 0])
    misses = []
    for cell in sorted(os.listdir(root)):
        d = os.path.join(root, cell)
        if not os.path.isdir(d):
            continue
        rs = runs(d)
        if not rs:
            continue
        n = len(rs)
        passed = sum(r.get("outcome") == "pass" for r in rs)
        created = 0
        attempted = 0
        b_oom = 0
        holes, nbs = [], []
        for r in rs:
            ins = next((e for e in r.get("poolEvents", []) if e.get("event") == "install"), None)
            if ins and ins.get("create", "-") != "-":
                attempted += 1
                created += ins.get("create") == "CUDA_SUCCESS" and ins.get("probe") == "CUDA_SUCCESS"
            if ins and ins.get("default_pool") == "CUDA_ERROR_OUT_OF_MEMORY":
                b_oom += 1
            md = os.path.join(d, "maps", r["_file"].replace(".json", ""))
            pre = glob.glob(os.path.join(md, "maps-*-pre-pool.txt"))
            post = glob.glob(os.path.join(md, "maps-*-post-pool.txt"))
            if pre:
                h = largest_hole(pre[0])
                holes.append(h)
                if post:
                    nbs.append(new_bytes(pre[0], post[0]))
                ok = ins and ins.get("create") == "CUDA_SUCCESS"
                m = re.search(r"-(\d+)G", cell)
                if m:
                    need = math.ceil(int(m.group(1)) * 1024 / 3 / 32) * 32 * MiB
                    if bool(ok) != (h >= need):
                        misses.append(f"{cell}/{r['_file']}: created={bool(ok)} largest_hole={h / GiB:.2f} GiB need={need / GiB:.2f} GiB")
        modes = defaultdict(int)
        for r in rs:
            if r.get("outcome") == "pass":
                modes[r.get("allocMode")] += 1
        variant = cell.split("-node")[0]
        pooled[variant][0] += passed
        pooled[variant][1] += n
        pooled[variant][2] += modes.get("explicit", 0) + modes.get("default", 0)
        print(f"| {cell} | {n} | {fmt_wilson(passed, n)} | {fmt_wilson(created, attempted) if attempted else '-'} | "
              f"{fmt_wilson(modes.get('explicit', 0), passed)} | {modes.get('default', 0)} | {modes.get('sync', 0)} | "
              f"{b_oom} | {min(holes) / GiB if holes else float('nan'):.2f} | {med(nbs) / MiB if nbs else float('nan'):.0f} |")
    print("\nPooled per variant: pass and stream-ordered fraction of passing processes")
    for v, (p, n, s) in sorted(pooled.items()):
        print(f"  {v}: pass {fmt_wilson(p, n)}; stream-ordered {fmt_wilson(s, p)}; zero-failure bound 3/N = {300 / n:.2f} %")
    print(f"\nRule 'created iff largest unmapped hole pre-pool >= ceil32(maxSize/3)': {len(misses)} misses")
    for m in misses[:20]:
        print("  " + m)


def cmd_r5b(root):
    """R5 revision 3: cells may be spread over chunk directories (interleaved
    blocks) and a cell label may carry a -b suffix (a second copy of the same
    cell); both are pooled by the label without the suffix."""
    cells = defaultdict(list)
    for dirpath, _dirs, files in os.walk(root):
        if "series-header.txt" in files and any(f.startswith("run-") for f in files):
            label = re.sub(r"-b$", "", os.path.basename(dirpath))
            for r in runs(dirpath):
                r["_dir"] = dirpath
                cells[label].append(r)
    print("## R5 lazy creation after heap growth (revision 3)\n")
    print("| Cell | n | Pass (Wilson 95 %) | Pool created and probed | private | explicit | default | sync | "
          "B default-pool OOM | min / median largest hole pre-pool GiB | runs with hole < need, created | "
          "detect 5 % / 2 % | CB1 reserved_high max MiB |")
    print("| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |")
    pooled = defaultdict(lambda: [0, 0, 0, 0, 0])
    misses = []
    order = {"P-first-use": 0, "A-first-use": 1, "B": 2}

    def key(c):
        m = re.match(r"(.+)-node(\d+)-h(\d+)-", c)
        return (order.get(m.group(1), 9), int(m.group(2)), int(m.group(3))) if m else (9, 0, 0)

    for cell in sorted(cells, key=key):
        rs = cells[cell]
        n = len(rs)
        passed = sum(r.get("outcome") == "pass" for r in rs)
        created = attempted = b_oom = inside = 0
        holes, res_high = [], []
        for r in rs:
            ins = next((e for e in r.get("poolEvents", []) if e.get("event") == "install"), None)
            if ins and ins.get("create", "-") != "-":
                attempted += 1
                created += ins.get("create") == "CUDA_SUCCESS" and ins.get("probe") == "CUDA_SUCCESS"
            if ins and ins.get("default_pool") == "CUDA_ERROR_OUT_OF_MEMORY":
                b_oom += 1
            for e in r.get("poolEvents", []):
                if e.get("event") == "teardown" and e.get("reserved_high", "-") != "-":
                    res_high.append(int(e["reserved_high"]))
            md = os.path.join(r["_dir"], "maps", r["_file"].replace(".json", ""))
            pre = glob.glob(os.path.join(md, "maps-*-pre-pool.txt"))
            if pre:
                h = largest_hole(pre[0])
                holes.append(h)
                ok = bool(ins and ins.get("create") == "CUDA_SUCCESS")
                m = re.search(r"-(\d+)G", cell)
                need = math.ceil(int(m.group(1)) * 1024 / 3 / 32) * 32 * MiB if m else 0
                if h < need and ok:
                    inside += 1
                if h >= need and not ok:
                    misses.append(f"{cell}/{r['_file']}: not created with largest hole {h / GiB:.2f} GiB >= need {need / GiB:.2f} GiB")
        modes = defaultdict(int)
        for r in rs:
            if r.get("outcome") == "pass":
                modes[r.get("allocMode")] += 1
        variant = cell.split("-node")[0]
        pooled[variant][0] += passed
        pooled[variant][1] += n
        pooled[variant][2] += modes.get("explicit", 0) + modes.get("default", 0) + modes.get("private", 0)
        pooled[variant][3] += created
        pooled[variant][4] += attempted
        det5, det2 = 1 - 0.95 ** n, 1 - 0.98 ** n
        hs = sorted(holes)
        print(f"| {cell} | {n} | {fmt_wilson(passed, n)} | {fmt_wilson(created, attempted) if attempted else '-'} | "
              f"{modes.get('private', 0)} | {modes.get('explicit', 0)} | {modes.get('default', 0)} | {modes.get('sync', 0)} | "
              f"{b_oom} | {hs[0] / GiB if hs else float('nan'):.2f} / {med(hs) / GiB if hs else float('nan'):.2f} | "
              f"{inside} | {det5 * 100:.0f} % / {det2 * 100:.0f} % | {max(res_high) / MiB if res_high else float('nan'):.0f} |")
    print("\nPooled per variant: pass, pool created, and stream-ordered fraction of passing processes")
    for v, (p, n, s, c, a) in sorted(pooled.items()):
        print(f"  {v}: pass {fmt_wilson(p, n)}; created {fmt_wilson(c, a) if a else '-'}; "
              f"stream-ordered {fmt_wilson(s, p)}; zero-failure bound 3/N = {300 / n:.2f} %")
    print(f"\nOne-sided rule 'a largest unmapped hole >= ceil32(maxSize/3) suffices': {len(misses)} misses")
    for m in misses[:20]:
        print("  " + m)


def read_maps(path):
    out = []
    for line in open(path):
        f = line.split()
        lo, hi = (int(x, 16) for x in f[0].split("-"))
        out.append((lo, hi, f[1], f[5] if len(f) > 5 else ""))
    return out


def reservations(maps, min_bytes=GiB):
    """Large anonymous PROT_NONE ranges overlapping [8, 128) GiB, merged when
    adjacent (a range the driver later splits by mapping pages into it shows
    as several entries)."""
    merged = []
    for lo, hi, perms, name in maps:
        if perms.startswith("---") and not name and hi > LO and lo < HI:
            if merged and merged[-1][1] == lo:
                merged[-1][1] = hi
            else:
                merged.append([lo, hi])
    return [(lo, hi) for lo, hi in merged if hi - lo >= min_bytes]


def cmd_poolloc(*roots):
    """Where the pool's first pages land (pool study revision 3, coordinator
    question): the mappings that appear between the policy's pre-pool and
    post-pool snapshots (the probe allocation), classified as inside a
    pre-existing large PROT_NONE reservation in [8, 128) GiB, in an unmapped
    hole in the window, or outside the window. Also the window totals: large
    reservations and the largest unmapped hole."""
    cells = defaultdict(list)
    for root in roots:
        for dirpath, _dirs, files in os.walk(root):
            if os.path.basename(dirpath).startswith("run-") and os.path.basename(os.path.dirname(dirpath)) == "maps":
                cell = re.sub(r"-b$", "", os.path.basename(os.path.dirname(os.path.dirname(dirpath))))
                pre = glob.glob(os.path.join(dirpath, "maps-*-pre-pool.txt"))
                post = glob.glob(os.path.join(dirpath, "maps-*-post-pool.txt"))
                if pre and post:
                    cells[cell].append((pre[0], post[0]))
    print("| Cell | n | new pages inside a pre-existing reservation | in an unmapped window hole | outside [8, 128) GiB | "
          "none mapped | reservations >= 1 GiB, total GiB (median) | largest reservation GiB (median) | largest unmapped hole GiB (median) |")
    print("| --- | --- | --- | --- | --- | --- | --- | --- | --- |")
    for cell in sorted(cells):
        inside = hole = outside = none = 0
        tot, big, holes = [], [], []
        for pre, post in cells[cell]:
            a, b = read_maps(pre), read_maps(post)
            res = reservations(a)
            tot.append(sum(hi - lo for lo, hi in res) / GiB)
            big.append(max((hi - lo for lo, hi in res), default=0) / GiB)
            holes.append(largest_hole(pre) / GiB)
            before = {(lo, hi, perms, name) for lo, hi, perms, name in a}
            fresh = [m for m in b if m not in before and m[2] != "---p" and "dmabuf" in m[3]]
            if not fresh:
                none += 1
                continue
            lo, hi = fresh[0][0], fresh[0][1]
            if not (hi > LO and lo < HI):
                outside += 1
            elif any(r_lo <= lo and hi <= r_hi for r_lo, r_hi in res):
                inside += 1
            else:
                hole += 1
        n = len(cells[cell])
        print(f"| {cell} | {n} | {inside} | {hole} | {outside} | {none} | {med(tot):.2f} | {med(big):.2f} | {med(holes):.2f} |")


def _cells(root):
    cells = defaultdict(list)
    for dirpath, _dirs, files in os.walk(root):
        if "series-header.txt" in files and any(f.startswith("run-") for f in files):
            for r in runs(dirpath):
                r["_dir"] = dirpath
                cells[os.path.relpath(dirpath, root)].append(r)
    return cells


def _ev(r, name):
    return [e for e in r.get("poolEvents", []) if e.get("event") == name]


def _pair(raw):
    rc, _, handle = (raw or "-:-").partition(":")
    return rc, handle


def cmd_phase2(root):
    """Phase 2 (protocol revision 4): one table per row family."""
    cells = _cells(root)
    print("## Phase 2 outcomes per cell\n")
    print("| Cell | n | Pass (Wilson 95 %) | allocMode | decide mismatches | embed hashes | rerank score sets |")
    print("| --- | --- | --- | --- | --- | --- | --- |")
    for cell in sorted(cells):
        rs = cells[cell]
        n = len(rs)
        p = sum(r.get("outcome") == "pass" for r in rs)
        modes = defaultdict(int)
        for r in rs:
            modes[r.get("allocMode")] += 1
        mism = sum(1 for r in rs for e in _ev(r, "decide") if e.get("mismatch") == "1")
        hashes = {r.get("embedSha") for r in rs if r.get("embedSha")}
        scores = {json.dumps(r.get("rerankScores")) for r in rs if r.get("rerankScores")}
        print(f"| {cell} | {n} | {fmt_wilson(p, n)} | {dict(modes)} | {mism} | {', '.join(sorted(hashes)) or '-'} | {len(scores)} |")
    fails = [(c, r["_file"], r.get("failedStep"), (r.get("error") or {}).get("message", "")[:160])
             for c, rs in cells.items() for r in rs if r.get("outcome") != "pass"]
    if fails:
        print("\nFailures:")
        for f in fails[:30]:
            print("  ", *f)

    print("\n## C9 (amended rule) and C9b: current pool vs private and default\n")
    print("| Cell | n | teardown pairs read | current == private | current == default when both succeed | current OOM | co-resident default pool success |")
    print("| --- | --- | --- | --- | --- | --- | --- |")
    for cell in sorted(c for c in cells if "c9" in c or "c2-cycles" in c):
        rs = cells[cell]
        read = eq_priv = eq_def = both = oom = dsucc = 0
        for r in rs:
            for e in _ev(r, "teardown") + _ev(r, "install"):
                if "current_pool" not in e:
                    continue
                read += 1
                crc, ch = _pair(e["current_pool"])
                drc, dh = _pair(e.get("default_pool_pair"))
                eq_priv += ch == e.get("private_pool") and ch != "0x0"
                if crc == "CUDA_SUCCESS" and drc == "CUDA_SUCCESS":
                    both += 1
                    eq_def += ch == dh
                oom += crc == "CUDA_ERROR_OUT_OF_MEMORY"
                dsucc += drc == "CUDA_SUCCESS" and e.get("event") == "teardown"
            co = r.get("coresident")
            if co:
                dsucc += co.get("defaultPoolRc") == 0 and co.get("allocRc") == 0
        print(f"| {cell} | {len(rs)} | {read} | {eq_priv} | {eq_def}/{both} | {oom} | {dsucc}/{len(rs)} |")

    print("\n## CB rows\n")
    base = defaultdict(list)
    for cell, rs in cells.items():
        if "cb-S" in cell:
            for r in rs:
                if r.get("allocMode") == "sync":
                    pt = (r.get("mem") or {}).get("points", {}).get("afterRerank")
                    if pt:
                        base["h400k" if "h400k" in cell else "h0"].append(pt["rssMiB"] + pt.get("swapMiB", 0))
    print("CB1 baseline (S sync, VmRSS+VmSwap at afterRerank, MiB): " +
          ", ".join(f"{k}: median {med(v):.0f} (n={len(v)})" for k, v in sorted(base.items())))
    print("\n| Cell | n | reserved_high max MiB | VmRSS + VmSwap − baseline, max MiB | CB1 excess (that − reserved_high), max MiB; pass ≤ 32 | CB2 reserved_cur after close+idle MiB (median, max) | used_cur after close MiB (median) | spare (reserved − used) MiB max |")
    print("| --- | --- | --- | --- | --- | --- | --- | --- |")
    for cell in sorted(c for c in cells if "cb-P" in c):
        rs = cells[cell]
        key = "h400k" if "h400k" in cell else "h0"
        b = med(base[key]) if base[key] else float("nan")
        rh, ex, rc, uc, sp, ov = [], [], [], [], [], []
        for r in rs:
            td = _ev(r, "teardown")
            pt = (r.get("mem") or {}).get("points", {}).get("afterRerank")
            if not td or td[-1].get("reserved_high", "-") == "-":
                continue
            t = td[-1]
            rh.append(int(t["reserved_high"]) / MiB)
            rc.append(int(t["reserved_cur"]) / MiB)
            uc.append(int(t["used_cur"]) / MiB)
            sp.append((int(t["reserved_cur"]) - int(t["used_cur"])) / MiB)
            if pt:
                ov.append(pt["rssMiB"] + pt.get("swapMiB", 0) - b)
                ex.append(ov[-1] - int(t["reserved_high"]) / MiB)
        print(f"| {cell} | {len(rs)} | {max(rh) if rh else float('nan'):.0f} | {max(ov) if ov else float('nan'):.0f} | {max(ex) if ex else float('nan'):.0f} | "
              f"{med(rc) if rc else float('nan'):.0f}, {max(rc) if rc else float('nan'):.0f} | {med(uc) if uc else float('nan'):.0f} | {max(sp) if sp else float('nan'):.0f} |")
    print("\n| CB3/CB4 cell | n | oversize failed | error class / code / kind | next embed on cuda with same hash | exhausted events |")
    print("| --- | --- | --- | --- | --- | --- |")
    for cell in sorted(c for c in cells if "cb34" in c):
        rs = cells[cell]
        over = [r.get("oversize") or {} for r in rs]
        failed = sum(o.get("ok") is False for o in over)
        kinds = defaultdict(int)
        for o in over:
            e = o.get("error") or {}
            kinds[f"{e.get('name')}/{e.get('code')}/{e.get('kind')}"] += 1
        same = sum(bool(o.get("sameHash")) and o.get("deviceAfter") == "cuda" for o in over)
        exh = sum(len(_ev(r, "exhausted")) for r in rs)
        print(f"| {cell} | {len(rs)} | {failed} | {dict(kinds)} | {same}/{len(rs)} | {exh} |")

    print("\n## Trim arm\n")
    for cell in sorted(c for c in cells if "trim" in c):
        rs = cells[cell]
        trims = [int(_ev(r, "teardown")[-1].get("trims", 0)) for r in rs if _ev(r, "teardown") and _ev(r, "teardown")[-1].get("trims")]
        line = f"{cell}: n={len(rs)} pass={sum(r.get('outcome') == 'pass' for r in rs)} trims/process median={med(trims) if trims else 0}"
        cyc = [c for r in rs for c in ((r.get("trim") or {}).get("cycles") or [])]
        if cyc:
            line += (f"; regrow embed ms median {med([c['embedMs'] for c in cyc]):.2f} vs steady {med([c['embedSteadyMs'] for c in cyc]):.2f}; "
                     f"rerank {med([c['rerankMs'] for c in cyc]):.2f}; cls {med([c['clsMs'] for c in cyc]):.2f} (cycles={len(cyc)})")
            hs = {h for r in rs for h in (r.get("trim") or {}).get("hashes", [])}
            line += f"; distinct embed hashes {len(hs)}"
        st = [r.get("stress") for r in rs if r.get("stress")]
        if st:
            line += (f"; stress embeds main={sum(s['iters'] for s in st)} workers={sum(s['workerEmbeds'] for s in st)} "
                     f"mismatches={sum(s['mismatches'] + s['workerMismatches'] for s in st)}")
        print(line)
    if any("trimcycle" in c for c in cells):
        for key in ("embedMs", "rerankMs", "clsMs", "embedSteadyMs"):
            # Unit is the process: the median of its cycles.
            per = {arm: [med([c[key] for c in (r.get("trim") or {}).get("cycles", [])])
                         for cell in cells if arm in cell for r in cells[cell]
                         if (r.get("trim") or {}).get("cycles")]
                   for arm in ("trim-idle", "trim-off")}
            a, b = per["trim-idle"], per["trim-off"]
            if a and b:
                lo, hi = boot_ratio(a, b)
                print(f"After-idle {key}, trim idle 200 ms vs off: medians {med(a):.2f} vs {med(b):.2f} ms, "
                      f"ratio {med(a) / med(b):.3f} [{lo:.3f}, {hi:.3f}] (processes {len(a)}/{len(b)})")


# ------------------------------------------------------------------------- P1
def cmd_p1(root):
    data = {}
    for v in sorted(os.listdir(root)):
        d = os.path.join(root, v)
        if os.path.isdir(d):
            data[v] = [r for r in runs(d) if r.get("outcome") == "pass"]
    print("## P1 import cost (installed, Node 25, import-only, 3 GiB, threshold 0)\n")
    print("| Variant | n | Import ms median (IQR) | RSS delta MiB median (IQR) | VmSize delta MiB median | Pool events |")
    print("| --- | --- | --- | --- | --- | --- |")
    for v, rs in data.items():
        imp = [r["timingsMs"]["import"] for r in rs]
        rss = [r["mem"]["importRssDeltaMiB"] for r in rs]
        vsz = [r["mem"]["importVszDeltaMiB"] for r in rs]
        ev = defaultdict(int)
        for r in rs:
            for e in r.get("poolEvents", []):
                ev[f"{e.get('event')}:{e.get('create', '-')}/{e.get('probe', '-')}/{e.get('release', '-')}"] += 1
        q1, q3 = iqr(imp)
        r1, r3 = iqr(rss)
        print(f"| {v} | {len(rs)} | {med(imp):.1f} ({q1:.1f}–{q3:.1f}) | {med(rss):.1f} ({r1:.1f}–{r3:.1f}) | {med(vsz):.0f} | {dict(ev)} |")
    s = data.get("S")
    if s:
        si = [r["timingsMs"]["import"] for r in s]
        sr = [r["mem"]["importRssDeltaMiB"] for r in s]
        print("\nGate (plan): import <= S + 5 ms and RSS <= S + 32 MiB")
        for v, rs in data.items():
            if v == "S":
                continue
            vi = [r["timingsMs"]["import"] for r in rs]
            vr = [r["mem"]["importRssDeltaMiB"] for r in rs]
            di = boot_diff(vi, si)
            dr = boot_diff(vr, sr)
            dmi, dmr = med(vi) - med(si), med(vr) - med(sr)
            verdict = "PASS" if di[1] <= 5 and dr[1] <= 32 else ("FAIL" if di[0] > 5 or dr[0] > 32 else "UNDECIDED")
            print(f"  {v}: import +{dmi:.1f} ms [{di[0]:.1f}, {di[1]:.1f}]; RSS +{dmr:.1f} MiB [{dr[0]:.1f}, {dr[1]:.1f}] -> {verdict}")


# ------------------------------------------------------------------------- hw
def cmd_hw(*dirs):
    print("## Workload high-water marks (teardown events)\n")
    for d in dirs:
        hs = []
        for r in runs(d):
            t = next((e for e in r.get("poolEvents", []) if e.get("event") == "teardown"), None)
            if t and t.get("used_high", "-") != "-":
                hs.append((int(t["used_high"]), int(t["reserved_high"])))
        if hs:
            print(f"  {d.rstrip('/').split('/')[-1]}: n={len(hs)} used_high max {max(h[0] for h in hs) / MiB:.1f} MiB, "
                  f"reserved_high max {max(h[1] for h in hs) / MiB:.1f} MiB")


if __name__ == "__main__":
    globals()["cmd_" + sys.argv[1]](*sys.argv[2:])
