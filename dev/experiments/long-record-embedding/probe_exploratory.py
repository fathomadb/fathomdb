"""Exploratory (not preregistered) cuts of the P1 probe: R@5 effects, rescue counts, types.

Usage: python probe_exploratory.py   (reads results/probe_per_question.jsonl)
"""

from __future__ import annotations

import json
import random
from collections import Counter
from pathlib import Path

RESULTS = Path(__file__).parent / "results"


def _mean(xs: list[int]) -> float:
    return sum(xs) / len(xs)


def effect(sample: dict[str, list[dict]], arm: str, k: int) -> float:
    def gap(a: str) -> float:
        return _mean([r[f"{a}@{k}"] for r in sample["B"]]) - _mean([r[f"{a}@{k}"] for r in sample["P"]])

    return gap(arm) - gap("text")


def main() -> None:
    rows = [json.loads(line) for line in (RESULTS / "probe_per_question.jsonl").open()]
    strata = {s: [r for r in rows if r.get("measured") and r["stratum"] == s] for s in "BP"}
    rng = random.Random(20261018)
    out: dict = {}
    for arm in ("vector", "hybrid"):
        boots = sorted(effect({s: [rng.choice(v) for _ in v] for s, v in strata.items()}, arm, 5) for _ in range(10_000))
        out[f"E({arm})@5"] = {"point": effect(strata, arm, 5), "ci95": [boots[250], boots[9750]]}
    for s, v in strata.items():
        out[f"{s}: hybrid@10 hit, text@10 miss"] = sum(bool(r["hybrid@10"] and not r["text@10"]) for r in v)
        out[f"{s}: text@10 hit, hybrid@10 miss"] = sum(bool(r["text@10"] and not r["hybrid@10"]) for r in v)
        out[f"{s}: vector@10 hit, text@10 miss"] = sum(bool(r["vector@10"] and not r["text@10"]) for r in v)
    by_type: dict = {}
    for t in sorted({r["type"] for r in strata["B"] + strata["P"]}):
        sub = {s: [r for r in v if r["type"] == t] for s, v in strata.items()}
        if sub["B"] and sub["P"]:
            by_type[t] = {
                "nB": len(sub["B"]), "nP": len(sub["P"]),
                "E(vector)@10": effect(sub, "vector", 10), "E(hybrid)@10": effect(sub, "hybrid", 10),
            }
    weights = {t: min(v["nB"], v["nP"]) for t, v in by_type.items()}
    for arm in ("vector", "hybrid"):
        out[f"E({arm})@10 within-type, min-n weighted"] = sum(
            weights[t] * v[f"E({arm})@10"] for t, v in by_type.items()
        ) / sum(weights.values())
    out["by_type"] = by_type
    out["mean_gold_sessions"] = {s: _mean([len(r["gold"]) for r in v]) for s, v in strata.items()}
    out["types"] = {f"{s}/{t}": n for (s, t), n in sorted(Counter((r["stratum"], r["type"]) for r in rows if r["stratum"] in "BP").items())}
    (RESULTS / "probe_exploratory.json").write_text(json.dumps(out, indent=2) + "\n")
    print(json.dumps(out, indent=2))


if __name__ == "__main__":
    main()
