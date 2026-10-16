---
title: FathomDB 0.8.28 Slice 41 — design
status: DRAFT (for review before any test or code)
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
  operations next to `rerank`, mapping core errors to the SDK
  `ErrorKind` for invalid arguments.
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
ContextPack(text: str, chars_used: int, items: tuple[ContextPackEntry, ...])
ContextPackEntry(id, source: "hit"|"context", tier: "full"|"compact"|"omitted",
                 chars, truncated, rendered_index: int | None,
                 parts: tuple[ContextPartEntry(role, chars, of_chars), ...])
```

The defaults are the measured top-3/300/k30 policy. The caller picks k
through the search `limit`. If M1 (test-plan § 4.1) adopts another policy,
the defaults change to it before W4 closes. They are part of the public
contract.

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
   - Full: the part texts unchanged. Compact: kept-role parts unchanged,
     other parts cut in order from one shared allowance of
     `compact_chars`; parts cut to empty are dropped.
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

- **Tier by selection index, not hit rank.** This reproduces the prototype
  exactly, so its numbers carry over. With a large `full_items` the result
  is the Slice 40 renderer (AC41-12), so one function covers both configs
  in M1 and M3.
- **Role none is kept.** Untagged bodies (every FathomDB store today)
  degrade to the Slice 40 behaviour, not to blind truncation.
- **Headers are free.** The harness does not count `[Session date: …]`
  against the 30,000 budget. Keeping that rule keeps the measured configs
  comparable. Callers who need a hard prompt bound subtract their own
  header cost.
- **Pure, caller-supplied context.** Expansion needs a group's other
  rounds, which a hit list does not hold. Fetching them would make the
  helper an engine read with view and eligibility rules. The caller passes
  them in instead, for example `read.list` on the session's `source_id`.
- **Hit order is the caller's.** Session-diverse ordering is a reordering
  of `hits` before the call. No SDK option is added until a measurement
  justifies one (requirements § 3).
- **No `V1` suffix.** The operation is not a governed request envelope like
  `EvidenceSearchRequestV1`. Open question Q3.

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
unknown code is refused, not guessed. Encoding refuses part texts that
contain U+001E or any of the role codes.

### 3.2 Why it is FTS-neutral

`unicode61` treats only letters, numbers and private-use characters as
token characters. C0 controls are separators. A marker always sits between
`"\n"` (or the start of the body) and the part text, so it only lengthens a
separator run. The token stream, the token count per row and hence `bm25()`
are unchanged. AC41-18 checks this end to end, not by argument. The query
side is untouched: `content_tokens` splits on non-alphanumerics.

### 3.3 Why it should be embedding-neutral

The default embedder loads a Hugging Face `tokenizer.json`. BERT-family
normalizers with `clean_text` delete C0 control characters other than tab,
newline and carriage return before tokenizing. This is unverified for the
pinned model. AC41-19 checks it by comparing vector bits. Nothing in this
slice relies on it until AC41-19 passes.

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
interface docs say so and point to `decode_round_body`. Erasure, lifecycle
and source evidence treat the body as opaque bytes, as today.

## 4. COMP-03 harness use

The harness is on branch `slice/0.8.28-40-comp03` (or its successor). It
gains an evidence mode `{"pack": <ContextPolicy fields>, "item_source":
"caller" | "hits"}`:

- **`caller`:** items are built from `case["docs"]` turns: `group` = the
  session, `position` = the round, `order_key` = the session date, header
  `Session date: <date>`. The stored body stays plain, so retrieval is
  byte-identical to Slice 40 by construction.
- **`hits`:** the FathomDB arms store tagged bodies and build items with
  `context_item_from_hit`. This shows the product path end to end. It
  requires the retrieval-identity check in test-plan § 4.2.

The Slice 40 mix is expressed as a `pack` policy with `full_items = 10**6`,
so both arms of every comparison go through one renderer. The LanceDB arm
keeps the Slice 40 renderer (open question Q1).

## 5. Public surface

- `dev/interfaces/python.md`, `typescript.md` and `rust-sdk.md` get one
  section: the four operations, the types, the defaults, the scalar-value
  unit, the marker table, the neutrality guarantees and the visible
  control characters. `rust.md` notes that the engine module is pure and
  re-exported by `fathomdb-sdk`.
- `src/conformance/governed-surface-allowlist.json` and
  `governed-operation-parity.json` gain the four operations.
  `rust-sdk.md`'s operation count is updated.
- ADR check: ADR-0.8.0 (governed, curated SDK surface) and ADR-0.8.27
  (Rust SDK parity) allow additions with parity, and this slice adds them
  in all three SDKs. Vector identity (decision-index row 11) is untouched.
  No ADR is contradicted. Whether an addition of this size needs its own
  ADR row is open question Q4.
- Public docs: a short guide section in `docs/guides/` on building
  prompt context from hits, linted by `scripts/agent-lint-docs.sh`.

## 6. Files touched

- New: `fathomdb-engine/src/context_pack.rs`,
  `fathomdb-engine/tests/slice41_context_pack.rs`, the parity fixtures
  under `src/conformance/` (or the existing parity fixture directory),
  `src/python/tests/test_context_pack_parity.py` and
  `src/ts/tests/context-pack.test.ts`.
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
  add about 900 characters. This is accepted to match the harness.
