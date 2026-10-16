---
title: FathomDB 0.8.28 Slice 41 — design
status: REVISED AFTER R0 (APPROVE-WITH-FIXES, F1–F15) and the owner rulings of 2026-10-16
target_release: 0.8.28
observed_on: 2026-10-16
---

# Slice 41 design

Mechanism for [requirements.md](requirements.md). There is one pure Rust
module, exposed as package-level operations in the three SDKs, plus an
optional body convention. No engine search, write, projection or schema
code changes.

## 1. Where it lives

- **Core:** a new module `fathomdb-engine/src/context_pack.rs`. Every
  binding already depends on `fathomdb-engine`, and none depends on another
  shared crate that would suit. The module uses no SQLite and no engine
  handle. It holds the types,
  `pack_context`, `encode_round_body` and `decode_round_body`.
- **Rust SDK:** `fathomdb-sdk/src/standalone.rs` re-exports the four
  operations next to `rerank`, mapping core errors to the existing
  `ErrorKind::InvalidArgument`. The message names the field; no error
  payload field is added (R0 F1).
- **Python:** `fathomdb-py` binds the core and converts `SearchHit` for
  `context_item_from_hit`. `src/python/fathomdb` exports frozen dataclasses
  and type stubs.
- **TypeScript:** `fathomdb-napi` binds the core. `src/ts` exports the
  functions and interfaces.

One implementation is the parity mechanism: the bindings only convert
values. Three hand-written renderers would drift on character counting
(Python counts scalar values, JavaScript counts UTF-16 units, Rust slices
bytes).

## 2. `pack_context`

### 2.1 Types (Python spelling; TypeScript camelCase, Rust snake_case)

```text
ContextPart(role: str | None, text: str)
ContextItem(id: str, parts: tuple[ContextPart, ...], group: str | None = None,
            position: int | None = None, order_key: str | None = None,
            header: str | None = None)
ContextPolicy(budget_chars=30000, item_chars=4000, full_items=3,
              compact_chars=300, keep_roles=("user",),
              expand_groups=True, order="rank")
ContextPack(text: str, chars_used: int, text_chars: int,
            items: tuple[ContextPackEntry, ...])
ContextPackEntry(id, source: "hit"|"context", tier: "full"|"compact"|"omitted",
                 chars, truncated, rendered_index: int | None,
                 parts: tuple[ContextPartEntry(role, chars, of_chars), ...])
```

The defaults (top 3 full, 300-character excerpt, keep user text) are a
product choice, fixed now. They are part of the public contract. The caller
picks k through the search `limit`. The M1 rule picks the COMP-03 harness
policy, which the harness passes explicitly. M1 does not change these
defaults (R0 F7). A single-corpus argmax such as `compact_chars = 0` would
fit LongMemEval's 94% user-turn share, not agents in general. The types
are plain value types (R0 Q3).

### 2.2 Algorithm

1. Validate (REQ41-P8). Build the selection: the hits in input order, then,
   if `expand_groups`, for each distinct hit `group` in order of first
   appearance, the `context` items of that group whose id is not yet
   selected, sorted by `position` with input order as tie-break. Context
   items in other groups, or with no group, are never selected; they are
   `"omitted"`.
2. Walk the selection with `remaining = budget_chars` and index `i`. Stop
   when `remaining == 0`; every later item is `"omitted"`.
   - The tier is full if `i < full_items`, else compact.
   - Full: every part text unchanged, empty parts included. Compact:
     kept-role and role-none parts unchanged, empty parts included. Other
     parts are cut in order from one shared allowance of `compact_chars`
     (join characters are not charged), and a non-kept part that renders 0
     characters is dropped (R0 F6).
   - Item text = the parts joined by `"\n"`. If it is whitespace-only, the
     item is `"omitted"`, `chars = 0`, and `i` still advances. This matches
     the harness, where an empty round keeps its slot.
   - `chars = min(len(text), item_chars, remaining)`. Cut the text to
     `chars` scalar values and record per-part `chars` by walking the cut
     across the parts and their `"\n"` joins. A join character is counted
     in the item, not in a part.
   - `remaining -= chars`.
3. Order the rendered items: selection order, or for `"chronological"` the
   stable sort in REQ41-P5.
4. Render (REQ41-P9) and set `rendered_index`.

`truncated` compares each part's rendered characters with its full length.
A compacted assistant part is therefore truncated even when the budget is
not the cause; `tier` tells the caller which rule cut it.

### 2.3 Decisions

- **Tier by selection index, not hit rank.** This follows the prototype.
  With a large `full_items` the result is the Slice 40 renderer (AC41-12,
  AC41-27), so one function covers both configs in M1 and M3.
- **Known differences from the prototype (R0 F6).** The prototype puts the
  joined user text first and the joined non-user text second, and renders
  `u + "\n" + a[:A]` even when `a` is empty. `pack_context` keeps per-turn
  parts in their original order, drops an empty non-kept part, and does not
  charge join characters to the allowance. Compact items can therefore
  differ by a few characters, and assistant-first rounds differ in order.
  The report's numbers are a guide only. M1 re-measures everything through
  `pack_context`.
- **Role none is kept.** Untagged bodies (every FathomDB store today)
  degrade to the Slice 40 behaviour, not to blind truncation.
- **Headers are free (R0 Q6).** The harness does not count
  `[Session date: …]` against the 30,000 budget. Keeping that rule keeps
  the measured configs byte-comparable. `budget_chars` bounds item text
  only. `text_chars` gives the full length, so a caller who needs a hard
  prompt bound can check it or subtract their own header cost.
- **Typed error without a payload (R0 F1).** No SDK error carries a
  `field_path` today. Adding one would change the error envelope in every
  binding. The message names the field instead.
- **Pure, caller-supplied context.** Expansion needs a group's other
  rounds, which a hit list does not hold. Fetching them would make the
  helper an engine read with view and eligibility rules. The caller passes
  them in instead, for example `read.list` on the session's `source_id`.
- **Hit order is the caller's.** Session-diverse ordering is a reordering
  of `hits` before the call. No SDK option is added until a measurement
  justifies one (requirements § 3).
- **No `V1` suffix (R0 Q3).** The operation is a pure function over value
  types, not a governed request envelope like `EvidenceSearchRequestV1`.

## 3. Role-tagged body convention

### 3.1 Format

A tagged body is the parts in order, each written as `MARK + text`, joined
by `"\n"`. `MARK` is two C0 control characters: U+001E (record separator)
followed by a role code:

| Role | Code |
| --- | --- |
| none | U+0010 |
| `user` | U+0011 |
| `assistant` | U+0012 |
| `system` | U+0013 |
| `tool` | U+0014 |

Other roles are refused. `decode_round_body` treats a body that does not
start with U+001E as untagged: one part, role none, the whole body. It
splits a tagged body on `"\n" + U+001E`. A tagged body with a missing or
unknown code is refused, not guessed. `context_item_from_hit` therefore
raises on such a body instead of returning a guess. Encoding refuses part
texts that contain U+001E or any of the role codes.

The convention is opt-in: nothing writes it unless the caller encodes. By
owner ruling (2026-10-16) it ships opt-in, with its hazards documented
(§ 3.5) and a decision-index row (§ 5), and it can be revisited.

### 3.2 Why it is FTS-neutral

`unicode61` treats only letters, numbers and private-use characters as
token characters. C0 controls are separators. A marker always sits between
`"\n"` (or the start of the body) and the part text, so it only lengthens a
separator run. The token stream, the token count per row and hence `bm25()`
are unchanged. This holds for every FTS table (`search_index`,
`search_index_v2`, `search_index_edges`, `property_search_index`), and for
BM25F field length, which counts tokens (`search.rs` `fts5_tokenize`). The
query side is untouched: `content_tokens` splits on non-alphanumerics.
R0 checked this empirically with SQLite FTS5: token offsets were identical
and `bm25()` was bit-identical. AC41-18 checks it in the engine.

### 3.3 Why it should be embedding-neutral

The default embedder loads a Hugging Face `tokenizer.json`. R0 found that
both cached tokenizers (the default model and `nomic-v1.5`) use
`BertNormalizer` with `clean_text: true`. That deletes C0 control
characters other than tab, newline and carriage return before
pre-tokenization, and the engine applies no character cap before the
tokenizer. AC41-19 checks the result by comparing vector bits, including a
body over the 512-token window. Nothing in this slice relies on embedding
neutrality until AC41-19 passes.

### 3.4 If a neutrality check fails

- AC41-18 fails: the convention is dropped from the slice. `pack_context`
  still ships, and callers build parts from their own data, as the
  COMP-03 harness can.
- AC41-19 fails: the convention is documented as FTS-only. The COMP-03
  hybrid arm builds items from caller data, not tagged bodies.

Neither fallback touches the engine. Stripping markers before embedding
would be an engine change and is out of scope.

### 3.5 Visible effect

Callers that read a tagged body directly see control characters. The
interface docs list these hazards (REQ41-B6, R0 F9) and point to
`decode_round_body`:

- Python `str.splitlines()` splits on U+001E (and U+001C, U+001D). A
  consumer that splits a tagged body into lines, as Memex may, gets the
  role code glued to the next line and the part boundary turned into a
  line break.
- U+0011 and U+0013 are XON and XOFF. They are harmless in output but
  surprising in logs and copy-paste.
- C0 controls other than tab, newline and carriage return are illegal in
  XML 1.0.
- JSON escapes them (`\u001e`), and `serde_json` escapes all C0, so the
  N-API, pyo3 and CLI JSON paths are safe.
- Slice 50 `resolve_evidence` computes spans on the stored bytes, so spans
  stay consistent, but resolved evidence text contains the markers.
- Erasure and lifecycle treat the body as opaque bytes, as today.

## 4. COMP-03 harness use

The harness is on branch `slice/0.8.28-40-comp03` (or its successor). It
gains an evidence mode `{"pack": <ContextPolicy fields>, "item_source":
"caller" | "hits"}`:

- **`caller`:** `case["docs"]` holds only `roles` and the joined `body`,
  and a turn containing a newline cannot be split back out of it (R0 F5).
  So `caller` mode re-derives one part per turn, in original order, from
  the dataset, as `compact.py` `round_parts` does. The dataset is pinned by
  its SHA-256, as in REQ40-G1. Item fields: `group` = the session,
  `position` = the round, `order_key` = the session date, header
  `[Session date: <date>]` (with brackets, as `render_evidence` writes it).
  Per-turn parts joined by `"\n"` equal the stored body (REQ41-B2). The
  stored body stays plain, so retrieval is byte-identical to Slice 40 by
  construction.
- **`hits`:** the FathomDB arms store tagged bodies and build items with
  `context_item_from_hit`. This shows the product path end to end. It
  requires the retrieval-identity check in test-plan § 4.2.

The Slice 40 mix is expressed as a `pack` policy with `full_items = 10**6`,
so both arms of every comparison go through one renderer. AC41-27 checks
that this policy reproduces every recorded comp03-run-mix
`evidence_sha256`. Those records are owner-accepted Slice 40 artifacts, so
this is a differential check against existing data, not a new oracle.

By owner ruling (2026-10-16, R0 Q1): the LanceDB arm keeps the Slice 40
renderer in M3, and every table names each arm's renderer. A LanceDB-plus-pack arm
is deferred to the next COMP-03 headline refresh.

## 5. Public surface

- `dev/interfaces/python.md`, `typescript.md` and `rust-sdk.md` get one
  section: the four operations, the types, the defaults, the scalar-value
  unit, the marker table, the neutrality guarantees and the visible
  control characters. `rust.md` notes that the engine module is pure and
  re-exported by `fathomdb-sdk`.
- `src/conformance/governed-operation-parity.json` gains the four
  operations, and `rust-sdk.md`'s operation count is updated.
- `src/conformance/governed-surface-allowlist.json` is HITL-signed: any
  diff re-opens the gate (R0 F2). The owner approved adding the four
  operations, with the R0 adjustments, on 2026-10-16. The entry is still
  written at W4, not now, and cites that ruling. The signature text to
  write is:

  > `_slice41_context_pack_signature`: the 0.8.28 Slice 41 context pack
  > adds four package-level pure operations in each binding:
  > `pack_context`, `encode_round_body`, `decode_round_body` and
  > `context_item_from_hit` (TypeScript `packContext`, `encodeRoundBody`,
  > `decodeRoundBody`, `contextItemFromHit`). They render caller-supplied
  > items within a character budget and encode or decode an opt-in
  > role-tagged body. They open no engine and read or write no database.
  > Core membership and the recovery denylist are unchanged. Approved by
  > the owner on 2026-10-16, with the R0 plan-review adjustments.
- ADR check: ADR-0.8.0 (governed, curated SDK surface) and ADR-0.8.27
  (Rust SDK parity) allow additions with parity, and this slice adds them
  in all three SDKs. Vector identity (decision-index row 11) is untouched.
  No ADR is contradicted. The four operations need no ADR; they need the
  signed allowlist delta, the parity-map rows, the interface docs and the
  CHANGELOG (R0 Q4).
- Owner ruling (2026-10-16, R0 Q4, Q5): the role-tagged body is a
  stored-bytes format in user databases. It cannot be withdrawn without a data
  migration, which AGENTS.md § 10 forbids. It therefore gets a row in
  `dev/adr/ADR-0.6.0-decision-index.md` covering the markers, the
  neutrality guarantees, opt-in use and the § 3.4 fallbacks, marked "can
  be revisited". The row is written at W4. A one-page ADR is added only if
  the owner wants the reasoning on file.
- Public docs: a short guide section in `docs/guides/` on building
  prompt context from hits, linted by `scripts/agent-lint-docs.sh`.

## 6. Files touched

- New: `fathomdb-engine/src/context_pack.rs`,
  `fathomdb-engine/tests/slice41_context_pack.rs`,
  `fathomdb-embedder/tests/slice41_marker_embedding.rs`,
  `src/conformance/context-pack-fixtures.json` (no cross-SDK operation
  fixture directory exists yet),
  `src/python/tests/test_context_pack_parity.py` and
  `src/ts/tests/context-pack.test.ts`.
- Edited at W4, citing the 2026-10-16 owner rulings:
  `src/conformance/governed-surface-allowlist.json` and
  `dev/adr/ADR-0.6.0-decision-index.md`.
- Edited: `fathomdb-engine/src/lib.rs` (module and re-export), the
  `fathomdb-sdk` standalone module and `lib.rs`, the `fathomdb-py` and
  `fathomdb-napi` binding modules, the Python package exports and stubs,
  the TypeScript exports and types, the conformance maps, the interface
  docs, one guide page and `CHANGELOG.md`.
- Harness, on the comp branch: the `pack` evidence mode, configs and the
  M1 evaluator.

## 7. Risks

- **Defaults chosen on one benchmark.** The 94% user-turn share is a
  LongMemEval property. Agents whose answers sit in assistant or tool text
  must set `keep_roles`. The docs say this, and the SSA guard keeps the
  measured loss visible.
- **Prefix property.** k = 30 hybrid is not the top 30 of a k = 100
  ranking until Unit H decouples the vector pool. M1 retrieves at the
  config's own k, not by slicing.
- **Parity surface growth.** Four operations and six types per SDK. The
  single core keeps behaviour parity cheap. The surface-parity checker
  covers names and signatures.
- **Header cost not budgeted.** About 30 characters per item; 30 items
  add about 900 characters. This is accepted to match the harness, and
  `text_chars` exposes the total.
