# AC-013 scale search: pre-registration

Written and committed before any timed run. Todo TC-b23a5ead (seq 352).
Branch `exp/0.8.29-ac013-scale-search`, cut from `exp/0.8.29-ac013-scale`
`1fd4bd2bc`. Tools: [README.md](README.md).

## Question

At n = 100k a hybrid search costs about 600 ms p50 under every vector fix.
Where does that time go, how does each part scale, is the AC-013 fixture a
fair workload for it, and what would remove each part without changing
results?

## Workloads

- **SYN**: the AC-013 fixture (`perf_gates.rs` generator, seeds,
  `VaryingEmbedder` 768-d). Queries are 55-74-token synthetic bodies.
- **LME**: LongMemEval-S turns (189,514 unique turns, first-seen order,
  prefix of n) as documents and its 500 questions as queries, written by
  `lme_corpus.py` into the scratchpad (never committed). Same embedder and
  dimension as SYN, so only the text side changes.
- Match counts per query come from the probe (`matches`) and are
  cross-checked by `match_counts.py`.

## Cells

`cells.json`. Layouts: BASE (vec0 default 1,024-row chunks) and CS64
(`FATHOMDB_PERF_VEC0_CHUNK_SIZE=64`). Prototypes, all under CS64 and all
env-gated behind `FATHOMDB_PERF_EXPERIMENTS`:

- `CS64+ELIG` (`FATHOMDB_PERF_ELIGIBILITY=cache`): the answers of the two
  O(N) direct-eligibility scans are cached per process, keyed by the
  snapshot's `_fathomdb_read_visibility_state` (generation, nonce) and the
  view flags, and only when the store has no validity windows or edge
  `t_invalid` at that generation.
- `CS64+FTS` (`FATHOMDB_PERF_FTS_ROWID_CURSOR=1` +
  `FATHOMDB_PERF_TEXT_ARM=deferred`): FTS rows are inserted with
  `rowid = write_cursor`; the deferred hybrid text arm scores every match
  from `rowid, bm25()` only and reads content only for the rows that can
  reach the fused top-k.
- `CS64+ALL`: both. `BASE+ELIG`: for the vector-stage cells only.

## Stages

| Stage | Cells | Corpus | n | Samples | Passes | Traced | Repeats |
| --- | --- | --- | --- | --- | --- | --- | --- |
| A attribution | BASE, CS64 | SYN, LME | 10k, 30k, 100k | SYN 1,000 / 1,000 / 300; LME 500 | hybrid, text, vector | yes | 1 |
| B latency | BASE, CS64, CS64+ELIG, CS64+FTS, CS64+ALL | SYN, LME | 10k, 30k, 100k | as A | hybrid, text | no | 2 at 10k and 100k, 1 at 30k |
| B' attribution | CS64+ELIG, CS64+FTS, CS64+ALL | SYN | 10k, 100k | as A | hybrid, text | yes | 1 |
| C vector stage | BASE+ELIG, CS64+ELIG | SYN | 100k, 1M (if seed <= 30 min and store <= 10 GB) | 200 | vector | yes and no | 1 each |
| D realism | CS64 | LME | 10k | 500 | hybrid, vector | no | 1, with `AC013_SUPERSEDE_N=10` |
| E gate | CS64, best prototype cell | SYN | 10k | gate | gate | no | 2 |

The `vector` pass sends queries whose tokens match no document, so its cost
is the eligibility checks, the vector stage and hydration. Stage C reads the
vector stage as the traced inclusive time of the phase-1 statement and as
the untraced `vector` pass p50 with the eligibility scans cached away.

Odd repeats run the cells in reverse order. Every run holds the shared
measurement lock and starts at 1-minute load <= 2; load before and after is
recorded and a run whose load after exceeds 2 is flagged and, if it decides
a verdict, repeated.

## Decision rules

1. **Identity.** A prototype's hybrid `digest` and `ids_digest`, and its
   text `digest`, must equal CS64's for the same corpus, n and samples. A
   difference is either explained exactly (which queries, which hits, why)
   or the prototype is rejected.
2. **Effect.** A prototype is material at an n when its hybrid p50 (median of
   repeats) is at least 10% below CS64's and every repeat points the same
   way. The 100k SYN and LME cells decide; 10k and 30k show the trend.
3. **Scaling.** For each component (FTS statement and its nested lookups,
   each eligibility scan, vector stage, the rest), b = log(cost@100k /
   cost@10k) / log(10) on traced exclusive page requests and ms per search.
   b >= 0.8 is O(N); b <= 0.2 is O(1)/O(log N). A component is O(matches)
   when its cost per mean match count stays within +-25% across n and
   across SYN and LME.
4. **Fixture realism.** The text arm's cost is a fixture artifact if LME's
   median match fraction is below 5% of n; it is real if LME's is within a
   factor of 3 of SYN's.
5. **CS64 at large n.** CS64 regresses the vector stage at large n if its
   phase-1 time exceeds BASE's by >= 10% at the largest n measured.
6. **Gate.** Stage E reports the AC-013 gate p50 for CS64 and the best cell;
   it does not change the verdicts above.
