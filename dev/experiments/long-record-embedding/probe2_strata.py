"""Per-stratum paired effects for P2b, from results/probe2b_per_question.jsonl.

Descriptive follow-up to the preregistered pooled estimator (re-review N1): the
pooled D can hide opposite-signed stratum effects. Writes results/probe2b_strata.json.
"""

from __future__ import annotations

import json
import random
import statistics

from probe import RESULTS, SEED

BOOT = 10_000


def main() -> None:
    rows = [json.loads(line) for line in (RESULTS / "probe2b_per_question.jsonl").open()]
    rng = random.Random(SEED)
    out: dict = {}
    for stratum in ("P", "B"):
        sub = [r for r in rows if r["measured"] and r["stratum"] == stratum]
        for arm in ("hybrid", "vector"):
            ds = [r["visible"][f"{arm}@10"] - r["hidden"][f"{arm}@10"] for r in sub]
            boots = sorted(statistics.fmean(rng.choice(ds) for _ in ds) for _ in range(BOOT))
            out[f"{stratum}: D({arm})"] = {
                "n": len(ds), "point": statistics.fmean(ds),
                "ci95": [boots[int(0.025 * BOOT)], boots[int(0.975 * BOOT)]],
                "se": statistics.pstdev(boots),
                "discordant": {"+1": ds.count(1), "-1": ds.count(-1)},
            }
    (RESULTS / "probe2b_strata.json").write_text(json.dumps(out, indent=2) + "\n")
    print(json.dumps(out, indent=2))


if __name__ == "__main__":
    main()
