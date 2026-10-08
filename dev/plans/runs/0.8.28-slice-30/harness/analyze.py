#!/usr/bin/env python3
"""Qualification analysis: one text summary per gate of plan 4.2.

Usage: analyze.py <series-root> <gate> [<study-root-for-nothing>]
Gates: g1 g2 g3 g4 g5 g6 g7 g8 g9. Reads run-NNN.json (run-000, the
warm-cache run, is skipped) under <series-root>/<gate>/. Prints the numbers
and a PASS/FAIL line per criterion; the verdict text in qualification.md is
written from this output. Bootstrap: 10 000 resamples, process as the unit.
"""

from __future__ import annotations

import glob
import json
import math
import os
import random
import statistics
import sys
from collections import Counter

MIB = 1 << 20


def wilson(k: int, n: int, z: float = 1.96) -> tuple[float, float]:
    if n == 0:
        return (float("nan"), float("nan"))
    p = k / n
    d = 1 + z * z / n
    c = p + z * z / (2 * n)
    r = z * math.sqrt(p * (1 - p) / n + z * z / (4 * n * n))
    return ((c - r) / d, (c + r) / d)


def fw(k: int, n: int) -> str:
    lo, hi = wilson(k, n)
    return f"{k}/{n} [{lo * 100:.1f}, {hi * 100:.1f}] %"


def med(xs: list[float]) -> float:
    return statistics.median(xs) if xs else float("nan")


def boot(a: list[float], b: list[float], kind: str, n: int = 10000, seed: int = 1) -> tuple[float, float]:
    """95 % percentile interval for median(a)/median(b) (kind 'ratio') or median(a)-median(b) ('diff')."""
    if not a or not b:
        return (float("nan"), float("nan"))
    rng = random.Random(seed)
    out = []
    for _ in range(n):
        ma = statistics.median([a[rng.randrange(len(a))] for _ in a])
        mb = statistics.median([b[rng.randrange(len(b))] for _ in b])
        if kind == "ratio":
            if mb:
                out.append(ma / mb)
        else:
            out.append(ma - mb)
    out.sort()
    return (out[int(0.025 * len(out))], out[int(0.975 * len(out)) - 1])


def runs(d: str) -> list[dict]:
    out = []
    for p in sorted(glob.glob(os.path.join(d, "run-*.json"))):
        base = os.path.basename(p)
        if base == "run-000.json" or "host" in base:
            continue
        try:
            r = json.load(open(p))
        except (OSError, json.JSONDecodeError):
            continue
        r["_file"] = base
        out.append(r)
    return out


def alloc_key(r: dict) -> str:
    a = r.get("allocator")
    return f"{a['path']}/{a['reason']}" if a else "none"


def outcome_line(rs: list[dict]) -> str:
    oc = Counter(r.get("outcome") for r in rs)
    ex = Counter(r.get("exitCode") for r in rs)
    return f"n={len(rs)} outcomes {dict(oc)} exit codes {dict(ex)}"


def verdict(ok: bool, text: str) -> None:
    print(f"  {'PASS' if ok else 'FAIL'}: {text}")


def g1(root: str) -> None:
    for lang in ("node", "py"):
        rs = runs(f"{root}/g1/{lang}")
        print(f"G1 {lang}: {outcome_line(rs)}")
        paths = Counter(alloc_key(r) for r in rs)
        print(f"  allocator path/reason {dict(paths)}")
        print(f"  poolMaxSizeBytes {dict(Counter((r.get('allocator') or {}).get('poolMaxSizeBytes') for r in rs))}")
        print(f"  releaseThreshold {dict(Counter((r.get('allocator') or {}).get('releaseThreshold') for r in rs))}")
        print(f"  moduleLoadInit {dict(Counter((r.get('allocator') or {}).get('moduleLoadInit') for r in rs))}")
        print(f"  reranker allocator {dict(Counter(str(r.get('rerankerAllocator')) for r in rs))}")
        ok = all(r.get("outcome") == "pass" for r in rs) and set(paths) == {"private/private_pool"}
        verdict(ok, f"{fw(sum(r.get('outcome') == 'pass' for r in rs), len(rs))} pass, every report private/3 GiB")


def g2(root: str) -> None:
    for d in sorted(glob.glob(f"{root}/g2/*")):
        name = os.path.basename(d)
        rs = runs(d)
        print(f"G2 {name}: {outcome_line(rs)}")
        if "fork" in name:
            kids = Counter()
            for r in rs:
                res = (r.get("forkChild") or {}).get("result") or {}
                err = res.get("error") or {}
                kids[(res.get("ok"), res.get("device"), err.get("kind") or err.get("name"), (res.get("allocator") or {}).get("reason"))] += 1
            print(f"  child (ok, device, error kind, allocator reason): {dict(kids)}")
            print(f"  parent moduleLoadInit at import {dict(Counter(r.get('moduleLoadInitAtImport') for r in rs))}")
            continue
        print(f"  allocator path/reason {dict(Counter(alloc_key(r) for r in rs))}")
        print(f"  moduleLoadInit {dict(Counter((r.get('allocator') or {}).get('moduleLoadInit') for r in rs))}")
        print(f"  device {dict(Counter(r.get('embedderDevice') for r in rs))} reason {dict(Counter(r.get('embedderReason') for r in rs))}")
        nonprivate = all((r.get("allocator") or {}).get("path") != "private" for r in rs)
        verdict(all(r.get("outcome") == "pass" for r in rs) and nonprivate,
                f"{sum(r.get('outcome') == 'pass' for r in rs)}/{len(rs)} opened and embedded (never refused), none private")


def growth(x: list[float]) -> float | None:
    return (x[-1] - x[1]) / (len(x) - 2) if len(x) >= 4 else None


def g3(root: str) -> None:
    limits = {"node": (100, 0.36, 0.44), "py": (50, 0.36, 0.44)}
    for lang, (cycles, lim_med, lim_max) in limits.items():
        rs = runs(f"{root}/g3/{lang}")
        print(f"G3 {lang}: {outcome_line(rs)} cycles={cycles}")
        gr = sorted(g for g in (growth(r.get("cyclesRssMiB") or []) for r in rs) if g is not None)
        print(f"  VmRSS growth per cycle (cycles 2..N), MiB: median {med(gr):.3f} max {max(gr) if gr else float('nan'):.3f} (n={len(gr)}), all {[round(x, 3) for x in gr]}")
        print(f"  cycle allocator paths {dict(Counter(tuple(r.get('cyclePaths') or []) for r in rs))}")
        # After the last close and 10 s idle: system and process residue.
        for r in rs[:0]:
            pass
        resid = []
        for r in rs:
            pts = r["mem"]["points"]
            if "afterIdle" in pts and "beforeOpen" in pts:
                resid.append((pts["beforeOpen"]["memAvailMiB"] - pts["afterIdle"]["memAvailMiB"], pts["afterIdle"]["rssMiB"] - pts["beforeOpen"]["rssMiB"]))
        if resid:
            print(f"  MemAvailable drop beforeOpen->afterIdle MiB median {med([a for a, _ in resid]):.0f} (range {min(a for a, _ in resid):.0f}..{max(a for a, _ in resid):.0f}); process RSS rise median {med([b for _, b in resid]):.0f} MiB")
        ok = (
            all(r.get("outcome") == "pass" for r in rs)
            and all(r.get("cyclePaths") == ["private/private_pool"] for r in rs)
            and bool(gr) and med(gr) <= lim_med and max(gr) <= lim_max
        )
        verdict(ok, f"all cycles private; growth median {med(gr):.3f} <= {lim_med}, max {max(gr) if gr else float('nan'):.3f} <= {lim_max}")
    print("  UNMEASURED: reserved_cur after the last close (the product does not expose pool counters); see the system-level residue above.")


def g3s(root: str) -> None:
    for lang, cycles in (("node", 100), ("py", 50)):
        rs = runs(f"{root}/g3s/{lang}")
        gr = sorted(g for g in (growth(r.get("cyclesRssMiB") or []) for r in rs) if g is not None)
        print(f"G3 control, pool off, {lang}: {outcome_line(rs)} cycles={cycles}; paths {dict(Counter(tuple(r.get('cyclePaths') or []) for r in rs))}")
        print(f"  VmRSS growth per cycle (cycles 2..N), MiB: median {med(gr):.3f} max {max(gr) if gr else float('nan'):.3f} (n={len(gr)}), all {[round(x, 3) for x in gr]}")
        resid = [
            (r["mem"]["points"]["beforeOpen"]["memAvailMiB"] - r["mem"]["points"]["afterIdle"]["memAvailMiB"], r["mem"]["points"]["afterIdle"]["rssMiB"] - r["mem"]["points"]["beforeOpen"]["rssMiB"])
            for r in rs if "afterIdle" in r["mem"]["points"]
        ]
        if resid:
            print(f"  MemAvailable drop beforeOpen->afterIdle MiB median {med([a for a, _ in resid]):.0f} (range {min(a for a, _ in resid):.0f}..{max(a for a, _ in resid):.0f}); process RSS rise median {med([b for _, b in resid]):.0f} MiB")


def g4(root: str) -> None:
    for lang in ("node", "py"):
        rs = runs(f"{root}/g4/{lang}")
        print(f"G4 {lang}: {outcome_line(rs)}")
        ov = [r.get("oversize") or {} for r in rs]
        print(f"  oversize ok flags {dict(Counter(o.get('ok') for o in ov))}")
        print(f"  error {dict(Counter((o.get('error') or {}).get('name') for o in ov))} code/kind {dict(Counter((o.get('error') or {}).get('code') or (o.get('error') or {}).get('kind') for o in ov))}")
        print(f"  error ordinal {dict(Counter((o.get('error') or {}).get('ordinal') for o in ov))} maxSize {dict(Counter((o.get('error') or {}).get('maxSizeBytes') for o in ov))}")
        print(f"  next embed ok {dict(Counter(o.get('embedAfterOk') for o in ov))} sameHash {dict(Counter(o.get('sameHash') for o in ov))} deviceAfter {dict(Counter(o.get('deviceAfter') for o in ov))} pathAfter {dict(Counter(o.get('pathAfter') for o in ov))}")
        ok = bool(rs) and all(
            r.get("outcome") == "pass" and (r.get("oversize") or {}).get("ok") is False
            and ((r["oversize"].get("error") or {}).get("code") == "FDB_CUDA_POOL_EXHAUSTED" or (r["oversize"].get("error") or {}).get("name") == "CudaPoolExhaustedError")
            and r["oversize"].get("embedAfterOk") and r["oversize"].get("sameHash") and r["oversize"].get("deviceAfter") == "cuda"
            for r in rs
        )
        verdict(ok, "typed CudaPoolExhaustedError, next embed on cuda with the pre-error hash")


def g5(root: str) -> None:
    for lang in ("node", "py"):
        p = runs(f"{root}/g1/{lang}")
        s = runs(f"{root}/g5/{lang}-S")
        print(f"G5 {lang}: P n={len(p)} S n={len(s)}")
        for key in ("embedSha", "clsSha", "rerankScores"):
            pv = Counter(json.dumps(r.get(key)) for r in p)
            sv = Counter(json.dumps(r.get(key)) for r in s)
            print(f"  {key}: P {dict(pv)} ; S {dict(sv)}")
        print(f"  S paths {dict(Counter(alloc_key(r) for r in s))}")
        ok = all(len({json.dumps(r.get('embedSha')) for r in p + s}) == 1 for _ in [0]) and (
            lang != "py" or len({json.dumps(r.get("clsSha")) for r in p + s}) == 1
        )
        if lang == "node":
            ok = ok and len({json.dumps(r.get("rerankScores")) for r in p + s}) == 1
        verdict(ok, f"{lang}: embedding hash{' , rerank scores' if lang == 'node' else ', cls hash'} identical P vs S")
    pp = runs(f"{root}/g9/py/pyP")
    ps = runs(f"{root}/g9/py/pyS")
    if pp and ps:
        sc = {json.dumps(r.get("rerankScores")) for r in pp + ps}
        eh = {r.get("embedSha") for r in pp + ps}
        print(f"G5 py rerank-cuda wheel (G9 series): P n={len(pp)} S n={len(ps)} rerank score sets {sc} embed hashes {eh}")
        verdict(len(sc) == 1 and len(eh) == 1, "py (rerank-cuda wheel): rerank scores and embedding hash identical P vs S")


def g6(root: str) -> None:
    for d in sorted(glob.glob(f"{root}/g6/*")):
        rs = runs(d)
        print(f"G6 {os.path.basename(d)}: {outcome_line(rs)}")
        print(f"  allocator {dict(Counter(alloc_key(r) for r in rs))} later reports {dict(Counter(k for r in rs for k in r.get('allocatorsLater', [])))}")
        print(f"  heapObjects {dict(Counter(r.get('heapObjects') for r in rs))} heapUsedMiB median {med([r.get('heapUsedMiB') or 0 for r in rs])}")
        ok = bool(rs) and all(
            r.get("outcome") == "pass" and alloc_key(r) == "private/private_pool"
            and set(r.get("allocatorsLater", [])) <= {"private/private_pool"} and r.get("embedShaAfterGrow") == r.get("embedSha")
            for r in rs
        )
        verdict(ok, f"{fw(sum(r.get('outcome') == 'pass' for r in rs), len(rs))} private path held through growth, second embed and rerank")


def g7(root: str) -> None:
    for d in sorted(glob.glob(f"{root}/g7/k*"), key=lambda x: int(x.rsplit("k", 1)[1])):
        rs = [r for r in runs(d)]
        print(f"G7 {os.path.basename(d)}: {outcome_line(rs)} paths {dict(Counter(alloc_key(r) for r in rs))}")
        floors = []
        for t in glob.glob(f"{d}/mem-*.tsv"):
            vals = [int(line.split("\t")[1]) for line in open(t) if line.count("\t") >= 2]
            if vals:
                floors.append(min(vals) / MIB)
        print(f"  min MemAvailable across trials GiB: {min(floors) if floors else float('nan'):.1f}")
        summ = open(f"{d}/summary.txt").read() if os.path.exists(f"{d}/summary.txt") else ""
        ok = bool(rs) and all(r.get("outcome") == "pass" and r.get("exitCode") == 0 for r in rs) and "STOP" not in summ and "FLOOR" not in summ and "REFUSED" not in summ
        verdict(ok, f"no failure or OOM kill; {sum(r.get('outcome') == 'pass' for r in rs)}/{len(rs)} processes")


def g8(root: str, d: str = "g8") -> None:
    rs = runs(f"{root}/{d}")
    for r in rs:
        lang = "node" if "node" in r else "py"
        soak = r.get("soak") or {}
        s = soak.get("samples") or []
        print(f"G8 {lang if r.get('node') else 'py'}: outcome {r.get('outcome')} iterations {soak.get('iterations')} errors {len(soak.get('errors') or [])} mismatches {soak.get('mismatches')} aborted {soak.get('aborted')} allocator {alloc_key(r)}")
        if not s:
            continue
        rss = [x["rssMiB"] for x in s]
        first5 = [x["rssMiB"] for x in s if x["minute"] <= 5]
        high = max(first5) if first5 else max(rss)
        late = rss[-5:]
        drift = (med(late) - med(first5)) / med(first5) if first5 else float("nan")
        print(f"  samples {len(s)}; RSS first-5-min high {high:.0f} MiB, max {max(rss):.0f}, last {rss[-1]:.0f}; late-5 median {med(late):.0f} vs first-5 median {med(first5):.0f} (drift {drift * 100:.1f} %)")
        print(f"  swap MiB max {max(x['swapMiB'] for x in s):.1f}; MemAvailable MiB min {min(x['memAvailMiB'] for x in s):.0f}; embed ms first {s[0]['embedMs']:.1f} last {s[-1]['embedMs']:.1f}")
        ok = r.get("outcome") == "pass" and not soak.get("errors") and max(rss) <= 1.10 * high and abs(drift) < 0.10
        verdict(ok, f"no allocator error; RSS within 10 % of the first-5-minute high ({max(rss) / high * 100 - 100:+.1f} %); drift {drift * 100:+.1f} % < 10 %")
    print("  UNMEASURED: reserved memory (pool counters are not exposed); RSS and system MemAvailable stand in.")


MEASURES = ["embed", "b1", "b8", "b32", "b128", "rerank"]


def per_process(r: dict) -> dict[str, float]:
    t = r.get("timingsMs") or {}
    out = {}
    if t.get("embedSteady"):
        out["embed"] = med(t["embedSteady"])
    for size in (1, 8, 32, 128):
        v = (t.get("embedBatch") or {}).get(str(size))
        if v:
            out[f"b{size}"] = med(v)
    if t.get("rerankSteady"):
        out["rerank"] = med(t["rerankSteady"])
    if t.get("import") is not None:
        out["import"] = t["import"]
    pts = (r.get("mem") or {}).get("points") or {}
    if pts:
        out["rssMax"] = max(p["rssMiB"] for p in pts.values())
    return out


def collect(d: str, only_path: str | None = None) -> dict[str, list[float]]:
    """Per-process measures of one cell; a Node cell is pooled over every
    g9/node* directory (a stopped series continues in node2, node3)."""
    acc: dict[str, list[float]] = {}
    dirs = sorted(glob.glob(d.replace("/g9/node/", "/g9/node*/"))) if "/g9/node/" in d else [d]
    for r in (r for x in dirs for r in runs(x)):
        if r.get("outcome") != "pass":
            continue
        if only_path and (r.get("allocator") or {}).get("path") != only_path:
            continue
        for k, v in per_process(r).items():
            acc.setdefault(k, []).append(v)
    return acc


def g9(root: str) -> None:
    node = f"{root}/g9/node"
    py = f"{root}/g9/py"
    for lang, cells in (("node", ("prodP", "studyP", "S")), ("py", ("pyP", "pyS"))):
        for c in cells:
            rs = [r for x in sorted(glob.glob(f"{root}/g9/{lang}*/{c}")) for r in runs(x)]
            if rs:
                print(f"G9 {lang} {c}: {outcome_line(rs)} paths {dict(Counter(alloc_key(r) for r in rs))}")
    p = collect(f"{node}/prodP")
    sp = collect(f"{node}/studyP")
    s = collect(f"{node}/S")
    s_sync = collect(f"{node}/S", "synchronous")
    print(f"\nNode 25 (medians of per-process medians, ms; n P={len(p.get('embed', []))}, studyP={len(sp.get('embed', []))}, S={len(s.get('embed', []))}, S-sync={len(s_sync.get('embed', []))})")
    print(f"{'measure':8} {'prodP':>9} {'studyP':>9} {'S':>9} {'Ssync':>9}  prodP/studyP [CI]            S/prodP [CI] (all S)        Ssync/prodP [CI]")
    ok_ratio = True
    ok_speed = True
    for m in MEASURES:
        a, b, c, e = p.get(m, []), sp.get(m, []), s.get(m, []), s_sync.get(m, [])
        r1 = boot(a, b, "ratio") if b else (float("nan"),) * 2
        r2 = boot(c, a, "ratio")
        r3 = boot(e, a, "ratio") if e else (float("nan"),) * 2
        m1 = med(a) / med(b) if b else float("nan")
        print(f"{m:8} {med(a):9.2f} {med(b) if b else float('nan'):9.2f} {med(c):9.2f} {med(e) if e else float('nan'):9.2f}  {m1:.3f} [{r1[0]:.3f}, {r1[1]:.3f}]   {med(c) / med(a):.2f} [{r2[0]:.2f}, {r2[1]:.2f}]   {med(e) / med(a) if e else float('nan'):.2f} [{r3[0]:.2f}, {r3[1]:.2f}]")
        if b:
            ok_ratio &= m1 <= 1.05
            if r1[1] > 1.10:
                print(f"    NOTE {m}: CI upper {r1[1]:.3f} > 1.10 (plan: double N once)")
        ok_speed &= (r3[0] if e else r2[0]) > 1.5
    verdict(ok_ratio, "Node product/study-P <= 1.05 per measure" if sp else "study P absent: not evaluated")
    verdict(ok_speed, "Node speed-up over S > 1.5 (CI lower bound; S-sync subset where present, else all S)")
    if "import" in p and "import" in s:
        d = med(p["import"]) - med(s["import"])
        lo, hi = boot(p["import"], s["import"], "diff")
        print(f"Node import: P {med(p['import']):.1f} ms, S {med(s['import']):.1f} ms, diff {d:.1f} [{lo:.1f}, {hi:.1f}]")
        verdict(d <= 5, "Node import <= S + 5 ms (point estimate)")
        dr = med(p["rssMax"]) - med(s["rssMax"])
        lo, hi = boot(p["rssMax"], s["rssMax"], "diff")
        print(f"Node peak RSS: P {med(p['rssMax']):.0f} MiB, S {med(s['rssMax']):.0f} MiB, diff {dr:.1f} [{lo:.1f}, {hi:.1f}]")
        verdict(dr <= 32, "Node RSS <= S + 32 MiB (point estimate)")
    pp, ps = collect(f"{py}/pyP"), collect(f"{py}/pyS")
    if pp and ps:
        print(f"\nPython (rerank-cuda wheel; n P={len(pp.get('embed', []))}, S={len(ps.get('embed', []))}); S = pool off and early cuInit off")
        ok = True
        for m in MEASURES:
            a, b = pp.get(m, []), ps.get(m, [])
            r = boot(a, b, "ratio")
            ratio = med(a) / med(b)
            print(f"{m:8} P {med(a):8.2f} S {med(b):8.2f}  P/S {ratio:.3f} [{r[0]:.3f}, {r[1]:.3f}]")
            ok &= ratio <= 1.15
            if r[1] > 1.15:
                print(f"    NOTE {m}: CI upper {r[1]:.3f} > 1.15")
        verdict(ok, "Python P/S <= 1.15 per measure (point estimate)")
        pi = {c: collect(f"{root}/g9/pyimp/{c}") for c in ("pyimpP", "pyimpS", "pyimpI")}
        if all(pi[c].get("import") for c in pi):
            ip, i_s, ii = (med(pi[c]["import"]) for c in ("pyimpP", "pyimpS", "pyimpI"))
            cuinit = ii - i_s
            print(f"Python import (n={len(pi['pyimpP']['import'])} each): P {ip:.1f} ms, S(no hook) {i_s:.1f} ms, I(hook, pool off) {ii:.1f} ms; measured cuInit cost {cuinit:.1f} ms")
            lo, hi = boot(pi["pyimpP"]["import"], pi["pyimpS"]["import"], "diff")
            print(f"  P - S {ip - i_s:.1f} [{lo:.1f}, {hi:.1f}] ms; allowed S + cuInit + 5 = {cuinit + 5:.1f} ms")
            verdict(ip - i_s <= cuinit + 5, "Python import <= S + measured cuInit + 5 ms (SD-4)")


GATES = {"g8s": lambda root: g8(root, "g8s"), "g3s": g3s, "g1": g1, "g2": g2, "g3": g3, "g4": g4, "g5": g5, "g6": g6, "g7": g7, "g8": g8, "g9": g9}

if __name__ == "__main__":
    GATES[sys.argv[2]](sys.argv[1])
