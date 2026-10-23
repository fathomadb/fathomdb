# agent-access-snippet-cost — EP-17 snippet point queries and latency

This is the cost receipt for bounding the EP-17 rule-3 per-term snippet pass
(`dev/design/agent-access/match-centred-snippets.md` § 6.5, § 10, decision
of 2026-10-21). It counts the `highlight()` point queries one search issues and
times the search, with snippets off and with snippets on.

## Files

- The sweep and its correctness tests are in
  `src/rust/crates/fathomdb-engine/tests/snippet_cost_receipt.rs`. The sweep
  is the ignored test `snippet_cost_receipt`.
- `summary-before.json` was taken before the bound, at `a9d6d7aed`, the
  commit that adds the tooling. `summary-after.json` was taken with the bound
  in place, at `1968e294b`.

## Result

With snippets on and k of 2 or more, a search issued 20 + 20k point queries
before the bound: 80 at k = 3 and 120 at k = 5. After it, a search issues at
most 2 × 20 = 40: 38 at k = 3, since six 3-term passes fit the pool of 20, and
40 at k = 5. One-term searches, and searches with snippets off, are unchanged.
The p50 latency with snippets on fell by between 36% and 46% at k = 3 and
k = 5 in every record size. The per-cell numbers are in the two summaries.

## Sweep

- Each (record size, k) pair gets its own store of 40 synthetic records of
  2, 10 or 50 KiB; its two cells, snippets off and on, share it.
  Each record holds the k query terms (k = 1, 3, 5), spread evenly through
  the body, so each term falls in its own window.
- The query is the k terms, run as a text-only search with limit 20.
  Snippets are either off, or on with `SnippetOptions::default()`:
  `max_bytes` 300, one window, and `max_point_queries` 20.
- Each cell runs 10 warm-up searches, then 200 measured ones. It reports the
  min and max point queries per search, and the nearest-rank p50 and p95
  latency in microseconds.
- The point-query count comes from the `test-hooks` statement trace, as the
  snippet tests count it. That trace stays on in both runs, so its own
  overhead is the same before and after.

## Command

Run it from the repository root. Change the label and file name for each run.

```sh
FATHOMDB_SNIPPET_COST_LABEL=before \
FATHOMDB_SNIPPET_COST_OUT=$PWD/dev/experiments/agent-access-snippet-cost/summary-before.json \
cargo test --release -p fathomdb-engine --features test-hooks \
  --test snippet_cost_receipt snippet_cost_receipt -- --ignored --exact
```

The runs were taken on windchill3, an AMD Ryzen Threadripper PRO 5945WX, on
the CPU only. The latencies are single-machine and single-caller numbers. They
show how cost scales with k and record size; they are not a perf gate.
