---
title: FathomDB 0.8.28 Slice 40 — test plan
status: REVISED AFTER R1 (W3–W5 results in § 5 and § 6); Defect C tests added 2026-10-16 (§ 3b, § 5.3)
target_release: 0.8.28
observed_on: 2026-10-16
---

# Slice 40 test plan

Every test of the slice, the AC it checks ([requirements.md](requirements.md)
§ 5), and its state at the W2 RED commit (production code still at the
baseline `574b09e78`). All tests are Rust; no Python or TypeScript test is
added because neither binding changes.

## 1. Commands

```bash
cd src/rust
cargo test -p fathomdb-query
cargo test -p fathomdb-engine --test slice40_fts_ranking
cargo test -p fathomdb-engine --test fts5_injection_safety
cargo test -p fathomdb-engine --test pr_g9_rrf_fusion
```

W5 then runs the full gate, `./scripts/agent-verify.sh`, once on the final
tree.

## 2. Engine: `src/rust/crates/fathomdb-engine/tests/slice40_fts_ranking.rs` (new)

Pure-function tests use hand-built `SearchHit` lists. End-to-end tests open
a real database with `Engine::open_with_choice(…, EmbedderChoice::Caller(NoopEmbedder))`
(the `slice20_fts_rank_stream.rs` / `slice23_text_limit_prefix_stability.rs`
setup), write nodes with logical ids, and call `search_text_only*`. The
duplicate corpus gives every row the same token count, so bm25 ties and
the FTS rank is write order; the byte-identical pair is written 13th and
14th, the measured NFCorpus shape.

| Test | AC | Proves | W2 state |
| --- | --- | --- | --- |
| `single_arm_duplicate_body_does_not_outrank_higher_ranked_distinct_body` | 01 | Pair at ranks 13/14 stays 13th, rank 1 stays first | RED |
| `single_arm_duplicate_body_scores_once_at_first_occurrence_rank` | 02 | Duplicate scores `w_text/(K+2)` only; next body keeps rank 3 | RED |
| `duplicate_body_representative_is_first_occurrence` | 03 | Representative cursor/id is the first occurrence | GREEN (guard) |
| `vector_arm_duplicate_body_scores_once` | 04 | Rule on the vector arm | RED |
| `graph_arm_duplicate_body_scores_once` | 04 | Rule on the graph arm | RED |
| `duplicate_dedupe_is_per_arm_not_global` | 05 | One vector term plus one text term | RED |
| `cross_arm_agreement_still_accumulates_with_in_arm_duplicates` | 06 | Agreement kept when the text arm also has a duplicate | RED |
| `three_arm_agreement_sums_one_term_per_arm` | 06 | Three-arm agreement sum, no duplicates | GREEN (guard) |
| `prop_text_only_fusion_is_monotone_in_first_occurrence_rank` | 07 | Property, text arm only | RED |
| `prop_fusion_matches_once_per_arm_reference` | 08 | Property, any three arms vs the reference model | RED |
| `prop_distinct_within_arm_inputs_are_unchanged` | 09 | Property, no in-arm duplicates: unchanged | GREEN (guard) |
| `prop_two_arm_fusion_equals_three_arm_with_empty_graph` | 10 | Property, two-arm contract | GREEN (guard) |
| `text_only_duplicate_bodies_do_not_win_rank_one` | 11 | End to end, top 10 in FTS order | RED |
| `text_only_duplicate_body_surfaces_once_at_first_occurrence` | 12 | End to end, pair once and 13th at `limit = 20` | RED |
| `text_only_duplicate_bodies_still_fill_the_limit` | 13 | End to end, 10 distinct results | GREEN (guard) |
| `text_only_single_char_token_ranks_matching_row_first` | 14 | "Vitamin D" ranks the "d" row first | RED |
| `text_only_two_char_and_digit_tokens_rank_matching_rows_first` | 15 | "D3" and "IGF-1" rank their rows first | RED |
| `text_only_possessive_query_ranks_topical_row_first` (W3, R1 F2) | 25 | "Cow's milk" ranks the cow/milk row first, no error | GREEN (guard) |
| `text_only_short_tokens_with_fts5_control_syntax_never_error` | 16 | Eight control-syntax queries with short tokens return `Ok` | GREEN (guard) |

The properties compare against `reference_fusion`, a model written from
the contract in REQ40-01..03 (per-arm credited set, positional rank,
accumulation in vector, text, graph order, then the existing sort). The
model accumulates in the same order as the engine, so scores compare
exactly, not within a tolerance. For inputs without in-arm duplicates the
model is the pre-slice algorithm, which is why AC40-09 is green now and
must stay green.

## 3. Query compiler: `src/rust/crates/fathomdb-query/src/lib.rs` test module

| Test | AC | Proves | W2 state |
| --- | --- | --- | --- |
| `content_tokens_are_or_joined` (existing) | — | OR shape, runs of spaces | GREEN |
| `stopwords_are_dropped` (renamed) | 20 | "of", "the" dropped | GREEN |
| `short_stopwords_are_still_dropped` | 20 | "a", "i" dropped; "d" kept | RED |
| `one_and_two_char_content_tokens_are_kept` | 19 | "c", "1", "d3", "ii" kept | RED |
| `possessive_and_contraction_fragments_are_content_tokens` (W3, R1 F2) | 26 | "Cow's Milk" gives `"cow" OR "s" OR "milk"`; "don't panic" gives `"don" OR "t" OR "panic"` | RED |
| `short_token_only_query_takes_the_content_path` | 19 | "D3" gives `"d3"`, "C" gives `"c"` | RED |
| `empty_split_fragments_are_never_emitted` | 20 | No `""` phrase from separator runs | RED |
| `duplicate_content_tokens_collapse_in_order` (existing) | — | First-seen de-duplication | GREEN |
| `control_characters_are_stripped_to_literals` (existing) | 18 | Control syntax becomes literals | GREEN |
| `all_stopword_query_falls_back_to_raw_or` (existing) | 21 | Fallback unchanged | GREEN |
| `escapes_double_quotes_in_fallback_tokens` (existing) | 21 | Fallback quote escaping | GREEN |
| `all_short_stopword_query_still_falls_back_to_raw_or` | 21 | "a I" still takes the fallback | GREEN (guard) |
| `prop_content_terms_are_every_non_stopword_token` | 17 | Property, content terms of any length | RED |
| `prop_every_term_is_one_fts5_string_literal` | 18 | Property, injection safety over any Unicode string | GREEN (guard) |
| `prop_content_path_terms_are_alphanumeric` | 18 | Property, content terms non-empty and alphanumeric | RED |

**The renamed test.** `stopwords_and_short_tokens_are_dropped` becomes
`stopwords_are_dropped`. The owner ruled the short-token drop a defect, so
the old name described behavior that is being removed. Its input ("status
of the alpha") has no short non-stopword token, so its expectation is
unchanged; only the name and comment change. No assertion of an existing
test was weakened.

`prop_content_path_terms_are_alphanumeric` is RED only because the content
path now covers more queries: under the old compiler "0!" takes the
fallback and emits `"0!"`.

## 3b. Defect C: `src/rust/crates/fathomdb-engine/tests/slice40_projected_fts_stream.rs` (new)

A new file, not `slice40_fts_ranking.rs`: these tests need the `test-hooks`
witnesses and statement hooks, so the target is registered with
`required-features = ["test-hooks"]`, while `slice40_fts_ranking.rs` runs
in the default build. The tests set process-wide environment variables, so
they serialize on one lock (recovered after a panic so one RED test does
not mask the others).

```bash
cd src/rust
cargo test -p fathomdb-engine --features test-hooks --test slice40_projected_fts_stream
```

**Corpus.** Fourteen nodes with a searchable FTS projection `title` and a
filterable projection `cohort` (alternating `c1`/`c2`). Twelve titles match
`s40proj` in three groups of identical text, so bm25 ties exactly inside a
group: A (2 rows, best), T (6 rows), C (4 rows, worst). Writes interleave
the groups, so write-cursor order differs from bm25 order. Two rows do not
match.

**Oracle.** The documented contract ("bm25 ascending then write cursor
ascending", `Engine::search_projected_text`) run as SQL directly against
the database file after the engine closes: `SELECT p.write_cursor,
bm25(property_search_index), n.body … WHERE p.attr_name = 'title' AND
property_search_index MATCH '"s40proj"' ORDER BY bm25(...) ASC,
p.write_cursor ASC`, with score `-bm25`. It is valid without the engine's
node predicate because the corpus has no validity windows, supersession,
non-active states or dependencies, so that predicate admits every row; the
oracle asserts it sees all 12 matches, the three tie groups, the strict
gaps between them, and that bm25 order differs from write order. It does
not share code with either engine statement, so it checks the stream and
the fallback alike.

| Test | AC | Proves | State now |
| --- | --- | --- | --- |
| `projected_search_matches_documented_order_across_tie_boundaries` | 27 | Limits 1, 2, 3, 5, 8, 10, 12, 100 equal the oracle prefix, `(write_cursor, score)` | GREEN (regression net) |
| `projected_search_uses_rank_stream_without_temp_order_by_btree` | 28 | Routes `projected_rank_stream_strict_boundary` (limit 2) and `projected_rank_stream_tie_completed` (limit 5); plan has no temporary ORDER BY b-tree; forced full sort records `projected_full_sort_forced` with a plan that has one | RED |
| `projected_search_falls_back_to_full_sort_when_stream_fails` | 29 | With `FATHOMDB_FTS_FAIL_STREAM_FOR_TEST`, the oracle order and exactly one route, `projected_full_sort_fallback` | RED (route); the order part already holds |
| `projected_search_attribute_filter_keeps_cohort_and_order` | 30 | Filter `cohort = c1`, limits 1, 3, 4 (cohort has 5 matches): only `c1` rows, in oracle order restricted to the cohort; unfiltered list equals the oracle | GREEN (regression net) |
| `projected_search_returns_exactly_limit_rows_at_strict_boundaries` | 31 | Limits 2 and 8 return exactly 2 and 8 of 12 matches | GREEN (regression net) |

The three GREEN tests pass against the current full-sort implementation,
as expected: they pin the contract the stream must keep. The two RED tests
fail because the projected path records no route and no plan today.

## 4. Existing tests touched

`src/rust/crates/fathomdb-engine/tests/fts5_injection_safety.rs` (AC-038):
its `content_tokens` reference mirrors the compiler, so its `t.len() >= 3`
filter becomes `!t.is_empty()` and two comments drop "≥3 chars". The
mirror alone would be vacuous, because the original corpus has no 1–2
character token (R1 F1). W3 therefore adds two corpus rows,
"vitamin d c 1 ii study" and "igf 1 d3 b12 trial", and adds "d3", "c",
"ii" and "1" to the query alphabet, so some of the 100 fixture queries mix
a short token with a long one (for example `delta AND d3`). The fixture
stays at exactly 100 queries. The test is RED before W4 (the compiler drops
the short token, the mirror keeps it) and must be GREEN after (AC40-22).

Expected to stay green at W4, with reasons: `pr_g9_rrf_fusion.rs` (no
in-arm duplicates), `slice23_text_limit_prefix_stability.rs` (asserts prefix
stability only; node 10 stops jumping to rank 1),
`slice71_search_statement_trace.rs` (optimized equals control; both fuse
the same way), `exp_obs_explain.rs` (per-arm ranks are first-occurrence
already). W5's full gate confirms.

## 5. RED run at W2 (2026-10-15)

The W2 run is below; the W3 additions are in § 5.1 and the GREEN run in
§ 5.2.

`cargo test -p fathomdb-query`: 8 passed, 6 failed.

```text
---- tests::empty_split_fragments_are_never_emitted stdout ----
assertion `left == right` failed
  left: "\"C\" OR \"..\" OR \"D\""
 right: "\"c\" OR \"d\""
---- tests::one_and_two_char_content_tokens_are_kept stdout ----
assertion `left == right` failed
  left: "\"vitamin\" OR \"enriched\" OR \"bacon\""
 right: "\"vitamin\" OR \"c\" OR \"enriched\" OR \"bacon\""
---- tests::short_stopwords_are_still_dropped stdout ----
assertion `left == right` failed
  left: "\"study\" OR \"ran\" OR \"vitamin\""
 right: "\"study\" OR \"ran\" OR \"vitamin\" OR \"d\""
---- tests::short_token_only_query_takes_the_content_path stdout ----
assertion `left == right` failed
  left: "\"D3\""
 right: "\"d3\""
---- tests::prop_content_terms_are_every_non_stopword_token stdout ----
Test failed: assertion failed: `(left == right)`
  left: `"\"^j\""`,
 right: `"\"j\""` at src/rust/crates/fathomdb-query/src/lib.rs:233.
minimal failing input: raw = "^j"
---- tests::prop_content_path_terms_are_alphanumeric stdout ----
Test failed: content term "\"0!\"" is not alphanumeric at src/rust/crates/fathomdb-query/src/lib.rs:257.
minimal failing input: raw = "0!"
```

`cargo test -p fathomdb-engine --test slice40_fts_ranking`: 6 passed,
12 failed.

```text
---- duplicate_dedupe_is_per_arm_not_global stdout ----
assertion `left == right` failed: one vector term + one text term
  left: 0.22278225806451613
 right: 0.12903225806451613
---- cross_arm_agreement_still_accumulates_with_in_arm_duplicates stdout ----
assertion `left == right` failed: agree = w_vec/(K+1) + w_text/(K+2); the rank-3 duplicate adds nothing
  left: 0.21691715542521994
 right: 0.12600806451612903
---- graph_arm_duplicate_body_scores_once stdout ----
assertion `left == right` failed
  left: ["dup", "g"]
 right: ["g", "dup"]
---- single_arm_duplicate_body_scores_once_at_first_occurrence_rank stdout ----
assertion `left == right` failed
  left: ["dup", "a", "b"]
 right: ["a", "dup", "b"]
---- single_arm_duplicate_body_does_not_outrank_higher_ranked_distinct_body stdout ----
assertion `left == right` failed: the rank-1 distinct body must stay first; fused order was ["duplicate body", "distinct 00", …]
  left: "duplicate body"
 right: "distinct 00"
---- prop_text_only_fusion_is_monotone_in_first_occurrence_rank stdout ----
Test failed: assertion failed: `(left == right)`
  left: `0.1905241935483871`,
 right: `0.0967741935483871` at src/rust/crates/fathomdb-engine/tests/slice40_fts_ranking.rs:286.
minimal failing input: symbols = [2, 2]
---- vector_arm_duplicate_body_scores_once stdout ----
assertion `left == right` failed: dup = w_vec/(K+1) only
  left: 0.06256109481915934
 right: 0.03225806451612903
---- prop_fusion_matches_once_per_arm_reference stdout ----
Test failed: assertion failed: `(left == right)`
  left: `[SearchHit { id: IdSpace { space: Content, value: "3000" }, write_cursor: 3000, kind: "doc", body: "body-1", score: 0.06350806451612903, branch: GraphArm, source_id: None, ce_score: None }]`,
 right: `[SearchHit { id: IdSpace { space: Content, value: "3000" }, write_cursor: 3000, kind: "doc", body: "body-1", score: 0.03225806451612903, branch: GraphArm, source_id: None, ce_score: None }]` at src/rust/crates/fathomdb-engine/tests/slice40_fts_ranking.rs:307.
minimal failing input: v = [], t = [], g = [1, 1]
---- text_only_duplicate_bodies_do_not_win_rank_one stdout ----
assertion `left == right` failed: the rank-1 distinct row must stay first; got ["s40dup duplicate document body", "s40dup distinct document 00", …]
---- text_only_two_char_and_digit_tokens_rank_matching_rows_first stdout ----
assertion `left == right` failed: "d3" must reach FTS5; got ["vitamin d2 dosing study", "vitamin d3 dosing study"]
---- text_only_single_char_token_ranks_matching_row_first stdout ----
assertion `left == right` failed: the row carrying the short token ranks first; got ["vitamin c supplement trial", "vitamin d supplement trial"]
---- text_only_duplicate_body_surfaces_once_at_first_occurrence stdout ----
assertion `left == right` failed: the pair sits at its first FTS rank (13th); got ["s40dup duplicate document body", …]
  left: 0
 right: 12
```

Long `got [...]` lists are shortened with "…"; the assertions are verbatim.
`cargo test -p fathomdb-engine --test fts5_injection_safety`: 2 passed.

proptest saved the minimal cases to
`src/rust/crates/fathomdb-query/proptest-regressions/lib.txt` and
`src/rust/crates/fathomdb-engine/tests/slice40_fts_ranking.proptest-regressions`;
both are committed so the cases replay first.

### 5.1 RED additions at W3 (R1 F1, F2)

`cargo test -p fathomdb-query`: 8 passed, 7 failed (the six of § 5 plus):

```text
thread 'tests::possessive_and_contraction_fragments_are_content_tokens' (2967378) panicked at src/rust/crates/fathomdb-query/src/lib.rs:134:9:
assertion `left == right` failed
  left: "\"cow\" OR \"milk\""
 right: "\"cow\" OR \"s\" OR \"milk\""
```

`cargo test -p fathomdb-engine --test slice40_fts_ranking`: 7 passed,
12 failed (the new `text_only_possessive_query_ranks_topical_row_first`
passes; it is a guard).

`cargo test -p fathomdb-engine --test fts5_injection_safety`: 1 passed,
1 failed.

```text
thread 'fts5_control_syntax_is_safe_and_matches_reference' (2966151) panicked at src/rust/crates/fathomdb-engine/tests/fts5_injection_safety.rs:214:5:
AC-038: 40 fixture queries diverged from safe-grammar reference; first 3: [
    (
        "\"foxtrot c\"",
```

One divergence in full, verbatim (query, engine bodies, reference bodies):

```text
    (
        "delta AND d3",
        [
            "alpha delta golf kilo",
            "delta echo foxtrot",
        ],
        [
            "alpha delta golf kilo",
            "delta echo foxtrot",
            "igf 1 d3 b12 trial",
        ],
    ),
```

### 5.2 GREEN run at W4

After the W4 change to `fusion.rs` and `fathomdb-query/src/lib.rs`, with no
test edited:

```text
cargo test -p fathomdb-query
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
cargo test -p fathomdb-engine --test slice40_fts_ranking --test fts5_injection_safety
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
cargo test -p fathomdb-engine --features test-hooks --test pr_g9_rrf_fusion
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
```

The three `fathomdb-query` lines are the unit tests, `tests/property_template.rs`
and the doctests. `pr_g9_rrf_fusion` needs `--features test-hooks`.

### 5.3 RED run for Defect C (phase 3, 2026-10-16)

Production code at `1f54bc3b8` (Defects A and B fixed, Defect C not):

```text
cargo test -p fathomdb-engine --features test-hooks --test slice40_projected_fts_stream
test projected_search_attribute_filter_keeps_cohort_and_order ... ok
test projected_search_falls_back_to_full_sort_when_stream_fails ... FAILED
test projected_search_matches_documented_order_across_tie_boundaries ... ok
test projected_search_returns_exactly_limit_rows_at_strict_boundaries ... ok
test projected_search_uses_rank_stream_without_temp_order_by_btree ... FAILED

thread 'projected_search_falls_back_to_full_sort_when_stream_fails' (3515748) panicked at src/rust/crates/fathomdb-engine/tests/slice40_projected_fts_stream.rs:330:5:
assertion `left == right` failed: a failed stream records exactly one route, the fallback
  left: []
 right: ["projected_full_sort_fallback"]
thread 'projected_search_uses_rank_stream_without_temp_order_by_btree' (3515751) panicked at src/rust/crates/fathomdb-engine/tests/slice40_projected_fts_stream.rs:286:5:
assertion `left == right` failed: each eligible projected search records the rank-stream route it took
  left: []
 right: ["projected_rank_stream_strict_boundary", "projected_rank_stream_tie_completed"]
test result: FAILED. 3 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.45s
```

`cargo clippy -p fathomdb-engine --features test-hooks --test
slice40_projected_fts_stream -- -D warnings` is clean.

### 5.4 GREEN run for Defect C (W4 at `a3830681d`)

No test edited. Each command from `src/rust`:

```text
cargo test -p fathomdb-engine --features test-hooks --test slice40_projected_fts_stream
test projected_search_falls_back_to_full_sort_when_stream_fails ... ok
test projected_search_attribute_filter_keeps_cohort_and_order ... ok
test projected_search_returns_exactly_limit_rows_at_strict_boundaries ... ok
test projected_search_matches_documented_order_across_tie_boundaries ... ok
test projected_search_uses_rank_stream_without_temp_order_by_btree ... ok
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.48s
cargo test -p fathomdb-engine --features test-hooks --test pr_g9_rrf_fusion
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
cargo test -p fathomdb-engine --test slice40_fts_ranking
test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
cargo test -p fathomdb-engine --test fts5_injection_safety
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
cargo test -p fathomdb-engine --features test-hooks --test slice35_virtual_mutation_manifest
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
cargo test -p fathomdb-engine --features test-hooks --test slice35_eligibility_pretruncation
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s
cargo test -p fathomdb-engine --features test-hooks --test slice20_fts_rank_stream
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.62s
cargo test -p fathomdb-engine --features test-hooks --test slice18_retrieval_result_limits
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
cargo test -p fathomdb-engine --features test-hooks --test slice45_nested_source_projections
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 13.86s
cargo test -p fathomdb-engine --features test-hooks --test slice30_dependency_closure
test result: ok. 27 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 9.23s
cargo test -p fathomdb-engine --features test-hooks --test slice80_search_view_paths
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s
cargo test -p fathomdb-query
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

`slice35_eligibility_pretruncation` owns `slice35_ranked_eligibility_sql_for_test`
and still pins the unchanged fallback statement. `slice35_virtual_mutation_manifest`
passes with the new SELECT builder. `cargo clippy -p fathomdb-engine
--all-targets -- -D warnings` and `cargo clippy -p fathomdb-engine
--features test-hooks --lib --tests -- -D warnings` are clean.

## 6. Pre-existing test failures at W5

A failing pre-existing test is not edited; it is listed here verbatim
(R1 F4). No test was edited at W4 or W5.

**`cargo test -p fathomdb-engine` (default features): exit 0**, 79 test
binaries, 474 passed, 0 failed, 7 ignored. This includes
`slice23_text_limit_prefix_stability`, `exp_obs_explain` and the AC-038
test.

**`cargo test -p fathomdb-engine --features test-hooks` (not a gate
invocation; run to reach the `test-hooks`-only targets such as
`pr_g9_rrf_fusion` and `slice71_search_statement_trace`).** The first run
(fail-fast, host load average 12) stopped in the lib unit tests:

```text
thread 'slice90_close_review_tests::provider_deadline_starts_after_sqlite_quiescence_and_is_never_renewed' (3050427) panicked at src/rust/crates/fathomdb-engine/src/slice90_close_review_tests.rs:183:5:
provider drain exceeded one budget: 252.40357ms
thread 'slice90_post_probe_real_error_tests::real_post_probe_visibility_error_uses_shared_unwind_and_retains_provider_only' (3050442) panicked at src/rust/crates/fathomdb-engine/src/slice90_post_probe_real_error_tests.rs:91:5:
assertion `left == right` failed: timed-out provider remains counted
  left: 0
 right: 1
```

Both are close-path timing tests. Rerun alone they pass, and the full lib
suite then passed twice (`test result: ok. 111 passed; 0 failed`).
Treated as a load-dependent flake unrelated to ranking.

The `--no-fail-fast` run: 211 test binaries, 1303 passed, 10 failed,
36 ignored. All ten fail outside search ranking:

```text
pr_g8_dangling_edges (7 of 8): s20_edge_to_one_missing_endpoint_counts_one, s20_default_flag_and_count_commits_the_batch, s20_count_is_sum_over_both_endpoints, s20_edge_to_superseded_node_counts, s20_legacy_null_logical_id_batch_writes_with_defined_count, s20_same_batch_later_inserted_node_is_not_flagged, s20_on_supersession_skips_earlier_keeps_final_active
  each: open: RuntimeConfiguration(TooLate)
tc57_governed_write_race (2 of 5): tc57_control_anonymous_write_loop_does_not_race, tc57_repro_governed_write_loop_races_projection_worker
  each: panicked at src/rust/crates/fathomdb-engine/tests/tc57_governed_write_race.rs:211:6:
  open: RuntimeConfiguration(TooLate)
slice135_robustness_first (1 of 7): slice135_persistent_sqlite_full_preserves_state_until_cap_lifts
  panicked at src/rust/crates/fathomdb-engine/tests/slice135_robustness_first.rs:437:5:
  assertion `left == right` failed: no persistent descriptor growth
    left: Some(4)
   right: Some(33)
```

These targets pass in the default-feature run above. Nine fail in
`Engine::open` with `RuntimeConfiguration(TooLate)`, before any search; the
tenth checks file-descriptor growth. None reaches fusion or the query
compiler before failing. Whether they also fail at the baseline under
`--features test-hooks` was not checked (it needs a second checkout). They are recorded for the
orchestrator, not ruled on here. The gate of record is
`./scripts/agent-verify.sh` (§ 6.1).

### 6.0 Defect C (phase 3, at `a3830681d`)

`cargo test -p fathomdb-engine` (default features): exit 0, 79 test
binaries, 474 passed, 0 failed, 7 ignored. No pre-existing test failed in
any command of § 5.4. Gate results for phase 3 are in § 6.2.

### 6.1 Gates

| Gate | Result |
| --- | --- |
| `./scripts/agent-lint.sh` | exit 0 (`lint-rust-tegra-pool` skipped: needs aarch64 Linux and CUDA) |
| `./scripts/agent-lint-md.sh` | exit 0 |
| `./scripts/agent-verify.sh` at `13879d4af` | exit 1: `registered=202 ran=202 passed=198 failed=4 skipped=0 excluded=0`; `FAILED SUITES: test-check-board-currency test-commission-manifest test-python test-python-native-receipt`; `test-rust rc=0` |
| `test-python`, `test-python-native-receipt` rerun on the clean tree at `0f3cdc403` (the exact commands `agent-test.sh` runs) | first run `1 failed, 1616 passed, 36 skipped`; second run `1617 passed, 36 skipped`, exit 0; receipt exit 0 |

The two Python suites failed in the full gate because the tree was not
clean (an uncommitted test-plan edit and the untracked `output.json`):

```text
E   RuntimeError: candidate artifact build requires a clean worktree:
E    M dev/plans/0.8.28/features/slice-40/test-plan.md
E   ?? dev/plans/0.8.28/features/slice-40/output.json
```

The first clean rerun had one failure, which passed in five isolated runs
and in the next full run (a race in the logging subscriber test, which
does not depend on ranking):

```text
FAILED src/python/tests/test_slice100_logging_subscriber.py::LoggingSubscriberTests::test_handler_database_reentry_is_rejected_without_recursion
>               self.assertEqual(len(errors), 3)
E               AssertionError: 6 != 3
```

**Open, needs an orchestrator ruling: the two release-state suites.**
They check `dev/plans/release-state-0.8.28.json` and
`dev/plans/runs/STATUS-0.8.28.md`, which this slice must not edit and has
not touched (`git diff --name-only 574b09e78..HEAD` names neither). They
come from the baseline state, not from Slice 40:

```text
FAIL  real-repository current-board regression: rc=1 out=STALE  0.8.28 Slice 31: dev/plans/release-state-0.8.28.json records landing SHA 1ceefddd894c041b0a8622856dc2677be3e49c88, but the hand-written ladder row at dev/plans/runs/STATUS-0.8.28.md:66 does not cite it (TC-133).
FAIL  arm 9c (real sweep): rc=1 out=FAIL commission-manifest: `dev/plans/release-state-0.8.28.json` declares end-of-ladder but its ladder is not fully landed (non-LANDED=[30, 31]; absent-from-landed=none).
```

The full gate is therefore not green at this head.

### 6.2 Gates for Defect C (phase 3)

| Gate | Result |
| --- | --- |
| `./scripts/agent-lint.sh` (clean tree, `c86471cb3`) | exit 0 (`lint-rust-tegra-pool` skipped: needs aarch64 Linux and CUDA) |
| `./scripts/agent-lint-md.sh` | exit 0 |
| `./scripts/agent-verify.sh` (clean tree, `c86471cb3`, worktree `.venv`) | exit 1: `registered=202 ran=202 passed=200 failed=2 skipped=0 excluded=0`; `FAILED SUITES: test-check-board-currency test-commission-manifest`; `test-rust rc=0`, `test-python rc=0`, `test-python-native-receipt rc=0` |

The two failures recur unchanged from § 6.1. They check the 0.8.28
release-state files, which this slice does not touch:

```text
FAIL  real-repository current-board regression: rc=1 out=STALE  0.8.28 Slice 31: dev/plans/release-state-0.8.28.json records landing SHA 1ceefddd894c041b0a8622856dc2677be3e49c88, but the hand-written ladder row at dev/plans/runs/STATUS-0.8.28.md:66 does not cite it (TC-133).
STALE  0.8.28: the board claims "Ladder remaining: 31" at dev/plans/runs/STATUS-0.8.28.md:68, naming slice id(s) {31}, but `remaining_ladder` in dev/plans/release-state-0.8.28.json is {-} (TC-133)
FAIL  arm 9c (real sweep): rc=1 out=FAIL commission-manifest: `dev/plans/release-state-0.8.28.json` declares end-of-ladder but its ladder is not fully landed (non-LANDED=[30, 31]; absent-from-landed=none).
```

## 7. R1 rulings on the phase 1 open questions

- **Q1.** Positional rank of the first occurrence: kept.
- **Q2.** The candidate-window shortfall at `limit` near 100 stays a
  follow-up; `collect_complete_rank_boundary` is not touched.
- **Q3.** Yes: the AC-038 corpus gains short-token rows (§ 4, F1).

## 8. Open questions for R1 (phase 3)

- **Q4.** Route names `projected_rank_stream_strict_boundary`,
  `projected_rank_stream_tie_completed`, `projected_full_sort_forced`,
  `projected_full_sort_fallback`: acceptable, or reuse the body path's
  names unprefixed?
- **Q5.** Should the stream statement pin the join order with `CROSS JOIN`
  now (design § 2b.4), or rely on the planner and the plan witness?
- **Q6.** Should `slice35_ranked_eligibility_sql_for_test` also return the
  new stream statement, so the eligibility-before-ranking check covers it?
  Left unchanged in the design (REQ40-C6).
