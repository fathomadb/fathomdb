---
title: FathomDB 0.8.28 Slice 41 — test plan
status: phase 1 landed (§ 7); phase 2 tests planned (§ 8), none written
target_release: 0.8.28
observed_on: 2026-10-16
---

# Slice 41 test plan

The tests for each AC in [requirements.md](requirements.md) § 4, and the
measurements. Every expected output in the in-repo tests, AC41-12
included, is written by hand from REQ41-P2 to P9 and checked by hand.
status.md records that they were hand-derived. No expected output is
captured from code, neither the code under test nor the harness (R0 F4).
The only code-to-data comparison is AC41-27. It checks `pack_context`
against the `evidence_sha256` values already recorded in the
owner-accepted comp03-run-mix records.

## 1. Commands

```bash
cargo test -p fathomdb-engine --test slice41_context_pack            # no embedder needed
cargo test -p fathomdb-engine --features default-embedder --test slice41_context_pack   # adds the hybrid case of AC41-18
cargo test -p fathomdb-sdk
cargo test -p fathomdb-embedder --features default-embedder --test slice41_marker_embedding
pytest src/python/tests/test_context_pack_parity.py
(cd src/ts && npm test)   # includes tests/context-pack.test.ts
python3 scripts/check-sdk-surface-parity.py
./scripts/agent-lint-md.sh && ./scripts/agent-lint-docs.sh
./scripts/agent-verify.sh        # once, on the final clean tree
```

The Python and TypeScript tests run against a wheel and addon built from
the slice commit into this worktree's own `.venv` and `node_modules`. Use
a built wheel, not `pip install -e` or `maturin develop`. The Slice 40
Unit E build recipe applies.

## 2. Rust: `fathomdb-engine/tests/slice41_context_pack.rs` (new)

| Test | AC |
| --- | --- |
| `top_tier_full_rest_compact_keeps_user_text` | 01 |
| `compact_allowance_is_shared_across_non_kept_parts_in_order` | 02 |
| `empty_non_kept_part_is_dropped_but_empty_kept_and_full_parts_stay` | 02 |
| `zero_full_items_renders_every_item_compact` | 02 |
| `tiers_follow_selection_index_into_expansion` | 03 |
| `budget_cuts_last_item_and_omits_the_rest` (header present, excluded from `chars_used`) | 04 |
| `exactly_spent_budget_omits_following_whitespace_item` | 04 |
| `item_chars_caps_full_tier_and_sets_truncated` | 05 |
| `expansion_orders_groups_by_first_hit_and_members_by_position` | 06 |
| `chronological_order_sorts_without_changing_cuts` (missing keys sort last) | 07 |
| `whitespace_only_item_is_omitted_and_keeps_its_slot` | 08 |
| `provenance_has_one_entry_per_input_in_input_order` | 09 |
| `cuts_count_scalar_values_on_astral_and_combining_text` | 10 |
| `invalid_inputs_raise_invalid_argument_naming_the_field` (one case per REQ41-P8 rule; asserts the field name as a message substring) | 11 |
| `large_full_items_matches_hand_derived_slice40_text` | 12 |
| `prop_chars_used_within_budget_and_text_chars_formula` (proptest) | 13 |
| `prop_raising_budget_never_removes_rendered_text` (proptest) | 14 |
| `prop_deterministic_and_context_order_independent` (proptest; distinct `(group, position)` per generated case) | 15 |
| `prop_round_body_round_trips_and_strips_to_plain` (proptest) | 16 |
| `encode_refuses_marker_characters_in_text`, `decode_refuses_malformed_tagged_body` | 17 |
| `tagged_and_plain_corpora_rank_identically` (real engine, tempdir) | 18 |
| `tagged_and_plain_corpora_fuse_identically` (`#[cfg(feature = "default-embedder")]`; run after AC41-19 passes) | 18 |
| `context_item_from_hit_decodes_tagged_and_keeps_untagged` | 20 |

The property body of AC41-13 is
`len(text) == text_chars == Σ chars + Σ (len(header) + 1 for rendered
items with a header) + 2 · (rendered − 1)`, with `text_chars == 0` when
nothing is rendered.

The AC41-12 fixture (in `context-pack-fixtures.json`) has about 6 rounds
in 2 sessions:

- one blank round;
- one round over 4,000 characters;
- one round with an empty assistant turn;
- a budget that cuts the last round;
- headers of the form `[Session date: …]`, chronological order.

Its expected text is written by hand from REQ41-P2 to P9, then checked by
hand against a reading of `render_evidence`.

AC41-18 builds two databases from one 200-row fixture: plain bodies and
tagged bodies. The fixture includes:

- tagged rows with duplicate plain twins;
- multi-part rounds with a query term as the very first token of a part,
  so the marker is directly before the token;
- empty parts.

It runs 40 queries, including 1-2 character tokens, stop words,
possessives and a query that matches only assistant text. It asserts equal
ids, equal order and `f64::to_bits` equal scores from `search_text_only`
at limits 10 and 100. The embedder-gated case runs hybrid `search` on both
databases and asserts equal ids, order and fused-score bits.

AC41-19 is `fathomdb-embedder/tests/slice41_marker_embedding.rs`
(the default embedder on CPU). It
embeds 20 tagged bodies and their plain twins and compares every `f32`
bit. At least one body is over the 512-token window, so truncation happens
after the markers are removed. It is skipped, with a printed notice, only when the model is not
cached. In that case the gate run must have the model cached.

## 3. Parity (AC41-21, AC41-22)

- One fixture file, `src/conformance/context-pack-fixtures.json`, with
  about 20 cases: inputs, policy and the hand-written expected
  `ContextPack` as JSON. It covers every REQ41-P rule, the R0 F6 empty-part
  case and three non-ASCII cases (CJK, emoji with ZWJ, combining marks).
- The Rust test loads it with `include_str!`; the Python and TypeScript
  tests load it by path from the repository root. Each compares `text`
  bytes, `chars_used`, `text_chars` and every provenance field to the
  expected value. The TypeScript test also asserts one astral-plane case
  where `chars` differs from `string.length` (AC41-10). Byte equality across the three SDKs follows from all
  three matching the expected bytes.
- `context_item_from_hit` parity uses a small real database: the same
  query through each SDK, then `pack_context`, then a compare.
- AC41-22: `check-sdk-surface-parity.py` and the operation-parity oracle
  `test_sdk_surface_parity_oracle.py` pass with the four new operations in
  the parity map and the allowlist. The allowlist entry
  `_slice41_context_pack_signature` is written at W4 and cites the owner
  approval of 2026-10-16.

## 4. Measurements (orchestrator runs them; harness branch)

### 4.0 AC41-27 differential ($0, no Unit H wait)

For each of the 60 answer questions, build `caller` items from the
dataset (design § 4). Pack the recorded comp03-run-mix top-10 ids with
the mix policy: `full_items = 10**6`, 4000/30000, expansion on,
chronological. Compare the SHA-256 of `text` with the recorded
`evidence_sha256`. Every one must match, on every arm whose records exist:
FTS, hybrid and LanceDB alike. `caller` items come from the dataset, not
the store, so the check is store-independent.
A mismatch is a defect in `pack_context` or in item building, and blocks
M1.

### 4.1 M1 offline coverage ($0)

- **Data:** the scored LongMemEval-S questions, FTS and hybrid arms (GPU
  embedder on a 3090 for hybrid). Two sets, both counted from the data:
  - the selection set: scored questions minus the scored answer-set
    questions (414 at Slice 40);
  - the answer set: the 60 M3 questions (56 scored), reported separately
    with its expected offline delta, as REQ40-G2 does. Retrieval runs at the Unit H head,
  at each config's own k. There is no slicing of a k = 100 list, because
  hybrid is not prefix-stable.
- **Renderer:** `pack_context` in `caller` item mode for every row below.
  The control row first passes AC41-27.

Grid:

| Policy | `full_items` | `compact_chars` | k | Prototype FTS weighted allfull |
| --- | ---: | ---: | ---: | ---: |
| Slice 40 mix (control) | 10^6 | n/a | 10 | .820 |
| T3/A300/k30 (the SDK default) | 3 | 300 | 30 | .921 |
| T3/A0/k30 | 3 | 0 | 30 | not run |
| T5/A300/k50 | 5 | 300 | 50 | .909 |
| T5/A0/k50 | 5 | 0 | 50 | .958 |

The last column holds prototype figures from the report (470 questions,
engine `5b09afd44`, the Python prototype). They are a guide, not M1
results.

All rows use `budget_chars = 30000`, `item_chars = 4000`, expansion on and
`keep_roles = ("user",)`.

- **Metrics per type and weighted:** gold-all, hasA_full, hasA_allfull,
  rounds shown. Type weights are those of the report: MS .266, TR .266, KU
  .156, SSU .14, SSA .112, SSP .06.
- **Rule, fixed before the run.** It is applied to the selection set only,
  and it picks the harness policy, not the SDK defaults (R0 F7):
  1. Keep the policies that pass REQ41-M2 on both arms.
  2. Of those, adopt the one with the highest mean weighted allfull over
     the two arms. Break ties with the smaller k, then the larger
     `compact_chars`.
  3. Adopt only if its weighted allfull beats the control by at least .05
     on both arms.
  4. If none qualifies, M3 uses T3/A300 (the SDK default) and the result
     is reported to the owner.
- The pre-Unit H preview at engine `5b09afd44` may be recorded, labelled
  "preview", and decides nothing. It runs through `pack_context`, not the
  prototype, so it also smoke-tests the harness `pack` mode.

### 4.2 Retrieval identity for `hits` mode ($0)

Tagged bodies are built from per-turn parts in original order, so they
strip to exactly the plain body (REQ41-B2, design § 4). Rebuild the
FathomDB stores with tagged bodies and compare top-30 ids against plain
stores on all 500 questions:

- FTS: always.
- Hybrid: only if AC41-19 passed.

Any difference blocks `hits` mode, and M3 then uses `caller` mode.

### 4.3 M3 paid answer run (one run)

- **Arms:** FathomDB FTS and FathomDB hybrid. Each runs the Slice 40 mix
  (k = 10) and the adopted policy at its k, at the Unit H head.
- **Control reuse (R0 F13):** Slice 40 re-runs the mix answer run after
  Unit H. If that run used the same reader, judge, seeds and questions, it
  is the control, and M3 runs only the two pack arm-configs. Otherwise all
  four arm-configs run in one run.
- **LanceDB:** owner ruling 2026-10-16 (R0 Q1). Not re-run; it stays on
  the Slice 40 renderer, and every table names each arm's renderer.
- **Questions:** the existing 60-question set. Reader, judge and seeds are
  as in the Slice 40 mix run.
- **Reported:** accuracy per arm-config and per type, paired flips (pack
  vs mix), Wilson intervals, and a reweighted-500 estimate (per-type
  accuracy times the type weights).
- **Reading:** the noise floor is about 3 of 60 flips per arm and the MDE
  about 6 of 60. The expected gain on 60 is +1 to +2, below the MDE. The
  run checks for no regression: a net loss above 3 of 60 on either arm is
  reported to the owner as a blocker. The M1 coverage gain is the evidence
  for adoption.
- **Budget:** priced runs need checkpoint, verified `--resume` and backoff
  before spend. The harness already has these from Slice 40.

### 4.4 Optional probe: session-diverse order ($0)

Reorder the k = 50 hits so that the best round of each session comes
first, then pack with the adopted policy. Record allfull next to M1. This
is input for a later slice only and changes nothing here.

### 4.5 COMP-01 and COMP-02 (AC41-26)

Re-run COMP-01 (13 checks) and COMP-02a at 10k once at the slice head, and
record them next to the Slice 40 final-head values. Neither benchmark
calls the new code, so any difference is noise or a defect elsewhere.

## 5. Unit H dependency

| Work | Waits for Unit H head? |
| --- | --- |
| Rust core, bindings, parity, docs (§ 2, § 3) | no |
| AC41-18 and AC41-19 | no |
| AC41-27 differential (§ 4.0) | no: uses the recorded Slice 40 ids |
| M1 preview | no |
| M1 adoption numbers (§ 4.1) | yes: the stop list moves FTS ranks; the vector pool moves hybrid at k = 30 and 50 |
| Retrieval identity (§ 4.2) | yes: compare at the head that M3 uses |
| M3 paid run (§ 4.3) | yes |
| COMP-01 and COMP-02 re-run (§ 4.5) | yes: run at the final slice head |

## 6. R0 rulings (orchestrator, 2026-10-16)

- **Q2 (F7):** decoupled. The SDK defaults are fixed by product judgement
  (top 3 full, 300-character excerpt). The § 4.1 rule picks the harness
  policy, which is passed explicitly.
- **Q3:** plain types.
- **Q6:** headers not counted; `ContextPack.text_chars` added.
- **F1:** no `field_path`; the message names the field.
- **F2:** the allowlist delta needs the owner's approval (given, see
  below); the entry is written at W4.
- **F3:** selection on the non-answer set; the answer set is reported
  separately.
- **F4:** hand-written AC41-12 fixture plus the AC41-27 differential
  check; bracketed header.
- **F13:** reuse Slice 40's post-Unit H mix run as the M3 control when the
  reader, judge and seeds match.

Owner rulings (2026-10-16):

- **Allowlist:** approved. The four context-pack operations are added to
  the governed-surface allowlist with the R0 adjustments. The entry is
  written at W4 and cites this ruling.
- **Q4, Q5:** the tagged body format ships opt-in with its hazards
  documented, and it gets a decision-index row marked "can be revisited".
- **Q1:** LanceDB stays on the Slice 40 renderer in M3, with the renderer
  labelled in every table. A LanceDB-plus-pack arm is deferred.

## 7. W2 and W4 record (2026-10-16)

### 7.1 RED at W2 (`86e15e333`, Cargo.lock `338d1da9f`)

The core functions were `todo!()` stubs; the bindings were already thin
wrappers over them.

| Suite | Result | Why |
| --- | --- | --- |
| `fathomdb-engine` `slice41_context_pack` (default features) | 24 of 24 fail | every test reaches a `todo!()` (103 "not yet implemented" panics) |
| `fathomdb-sdk` `slice41_context_pack` | 4 fail, 1 pass | the stubs; `default_policy_matches_the_documented_defaults` passes (the default is data) |
| `fathomdb-sdk` `surface` | 15 pass | signatures only |
| `fathomdb-embedder` `slice41_marker_embedding` (AC41-19) | pass | characterization: no code is needed; the cached default tokenizer deletes the markers |
| Python `test_context_pack_parity.py` | 42 fail, 3 pass | `PanicException` from the stubs; the default, type-error and export tests pass |
| TypeScript `context-pack.test.ts` | 43 of 43 fail | `FathomDbPanicError` from the stubs |

### 7.2 GREEN at W4 (core `3e0903d91`)

No test was edited.

| Suite | Result |
| --- | --- |
| `fathomdb-engine` `slice41_context_pack` | 24 of 24 pass |
| `fathomdb-sdk` `slice41_context_pack`, `surface` | 5 and 15 pass (14 existing plus the new signature pin) |
| `fathomdb-embedder --features default-embedder` `slice41_marker_embedding` | pass, 20 bodies, one over 512 tokens |
| Python `test_context_pack_parity.py` | 45 of 45 pass |
| TypeScript `context-pack.test.ts` | 43 of 43 pass |

`tagged_and_plain_corpora_fuse_identically` (`--features default-embedder`)
fails at the committed `drain(30_000)`: on CPU the two 200-row stores need
about 100 s to embed, and `drain` returns `Scheduler`. A scratch copy with
`drain(600_000)` passed: ids, order and fused-score bits were identical on
all 40 queries. By orchestrator ruling 1 the committed test now uses
`drain(600_000)` (a timing parameter; no assertion changed), and the file
passes 25 of 25 with `--features default-embedder` (102 s).

### 7.3 AC41-27 differential ($0)

The check was run as a scratch script, not committed, because the harness
branch is read-only for this slice. `pack_context` with
`full_items = 10**6`, 4000/30000, expansion on and chronological order,
over items built from the dataset (SHA-256 checked against the run config),
reproduced the recorded `evidence_sha256` for 180 of 180 comp03-run-mix
records: FTS, hybrid and LanceDB 60 each.

### 7.4 M1 preview (engine `5b09afd44`, decides nothing)

This is a scratch script, rendered through `pack_context`. FTS uses the
recorded FTS top-100 prefix, which is prefix-stable. "Hybrid@100" uses the
hybrid limit-100 ranking prefix, which is not the native limit-k hybrid arm
(test-plan § 4.1), so its rows are indicative only. Cells are hasA_allfull;
the last two columns are weighted allfull and weighted gold-all.

Selection set (414 scored questions; SSU 56, SSA 46, SSP 20, MS 112, KU 63,
TR 117):

| Arm | Policy | SSU | SSA | SSP | MS | KU | TR | w. allfull | w. gold-all |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| FTS | mix (control) | 1.000 | .978 | .850 | .661 | .952 | .752 | .825 | .875 |
| FTS | T3/A300/k30 | 1.000 | .913 | .950 | .866 | 1.000 | .906 | .927 | .961 |
| FTS | T3/A0/k30 | 1.000 | .913 | 1.000 | .929 | 1.000 | .957 | .960 | .961 |
| FTS | T5/A300/k50 | 1.000 | .913 | .850 | .857 | .984 | .889 | .911 | .972 |
| FTS | T5/A0/k50 | 1.000 | .913 | 1.000 | .929 | 1.000 | .966 | .962 | .979 |
| Hybrid@100 | mix (control) | 1.000 | .978 | .950 | .688 | .968 | .786 | .850 | .891 |
| Hybrid@100 | T3/A300/k30 | 1.000 | .913 | .950 | .902 | 1.000 | .923 | .941 | .970 |
| Hybrid@100 | T3/A0/k30 | 1.000 | .913 | 1.000 | .955 | 1.000 | .957 | .967 | .970 |
| Hybrid@100 | T5/A300/k50 | 1.000 | .913 | .850 | .884 | .984 | .915 | .925 | .977 |
| Hybrid@100 | T5/A0/k50 | 1.000 | .913 | 1.000 | .938 | 1.000 | .966 | .965 | .982 |

Answer set (56 scored), weighted allfull: FTS mix .841, T3/A300 .888,
T3/A0 .924, T5/A300 .888, T5/A0 .924; Hybrid@100 mix .876, T3/A300 .888,
T3/A0 .924, T5/A300 .918, T5/A0 .930.

If the § 4.1 rule were applied to this preview, T5/A300/k50 would fail the
guard on hybrid SSP (.850 against .950 − .02). T3/A0/k30 and T5/A0/k50 tie
on mean weighted allfull (.9635), and the smaller k wins, so T3/A0/k30
would be picked. The binding M1 run at the Unit H head decides.

## 8. Phase 2 tests and measurements (planned; none written)

New test files only; no existing test is edited. Expected values are
derived by hand from requirements § 5. The P2-R0 plan review's findings
(F-Q1 to F-Q8, F-F1 to F-F3) are applied in requirements § 5 and design
§ 8–9.

### 8.1 Unit F

| Test | Where | AC |
| --- | --- | --- |
| fixture cases `full_ids_past_full_items`, `full_ids_inside_full_items_and_unselected`, `full_ids_over_budget` (the last includes a forced item the budget omits: `tier = "omitted"`, `forced = false`) | `src/conformance/context-pack-fixtures.json`, new key `full_ids_cases` | 28–30 |
| fixture error cases for unknown, repeated and empty `full_ids`, and an id repeated across `hits ∪ context` that is also in `full_ids` (expects the REQ41-P8 error) | same file, new key `full_ids_errors` | 31 |
| `prop_empty_full_ids_is_phase_one`, `prop_forcing_changes_only_forced_and_later_items` (the three AC41-32 clauses) | `fathomdb-engine/tests/slice41_full_ids.rs` (new) | 32 |
| `every_full_ids_case_matches_through_*` | `fathomdb-sdk/tests/slice41_full_ids.rs`, `src/python/tests/test_context_pack_full_ids.py`, `src/ts/tests/context-pack-full-ids.test.ts` (new) | 33 |

Today's suites already load every `pack_cases` entry. The appended cases
therefore run there too and fail RED until W4, which is intended.

Q-T1, ruled YES by the orchestrator on P2-R0: the existing comparators
compare whole objects (TypeScript `deepStrictEqual`, Rust `assert_eq!` on
JSON, Python `==`), so the phase 1 expected entries gain `"forced": false`
in the RED commit, as one mechanical fixture-data edit; no test code
changes and the comparators are not loosened. The edit is checked by a
scratch script that deletes every `"forced": false` key and compares the
result with the previous fixture byte for byte; status.md records the
result. The phase 1 cases then fail RED too until W4 adds `forced` to the
wire output.

Ruling 5 (status.md, option A): the TypeScript and Rust phase 1
comparators list fields by hand, so they gain `forced`; the default-policy
checks gain `full_ids`; the fixture's `defaults` gains `"full_ids": []`.
The phase 2 cases therefore go under new keys `full_ids_cases` and
`full_ids_errors`, not `pack_cases` and `pack_errors`: the phase 1 helpers
do not pass `full_ids`, so they would render those cases wrongly forever.

### 8.2 Unit Q

| Test | Where | AC |
| --- | --- | --- |
| `default_set_compiles_like_today` (fixed query list, including every phase 1 and Slice 40 query-module case) and `prop_default_set_is_identity` | `fathomdb-query` tests module, new `#[cfg(test)] mod stop_words_tests` | 34 |
| `extra_keep_and_none_bases_shape_the_terms`, `all_stopword_fallback_uses_the_effective_set`, `matching_is_exact_token` (`suggest` leaves `suggests`; `cafe` leaves `café`) | same | 35 |
| `invalid_settings_are_refused_naming_the_field` (each REQ41-Q2 rule, including `base = none` with a non-empty `keep`; asserts `field`, `index` and `reason`) | same, plus engine open (`EngineConfigurationError::QueryStopWords`) in `fathomdb-engine/tests/slice41_query_stop_words.rs` (new) | 36 |
| `invalid_query_stop_words_are_invalid_argument` | `fathomdb-sdk/tests/slice41_query_stop_words.rs` (new), asserting `ErrorKind::InvalidArgument` at open | 36 |
| `every_compile_site_honours_the_setting` (text-only, search text branch, frozen search, evidence search, projected text search, graph-expand query seed; each against a corpus where the three words decide the top hit; the match expression via `take_slice71_search_statement_trace_for_test` where it reaches the site, else a ranking witness) | `slice41_query_stop_words.rs` | 37 |
| `trace_records_the_stop_word_hash` (explain trace, graph-evidence trace, open report), `default_hash_constant_matches_the_list` | `slice41_query_stop_words.rs`, `fathomdb-query` tests | 38 |
| Python and TypeScript open-time refusal and hash tests (explain trace, graph-evidence trace, open report, `DEFAULT_STOP_WORDS_SHA256` equal to the Rust constant's literal) | `test_query_stop_words.py`, `query-stop-words.test.ts` (new) | 36, 38 |
| in-repo cross-SDK case: `src/conformance/query-stop-words-fixtures.json` (new; small hand-written corpus, the three-word setting, hand-derived top-3 ids with and without it); each SDK asserts the ids and score bits equal to its own default engine queried with the words stripped | `slice41_query_stop_words.rs`, `test_query_stop_words.py`, `query-stop-words.test.ts` | 40 |
| harness cross-SDK case (equal ids and score bits across the SDKs at measurement scale) | the Unit E harness suite, on the harness branch; run by the Unit H owner and recorded with MQ1; does not gate this slice | 40 (recorded) |

AC41-39 is the existing engine, Python and TypeScript suites, run
unchanged.

### 8.3 Measurements ($0)

- **MF1:** `variants.py` (c) and (b) re-run through `pack_context`, with
  `full_ids` for (c), on recorded FTS top-100. Pass: SSA .978 and weighted
  allfull .962 for (c), and .967 for (b), on the 414 selection set. The
  expected cost of (c) is SSP .950 (−1 question); that value is not a
  defect.
- **MQ1:** the COMP-03 sweep at the merged head with REQ41-Q8's config.
  Pass: FTS top-10 equal to the harness transform's (the words stripped,
  default engine) at the same head, for 500 of 500. Informative row:
  how many of the 500 equal `simulated_fts_ids` (simulated at
  `5b09afd44`).
- **MQ2:** the defaults at the merged head. Pass: COMP-03 sweep FTS top-10
  equal to the Slice 40 final-head receipts; COMP-01 FTS ids identical;
  COMP-02a within tolerance.

### 8.4 Order

1. Plan review.
2. RED: the new tests and fixture cases.
3. GREEN: unit F, then unit Q.
4. Code review.
5. MF1, which needs no Unit H.
6. Rebase onto Slice 40 after Unit H, then MQ1 and MQ2 (design § 9.5).
   The precondition that Slice 40's H1 RED tests are gone is satisfied by
   their revert at `fb42f612e`.
