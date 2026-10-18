---
title: Hints sidecar for agents (SDK and MCP)
date: 2026-10-18
status: PROPOSED
desc: Design for a FathomDB-authored `hints` sidecar of suggested next actions — vocabulary, where each hint is computed (engine/SDK versus MCP adapter), cost budget, configuration, budget accounting, the Block 2 rows B2-1..B2-6, public-surface impact and the RED test plan.
inputs:
  - dev/design/agent-access/README.md
  - dev/experiments/mcp-agent-loop/README.md
  - dev/design/mcp-adapter/architecture.md
  - dev/design/mcp-adapter/requirements.md
  - dev/interfaces/mcp.md
  - dev/interfaces/rust-sdk.md
  - dev/design/0.8.8-explain-and-telemetry-adr.md
  - src/rust/crates/fathomdb-mcp/src/{tools,shape,output,error,edge,session,config}.rs
  - src/rust/crates/fathomdb-engine/src/{search_types,search_api,search}.rs
  - src/rust/crates/fathomdb-sdk/src/{lib,engine,options}.rs
---

# Hints sidecar for agents (SDK and MCP)

Work item 2 of [README.md](README.md), and the design home of Block 2 rows
B2-1..B2-6. This is a design only. No code or test is written until the
Fable-medium review, its fixes and the owner's HITL review are done.

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
- `state: {status: "unavailable"}` and `verdict: "unavailable"` appear on every
  search (`output.rs:267`, `tools.rs:780`). Haiku flagged them as a failure
  (README row B2-1).
- Guided gpt-5.4-mini called `list_stores` on 92 of 150 questions although a
  default store was set (`mcp-agent-loop/README.md:480-487`).
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

- **H1. FathomDB-authored, data-free.** Every string in a hint is a closed-enum
  word, an adapter-minted handle (`continuation`), an operator-configured store
  name, or a `ref` copied from the same response's `results[].ref`. A hint
  never carries record content, kind names, `source_id`, or text from SDK error
  messages. This keeps the data-not-instructions boundary of
  `dev/design/mcp-adapter/architecture.md:388-393` and MR-012
  (`requirements.md:181-192`). See § 2.3 for refs.
- **H2. Separate from content.** Hints live in their own `hints` field, never
  inside `results`, `items`, `targets` or a record field.
- **H3. Derived, not fetched.** A hint is computed only from values the call
  already holds. No extra query, read, embed or lock (§ 4).
- **H4. Deterministic.** Same request, same state, same hints, byte for byte,
  so MR-043 holds (`requirements.md:405-415`).
- **H5. Advisory.** Ignoring every hint never makes a result wrong. Hints never
  replace a typed field: `snippet_cut`, `continuation`, `completeness` and
  `degradation` stay as they are.

### 2.1 Vocabulary, version 1

`action` names the tool to call next (or `none`). `reason` says why, and picks
which optional fields are present. Both are closed enums in a version. Clients
switch on `reason`.

| `reason` | `action` | Fields | Emitted by | When |
| --- | --- | --- | --- | --- |
| `snippet_cut` | `fetch` | `refs`, `refs_omitted?` | adapter | `search` hits with `snippet_cut: true` |
| `body_cut` | `fetch` | `refs`, `refs_omitted?`, `max_tokens` | adapter | `fetch` items or `expand` targets with `body_cut: true` |
| `more_results` | `search` | `continuation` (adapter) or `limit` (SDK) | shared rule | page reached its limit and more may exist |
| `cut_by_budget` | `search` | `continuation`, `max_tokens` | adapter | hits were dropped to fit `max_tokens` |
| `no_results` | `search` | `mode?` | shared rule | zero hits, nothing degraded (B2-2) |
| `index_not_ready` | `none` | `arm` | shared rule | zero hits with a degradation entry (`empty_not_ready`) (B2-2) |
| `weak_results` | `search` | `mode?` | reserved | needs a calibrated verdict (MR-031); not emitted in v1 (B2-2) |
| `degraded` | `none` | `arm`, `cause` | shared rule + adapter | a degradation entry is present (B2-3) |
| `not_in_this_release` | `none` | `fields` | adapter, `orient` only | signals reported `unavailable` (B2-1) |
| `default_store_set` | `search` | `store` | adapter, `list_stores` only | the session has a default store (B2-6) |

Closed field value sets in v1:

- `action`: `fetch`, `search`, `none`. (`retry` is reserved for item 1's
  queue; busy is an error today, § 6 B2-7.)
- `mode`: `text`, `hybrid`.
- `arm`: `vector`, `text`, `text_edge`, `graph_arm`, `graph` (the arms the
  adapter already reports, `tools.rs:283-290`, `tools.rs:1042-1046`).
- `cause`: `soft_fallback`, `dense_disabled`, `embedder_unavailable` (the
  adapter's existing degradation reasons, `tools.rs:668-688`), plus the
  `graph` codes of `tools.rs:947-958`.
- `fields`: `state`, `verdict`, `kinds`, `counts`.

Versioning:

- The response carries `hints_version: 1` next to `hints`, and both are omitted
  when `hints` would be empty.
- Within a version the sets are closed and the output schema lists them.
- A new `reason`, `action` or field is a new version. It is additive: clients
  must ignore a hint whose `action` or `reason` they do not know. The interface
  docs state this rule once.
- Changing what an existing reason means is breaking and needs an interface
  update in the same change (`architecture.md:432-443`).

### 2.2 Bounds

- At most **4 hints** per response, one per reason. Emission order is fixed:
  `degraded`, `index_not_ready`, `no_results`, `snippet_cut`, `body_cut`,
  `cut_by_budget`, `more_results`, then the store and release notes.
  `cut_by_budget` and `more_results` exclude each other, because both point at
  the same continuation.
- At most **5 refs** per hint, in result rank order (request order for
  `fetch`). Further cut refs are counted in `refs_omitted`. Five is half the
  default page, and the top-ranked cut hits are the ones worth reading.
- A ref longer than **128 bytes** is left out of hints and counted in
  `refs_omitted`. It still appears in `results`.
- Worst case with 128-byte refs: 5 × 131 + about 80 bytes ≈ 0.7 KB per fetch
  hint. The typical LoCoMo case is about 25-byte refs, so ≈ 0.2 KB.

### 2.3 Refs inside hints and the trust boundary

Logical ids are set by writers, not by FathomDB. An id can be any string, so an
id inside an imperative-looking hint has more authority than the same id in
`results[].ref`. Mitigations:

- Hints only copy refs that already appear in the same response, so they expose
  no new data.
- In the text block, refs inside hint lines are JSON-quoted with `shape::quote`
  (`fathomdb-mcp/src/shape.rs:150-155`), the same treatment content gets. The
  hit lines today print `ref` unquoted (`tools.rs:534-537`). That is unchanged
  here and noted as an open question (§ 10).
- The 128-byte cap bounds what an adversarial id can carry.
- Each hint's text line comes from a compile-time template per `(action,
  reason)`. No hint text is built from data.

## 3. Placement: what is computed where

### 3.1 Constraint found in code

The task assumes the Python and TypeScript bindings sit over `fathomdb-sdk`.
They do not. `fathomdb-py` and `fathomdb-napi` depend on `fathomdb-engine`
directly (`fathomdb-py/Cargo.toml:43-46`, `fathomdb-napi/Cargo.toml:29-32`).
Only `fathomdb-mcp` depends on `fathomdb-sdk` (`fathomdb-mcp/Cargo.toml:21`).
`fathomdb-sdk` re-exports the engine's `SearchResult` as is
(`fathomdb-sdk/src/lib.rs:105`), and `Engine::search` returns it unchanged
(`fathomdb-sdk/src/engine.rs:185-211`). So one implementation that all three
SDKs reach has to live in `fathomdb-engine`, or in a crate below it.

### 3.2 Decision

Two layers, one vocabulary.

**Layer 1: shared vocabulary and rule, in `fathomdb-engine` (re-exported by the
SDKs).**

- A new pure module `fathomdb-engine/src/hints.rs`, with no SQL, I/O, clock or
  locks. It holds the v1 types (`Hint`, `HintAction`, `HintReason`,
  `HintCause`, `HINTS_VERSION`) and one function:

  ```rust
  pub fn search_hints(facts: &SearchFacts<'_>, out: &mut Vec<Hint>);
  // SearchFacts: returned hit count, limit, mode (text | hybrid),
  // degradation entries, embedder usable, continuation: Option<&str>.
  ```

- This is the **shared rule** column of § 2.1: `no_results`, `index_not_ready`,
  `degraded` and `more_results`.
- `fathomdb-sdk` re-exports the types, as it re-exports `Explanation`
  (`lib.rs:85`). The Python and TypeScript bindings map them the way they map
  the explanation sidecar (`fathomdb-napi/src/read_search.rs:688-714`;
  `src/python/fathomdb/types.py:1169-1179`).

**Layer 2: adapter-only hints, in `fathomdb-mcp`.** These need facts only the
adapter has:

| Hint | Why it is adapter-only |
| --- | --- |
| `snippet_cut` | Snippets are adapter presentation. The SDK returns whole bodies (`search_types.rs:95`), and the adapter cuts them at `snippet_chars` (`tools.rs:701-709`, `shape.rs:13-20`). |
| `body_cut` | Body cuts come from the adapter's budget fitting (`tools.rs:931-944`, `tools.rs:1088-1101`). |
| `more_results` with `continuation`, `cut_by_budget` | Continuations and the budget are adapter state (`tools.rs:732-804`, `shape.rs:36-48`). The adapter passes its own continuation into `SearchFacts`, so the rule stays in Layer 1 and only the handle comes from the adapter. |
| `degraded` / `embedder_unavailable` | The adapter reads embedding readiness once per open epoch (`tools.rs:682-687`). It passes this fact into `SearchFacts`. |
| `default_store_set` | Sessions are adapter state (`session.rs:89-104`). |
| `not_in_this_release` | Which signals are `unavailable` is an adapter-release fact (`output.rs:10-27`). |

**SDK result carrier (stage 2).** The three SDK search verbs (`search`,
`search_text_only`, `search_frozen`) gain an option `hints: bool`, default
`false`. When it is true, the engine fills a new sidecar,
`SearchResult.hints: Option<Vec<Hint>>`, at the four construction sites
(`search_api.rs:668`, `search_api.rs:1084`, `search.rs:280`, `search.rs:824`).
It uses the SDK facts: `limit` as given and no continuation, so `more_results`
carries `limit`, the smaller of 2 × limit and 100. This follows the
HITL-ratified explanation-sidecar precedent: `Option` on `SearchResult`, `None`
when off, `SearchHit` untouched (`dev/design/0.8.8-explain-and-telemetry-adr.md:42-56`).
`SearchResult` is `#[non_exhaustive]` (`search_types.rs:137`), so the field is
additive in Rust.

**Why the adapter does not call the SDK carrier.** The adapter asks the engine
for `offset + limit` hits and skips `offset` (`tools.rs:637`, `tools.rs:706`).
So the SDK's "limit reached" is not the page's "limit reached". The adapter
calls `search_hints` itself with page facts, using the same function. That
gives one rule with no drift, and no duplicate work.

**Staging.** Stage 1 (Block 1) adds only the pure engine module and the adapter
wiring. No engine search path changes, so MCP agents benefit at once. Stage 2
adds the SDK `hints` option and the `SearchResult.hints` carrier across the
three SDKs, under its own ADR. If the owner prefers no engine-crate change in
stage 1, the module can start in `fathomdb-sdk` (Rust only). It would then
move down to the engine for stage 2. This choice is open question 1.

### 3.3 Hints per tool

| Tool | Hints (v1) |
| --- | --- |
| `search` | `degraded`, `index_not_ready`, `no_results`, `snippet_cut`, `cut_by_budget` or `more_results` |
| `fetch` | `body_cut` (refs of the cut items; `max_tokens` = the current estimate plus the bytes cut ÷ 4 × 2, because each body appears in both the structured content and the text block, `shape.rs:166-169`) |
| `expand` | `body_cut` (refs of the cut targets), `degraded` (graph codes) |
| `orient` | `not_in_this_release` (B2-1) |
| `list_stores` | `default_store_set` (B2-6) |
| `use_store`, `feedback` | none |

`no_results` carries `mode: "hybrid"` only when the call ran in text mode and
hybrid can serve (an embedder is usable, which the adapter knows from
`answered.info.embedder_usable`, `tools.rs:682`). Otherwise it carries no
`mode`, and the text template says "rephrase with synonyms, names or related
events". That wording already exists in the guide (`edge.rs:35-37`).

## 4. Cost: laser-fast

### 4.1 Mechanism

- **Off means no allocation.** The adapter output gets
  `#[serde(skip_serializing_if = "Vec::is_empty")] hints: Vec<Hint>`. An empty
  `Vec::new()` does not allocate, and the `hints_version` field is skipped the
  same way. The SDK carrier is `None`. With hints off, every output is
  byte-identical to today's, and the existing determinism and schema suites
  prove it unchanged.
- **One planning pass, outside the budget loop.** The search budget loop
  re-renders and re-serializes the page each time it drops a hit
  (`tools.rs:731-791`). So hints are planned once, before the loop, as a small
  plan of hit indices (cut hits in rank order, up to the ref cap), using the
  `snippet_cut` flags already computed (`tools.rs:709`). Each loop iteration
  emits only the indices below `kept`. No hint needs a second pass over bodies.
- **No new reads.** Every input is already in scope: `kept`, `page_hits`,
  `page_limit`, `degradation`, `mode`, `answered.info`, `continuation`, the
  `snippet_cut` and `body_cut` flags, and `session.default_store()`.
- **Error-path data only on error.** B2-4 and B2-5 build their details only
  when the call already fails (§ 6).

### 4.2 Budget

- Hint derivation and rendering: **≤ 10 µs at p50 and ≤ 25 µs at p99** added
  per `search` call at `limit` 10, and ≤ 50 µs at p50 at `limit` 100,
  including the extra serialization in the budget loop.
- End to end: hints on against hints off differ by at most **max(10 µs, 1% of
  search p50)** at p50. For scale, the binding 10k-row text fixture's p50
  budget is 20 ms (`dev/acceptance.md:1172`), so 1% is 200 µs. The absolute
  bound is the one that binds on small stores such as LoCoMo.
- Allocation: **zero** heap allocations from the hints path when hints are
  off. When they are on, at most one `Vec` per response plus one `String` clone
  per emitted ref (≤ 5 per hint).
- Tokens: typically ≤ 60 estimated tokens per search response (one fetch hint
  of five ~25-byte refs, in both the structured and text forms). The worst
  case is bounded by § 2.2, at about 0.7 KB × 2 ÷ 4 ≈ 350 tokens.

### 4.3 How it is measured (design, not code)

The workspace has no `criterion` benches (no `criterion` in any
`src/rust/**/Cargo.toml`). The design therefore uses std-only harnesses, gated
like the existing long-running perf tests (`AGENT_LONG=1`, AGENTS.md § 3):

1. **Allocation test (always on, not timing-based).** A test binary of its own
   with a counting `#[global_allocator]`. It shapes a fixed 100-hit synthetic
   page with hints off and asserts zero allocations from the hints module. With
   hints on, it asserts at most one plus the emitted-ref count.
2. **Micro-timing (`AGENT_LONG`).** A synthetic `SearchFacts` and a 10- and
   100-hit page with bodies of 0.2–4 KB (half over `snippet_chars`). It runs
   31 trials of 10,000 iterations, takes the median of the trial medians, and
   compares off against on, run interleaved to cancel thermal drift. It fails
   if § 4.2's micro budget is exceeded.
3. **End to end (`AGENT_LONG`).** An in-process MCP client (the existing
   protocol-test harness) against the LoCoMo conv0 store and the 10k-row text
   fixture: 2,000 `search` calls per arm with interleaved on/off pairs and the
   same 50 queries. It reports p50 and p99 and applies the max(10 µs, 1%)
   rule. The numbers are recorded in the board, not asserted on shared CI
   runners.

## 5. Configuration

- **Per process.** A new config key `hints` (bool, default `true`) in
  `dev/interfaces/mcp.md` § 2. It joins `TOP_LEVEL_KEYS`
  (`fathomdb-mcp/src/config.rs:140-155`), because unknown keys are invalid.
- **Per call.** A new optional argument `hints` (bool) on every tool. All
  argument structs use `deny_unknown_fields` (`tools.rs:72-203`), so it is
  declared on each. A continuation call may also pass it, which widens the rule
  at `tools.rs:566-582`, because it does not change the page.
- **Precedence.** The per-call argument wins over the process config, which
  wins over the built-in default. There is no operator lock. Hints change no
  data, so a caller who opts in on a hints-off server takes no risk.
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

### 5.1 Budget accounting (bytes ÷ 4)

- Hints are part of the structured content and the text block, so
  `shape::with_budget` counts them like any other field
  (`shape.rs:36-48`, `dev/interfaces/mcp.md:76`). There is no separate
  allowance.
- In `search`, hints are emitted inside the budget loop from `page[..kept]`.
  Dropping a hit also drops its ref from the hint, and the loop's existing exit
  rule stays as it is (`tools.rs:787-790`). With `kept == 0` there is no
  continuation (`shape.rs:97-101`), so only `degraded` or the empty-result
  hints can remain.
- In `fetch` and `expand`, the `body_cut` hint is part of what
  `shape::cut_bodies` sizes (`tools.rs:931-935`, `tools.rs:1088-1092`). Its
  refs come from items already cut, so `cut_bodies` reaches its usual fixed
  point. The hint is planned before cutting, from the refs whose bodies cannot
  fit, and then trimmed to the refs actually cut.
- Text rendering: one line per hint after the header line and before the hits,
  for example `hint: fetch ["l:a","l:b"] (snippet_cut: snippets are prefixes;
  fetch to read the full record)`. The fixed template text is short, and the
  `reason` word is the stable key.

## 6. Block 2 rows as vocabulary entries

Block 2 is implemented only after Block 1 passes all tests. Each row below adds
one vocabulary entry or one error-detail field, and no new mechanism.

**Error-detail fields.** `ToolError` (`fathomdb-mcp/src/error.rs:22-38`)
gains optional typed fields, omitted when absent, as `candidates` is today:

| Field | Type | Row |
| --- | --- | --- |
| `argument` | string (≤ 64 bytes, else truncated with `…`) | B2-4 |
| `accepted` | array of string (compile-time list) | B2-4 |
| `refs_not_in_response` | array of string (≤ 5, each ≤ 128 bytes), `refs_omitted?` | B2-5 |
| `retry_after_ms` | integer | B2-7 (item 1) |

Errors carry no `hints` array. For errors, `remedy` is already the next-action
slot, and it is a compile-time constant (`error.rs:30`).

### B2-1 — `verdict`/`state` "unavailable" on every call

- Today `state: {status: "unavailable"}` is in every envelope (`output.rs:48-56`,
  `tools.rs:256-271`), and `verdict: "unavailable"` is on every search
  (`output.rs:247`, `tools.rs:780`). Neither carries any information in this
  release.
- **Recommended:** omit `state` and `verdict` from per-call results while they
  can only be `unavailable`. `state` returns with EP-01 and `verdict` with
  MR-031's calibrated scores (EP-13). Report them once in `orient` with the
  hint `{"action":"none","reason":"not_in_this_release","fields":["state","verdict","kinds","counts"]}`.
  This saves about 14 estimated tokens per search and removes the misleading
  signal.
- **Requirements impact:** MR-031 says "every search response carries a
  verdict" (`requirements.md:322-329`), and the envelope table lists `state`
  (`dev/interfaces/mcp.md:74`). Omission needs both amended, which is open
  question 2.
- **Fallback if the owner keeps the fields:** add
  `reason: "not_in_this_release"` beside `status` in `Unavailable`
  (`output.rs:17-27`), and leave `verdict` as is. The orient hint is the same.

### B2-2 — zero or weak results

- `no_results` (`action: "search"`, optional `mode`) when `completeness` is
  `empty` (`shape.rs:62-64`).
- `index_not_ready` (`action: "none"`, `arm`) when it is `empty_not_ready`.
  Rephrasing does not help there, and the hint says so.
- `weak_results` is reserved in v1. The only calibrated per-hit score today is
  the cross-encoder `ce_score` (`search_types.rs:99-105`). A "weak" threshold
  is an MR-031 ADR decision (`requirements.md:322-333`), so it is not emitted
  until that ADR lands.

### B2-3 — hybrid fell back to text only

`degraded` with `arm` and `cause`, one hint per response. When several entries
are present, it names the first entry in canonical order (`shape.rs:104-110`),
and the `degradation` array stays the full record. Text template: "hybrid ran
without the vector arm (`cause`); results are lexical only". There is no
suggested call, because an agent cannot fix degradation.

### B2-4 — unknown argument (for example `limit` on `fetch`)

- Today `parse` maps serde's error to `bad_argument` with the message
  `invalid arguments: <serde text>` (`tools.rs:205-208`). Serde's
  unknown-field text already lists the expected fields in prose ("unknown field
  `…`, expected …", serde_core 1.0.228 `src/de/mod.rs:273-278`). Agents still
  repeated the mistake.
- **Design:** before deserializing, compare the argument keys against a
  per-tool `const ACCEPTED: &[&str]`. A test pins that list to the tool's input
  schema properties. The first unknown key gives `code: "unknown_argument"`
  (new, class `invalid_request`), `argument: "limit"`, the full
  `accepted: [...]`, and a per-tool remedy constant. For `fetch` the remedy is
  "fetch takes no limit; pass fewer refs, or max_tokens to bound the size".
- The check runs only on the parse path, which is already linear in argument
  count, and builds no details on success.

### B2-5 — `feedback` refs not in the response

- Today it is one generic `bad_argument` (`tools.rs:1137-1142`).
- **Design:** `code: "refs_not_in_response"` (new), with
  `refs_not_in_response` holding the offending refs in request order (≤ 5,
  plus `refs_omitted`). They come from the same set difference against
  `entry.refs` (`session.rs:32`) that the check already computes.
- They are the caller's own strings, never record content, and they go in a
  structured field, never in `message`, so MR-012.2 holds.

### B2-6 — default store visibility

- `list_stores` already marks each row's `default` (`output.rs:76-83`,
  `tools.rs:321-328`). The agent still has to infer that store-less calls work.
- **Design:** `list_stores` gains a top-level `default_store: string|null`. A
  `default_store_set` hint (`{"action":"search","reason":"default_store_set","store":"conv0"}`)
  is emitted when that value is non-null. The text block's first line becomes
  `stores: N default=conv0`.
- **Guide:** `INSTRUCTIONS` (`edge.rs:30-45`) changes its store bullet to:
  "Calls without `store` use the session default; list_stores and use_store are
  only needed to switch stores." It stays a compile-time constant, so MR-012.1
  holds.
- Whether the guide change alone recovers the wasted calls is untested
  (`mcp-agent-loop/README.md:485-488`). Item 5's re-measure covers it.

### B2-7 — busy

Designed in item 1 (`queue-admission.md`). This design reserves the
`retry_after_ms` error-detail field and the `retry` action word, so the queue
adds entries rather than a new shape.

## 7. Public-surface impact and compatibility

| Surface | Change | Stage |
| --- | --- | --- |
| `dev/interfaces/mcp.md` | § 2 `hints` key; § 3 `hints`/`hints_version` envelope fields, the vocabulary table and the "ignore unknown reasons" rule; § 4 `hints` argument on every tool, `list_stores.default_store`; § 6 `unknown_argument`, `refs_not_in_response` and the new error-detail fields | 1 (hints), Block 2 (B2 rows) |
| MCP output schemas (`output.rs`, derived by schemars, `edge.rs:51-58`) | Optional `hints`, `hints_version`; new `ToolError` optional fields | 1 / Block 2 |
| MCP server instructions | B2-6 wording; one sentence: "`hints` lists suggested next calls" | 1 |
| `dev/design/mcp-adapter/requirements.md` | New MR for hints (H1–H5, bounds, config); MR-031 and MR-050 amendment if B2-1 omission is chosen | 1 / Block 2 |
| `fathomdb-engine` public API, `dev/interfaces/rust.md` | New `hints` module and types (stage 1); `SearchResult.hints` (stage 2) | 1 / 2 |
| `dev/interfaces/rust-sdk.md`, `python.md`, `typescript.md` | Re-exported types; `hints` option on `search`, `search_text_only`, `search_frozen`; `SearchResult.hints` | 2 |
| `src/conformance/governed-operation-parity.json` | None: an option, not an operation (the same precedent as `view`, `dev/interfaces/python.md:969-970`) | — |
| ADRs | A new ADR, "Agent hints sidecar", covering the vocabulary, versioning rule, defaults and the trust rule H1. Stage 2 cites the 0.8.8 sidecar ADR (§ A.2) as precedent. | 1 |

Compatibility:

- **Existing MCP clients.** New optional fields are additive under the
  interface's own rule (`architecture.md:438-441`). `outputSchema` stays
  `anyOf[result, {error}]` (`dev/interfaces/mcp.md:60-65`) with extra optional
  properties. A client that wants today's bytes passes `hints: false` or sets
  `hints: false` in config.
- **SDK callers.** Off by default, so results are byte-identical. Python adds
  `hints: list[Hint] | None = None`, the Python evolution rule
  (`types.py:1176-1179`). TypeScript adds an optional `hints` that is null when
  off.
- **Breaking items.** Only the B2-1 omission (if chosen) and the B2-4 code
  change from `bad_argument` to `unknown_argument`. Both land in Block 2 with
  an interface update in the same change. The crate is `publish = false` until
  the evaluation is done (`dev/design/mcp-adapter/design.md`, § 2).

## 8. Test plan for later (RED first)

Each RED test is its own commit with failure evidence (owner rule: one RED per
behaviour change). Integration tests use a real database through the existing
in-process protocol harness (`fathomdb-mcp/tests/`), with no mocks.

Block 1:

1. `search_with_cut_snippets_emits_one_fetch_hint_with_top_cut_refs`: long
   bodies, `snippet_chars` 40. Expect one `snippet_cut` hint, refs in rank
   order, ≤ 5, and `refs_omitted` correct.
2. `fetch_hint_refs_are_a_subset_of_returned_refs`: a property test
   (`proptest`) over random pages, budgets and cut patterns. Every hint ref is
   among `results[..].ref`, and no hint ever references a dropped hit.
3. `hints_never_contain_record_content`: a property test with bodies, kinds and
   source ids drawn from a marker alphabet. No hint string, structured or text,
   contains a marker. Every string is an enum word, a ref from the response, a
   continuation or the store name.
4. `continuation_hint_equals_result_continuation`: `more_results` and
   `cut_by_budget` carry exactly `continuation` and never both.
5. `empty_text_search_suggests_hybrid_only_when_embedder_usable`: the
   `no_results` mode rule, both branches.
6. `empty_not_ready_emits_index_not_ready_not_rephrase`.
7. `degraded_hybrid_emits_degraded_hint_matching_first_degradation_entry`.
8. `fetch_body_cut_emits_hint_with_sufficient_max_tokens`: re-fetching the
   hinted refs with the hinted `max_tokens` returns them uncut.
9. `hints_count_against_the_budget`: `used_tokens` equals ceil(bytes ÷ 4) with
   hints present, and responses stay ≤ `max_tokens` for every budget ≥ 256 (an
   extension of the existing `shape.rs` budget property).
10. `hints_off_output_is_byte_identical_to_baseline`: config `hints: false`,
    and separately the per-call `hints: false`. Today's determinism fixture
    bytes are unchanged.
11. `per_call_hints_argument_overrides_config`: all four combinations.
12. `hints_are_deterministic_across_process_lifetimes`: an extension of the
    determinism suite, with the validity instant pinned.
13. `hint_bounds_hold`: ≤ 4 hints, ≤ 5 refs, refs over 128 bytes omitted and
    counted.
14. `hints_validate_against_output_schema`: an extension of the schema suite
    with hints on.
15. `hints_path_allocates_nothing_when_off`: the counting-allocator binary
    (§ 4.3.1).
16. `AGENT_LONG` timing harnesses (§ 4.3.2–3). These report, and enforce the
    micro budget only.

Block 2:

1. `orient_reports_unavailable_signals_once`, and, if omission is chosen,
   `search_omits_state_and_verdict` (B2-1).
2. `unknown_argument_names_argument_and_accepted_list` for each tool. A table
   test that `ACCEPTED` equals the input-schema properties (B2-4).
3. `feedback_names_refs_not_in_response`, with refs never placed in `message`
   (B2-5).
4. `list_stores_reports_default_store_and_hint`, and
   `instructions_say_store_less_calls_use_the_default` (B2-6).

Stage 2 (SDK):

1. `sdk_search_hints_default_off_returns_none` on Rust, Python and TS.
2. `sdk_search_hints_on_matches_shared_rule`: the same facts give the same
   hints in all three bindings (a parity fixture).

## 9. Not in scope

- Match-centred snippets (item 3). They change what a snippet shows, not
  whether it is cut, so `snippet_cut` hints stay valid.
- Whether hints help accuracy. Item 4's replay and item 5's re-measure test
  that. This design only makes the hints cheap and safe.
- Hints on writes (there are no write tools).

## 10. Open questions for the owner

1. Stage 1 placement: the pure `hints` module in `fathomdb-engine` now
   (recommended, reachable by all three SDKs later), or in `fathomdb-sdk` now
   and moved down at stage 2?
2. B2-1: omit `state`/`verdict` until they mean something (recommended, which
   amends MR-031 and MR-050 wording), or keep them labelled?
3. Defaults: MCP on and SDK off, as recommended?
4. The ref cap of 5 per hint. Should `snippet_cut` instead list every cut ref
   on the page, up to the 50-ref `fetch` cap (`tools.rs:40`), at more tokens?
5. The text block quotes refs inside hint lines but not in hit lines
   (`tools.rs:534-537`). Should hit-line refs be quoted too, for consistency?
   That is a small text-format change.
6. Placement of hint lines in the text block: after the header (recommended)
   or at the end? Item 4's replay could measure this.
