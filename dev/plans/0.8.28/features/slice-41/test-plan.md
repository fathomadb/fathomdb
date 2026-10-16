---
title: FathomDB 0.8.28 Slice 41 — test plan
status: REVISED AFTER R0 (APPROVE-WITH-FIXES, F1–F15); no test written yet
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
  the parity map. The allowlist entries land only with the owner's
  `_slice41_context_pack_signature`. Until then this AC is open and W4
  cannot close.

## 4. Measurements (orchestrator runs them; harness branch)

### 4.0 AC41-27 differential ($0, no Unit H wait)

For each of the 60 answer questions, build `caller` items from the
dataset (design § 4). Pack the recorded comp03-run-mix top-10 ids with
the mix policy: `full_items = 10**6`, 4000/30000, expansion on,
chronological. Compare the SHA-256 of `text` with the recorded
`evidence_sha256`. Every one must match, on every arm whose records exist.
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

| Policy | `full_items` | `compact_chars` | k |
| --- | ---: | ---: | ---: |
| Slice 40 mix (control) | 10^6 | n/a | 10 |
| T3/A300/k30 (the SDK default) | 3 | 300 | 30 |
| T3/A0/k30 | 3 | 0 | 30 |
| T5/A300/k50 | 5 | 300 | 50 |
| T5/A0/k50 | 5 | 0 | 50 |

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
- **LanceDB:** pending owner ruling (R0 Q1). Not re-run; it stays on the
  Slice 40 renderer, and every table names each arm's renderer.
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
- **F2:** the allowlist delta is a W4 gate that needs the owner's
  signature; the implementer does not edit the allowlist.
- **F3:** selection on the non-answer set; the answer set is reported
  separately.
- **F4:** hand-written AC41-12 fixture plus the AC41-27 differential
  check; bracketed header.
- **F13:** reuse Slice 40's post-Unit H mix run as the M3 control when the
  reader, judge and seeds match.

Pending owner ruling, written with these defaults:

- **Q1:** LanceDB stays on the Slice 40 renderer in M3, with the renderer
  labelled in every table. A LanceDB-plus-pack arm is deferred.
- **Q4, Q5:** the tagged body format is opt-in, its hazards are
  documented, and it gets a decision-index row as a stored-bytes format.
