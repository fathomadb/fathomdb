"""Single-caller search latency on the probe store (no contention).

Usage: FATHOMDB_EMBED_DEVICE=cuda:0 python latency.py --db PATH --out results/latency_<store>.json [--n 200]
Writes p50/p95/p99 ms per call shape; results/latency.json is the hand-combined table.
"""

from __future__ import annotations

import argparse
import json
import time
from pathlib import Path

import size_corpora as sc


def _pcts(xs: list[float]) -> dict:
    xs = sorted(xs)
    return {p: round(xs[min(len(xs) - 1, int(q * len(xs)))], 2) for p, q in (("p50", 0.5), ("p95", 0.95), ("p99", 0.99))}


def main() -> None:
    from fathomdb.engine import Engine

    ap = argparse.ArgumentParser()
    ap.add_argument("--db", type=Path, required=True)
    ap.add_argument("--n", type=int, default=200)
    ap.add_argument("--out", type=Path, required=True)
    a = ap.parse_args()
    questions = [q["question"] for q in sc._lme(sc.DEFAULT_DATA)][: a.n]
    engine = Engine.open(str(a.db), use_default_embedder=True)
    for q in questions[:20]:
        engine.search(q, limit=10)
    shapes = {
        "text_limit10": lambda q: engine.search_text_only(q, limit=10),
        "hybrid_limit10": lambda q: engine.search(q, limit=10),
        "hybrid_limit10_explain": lambda q: engine.search(q, limit=10, explain=True),
        "hybrid_limit100": lambda q: engine.search(q, limit=100),
    }
    out = {"n": len(questions)}
    for name, call in shapes.items():
        ts = []
        for q in questions:
            t0 = time.perf_counter()
            call(q)
            ts.append((time.perf_counter() - t0) * 1e3)
        out[name] = _pcts(ts)
        print(name, out[name], flush=True)
    a.out.write_text(json.dumps(out, indent=2) + "\n")


if __name__ == "__main__":
    main()
