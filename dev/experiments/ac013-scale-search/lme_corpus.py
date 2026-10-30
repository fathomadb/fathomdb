#!/usr/bin/env python3
"""Write a LongMemEval-S turn corpus and question set as JSONL for the AC-013
scale-search probe (`exp_ac013_scale_search.rs`, `AC013_CORPUS_FILE` /
`AC013_QUERY_FILE`).

The text stays out of the repository: point `--lme` at the Hugging Face
`longmemeval_s_cleaned.json` and `--out` at a scratch directory.

- `corpus.jsonl`: one JSON string per line, each a unique non-empty turn
  `content`, in first-seen order (questions in file order, then haystack
  sessions in listed order, each session once, turns in order). 189,514
  turns for the cleaned S file.
- `queries.jsonl`: the 500 `question` strings, in file order.

Usage:
  python3 -I lme_corpus.py --lme <longmemeval_s_cleaned.json> --out <dir>
"""

import argparse
import json
import os


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--lme", required=True)
    ap.add_argument("--out", required=True)
    args = ap.parse_args()
    with open(args.lme, encoding="utf-8") as f:
        data = json.load(f)
    sessions: set[str] = set()
    turns: set[str] = set()
    os.makedirs(args.out, exist_ok=True)
    n = 0
    with open(os.path.join(args.out, "corpus.jsonl"), "w", encoding="utf-8") as out:
        for question in data:
            for sid, session in zip(question["haystack_session_ids"], question["haystack_sessions"]):
                if sid in sessions:
                    continue
                sessions.add(sid)
                for turn in session:
                    text = turn["content"].strip()
                    if text and text not in turns:
                        turns.add(text)
                        out.write(json.dumps(text) + "\n")
                        n += 1
    with open(os.path.join(args.out, "queries.jsonl"), "w", encoding="utf-8") as out:
        for question in data:
            out.write(json.dumps(question["question"]) + "\n")
    print(json.dumps({"sessions": len(sessions), "turns": n, "queries": len(data)}))


if __name__ == "__main__":
    main()
