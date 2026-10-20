"""Combine results/latency_*.json and fit hybrid-minus-text cost against vector rows.

Usage: python latency_fit.py   writes results/latency.json
Least squares over the short-body stores (1,500-char bodies), p50 and p95.
"""

from __future__ import annotations

import json
from pathlib import Path

RESULTS = Path(__file__).parent / "results"
SHORT = {"short1500_n4800": 4800, "short1500_n9600": 9600, "short1500_n19195": 19195, "short1500_x2": 38390}


def _fit(xs: list[float], ys: list[float]) -> tuple[float, float]:
    n = len(xs)
    mx, my = sum(xs) / n, sum(ys) / n
    slope = sum((x - mx) * (y - my) for x, y in zip(xs, ys)) / sum((x - mx) ** 2 for x in xs)
    return slope, my - slope * mx


def main() -> None:
    stores = {p.stem.removeprefix("latency_"): json.loads(p.read_text())
              for p in sorted(RESULTS.glob("latency_*.json"))}
    out: dict = {"_comment": "Single-caller ms, 60 LongMemEval-S questions, 20 warm-up calls, query embed on RTX 3090 cuda:0.",
                 "stores": stores, "fit": {}}
    for pct in ("p50", "p95"):
        xs = [SHORT[k] for k in SHORT if k in stores]
        ys = [stores[k]["hybrid_limit10"][pct] - stores[k]["text_limit10"][pct] for k in SHORT if k in stores]
        slope, icpt = _fit(xs, ys)
        out["fit"][f"hybrid_minus_text_{pct}"] = {
            "per_vector_row_us": round(slope * 1000, 2), "fixed_ms": round(icpt, 1),
            "points": {k: round(y, 1) for k, y in zip([k for k in SHORT if k in stores], ys)},
            "residuals_ms": {k: round(y - (icpt + slope * x), 1) for k, x, y in zip([k for k in SHORT if k in stores], xs, ys)},
        }
    full = stores.get("full_bodies")
    if full:
        out["fit"]["full_bodies_n19195_hybrid_minus_text_p50"] = round(
            full["hybrid_limit10"]["p50"] - full["text_limit10"]["p50"], 1)
    (RESULTS / "latency.json").write_text(json.dumps(out, indent=2) + "\n")
    print(json.dumps(out["fit"], indent=2))


if __name__ == "__main__":
    main()
