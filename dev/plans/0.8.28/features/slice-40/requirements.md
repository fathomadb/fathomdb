---
title: FathomDB 0.8.28 Slice 40 — requirements and acceptance criteria
status: REVISED AFTER R1 (APPROVE-WITH-FIXES, findings F1–F6)
target_release: 0.8.28
observed_on: 2026-10-15
---

# Slice 40 requirements and acceptance criteria

Slice 40 fixes two ranking defects in FathomDB's text retrieval. Both were
found by the COMP-01 benchmark and both are correctness bugs, not tuning.
[design.md](design.md) gives the mechanism, [test-plan.md](test-plan.md)
the tests.

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

## 3. Non-goals

- No BM25 `k1`/`b` tuning and no change to `RRF_K` or the arm weights.
- No change to the stopword list.
- No new public API, no new knob, no schema change, no migration.
- No near-duplicate detection; only byte-identical bodies are affected, as
  they already are by fusion's dedupe on body.
- No change to the direct text-only candidate window (§ 4).
- No change to the frozen IR-C experiment harnesses
  (`tests/support/ir_retrieval.rs` and its includers), which document
  themselves as a verbatim historical copy.
- No fix to the stale `Σ 1/(60 + rank)` formula in the public docs
  (`RRF_K` is 30 and the weights are 3:1). Recorded as a follow-up; it is
  not caused by this slice.

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
another file is named; query tests are in the `tests` module of
`src/rust/crates/fathomdb-query/src/lib.rs`.

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
| AC40-24 | The full gate passes | `cargo test -p fathomdb-engine`, `cargo test -p fathomdb-query`, `./scripts/agent-lint.sh`, `./scripts/agent-lint-md.sh`, then `./scripts/agent-verify.sh` on the final W5 tree | all |
