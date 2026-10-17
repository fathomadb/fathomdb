---
title: FathomDB 0.8.28 Slice 41 — requirements and acceptance criteria
status: phase 1 (§ 2–4) landed; phase 2 (§ 5, units F and Q, owner ruling 2026-10-16) drafted for review
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
`group`, `position`, `order_key` and `header` fields. Types are plain
value types, like `RerankPassage`, with no `V1` suffix (R0 Q3). The function does no
I/O and needs no open engine.

**REQ41-P2 — tiers.** The first `full_items` items in selection order
(REQ41-P4) are in the full tier: every part is rendered. Hits come first,
so with `full_items` no larger than the hit count only hits are full. Every
other rendered item is in the compact tier:

- parts whose role is in `keep_roles`, or whose role is none, are rendered
  unchanged, empty or not;
- the other parts share one allowance of `compact_chars` characters, cut
  in part order; the join characters do not count against it;
- a non-kept part that renders 0 characters is dropped before the join.
  This covers a part that was empty to begin with and one cut to empty.

The full tier never drops a part, so its text is exactly the
`"\n"`-joined part texts. `full_items = 0` makes every item compact.

These defaults are a product choice, fixed now and independent of the
COMP-03 measurement (R0 F7): `full_items = 3`, `compact_chars = 300`,
`keep_roles = ("user",)`. The harness passes the policy it adopts
explicitly. A `full_items` at least as large as the selection renders
everything in full, which is the Slice 40 renderer.

**REQ41-P3 — budget.** Items are taken in selection order. Each item's text
is cut to `item_chars` (default 4000) and then to what is left of
`budget_chars` (default 30000). Selection stops when the budget reaches 0.
Item text is the rendered parts joined by `"\n"`. Headers and separators
do not count against the budget (R0 Q6). `budget_chars` bounds item text
only. The caller supplies the headers and knows their cost, and the result
reports the total length as `text_chars`.

**REQ41-P4 — group expansion.** With `expand_groups = True` (the default),
selection order is: the hits in rank order, then, for each group in order
of its first hit, the `context` items of that group not already listed,
ordered by `position`, with input order as the tie-break. Items without a
group are not expanded.

**REQ41-P5 — output order.** With `order = "rank"`, selected items are
rendered in selection order. With `order = "chronological"`, they are
sorted by `(order_key, group, position)`, with selection index as the final
tie-break. For each of the three keys, an item without a value sorts after
those with one. Strings compare by Unicode scalar value. Tier and budget are decided before
sorting.

**REQ41-P6 — provenance.** The result has the rendered `text`,
`chars_used` (the sum of item `chars`), `text_chars` (the scalar length of
`text`), and one provenance entry per input item, hits first and then
context, in input order. Each entry has:

- `id`;
- `source`: `"hit"` or `"context"`;
- `tier`: `"full"`, `"compact"` or `"omitted"`;
- `chars`;
- `truncated`;
- `rendered_index`: position in the output, or none;
- per part: `role`, `chars` and `of_chars`.

Item `chars` counts the `"\n"` joins between rendered parts, so part
`chars` plus the joins shown equal item `chars`. For a rendered item,
`truncated` is true when any part has `chars < of_chars`. An item that is
not reached, or is whitespace-only after rendering, is `"omitted"` with
`chars = 0`, every part `chars = 0` and `truncated = false` (W4 decision:
omission is reported by the tier, not by `truncated`).

**REQ41-P7 — character unit.** Every count and cut is in Unicode scalar
values (Python `len`, Rust `chars()`), never bytes or UTF-16 code units. A
cut never splits a scalar value. In TypeScript, `chars`, `of_chars` and
`text_chars` therefore differ from `string.length` for astral-plane text.

**REQ41-P8 — validation.** Invalid input raises the SDK's existing
invalid-argument error (`InvalidArgumentError`, Rust
`ErrorKind::InvalidArgument`). Its message names the offending field, for
example `policy.budget_chars` or `context[3].position`. No error payload
field is added (R0 F1). Invalid input means:

- negative counts or a `budget_chars` of 0;
- an empty `id`, or an `id` repeated across `hits` and `context`;
- a `position` missing on a grouped context item when `expand_groups` is
  on;
- an unknown `order` value.

**REQ41-P9 — rendering.** Each rendered item is `header + "\n" + text` when
`header` is set, otherwise `text`; the header is used verbatim. Items are
joined by `"\n\n"`. Hence
`text_chars = Σ chars + Σ (len(header) + 1 over rendered items with a
header) + 2 · (rendered − 1)`, or 0 when nothing is rendered. Output is a
pure function of the inputs.

### Role-tagged body (B)

The convention is opt-in. Nothing in FathomDB writes it unless the caller
calls `encode_round_body`. Owner ruling (2026-10-16, R0 Q4, Q5): it ships
opt-in with its hazards documented (REQ41-B6). As a stored-bytes format it
gets a decision-index row, marked "can be revisited" (design § 5).

**REQ41-B1 — encode and decode.** `encode_round_body(parts)` returns a
body string. `decode_round_body(body)` returns the parts. Decoding is
strict: a body that starts with U+001E but does not decode raises the
invalid-argument error.
`decode_round_body(encode_round_body(p)) == p` for every valid `p`. A part
text that contains a marker character raises the invalid-argument error.

**REQ41-B2 — plain-text equivalence.** Removing the markers from an encoded
body gives exactly the `"\n"`-joined part texts. With one part per turn in
original order, this is the body the COMP-03 harness stores today
(`"\n".join(turn contents)`). It is not that body if the parts are joined
per role.

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

**REQ41-B6 — hazards documented.** The interface docs name the visible
effects of the markers (design § 3.5): Python `str.splitlines()` splits on
U+001E; U+0011 and U+0013 are XON/XOFF; the characters are illegal in
XML 1.0; JSON escapes them; resolved source evidence contains them.

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
the policy defaults, the character unit, the marker convention, its neutrality guarantees and its hazards (REQ41-B6).
The operation-parity map gains the operations, and
`scripts/check-sdk-surface-parity.py` passes.

The governed allowlist (`src/conformance/governed-surface-allowlist.json`)
is HITL-signed: any change to it re-opens the gate. The delta and its
`_slice41_context_pack_signature` text are drafted in design § 5. The
owner approved the four operations, with the R0 adjustments, on 2026-10-16
(R0 F2). The entry is written at W4, not now, and cites that ruling.

**REQ41-S4 — no search-path change (phase 1).** Phase 2 unit Q (§ 5) is the
one deliberate exception: it changes query compilation, and its default
is byte-identical to today (REQ41-Q5). No engine search, write, projection
or schema code changes. The diff touches only the new pure module, the SDK
bindings, the conformance maps and docs, and the decision-index row (REQ41-B1).

### Measurement (M)

**REQ41-M1 — offline coverage.** For FTS and hybrid, with retrieval at the
Slice 40 Unit H head, record per type and weighted: gold-all, hasA_full,
hasA_allfull and rounds shown. Do this for the Slice 40 mix renderer and
for each grid policy in test-plan § 4.1, all rendered through
`pack_context`.

As in Slice 40 Unit G (REQ40-G1), the harness policy is chosen on the
selection set: the scored questions minus the scored answer-set questions,
414 at Slice 40, counted from the data. The 60-question answer set (56
scored) is reported separately, with its expected offline delta (R0 F3).
The rule is in test-plan § 4.1 and is frozen before the Unit H run. The
chosen policy is the harness policy only. It does not change the SDK
defaults (REQ41-P2).

**REQ41-M2 — SSA guard.** The adopted policy keeps SSA allfull at or above
.89 on both arms, on the selection set. Every other type's allfull is at least the Slice 40 mix
value minus .02. SSA is exempt from the .02 rule because the measured
policy loses .053 there by design.

**REQ41-M3 — paid answer run.** One COMP-03 answer run (60 questions)
compares, on the FathomDB FTS and hybrid arms, the Slice 40 mix config
against `pack_context` with the adopted policy, both at the Unit H head.
Slice 40 re-runs its mix answer run after Unit H. If that run uses the
same reader, judge, seeds and question set, it serves as the control, and
M3 runs only the two pack arm-configs (R0 F13). Otherwise all four run
together. It reports accuracy, paired flips and a reweighted-500
estimate (per-type accuracy times type weight). The noise floor is about 3
of 60 flips per arm, with an MDE of about 6 of 60. The run is a
no-regression check, not proof of the gain. Every table names each arm's
renderer.

**REQ41-M4 — COMP-01 and COMP-02 unaffected.** By REQ41-S4 they cannot
move. COMP-01 (13 checks) and the COMP-02a 10k latency run are re-run once
at the slice head as a check.

**REQ41-M5 — order of work.** The SDK work, tests and parity do not wait
for Unit H. The M1 numbers that decide adoption, and M3, use the Unit H
head, because Unit H changes FTS ranking (the stop list), hybrid ranking
at k = 30 (the vector pool) and the filter plan. A pre-Unit H M1 preview
may be recorded but decides nothing. It runs through `pack_context`, not
the Python prototype, so it also smoke-tests the harness `pack` mode.

## 3. Non-goals

- Session-diverse ordering or a session-grouped search API (report
  opportunity 3). Hit order belongs to the caller: `pack_context` keeps
  the input order. Its combination with tiering is unmeasured. An offline
  probe can reorder hits without any SDK option (test-plan § 4.4).
- An engine-backed variant that fetches group members itself. The caller
  supplies `context`, for example from `read.list` by `source_id`.
- Fielded BM25F on user and assistant text (report opportunity 8).
- Changes to how competitor arms render evidence in COMP-03. Owner
  ruling (2026-10-16, R0 Q1): in M3, LanceDB stays on the Slice 40 renderer,
  and every table labels each arm's renderer. A LanceDB-plus-pack arm (the
  same renderer for every store) is deferred to the next COMP-03 headline
  refresh, which would report a store table and a shipped-path table.
- Any change to retrieval, ranking, schema or projections.

## 4. Acceptance criteria

Rust tests are in `src/rust/crates/fathomdb-engine/tests/slice41_context_pack.rs`
unless another file is named. The shared fixture is
`src/conformance/context-pack-fixtures.json`, loaded by the Rust test,
`src/python/tests/test_context_pack_parity.py` and the TypeScript test
`src/ts/tests/context-pack.test.ts`. Every expected value in it is written
by hand from REQ41-P2 to P9 and checked by hand.

| AC | Check | REQ |
| --- | --- | --- |
| AC41-01 | Hand-built fixture: 5 hits, top 3 full, 2 compact; compact items keep the user part whole and cut assistant text to 300 chars | P2 |
| AC41-02 | Compact items with several non-kept parts share `compact_chars` in part order; a non-kept part that renders 0 chars (empty or cut to empty) is dropped; an empty kept part and an empty full-tier part are kept; `full_items = 0` makes every item compact | P2 |
| AC41-03 | Tiers follow selection order: with 2 hits and `full_items = 3`, the first expansion item is full and the next is compact | P2, P4 |
| AC41-04 | Budget: the last item is cut to the remaining characters with a header present, later items are `"omitted"`, `chars_used <= budget_chars` and excludes the header; an item that uses the budget exactly, followed by a whitespace-only item, leaves that item omitted, not rendered empty | P3, P6 |
| AC41-05 | `item_chars` caps a full-tier item; `truncated` is true and part `chars` plus the joins shown equal item `chars`, including a cut that ends on a join | P3, P6 |
| AC41-06 | Expansion order: groups by first hit rank, members by `position`, listed hits not repeated | P4 |
| AC41-07 | Chronological order sorts by `(order_key, group, position)`, with missing values last for each key, without changing tiers or cuts | P5 |
| AC41-08 | Whitespace-only rendered items are omitted, consume no budget and keep their tier slot | P6, P9 |
| AC41-09 | Provenance has one entry per input item in input order, with `rendered_index` matching the output | P6 |
| AC41-10 | Cuts count scalar values on astral-plane and combining-mark text; no output contains a split scalar; the TypeScript test asserts a case where `chars` differs from `string.length` | P7 |
| AC41-11 | Each invalid input in REQ41-P8 raises `InvalidArgumentError` (Rust `ErrorKind::InvalidArgument`) whose message contains the field name | P8 |
| AC41-12 | Hand-written fixture of about 6 rounds in 2 sessions (one blank round, one over 4,000 chars, one with an empty assistant turn, a budget that cuts the last round, headers `[Session date: …]`): with `full_items = 10**6` and expansion on, output equals the hand-derived Slice 40 renderer text | P2, P3, P9 |
| AC41-13 | Property: `chars_used <= budget_chars`; `text_chars == len(text)` and equals the REQ41-P9 formula | P3, P6, P9 |
| AC41-14 | Property: raising `budget_chars` never removes an item or shortens a part already rendered | P3 |
| AC41-15 | Property: output and provenance are identical across repeated calls and reversed `context` order; the generator gives distinct `(group, position)` pairs, since ties fall back to input order | P4, P9 |
| AC41-16 | Property: `decode(encode(p)) == p`; markers removed equal the `"\n"`-joined texts | B1, B2 |
| AC41-17 | A part text containing a marker character is refused; a body starting with U+001E that does not decode is refused | B1 |
| AC41-18 | Engine test: a tagged corpus and its plain twin give equal ids, order and score bits from `search_text_only`. The corpus has duplicate plain twins, multi-part rounds with a query term at the very start of a part, and empty parts. Queries include short tokens and stop words. With `--features default-embedder`, and after AC41-19 passes, hybrid `search` fused scores are bit-equal too | B3, B4 |
| AC41-19 | Embedder test: tagged and plain bodies embed to bit-identical vectors (default embedder, CPU); at least one body is over the 512-token window | B4 |
| AC41-20 | `context_item_from_hit` decodes tagged hits; untagged hits render as one kept part | B5 |
| AC41-21 | Cross-SDK: Python, TypeScript and Rust give identical text bytes and provenance on every fixture, ASCII and non-ASCII | S1, S2 |
| AC41-22 | Interface docs describe the operations and the REQ41-B6 hazards; `check-sdk-surface-parity.py` and the operation-parity oracle pass; the allowlist entry `_slice41_context_pack_signature` cites the 2026-10-16 owner approval; the decision-index row for the tagged body exists | S3, B1, B6 |
| AC41-23 | The slice diff touches no engine search, write, projection or schema file (review check and `git diff --stat` listing) | S4 |
| AC41-24 | M1 table recorded at the Unit H head: the policy is chosen on the selection set by the pre-written rule, the answer set is reported separately, and the SSA guard is met | M1, M2 |
| AC41-25 | M3 answer run recorded with flips, noise floor and reweighted-500 estimate | M3 |
| AC41-26 | COMP-01 13/13 and the COMP-02a 10k run recorded at the slice head | M4 |
| AC41-27 | Harness differential: on the 60 answer questions, `pack_context(full_items = 10**6)` over the recorded comp03-run-mix top-10 ids reproduces every recorded `evidence_sha256` | P2–P9 |

## 5. Phase 2: full-render override and configurable query stop words

Added by owner ruling on 2026-10-16. Phase 2 has its own plan review,
failing-tests step (RED), implementation and code review, as phase 1 did.

### 5.1 Evidence

The evidence is the read-only Q&A of 2026-10-16 (orchestrator scratchpad
`qa-ssa-stop/answer.md`, with `variants.py`). [M] marks measured values,
[R] values read from recorded files. Rows are FTS top-100, the 414-question
selection set, a 30,000-character budget and group expansion on.

- **SSA drops from .978 to .913 because of rank [M].** Three SSA answer
  rounds sit at rank 6, 7 and 14, behind sibling rounds of their own
  session, so the tiers compact exactly the round that holds the answer.
  The excerpt length is not the cause: query-aware excerpts gain one
  question at most.

Two caller-side rules restore SSA [M]:

| Variant | SSA | Weighted allfull |
| --- | ---: | ---: |
| T3/A0/k30 | .913 | .960 |
| (b) assistant-reference detector, `full_items = 8` when it fires | .978 | .967 |
| (c) the rank-1 session's hits in full, up to 10 | .978 | .962 (SSP .950, −1 question) |

Variant (b) is a LongMemEval question-template heuristic. Variant (c) needs
per-item tier control, which the current `pack_context` cannot express.

- **The H1 stop words are LongMemEval-specific [R, M].** Slice 40 Unit H1
  selected `had`, `long` and `suggest`
  (`experiments/configs/comp-03/stoplist-verdict.v1.json` on
  `slice/0.8.28-40-comp03`): +4 questions on 414.
  - They are question-template words. Their document frequency is 6–9% on
    LongMemEval, 7–21% on NFCorpus and 1–18% on TC-5.
  - They are inert on NFCorpus and TC-5 only because those query sets
    almost never contain them (0–1 queries).
  - Owner ruling: they must not become an engine default, and Slice 40
    keeps `STOPWORDS` at 63 words.
- **Stop words are query-side only today.** `fathomdb-query` applies
  `STOPWORDS` in `compile_text_query` and nowhere else. Indexing never
  removes words, so a different list needs no reindex.
- **Where compilation is called today** (`aea8e0457`):
  - `search_api.rs`: `search_text_only_view_with_limit`, and
    `search_inner_with_frozen_binding_and_expansion` (the non-frozen search
    text branch);
  - `search.rs`: `read_projected_text_in_tx` (projected text search), and
    `read_search_in_tx` (the reader-side compile used by frozen and
    evidence search);
  - `graph_expand/execution.rs`: query seeds.

### 5.2 Unit F: per-item full-render override

**REQ41-F1 — `full_ids`.** `ContextPolicy` gains `full_ids`, a set of item
ids; it is empty by default. A selected item whose id is in `full_ids`
renders in the full tier, whatever its selection index. Other items keep
the REQ41-P2 rule. A forced item does not use one of the `full_items` slots.
The full tier is therefore the first `full_items` selected items plus every
selected forced item.

**REQ41-F2 — budget.** Forced items are walked in selection order like
every other item. They are cut by `item_chars` and by the budget left, and
they are omitted once the budget is spent. The budget is not reserved for
them, and selection is never reordered. If forced items alone exceed the
budget, the earliest-selected ones render and the rest are cut or omitted,
exactly as full-tier items are today.

**REQ41-F3 — validation.** These are refused with the invalid-argument
error naming the field (`policy.full_ids[i]`):

- an id that matches no input item;
- a repeated id;
- an empty id.

An id that matches an input item that is not selected has no effect.

REQ41-P8 validation runs first and F3 after it. A request that fails P8,
for example an id repeated across `hits ∪ context`, is refused with the
P8 error naming the P8 field, even when that id is also in `full_ids`.

**REQ41-F4 — provenance.** `ContextPackEntry` gains `forced`: true when the
item's id is in `full_ids` and the item was rendered in the full tier
because of it. `forced` is false for an item whose index already put it in
the full tier. A forced item that is not rendered has `tier = "omitted"`
and `forced = false`; this covers an item omitted because the budget is
spent and an item whose text is whitespace-only. A forced item cut by
`item_chars` or by the budget left is rendered, so it keeps `tier = "full"`
and `forced = true`, with `truncated` showing the cut. Every phase 1 entry
carries `forced = false`.

**REQ41-F5 — parity and surface.** Python `full_ids: tuple[str, ...] = ()`,
TypeScript `fullIds?: readonly string[]`, Rust `full_ids: Vec<String>`; the
wire field is `full_ids`. New shared-fixture cases cover REQ41-F1 to F4. The
allowlist signature text and the pin are updated under the owner's earlier
approval of the context-pack surface. No operation is added.

**REQ41-F6 — callers own the rules.** No question-pattern detector or
session rule goes into the SDK. The COMP-03 harness implements (c) through
`full_ids`, and implements (b) by raising `full_items`, both labelled as
harness heuristics.

### 5.3 Unit Q: configurable query stop words

**REQ41-Q1 — the setting.** `EngineConfig` gains `query_stop_words`,
default `None`. When set, it holds:

- `base`: `"fathomdb"` (the engine list, today 63 words) or `"none"`;
- `extra`: words to add;
- `keep`: words to remove from `base`.

The effective set is `(base ∪ extra) \ keep`. It is fixed for the life of
the opened engine.

**REQ41-Q2 — validation at open.** Each word must be a single lowercase
token in the compiler's own terms: non-empty, every character alphanumeric
(`char::is_alphanumeric`), and equal to its lowercase form. Further rules:

- at most 256 words in `extra` and in `keep`, each at most 64 characters
  (Unicode scalar values);
- no repeats within a list;
- no word in both `extra` and `keep`;
- every `keep` word is in `base` (a typo would otherwise be silent). With
  `base = "none"` the base is empty, so any non-empty `keep` is refused;
- no `extra` word is already in `base`.

Matching is exact on the compiler's tokens. The compiler lowercases the
query, splits it on non-alphanumeric characters and drops a token only when
it equals a word in the set. This happens before FTS5's porter stemming and
without diacritic folding, both of which run inside FTS5 afterwards. So
`extra = ["suggest"]` does not drop `suggests`, and `extra = ["cafe"]` does
not drop `café`. A caller that wants both forms lists both. The Slice 40
H1 harness transform used the same exact-token rule, so its recorded
`simulated_fts_ids` were produced under these semantics.

A violation is an open-time configuration error naming the field: in Rust
`EngineOpenError::EngineConfiguration(EngineConfigurationError::QueryStopWords { .. })`
(design § 9.1), and `InvalidArgumentError` in the Python and TypeScript
SDKs and `ErrorKind::InvalidArgument` in `fathomdb-sdk`.

**REQ41-Q3 — where it applies.** Every query compilation site listed in
§ 5.1 uses the effective set: text-only, the search text branch, frozen
search, evidence search, projected text search and graph-expand query
seeds. Nothing else changes: indexing, the vector branch's query
embedding, fusion and the all-stopword fallback rule, which now uses the
effective set.

The BM25F lexical arm (`bm25f_search_inner`) is not a compile site. It
tokenizes the query through FTS5 with no stop list, so this setting does
not affect it. `search_api.rs`'s plan witness binds the literal
`"fixture"`, not a query, and is also unaffected.

**REQ41-Q4 — recorded.**

- `QueryTrace` gains `stop_words_sha256`: the lowercase hex SHA-256 of the
  effective words, sorted by Unicode scalar value and joined by `"\n"`.
  Every place that builds or maps a `QueryTrace` fills it, including the
  graph-evidence traces in the Python and napi bindings, which carry the
  same engine's effective hash (design § 9.1 lists the sites).
- The default list's hash is a constant, `DEFAULT_STOP_WORDS_SHA256`, in
  `fathomdb-query` (re-exported by `fathomdb-engine`, the `fathomdb` facade
  and `fathomdb-sdk`) and as `DEFAULT_STOP_WORDS_SHA256` in the Python and
  TypeScript packages, so receipts can say "default".
- The effective hash is on the open report (`OpenReport.stop_words_sha256`
  / `stopWordsSha256`), so a receipt can record it without running a query.
  The open report already exists in all three SDKs, so no operation is
  added.
- `Engine.config()` (where the SDK has it) echoes the setting.

**REQ41-Q5 — default identity.** With `query_stop_words` unset, every
compiled MATCH expression is byte-identical to today's, and all existing
search tests pass unchanged. Setting `{"base": "fathomdb"}` with empty
`extra` and `keep` gives the same output, and the same hash as unset.

**REQ41-Q6 — parity.** One implementation, in `fathomdb-query`, does the
validation and the effective-set computation; the bindings convert values
only.

- **In-repo case (gates this slice).** A shared fixture
  `src/conformance/query-stop-words-fixtures.json` holds a small
  hand-written corpus, the setting `extra = ["had", "long", "suggest"]`,
  queries, and the hand-derived top-3 ids with and without the setting
  (the three words decide the top hit). The Rust engine test, the Python
  test and the TypeScript test each load it into a fresh store, open it
  with the setting and assert the hand-derived ids. Each also asserts that
  every score equals, bit for bit, the score of the same store opened with
  the defaults and queried with the three words stripped (the harness
  transform). Score bits are not stored in the fixture, because a captured
  BM25 value would be an implementation-generated oracle.
- **Harness case (measurement scale).** The Unit E cross-SDK suite on the
  harness branch gains the same-store, same-setting case with equal ids and
  score bits across the three SDKs. The Unit H owner runs it; it is
  recorded in MQ1, and does not gate this slice's own close.

**REQ41-Q7 — no per-query override, no CLI suggester (this slice).** See
design § 9.4 for the reasons.

**REQ41-Q8 — harness step, specified here and done on the harness branch.**
The COMP-03 config gains
`query_stop_words: {"base": "fathomdb", "extra": ["had", "long", "suggest"]}`.
The harness passes it to every FathomDB arm's `Engine.open` and records
`stop_words_sha256` in each receipt. The Unit H implementer owns
`slice/0.8.28-40-comp03` and makes the change; this slice does not edit
that worktree.

### 5.4 Phase 2 measurement

- **MF1 ($0).** Re-run variant (c) through `pack_context` with `full_ids`:
  the rank-1 session's hits past the first `full_items`, so that at most 10
  items render in full (`variants.py` `promote`; design § 8.3). On the
  414-question FTS selection set
  it must reproduce SSA .978 and weighted allfull .962, at the recorded
  cost of SSP .950 (−1 question against T3/A0/k30); that SSP value is the
  expected result, not a defect. Run (b) through
  `full_items = 8` for comparison. This step sets no SDK default.
- **MQ1 ($0).** At the merged head, the COMP-03 sweep with the three words
  supplied through the setting. Pass: for every one of the 500 questions,
  the FTS top-10 equals the harness transform's top-10 at the same head
  (strip the three words from the query and search a default engine);
  500 of 500. The Slice 40 property
  `test_stripping_words_equals_compiling_with_them_as_stopwords` pins that
  transform. A second, informative row reports how many of the 500 equal
  the verdict's `simulated_fts_ids`, which were simulated at engine
  `5b09afd44`; a difference there is a head difference (H2/H3), not a
  Unit Q defect.
- **MQ2 ($0).** With the defaults:
  - COMP-03 sweep FTS top-10 equals the Slice 40 final-head receipts;
  - COMP-01 FTS nDCG@10 and MRR@10 are unchanged (ids identical);
  - COMP-02a stays within its manifest tolerances.
- **Order with Unit H.** MQ1 and MQ2 run at the merged head (design § 9.5).

### 5.5 Phase 2 acceptance criteria

| AC | Check | REQ |
| --- | --- | --- |
| AC41-28 | Fixture: a forced item at selection index past `full_items` renders in full with `forced = true`; non-forced items keep index tiers | F1, F4 |
| AC41-29 | Fixture: a forced item inside `full_items` has `forced = false`; a forced id matching an unselected `context` item has no effect | F1, F3, F4 |
| AC41-30 | Fixture: forced items exceeding the budget are cut and omitted in selection order; no reordering; a forced item the budget omits has `tier = "omitted"`, `forced = false` | F2, F4 |
| AC41-31 | Unknown, repeated and empty `full_ids` are refused, naming `policy.full_ids[i]`; an id repeated across `hits ∪ context` and also listed in `full_ids` gets the REQ41-P8 error | F3 |
| AC41-32 | Property: with `full_ids` empty the output equals phase 1's for every request. With `full_ids` non-empty, against the same request without it: (i) the first forced item past `full_items` (in selection order) has `chars` ≥ its phase 1 `chars`; (ii) every item selected before it, which includes every index < `full_items`, is unchanged; (iii) so the first non-forced item whose `chars` differs from phase 1 is selected after the first forced item. Clause (i) holds only for the first forced item: a second forced item can get fewer chars than its compact phase 1 rendering, because the first one spent the budget (W2 note) | F1, F2 |
| AC41-33 | Python, TypeScript and Rust match the extended fixture field for field | F5 |
| AC41-34 | Unit: with no setting, `compile_text_query_with(q, default)` equals `compile_text_query(q)` for a fixed query set; property: equal for any query | Q5 |
| AC41-35 | Unit: `extra` words are dropped, `keep` words return, `base = none` keeps every token; the all-stopword fallback uses the effective set; matching is exact-token: `extra = ["suggest"]` leaves `suggests`, `extra = ["cafe"]` leaves `café` | Q1, Q2, Q3 |
| AC41-36 | Each REQ41-Q2 violation, including `base = none` with a non-empty `keep`, is refused at open with the field named: Rust engine (`EngineConfigurationError::QueryStopWords`), `fathomdb-sdk` (`ErrorKind::InvalidArgument`), Python and TypeScript (`InvalidArgumentError`) | Q2 |
| AC41-37 | Engine: with `extra = [had, long, suggest]`, `search_text_only`, `search` (text branch), frozen search, evidence search, projected text search and graph-expand query seeds all drop the words (match expression via `take_slice71_search_statement_trace_for_test` where it reaches the site, else a ranking witness) | Q3 |
| AC41-38 | `stop_words_sha256` on the explain trace, the graph-evidence trace and the open report equals `DEFAULT_STOP_WORDS_SHA256` when unset or `base = fathomdb` with empty lists, and equals the hand-computed hash of the effective list with extras; the constant is equal across the three SDKs | Q4 |
| AC41-39 | Every existing search test passes unchanged (gate run) | Q5 |
| AC41-40 | In-repo cross-SDK case (REQ41-Q6): the shared small-corpus fixture gives the hand-derived top-3 ids through the Rust engine, Python and TypeScript, with score bits equal to the in-SDK harness transform; the harness Unit E case is recorded with MQ1 | Q6 |
| AC41-41 | MF1 recorded | F6 |
| AC41-42 | MQ1: 500 of 500 FTS top-10 lists equal the harness transform's at the same merged head; equality with `simulated_fts_ids` is reported as an informative row | Q8 |
| AC41-43 | MQ2: defaults unchanged on COMP-03 sweep, COMP-01 and COMP-02a | Q5 |
