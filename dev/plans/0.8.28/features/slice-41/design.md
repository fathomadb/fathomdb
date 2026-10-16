---
title: FathomDB 0.8.28 Slice 41 — design
status: phase 1 landed; phase 2 (§ 8 unit F, § 9 unit Q) drafted for review
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

## 8. Phase 2 unit F: `full_ids`

### 8.1 Mechanism

- **Policy field.** `ContextPolicy.full_ids` holds item ids; the default is
  empty. The wire field is `full_ids`, a JSON array of strings.
- **Validation.** It runs before selection, after REQ41-P8: a request that
  fails P8 gets the P8 error even when the offending id is also in
  `full_ids`. It refuses ids that match no item in `hits ∪ context`,
  repeats and empty strings, naming `policy.full_ids[i]`.
- **Tiers.** In the walk (§ 2.2 step 2):
  - `full = index < full_items || id ∈ full_ids`;
  - `entry.forced = id ∈ full_ids && index >= full_items && rendered`,
    where `rendered` means the entry's tier is not `"omitted"`. A forced
    item omitted by a spent budget or by whitespace-only text is
    `"omitted"` with `forced = false`; a forced item cut by `item_chars` or
    the budget left is `"full"` with `forced = true`.
  - Nothing else in the walk changes. Budget, `item_chars`, whitespace
    omission and ordering are as in phase 1.
- **What forcing can change (AC41-32).** Forcing raises a forced item's
  chars, and the extra chars come out of the shared budget, so items
  walked after it can be cut or omitted. Items walked before the first
  forced item, which include every item at index < `full_items`, see the
  same `remaining` as in phase 1 and are unchanged.
- **SDK types.** Python `full_ids: tuple[str, ...] = ()`; a bare `str` is a
  `TypeError`, as for `keep_roles`. TypeScript `fullIds?: readonly string[]`.
  Rust `full_ids: Vec<String>` (default empty). Provenance gains `forced`
  in every SDK.
- **Fixture.** The phase 1 expected entries in
  `src/conformance/context-pack-fixtures.json` gain `"forced": false`, as
  one mechanical edit in the RED commit (orchestrator ruling Q-T1). The
  comparators keep comparing whole objects. A scratch check that deletes
  every `"forced": false` key must reproduce the previous fixture byte for
  byte; status.md records the result.

### 8.2 Decisions

- **Forced items are added to the full tier; they do not use a
  `full_items` slot.** Variant (c) needs "rank tiers as before, plus these
  ids". Counting forced ids against `full_items` would demote a top-ranked
  item whenever a forced item exists, which no measured variant wants. A
  caller that wants a cap passes at most N ids, as (c) does with 10.
- **No reserved budget and no reordering.** A forced item late in the
  selection can still be cut. The alternative, reserving budget or moving
  forced items forward, makes the output depend on how big the forced set
  is in a way the caller cannot see from rank. Phase 1's
  selection-order rule stays the single rule. The provenance (`forced`,
  `chars`, `tier`) shows what happened.
- **Unknown ids are errors and unselected ids are no-ops.** An unknown id is
  almost always a caller bug. An id of a `context` item whose group is not
  expanded is a legitimate "if selected" request.
- **`forced` is only true when it changed the outcome.** That keeps the
  audit meaningful.

### 8.3 Harness use (harness branch, specified only)

- **(c)** `full_ids` = the ids of the hits whose group equals the rank-1
  hit's group, first 10.
- **(b)** `full_items = 8` when the question matches the
  assistant-reference patterns of `variants.py`.

Both are harness-side and labelled as heuristics in receipts.

### 8.4 Surface

The operations do not change. The `_slice41_context_pack_signature` text
gains a sentence for `full_ids`. That edits the allowlist bytes, so the pin
is re-issued, under the owner's approval of the context-pack surface "and
any recommended adjustments". Interface docs and the CHANGELOG are updated.

## 9. Phase 2 unit Q: configurable query stop words

### 9.1 Mechanism

- **In `fathomdb-query`:**
  - `QueryStopWords { base: StopWordBase, extra: Vec<String>, keep: Vec<String> }`,
    with `StopWordBase::{Fathomdb, None}`;
  - `StopWordSet`, the validated effective set plus its SHA-256 hex;
  - `StopWordSet::default()`, built from `STOPWORDS`;
  - `fn validate(&QueryStopWords) -> Result<StopWordSet, StopWordError>`.
    `StopWordError` is `Copy`: `field: &'static str`
    (`"query_stop_words.extra"` or `"query_stop_words.keep"`),
    `index: Option<usize>` (the offending word's position; `None` for a
    list-length error), and `reason: StopWordRejection`, a `Copy`,
    `#[non_exhaustive]` enum (`Empty`, `NotAlphanumeric`, `NotLowercase`,
    `TooLong`, `TooMany`, `Repeated`, `InExtraAndKeep`, `KeepNotInBase`,
    `ExtraAlreadyInBase`). The `Display` text names the field and index and
    quotes the word; the error value itself holds no `String`;
  - matching is exact on the compiler's tokens, before FTS5 stemming and
    without diacritic folding (requirements REQ41-Q2);
  - `pub fn compile_text_query_with(raw, &StopWordSet) -> CompiledQuery`;
    `compile_text_query(raw)` becomes `compile_text_query_with(raw,
    &StopWordSet::default())`, so every current caller and test is
    unchanged;
  - `pub const DEFAULT_STOP_WORDS_SHA256: &str`, with a unit test that
    recomputes it from `STOPWORDS`.
  - `sha2` is added as a dependency. It is already a workspace dependency
    of `fathomdb-engine`.
- **In `fathomdb-engine`:**
  - `EngineConfig` gains `query_stop_words: Option<QueryStopWords>`,
    validated at open with the other `EngineConfig` checks.
    `EngineConfig` is not `#[non_exhaustive]`, so a downstream struct
    literal without `..Default::default()` stops compiling. Pre-1.0 that is
    accepted, and `rust.md`'s `EngineConfig` entry says so. Three in-repo
    sites build full literals and are W4 edit sites:
    `fathomdb-napi/src/admin.rs:109` (`into_rust`) and
    `fathomdb-napi/src/test_support.rs:21` (the requested-config echo), both
    binding mappings, and `fathomdb-py/src/test_support.rs:87`, a unit test
    where adding `query_stop_words: None` is a compile-required mechanical
    change (orchestrator ruling on P2-R0b; nothing else there changes). The
    rest use `..EngineConfig::default()`;
  - `EngineConfigurationError` gains
    `QueryStopWords { field: &'static str, index: Option<usize>, reason: StopWordRejection }`,
    which keeps the enum `Copy`. Because a variant is being added anyway,
    the enum is marked `#[non_exhaustive]` now (pre-1.0, no shim), with a
    line in `rust.md` and `rust-sdk.md`. Every in-repo match on it already
    has a wildcard or `let … else` (`fathomdb-sdk/src/error.rs:319`, the
    Python and napi error mappers, the Slice 31 and Slice 90 tests), so
    nothing in the repo breaks;
  - the resolved `Arc<StopWordSet>` is held on the engine runtime and
    passed to each compile site in requirements § 5.1;
  - `OpenReport.stop_words_sha256` carries the effective hash;
  - `QueryTrace.stop_words_sha256` is filled from it.
- **Every `QueryTrace` site (W4 edit list).**
  - engine: `search_types.rs` (the struct) and `search.rs:2450` (the
    constructor);
  - napi: `fathomdb-napi/src/graph_evidence.rs:335/351` (the native
    `QueryTrace` and its conversion from the engine type), and the native
    type in `src/ts/src/binding.ts:315`;
  - Python native: `fathomdb-py/src/graph_evidence.rs:361-377`
    (`PyQueryTrace`), and the stub `src/python/fathomdb/_fathomdb.pyi:141`;
  - Python package: `src/python/fathomdb/types.py:1086` (`QueryTrace`) and
    `src/python/fathomdb/_sdk_search.py:277-305/370` (the attribute and
    JSON-pointer mappers, `/trace/stopWordsSha256`);
  - TypeScript: `src/ts/src/search.ts:287` (`QueryTrace`) and its mapper
    (`search.ts:500/561`);
  - Rust SDK: the re-exported engine type, nothing to add.
  The graph-evidence trace carries the same engine's effective hash: one
  engine has one set.
- **Docs.** The EXP-OBS field set was ratified in
  `dev/design/0.8.8-explain-and-telemetry-adr.md` (§ `QueryTrace`); that
  record gets a dated note that 0.8.28 Slice 41 adds `stop_words_sha256`,
  and the `QueryTrace` paragraphs in `python.md`, `typescript.md`,
  `rust.md` and `rust-sdk.md` list the field. The interface docs state the
  exact-token semantics.
- **In the SDKs:**
  - Python `EngineConfig.query_stop_words: QueryStopWords | None`
    (frozen dataclass: `base: Literal["fathomdb", "none"] = "fathomdb"`,
    `extra: tuple[str, ...] = ()`, `keep: tuple[str, ...] = ()`), also
    accepted as the `Engine.open` keyword;
  - TypeScript `EngineConfig.queryStopWords?: { base?, extra?, keep? }`;
  - Rust SDK `OpenOptions` / `EngineConfig` pass-through;
  - `QueryTrace.stop_words_sha256` / `stopWordsSha256` and
    `OpenReport.stop_words_sha256` / `stopWordsSha256` in every SDK;
  - `DEFAULT_STOP_WORDS_SHA256` as a package constant in Python and
    TypeScript, and re-exported by the `fathomdb` facade and `fathomdb-sdk`.
    A constant is not an operation, so the governed operation map does not
    change. The Python declaration registry
    (`src/python/tests/python_surface_additions.json`) gains the new names,
    fields and the new native stub hash, by the mechanism phase 1 used
    (status.md ruling 3). If any existing count pin moves, W4 stops and asks;
  - refusals map to `InvalidArgumentError`, as the other `EngineConfig`
    range errors do, and to `ErrorKind::InvalidArgument` in `fathomdb-sdk`.

### 9.2 Default identity

The default `StopWordSet` holds exactly `STOPWORDS`, and `content_tokens`
reads the set instead of the constant. With the setting unset, compilation
does the same work over the same words, so the output is byte-identical
(AC41-34 property). No existing test changes. The AC-038 private mirror
in `fts5_injection_safety.rs` stays as it is.

### 9.3 Open-time config, not store-persisted metadata

The setting lives on `EngineConfig`: it is given at open and not stored.

- **Persisting would need a schema change.** The store schema is pinned:
  `Engine.open` admits only `PRAGMA user_version` 34, and the SDKs have no
  automatic-upgrade route. Persisting the list needs a new table or column,
  so schema 35. Every existing store would then need an upgrade, and older
  SDKs would refuse upgraded stores. Persisting also needs a governed write
  verb to change it.
- **Nothing else needs persistence.** Stop words are query-side only, so a
  store has no index state that depends on them. Per-open configuration is
  enough for determinism once the hash is in the trace and in receipts.
- **The risk** is two processes opening one store with different lists and
  ranking differently. Every result's trace carries the list hash, and the
  COMP harness records it. The same is true of the other per-open
  `EngineConfig` knobs.
- **"One engine table".** The owner's "one engine table for cross-SDK
  parity" is met by the single `fathomdb-query` implementation, which all
  three SDKs call. No SDK keeps a copy of the list.
- **Later.** Persistence can be added, additively and with its own ruling,
  if a store-bound default is ever needed (open question Q-B1).

### 9.4 Out of scope, with reasons

- **A per-query override.** It would touch every search verb's signature in
  three SDKs (`search`, `search_text_only`, `search_projected_text`, frozen
  search, evidence search, graph-expand seeds). It would also weaken the
  "one list per opened engine" anchor that makes the hash meaningful. No
  measurement needs it: the harness sets the list per store. If it is ever
  wanted, it should be a strict subset (`extra_stop_words` for one call),
  never a replacement.
- **`fathomdb doctor --stop-words`.** A document-frequency suggester would
  not have proposed `had`, `long` or `suggest` (6–9% DF), so it does not
  serve the measured case. It would add a CLI surface and tests for an
  unmeasured need. It can be a follow-up, read-only, never writing the
  setting.

### 9.5 Composition with Slice 40 Unit H

Slice 41 is cut from Slice 40 at `aea8e0457`, before Unit H.

- **H1 (STOPWORDS).** Per the owner, Slice 40 keeps 63 words.
  Precondition for AC41-39 at the merged head: the Unit H1 failing tests
  for `had`, `long` and `suggest` (Slice 40 `ee0c5e917`) are gone. This is
  satisfied: Slice 40 reverted them at `fb42f612e` (a pure revert of
  `ee0c5e917`, 58 lines removed from `fathomdb-query/src/lib.rs`). The
  AC-038 private mirror in `fts5_injection_safety.rs` stays at 63 words.
  If an H1 engine commit landed anyway, the Q2 rule "no `extra` word
  already in `base`" would make the harness's three-word config fail
  loudly at open, not double-apply.
- **H2 (vector fusion pool).** This changes the vector branch's hydrated
  count only. Unit Q changes only MATCH compilation, so the two are
  orthogonal. A textual conflict is possible in `search_api.rs` and
  `search.rs` imports.
- **H3 (cohort prefilter).** This edits `read_projected_text_in_tx`, which
  is also a Unit Q compile site. The edits are adjacent but independent:
  Q changes the MATCH string bound into the statement, and H3 adds an
  `IN (cohort)` clause and a probe. The merge is a manual conflict
  resolution; the behaviour composes.
- **At merge.** Slice 41 rebases onto the Slice 40 head after Unit H lands.
  AC41-37 (every site drops the words) and AC41-39 (existing tests,
  including Unit H's) are re-run there. MQ1 and MQ2 are measured at that
  merged head, with the harness step REQ41-Q8 applied on the harness branch.
  MQ1's pass criterion compares two runs at that same head (requirements
  § 5.4), so it does not assume H2 and H3 leave the FTS text path
  byte-identical to `5b09afd44`.
