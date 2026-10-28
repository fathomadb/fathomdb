"""Why does the text arm disagree between the original and rotated stores in P2?

Usage: python diag_text.py --original A.sqlite --rotated B.sqlite [--n 15]
For the first n disagreeing questions: repeatability on each store, and the
gold session's text rank and score on both.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

import size_corpora as sc
from probe import RESULTS


def main() -> None:
    from fathomdb.engine import Engine

    ap = argparse.ArgumentParser()
    ap.add_argument("--original", type=Path, required=True)
    ap.add_argument("--rotated", type=Path, required=True)
    ap.add_argument("--n", type=int, default=15)
    ap.add_argument("--embedder", action="store_true")
    a = ap.parse_args()
    text_of = {q["question_id"]: q["question"] for q in sc._lme(sc.DEFAULT_DATA)}
    rows = [json.loads(line) for line in (RESULTS / "probe2_per_question.jsonl").open()]
    bad = [r for r in rows if r["visible"]["text@10"] != r["hidden"]["text@10"]][: a.n]
    eng = {k: Engine.open(str(p), use_default_embedder=a.embedder) for k, p in (("original", a.original), ("rotated", a.rotated))}
    gold_of = {q["question_id"]: q["answer_session_ids"] for q in sc._lme(sc.DEFAULT_DATA)}
    out = []
    for r in bad:
        q = text_of[r["qid"]]
        rec = {"qid": r["qid"], "stratum": r["stratum"]}
        for name, e in eng.items():
            runs = [[(h.source_id, round(h.score, 4)) for h in e.search_text_only(q, limit=50).results] for _ in range(2)]
            ids = [s for s, _ in runs[0]]
            gold = [g for g in gold_of[r["qid"]] if g in ids]
            rec[name] = {
                "repeatable": runs[0] == runs[1],
                "gold_rank_score": [(ids.index(g) + 1, runs[0][ids.index(g)][1]) for g in gold],
                "top3": runs[0][:3],
            }
        out.append(rec)
        print(json.dumps(rec), flush=True)
    (RESULTS / "diag_text.json").write_text(json.dumps(out, indent=2) + "\n")


if __name__ == "__main__":
    main()
