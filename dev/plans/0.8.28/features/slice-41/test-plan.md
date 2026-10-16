---
title: FathomDB 0.8.28 Slice 41 — test plan
status: DRAFT (for review; no test written yet)
target_release: 0.8.28
observed_on: 2026-10-16
---

# Slice 41 test plan

The tests for each AC in [requirements.md](requirements.md) § 4, and the
measurements. Expected outputs are written by hand from the rules in
REQ41-P2 to P9. AC41-12 is the exception: its expected text is the output
of the reviewed Slice 40 harness function `render_evidence`, recorded once
as a fixture. No expected output comes from the code under test.

## 1. Commands

```bash
cargo test -p fathomdb-engine --test slice41_context_pack
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
| `tiers_follow_selection_index_into_expansion` | 03 |
| `budget_cuts_last_item_and_omits_the_rest` | 04 |
| `item_chars_caps_full_tier_and_sets_truncated` | 05 |
| `expansion_orders_groups_by_first_hit_and_members_by_position` | 06 |
| `chronological_order_sorts_without_changing_cuts` | 07 |
| `whitespace_only_item_is_omitted_and_keeps_its_slot` | 08 |
| `provenance_has_one_entry_per_input_in_input_order` | 09 |
| `cuts_count_scalar_values_on_astral_and_combining_text` | 10 |
| `invalid_inputs_raise_typed_error_with_field_path` (one case per REQ41-P8 rule) | 11 |
| `large_full_items_matches_slice40_renderer_fixture` | 12 |
| `prop_chars_used_within_budget_and_lengths_add_up` (proptest) | 13 |
| `prop_raising_budget_never_removes_rendered_text` (proptest) | 14 |
| `prop_deterministic_and_context_order_independent` (proptest) | 15 |
| `prop_round_body_round_trips_and_strips_to_plain` (proptest) | 16 |
| `encode_refuses_marker_characters_in_text` | 17 |
| `tagged_and_plain_corpora_rank_identically` (real engine, tempdir) | 18 |
| `context_item_from_hit_decodes_tagged_and_keeps_untagged` | 20 |

AC41-18 builds two databases from one 200-row fixture: plain bodies and
tagged bodies. It runs 40 queries, including 1-2 character tokens, stop
words, possessives and a query that matches only assistant text. It
asserts equal ids, equal order and `f64::to_bits` equal scores from
`search_text_only` at limits 10 and 100.

AC41-19 is `fathomdb-embedder/tests/slice41_marker_embedding.rs`
(the default embedder on CPU). It
embeds 20 tagged bodies and their plain twins and compares every `f32`
bit. It is skipped, with a printed notice, only when the model is not
cached. In that case the gate run must have the model cached.

## 3. Parity (AC41-21, AC41-22)

- One fixture file of about 20 cases: inputs, policy and the hand-written
  expected `ContextPack` as JSON. It covers every REQ41-P rule and three
  non-ASCII cases (CJK, emoji with ZWJ, combining marks).
- The Rust test, the Python test and the TypeScript test each load the
  fixture and compare `text` bytes and every provenance field to the
  expected value. Byte equality across the three SDKs follows from all
  three matching the expected bytes.
- `context_item_from_hit` parity uses a small real database: the same
  query through each SDK, then `pack_context`, then a compare.
- AC41-22: `check-sdk-surface-parity.py` and the operation-parity oracle
  `test_sdk_surface_parity_oracle.py` pass with the four new operations in
  both maps.

## 4. Measurements (orchestrator runs them; harness branch)

### 4.1 M1 offline coverage ($0)

- **Data:** the 470 scored LongMemEval-S questions, FTS and hybrid arms
  (GPU embedder on a 3090 for hybrid). Retrieval runs at the Unit H head,
  at each config's own k. There is no slicing of a k = 100 list, because
  hybrid is not prefix-stable.
- **Renderer:** `pack_context` in `caller` item mode for every row below.

Grid:

| Policy | `full_items` | `compact_chars` | k |
| --- | ---: | ---: | ---: |
| Slice 40 mix (control) | 10^6 | n/a | 10 |
| T3/A300/k30 (the default) | 3 | 300 | 30 |
| T3/A0/k30 | 3 | 0 | 30 |
| T5/A300/k50 | 5 | 300 | 50 |
| T5/A0/k50 | 5 | 0 | 50 |

All rows use `budget_chars = 30000`, `item_chars = 4000`, expansion on and
`keep_roles = ("user",)`.

- **Metrics per type and weighted:** gold-all, hasA_full, hasA_allfull,
  rounds shown. Type weights are those of the report: MS .266, TR .266, KU
  .156, SSU .14, SSA .112, SSP .06.
- **Rule, fixed before the run:**
  1. Keep the policies that pass REQ41-M2 on both arms.
  2. Of those, adopt the one with the highest mean weighted allfull over
     the two arms. Break ties with the smaller k, then the larger
     `compact_chars`.
  3. Adopt only if its weighted allfull beats the control by at least .05
     on both arms.
  4. If none qualifies, the defaults stay T3/A300 and the result is
     reported to the owner before W4 closes.
- The pre-Unit H preview at engine `5b09afd44` may be recorded, labelled
  "preview", and decides nothing.

### 4.2 Retrieval identity for `hits` mode ($0)

Rebuild the FathomDB stores with tagged bodies and compare top-30 ids
against plain stores on all 500 questions:

- FTS: always.
- Hybrid: only if AC41-19 passed.

Any difference blocks `hits` mode, and M3 then uses `caller` mode.

### 4.3 M3 paid answer run (one run)

- **Arms:** FathomDB FTS and FathomDB hybrid. Each runs the Slice 40 mix
  (k = 10) and the adopted policy at its k. All four arm-configs are in
  one run at the Unit H head. LanceDB is not re-run (Q1).
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
| M1 preview | no |
| M1 adoption numbers (§ 4.1) | yes: the stop list moves FTS ranks; the vector pool moves hybrid at k = 30 and 50 |
| Retrieval identity (§ 4.2) | yes: compare at the head that M3 uses |
| M3 paid run (§ 4.3) | yes |
| COMP-01 and COMP-02 re-run (§ 4.5) | yes: run at the final slice head |

## 6. Open questions for review

- **Q1.** Should the LanceDB arm in M3 use `pack_context` too? The plan
  says no: the product claim is "FathomDB ships it". A store-neutral
  comparison would need LanceDB rendered the same way. That is a fairness
  framing for the owner.
- **Q2.** The report's best weighted allfull was T5/A0/k50 (.958), not
  T3/A300/k30 (.921), with the same SSA (.911). The grid includes both.
  Is choosing by the § 4.1 rule acceptable, or does the owner want
  T3/A300 fixed?
- **Q3.** Should the types be governed `…V1` envelopes like the Slice 50
  evidence types, or plain types like `RerankPassage`?
- **Q4.** Do four new governed operations need an ADR or decision-index
  row, or is the interface-doc update enough (AGENTS.md § 1)?
- **Q5.** Is the control-character marker convention acceptable as a
  public body format? The alternative is to drop it and ship only
  `caller` items.
- **Q6.** Is "headers are not budgeted" right for a product API? It
  matches the harness. A product caller may expect `budget_chars` to bound
  the whole string.
