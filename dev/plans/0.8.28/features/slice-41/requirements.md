---
title: FathomDB 0.8.28 Slice 41 — requirements and acceptance criteria
status: DRAFT (for review before any test or code)
target_release: 0.8.28
observed_on: 2026-10-16
---

# Slice 41 requirements and acceptance criteria

Slice 41 makes tiered evidence rendering an SDK function. The owner ruled
it in on 2026-10-16 as opportunity 1 of the COMP-03 improvement report.
[design.md](design.md) gives the mechanism, [test-plan.md](test-plan.md)
the tests and measurements.

## 1. Evidence

Source: the read-only COMP-03 improvement report of 2026-10-16 and its
offline prototypes `tiered.py` and `compact.py` (orchestrator scratchpad,
not tracked). All numbers are offline and deterministic: no LLM calls. They
come from the Slice 40 wheel at engine `5b09afd44`, using FTS top-100
retrieval on 470 scored questions (the 500 minus 30 abstention questions).
"allfull" means every `has_answer` turn is present in full.

- 842 of 896 `has_answer` turns (94%) are user turns. Only
  single-session-assistant (SSA) depends on assistant turns (51 of 56).
- User text per round has a median of 182 characters; assistant text has a
  median of 1,703. The Slice 40 renderer spends about 87% of 30,000
  characters on assistant text. The prompt holds about 13.5 rounds from
  about 5.5 sessions.

Type-weighted allfull at the same 30,000-character budget and the same
session expansion:

| Policy | SSU | SSA | SSP | MS | KU | TR | weighted |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Slice 40: 4000 per round, k = 10 | 1.000 | .964 | .733 | .653 | .958 | .772 | .820 |
| Top 3 full, rest user + 300 assistant chars, k = 30 | 1.000 | .911 | .900 | .851 | 1.000 | .913 | .921 |
| Top 5 full, rest user only, k = 50 | 1.000 | .911 | .967 | .917 | 1.000 | .969 | .958 |
| 300 assistant chars everywhere, k = 30 | .969 | .107 | .900 | .851 | 1.000 | .937 | .833 |

- On hybrid top-100, the top-3/300/k30 policy gives .934 weighted (Slice 40
  hybrid: .846).
- SSA is the one type that loses (.964 to .911). Compacting every round
  kills it (.107), so the top tier must keep assistant text.
- Session-diverse selection (one round per session) with full rounds lowers
  allfull to .652 because session expansion evicts answer rounds. Its
  combination with tiering was not measured.
- The logic exists only in the prototype and in the COMP-03 harness
  (`experiments/comp03_retrieval.py` `evidence_rounds` and
  `experiments/comp03_answer.py` `render_evidence`, branch
  `slice/0.8.28-40-comp03`). It therefore helps every store equally until
  FathomDB ships it.

Facts read from the code at `aea8e0457`:

- `SearchHit` carries `id`, `kind`, `body`, `score`, `branch`, `source_id`
  and `ce_score`. It has no role or structure field.
- Base FTS indexes the raw node body with
  `porter unicode61 remove_diacritics 2`. A JSON body would add its keys
  and escapes as tokens.
- The name "evidence" is taken. `search_with_evidence` and
  `resolve_evidence` (0.8.25 Slice 50) mean exact source-byte evidence. This
  slice therefore uses "context pack".
- Precedent for a package-level, engine-free operation: `rerank` in Python,
  TypeScript and `fathomdb-sdk` `standalone.rs`.

## 2. Requirements

### Pack function (P)

**REQ41-P1 — inputs.** `pack_context(hits, policy, context=())` takes:

- `hits`: ranked context items;
- `context`: optional extra items, used only for group expansion;
- `policy`: the budget and tier settings.

An item has `id`, ordered `parts` (each a `role` and `text`), and optional
`group`, `position`, `order_key` and `header` fields. The function does no
I/O and needs no open engine.

**REQ41-P2 — tiers.** The first `full_items` items in selection order
(REQ41-P4) are in the full tier: every part is rendered. Hits come first,
so with `full_items` no larger than the hit count only hits are full. Every
other rendered item is in the compact tier:
parts whose role is in `keep_roles` are rendered in full. The remaining
parts share `compact_chars` characters in part order, and a part cut to 0
characters is dropped. Defaults: `full_items = 3`, `compact_chars = 300`,
`keep_roles = ("user",)`. A `full_items` at least as large as the
selection renders everything in full, which is the Slice 40 renderer.

**REQ41-P3 — budget.** Items are taken in selection order. Each item's text
is cut to `item_chars` (default 4000) and then to what is left of
`budget_chars` (default 30000). Selection stops when the budget reaches 0.
Item text is the rendered parts joined by `"\n"`. Headers and separators
do not count against the budget.

**REQ41-P4 — group expansion.** With `expand_groups = True` (the default),
selection order is: the hits in rank order, then, for each group in order
of its first hit, the `context` items of that group not already listed,
ordered by `position`. Items without a group are not expanded.

**REQ41-P5 — output order.** With `order = "rank"`, selected items are
rendered in selection order. With `order = "chronological"`, they are
sorted by `(order_key, group, position)`, with selection index as the final
tie-break. Items without an `order_key` sort after those with one. Strings
compare by Unicode scalar value. Tier and budget are decided before
sorting.

**REQ41-P6 — provenance.** The result has the rendered `text`,
`chars_used`, and one provenance entry per input item, hits first and then
context, in input order. Each entry has:

- `id`;
- `source`: `"hit"` or `"context"`;
- `tier`: `"full"`, `"compact"` or `"omitted"`;
- `chars`;
- `truncated`;
- `rendered_index`: position in the output, or none;
- per part: `role`, `chars` and `of_chars`.

`truncated` is true when any part has `chars < of_chars`. An item that is
not reached, or is whitespace-only after rendering, is `"omitted"` with
`chars = 0`.

**REQ41-P7 — character unit.** Every count and cut is in Unicode scalar
values (Python `len`, Rust `chars()`), never bytes or UTF-16 code units. A
cut never splits a scalar value.

**REQ41-P8 — validation.** Invalid input raises the SDK's typed
invalid-argument error, carrying a `field_path`. Invalid input means:

- negative counts or a `budget_chars` of 0;
- an empty `id`, or an `id` repeated across `hits` and `context`;
- a `position` missing on a grouped context item when `expand_groups` is
  on;
- an unknown `order` value.

**REQ41-P9 — rendering.** Each rendered item is `header + "\n" + text` when
`header` is set, otherwise `text`. Items are joined by `"\n\n"`. Output is
a pure function of the inputs.

### Role-tagged body (B)

**REQ41-B1 — encode and decode.** `encode_round_body(parts)` returns a
body string. `decode_round_body(body)` returns the parts.
`decode_round_body(encode_round_body(p)) == p` for every valid `p`. A part
text that contains a marker character raises the invalid-argument error.

**REQ41-B2 — plain-text equivalence.** Removing the markers from an encoded
body gives exactly the `"\n"`-joined part texts, the body the COMP-03
harness stores today.

**REQ41-B3 — FTS neutrality.** For any encoded body and any query,
`search_text_only` returns the same ids, in the same order, with
bit-identical scores as for the plain-text body.

**REQ41-B4 — embedding neutrality.** The default embedder returns
bit-identical vectors for an encoded body and its plain-text body. If this
fails, the convention is not used on vector-indexed kinds, and the design's
fallback applies (design § 3.4).

**REQ41-B5 — from hits.** `context_item_from_hit(hit, group=None,
position=None, order_key=None, header=None)` builds an item from a
`SearchHit`. Its `id` is the hit's prefixed id and its parts come from
`decode_round_body(hit.body)`. An untagged body decodes to one part with
role none, and role none is always kept in full. An untagged item
therefore renders as in the Slice 40 renderer, with the per-item and total
caps.

### Surface and parity (S)

**REQ41-S1 — names.** Python `fathomdb.pack_context`,
`fathomdb.encode_round_body`, `fathomdb.decode_round_body` and
`fathomdb.context_item_from_hit`. TypeScript `packContext`,
`encodeRoundBody`, `decodeRoundBody` and `contextItemFromHit`. Rust
`fathomdb_sdk::pack_context` and the others in `standalone`. One Rust
implementation backs all three.

**REQ41-S2 — exact parity.** On the shared fixture set, the three SDKs
return identical `text` bytes and identical provenance, field for field.
Separate fixtures cover non-ASCII text (multi-byte, astral-plane, combining
marks).

**REQ41-S3 — public-surface contract.** `dev/interfaces/python.md`,
`typescript.md`, `rust.md` and `rust-sdk.md` describe the four operations,
the policy defaults, the character unit, the marker convention and its
neutrality guarantees. The governed allowlist and operation-parity map gain
the operations. `scripts/check-sdk-surface-parity.py` passes.

**REQ41-S4 — no search-path change.** No engine search, write, projection
or schema code changes. The diff touches only the new pure module, the SDK
bindings, the conformance maps and docs.

### Measurement (M)

**REQ41-M1 — offline coverage.** On the 470 scored LongMemEval-S questions,
for FTS and hybrid, with retrieval at the Slice 40 Unit H head, record per
type and weighted: gold-all, hasA_full, hasA_allfull and rounds shown. Do
this for the Slice 40 mix renderer and for each grid policy in
test-plan § 4.1, all rendered through `pack_context`. The grid's winner is
chosen by the rule written there before the run.

**REQ41-M2 — SSA guard.** The adopted policy keeps SSA allfull at or above
.89 on both arms. Every other type's allfull is at least the Slice 40 mix
value minus .02. SSA is exempt from the .02 rule because the measured
policy loses .053 there by design.

**REQ41-M3 — paid answer run.** One COMP-03 answer run (60 questions)
compares, on the FathomDB FTS and hybrid arms, the Slice 40 mix config
against `pack_context` with the adopted policy, both at the Unit H head in
the same run. It reports accuracy, paired flips and a reweighted-500
estimate (per-type accuracy times type weight). The noise floor is about 3
of 60 flips per arm, with an MDE of about 6 of 60. The run is a
no-regression check, not proof of the gain.

**REQ41-M4 — COMP-01 and COMP-02 unaffected.** By REQ41-S4 they cannot
move. COMP-01 (13 checks) and the COMP-02a 10k latency run are re-run once
at the slice head as a check.

**REQ41-M5 — order of work.** The SDK work, tests and parity do not wait
for Unit H. The M1 numbers that decide adoption, and M3, use the Unit H
head, because Unit H changes FTS ranking (the stop list), hybrid ranking
at k = 30 (the vector pool) and the filter plan. A pre-Unit H M1 preview
may be recorded but decides nothing.

## 3. Non-goals

- Session-diverse ordering or a session-grouped search API (report
  opportunity 3). Hit order belongs to the caller: `pack_context` keeps
  the input order. Its combination with tiering is unmeasured. An offline
  probe can reorder hits without any SDK option (test-plan § 4.4).
- An engine-backed variant that fetches group members itself. The caller
  supplies `context`, for example from `read.list` by `source_id`.
- Fielded BM25F on user and assistant text (report opportunity 8).
- Changes to how competitor arms render evidence in COMP-03.
- Any change to retrieval, ranking, schema or projections.

## 4. Acceptance criteria

Rust tests are in `src/rust/crates/fathomdb-engine/tests/slice41_context_pack.rs`
unless another file is named. Parity tests are in
`src/python/tests/test_context_pack_parity.py` and the TypeScript test
`src/ts/tests/context-pack.test.ts`.

| AC | Check | REQ |
| --- | --- | --- |
| AC41-01 | Hand-built fixture: 5 hits, top 3 full, 2 compact; compact items keep the user part whole and cut assistant text to 300 chars | P2 |
| AC41-02 | Compact items with several non-kept parts share `compact_chars` in part order; a part cut to 0 is dropped | P2 |
| AC41-03 | Tiers follow selection order: with 2 hits and `full_items = 3`, the first expansion item is full and the next is compact | P2, P4 |
| AC41-04 | Budget: the last item is cut to the remaining characters, later items are `"omitted"`, `chars_used <= budget_chars` | P3, P6 |
| AC41-05 | `item_chars` caps a full-tier item; `truncated` is true and part `chars` sum to item `chars` | P3, P6 |
| AC41-06 | Expansion order: groups by first hit rank, members by `position`, listed hits not repeated | P4 |
| AC41-07 | Chronological order sorts by `(order_key, group, position)` without changing tiers or cuts | P5 |
| AC41-08 | Whitespace-only rendered items are omitted, consume no budget and keep their tier slot | P6, P9 |
| AC41-09 | Provenance has one entry per input item in input order, with `rendered_index` matching the output | P6 |
| AC41-10 | Cuts count scalar values on astral-plane and combining-mark text; no output contains a split scalar | P7 |
| AC41-11 | Each invalid input in REQ41-P8 raises the typed error with the right `field_path` | P8 |
| AC41-12 | With `full_items` at least the selection size and expansion on, output equals the Slice 40 renderer (`render_evidence` with 4000/30000) on a recorded fixture | P2, P3, P9 |
| AC41-13 | Property: `chars_used <= budget_chars`; rendered text length equals the sum of item chars plus headers and separators | P3, P6 |
| AC41-14 | Property: raising `budget_chars` never removes an item or shortens a part already rendered | P3 |
| AC41-15 | Property: output and provenance are identical across repeated calls and reversed `context` order (ordering by `position`) | P4, P9 |
| AC41-16 | Property: `decode(encode(p)) == p`; markers removed equal the `"\n"`-joined texts | B1, B2 |
| AC41-17 | A part text containing a marker character is refused | B1 |
| AC41-18 | Engine test: a tagged corpus and its plain twin give equal ids, order and score bits for a query set that includes short tokens and stop words | B3 |
| AC41-19 | Embedder test: tagged and plain bodies embed to bit-identical vectors (default embedder, CPU) | B4 |
| AC41-20 | `context_item_from_hit` decodes tagged hits; untagged hits render as one kept part | B5 |
| AC41-21 | Cross-SDK: Python, TypeScript and Rust give identical text bytes and provenance on every fixture, ASCII and non-ASCII | S1, S2 |
| AC41-22 | Interface docs describe the operations; `check-sdk-surface-parity.py` and the operation-parity oracle pass | S3 |
| AC41-23 | The slice diff touches no engine search, write, projection or schema file (review check and `git diff --stat` listing) | S4 |
| AC41-24 | M1 table recorded at the Unit H head; adopted policy chosen by the pre-written rule; SSA guard met | M1, M2 |
| AC41-25 | M3 answer run recorded with flips, noise floor and reweighted-500 estimate | M3 |
| AC41-26 | COMP-01 13/13 and the COMP-02a 10k run recorded at the slice head | M4 |
