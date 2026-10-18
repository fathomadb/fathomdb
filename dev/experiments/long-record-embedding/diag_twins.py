"""Body-identical sessions in LongMemEval-S, and how many P2 gold sessions have twins.

Usage: python diag_twins.py   writes results/diag_twins.json
"""

from __future__ import annotations

import json
from collections import defaultdict

import size_corpora as sc
from probe import RESULTS
from probe2 import plan


def main() -> None:
    original, _, questions, _ = plan(sc.DEFAULT_DATA)
    by_body: dict[str, list[str]] = defaultdict(list)
    for sid, body in original.items():
        by_body[body].append(sid)
    groups = [sids for sids in by_body.values() if len(sids) > 1]
    gold = {g for q in questions for g in q["gold"]}
    out = {
        "sessions": len(original),
        "twin_groups": len(groups),
        "sessions_in_twin_groups": sum(len(g) for g in groups),
        "p2_gold_sessions": len(gold),
        "p2_gold_sessions_with_twin": sum(1 for g in gold if len(by_body[original[g]]) > 1),
        "p2_questions_with_twinned_gold": sum(
            1 for q in questions if any(len(by_body[original[g]]) > 1 for g in q["gold"])
        ),
    }
    (RESULTS / "diag_twins.json").write_text(json.dumps(out, indent=2) + "\n")
    print(json.dumps(out, indent=2))


if __name__ == "__main__":
    main()
