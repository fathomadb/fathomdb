---
title: FathomDB 0.8.28 Slice 40 — test plan
status: REVISED AFTER R1; Defect C and Units D–G GREEN (§ 5.4, § 5.6, § 5.8, § 5.10); Unit H tests planned 2026-10-16 (§ 3g, § 11), for review
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

The Unit D harness tests run in the `comp03-slice40` worktree from its own
`.venv`, which gained `lancedb==0.40.0` and `pyarrow==26.0.0`
(`.venv/bin/pip install lancedb==0.40.0 pyarrow==26.0.0`, R1 ruling Q-D4)
so the LanceDB-backed tests run instead of skipping:

```bash
cd ~/projects/fathomdb-worktrees/comp03-slice40
env PYTHONPATH=. .venv/bin/python -m pytest -q tests/experiments/test_comp03_retrieval.py tests/experiments/test_comp03_answer.py
env PYTHONPATH=. .venv/bin/python -m pytest -q tests/experiments
```

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

## 3c. Unit D tests

### D3, `fathomdb-query` test module (this worktree)

```bash
cd src/rust
cargo test -p fathomdb-query
```

| Test | AC | State now |
| --- | --- | --- |
| `unit_d_words_are_in_the_stop_list_and_content_letters_are_not` | 46 | RED |
| `each_unit_d_word_alone_takes_the_fallback` (upper-case input tells the fallback from the content path) | 46 | RED |
| `possessive_and_contraction_fragments_are_dropped` | 46 | RED |
| `short_function_words_are_dropped` | 46 | RED |
| `short_content_tokens_survive_the_unit_d_stop_list` | 46 | RED (its inputs carry "my", "so", "up") |

**Superseded test (R1 ruling Q-D1).**
`possessive_and_contraction_fragments_are_content_tokens` (added at W3 for
R1 F2 to pin that "s" and "t" were content) asserted `"cow" OR "s" OR
"milk"`. REQ40-D5 reverses that rule by owner ruling, so the D3 commit
deletes it; `possessive_and_contraction_fragments_are_dropped` covers the
same inputs with the D3 expectation. Every other existing compiler test is
unchanged and passes: none of their inputs contains one of the twelve words
as content, and the property tests read `STOPWORDS` itself.

**Engine tests.** `slice40_fts_ranking.rs` (the "Cow's milk" test still
ranks the cow row first with `"cow" OR "milk"`), `slice40_projected_fts_stream.rs`
and `pr_g9_rrf_fusion.rs` use none of the twelve words as content; they
must stay green at W4.

**AC-038 (`fts5_injection_safety.rs`), read in full.** Its reference model
keeps a private copy of `STOPWORDS` and of the content-token rule, and its
parity check compares the engine's result set with that model's for 100
fixture queries over a 12-row corpus. The query alphabet (`alpha` …
`juliet`, `d3`, `c`, `ii`, `1`) and the template literals (`NEAR`, `body`,
`col`, `NOT`, `AND`, `OR`, `5`, `2`, `3`) contain none of the twelve words,
and neither does the corpus. So the test stays green with or without D3,
and its fixture (corpus and queries) needs no regeneration. By R1 ruling
Q-D3 the D3 commit adds the twelve words to its private stop-list copy so
the parity oracle stays in sync. Because the fixture corpus and queries
contain none of the twelve words, the test is green either way and does
not exercise them; the `fathomdb-query` unit tests are the oracle for D3.

### D1 and D2, harness (`comp03-slice40` worktree)

Run from the worktree root with its own `.venv`, as `scripts/agent-test.sh`
runs the experiment suites (`env PYTHONPATH=. .venv/bin/python -m pytest -q
tests/experiments/…`); the COMP tests are not in the agent-test list, so
they run explicitly:

```bash
cd ~/projects/fathomdb-worktrees/comp03-slice40
env PYTHONPATH=. .venv/bin/python -m pytest -q \
  tests/experiments/test_comp03_retrieval.py tests/experiments/test_comp03_answer.py
```

The worktree's `.venv` (bootstrap of `0e246ad31`) has the dev wheel with
`default-embedder` on CPU and the cached `bge-small` weights, so no hybrid
test skips; it does not have `lancedb`, so the two pre-existing
LanceDB-backed tests skip, and no new test needs LanceDB (the gate tests
drop the LanceDB arm from their config copy). Hybrid tests skip only when
`Engine.open(use_default_embedder=True)` fails, printing the reason.

New file `tests/experiments/test_comp03_retrieval.py` (all RED: the module,
the configs and the new arms do not exist yet):

| Test | AC |
| --- | --- |
| `test_sweep_reuses_answer_harness_conversion_and_gold` | 37 |
| `test_recall_all_needs_every_answer_session_while_recall_any_needs_one` | 33 |
| `test_two_rounds_of_one_session_do_not_satisfy_recall_all` | 33 |
| `test_metrics_are_none_without_relevant_rounds_and_zero_when_nothing_found` | 33 |
| `test_bootstrap_ci_is_seeded_and_degenerate_on_constant_differences` | 34 |
| `test_paired_deltas_are_treatment_minus_control_overall_and_per_type` | 34 |
| `test_decision_rule_adopts_when_all_three_clauses_hold` | 35 |
| `test_decision_rule_rejects_when_any_clause_fails` (a: lower −0.01; a: lower exactly 0; b; c) | 35 |
| `test_decision_rule_accepts_an_overall_delta_of_exactly_zero` | 35 |
| `test_sweep_config_is_typed_and_states_the_rule_verbatim` | 36 |
| `test_device_check_refuses_a_cuda_request_that_resolved_to_cpu` | 39 |
| `test_comp_fts_fts_arm_is_byte_unchanged_and_hybrid_arm_is_registered` (pins the SHA-256 of `inspect.getsource(FathomArm)` at `0e246ad31` and its settings dict) | 42 |
| `test_hybrid_arm_store_has_one_vector_row_per_document_and_vector_hits` | 40 |
| `test_vector_coverage_check_refuses_a_store_without_vectors` | 40 |
| ~~`test_fts_projected_arm_stores_json_bodies_and_searches_text_only`~~ (removed, Q-D2) | 41 |
| `test_sweep_covers_every_question_and_resume_does_no_new_work` | 38, 42 |
| `test_sweep_refuses_a_dataset_whose_digest_differs` | 38 |
| `test_sweep_hybrid_arm_records_the_resolved_device_and_writes_a_verdict` | 39, 40 |

Additions to `tests/experiments/test_comp03_answer.py` (pre-existing tests
untouched):

| Test | AC | State |
| --- | --- | --- |
| `test_v2_config_keeps_v1_inputs_and_widens_to_three_arms_at_k10` | 43 | RED |
| `test_arm_order_rotation_reduces_to_the_v1_alternation_for_two_arms` | 44 | RED |
| `test_arm_order_rotation_puts_each_of_three_arms_in_each_position_once` | 44 | RED |
| `test_gold_changes_leave_hybrid_retrieval_byte_identical` | 37 | RED |
| `test_hybrid_arm_is_skipped_unless_the_sweep_verdict_is_adopt` | 45 | RED |
| `test_hybrid_arm_runs_when_the_sweep_verdict_is_adopt` | 45 | RED |

## 3d. Unit E tests (`comp03-slice40` worktree)

```bash
cd ~/projects/fathomdb-worktrees/comp03-slice40
env PYTHONPATH=. .venv/bin/python -m pytest -q tests/experiments/test_sdk_parity_slice40.py
```

New file `tests/experiments/test_sdk_parity_slice40.py`, all RED:

| Test | AC |
| --- | --- |
| `test_parity_requires_exact_id_score_and_branch_per_hit` | 49 |
| `test_text_only_parity_fixture_carries_the_slice_40_query_shapes` | 50 |
| `test_projected_parity_fixture_puts_a_bm25_tie_group_across_the_limit` | 51 |
| `test_dense_parity_fixture_store_embeds_every_document` | 52 |
| `test_registry_measures_the_three_slice_40_verbs_with_ordered_hits` | 53 |
| `test_python_driver_hits_equal_the_python_oracle_on_the_same_database` (text-only, projected) | 54 |
| `test_python_driver_dense_hits_equal_the_python_oracle` | 54 |
| `test_pre_existing_requests_without_a_normalizer_still_return_ids_only` | 54 |

These run against the harness `.venv`'s comp-lineage wheel. Parity does not
depend on the engine version; the slice-head artifacts are for the suite
run (design § 2d.2).

The TypeScript and Rust driver tests (AC40-55) are written at W4 in their
existing suites (`tests/experiments/test_sdk_typescript_driver.py`,
`experiments/sdk_drivers/rust/tests/`). Their pre-existing capability
assertions pin today's three operations and need the Q-E1 ruling first.

In this worktree there is no RED test for E4: no checker pins the
interface docs' verb descriptions. `scripts/check-sdk-surface-parity.py`
checks names and signatures only.

## 3e. Unit F tests (`comp03-slice40` worktree)

```bash
env PYTHONPATH=. .venv/bin/python -m pytest -q tests/experiments/test_baseline_check.py
```

New file `tests/experiments/test_baseline_check.py`, all RED:

| Test | AC |
| --- | --- |
| `test_every_metric_inside_its_tolerance_passes` | 57 |
| `test_a_metric_outside_its_tolerance_drifts` | 57 |
| `test_a_missing_metric_is_reported_and_is_not_a_pass` | 57 |
| `test_question_tolerance_is_one_question_of_the_type_count` | 57 |
| `test_cli_exits_non_zero_on_drift_or_missing` | 57 |
| `test_slice_40_baseline_manifest_lists_the_five_benchmarks` | 58 |
| `test_latency_harness_keeps_the_diagnostic_cells_and_percentiles` | 59 |
| `test_engine_identity_records_the_commit_and_the_native_digest` | 60 |

The drift cases are hand-written: one inside every tolerance, one outside
(absolute, relative and per-question), one missing metric.

## 3f. Unit G tests (`comp03-slice40` worktree)

Revised after the Unit G review (findings F1–F10). The file was rewritten
for the new rule, the single-factor runs, the shared evidence implementation
and the best mix. It replaces the 15 tests committed at `9d864e39e` and
`d1bc2c9f6`, which were RED and never implemented.

```bash
env PYTHONPATH=. .venv/bin/python -m pytest -q tests/experiments/test_comp03_evidence.py
```

After the re-review (APPROVE-WITH-FIXES, N1–N4 and v4c) the file has 26
tests, all RED before W4, on hand-built docs (three sessions, one gold session) and
hand-written policy tables:

| Test | AC |
| --- | --- |
| `test_caps_apply_per_round_then_total_in_rank_order` | 61 |
| `test_the_baseline_policy_matches_the_answer_harness_renderer` | 61 |
| `test_expansion_appends_remaining_rounds_of_hit_sessions_by_best_hit_rank` | 62 |
| `test_expansion_stops_at_the_total_cap` | 62 |
| `test_expansion_enumerates_blank_rounds_at_zero_chars` (F10) | 62 |
| `test_prompt_metrics_count_gold_sessions_and_answer_rounds` | 63 |
| `test_untruncated_and_char_fraction_metrics_see_a_cut_answer_round` (F1) | 63 |
| `test_policy_grid_is_the_stated_twelve_policies` | 63 |
| `test_rule_maximises_has_answer_and_rejects_policies_that_cut_answer_rounds` (F1; the lowest-chars candidate loses on untruncation) | 64 |
| `test_pooled_tolerance_is_three_counts_and_fts_tolerance_is_one` (F5) | 64 |
| `test_rule_also_holds_for_fathomdb_fts_alone_and_falls_back_when_the_pooled_winner_breaks_it` (Q-G1) | 64 |
| `test_rule_breaks_ties_on_fewer_chars_and_keeps_the_baseline_when_nothing_beats_it` | 64 |
| `test_renderer_shows_exactly_the_evaluator_rounds_with_expansion` (F2) | 65 |
| `test_evidence_record_digests_the_expanded_text_and_counts_it` (F2) | 65 |
| `test_chronological_rendering_orders_by_session_date_newest_last_and_keeps_dates` | 66 |
| `test_chronological_rendering_breaks_date_ties_by_session_then_round_position` (F8) | 66 |
| `test_chronological_rendering_selects_the_same_rounds_as_rank_order_under_the_cap` | 66 |
| `test_recency_sentence_is_added_only_when_enabled_and_v2_rendering_is_unchanged` | 66 |
| `test_recency_rule_adopts_unless_ku_plus_temporal_drops_or_overall_drops_by_more_than_one` | 67 |
| `test_recency_rule_boundaries` (F9) | 67 |
| `test_v3_v4a_v4b_v4c_configs_add_one_factor_each` (F3, F6, v4c) | 68 |
| `test_evidence_verdict_file_names_its_inputs` (F4, F7, F10, N1, N3) | 68 |
| `test_mix_config_follows_the_three_verdicts` (the best mix) | 69 |
| `test_mix_config_follows_the_arm_of_record` (N2) | 69 |
| `test_mix_config_needs_v4c_for_chronological_order_alone` (v4c) | 69 |
| `test_render_evidence_refuses_a_policy_together_with_caps` (N4) | 69 |

The rule tests do not hard-code the replay's winner (4000/30000/on); each
fixture's expected policy follows from the rule. The existing
`test_comp03_answer.py` tests are the byte-identity guard for both factors
off.

## 3g. Unit H tests

### 3g.1 Harness (`comp03-slice40` worktree), `tests/experiments/test_comp03_unit_h.py` (new)

```bash
env PYTHONPATH=. .venv/bin/python -m pytest -q tests/experiments/test_comp03_unit_h.py
```

All tests are RED before `comp03_unit_h.py` exists. They run on hand-built
question records and hand-written per-word tables; none reads the real
sweep.

The selection commands, run after the module is GREEN:

```bash
# H1 stop list: implementer, CPU, the 5b09afd44 wheel's Python environment
env PYTHONPATH=. <5b09afd44 venv>/bin/python -m experiments.comp03_unit_h stoplist \
    experiments/configs/comp-03/stoplist-selection.v1.json \
    --dataset <longmemeval_s_cleaned.json> \
    --nfcorpus-inputs ~/.cache/fathomdb-comp/nfcorpus-input \
    --attempt-root ~/.cache/fathomdb-comp/h1-stoplist-selection \
    --engine-commit 5b09afd44112251ee634e034edf0368bbdce684d

# H2 vector pool: orchestrator, cuda:0, a CUDA wheel built at 5b09afd44 (ruling Q-H4)
env FATHOMDB_EMBED_DEVICE=cuda:0 PYTHONPATH=. <CUDA 5b09afd44 venv>/bin/python -m experiments.comp03_unit_h pool \
    experiments/configs/comp-03/vector-pool-selection.v1.json \
    --dataset <longmemeval_s_cleaned.json> \
    --attempt-root ~/.cache/fathomdb-comp/h2-pool-selection \
    --engine-commit 5b09afd44112251ee634e034edf0368bbdce684d
```

Each command writes its attempt's `summary.json` (per-question IDs per
list or per `P`) and then its verdict file under `experiments/configs/comp-03/`.
Both resume per question, as the sweep does.

| Test | AC |
| --- | --- |
| `test_stoplist_selection_config_states_the_candidates_and_rule` (`engine_stopwords` equals the 63-word list verbatim; the emptiness rule is in the config text) | 70 |
| `test_stripping_words_equals_compiling_with_them_as_stopwords` (`hypothesis`: content tokens of the stripped query under the D3 list equal those of the raw query under D3 plus the words) | 71 |
| `test_a_word_that_empties_a_query_is_excluded` (one word alone; and two words that empty "can you help" only jointly, where the lower-`ΔnDCG` one is pruned) | 71 |
| `test_stoplist_rule_keeps_gaining_words_and_prunes_until_constraints_hold` | 72 |
| `test_stoplist_rule_boundaries` (nDCG exactly −0.002 passes, −0.0021 prunes; a type exactly one question down passes, two prunes; Recall-all@10 exactly +1 with CI lower bound exactly −1 question adopts; +1 with a lower bound below −1 question, or a point delta of 0, does not; owner ruling N1) | 72 |
| `test_stoplist_rule_is_not_adopted_when_nothing_gains` | 72 |
| `test_stoplist_verdict_names_its_inputs` | 73 |
| `test_vector_pool_selection_config_is_typed_and_states_the_rule` (and the pool command's retrieval is `comp03_answer.retrieve`, the sweep's function) | 77 |
| `test_pool_metrics_use_prefixes_of_the_limit_p_ranking` | 78 |
| `test_pool_rule_picks_the_largest_pool_that_keeps_recall` | 78 |
| `test_pool_rule_boundaries` (one question down on a type passes, two fails; the reference always passes) | 78 |
| `test_slice_40_baseline_manifest_lists_the_nine_benchmarks` (`test_baseline_check.py`, renamed from `…_six_benchmarks` under owner ruling Q-H5; new assertions: the three COMP-02 entries carry FathomDB query-ready, p50 and p95, violations and short results, with the § 3g.7 tolerances) | 58 |

The rule tests do not hard-code the real verdicts. Each fixture's expected
list or pool follows from the rule.

### 3g.2 Engine, H1: `fathomdb-query` test module (this worktree)

These tests are written after the stop-list verdict, since they pin its
words.

| Test | AC |
| --- | --- |
| `unit_h_words_are_in_the_stop_list_and_content_tokens_are_not` (the verdict's words in; `d`, `c`, `d3`, `ii`, `1`, `b12` out; the Unit D twelve still in; `STOPWORDS.len()` is 63 plus the verdict's count) | 74 |
| `each_unit_h_word_alone_takes_the_fallback` (each word upper-cased, as the D3 test does: the content path would emit the lowercased token and the fallback emits the raw one, so the test is RED before the change, review F6) | 74 |
| `unit_h_question_shapes_compile_to_content_terms` ("how many siblings do I have", "can you recommend some", reduced to whichever of their words the verdict keeps) | 74 |

`fts5_injection_safety.rs` gets the same words in its private copy (AC40-75).
That is the one edit to an existing test, by the precedent of ruling Q-D3.

### 3g.3 Engine, H2: `tests/slice40_vector_pool.rs` (new, `test-hooks`)

The tests use `Engine::open_with_embedder_for_test` and
`configure_vector_kind_for_test`, as `slice18_retrieval_result_limits.rs`
does.

The corpus has 40 rows:

- one row whose vector rank is 15 and text rank 1;
- rows that match the vector only;
- rows that match the text only.

Against today's engine, the limit-10 ranking is not the first ten of the
limit-40 ranking, so the prefix test is RED. The RED reason may be the
score bits alone: the vector-rank-15 row scores `3/31` at limit 10 and
`3/31 + 1/45` at limit 40, even where its position does not move. That is a
valid failure; do not change the corpus until the order flips (review
F14). The tests read the pool through `vector_fusion_pool_for_test()`.

| Test | AC |
| --- | --- |
| `hybrid_results_are_prefix_stable_up_to_the_vector_pool` (k ∈ {1, 5, 10} against k' ∈ {10, `VECTOR_FUSION_POOL`}: ids and score bits) | 79 |
| `vector_branch_hydrates_the_pool_below_it_and_the_limit_above_it` (explain `vector_hits` at limit 5, at `VECTOR_FUSION_POOL`, and at `VECTOR_FUSION_POOL + 1` when the pool is below 100) | 80 |
| `fused_score_formula_is_unchanged_with_the_pool` | 81 |
| `vector_stage_only_and_filtered_vector_search_keep_their_contracts` | 82 |

If the verdict is `P0` = 10:

- the prefix test runs with k' = 10 only;
- the corpus moves the disagreement to limit 5 against limit 10, so the
  test stays RED against today's engine.

### 3g.4 Engine, H3: `tests/slice40_cohort_prefilter.rs` (new, `test-hooks`)

The tests reuse the Defect C fixture shape: a `text` SEARCHABLE projection
with FTS, a `cohort` FILTERABLE projection, and three bm25 tie groups.

| Test | AC |
| --- | --- |
| `small_cohort_takes_the_cohort_prefilter_route` (route and plan witness) | 84 |
| `cohort_prefilter_results_equal_rank_stream_and_full_sort` (forced on, forced off, forced full sort; limits 1, 2, 3, 5, 8, 10, 12, 100) | 85 |
| `cohort_of_t_rows_prefilters_and_t_plus_one_keeps_the_rank_stream` (256 and 257 rows) | 86 |
| `cohort_prefilter_failure_falls_back_to_the_rank_stream` (route list exactly `projected_cohort_stream_fallback` then the Defect C route; forced off records no cohort route) | 87 |
| `multi_attribute_filter_prefilters_on_the_smallest_cohort` | 88 |
| `cohort_prefilter_keeps_lifecycle_and_other_filter_fields` (a superseded node with the attribute; a `kind` filter) | 89 |
| `prop_cohort_prefilter_matches_the_full_sort` (`proptest`, 32 cases, at most 60 rows, random cohort sizes, tie groups and limits) | 90 |
| `cohort_prefilter_timing_report` (`#[ignore]`, report-only: cohort route and Defect C route forced in turn at c01 and c10 on a store built from `FATHOMDB_TC5_INPUTS`; prints p50 and p95) | — |

Two tests pass today, because the cohort route does not exist and results
are already equal:

- `cohort_prefilter_results_equal_rank_stream_and_full_sort` and
  `prop_cohort_prefilter_matches_the_full_sort` are the regression net.
- They fail only if the forcing environment variable is ignored. To guard
  against that, each one asserts its route witness.

`slice40_projected_fts_stream.rs` (AC40-27 to AC40-31) is unchanged.
AC40-30 asserts results only and is unaffected. The cohort route for small
cohorts is pinned by `small_cohort_takes_the_cohort_prefilter_route`.

If the plan witness of `small_cohort_takes_the_cohort_prefilter_route`
fails on the bundled SQLite, the conjunct changes form as design § 2g.4
sets out, and the witness stays (review F9).

The report-only timing runs once before the code review:

```bash
env FATHOMDB_TC5_INPUTS=~/.cache/fathomdb-comp/tc5-10000 \
    cargo test -p fathomdb-engine --release --features test-hooks \
    --test slice40_cohort_prefilter -- --ignored cohort_prefilter_timing_report --nocapture
```

### 3g.5 Existing tests to watch at GREEN

None is edited to pass; any failure is analysed and reported. The review
read the H2 candidates (review F15): `slice18_retrieval_result_limits.rs`
asserts `results.len() == limit`, `exp_obs_explain.rs` asserts
`vector_hits > 0`, and the Python explain-parity test checks a type. None
pins `vector_hits == limit`, so the H2 list is likely to come back empty.

- **H2**, tests that fix hybrid order at a limit below the pool on a corpus
  with more vector rows than the limit:
  - `pr_g9_rrf_fusion.rs`;
  - `exp_obs_explain.rs`;
  - `pr_g12_recency.rs`;
  - `f9_importance_ranking.rs`;
  - `pr_g10_reranker*.rs`;
  - `ga2_vector_stage_seam.rs`;
  - `slice18_retrieval_result_limits.rs`;
  - the Python and TypeScript hybrid tests under `src/python/tests` and
    `src/ts`.
- **H1:** query-compiler pins with the selected words in the `fathomdb-query`
  test module, `slice6_tokenizer_experiment.rs`, `cursor_read_after_write.rs`
  (it reads the compiled SQL), and the cross-SDK text-only fixture (the
  oracle is computed at run time, so no change is expected).
- **H3:** `slice40_projected_fts_stream.rs` (above), `slice45_nested_source_projections.rs`,
  `slice35_eligibility_pretruncation.rs`, `slice30_dependency_closure.rs`,
  `slice80_search_view_paths.rs`.

### 3g.6 Expected changes at the re-runs

Each row is measured at the commit that changes it, against the reference
in the requirements' gate table: the final-head baseline for H1, the H1
commit for H2, the H2 commit for H3. The direction is the design's
expectation; the gate is in REQ40-H3, H6 and H9. COMP-02b runs twice: at the
H2 commit, which carries H1's change to the compiled query, and at the H3
commit. The hybrid sweep also runs twice: at the H1 commit (pool = limit)
and at the H2 commit.

| Benchmark | Metric | H1 | H2 | H3 | Expected |
| --- | --- | --- | --- | --- | --- |
| COMP-01 FTS | FathomDB nDCG@10, MRR@10 | small, by the verdict's NFCorpus row | — | — | within −0.002 and −0.005; the verdict gives the exact number |
| COMP-01 FTS | FathomDB p50, p95 | fewer OR terms | — | — | equal or lower |
| COMP-01 FTS | LanceDB | — | — | — | unchanged (same inputs, same harness) |
| comp01-hybrid | `fathomdb-hybrid` nDCG@10, MRR@10 | text branch | pool | — | unknown sign; reported |
| comp01-hybrid | `fathomdb-hybrid` p50 | — | up to `P0 − 10` more lookups | — | within +25% |
| COMP-02a 10k, 17k | FathomDB ready, p50, p95 | text compile only | — | — | within tolerance |
| COMP-02a | SurrealDB | — | — | — | unchanged |
| COMP-02b 10k | FathomDB c01 p50 | — | — | prefilter | lower, about 45% by the probe |
| COMP-02b 10k | FathomDB c10, c33 p50 | — | — | probe only | within +25% |
| COMP-02b 10k | violations, short results | — | — | identity | 0; short results unchanged (c01: 6 per rep, pre-existing) |
| tc5-10k-latency | `projected-c01` p50 | — | — | prefilter | ≥ 25% below 29.8 ms |
| tc5-10k-latency | other cells | fewer OR terms | — | probe only | within tolerance |
| COMP-03 sweep | `fathomdb-fts` per type | by the verdict | — | — | preference and multi-session up, temporal within one question; equals the simulation exactly |
| COMP-03 sweep | `fathomdb-hybrid` per type | text branch | pool | — | within one question per type at k = 5 and 10 |
| COMP-03 sweep | `lancedb-fts` | — | — | — | byte-identical IDs |
| COMP-03 sweep | `fathomdb-hybrid` warm p50, p95 | — | pool | — | within +25%, +35% |
| COMP-03 mix answer | correct of 60 | FTS retrieval | hybrid retrieval | — | within ±3 (noise); LanceDB within ±3 |

### 3g.7 Baseline-manifest update plan

1. **Before the re-runs.** The H1 gate reads the current manifest at the
   final head (`5b09afd44`, harness `5a1d0eb12`), unchanged. The H2 and
   H3 gates read the H1-commit and H2-commit attempts with
   `baseline_check.py` against a scratch copy of the manifest whose
   expected values are those attempts' values. The scratch copy is not
   committed.
2. **New metrics, added when the gates are written.** Adding a metric
   needs no test change:
   - `comp01-hybrid`: `fathomdb-hybrid` p50 and p95, needed by REQ40-H6
     (ii);
   - `comp03-retrieval-sweep`: `fathomdb-hybrid` session Recall-any@10
     per type, needed by REQ40-H6 (i).

   Their expected values come from the final-head attempts
   (`final-comp01-hybrid`, `final-comp03-sweep`), so the gate has a
   reference.
3. **After the re-runs and passed gates.** Every benchmark's expected
   value is replaced by the Unit H measurement, with provenance `final`,
   the Unit H engine commit and the new receipt. That includes
   `comp03-answer-mix`, which stays report-only and provisional.
   Tolerances do not change.
4. **After a failed gate.** If a gate fails and its commit is reverted,
   that benchmark keeps its pre-H values and the verdict says why.
5. **COMP-02a and COMP-02b** (owner ruling Q-H5, 2026-10-16, review F8).
   In the harness step, before the H1 gate, they become benchmarks
   `comp02a-10000`, `comp02a-17272` and `comp02b-10000`, with expected
   values from the `final-comp02a-*` and `final-comp02b-10000` receipts:
   - FathomDB query-ready, p50 and p95, at ±25% / ±25% / ±35%;
   - COMP-02b violations at abs 0, and short results at abs 0.

   `test_slice_40_baseline_manifest_lists_the_six_benchmarks` is renamed
   for nine ids under that ruling, and its new assertions are written
   RED first (§ 3g.1).

## 4. Existing tests touched

**Units E and F (phase 5).** Three capability-list assertions are
superseded by REQ40-E1, owner-directed scope (ruling Q-E1, 2026-10-16), and
updated in the registry commit `d709db1a8` from three to five operations:

- `tests/experiments/test_sdk_python_driver.py::test_capabilities_advertise_only_implemented_operations`;
- `tests/experiments/test_sdk_typescript_driver.py::test_capabilities_is_one_protocol_response`;
- `experiments/sdk_drivers/rust/tests/protocol.rs::capabilities_advertise_only_implemented_operations`.

No other existing test changed. In `test_comp03_evidence.py`, commit
`d1bc2c9f6` added one Unit G test for the Q-G1 ruling
(`test_rule_also_holds_for_fathomdb_fts_alone_and_falls_back_when_the_pooled_winner_breaks_it`),
before the Unit G hold. It is after the reviewer's pin `9d864e39e`.

**Unit D (phase 4).**

- `fathomdb-query`: `possessive_and_contraction_fragments_are_content_tokens`
  deleted in the D3 commit `5b09afd44`. It pinned the Defect B rule that
  "s" and "t" are content; REQ40-D5 supersedes it by owner ruling (Q-D1).
- `fts5_injection_safety.rs`: the private stop-list copy gains the twelve
  words in `5b09afd44` (Q-D3); fixture unchanged.
- Harness RED tests of this phase, before any production code:
  - `test_fts_projected_arm_stores_json_bodies_and_searches_text_only`
    removed and the sweep-config arm list cut to three, because Q-D2
    dropped the `fathomdb-fts-projected` variant (`34cacdf87`); the
    hybrid store test gained an assertion that the store holds the fts
    arm's plain bodies.
  - `test_v2_config_keeps_v1_inputs_and_widens_to_three_arms_at_k10` no
    longer requires v2's `metrics` list to equal v1's; it requires v1's
    names with `_at_5` read as `_at_10`, since v2 reports at k = 10
    (`c08a25691`). The first form would have pinned a stale description.

**Earlier phases.**

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

### 5.5 RED run for Unit D (phase 4, 2026-10-16)

D3, this worktree at `47c54b78d` plus the new tests:

```text
cargo test -p fathomdb-query
thread 'tests::each_unit_d_word_alone_takes_the_fallback' (3964612) panicked at src/rust/crates/fathomdb-query/src/lib.rs:167:13:
assertion `left == right` failed: "s" alone has no content token
  left: "\"s\""
 right: "\"S\""
thread 'tests::possessive_and_contraction_fragments_are_dropped' (3964617) panicked at src/rust/crates/fathomdb-query/src/lib.rs:177:9:
assertion `left == right` failed
  left: "\"cow\" OR \"s\" OR \"milk\""
 right: "\"cow\" OR \"milk\""
thread 'tests::short_content_tokens_survive_the_unit_d_stop_list' (3964621) panicked at src/rust/crates/fathomdb-query/src/lib.rs:195:9:
assertion `left == right` failed
  left: "\"my\" OR \"vitamin\" OR \"d\" OR \"level\""
 right: "\"vitamin\" OR \"d\" OR \"level\""
thread 'tests::short_function_words_are_dropped' (3964622) panicked at src/rust/crates/fathomdb-query/src/lib.rs:187:9:
assertion `left == right` failed
  left: "\"my\" OR \"dog\" OR \"likes\" OR \"me\" OR \"so\" OR \"take\" OR \"us\" OR \"up\" OR \"hill\" OR \"am\""
 right: "\"dog\" OR \"likes\" OR \"take\" OR \"hill\""
thread 'tests::unit_d_words_are_in_the_stop_list_and_content_letters_are_not' (3964626) panicked at src/rust/crates/fathomdb-query/src/lib.rs:154:13:
"s" must be a stopword
test result: FAILED. 15 passed; 5 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
```

Harness, `comp03-slice40` at `0e246ad31` plus the new tests:

```text
env PYTHONPATH=. .venv/bin/python -m pytest -q tests/experiments/test_comp03_retrieval.py tests/experiments/test_comp03_answer.py
SKIPPED [1] tests/experiments/test_comp03_answer.py:129: could not import 'lancedb': No module named 'lancedb'
SKIPPED [1] tests/experiments/test_comp03_answer.py:149: could not import 'lancedb': No module named 'lancedb'
27 failed, 9 passed, 2 skipped in 1.26s
```

The 9 passing are the pre-existing `test_comp03_answer.py` tests; the 2
skips are its pre-existing LanceDB tests. The 27 failures are every new
test, by cause:

```text
     17 E       ImportError: cannot import name 'comp03_retrieval' from 'experiments' (/home/coreyt/projects/fathomdb-worktrees/comp03-slice40/experiments/__init__.py)
      3 E       KeyError: 'fathomdb-hybrid'
      3 E       FileNotFoundError: [Errno 2] No such file or directory: '/home/coreyt/projects/fathomdb-worktrees/comp03-slice40/experiments/configs/comp-03/hermes-answer.v2.json'
      2 E           AttributeError: module 'experiments.comp03_answer' has no attribute 'arm_order'
      1 E       KeyError: 'fathomdb-fts-projected'
      1 E           AttributeError: module 'experiments.fathomdb_test_setup' has no attribute 'require_embed_device'
```

In `test_comp_fts_fts_arm_is_byte_unchanged_and_hybrid_arm_is_registered`
the source-hash and settings assertions pass; it fails at the missing
`fathomdb-hybrid` arm. `tests/experiments/test_comp_fts.py` is unchanged
and still passes (`7 passed, 5 skipped`).

### 5.6 GREEN run for Unit D (W4, 2026-10-16)

No test edited after the commits listed in § 4.

D3 at `5b09afd44` (CHANGELOG at `8d5d93cef`), from `src/rust`:

```text
cargo test -p fathomdb-query
test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
cargo test -p fathomdb-engine --test slice40_fts_ranking --test fts5_injection_safety
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
cargo test -p fathomdb-engine --features test-hooks --test slice40_projected_fts_stream
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.48s
cargo test -p fathomdb-engine
exit 0; 79 test binaries; 474 passed, 0 failed, 7 ignored
```

19 = the 15 tests before Unit D, minus the deleted one, plus the five D3
tests.

Harness at `8ef511554` (`comp03-slice40`), from the worktree root:

```text
env PYTHONPATH=. .venv/bin/python -m pytest -q tests/experiments/test_comp03_retrieval.py tests/experiments/test_comp03_answer.py
37 passed, 3 warnings in 36.27s
env PYTHONPATH=. .venv/bin/python -m pytest -q tests/experiments
FAILED tests/experiments/test_locomo_live_executor_review_remediation.py::test_full_grid_combiner_requires_same_release_config_and_exactly_52_unique_cells
FAILED tests/experiments/test_release_0825_slice71.py::test_checked_in_receipt_binds_manifest_fixture_and_raw_logs
FAILED tests/experiments/test_release_0825_slice71b.py::test_checked_in_attribution_manifest_is_strict_and_valid
FAILED tests/experiments/test_scale_02_scale_followup.py::test_config_freezes_two_hypotheses_and_three_points
4 failed, 811 passed, 23 skipped, 5 warnings in 46.90s
```

The 37 are 20 collected from `test_comp03_retrieval.py` (17 functions,
the parametrized reject test counting four) and 17 from
`test_comp03_answer.py` (11 pre-existing, the two LanceDB tests now
running, and 6 new). The 3 warnings are LanceDB's `create_fts_index`
deprecation in the unchanged `LanceArm`. The 4 failures in the whole tree
fail the same way with the Unit D changes stashed (`4 failed`); none
touches COMP code:

```text
E           experiments.locomo_live_executor.LiveExecutorError: PARENT cell receipt parent context exceeds the released bound
E           AssertionError: assert '73646fdac08d...04045c577e628' == 'd6bfeca0242d...493d5cd706b00'
E       FileNotFoundError: [Errno 2] No such file or directory: '/home/coreyt/projects/fathomdb-worktrees/comp03-slice40/data/performance-benchmarking/scale-02/slice71b/scale02-input.v1.jsonl'
E               experiments.scale_02_scale_followup.Scale02ScaleFollowupError: dependency digest drifted
```

A smoke of the `comp01-hybrid` cell on three documents
(`python -m experiments.comp_fts _cell --mode fts --arm fathomdb-hybrid`,
CPU) recorded `vector_rows` 3, a positive `drain_s` and `Engine.search`.

### 5.7 RED run for Units E, F, G (phase 5, 2026-10-16)

Harness at `e3dce3739` plus the three new files, from the `comp03-slice40`
worktree root:

```text
env PYTHONPATH=. .venv/bin/python -m pytest -q tests/experiments/test_sdk_parity_slice40.py
9 failed
      7 E       AttributeError: module 'experiments.sdk_benchmark' has no attribute 'build_parity_fixture'
      1 E       AttributeError: module 'experiments.sdk_benchmark' has no attribute 'compare_parity'
      1 E           AssertionError: engine.search_projected_text
                    assert 'excluded_with_reason' == 'measure'
env PYTHONPATH=. .venv/bin/python -m pytest -q tests/experiments/test_baseline_check.py
8 failed
      6 E       ImportError: cannot import name 'baseline_check' from 'experiments'
      1 E       ImportError: cannot import name 'comp02b_latency' from 'experiments'
      1 E       AttributeError: module 'experiments.comp_record' has no attribute 'engine_identity'
env PYTHONPATH=. .venv/bin/python -m pytest -q tests/experiments/test_comp03_evidence.py
14 failed
      4 E       AttributeError: module 'experiments.comp03_retrieval' has no attribute 'evidence_rounds'
      2 E       TypeError: render_evidence() got an unexpected keyword argument 'chronological'
      2 E       AttributeError: module 'experiments.comp03_retrieval' has no attribute 'select_evidence_policy'
      1 E       TypeError: answer_messages() got an unexpected keyword argument 'recency_instruction'
      1 E       FileNotFoundError: … experiments/configs/comp-03/hermes-answer.v3.json
      1 E       FileNotFoundError: … experiments/configs/comp-03/evidence-policy-verdict.v1.json
      1 E       AttributeError: module 'experiments.comp03_retrieval' has no attribute 'evidence_policy_grid'
      1 E       AttributeError: module 'experiments.comp03_retrieval' has no attribute 'evidence_metrics'
      1 E       AttributeError: module 'experiments.comp03_answer' has no attribute 'evaluate_recency_rule'
```

31 RED, 0 GREEN, 0 skipped. The harness `.venv` opens the default embedder
on CPU, so the dense tests ran. Ruff is clean on the three files. The
`FileNotFoundError` lines are shortened with "…"; the rest is verbatim.

### 5.8 GREEN run for Units E and F (phase 5, 2026-10-16)

Harness at `5e6ff1a34` (`comp03-slice40`), from the worktree root. Node 25
for the TypeScript driver tests comes from
`FATHOMDB_BENCH_NODE=$HOME/.nvm/versions/node/v25.9.0/bin/node`.

```text
env PYTHONPATH=. .venv/bin/python -m pytest -q tests/experiments/test_sdk_parity_slice40.py
9 passed
env PYTHONPATH=. .venv/bin/python -m pytest -q tests/experiments/test_baseline_check.py
8 passed
env PYTHONPATH=. .venv/bin/python -m pytest -q tests/experiments/test_comp03_retrieval.py tests/experiments/test_comp03_answer.py tests/experiments/test_comp_fts.py tests/experiments/test_comp01_nfcorpus.py tests/experiments/test_baseline_check.py
68 passed, 3 skipped
FATHOMDB_BENCH_NODE=… env PYTHONPATH=. .venv/bin/python -m pytest -q tests/experiments/test_sdk_typescript_driver.py
11 passed, 1 skipped   (the real-SDK test skips: no TS native build in the harness tree)
cd experiments/sdk_drivers/rust && cargo test --release
protocol 5 passed; real_database 1 passed
FATHOMDB_BENCH_NODE=… env PYTHONPATH=. .venv/bin/python -m pytest -q tests/experiments
19 failed, 849 passed, 12 skipped
.venv/bin/ruff check src/python experiments tests/experiments
All checks passed!
```

The 19 failures in `tests/experiments` are:

- the 15 held Unit G tests in `test_comp03_evidence.py`, RED by design;
- the same 4 pre-existing failures as in § 6.1, unchanged by name:
  - `test_locomo_live_executor_review_remediation.py::test_full_grid_combiner_requires_same_release_config_and_exactly_52_unique_cells`
  - `test_release_0825_slice71.py::test_checked_in_receipt_binds_manifest_fixture_and_raw_logs`
  - `test_release_0825_slice71b.py::test_checked_in_attribution_manifest_is_strict_and_valid`
  - `test_scale_02_scale_followup.py::test_config_freezes_two_hypotheses_and_three_points`

**Cross-SDK parity, end to end on CPU (design § 2d.2).**

Artifacts were built in the session scratchpad (`sdk-parity/`) from this
worktree at `ad2773e6a`, a `git archive` export and `build_against.py`:

| Artifact | SHA-256 |
| --- | --- |
| Python wheel `fathomdb-0.8.27-cp310-abi3-linux_x86_64.whl` | `7f2cf1a108eb6a4768280026a8b43ab72c9a024f31848eeb9a8b70cf50d0ff52` |
| Python native `_fathomdb.abi3.so` | `6ef917cb42f7744a172803ec945d21b833cfad42d701c83e6d34f0e865bf16db` |
| TypeScript native `fathomdb.linux-x64-gnu.node` | `b7691e8973d78d647566b72eea463723e6fa79c7a256b37f0674837b289faec7` |
| Rust driver (`default-embedder`) | `8d1fe37b099bd9766569bedf6f0515537942c3103ed910fa621e74fb4b5bfab0` |
| CLI `fathomdb` (`default-embedder`) | `74aebb9892088267…` (full digest in each arm manifest) |

Every SDK reports version `0.8.27`. The arm manifests record
`engine_revision` `ad2773e6a8293c190e028e04c81b5da291ffaab4`.

Three suites ran, three repetitions each, each repetition on a fresh
database. Each SDK's ordered `(id, score, branch)` hits were compared
bit-for-bit with the in-process Python oracle on the same file; the oracle's
native digest equals the Python arm's.

| Suite / case | Hits per repetition | Python | TypeScript | Rust |
| --- | ---: | --- | --- | --- |
| `text-only-v1` / `text-only` (10 queries, limit 10) | 29 (text) | equal ×3 | equal ×3 | equal ×3 |
| `projected-v1` / `projected-unfiltered` (limit 5, tie across the limit) | 10 (text) | equal ×3 | equal ×3 | equal ×3 |
| `projected-v1` / `projected-cohort-c1` | 10 (text) | equal ×3 | equal ×3 | equal ×3 |
| `dense-v1` / `dense-hybrid` (7 queries, limit 10) | 70 (vector representative; q06 and q07 fuse text and vector) | equal ×3 | equal ×3 | equal ×3 |

Every cell was `complete` with oracle status `verified`. The first run
blocked the TypeScript cells with `SDK public wrapper digest drift`: the
pinned-artifact check did not pass `package_root`. Fixed in `5e6ff1a34`.

### 5.9 RED run for the revised Unit G (2026-10-16)

Harness at `5e6ff1a34` plus the rewritten `test_comp03_evidence.py`:

```text
env PYTHONPATH=. .venv/bin/python -m pytest -q tests/experiments/test_comp03_evidence.py
23 failed
      6 E       AttributeError: module 'experiments.comp03_retrieval' has no attribute 'evidence_rounds'
      4 E       AttributeError: module 'experiments.comp03_retrieval' has no attribute 'select_evidence_policy'
      3 E       TypeError: render_evidence() got an unexpected keyword argument …
      2 E       AttributeError: module 'experiments.comp03_retrieval' has no attribute 'evidence_metrics'
      2 E       AttributeError: module 'experiments.comp03_answer' has no attribute 'evaluate_recency_rule'
      1 E       TypeError: answer_messages() got an unexpected keyword argument …
      1 E       FileNotFoundError: … experiments/configs/comp-03/hermes-answer.v3.json
      1 E       FileNotFoundError: … experiments/configs/comp-03/evidence-policy-verdict.v1.json
      1 E       AttributeError: module 'experiments.comp03_retrieval' has no attribute 'evidence_policy_grid'
      1 E       AttributeError: module 'experiments.comp03_answer' has no attribute 'evidence_record'
      1 E       AttributeError: module 'experiments.comp03_answer' has no attribute 'build_mix_config'
```

Lines shortened with "…" lose only the keyword name and path tail. No
Unit G implementation code exists.

### 5.10 GREEN run for Unit G (W4, 2026-10-16)

Harness at `1419aae4f` (`comp03-slice40`). The tests were committed failing
at `3b812cce4`, 26 RED, and no test was edited after that.

```text
env PYTHONPATH=. .venv/bin/python -m pytest -q tests/experiments/test_comp03_evidence.py tests/experiments/test_comp03_answer.py tests/experiments/test_comp03_retrieval.py tests/experiments/test_comp_fts.py tests/experiments/test_baseline_check.py tests/experiments/test_comp01_nfcorpus.py
97 passed, 6 warnings
FATHOMDB_BENCH_NODE=… env PYTHONPATH=. .venv/bin/python -m pytest -q tests/experiments
4 failed, 878 passed, 9 skipped
.venv/bin/ruff check src/python experiments tests/experiments
All checks passed!
```

The 4 failures are the pre-existing four named in § 5.8, and nothing else.
The skip count fell from 12 to 9 because the orchestrator built the SurrealDB
driver in the harness tree.

**Evidence-policy verdict.**

- Command:

  ```bash
  python -m experiments.comp03_retrieval evidence \
    --attempt-root ~/.cache/fathomdb-comp/comp03-sweep-d3 \
    --dataset <longmemeval_s_cleaned.json> \
    --answer-config experiments/configs/comp-03/hermes-answer.v2.json \
    --out experiments/configs/comp-03/evidence-policy-verdict.v1.json
  ```

  It ran in 6 s. Verdict file SHA-256:
  `6f9fc2dc32c2796bbee7400788a22280e6b55ce70fcbf2781546b907164a2d2c`.
- Question counts, derived from the data: 470 scored, 414 selection,
  60 answer IDs of which 56 are scored.
- Selection on the 414: **4000/30000/on**, the same as the reviewer's replay.
- Excluded by the untruncation constraint: every 2000- and 1000-character
  policy, for every type, both pooled and on `fathomdb-fts` alone.

Table: has-answer in prompt / untruncated, pooled or per arm. The "470
pooled" column also gives any / all and characters, and equals the
reviewer's replay wherever it lists a row.

| Policy | 470 pooled any / all / ans / full / chars | selection pooled | selection fts | selection hybrid | selection LanceDB | answer set pooled |
| --- | --- | --- | --- | --- | --- | --- |
| 4000/20000/off (current) | 1383 / 1225 / 1321 / 1302 / 27.0M | 1169 / 1153 | 387 / 381 | 395 / 390 | 387 / 382 | 152 / 149 |
| 4000/20000/on | 1383 / 1225 / 1325 / 1304 / 28.1M | 1173 / 1155 | 387 / 381 | 397 / 391 | 389 / 383 | 152 / 149 |
| 4000/30000/off | 1388 / 1248 / 1334 / 1328 / 31.3M | 1180 / 1177 | 392 / 391 | 397 / 396 | 391 / 390 | 154 / 151 |
| **4000/30000/on** | 1388 / 1248 / 1344 / 1338 / 41.8M | 1190 / 1187 | 394 / 393 | 401 / 400 | 395 / 394 | 154 / 151 |
| 2000/20000/off | 1388 / 1248 / 1334 / 554 / 24.7M | 1180 / 505 | 392 / 167 | 397 / 172 | 391 / 166 | 154 / 49 |
| 2000/20000/on | 1388 / 1248 / 1344 / 564 / 28.1M | 1190 / 515 | 394 / 170 | 401 / 174 | 395 / 171 | 154 / 49 |
| 2000/30000/off | 1388 / 1248 / 1334 / 554 / 24.7M | 1180 / 505 | 392 / 167 | 397 / 172 | 391 / 166 | 154 / 49 |
| 2000/30000/on | 1388 / 1248 / 1353 / 572 / 41.3M | 1198 / 523 | 398 / 173 | 404 / 176 | 396 / 174 | 155 / 49 |
| 1000/20000/off | 1388 / 1248 / 1334 / 118 / 13.7M | 1180 / 106 | 392 / 35 | 397 / 35 | 391 / 36 | 154 / 12 |
| 1000/20000/on | 1388 / 1248 / 1358 / 118 / 27.1M | 1203 / 106 | 401 / 35 | 404 / 35 | 398 / 36 | 155 / 12 |
| 1000/30000/off | 1388 / 1248 / 1334 / 118 / 13.7M | 1180 / 106 | 392 / 35 | 397 / 35 | 391 / 36 | 154 / 12 |
| 1000/30000/on | 1388 / 1248 / 1369 / 118 / 36.2M | 1211 / 106 | 403 / 35 | 407 / 35 | 401 / 36 | 158 / 12 |

**Answer-set expectation** (N3): the chosen policy minus the current one on
the 56 scored answer-set questions.

- Pooled: has-answer +2, untruncated +2, gold sessions any and all +4 each,
  788 more rounds, +1.71M characters.
- Per arm, has-answer / untruncated: `fathomdb-fts` 0 / 0,
  `fathomdb-hybrid` +1 / +1, `lancedb-fts` +1 / +1.
- So v3 vs v2 is expected to be near-null on `fathomdb-fts`. The paid v3
  result is read against this.

**Configs.** `hermes-answer.v3.json`, `v4a`, `v4b` and `v4c` are committed
with the verdict's policy, bound by its SHA-256. A stub-client smoke run of
v3 and v4b through `comp03_answer.run` recorded `rounds_in_prompt` and
`evidence_round_chars` in the answer rows.

**Not written yet:** `hermes-answer.mix.json`. It needs the v4a and v4b
verdicts; the commands are `comp03_answer recency-verdict` and
`comp03_answer mix`.

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

### 6.3 Gates for Unit D (phase 4)

| Gate | Result |
| --- | --- |
| W-slice `./scripts/agent-lint.sh` (`8d5d93cef`) | exit 0 (`lint-rust-tegra-pool` skipped: needs aarch64 Linux and CUDA) |
| W-slice `./scripts/agent-lint-md.sh` | exit 0 |
| W-slice `./scripts/agent-verify.sh` (clean tree, `e373aeca6`, worktree `.venv`) | exit 1: `registered=202 ran=202 passed=200 failed=2 skipped=0 excluded=0`; `FAILED SUITES: test-check-board-currency test-commission-manifest`; `test-rust rc=0`, `test-python rc=0`, `test-python-native-receipt rc=0` |
| W-harness `./scripts/agent-lint.sh` (`8ef511554`) | exit 1 at its first step, `FAIL measurement-classification: classification_missing: comp-03-hermes-answer-20261016T1239Z-2e410a4f`. That receipt was committed at `0e246ad31`, before Unit D, and lacks a measurement classification; the script stops there, so its ruff step did not run |
| W-harness ruff, run directly: `.venv/bin/ruff check src/python experiments tests/experiments` | `All checks passed!` |

The two W-slice failures are the release-state suites of § 6.1 and § 6.2,
unchanged:

```text
FAIL  real-repository current-board regression: rc=1 out=STALE  0.8.28 Slice 31: dev/plans/release-state-0.8.28.json records landing SHA 1ceefddd894c041b0a8622856dc2677be3e49c88, but the hand-written ladder row at dev/plans/runs/STATUS-0.8.28.md:66 does not cite it (TC-133).
FAIL  arm 9c (real sweep): rc=1 out=FAIL commission-manifest: `dev/plans/release-state-0.8.28.json` declares end-of-ladder but its ladder is not fully landed (non-LANDED=[30, 31]; absent-from-landed=none).
```

### 6.4 Gates for Units E and F (phase 5)

| Gate | Result |
| --- | --- |
| W-slice `./scripts/agent-lint-docs.sh` | exit 0 |
| W-slice `./scripts/agent-lint-md.sh` | exit 0 |
| W-harness `./scripts/agent-lint-md.sh` | exit 2: structure passes; links fail only on the two pre-existing links to the absent, gitignored `data/corpus-data/raw/beir/nfcorpus` (in `plans/2026-10-13-fathomdb-0.8.27-performance-testing.md` and `experiments/research/2026-10-13-embedded-competitor-benchmark-survey.md`), as in phase 4 |
| W-harness ruff (`src/python experiments tests/experiments`) | `All checks passed!` |

No full `agent-verify.sh`: W-slice changed only docs this phase.

### 6.5 Gates for Unit G (phase 5)

| Gate | Result |
| --- | --- |
| W-harness ruff | `All checks passed!` |
| W-harness `./scripts/agent-lint-md.sh` | exit 1 at structure. The only findings are 4 in `experiments/comp_drivers/surrealdb/target/release/build/rust_decimal-…/out/README-lib.md`, a build output of the orchestrator's SurrealDB driver build. Its nested `target/` is not covered by the lint ignore list (which has `target/**` at the root only). Left in place as instructed. The link check did not run |
| W-slice `./scripts/agent-lint-md.sh` | exit 0 |

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

## 9. Open questions for R1 (phase 4)

- **Q-D1.** `possessive_and_contraction_fragments_are_content_tokens`
  contradicts D3 (§ 3c). Delete it in the D3 commit?
- **Q-D2.** The projection worker embeds the full node body, not the
  projected attribute (design § 2c.2), and a plain-body store with any
  `vector=True` searchable projection gets one vector row per document.
  Keep the JSON-body hybrid shape as directed, add a plain-body hybrid
  variant to the sweep as a pure vector-arm ablation, or switch the hybrid
  arm to the plain-body shape?
- **Q-D3.** Update the AC-038 reference model's private stop-list copy in
  the D3 commit (fixture unchanged)?
- **Q-D4.** `lancedb` is not installed in the `comp03-slice40` `.venv`.
  Install `lancedb==0.40.0` there for the real sweep, or run the sweep
  from a venv that has it?
- **Q-D5.** The NFCorpus informational row runs as a new campaign
  `comp01-hybrid` (`fathomdb` vs `fathomdb-hybrid`), not inside the
  existing `comp01` roster, which is fixed at two arms. Acceptable?

## 10. Open questions for R1 (phase 5)

- **Q-E1.** The Python, TypeScript and Rust driver suites each pin the
  capability set to today's three operations. Implementing
  `engine.search_projected_text` and `engine.search` changes it. Update the
  three assertions to the five operations at W4?
- **Q-E2.** No SDK exposes a build commit. Record the declared commit plus
  artifact digests (designed), or add a product build-info accessor in all
  three SDKs in this slice?
- **Q-E3.** The TypeScript wrapper digest is computed over the harness
  tree's `src/ts`. Add a `package_root` so the arm can point at the
  slice-head package (designed)?
- **Q-G1.** Apply the evidence-policy rule to counts pooled across the
  three arms (designed, one policy serves v3), or to `fathomdb-fts` alone?
- **Q-G2.** The evaluator runs on the 470 scored sweep questions, not only
  the 60 answer questions. Keep?
- **Q-F1.** Fill the manifest's expected values from the Unit D
  measurements, marked `provisional` until your final-head re-measurement
  (designed), or leave them empty?

## 11. Unit H review rulings (APPROVE-WITH-FIXES, 2026-10-16)

Findings F1 to F15 are applied in these docs. Rulings on the open
questions:

- **Q-H1.** Keep per-word `Δall ≥ 1`; no zero-gain words. Clause (c)
  gains a bootstrap CI lower bound (F4). Owner ruling N1 (2026-10-16)
  sets it: point delta ≥ +1 question and CI lower bound ≥ −1 question.
- **Q-H2.** Scope accepted. The docs state only what AC40-79 tests.
- **Q-H3.** `T` = 256 is fixed, with no retuning. One report-only
  in-engine timing at c10 runs before the gate (F12).
- **Q-H4.** The orchestrator runs the pool selection on cuda:0, with a
  CUDA wheel it builds at `5b09afd44`. The command is in § 3g.1.
- **Q-H5.** Owner ruling, 2026-10-16: COMP-02a and COMP-02b join the
  manifest, six ids to nine (§ 3g.7, item 5; three ids, "eight" was a
  miscount). The manifest test may change from six ids to nine; its new assertions are written RED first.
- **Q-H6.** Owner ruling, 2026-10-16: if hybrid fails its recall clause at
  the H1 commit (REQ40-H3 (vi)), H1 is kept, and the hybrid regression is
  reported for follow-up. It is not reverted.
- **Q-H7.** Moot (F1): AC40-30 asserts results only.

The review's F5 named a 72-word list. The engine's list at `5b09afd44`
has 63 words (51 plus the twelve of REQ40-D5), counted from `STOPWORDS`;
the docs use 63. The review's F11 self-check `ids[P][:k] ==
ids[100][:k]` does not hold on today's engine, because pool = limit is
the defect. It is replaced by the in-engine prefix check REQ40-H6 (iv).

Re-review at `39d1f67dc`: APPROVE. N1 (owner ruling, 2026-10-16): clause
(c) is *point delta ≥ +1 question AND paired-bootstrap CI lower bound ≥ −1
question (bootstrap_ci, seed 404, 10,000 resamples) on Recall-all@10 over
the selection set*. N2: REQ40-H6 (ii) compares `comp01-hybrid` to the
final-head baseline. N3: the extra limit-`P0` hybrid pass is budgeted in
design § 2g.1 step 6.
