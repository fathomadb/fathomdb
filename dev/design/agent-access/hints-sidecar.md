---
title: Hints sidecar for agents (SDK and MCP)
date: 2026-10-18
status: PROPOSED
desc: Design for a FathomDB-authored `hints` sidecar of suggested next actions — vocabulary, where each hint is computed (engine/SDK versus MCP adapter), cost budget, configuration, budget accounting, the Block 2 rows B2-1..B2-6, public-surface impact, the RED test plan and a draft ADR.
inputs:
  - dev/design/agent-access/README.md
  - dev/experiments/mcp-agent-loop/README.md
  - dev/design/mcp-adapter/architecture.md
  - dev/design/mcp-adapter/requirements.md
  - dev/interfaces/mcp.md
  - dev/interfaces/rust-sdk.md
  - dev/design/0.8.8-explain-and-telemetry-adr.md
  - dev/design/perf-gates.md
  - src/rust/crates/fathomdb-mcp/src/{tools,shape,output,error,edge,session,config,registry}.rs
  - src/rust/crates/fathomdb-engine/src/{search_types,search_api,search}.rs
  - src/rust/crates/fathomdb-sdk/src/{lib,engine,options}.rs
---

# Hints sidecar for agents (SDK and MCP)

Work item 2 of [README.md](README.md), and the design home of Block 2 rows
B2-1..B2-6. This is a design only. No code or test is written until the owner's
HITL review is done. The Fable-medium review (APPROVE-WITH-FIXES) is folded in.

Paths are relative to `src/rust/crates/` unless given in full. Line numbers
are at `9ea33ff19`.

## 1. Problem

In the LoCoMo MCP runs, agents acted on explicit instructions but not on flags:

- They ignored `snippet_cut: true` and rarely fetched. They answered from
  prefix snippets, even when the record held the answer (README § Why this
  exists). `snippet_cut` is one boolean among eight fields per hit
  (`fathomdb-mcp/src/output.rs:175-187`), inside a result with eleven
  top-level fields (`output.rs:238-256` plus the flattened envelope,
  `output.rs:48-56`).
- When the guidance was explicit ("fetch … cut off"), the guided gpt-4o-mini
  arm fetched on 147 of 150 questions, up from 0
  (`dev/experiments/mcp-agent-loop/README.md:416`, `:470-471`).
- `state: {status: "unavailable"}` is in every envelope (`output.rs:53`), and
  `verdict: "unavailable"` is on every search (`output.rs:247`,
  `tools.rs:780`). Haiku flagged them as a failure (README row B2-1).
- Guided gpt-5.4-mini called `list_stores` on 92 of 150 questions although a
  default store was set (`mcp-agent-loop/README.md:480-488`).
- Agents passed `limit` to `fetch` five times and sent `feedback` with 12
  refs against a 10-result response (`mcp-agent-loop/README.md:428-430`).

Prior art points the same way. Anthropic's tool-writing guidance says: "If you
choose to truncate responses, be sure to steer agents with helpful
instructions", and suggests error responses that "clearly communicate specific
and actionable improvements"
(<https://www.anthropic.com/engineering/writing-tools-for-agents>). REST's
uniform interface makes the response carry the next available transitions
("hypermedia as the engine of application state", Fielding 2000 § 5.1.5,
<https://ics.uci.edu/~fielding/pubs/dissertation/rest_arch_style.htm>). The
hints sidecar applies both ideas: each response carries the next calls worth
making, in a form an agent can execute without inference.

## 2. What a hint is

A hint is a suggested next action authored by FathomDB. It is a typed record,
not prose:

```json
{"action": "fetch", "reason": "snippet_cut", "refs": ["l:conv0-s1-t3", "l:conv0-s2-t7"]}
{"action": "search", "reason": "more_results", "continuation": "c1-9f2c4a1e0b7d3e55"}
{"action": "search", "reason": "no_results", "mode": "hybrid"}
{"action": "none", "reason": "degraded", "arm": "vector", "cause": "embedder_unavailable"}
```

Invariants:

- **H1. FathomDB-authored, data-free.** Every string in a hint is one of
  these: a closed-enum word, an adapter-minted handle (`continuation`), an
  operator-configured store name, or a `ref` copied from the same response's
  `results[].ref`, `items[].ref` or `targets[].ref`. A hint never carries
  record content, kind names, `source_id`, or text from SDK error messages.
  This keeps the data-not-instructions boundary of
  `dev/design/mcp-adapter/architecture.md:388-393` and MR-012
  (`requirements.md:181-192`). See § 2.3 for refs.
- **H2. Separate from content.** Hints live in their own `hints` field, never
  inside `results`, `items`, `targets` or a record field.
- **H3. Derived, not fetched.** A hint is computed only from values the call
  already holds. No extra query, read, embed or lock (§ 4).
- **H4. Deterministic.** The same request on the same state with the same
  hints setting gives the same hints, byte for byte, so MR-043 holds
  (`requirements.md:405-415`).
- **H5. Advisory, with one stated side effect.** Ignoring every hint never
  makes a result wrong, and hints never replace a typed field (`snippet_cut`,
  `continuation`, `completeness` and `degradation` stay). Hints do share the
  response budget, so at the budget margin, turning hints on can keep fewer
  hits. § 5.1 states this exactly.

### 2.1 Two layers, one wire vocabulary

The engine owns a small shared subset. The adapter owns the wire type that
agents see, which adds its own reasons (§ 3). The **one** vocabulary table for
agents lives in `dev/interfaces/mcp.md`. `dev/interfaces/rust.md` documents
only the engine subset.

| `reason` | `action` | Fields | Owner | When |
| --- | --- | --- | --- | --- |
| `no_results` | `search` | `mode?` | engine subset | zero hits returned (B2-2) |
| `degraded` | `none` | `arm`, `cause` | engine subset (adapter adds `embedder_unavailable`) | a degradation entry is present (B2-3) |
| `more_results` | `search` | engine: `limit`; adapter: `continuation` | engine subset; adapter converts | the page reached its limit and more may exist |
| `snippet_cut` | `fetch` | `refs`, `refs_omitted?` | adapter | `search` hits with `snippet_cut: true` |
| `body_cut` | `fetch` | `refs`, `refs_omitted?`, `max_tokens` | adapter | `fetch` items or `expand` targets with `body_cut: true` |
| `cut_by_budget` | `search` | `continuation`, `max_tokens` | adapter | hits were dropped to fit `max_tokens` |
| `default_store_set` | `search` | `store` | adapter, `list_stores` only | the session has a default store (B2-6) |

Words reserved for later versions, not emitted in v1:

- `weak_results` needs a calibrated verdict (MR-031, B2-2).
- `index_not_ready` waits for a real projection-readiness signal (§ 6 B2-2
  explains why `empty_not_ready` is not one).
- `retry` (an action) belongs to item 1's queue (B2-7).

Closed field value sets in v1:

- `action`: `fetch`, `search`, `none`.
- `mode`: `text`, `hybrid`.
- `arm`: `vector`, `text`, `text_edge`, `graph_arm`, `graph`. These are the
  arms the adapter already reports (`tools.rs:283-290`, `tools.rs:1042-1046`).
- `cause`: `soft_fallback`, `dense_disabled`, `embedder_unavailable` (the
  adapter's existing degradation reasons, `tools.rs:668-688`), plus the
  `graph` codes of `tools.rs:947-958`.

Versioning:

- There is no per-response version field. The vocabulary version is the
  `dev/interfaces/mcp.md` revision, and the output schema lists the closed
  enums.
- A new `reason`, `action` or field is additive, and clients must ignore a hint
  whose `action` or `reason` they do not know. The interface states this rule
  once.
- Changing what an existing reason means is breaking and needs an interface
  update in the same change (`architecture.md:432-443`).

### 2.2 Bounds

- At most one hint per reason. The reachable maximum is three hints on
  `search` (`degraded`, `no_results` or `snippet_cut`, and `cut_by_budget` or
  `more_results`), one on `fetch`, two on `expand` and one on `list_stores`.
  No separate count cap is needed. Emission order is fixed: `degraded`,
  `no_results`, `snippet_cut`, `body_cut`, `cut_by_budget`, `more_results`,
  `default_store_set`.
- `cut_by_budget` and `more_results` exclude each other, because both point at
  the same continuation.
- `snippet_cut` and `body_cut` list **every** cut ref in result order, up to
  `MAX_FETCH_REFS` (50, `tools.rs:40`), so the hint is one executable `fetch`
  call. A search page has at most 100 hits (`tools.rs:36`).
- `refs_omitted` counts only refs beyond 50, or refs left out by the
  128-byte rule. A ref longer than 128 bytes is left out of hints but still
  appears in `results`.
- Size: in the typical LoCoMo case (10 hits, ~25-byte refs, all cut), the
  fetch hint is about 0.3 KB. The worst case is 50 refs of 128 bytes, about
  6.6 KB. That case has to fit the response budget like any other field
  (§ 5.1).

### 2.3 Refs inside hints and the trust boundary

Logical ids are set by writers, not by FathomDB, so an id can be any string.
An id inside an imperative-looking hint carries more authority than the same id
in `results[].ref`. Mitigations:

- Hints copy only refs that already appear in the same response, so they
  expose no new data.
- In the text block, every ref is JSON-quoted with `shape::quote`
  (`fathomdb-mcp/src/shape.rs:150-155`), in hint lines **and** in hit lines.
  Hit lines print `ref` unquoted today (`tools.rs:534-537`), and this design
  changes that (the reviewer's resolution of open question 5).
- The 128-byte cap bounds what an adversarial id can carry inside a hint.
- Each hint's text line comes from a compile-time template per `(action,
  reason)`. No hint text is built from data.

## 3. Placement: what is computed where

### 3.1 Constraint found in code

The Python and TypeScript bindings do not sit over `fathomdb-sdk`. They depend
on `fathomdb-engine` directly (`fathomdb-py/Cargo.toml:43-46`,
`fathomdb-napi/Cargo.toml:29-32`). Only `fathomdb-mcp` depends on
`fathomdb-sdk` (`fathomdb-mcp/Cargo.toml:21`). `fathomdb-sdk` re-exports the
engine's `SearchResult` as is (`fathomdb-sdk/src/lib.rs:105`), and
`Engine::search` returns it unchanged (`fathomdb-sdk/src/engine.rs:185-211`).
So any rule that all three SDKs share has to live in `fathomdb-engine`.

### 3.2 Decision

**Engine subset: `fathomdb-engine/src/hints.rs`, a pure module with no SQL,
I/O, clock or locks.**

- Types: `Hint`, `HintAction`, `HintReason` (`NoResults`, `Degraded`,
  `MoreResults`) and `HintCause`. Every enum is `#[non_exhaustive]`.
- One function:

  ```rust
  pub fn search_hints(facts: &SearchFacts, out: &mut Vec<Hint>);
  // SearchFacts: returned hit count, limit, mode (text | hybrid),
  // degradation entries (arm, cause), hybrid_available: bool.
  // No continuation, snippet, budget or session concept.
  ```

- It emits `no_results{mode?}`, `degraded{arm, cause}` and
  `more_results{limit}`, where `limit` is the smaller of 2 × limit and 100.
- `fathomdb-sdk` re-exports the types, as it re-exports `Explanation`
  (`lib.rs:85`).

**Adapter wire type: `fathomdb-mcp/src/output.rs`.**

- `output.rs` owns the serialized `Hint` that agents see, with
  `impl From<fathomdb_sdk::Hint>`. The conversion is where the adapter
  rewrites `more_results{limit}` into `more_results{continuation}`, attaching
  its own handle (`tools.rs:734-737`). With no continuation, the adapter drops
  that hint. With a budget cut, it emits `cut_by_budget` instead.
- The adapter adds its own reasons, which need facts only it has:

| Reason | Why it is adapter-only |
| --- | --- |
| `snippet_cut` | Snippets are adapter presentation. The SDK returns whole bodies (`search_types.rs:95`), and the adapter cuts them at `snippet_chars` (`tools.rs:701-709`, `shape.rs:13-20`). |
| `body_cut` | Body cuts come from the adapter's budget fitting (`tools.rs:931-944`, `tools.rs:1088-1101`). |
| `cut_by_budget`, continuation on `more_results` | Continuations and the budget are adapter state (`tools.rs:732-804`, `shape.rs:36-48`). |
| `degraded` / `embedder_unavailable` | The adapter reads embedding readiness once per open epoch (`registry.rs:696-698`, used at `tools.rs:682-687`) and passes it in as a degradation fact. |
| `default_store_set` | Sessions are adapter state (`session.rs:89-104`). |

- The adapter calls `search_hints` itself, with page facts, rather than
  reading an SDK carrier. It asks the engine for `offset + limit` hits and
  skips `offset` (`tools.rs:637`, `tools.rs:706`), so the SDK's "limit
  reached" is not the page's "limit reached". The same function serves both
  layers with different facts, so the rule cannot drift.

**SDK carrier (stage 2).**

- The three SDK search verbs (`search`, `search_text_only`, `search_frozen`)
  gain an option `hints: bool`, default `false`.
- When the option is true, the engine fills a new sidecar,
  `SearchResult.hints: Option<Vec<Hint>>`, at the four construction sites
  (`search_api.rs:668`, `search_api.rs:1084`, `search.rs:280`,
  `search.rs:824`), from the engine subset only.
- This follows the HITL-ratified explanation-sidecar precedent: an `Option` on
  `SearchResult`, `None` when off, and `SearchHit` untouched
  (`dev/design/0.8.8-explain-and-telemetry-adr.md:42-56`). `SearchResult` is
  `#[non_exhaustive]` (`search_types.rs:137`), so the field is additive in
  Rust.
- The Python and TypeScript bindings map it the way they map `explanation`
  (`fathomdb-napi/src/read_search.rs:688-714`;
  `src/python/fathomdb/types.py:1169-1179`).

**Staging.**

- Stage 1 (Block 1) adds the pure engine module and the adapter wiring. No
  engine search path changes.
- Stage 2 adds the SDK option and carrier across the three SDKs.
- Both stages fall under the one ADR in § 11.

### 3.3 Hints per tool

| Tool | Hints (v1) |
| --- | --- |
| `search` | `degraded`, `no_results`, `snippet_cut`, then `cut_by_budget` or `more_results` |
| `fetch` | `body_cut`: the refs of the cut items. Its `max_tokens` is the current estimate plus the bytes cut ÷ 4 × 2, because each body appears in both the structured content and the text block (`shape.rs:166-169`). |
| `expand` | `body_cut` (the refs of the cut targets), `degraded` (graph codes) |
| `list_stores` | `default_store_set` (B2-6) |
| `orient`, `use_store`, `feedback` | none (`orient` gets a plain field instead, § 6 B2-1) |

**When `no_results` carries `mode: "hybrid"`.** It does so only when all of
these hold:

1. The call ran in text mode.
2. The store has a usable embedder: `answered.info.embedder_usable`
   (`tools.rs:682`), set from `read::embedding_readiness` at open
   (`registry.rs:696-698`).
3. The configuration permits hybrid. For an adapter-opened store, that means
   `use_default_embedder` is true, which `search_mode: hybrid` requires
   (`config.rs:257-259`). For a hosted engine, condition 2 decides it.
4. `run.dense_disabled` is false, as read after the text run
   (`tools.rs:663`).

`dense_disabled` can still turn on during the suggested hybrid run (only that
run's result shows it), so the suggestion stays advisory. A hybrid run that
then degrades reports `degraded`. When the conditions do not hold, `no_results`
carries no `mode`, and its template says "rephrase with synonyms, names or
related events". That wording already exists in the guide (`edge.rs:35-37`).
In the engine subset, the caller supplies the result of conditions 2–4 as
`hybrid_available`.

## 4. Cost: laser-fast

### 4.1 Mechanism

- **Off means nothing is allocated.** The adapter output gets
  `#[serde(skip_serializing_if = "Vec::is_empty")] hints: Vec<Hint>`. An empty
  `Vec::new()` does not allocate. The SDK carrier is `None`. With hints off,
  every output is byte-identical to today's (except the B2 changes, § 6).
- **Planned once, sliced per iteration.** The search budget loop re-renders and
  re-serializes the page each time it drops a hit (`tools.rs:731-791`).
  Hints are therefore planned once, before the loop, as a fixed plan: the
  indices of the cut hits in rank order, plus the engine-subset hints. The
  plan uses the `snippet_cut` flags already computed (`tools.rs:709`). Each
  iteration builds its output from the plan's prefix of indices below `kept`.
  The cost per iteration is one `Vec` for the hint list and one `String`
  clone per emitted ref. It is not per response, because the loop rebuilds the
  whole `SearchOutput` each iteration (`tools.rs:759-786`). An implementation
  may serialize borrowed refs to avoid the clones. No hint needs a second pass
  over bodies.
- **No new reads.** Every input is already in scope: `kept`, `page_hits`,
  `page_limit`, `degradation`, `mode`, `answered.info`, `continuation`, the
  `snippet_cut` and `body_cut` flags, `run.dense_disabled`, and
  `session.default_store()`.
- **Error-path data only on error.** B2-4 and B2-5 build their details only
  when the call is already failing (§ 6).

### 4.2 Budget

| Measure | Bound | Disposition |
| --- | --- | --- |
| Added time per `search` shaping at `limit` 100, hints on against off | ≤ 200 µs | **BLOCK** (catastrophic ceiling) |
| Added time at `limit` 10 | ≤ 10 µs p50, ≤ 25 µs p99 | notify-only |
| Allocations, hints off against a no-hints baseline | 0 | **BLOCK** |
| Allocations, hints on against off | ≤ (1 + emitted refs) × budget-loop iterations | **BLOCK** |
| End-to-end p50, on against off | max(10 µs, 1% of search p50) | report-only |

- The end-to-end rule is report-only because it sits below run-to-run noise.
- For scale, the binding 10k-row text fixture's p50 budget is 20 ms
  (`dev/acceptance.md:1172`).
- Tokens: in the typical case, a search with 10 cut hits adds about 0.3 KB per
  form (structured and text), about 150 estimated tokens. That is the price of
  replacing ten ignored booleans with one executable call. § 5.1 explains how
  the budget absorbs it.

### 4.3 How it is measured (design, not code)

The workspace has no `criterion` benches (no `criterion` in any
`src/rust/**/Cargo.toml`). No long-run runner exercises `fathomdb-mcp`, so an
`AGENT_LONG`-only gate would pass without ever running. The gate therefore
follows the devloop tier of `dev/design/perf-gates.md`: an always-on test
with a blocking catastrophic ceiling and notify-only soft bounds
(`perf-gates.md:76-96`).

1. **Allocation delta test (always on, BLOCK).** A test binary of its own
   with a counting `#[global_allocator]`. A global allocator cannot attribute
   allocations to a module, so the test measures deltas around one call of the
   shaping function on a fixed 100-hit synthetic page. It runs three ways —
   no-hints baseline, hints off, and hints on — and asserts
   `off − baseline == 0` and `on − off ≤ (1 + refs) × iterations`. The
   iteration count is reported by the shaping function's test seam.
2. **Devloop timing (always on, `cargo test`).** A synthetic 10- and 100-hit
   page with bodies of 0.2–4 KB, half over `snippet_chars`. It runs 31
   trials, interleaved off/on, and takes the median of the trial medians. It
   blocks only on the 200 µs ceiling at `limit` 100. The 10 µs and 25 µs
   bounds print a WARN line.
3. **End to end (report-only).** An in-process MCP client (the existing
   protocol-test harness) against the LoCoMo conv0 store and the 10k-row text
   fixture: 2,000 `search` calls per arm, interleaved on/off, over the same 50
   queries. It prints p50 and p99. The numbers go on the board.
4. **Registration.** The same change that adds these tests adds a row to the
   per-AC gate map in `dev/design/perf-gates.md`:
   `hints overhead (MCP shaping) | fathomdb-mcp/tests/hints_overhead.rs |
   per-push | synthetic | BLOCK ≤200 µs @ limit 100 + alloc deltas; 10/25 µs
   notify-only; E2E report-only`. The row is not added now: a gate row for a
   test that does not exist would be stale (AGENTS.md § 1).

## 5. Configuration

- **Per process.** A new config key `hints` (bool, default `true`) in
  `dev/interfaces/mcp.md` § 2. It joins `TOP_LEVEL_KEYS`
  (`fathomdb-mcp/src/config.rs:140-155`), because unknown keys are invalid.
- **Per call.** A new optional argument `hints` (bool) on every tool. All
  argument structs use `deny_unknown_fields` (`tools.rs:72-203`), so it is
  declared on each. A continuation call may also pass it, which widens the
  rule at `tools.rs:566-582`. Hints are not part of `CanonicalSearch`
  (`shape.rs:113-125`), so turning them on or off never changes the
  continuation handle's hash.
- **Precedence.** Per call, then per process, then built in. There is no
  operator lock.
- **SDK.** The `hints: bool` option on the three search verbs defaults to
  `false`.

**Recommendation: on by default for MCP, off by default for the SDKs.**

- MCP callers are agents, and the evidence in § 1 is that agents act on
  explicit actions and skip flags. The cost is bounded (§ 4.2), and off is one
  argument away.
- SDK callers are mostly programs that already read typed fields. Off keeps
  every SDK result byte-identical, matching the explanation sidecar's
  zero-cost contract (R-OBS-2, `0.8.8-explain-and-telemetry-adr.md:46-56`).
  It also leaves the existing SDK parity and golden suites untouched.

### 5.1 Budget accounting (bytes ÷ 4) and its one side effect

- Hints are part of the structured content and the text block, so
  `shape::with_budget` counts them like any other field (`shape.rs:36-48`,
  `dev/interfaces/mcp.md:76`). There is no separate allowance.
- **Side effect, stated honestly.** In `search`, the budget loop drops trailing
  hits until the whole response fits (`tools.rs:731-791`). Hints add bytes, so
  near the margin a hints-on response can keep fewer hits than a hints-off
  one. A different `kept` changes `results`, `response_id` (hashed over the
  kept refs, `tools.rs:738-744`), `continuation` (hashed over the next offset,
  `tools.rs:734-737`), and the stored continuation offset (`tools.rs:793-803`).
- **What stays true.** Each response is internally consistent: its
  continuation resumes exactly at `offset + kept` of that response, and
  following continuations still returns every hit once, in order. This is the
  property `shape.rs` already tests for any kept count (`shape.rs:302-329`).
  Hints stay out of `CanonicalSearch`, so the handle does not depend on the
  hints setting except through `kept`.
- The same wording goes into `dev/interfaces/mcp.md` § 3: "`hints` counts
  against `max_tokens`; with hints on, a page may keep fewer results."
- Hints are emitted from `page[..kept]`, so dropping a hit also drops its ref
  from the hint. The loop's exit rule stays (`tools.rs:787-790`). With
  `kept == 0` there is no continuation (`shape.rs:97-101`).
- In `fetch` and `expand`, the `body_cut` hint is part of what
  `shape::cut_bodies` sizes (`tools.rs:931-935`, `tools.rs:1088-1092`). It is
  re-emitted from the current cut flags on each sizing step, and the ref list
  only grows as more bodies are cut. If the hint's own bytes stop the response
  from fitting, the existing `fetch` refusal applies (`tools.rs:939-943`).
- Text rendering: one line per hint after the header line and before the hits,
  for example `hint: fetch ["l:a","l:b"] (snippet_cut: snippets are prefixes;
  fetch to read the full record)`.

## 6. Block 2 rows as vocabulary or field entries

Block 2 is implemented only after Block 1 passes all tests. Each row below adds
one vocabulary entry, one field or one error-detail field — no new mechanism.

**Error-detail fields.** `ToolError` (`fathomdb-mcp/src/error.rs:22-38`) gains
optional typed fields, omitted when absent, as `candidates` is today:

| Field | Type | Row |
| --- | --- | --- |
| `argument` | string (≤ 64 bytes, else truncated with `…`) | B2-4 |
| `accepted` | array of string (from the tool's input schema) | B2-4 |
| `refs_not_in_response` | array of string (≤ 50, each ≤ 128 bytes), `refs_omitted?` | B2-5 |
| `retry_after_ms` | integer | B2-7 (item 1) |

Errors carry no `hints` array. For errors, `remedy` is already the next-action
slot, and it is a compile-time constant (`error.rs:30`).

### B2-1 — `verdict`/`state` "unavailable" on every call

- Today `Envelope.state` is a required `Unavailable` (`output.rs:53`) built on
  every result (`tools.rs:256-271`). `SearchOutput.verdict` is a required
  `UnavailableWord` (`output.rs:247`), set at `tools.rs:780`.
- **Design.** Declare both as `Option` with
  `#[serde(skip_serializing_if = "Option::is_none")]`, and set them to `None`
  while they can only be `unavailable`. When EP-01 (state read) or EP-13 with
  the MR-031 ADR (verdict) lands, the field is filled again. That is additive,
  with no schema break.
- `orient` reports what is missing once, as a plain field:
  `OrientOutput.not_in_this_release: ["state", "verdict", "kinds", "counts"]`
  (`output.rs:134-145`). This is not a hint, so there is no
  `not_in_this_release` reason in the vocabulary. The `kinds` and `counts`
  fields stay as they are.
- **Amendments in the same change** (drafted here, applied with the Block 2
  code):
  - MR-031.1 (`requirements.md:328-329`): "Without a calibrated score …, the
    search response omits the verdict, and `orient` lists `verdict` in
    `not_in_this_release`."
  - MR-050, last sentence (`requirements.md:425`): "Until the read exists,
    responses omit the identifier, and `orient` lists `state` in
    `not_in_this_release`."
  - `dev/interfaces/mcp.md` § 3 envelope table (`mcp.md:74`) and the search
    `verdict` bullet (`mcp.md:138`).
- This saves about 14 estimated tokens per search and removes the misleading
  signal.

### B2-2 — zero or weak results

- `no_results` (`action: "search"`, `mode?` per § 3.3) whenever zero hits
  come back. That covers both `empty` and `empty_not_ready`.
- **Why there is no `index_not_ready` hint.** `EmptyNotReady` fires on zero
  hits with *any* degradation entry (`shape.rs:62-64`, called with
  `!degradation.is_empty()` at `tools.rs:773-779`). The entries are a soft
  fallback, dense disabled, or no usable embedder (`tools.rs:668-688`). In all
  three the text arm ran, so rephrasing can help. The response therefore
  carries `no_results` plus `degraded`. `index_not_ready` is reserved for a
  real projection-readiness signal (such as EP-03's typed degradation).
- `weak_results` is reserved. The only calibrated per-hit score today is the
  cross-encoder `ce_score` (`search_types.rs:99-105`), and a "weak" threshold
  is an MR-031 ADR decision (`requirements.md:322-333`).

### B2-3 — hybrid fell back to text only

`degraded` with `arm` and `cause`, one hint per response. When several entries
are present, it names the first in canonical order (`shape.rs:104-110`), and
the `degradation` array stays the full record. Text template: "hybrid ran
without the vector arm (`cause`); results are lexical only". There is no
suggested call, because an agent cannot fix degradation.

### B2-4 — unknown argument (for example `limit` on `fetch`)

- Today `parse` maps serde's error to `bad_argument` with the message
  `invalid arguments: <serde text>` (`tools.rs:205-208`). Serde's
  unknown-field text already lists the expected fields in prose ("unknown field
  `…`, expected …", serde_core 1.0.228 `src/de/mod.rs:273-278`). Agents
  repeated the mistake anyway.
- **Design.** Before deserializing, compare the argument keys against the
  tool's accepted list. The preferred source is the input schema `edge.rs`
  already builds per tool (`edge.rs:47-49`, `schema_for_input::<T>()`): take
  its `properties` keys once, at router construction. The fallback is a
  per-tool `const ACCEPTED: &[&str]` with a test pinning it to those schema
  properties. Either way, one list is authoritative and the two cannot drift.
- The first unknown key gives `code: "unknown_argument"` (new, class
  `invalid_request`), `argument: "limit"`, the full `accepted: [...]`, and a
  per-tool remedy constant. For `fetch` the remedy is "fetch takes no limit;
  pass fewer refs, or max_tokens to bound the size".
- The same change updates `dev/interfaces/mcp.md:119` ("Unknown arguments are
  …") and the code table at `mcp.md:244`.
- The check runs on the parse path, which is already linear in argument
  count, and builds no details on success.

### B2-5 — `feedback` refs not in the response

- Today it is one generic `bad_argument` (`tools.rs:1137-1142`).
- **Design.** `code: "refs_not_in_response"` (new), with
  `refs_not_in_response` holding the offending refs in request order (≤ 50,
  then `refs_omitted`). They come from the same set difference against
  `entry.refs` (`session.rs:32`) that the check already computes. The same
  change updates the `mcp.md:244` code table and the `feedback` text at
  `mcp.md:204-206`.
- They are the caller's own strings, never record content. They go in a
  structured field, never in `message`, so MR-012.2 holds.

### B2-6 — default store visibility

- `list_stores` already marks each row's `default` (`output.rs:76-83`,
  `tools.rs:321-328`). The agent still has to infer that store-less calls work.
- **Design.**
  - `list_stores` gains a top-level `default_store: string|null`.
  - It emits a `default_store_set` hint
    (`{"action":"search","reason":"default_store_set","store":"conv0"}`) when
    that value is non-null.
  - The text block's first line becomes `stores: N default="conv0"`.
- **Guide.** `INSTRUCTIONS` (`edge.rs:30-45`) changes its store bullet to:
  "Calls without `store` use the session default; list_stores and use_store are
  only needed to switch stores." It stays a compile-time constant, so MR-012.1
  holds.
- Whether this recovers the wasted calls is untested
  (`mcp-agent-loop/README.md:485-488`). Item 5's re-measure covers it.

### B2-7 — busy

Designed in item 1 (`queue-admission.md`). This design reserves the
`retry_after_ms` error-detail field and the `retry` action word, so the queue
adds entries rather than a new shape.

## 7. Public-surface impact and compatibility

| Surface | Change | Stage |
| --- | --- | --- |
| `dev/interfaces/mcp.md` | § 2: the `hints` key. § 3: the `hints` field, the one vocabulary table, the ignore-unknown rule and the budget side effect (§ 5.1). § 4: the `hints` argument on every tool and `list_stores.default_store`. | 1 |
| `dev/interfaces/mcp.md` | `state`/`verdict` omission and `orient.not_in_this_release`. § 6: `unknown_argument`, `refs_not_in_response` and the error-detail fields. `mcp.md:119`. Quoted refs in text blocks. | Block 2 |
| MCP output schemas (`output.rs`, derived by schemars, `edge.rs:51-58`) | Optional `hints`; optional `state`, `verdict`; new `ToolError` optional fields | 1 / Block 2 |
| MCP server instructions | One sentence: "`hints` lists suggested next calls". The B2-6 wording. | 1 / Block 2 |
| `dev/design/mcp-adapter/requirements.md` | A new MR for hints (H1–H5, bounds, config). MR-031.1 and MR-050 amendments (§ 6 B2-1). | 1 / Block 2 |
| `fathomdb-engine` public API, `dev/interfaces/rust.md` | New `hints` module (engine subset only); `SearchResult.hints` | 1 / 2 |
| `dev/interfaces/rust-sdk.md`, `python.md`, `typescript.md` | Re-exported types; the `hints` option on `search`, `search_text_only`, `search_frozen`; `SearchResult.hints` | 2 |
| `src/conformance/governed-operation-parity.json` | None. This is an option, not an operation (the same precedent as `view`, `dev/interfaces/python.md:969-970`). | — |
| `dev/design/perf-gates.md` | The gate row of § 4.3.4, added with the test | 1 |
| ADR | One ADR, "Agent hints sidecar" (draft in § 11) | 1 |

Compatibility:

- **Existing MCP clients.** New optional fields are additive under the
  interface's own rule (`architecture.md:438-441`). `outputSchema` stays
  `anyOf[result, {error}]` (`dev/interfaces/mcp.md:60-65`). A client that
  wants hints-free bytes passes `hints: false` or sets `hints: false` in
  config.
- **SDK callers.** Off by default, so results are byte-identical. Python adds
  `hints: list[Hint] | None = None`, the Python evolution rule
  (`types.py:1176-1179`). TypeScript adds an optional `hints` that is null when
  off.
- **Breaking items (Block 2).** The B2-1 omission (fields become optional and
  absent), the B2-4 code change from `bad_argument` to `unknown_argument`,
  B2-5's new code, and quoted refs in text blocks. Each lands with its
  interface update in the same change. The crate is `publish = false` until the
  evaluation is done (`dev/design/mcp-adapter/design.md` § 2).

## 8. Test plan for later (RED first)

Each RED test is its own commit with failure evidence (owner rule: one RED per
behaviour change). Integration tests use a real database through the existing
in-process protocol harness (`fathomdb-mcp/tests/`), with no mocks.

Block 1:

1. `search_with_cut_snippets_emits_one_fetch_hint_with_every_cut_ref`: long
   bodies, `snippet_chars` 40, `limit` 60. One `snippet_cut` hint lists every
   cut ref in rank order up to 50, with `refs_omitted` = 10.
2. `fetch_hint_refs_are_a_subset_of_returned_refs`: a `proptest` property over
   random pages, budgets and cut patterns. Every hint ref is among
   `results[..].ref`, and no hint references a dropped hit.
3. `hints_never_contain_record_content`: a property test with bodies, kinds and
   source ids drawn from a marker alphabet. No hint string, structured or text,
   contains a marker.
4. `continuation_hint_equals_result_continuation`: `more_results` and
   `cut_by_budget` carry exactly `continuation` and never both.
5. `empty_text_search_suggests_hybrid_only_when_hybrid_available`: each of the
   § 3.3 conditions false in turn gives no `mode`.
6. `empty_with_degradation_emits_no_results_and_degraded`: no
   `index_not_ready` word appears.
7. `degraded_hybrid_emits_degraded_hint_matching_first_degradation_entry`.
8. `fetch_body_cut_emits_hint_with_sufficient_max_tokens`: re-fetching the
   hinted refs with the hinted `max_tokens` returns them uncut.
9. `hints_count_against_the_budget`: `used_tokens` equals ceil(bytes ÷ 4) with
   hints present, and responses stay ≤ `max_tokens` for every budget ≥ 256 (an
   extension of the existing `shape.rs` budget property).
10. `hints_on_may_shrink_kept_but_offsets_stay_consistent`: a property test
    that runs hints on and off over the same ranked list and budgets. `kept_on
    ≤ kept_off`. Each mode's continuation chain returns every hit exactly once,
    in order. Each continuation's stored offset equals `offset + kept` of its
    own response.
11. `hints_off_output_is_byte_identical_to_baseline`: config `hints: false`,
    and separately the per-call `hints: false`. Today's determinism fixture
    bytes are unchanged.
12. `per_call_hints_argument_overrides_config`: all four combinations.
13. `hints_are_deterministic_across_process_lifetimes`: an extension of the
    determinism suite, with the validity instant pinned.
14. `hint_bounds_hold`: one hint per reason, ≤ 50 refs, refs over 128 bytes
    omitted and counted.
15. `hints_validate_against_output_schema`: an extension of the schema suite.
16. `hints_allocation_deltas`: the counting-allocator binary (§ 4.3.1).
17. `hints_overhead_devloop`: the always-on timing test (§ 4.3.2), blocking at
    200 µs.
18. `hit_line_refs_are_quoted_in_text_blocks`.

Block 2:

1. `search_omits_state_and_verdict` and `orient_lists_not_in_this_release`
   (B2-1).
2. `unknown_argument_names_argument_and_accepted_list` for each tool, plus a
   test that the accepted list equals the input-schema properties (B2-4).
3. `feedback_names_refs_not_in_response`, with refs never placed in `message`
   (B2-5).
4. `list_stores_reports_default_store_and_hint`, and
   `instructions_say_store_less_calls_use_the_default` (B2-6).

Stage 2 (SDK):

1. `sdk_search_hints_default_off_returns_none` on Rust, Python and TS.
2. `sdk_search_hints_on_matches_engine_subset`: the same facts give the same
   hints in all three bindings (a parity fixture).

## 9. Not in scope

- Match-centred snippets (item 3). They change what a snippet shows, not
  whether it is cut, so `snippet_cut` hints stay valid.
- Whether hints help accuracy. Item 4's replay and item 5's re-measure test
  that.
- Hints on writes (there are no write tools).

## 10. Open questions for the owner

Each question stays open. The reviewer's suggested answer follows it.

1. Stage 1 placement: the pure engine subset in `fathomdb-engine` now, or
   start in `fathomdb-sdk` and move it down at stage 2?
   *Reviewer:* engine now, with the § 3.2 split (the engine subset only, and
   the wire type in the adapter).
2. B2-1: omit `state`/`verdict` until they carry information (§ 6), or keep
   them labelled?
   *Reviewer:* omit, as `Option` fields, so re-adding them is additive.
3. Defaults: MCP on, SDK off?
   *Reviewer:* yes.
4. Ref cap: every cut ref up to 50 (this design), or a smaller cap at fewer
   tokens?
   *Reviewer:* all cut refs, up to 50.
5. Quote refs in hit lines as well as in hint lines?
   *Reviewer:* quote them (adopted in § 2.3).
6. Hint lines after the header or at the end of the text block? Item 4's
   replay could measure this.
   *Reviewer:* after the header.

## 11. Draft ADR — Agent hints sidecar

Status: draft, **not accepted**. It is one ADR, with stage 2 as a section,
and it would be filed under `dev/adr/` on owner approval.

**Context.** § 1. Precedent: the HITL-ratified explanation sidecar, which
added an optional field on `SearchResult`, `None` and zero-cost when off,
with `SearchHit` untouched (`dev/design/0.8.8-explain-and-telemetry-adr.md:42-56`).

**Decisions.**

- **(a) Split.** `fathomdb-engine` owns a pure engine subset (`no_results`,
  `degraded`, `more_results{limit}`) that every SDK can reach. `fathomdb-mcp`
  owns the agent-facing wire type, converts the engine subset (attaching
  continuations), and adds the adapter-only reasons. The one agent vocabulary
  table is in `dev/interfaces/mcp.md`.
- **(b) Trust.** Rule H1: hint strings are enum words, adapter handles,
  operator store names or refs from the same response. Refs are capped at
  128 bytes inside hints, and every ref in text blocks is JSON-quoted. Each
  cut-ref hint lists every cut ref up to 50.
- **(c) Defaults and precedence.** MCP on, SDK off. Per call, then per
  process, then built in.
- **(d) Budget.** Hints share the response budget (bytes ÷ 4), with no
  separate allowance. With hints on, a page may keep fewer results; every
  response stays internally consistent. Hints are not part of the canonical
  request.
- **(e) Versioning.** Closed enums per interface revision. Clients ignore
  unknown actions and reasons. There is no per-response version field.
- **(f) B2-1.** `state` and `verdict` are optional and omitted while they can
  only be `unavailable`. They are re-added additively when EP-01, or EP-13 with
  the MR-031 ADR, lands. `orient.not_in_this_release` lists what is missing.

**Stage 2 section.** The `hints: bool` option (default `false`) on
`search`, `search_text_only` and `search_frozen` across Rust, Python and
TypeScript, and `SearchResult.hints: Option<Vec<Hint>>` from the engine
subset. It is an option, not an operation, so the parity manifest is
unchanged.

**Consequences.** Agents get executable next calls. MCP output grows by
about 150 tokens on a fully cut 10-hit page. A stated overhead gate joins the
devloop tier of `dev/design/perf-gates.md`. Block 2's breaking items land with
their interface updates.
