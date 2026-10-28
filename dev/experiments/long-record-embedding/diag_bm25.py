"""Raw FTS5 bm25 rank of a question's gold session on two stores (read-only).

The match expression approximates fathomdb-query's content-OR (tokens >= 3 chars);
the same expression is used on both stores, so only the stores can differ.
Usage: python diag_bm25.py --original A --rotated B --qid QID [...]
"""

from __future__ import annotations

import argparse
import re
import sqlite3
from pathlib import Path

import size_corpora as sc


def rank(db: Path, expr: str, gold: list[str]) -> list[tuple[str, int, float]]:
    con = sqlite3.connect(f"file:{db}?mode=ro", uri=True)
    try:
        rows = con.execute(
            "SELECT cn.source_id, bm25(search_index) FROM search_index "
            "JOIN canonical_nodes cn ON cn.write_cursor = search_index.write_cursor "
            "WHERE search_index MATCH ? ORDER BY rank LIMIT 200",
            (expr,),
        ).fetchall()
    finally:
        con.close()
    ids = [r[0] for r in rows]
    return [(g, ids.index(g) + 1, round(rows[ids.index(g)][1], 4)) for g in gold if g in ids]


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--original", type=Path, required=True)
    ap.add_argument("--rotated", type=Path, required=True)
    ap.add_argument("--qid", nargs="+", required=True)
    a = ap.parse_args()
    qs = {q["question_id"]: q for q in sc._lme(sc.DEFAULT_DATA)}
    for qid in a.qid:
        q = qs[qid]
        toks = dict.fromkeys(t for t in re.findall(r"[a-z0-9]+", q["question"].lower()) if len(t) >= 3)
        expr = " OR ".join(f'"{t}"' for t in toks)
        print(qid, "original", rank(a.original, expr, q["answer_session_ids"]),
              "rotated", rank(a.rotated, expr, q["answer_session_ids"]))


if __name__ == "__main__":
    main()
