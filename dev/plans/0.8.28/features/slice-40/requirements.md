---
title: FathomDB 0.8.28 Slice 40 — requirements and acceptance criteria
status: REVISED AFTER R1 (APPROVE-WITH-FIXES, F1–F6); Defect C and Unit D landed; Units E, F, G added 2026-10-16 (owner ruling), docs and RED tests for review
target_release: 0.8.28
observed_on: 2026-10-16
---

# Slice 40 requirements and acceptance criteria

Slice 40 fixes three defects in FathomDB's text retrieval. Defects A and B
are ranking bugs found by the COMP-01 benchmark. Defect C, added by owner
ruling on 2026-10-16, is a latency bug in `search_projected_text` found by
COMP-02b; its fix must not change which rows are returned or their order.
Unit D, added by owner ruling on 2026-10-16, is the COMP-03 retrieval-lever
work: a retrieval-only sweep (D1), a three-arm answer run (D2) and a
stop-list extension (D3). Units E (SDK parity and the public-surface
contract), F (a known-run baseline) and G (evidence budget, session
completion and recency in the COMP-03 answer harness) were added by owner
ruling on the same day. [design.md](design.md) gives the mechanism,
[test-plan.md](test-plan.md) the tests.

## 1. Evidence

COMP-01 ran BEIR NFCorpus (3,633 documents, 323 test queries, official
graded qrels) through FathomDB 0.8.27 `Engine.search_text_only(limit=10)`
and LanceDB 0.40 native FTS. Result note (read-only, in the `comp-01`
worktree):
`dev/performance-benchmarking/2026-10-15-comp-01-nfcorpus-fts-result.md`.

| Arm | nDCG@10 |
| --- | ---: |
| FathomDB 0.8.27 `search_text_only` | 0.3087 |
| LanceDB 0.40 FTS | 0.3265 |
| Raw SQLite FTS5, FathomDB tokenizer and query translation, no engine post-processing | 0.3228 |
| Raw SQLite FTS5 as above, 1–2 character tokens kept | 0.3260 |

The orchestrator's follow-up diagnosis (scripts `diag2.py`, `fts5exp.py`,
inputs under `~/.cache/fathomdb-comp/`) split the 0.018 gap into two causes.
The result note's own interpretation ("body deduplication is not the
cause") predates that diagnosis and is superseded by it.

- **Defect A, within-arm double counting in RRF.** About 0.014 of the gap.
  NFCorpus has 40 pairs of byte-identical bodies. In run 1, 53 of 323
  queries had a duplicate-body document at rank 1; it was relevant in 4.
  Duplicate documents appeared 164 times in FathomDB top-10s against 65 for
  LanceDB.
- **Defect B, 1–2 character tokens dropped by the query compiler.** About
  0.003 (0.3228 to 0.3260 on raw FTS5; p95 latency 2.24 to 2.30 ms). It deletes "C" from "Vitamin
  C-Enriched Bacon", "D", "D2" and "D3" from vitamin D queries, "II" from
  "Physicians' Study II" and "1" from "IGF-1".

### 1.1 Defect A, read from the code

`fuse_three_arms` (`src/rust/crates/fathomdb-engine/src/fusion.rs`) adds
`weight / (RRF_K + rank)` to one accumulator per distinct `SearchHit.body`,
for every hit of every arm. Two rows with the same body that land in the
same arm both add to that body's score. With `RRF_K = 30` and
`RRF_WEIGHT_TEXT = 3`:

- the true rank-1 hit scores `3/31 = 0.0968`;
- a pair at text ranks 13 and 14 scores `3/43 + 3/44 = 0.1380` and jumps to
  rank 1;
- even a pair at ranks 27 and 28 (`0.1048`) beats rank 1.

The documented contract does not ask for this. `fuse_three_arms`'s doc
comment and `dev/design/slice-10-design.md` give the score as
`Σ_branch weight / (RRF_K + rank_branch(b))`, one term per branch;
`docs/guides/hybrid-search-filtering.md` says "the sum … over the branches
that surfaced it". The explain sidecar's per-arm `body_rank_map`
(`search.rs`) already records one rank per body per arm, the first
occurrence. The code is the outlier.

The same arm can also hold a body twice when an edge body equals a node
body: the edge FTS hits are appended to the text arm after the node hits
(`text_results.extend(edge_candidates)`).

### 1.2 Defect B, read from the code

`content_tokens` (`src/rust/crates/fathomdb-query/src/lib.rs`) skips any
token with `token.len() < 3` (a byte length) before the stopword check.
The same filter also removes the empty fragments that splitting on
non-alphanumeric characters produces, so the fix must keep an
`is_empty()` check. The stopword list already holds the one-character
words that should go ("a", "i").

### 1.3 Defect C, read from the code

Evidence: [comp02b-projected-fts-latency.md](comp02b-projected-fts-latency.md).
On the 10k TC-5 corpus, `search_projected_text` runs at 82 ms p50 against
13.9 ms for `search_text_only` on the same database. The median query
matches 3,806 of 10,000 property rows. Raw sqlite3 timings of the engine's
statement: 52.0 ms as shipped, 5.2 ms with the same predicate and the FTS5
rank stream (`rank MATCH 'bm25()' ORDER BY rank LIMIT 10`). The attribute
filter is not the cost (it makes the query faster by shrinking the sort);
the two dependency-closure `NOT EXISTS` subqueries cost about 6 ms and are
not the cause.

`read_projected_text_in_tx` (`search.rs`) prepares `property_fts_rank_sql`
(`filter.rs`): the FTS5 match joined to `canonical_nodes`, the full
`FrozenView::node_sql` predicate and the attribute-filter predicate,
selecting `n.body`, `ORDER BY bm25(property_search_index) ASC,
p.write_cursor ASC`, with no `LIMIT`. The Rust loop stops after `limit`
rows, but SQLite has already sorted every matched row, body included, in a
temporary b-tree. The body path had the same shape until SCALE-02 (HITL
`seq-268`); `read_search_in_tx` now runs a rank-stream statement
(`AND rank MATCH 'bm25()' ORDER BY rank`), collects it with
`collect_complete_rank_boundary`, and keeps the full sort only as a
fallback (`.ok()` on prepare or execute). The projected path never got
that change.

The public contract is the doc comment on `Engine::search_projected_text`
(`search_api.rs`): results "are ordered by property-FTS bm25 ascending then
write cursor ascending". Scores are `-bm25`.

### 1.4 Unit D evidence (COMP-03)

Source: the COMP-03 result note
(`dev/performance-benchmarking/2026-10-16-comp-03-hermes-answer-result.md`
on the `llm/comp-01` lineage) and the orchestrator's review of it.

- **Answer accuracy:** FathomDB 0.8.27 and LanceDB both 42/60 on the 60
  seeded LongMemEval-S questions (top-5 rounds, gpt-5.4 answerer and judge).
- **FathomDB's 18 misses:**
  - 6 retrieval misses, 5 of them preference questions (a vocabulary gap
    between the question and the round that holds the preference);
  - 10 with the gold round in the prompt but a wrong answer: 4 multi-session
    counts, 3 knowledge-update, 2 preference, 1 temporal;
  - 2 missed abstentions.
- **Retrieval-only rerun on this slice's engine** (Defects A and B fixed):
  top-5 changed on 19 of 60 questions; session Recall@5 0.893 → 0.911;
  session MRR 0.848 → 0.841.
- **Short tokens after Defect B:** 41 of 60 questions carry a 1–2 character
  content token, nearly all "my", "me", "so", "up", "m", "ve". They are
  function words and contraction fragments, not the content tokens ("C",
  "D3", "II", "1") that Defect B set out to keep.
- **Prior finding behind the pre-stated rule:** EARP found hybrid retrieval
  null-or-negative on real traces (memory
  `earp-tiered-run-program-20260809-complete.md`). Hybrid is therefore
  adopted for the answer run only by a rule stated before the sweep runs
  (REQ40-D3), not by inspection of its results.
- **Dataset:** LongMemEval-S cleaned, 500 questions (30 abstention), SHA-256
  `d6f21ea9d60a0d56f34a05b609c79c88a451d2ae03597821ea3d5a9678c3a442`,
  pinned in `hermes-answer.v1.json`. Per type (non-abstention):
  temporal-reasoning 127, multi-session 121, knowledge-update 72,
  single-session-user 64, single-session-assistant 56,
  single-session-preference 30.

### 1.5 Units E, F and G evidence

- **Unit E, SDK parity.** Slice 40 changed behaviour under three public
  verbs without changing a signature:
  - `search_text_only`: per-arm dedupe (A), short tokens (B), the stop list
    (D3);
  - `search_projected_text`: the rank stream (C);
  - `search`: dense retrieval through the `vector=True` projection switch
    (Unit D).

  The existing cross-SDK suite
  (`dev/performance-benchmarking/2026-10-13-cross-sdk-fts-comparison-result.md`
  on the comp lineage) checks only ordered IDs from `search_text_only` on 100
  synthetic single-term documents, against fixture oracles. Facts read from
  the code on 2026-10-16:
  - The TypeScript SDK has `useDefaultEmbedder`, `ProjectionSpec.vector`,
    `searchProjectedText(query, name, filter?, {limit, …view})` and `search`.
    The Rust SDK has `use_default_embedder`, `configure_projections`,
    `search`, `search_text_only` and `search_projected_text`. No product gap
    blocks the three parity cases.
  - No SDK exposes the engine's build commit. Every build reports version
    `0.8.27`.
  - The public docs give the RRF score as `Σ 1/(60 + rank)` in
    `docs/guides/structured-search-hits.md`,
    `docs/guides/hybrid-search-filtering.md`,
    `docs/reference/python-api.md` and `docs/reference/typescript-api.md`.
    The code uses `RRF_K = 30` with weight 3 for text and 1 for vector and
    graph.
  - No checker pins the interface docs' verb descriptions.
    `scripts/check-sdk-surface-parity.py` checks operation names and
    signatures only.
- **Unit F, baseline.** The Unit D numbers came from runs at three engine
  commits:
  - NFCorpus D3 at `5b09afd44`;
  - `comp01-hybrid` and the pre-D3 sweep at `a3830681d`;
  - the D3 sweep and answer v2 at `5b09afd44`.

  The pre-D3 sweep and both NFCorpus runs have no receipts. The 10k TC-5
  latency figures came from an unversioned scratchpad script. A later run
  cannot be checked against them.
- **Unit G, the COMP-03 misses** (orchestrator's miss analysis of answer
  v2). The 11 FathomDB FTS misses break down as:
  - 4 where the gold was cut by the 20,000-character cap or the session
    was only partly shown;
  - 4 with the gold present but a wrong answer, including one stale
    knowledge-update value;
  - 2 retrieval misses;
  - 1 abstention.

  The cap was reached on 46 of 60 questions, and the median question showed
  9 of its 10 rounds. Only 6 of 10 multi-session questions had all their
  gold sessions in the prompt.

## 2. Requirements

**REQ40-01 — once per arm.** In `fuse_three_arms`, each arm (vector, text,
graph) contributes at most one RRF term to a given body. The term uses the
positional rank (1-based index in that arm's list) of the body's first
occurrence in that arm. Later occurrences in the same arm contribute
nothing and do not change other hits' ranks.
*Rationale:* removes the double counting of § 1.1 and keeps fusion's
per-arm rank equal to the explain sidecar's.

**REQ40-02 — cross-arm agreement kept.** A body surfaced by more than one
arm still accumulates one term from each of those arms.
*Rationale:* agreement boosting is the purpose of hybrid fusion (ADR
`ADR-0.8.0-agent-memory-retrieval-and-identity.md`, Q3).

**REQ40-03 — everything else unchanged.** For inputs in which no arm holds
a body twice, the output of `fuse_three_arms` (order, score bits,
representative hit) is unchanged. The representative of a body is still its
first occurrence (vector arm first, then text, then graph); the sort is
still score descending, then vector-first, then first-insertion order;
`fuse_rrf(v, t) == fuse_three_arms(v, t, [])` still holds.

**REQ40-04 — short tokens kept.** `compile_text_query` keeps alphanumeric
tokens of every length, including 1 and 2 characters, as content tokens.
Empty split fragments are still discarded.

**REQ40-05 — stop list and query shape kept.** The `STOPWORDS` list is
unchanged and still applied. Lowercasing, splitting on non-alphanumeric
characters, first-seen de-duplication and the `"tok" OR "tok"` shape are
unchanged. Removing the stop list cost 0.004 nDCG@10 and doubled tail
latency in the diagnosis, so it stays.

**REQ40-06 — injection safety kept (AC-038).** Every emitted term is one
FTS5 string literal; content-path terms are non-empty and alphanumeric.

**REQ40-07 — fallback kept.** A query with no content token (only
stopwords, only symbols) still takes the raw whitespace-token OR fallback,
byte-for-byte as today. A query whose only tokens are short non-stopwords
now has content tokens and takes the content path.

**REQ40-08 — documentation matches.** Every doc that states the old
behavior is corrected in the change that fixes it (list in
[design.md](design.md) § 5).

**Defect C requirements (R-C1 to R-C6).** The fix changes how projected
search reaches its rows, never which rows it returns or their order.

**REQ40-C1 — streamed rank path.** `read_projected_text_in_tx` first runs
the projected statement with `AND rank MATCH 'bm25()' ORDER BY rank` and
collects it with the boundary-tie completion
(`collect_complete_rank_boundary(limit)`), so SQLite never sorts the full
match set.

**REQ40-C2 — identical ordered results.** For every input, the ordered
`(write_cursor, score, id, kind, body, source_id)` list equals what the
full-sort statement returns today, including rows tied on bm25 at the
limit boundary (ties order by `write_cursor` ascending).

**REQ40-C3 — limit honoured exactly.** The collector is bounded by the
validated `limit` argument (1..=100), not a fixed window. When at least
`limit` rows match, exactly `limit` come back.

**REQ40-C4 — filter semantics unchanged.** The streamed statement keeps the
full `FrozenView::node_sql` predicate and the attribute-filter predicate
from `append_node_eligibility_sql`, byte-identical to the full-sort
statement's.

**REQ40-C5 — fallback.** If the streamed statement fails to prepare or
execute, the existing full-sort statement runs, unchanged, in the same read
transaction, exactly as the body path does.

**REQ40-C6 — nothing else changes.** `search_text_only`, hybrid `search`,
`property_fts_rank_sql` and its `slice35_ranked_eligibility_sql_for_test`
seam, the public API and the schema are unchanged.

**Unit D requirements.** D1 and D2 change the COMP harness only; D3
changes the product.

**REQ40-D1 — retrieval sweep (D1).** A new harness module runs every
LongMemEval-S question through the arms `fathomdb-fts`
(`search_text_only`, v1 store shape), `fathomdb-hybrid` (default embedder,
`Engine.search`, no rerank, no graph arm, over the same plain bodies plus a
searchable `vector=True` projection declaration) and `lancedb-fts`
(reference). `fathomdb-fts` vs `fathomdb-hybrid` is a pure vector-arm
ablation over identical text (R1 ruling Q-D2). Per question and arm it records the top-10 IDs, query latency,
and build time, with embedding time recorded apart. It reports, overall and
per question type: session Recall@5 and @10 (any), session Recall-all@5
and @10, session MRR@10, turn Recall@5 and @10, and a paired bootstrap 95%
CI for every `fathomdb-hybrid − fathomdb-fts` delta (seed in the config).

**REQ40-D2 — sweep discipline.** A fresh store per question per arm; the
gold-leak discipline of `comp03_answer` (its `convert_question` and
`gold_for`, reused, not copied); resumable per question (a finished
question does no work on rerun); the dataset digest checked against the
config; the embedding device read from `FATHOMDB_EMBED_DEVICE`, recorded,
and a CUDA request that resolved to CPU refused; every hybrid store's
vector coverage checked on the closed database (distinct embedded doc
cursors == documents written) and a mismatch refused, not reported.

**REQ40-D3 — pre-stated decision rule.** The rule lives in the sweep config
and is evaluated by a pure function whose verdict is written into the sweep
summary. Verbatim: *adopt hybrid for the answer run iff (a) preference
session Recall@5 delta CI lower bound > 0, (b) no other type's delta CI
upper bound < 0, (c) overall delta ≥ 0.* The delta is
`fathomdb-hybrid − fathomdb-fts` on session Recall@5 (any) for all three
clauses.

**REQ40-D4 — three-arm answer run (D2).** `comp03_answer` gains a v2 config
(`hermes-answer.v2.json`): the same 60 question IDs, models, prompts, caps
and judge as v1, `top_k` 10 for every arm, arms `fathomdb-fts`,
`fathomdb-hybrid`, `lancedb-fts`. The hybrid arm runs only when the D1
sweep verdict is `adopt`. Arm order rotates by question so that, for two
arms, it is exactly v1's alternation. The v1 config, its receipt and its
code path are unchanged. A `fathomdb-hybrid` arm is also registered in
`comp_fts` (campaign `comp01-hybrid`) so the NFCorpus informational row can
run; the existing `fathomdb` arm is unchanged in every byte.

**REQ40-D5 — stop-list extension (D3).** `STOPWORDS` in `fathomdb-query`
gains exactly twelve words, in two groups:

- possessive and contraction fragments left by splitting on the apostrophe:
  `s` ('s), `t` (n't), `ll` ('ll), `re` ('re), `ve` ('ve), `m` ('m);
- short function words: `my`, `me`, `us`, `am`, `so`, `up`.

`d`, `c`, `d3`, `ii`, `1` and every other short token stay content.
Nothing else in the compiler changes.

**REQ40-D6 — D3 measurement gate (orchestrator, after W4).** Verbatim:
*NFCorpus nDCG@10 paired delta vs 0.3260 with CI not below zero, COMP-03
per-type recall unchanged within CI, 10k TC-5 p95 not higher.* If the gate
fails, D3 is reverted; it is therefore one commit of its own.

**Unit E requirements.**

**REQ40-E1 — parity cases.** The cross-SDK suite gains three cases, each
run through the Python, TypeScript and Rust SDKs:

- **(1) `search_text_only`.** The corpus has duplicate bodies. Queries carry
  1–2 character content tokens, possessive and contraction fragments, and
  every D3 word.
- **(2) `search_projected_text`.** Run with and without an attribute
  filter, on a corpus whose bm25 tie group straddles the limit.
- **(3) `search` with the `dense` projection declared.** Default embedder,
  CPU, fixed corpus.

**REQ40-E2 — exact parity.** For every case and repetition, each SDK's
ordered hits equal the oracle's in `(id, score, branch)`, scores compared
bit-for-bit. The oracle is the Python SDK run in the coordinator on the
same database file the SDK just queried. The engine computes every score,
vector scores included, so any difference is a binding defect, not
numerical noise.

**REQ40-E3 — the engine under test is this slice's.** All three SDK
artifacts (Python wheel, TypeScript N-API addon, Rust driver) and the
bootstrap CLI are built from one recorded engine commit and pinned by
SHA-256. No SDK exposes a build commit, so the arm manifest records the
source commit, its clean state at build time and the artifact digests.

**REQ40-E4 — public-surface contract.** `dev/interfaces/python.md`,
`typescript.md`, `rust.md` and `rust-sdk.md` state, for the three verbs:

- per-arm dedupe;
- short tokens kept and the extended stop list;
- the projected rank stream, with the order contract unchanged;
- the dense-projection switch, and that the full node body is embedded.

The public docs' RRF formula is corrected to `RRF_K = 30` with 3:1
text-to-vector weighting. `./scripts/agent-lint-docs.sh` passes.

**Unit F requirements.**

**REQ40-F1 — baseline manifest.** `experiments/configs/baselines/slice-40-baseline.v1.json`
lists, per benchmark:

- inputs identifier and SHA-256;
- engine commit;
- wheel build recipe (features, `CUDA_PATH`/`PATH`/`LIBRARY_PATH`,
  `--compatibility linux`);
- exact command line;
- receipt ID or "to be recorded";
- expected metrics with tolerances: nDCG@10 ±0.002; MRR ±0.005;
  per-type session recall ±1 question; latency p50 ±25% and p95 ±35% on a
  quiet host; answer accuracy ±3 questions, recorded as noise, not a gate.

The benchmarks are COMP-01 NFCorpus (fts vs LanceDB), `comp01-hybrid`, the
COMP-03 retrieval sweep (three arms), COMP-03 answer v2, and 10k TC-5
projected and body latency.

**REQ40-F2 — latency harness.** `experiments/comp02b_latency.py` turns the
orchestrator's diagnostic script into a receipt-producing command with the
same cells (projected unfiltered, cohorts c01, c10 and c33, body
`search_text_only`, projected single term) and the same percentiles. It has
no SurrealDB arm.

**REQ40-F3 — drift check.** `experiments/baseline_check.py` takes the
manifest, a benchmark ID and an attempt's summary or report. It prints PASS,
DRIFT or MISSING per metric, with measured value, expected value and
tolerance, and exits non-zero on any DRIFT or MISSING.

**REQ40-F4 — engine identity in receipts.** Every COMP receipt from now on
records the engine commit (declared at run time, validated as a 40-hex
SHA), the installed native module's SHA-256 and the version string.

**REQ40-F5 — measured at the final head.** The baseline is re-measured by
the orchestrator, with receipts, at the final slice head after Units E to
G land. The Unit D runs are not the baseline.

**Unit G requirements.** All of G is harness-side this round.

**REQ40-G1 — offline evidence evaluator.** `comp03_retrieval` evaluates
evidence policies on the stored sweep records, with no retrieval and no
model calls. A policy is `per_round_chars` ∈ {4000, 2000, 1000},
`total_chars` ∈ {20000, 30000} and `session_expansion` ∈ {off, on}: twelve
policies.

- Rounds enter the prompt in rank order: per-round cap first, then the
  total cap, as `render_evidence` does.
- Session expansion appends, after the top-k rounds, the remaining rounds
  of each hit session. Sessions go in the order of their best hit's rank,
  and rounds within a session in position order, until the total cap.
- Metrics per policy, per type and overall: gold-session-any-in-prompt,
  gold-session-all-in-prompt, has-answer-round-in-prompt, rounds in prompt,
  and characters. A round is in the prompt if at least one of its
  characters is.

**REQ40-G2 — pre-stated policy rule** (in the config): *pick the policy
with the highest overall gold-session-all-in-prompt such that no type's
gold-session-any-in-prompt is lower than the current policy's (4000 /
20000 / off) by more than 1 question; ties broken by fewer chars.* The
verdict is written to a file that names its inputs.

**REQ40-G3 — recency rendering.** The answer harness can render evidence in
chronological order by session date, newest last, with dates kept, and add
one sentence to the answerer prompt: *When the history states different
values for the same fact at different dates, use the most recent one.*

- Both are off by default.
- With both off, the rendering and the prompt are byte-identical to v2's.
- Chronological order reorders the rounds the rank-order cap selected; it
  does not change which rounds are selected.

**REQ40-G4 — answer configs and rules.**

- `hermes-answer.v3.json` is v2 (three arms, k = 10, the same 60 IDs and
  models) with the evidence policy from the G1/G2 verdict file, bound by
  SHA-256.
- `hermes-answer.v4.json` is v3 with G3 on.
- v3 vs v2 is reported paired per arm. Gains are expected on multi-session
  and preference; this is a report, not a gate.
- v4 vs v3 adopts G3 iff knowledge-update plus temporal correct does not
  drop and overall does not drop by more than 1 question, with the CI
  reported.

Receipts and classifications are made as for v2.

**REQ40-G5 — product-side equivalents, named only.** The design states the
engine-side equivalents (turn-level retrieval units with session expansion;
a recency-aware ranking pass), so the owner can decide on a product unit.
None is built in this slice.

## 3. Non-goals

- No BM25 `k1`/`b` tuning and no change to `RRF_K` or the arm weights.
- No change to the stopword list for Defects A to C; Unit D3 extends it
  (REQ40-D5).
- No new public API, no new knob, no schema change, no migration.
- No near-duplicate detection; only byte-identical bodies are affected, as
  they already are by fusion's dedupe on body.
- No change to the direct text-only candidate window (§ 4).
- No fielded title boost through declared `searchable` projections.
  Potential, deferred by owner ruling 2026-10-16 (`status.md`).
- Unit D does not touch the attribute-filter row-key change or the
  filter-first plan; both stay listed as potentials in `status.md`.
- No rerank and no graph arm in the hybrid arm.
- No performance assertion in tests for Defect C; the orchestrator
  measures latency on the 10k corpus after W4.
- No change to the frozen IR-C experiment harnesses
  (`tests/support/ir_retrieval.rs` and its includers), which document
  themselves as a verbatim historical copy.
- The stale `Σ 1/(60 + rank)` formula in the public docs was a non-goal for
  Defects A to D; Unit E now fixes it (REQ40-E4).
- Units E to G do not touch the five unclassified pre-existing COMP
  receipts, landing, the attribute-filter row-key and filter-first
  potentials, or fielded title boost.
- No product change for Unit G this round (REQ40-G5).

## 4. Candidate bound of `search_text_only`: out of scope

The direct text-only path fetches a fixed node window of
`MAX_SEARCH_RESULT_LIMIT` (100) FTS rows for every accepted `limit`
(`SearchReaderWork::text_only`, `direct_text_candidate_limit`), then fuses
and truncates to `limit`. `candidate_limit = max(search_limit_override,
limit)` in `search_api.rs` feeds the vector stage, not this window. Fusion
already collapsed duplicate bodies before this slice, so a caller asking
for close to 100 results can get fewer than `limit` when the window holds
duplicate rows; for the default `limit = 10` that needs more than 90
duplicate rows in the window. The fix is not a small over-fetch: the window
would have to count distinct bodies inside `collect_complete_rank_boundary`
while keeping the Slice 20 boundary-tie completion and the Slice 23 prefix
stability. Recorded as a follow-up; AC40-13 guards the default limit.

## 5. Acceptance criteria

Each AC is checked by the named test. Engine tests are in
`src/rust/crates/fathomdb-engine/tests/slice40_fts_ranking.rs` unless
another file is named; the Defect C tests (AC40-27 to AC40-31) are in
`tests/slice40_projected_fts_stream.rs`, which needs `--features
test-hooks`; query tests are in the `tests` module of
`src/rust/crates/fathomdb-query/src/lib.rs`. Defect C's criteria start at
AC40-27 because AC40-23 to AC40-26 were already assigned. Units E to G's
harness tests (AC40-49 to AC40-66) are in `tests/experiments/test_sdk_parity_slice40.py`,
`test_baseline_check.py` and `test_comp03_evidence.py` on the comp branch.
Unit D's
harness tests (AC40-33 to AC40-45) are in `tests/experiments/test_comp03_retrieval.py`
and `tests/experiments/test_comp03_answer.py` on the `slice/0.8.28-40-comp03`
branch; its D3 tests are in the `fathomdb-query` test module.

| AC | Check | Test | REQ |
| --- | --- | --- | --- |
| AC40-01 | A duplicate pair at text ranks 13/14 does not outrank the distinct rank-1 body and sits 13th | `single_arm_duplicate_body_does_not_outrank_higher_ranked_distinct_body` | 01 |
| AC40-02 | A duplicated body scores exactly `w_text/(K + first rank)`; later bodies keep positional ranks | `single_arm_duplicate_body_scores_once_at_first_occurrence_rank` | 01 |
| AC40-03 | The representative of a duplicated body is its first occurrence | `duplicate_body_representative_is_first_occurrence` | 03 |
| AC40-04 | The rule holds on the vector and graph arms | `vector_arm_duplicate_body_scores_once`, `graph_arm_duplicate_body_scores_once` | 01 |
| AC40-05 | The dedupe is per arm: one vector term plus one text term | `duplicate_dedupe_is_per_arm_not_global` | 01, 02 |
| AC40-06 | Cross-arm agreement accumulates, with and without in-arm duplicates | `cross_arm_agreement_still_accumulates_with_in_arm_duplicates`, `three_arm_agreement_sums_one_term_per_arm`; existing `pr_g9_rrf_fusion.rs` tests | 02 |
| AC40-07 | Property: text arm only, fused order equals first-occurrence order, scores strictly decreasing | `prop_text_only_fusion_is_monotone_in_first_occurrence_rank` | 01 |
| AC40-08 | Property: any three arms equal the once-per-arm reference (order, score bits, identity) | `prop_fusion_matches_once_per_arm_reference` | 01–03 |
| AC40-09 | Property: inputs with no in-arm duplicate are unchanged | `prop_distinct_within_arm_inputs_are_unchanged` | 03 |
| AC40-10 | Property: `fuse_rrf(v, t) == fuse_three_arms(v, t, [])` | `prop_two_arm_fusion_equals_three_arm_with_empty_graph` | 03 |
| AC40-11 | End to end: duplicate pair does not win rank 1; top 10 are the first ten distinct rows | `text_only_duplicate_bodies_do_not_win_rank_one` | 01 |
| AC40-12 | End to end: the pair surfaces once, at its first FTS rank | `text_only_duplicate_body_surfaces_once_at_first_occurrence` | 01, 03 |
| AC40-13 | End to end: the default limit is filled with distinct bodies | `text_only_duplicate_bodies_still_fill_the_limit` | 03 |
| AC40-14 | End to end: a 1-character token ranks the row carrying it first | `text_only_single_char_token_ranks_matching_row_first` | 04 |
| AC40-15 | End to end: "d3" and "1" each rank their row first | `text_only_two_char_and_digit_tokens_rank_matching_rows_first` | 04 |
| AC40-16 | End to end: short tokens with FTS5 control syntax never error | `text_only_short_tokens_with_fts5_control_syntax_never_error` | 06 |
| AC40-17 | Property: content terms are every non-stopword token of any length, in first-seen order; else fallback | `prop_content_terms_are_every_non_stopword_token` | 04, 05, 07 |
| AC40-18 | Property: every term is one FTS5 literal; content terms are alphanumeric | `prop_every_term_is_one_fts5_string_literal`, `prop_content_path_terms_are_alphanumeric` | 06 |
| AC40-19 | Unit: short tokens kept ("c", "1", "d3", "ii"); short-only query takes the content path | `one_and_two_char_content_tokens_are_kept`, `short_token_only_query_takes_the_content_path` | 04, 07 |
| AC40-20 | Unit: stopwords, including one-character ones, are dropped; empty fragments never emitted | `stopwords_are_dropped`, `short_stopwords_are_still_dropped`, `empty_split_fragments_are_never_emitted` | 04, 05 |
| AC40-21 | Unit: fallback unchanged for all-stopword queries | `all_stopword_query_falls_back_to_raw_or`, `all_short_stopword_query_still_falls_back_to_raw_or`, `escapes_double_quotes_in_fallback_tokens` | 07 |
| AC40-22 | AC-038 parity holds with short tokens: the corpus has two rows with 1–2 character tokens, the query alphabet has "d3", "c", "ii", "1", and the reference mirror keeps short tokens | `fts5_injection_safety.rs`: `fts5_control_syntax_is_safe_and_matches_reference` | 04, 06 |
| AC40-23 | No doc still states the old behavior | Review of the files in [design.md](design.md) § 5, plus `rg -n "shorter than three\|≥3 chars\|sub-3-char" src/rust/crates/fathomdb-query src/rust/crates/fathomdb-engine/src` returns nothing | 08 |
| AC40-25 | End to end: a possessive query ("Cow's milk") ranks the topical row first and does not error | `text_only_possessive_query_ranks_topical_row_first` | 04 |
| AC40-26 | Unit: possessive and contraction fragments ("s", "t") are content tokens, pinned explicitly | `possessive_and_contraction_fragments_are_content_tokens` | 04, 05 |
| AC40-27 | Projected search order equals the documented full sort for limits 1, 2, 3, 5, 8, 10, 12, 100 on a corpus with three bm25 tie groups | `projected_search_matches_documented_order_across_tie_boundaries` | C2, C3 |
| AC40-28 | An eligible projected search records route `projected_rank_stream_strict_boundary` or `projected_rank_stream_tie_completed`, and its query plan has no `USE TEMP B-TREE FOR ORDER BY`; the forced full sort records `projected_full_sort_forced` and its plan does | `projected_search_uses_rank_stream_without_temp_order_by_btree` | C1 |
| AC40-29 | With the stream statement broken, projected search returns the documented order and records exactly one route, `projected_full_sort_fallback` | `projected_search_falls_back_to_full_sort_when_stream_fails` | C5 |
| AC40-30 | An attribute-filtered projected search returns only in-cohort rows in the documented order restricted to the cohort; the unfiltered list is unchanged | `projected_search_attribute_filter_keeps_cohort_and_order` | C4 |
| AC40-31 | Exactly `limit` rows at strict boundaries (2, 8) with 12 matches | `projected_search_returns_exactly_limit_rows_at_strict_boundaries` | C3 |
| AC40-32 | Body-path and existing projected-search contracts unchanged | existing `slice20_fts_rank_stream.rs`, `slice18_retrieval_result_limits.rs`, `slice45_nested_source_projections.rs`, `slice30_dependency_closure.rs`, `slice80_search_view_paths.rs`, `slice35_eligibility_pretruncation.rs` | C6 |
| AC40-33 | Recall-any, Recall-all and MRR on hand-built gold: two answer sessions, one found at 5, both at 10; two rounds of one session do not satisfy Recall-all; `None` without relevant rounds | `test_recall_all_needs_every_answer_session_while_recall_any_needs_one`, `test_two_rounds_of_one_session_do_not_satisfy_recall_all`, `test_metrics_are_none_without_relevant_rounds_and_zero_when_nothing_found` | D1 |
| AC40-34 | Seeded bootstrap CI; paired deltas are treatment − control overall and per type | `test_bootstrap_ci_is_seeded_and_degenerate_on_constant_differences`, `test_paired_deltas_are_treatment_minus_control_overall_and_per_type` | D1 |
| AC40-35 | Decision rule: adopt when (a), (b), (c) hold; reject on each failed clause, including the boundary cases lower bound = 0 and overall delta = 0 | `test_decision_rule_adopts_when_all_three_clauses_hold`, `test_decision_rule_rejects_when_any_clause_fails` (4 cases), `test_decision_rule_accepts_an_overall_delta_of_exactly_zero` | D3 |
| AC40-36 | Sweep config is typed, pins the v1 corpus and document unit, the arms, k = 5 and 10, the bootstrap seed and the rule verbatim; a wrong schema is refused | `test_sweep_config_is_typed_and_states_the_rule_verbatim` | D1, D3 |
| AC40-37 | Gold-leak discipline: the sweep's `convert_question` and `gold_for` are `comp03_answer`'s; gold changes leave hybrid retrieval byte-identical | `test_sweep_reuses_answer_harness_conversion_and_gold`, `test_gold_changes_leave_hybrid_retrieval_byte_identical` | D2 |
| AC40-38 | Every question (abstentions included) is retrieved, abstentions are not scored, rerun does no new work, a wrong dataset digest is refused | `test_sweep_covers_every_question_and_resume_does_no_new_work`, `test_sweep_refuses_a_dataset_whose_digest_differs` | D2 |
| AC40-39 | Device: a CUDA request resolved to CPU (or unresolved) is refused; the sweep records requested and effective device | `test_device_check_refuses_a_cuda_request_that_resolved_to_cpu`, `test_sweep_hybrid_arm_records_the_resolved_device_and_writes_a_verdict` | D2 |
| AC40-40 | Hybrid store: the fts arm's plain bodies, one vector row per document, at least one `vector` hit, crosses a vocabulary gap FTS cannot; coverage check refuses a store without vectors; build records drain (embedding) time apart | `test_hybrid_arm_store_has_one_vector_row_per_document_and_vector_hits`, `test_vector_coverage_check_refuses_a_store_without_vectors`, `test_sweep_hybrid_arm_records_the_resolved_device_and_writes_a_verdict` | D1, D2 |
| AC40-41 | Withdrawn by R1 ruling Q-D2: the `fathomdb-fts-projected` variant was dropped with the JSON-body hybrid shape | — | — |
| AC40-42 | `comp_fts`: the `fathomdb` arm's source and settings are byte-unchanged; `fathomdb-hybrid` and campaign `comp01-hybrid` are registered; the sweep's fts record carries v1's settings | `test_comp_fts_fts_arm_is_byte_unchanged_and_hybrid_arm_is_registered`, `test_sweep_covers_every_question_and_resume_does_no_new_work` | D4 |
| AC40-43 | v2 answer config keeps v1's questions, corpus, document unit, evidence, models and metrics, widens to three arms at k = 10, gates only the hybrid arm | `test_v2_config_keeps_v1_inputs_and_widens_to_three_arms_at_k10` | D4 |
| AC40-44 | Arm order: rotation equals v1's alternation for two arms; each of three arms takes each position once per three questions | `test_arm_order_rotation_reduces_to_the_v1_alternation_for_two_arms`, `test_arm_order_rotation_puts_each_of_three_arms_in_each_position_once` | D4 |
| AC40-45 | Hybrid gate: a `reject` sweep verdict runs no hybrid work; `adopt` runs it; v1 tests unchanged | `test_hybrid_arm_is_skipped_unless_the_sweep_verdict_is_adopt`, `test_hybrid_arm_runs_when_the_sweep_verdict_is_adopt`, the pre-existing `test_comp03_answer.py` tests | D4 |
| AC40-46 | D3: the twelve words are stopwords and `d`, `c`, `d3`, `ii`, `1`, `b12` are not; each word alone takes the fallback; possessive and contraction fragments and the short function words are dropped; short content tokens survive | `unit_d_words_are_in_the_stop_list_and_content_letters_are_not`, `each_unit_d_word_alone_takes_the_fallback`, `possessive_and_contraction_fragments_are_dropped`, `short_function_words_are_dropped`, `short_content_tokens_survive_the_unit_d_stop_list` | D5 |
| AC40-47 | D3: every other Slice 40 compiler and engine test still passes unchanged; `possessive_and_contraction_fragments_are_content_tokens` is deleted (R1 ruling Q-D1) and the AC-038 private stop-list copy gains the twelve words (Q-D3) | the `fathomdb-query` test module, `slice40_fts_ranking.rs`, `fts5_injection_safety.rs`, `slice40_projected_fts_stream.rs` | D5 |
| AC40-48 | D3 measurement gate (REQ40-D6) | orchestrator measurement after W4 | D6 |
| AC40-49 | Parity comparison is exact in `(id, score, branch)`: order, a one-ULP score change, branch, hit count and query set all mismatch | `test_parity_requires_exact_id_score_and_branch_per_hit` | E2 |
| AC40-50 | Text-only parity fixture: at least two duplicate bodies; every D3 word, the short content tokens `d`, `c`, `d3`, `1`, `ii` and an apostrophe across the queries; deterministic | `test_text_only_parity_fixture_carries_the_slice_40_query_shapes` | E1 |
| AC40-51 | Projected parity fixture: a bm25 tie group straddles the limit; the filter case returns a strict, non-empty subset | `test_projected_parity_fixture_puts_a_bm25_tie_group_across_the_limit` | E1 |
| AC40-52 | Dense parity fixture: one vector row per document; `search` returns a `vector` hit | `test_dense_parity_fixture_store_embeds_every_document` | E1 |
| AC40-53 | Registry measures `search_text_only`, `search_projected_text` and `search`; the latter two normalise to ordered hits | `test_registry_measures_the_three_slice_40_verbs_with_ordered_hits` | E1 |
| AC40-54 | Python driver emits `(id, score, branch)` hits on request, equal to the Python oracle on the same file, for all three cases; requests without a normalizer still return IDs only | `test_python_driver_hits_equal_the_python_oracle_on_the_same_database` (2 cases), `test_python_driver_dense_hits_equal_the_python_oracle`, `test_pre_existing_requests_without_a_normalizer_still_return_ids_only` | E1, E2 |
| AC40-55 | TypeScript and Rust drivers pass the same cases; the suite records every SDK equal to the oracle in every repetition | the TS and Rust driver suites (`test_sdk_typescript_driver.py`, `sdk_drivers/rust/tests`) extended at W4; the suite run by the orchestrator | E1–E3 |
| AC40-56 | Interface docs describe the three verbs' Slice 40 behaviour; the public RRF formula reads `RRF_K = 30`, 3:1; `agent-lint-docs.sh` passes | review plus `./scripts/agent-lint-docs.sh` and `./scripts/agent-lint-md.sh` | E4 |
| AC40-57 | Drift check: PASS inside tolerance, DRIFT outside (absolute, relative and per-question tolerances), MISSING when absent; non-zero exit on DRIFT or MISSING | `test_every_metric_inside_its_tolerance_passes`, `test_a_metric_outside_its_tolerance_drifts`, `test_a_missing_metric_is_reported_and_is_not_a_pass`, `test_question_tolerance_is_one_question_of_the_type_count`, `test_cli_exits_non_zero_on_drift_or_missing` | F3 |
| AC40-58 | Baseline manifest lists the five benchmarks with inputs, commit, build recipe, command, receipt and tolerances; answer accuracy marked not a gate | `test_slice_40_baseline_manifest_lists_the_five_benchmarks` | F1 |
| AC40-59 | Latency harness keeps the six diagnostic cells and the script's percentiles | `test_latency_harness_keeps_the_diagnostic_cells_and_percentiles` | F2 |
| AC40-60 | Receipts record engine commit, native SHA-256 and version; a malformed commit is refused | `test_engine_identity_records_the_commit_and_the_native_digest` | F4 |
| AC40-61 | Evidence caps: per-round then total, rank order; the baseline policy shows the same rounds as `render_evidence` | `test_caps_apply_per_round_then_total_in_rank_order`, `test_the_baseline_policy_matches_the_answer_harness_renderer` | G1 |
| AC40-62 | Session expansion order and its stop at the cap | `test_expansion_appends_remaining_rounds_of_hit_sessions_by_best_hit_rank`, `test_expansion_stops_at_the_total_cap` | G1 |
| AC40-63 | Prompt metrics; the twelve-policy grid; the rule with its one-question constraint, tie-break and baseline fallback | `test_prompt_metrics_count_gold_sessions_and_answer_rounds`, `test_policy_grid_is_the_stated_twelve_policies`, `test_rule_picks_the_highest_all_in_prompt_without_losing_more_than_one_question_per_type`, `test_rule_breaks_ties_on_fewer_chars_and_keeps_the_baseline_when_nothing_beats_it` | G1, G2 |
| AC40-64 | Chronological rendering (newest last, dates kept, same rounds); the recency sentence only when enabled; both off is byte-identical to v2 | `test_chronological_rendering_orders_by_session_date_newest_last_and_keeps_dates`, `test_chronological_rendering_selects_the_same_rounds_as_rank_order_under_the_cap`, `test_recency_sentence_is_added_only_when_enabled_and_v2_rendering_is_unchanged` | G3 |
| AC40-65 | The recency rule (KU plus temporal does not drop; overall drops by at most 1) | `test_recency_rule_adopts_unless_ku_plus_temporal_drops_or_overall_drops_by_more_than_one` | G4 |
| AC40-66 | v3 and v4 configs and the verdict file | `test_v3_and_v4_configs_take_the_policy_from_the_verdict_and_differ_only_in_recency`, `test_evidence_verdict_file_names_its_inputs` | G2, G4 |
| AC40-24 | The full gate passes | `cargo test -p fathomdb-engine`, `cargo test -p fathomdb-query`, `./scripts/agent-lint.sh`, `./scripts/agent-lint-md.sh`, then `./scripts/agent-verify.sh` on the final W5 tree | all |
