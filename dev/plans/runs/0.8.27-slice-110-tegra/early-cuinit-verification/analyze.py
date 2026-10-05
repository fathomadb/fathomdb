#!/usr/bin/env python3
"""Summarise the Slice 110 early-cuInit verification series.

Usage: analyze.py <logs-dir>

Each series directory holds run-NN.out (the consumer's JSON line),
run-NN.err (its stderr) and series-header.txt. For every series this prints
the pass count, every failure with its step and message, the inferred
allocator split, the steady-embed medians, witness deltas, the import cost
and whether the embeddings agree. The allocator is not reported by the
product: as in ../fix-verification/analyze.py, a steady embed median above
18 ms is read as synchronous allocation and below as stream-ordered.
"""

from __future__ import annotations

import json
import statistics
import sys
from pathlib import Path

SYNC_MS = 18.0


def runs(series: Path) -> list[tuple[str, dict, int]]:
    out = []
    for path in sorted(series.glob("run-*.out")):
        text = path.read_text().strip()
        record = json.loads(text) if text else {"outcome": "no-output"}
        err = path.with_suffix(".err")
        stderr_lines = len(err.read_text().splitlines()) if err.exists() else -1
        out.append((path.stem, record, stderr_lines))
    return out


def fmt(xs: list[float]) -> str:
    if not xs:
        return "-"
    return f"median={statistics.median(xs):.1f} min={min(xs):.1f} max={max(xs):.1f} (n={len(xs)})"


def main() -> int:
    root = Path(sys.argv[1])
    heads = set()
    ratios = []
    for series in sorted(p for p in root.iterdir() if p.is_dir()):
        records = runs(series)
        header = (series / "series-header.txt").read_text().splitlines()[0]
        passed = [r for _, r, _ in records if r.get("outcome") == "pass"]
        print(f"== {series.name}: {len(passed)} / {len(records)} pass  [{header}]")
        stderr = sorted({n for _, _, n in records})
        print(f"   stderr lines per run: {stderr}")
        for name, record, _ in records:
            if record.get("outcome") != "pass":
                error = record.get("error") or {}
                print(
                    f"   FAIL {name} step={record.get('failedStep')} code={error.get('code')} "
                    f"kind={error.get('kind')} heapUsedMiB={record.get('heapUsedMiB')}\n"
                    f"        message={error.get('message')}"
                )
        imports = [r["timingsMs"]["import"] for _, r, _ in records if "import" in r.get("timingsMs", {})]
        vsz = [r["mem"]["importVszDeltaMiB"] for _, r, _ in records if "importVszDeltaMiB" in r.get("mem", {})]
        rss = [r["mem"]["importRssDeltaMiB"] for _, r, _ in records if "importRssDeltaMiB" in r.get("mem", {})]
        heap = [r["heapUsedMiB"] for _, r, _ in records if "heapUsedMiB" in r]
        print(f"   import ms {fmt(imports)}")
        print(f"   import VmSize delta MiB {fmt(vsz)}; VmRSS delta MiB {fmt(rss)}")
        print(f"   heapUsed MiB before open {fmt(heap)}")
        steady = [r["timingsMs"]["embedSteadyMedian"] for r in passed if "embedSteadyMedian" in r["timingsMs"]]
        if steady:
            sync = [s for s in steady if s > SYNC_MS]
            asyn = [s for s in steady if s <= SYNC_MS]
            cuda = all(r.get("embedderDevice") == "cuda" for r in passed)
            if cuda:
                print(f"   inferred allocator: {len(sync)} synchronous, {len(asyn)} stream-ordered")
                print(f"   steady embed ms, synchronous: {fmt(sync)}")
                print(f"   steady embed ms, stream-ordered: {fmt(asyn)}")
                if sync and asyn:
                    ratio = statistics.median(sync) / statistics.median(asyn)
                    ratios.append((series.name, ratio))
                    print(f"   median ratio synchronous / stream-ordered: {ratio:.2f}")
            else:
                print(f"   steady embed ms (CPU): {fmt(steady)}")
            devices = sorted({(r.get("embedderDevice"), r.get("embedderReason")) for r in passed}, key=str)
            print(f"   embedder device, reason: {devices}")
            witness = [r["witnessDeltaMiB"] for r in passed if r.get("witnessDeltaMiB") is not None]
            if witness:
                print(f"   witness delta MiB {fmt(witness)}")
            first = [r["timingsMs"]["embedFirst"] for r in passed]
            opens = [r["timingsMs"]["open"] for r in passed]
            print(f"   open ms {fmt(opens)}; first embed ms {fmt(first)}")
            for r in passed:
                heads.add((r.get("embedderDevice"), tuple(round(x, 6) for x in r.get("embedHead", []))))
    print(f"== distinct leading embedding values by device: {sorted(heads, key=str)}")
    if ratios:
        print("== per-series synchronous / stream-ordered steady-embed median ratios:")
        for name, ratio in ratios:
            print(f"   {name}: {ratio:.2f}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
