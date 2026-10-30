#!/usr/bin/env python3
"""How many documents each query's text arm matches (AC-013 workload realism).

Builds an FTS5 table with the engine's tokenizer (`porter unicode61
remove_diacritics 2`), compiles each query the way `fathomdb-query`
`compile_text_query` does (lowercase, split on non-alphanumeric, drop the
default stop words, de-duplicate, OR of quoted terms), and prints the match
count distribution at each requested corpus size (a prefix of the corpus).

Corpora:
- `--synthetic`: the AC-013 fixture generator (`perf_gates.rs`
  `ac013_corpus_bodies` / `ac013_query_bodies`, copied in
  `tests/exp_ac013_vector_reads.rs`), re-implemented here bit-for-bit.
- `--corpus FILE --queries FILE`: JSONL files of strings (`lme_corpus.py`).

The Rust probe (`exp_ac013_scale_search.rs`) reports the same counts from the
engine's own store; this script is the quick cross-check and reaches sizes the
probe does not seed.

Usage:
  python3 -I match_counts.py --synthetic --sizes 10000,30000,100000 --queries-n 1000
  python3 -I match_counts.py --corpus c.jsonl --queries q.jsonl --sizes 10000,100000
"""

import argparse
import json
import sqlite3
import statistics

STOPWORDS = set(
    """the and for are was were what when where who whom which how why did does do is of to in on at by an a it
its this that these those with from as be or if about into over than then they them their you your we our i s
t ll re ve m my me us am so up""".split()
)
MASK = (1 << 64) - 1
CORPUS_SEED = 0x0AC013D013D0
QUERY_SEED = 0x0AC0130DEC0DE000


class Rng:
    def __init__(self, seed: int) -> None:
        self.state = (seed * 0x9E3779B97F4A7C15 + 1) & MASK

    def next_u64(self) -> int:
        self.state = (self.state * 6364136223846793005 + 1442695040888963407) & MASK
        return self.state

    def next_in(self, bound: int) -> int:
        return self.next_u64() % bound

    def next_f64(self) -> float:
        return (self.next_u64() >> 11) / float(1 << 53)


def synthetic(seed: int, count: int) -> list[str]:
    vocab = []
    for i in range(1024):
        a = chr(ord("a") + (i // 26 // 26) % 26)
        b = chr(ord("a") + (i // 26) % 26)
        c = chr(ord("a") + i % 26)
        vocab.append(f"{a}{b}{c}{i:04d}")
    cumulative, acc = [], 0.0
    for k in range(1, 1025):
        acc += 1.0 / k
        cumulative.append(acc)
    import bisect

    rng = Rng(seed)
    out = []
    for _ in range(count):
        n = 55 + rng.next_in(20)
        words = []
        for _ in range(n):
            r = rng.next_f64() * cumulative[-1]
            idx = min(bisect.bisect_left(cumulative, r), len(cumulative) - 1)
            words.append(vocab[idx])
        out.append(" ".join(words))
    return out


def compile_query(raw: str) -> str:
    seen, terms = set(), []
    token = []
    for ch in raw.lower() + " ":
        if ch.isalnum():
            token.append(ch)
            continue
        t = "".join(token)
        token = []
        if t and t not in STOPWORDS and t not in seen:
            seen.add(t)
            terms.append(t)
    if not terms:
        terms = [t.replace('"', '""') for t in raw.split()]
    return " OR ".join(f'"{t}"' for t in terms)


def read_jsonl(path: str, limit: int | None = None) -> list[str]:
    out = []
    with open(path, encoding="utf-8") as f:
        for line in f:
            out.append(json.loads(line))
            if limit is not None and len(out) >= limit:
                break
    return out


def pct(sorted_values: list[int], p: float) -> int:
    return sorted_values[min(len(sorted_values) - 1, int(len(sorted_values) * p))]


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--synthetic", action="store_true")
    ap.add_argument("--corpus")
    ap.add_argument("--queries")
    ap.add_argument("--sizes", default="10000,30000,100000")
    ap.add_argument("--queries-n", type=int, default=1000)
    args = ap.parse_args()
    sizes = [int(s) for s in args.sizes.split(",")]
    top = max(sizes)
    if args.synthetic:
        docs = synthetic(CORPUS_SEED, top)
        queries = synthetic(QUERY_SEED, args.queries_n)
        name = "synthetic"
    else:
        docs = read_jsonl(args.corpus, top)
        queries = read_jsonl(args.queries)[: args.queries_n]
        name = args.corpus
    db = sqlite3.connect(":memory:")
    db.execute("CREATE VIRTUAL TABLE t USING fts5(body, tokenize='porter unicode61 remove_diacritics 2')")
    compiled = [compile_query(q) for q in queries]
    loaded = 0
    for size in sizes:
        if size > len(docs):
            break
        db.executemany("INSERT INTO t(rowid, body) VALUES(?, ?)", ((i + 1, docs[i]) for i in range(loaded, size)))
        loaded = size
        counts = sorted(db.execute("SELECT count(*) FROM t WHERE t MATCH ?", (q,)).fetchone()[0] for q in compiled)
        terms = [q.count(" OR ") + 1 for q in compiled]
        print(
            json.dumps(
                {
                    "corpus": name,
                    "n": size,
                    "queries": len(compiled),
                    "terms_mean": round(statistics.mean(terms), 1),
                    "matches_min": counts[0],
                    "matches_p50": pct(counts, 0.5),
                    "matches_p90": pct(counts, 0.9),
                    "matches_max": counts[-1],
                    "matches_mean": round(statistics.mean(counts), 1),
                    "frac_p50": round(pct(counts, 0.5) / size, 4),
                    "frac_mean": round(statistics.mean(counts) / size, 4),
                }
            ),
            flush=True,
        )


if __name__ == "__main__":
    main()
