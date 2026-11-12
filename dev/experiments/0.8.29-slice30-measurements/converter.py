"""Convert pinned LongMemEval-S turns/questions for Slice30; no answer fields."""

import argparse
import hashlib
import json
from pathlib import Path


def convert(data, limit):
    """Return first-seen unique nonempty turns and questions with counts."""
    sessions = set()
    turns = set()
    bodies = []
    for question in data:
        for sid, session in zip(question["haystack_session_ids"], question["haystack_sessions"]):
            if sid in sessions:
                continue
            sessions.add(sid)
            for turn in session:
                text = turn["content"].strip()
                if text and text not in turns:
                    turns.add(text)
                    bodies.append(text)
    return {
        "bodies": bodies[:limit],
        "queries": [question["question"] for question in data],
        "sessions": len(sessions),
        "total_unique_turns": len(bodies),
        "selected_count": len(bodies[:limit]),
    }


def main():
    """Write corpus/query JSONL and a compact input/count record."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--lme", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--limit", type=int, default=100000)
    args = parser.parse_args()
    raw = args.lme.read_bytes()
    result = convert(json.loads(raw), args.limit)
    args.out.mkdir(parents=True, exist_ok=False)
    for name, key in (("corpus", "bodies"), ("queries", "queries")):
        payload = "".join(json.dumps(text) + "\n" for text in result.pop(key)).encode()
        (args.out / f"{name}.jsonl").write_bytes(payload)
        result[f"{name}_sha256"] = hashlib.sha256(payload).hexdigest()
    result["raw_sha256"] = hashlib.sha256(raw).hexdigest()
    (args.out / "inputs.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result))


if __name__ == "__main__":
    main()
