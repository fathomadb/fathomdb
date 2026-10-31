#!/usr/bin/env python3
"""Apply PREREG.md's decision rules to results.json.

  python3 dev/experiments/ac013-seed-drain/analyze.py dev/experiments/ac013-seed-drain/results.json

Prints, from timed (untraced, exit 0) runs: per-vector drain time per cell and
N with the t(N)/t(10k) ratios (rule 2) and a least-squares fit
drain = a*N + b*N^2 over the medians with the N^2 share at 100k; the backlog
share at 100k under CS64 (rule 3); store-digest equality of each -AL cell
against its shipped-scan cell (rule 4); the fix criterion (rule 5). From traced
runs: scheduler-scan and writer candidate-page page requests per run (rule 1).
"""

from __future__ import annotations

import json
import statistics
import sys
from pathlib import Path

PAIRS = [("CS64", "CS64-AL"), ("BASE-c4000", "BASE-c4000-AL"), ("CS64-o", "CS64-AL-o"),
         ("CS64-t1", "CS64-AL-t1")]
HELD = ["BASE", "BASE-c4000", "BASE-c4000-AL", "CS64", "CS64-AL"]
SCHED = "SELECT pending.write_cursor, pending.kind, pending.body"
CAND = "SELECT pending.write_cursor,pending.kind FROM"


def runs(data: dict, cell: str, n: int, traced: bool = False) -> list[dict]:
    entry = data.get(f"{cell}@n{n}", {"runs": []})
    return [r for r in entry["runs"] if r["traced"] == traced and r["exit"] == 0]


def med(values):
    values = [v for v in values if v is not None]
    return statistics.median(values) if values else None


def fit(points: list[tuple[int, float]]) -> tuple[float, float]:
    """Least squares for y = a*N + b*N^2 (no intercept)."""
    s11 = sum(n * n for n, _ in points)
    s12 = sum(n ** 3 for n, _ in points)
    s22 = sum(n ** 4 for n, _ in points)
    t1 = sum(n * y for n, y in points)
    t2 = sum(n * n * y for n, y in points)
    det = s11 * s22 - s12 * s12
    return (t1 * s22 - t2 * s12) / det, (s11 * t2 - s12 * t1) / det


def stmt_per_run(run: dict, role: str, prefix: str) -> float | None:
    rows = [r for r in run.get("trace_top", []) if r["role"] == role and r["sql"].startswith(prefix)]
    total_runs = sum(r["runs"] for r in rows)
    if not total_runs:
        return None
    return sum(r["exclusive_hit"] + r["exclusive_miss"] for r in rows) / total_runs


def fmt(value: float | None) -> str:
    return f"{value:,.0f}" if value is not None else "n/a (not in top 12)"


def main() -> None:
    data = json.loads(Path(sys.argv[1]).read_text())
    ns = sorted({c["n"] for c in data.values()})
    print("Rule 2: per-vector held drain (us), median of timed runs; ratio to 10k")
    for cell in HELD:
        pts = []
        row = []
        for n in ns:
            d = med([r["drain_ms"] for r in runs(data, cell, n)])
            if d is None:
                continue
            pts.append((n, d))
            row.append(f"{n // 1000}k={d * 1000 / n:.1f}")
        base = pts[0][1] * 1000 / pts[0][0] if pts else None
        ratios = " ".join(f"{n // 1000}k:{(d * 1000 / n) / base:.2f}" for n, d in pts[1:])
        a, b = fit(pts) if len(pts) >= 2 else (0, 0)
        share = b * 1e10 / (a * 1e5 + b * 1e10) if pts else 0
        verdict = ("linear" if all((d * 1000 / n) / base <= (1.5 if n > 100_000 else 1.3)
                                   for n, d in pts[1:]) else
                   "super-linear" if any((d * 1000 / n) / base >= 1.5 for n, d in pts[1:] if n <= 100_000)
                   else "between")
        print(f"  {cell:<14} {' '.join(row):<40} ratios {ratios:<22} fit a={a * 1000:.1f} us/vec "
              f"b={b * 1e6:.4f} ms/1e6 N^2 share@100k={share:.0%} -> {verdict}")

    print("Rule 3: backlog share at 100k")
    for shipped, fixed, field in (("CS64", "CS64-AL", "drain_ms"), ("CS64-o", "CS64-AL-o", "seed_ms")):
        s = med([r["seed_ms"] for r in runs(data, shipped, 100_000)])
        a = med([r[field] for r in runs(data, shipped, 100_000)])
        b = med([r[field] for r in runs(data, fixed, 100_000)])
        if None not in (s, a, b):
            print(f"  ({shipped} {field} {a:,.0f} - {fixed} {b:,.0f}) / {shipped} seed {s:,.0f} "
                  f"= {(a - b) / s:.1%}")

    print("Rule 4: store digests, -AL cell vs shipped cell at the same N")
    ok = True
    for shipped, fixed in PAIRS:
        for n in ns:
            ref = {json.dumps(r["digest"], sort_keys=True)
                   for r in runs(data, shipped, n) + runs(data, shipped, n, True)}
            got = {json.dumps(r["digest"], sort_keys=True)
                   for r in runs(data, fixed, n) + runs(data, fixed, n, True)}
            if not ref or not got:
                continue
            same = ref == got and len(ref) == 1
            ok &= same
            print(f"  {fixed:<14} n={n:<7} {'identical' if same else 'DIFFERENT'} "
                  f"({len(runs(data, fixed, n)) + len(runs(data, fixed, n, True))} runs)")
    print(f"  all identical: {ok}")

    print("Rule 5: CS64-AL vs CS64 held drain")
    a100 = [r["drain_ms"] for r in runs(data, "CS64", 100_000)]
    b100 = [r["drain_ms"] for r in runs(data, "CS64-AL", 100_000)]
    print(f"  100k: CS64 {a100} CS64-AL {b100}; ratio per repeat "
          f"{[round(b / a, 3) for a, b in zip(a100, b100)]} (need <= 0.8)")
    a10 = med([r["drain_ms"] for r in runs(data, "CS64", 10_000)])
    b10 = med([r["drain_ms"] for r in runs(data, "CS64-AL", 10_000)])
    print(f"  10k: CS64 median {a10:,.0f} CS64-AL median {b10:,.0f} -> {b10 / a10 - 1:+.1%} (need <= +5%)")

    print("Rule 1: page requests per run (traced)")
    for cell in ("BASE-c4000", "CS64", "CS64-AL", "CS64-t1", "CS64-AL-t1"):
        for n in ns:
            for r in runs(data, cell, n, True):
                sched = stmt_per_run(r, "projection_dispatcher", SCHED)
                cand = stmt_per_run(r, "writer", CAND)
                writer = next((x for x in r.get("trace_by_role", []) if x["role"] == "writer"), None)
                cand_rows = [x for x in r.get("trace_top", [])
                             if x["role"] == "writer" and x["sql"].startswith(CAND)]
                cand_req = sum(x["exclusive_hit"] + x["exclusive_miss"] for x in cand_rows)
                writer_req = (writer["exclusive_hit"] + writer["exclusive_miss"]) if writer else 0
                share = f"{cand_req / writer_req:.0%}" if writer_req and cand_rows else "n/a"
                print(f"  {cell:<12} n={n:<7} scheduler scan req/run={fmt(sched)} "
                      f"writer probe req/run={fmt(cand)} probe share of writer drain requests={share} "
                      f"probes={r['phase']['drain_probes']}")


if __name__ == "__main__":
    main()
