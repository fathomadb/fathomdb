---
title: MCP adapter requirements and acceptance criteria
date: 2026-10-17
status: PROPOSED
desc: Falsifiable requirements and acceptance criteria for an MCP adapter over FathomDB that meet the Agent Needs in agent-needs.md.
inputs:
  - dev/design/mcp-adapter/agent-needs.md
  - dev/adr/ADR-0.6.0-cli-scope.md
  - dev/adr/ADR-0.6.0-async-surface.md
  - dev/adr/ADR-0.6.0-database-lock-mechanism-reader-pool-revision.md
  - dev/adr/ADR-0.8.25-eligibility-and-frozen-reads.md
  - src/rust/crates/fathomdb-sdk/src/{engine,read,graph,guard,options}.rs
---

# MCP adapter requirements and acceptance criteria

Conventions:

- `MR-###` is a requirement: one observable outcome for an agent, a harness,
  or the embedding application. `MR-###.n` is an acceptance criterion: a check
  a test can pass or fail. Ids are local to this proposal, not entries in
  `dev/requirements.md` or `dev/acceptance.md`; when adopted, each AC maps to a
  named TDD test. Ids are stable from review cycle FIX-1 on.
- "Tool" is an MCP tool exposed by the adapter; "result" is one item in a
  tool's structured output; "harness" is the MCP client hosting the agent;
  "reference" is the record identifier a tool returns.
- **[ADR]** marks a number or a closed set that the adopting ADR owns; the value
  shown is the proposed default.
- **[engine]** marks a requirement that needs a new or extended `fathomdb-sdk`
  verb, under its own ADR, before the adapter can meet it. Until then the
  adapter reports the signal as `unavailable` (MR-032), never an approximation.
- "Store state" means the frozen-read state binding: canonical write boundary,
  read-visibility generation, dependency generation and projection digests
  (ADR-0.8.25). Any committed write advances it.

## Reach and protocol hygiene

**MR-001 — Local stdio launch.** A harness can launch the adapter as a local
process over MCP stdio, pointing at one database file, with no network
listener. Meets AN-018.

- MR-001.1 Launching the binary with a database path and completing the MCP
  `initialize` handshake succeeds against a fresh and an existing database.
- MR-001.2 While running, the process holds no listening socket.
- MR-001.3 The adapter passes the official MCP conformance server scenarios
  applicable to stdio and the tools capability.

**MR-002 — Protocol channel purity.** Only MCP JSON-RPC messages are written
to the protocol output channel, including during open, migration, embedder
warm-up and model download, and when a handler panics. Meets AN-018.

- MR-002.1 Over a session that opens a database requiring migration, warms the
  embedder, and calls every tool, every stdout byte parses as JSON-RPC.
- MR-002.2 A forced panic in one tool handler yields a JSON-RPC error for that
  request, writes nothing else to stdout, and a following call to another tool
  succeeds.
- MR-002.3 All diagnostics go to stderr or to the host's registered subscriber.

**MR-003 — Responsive before the engine is ready.** The handshake and
`tools/list` complete without waiting for engine open, migration or embedder
warm-up; tool calls made before the engine is ready return a typed not-ready
result. Meets AN-012, AN-018.

- MR-003.1 With engine open artificially delayed beyond the handshake timeout
  *(ADR: 10 s)*, `initialize` and `tools/list` still complete within 1 s.
- MR-003.2 A search issued during warm-up returns not-ready with the phase
  named, not an error and not an empty result.
- MR-003.3 When open fails (lock held, corruption, migration refusal), every
  tool call returns the typed open error with its remedy, and stderr carries the
  same; the process does not exit until the harness closes it.

**MR-004 — Embeddable in the owning process.** An application that holds an
open `Engine` can serve the same tools over a transport it chooses, so the
application and its agents share one store without a second opener. Meets
AN-018.

- MR-004.1 A Rust application holding an open `Engine` serves the tool set over
  an in-memory transport; a client on that transport completes a search; no
  second `Engine::open` of the file occurs.
- MR-004.2 Starting the standalone adapter against a file another process holds
  yields the typed lock error per MR-003.3, naming the database path.

## Trust and scope

**MR-010 — Origin class on every result.** Every result carries its record's
`source_id` and origin class as structured fields separate from content.
Meets AN-001, AN-002. **[engine]** first-class origin field; the class set is
**[ADR]** (proposed: `owner`, `agent`, `untrusted`, `system`, `unclassified`).

- MR-010.1 On a fixture containing each class, every result of every read tool
  carries the expected class and `source_id`.
- MR-010.2 Records with no recorded class report `unclassified`, never a
  trusted class.
- MR-010.3 A caller can restrict a search to a set of classes; a search
  restricted to `owner` returns no record of any other class.
- MR-010.4 An ingested record whose body contains a field or text claiming a
  class (for example `"origin": "owner"`) reports the class its writer was
  entitled to, unchanged by the body.

**MR-011 — Writers cannot choose their trust.** Records written through the
adapter are stored with the origin class set by launch policy; no tool argument
or content can change it. Meets AN-002.

- MR-011.1 A write call that attempts to set any class is rejected as invalid
  request; the stored class of adapter writes always equals the policy class.

**MR-012 — Content is framed as data.** Tool descriptions, server instructions
and error-message templates are fixed at build time; record content and
record-derived names (kinds, fields) appear only in designated result fields.
Meets AN-002.

- MR-012.1 `tools/list` and server instructions are byte-identical across two
  databases with different contents.
- MR-012.2 No error message contains record content or any kind or field name
  not declared by the operator or the projection registry.

**MR-013 — Currency on every result.** Every result states its validity window
relative to the response's validity instant and whether a newer record
superseded it. Meets AN-003. **[engine]** for search hits: `SearchHit` carries
id, kind, body, scores and `source_id` but no validity or supersession fields
today; `NodeRecord` reads do.

- MR-013.1 On a fixture with a superseded record and an expired record, every
  read tool that returns them, under a view that includes them, reports the
  supersession and the expiry.
- MR-013.2 Under the default view, neither appears in search results.

**MR-014 — Operator-bound scope.** The operator can bind an adapter session to
a scope expressed over declared exact projections; every tool honours it, and
no tool argument can widen it. Meets AN-019.

- MR-014.1 With a session bound to scope A, no read tool returns a record
  outside A, including by direct reference.
- MR-014.2 A call that passes a filter or reference outside A behaves as if
  the record does not exist (not found), and does not disclose its existence.

## Finding

**MR-020 — Exact constraints compose with relevance search.** A caller can
require that every result contain an exact literal token or phrase, combined
with relevance ranking and structural filters, in one call. Meets AN-004.
**[engine]** (a lexical phrase predicate on `search`; distinct from the
`Json` predicate ADR-0.8.11 D3 rejects there).

- MR-020.1 With an exact constraint, every result contains the literal in its
  indexed text, and no record lacking it appears.
- MR-020.2 A record containing a rare identifier that relevance alone ranks
  outside the default limit is ranked first when the identifier is given as an
  exact constraint.

**MR-021 — Measured default quality.** The default search configuration is
evaluated through the adapter on a designated agent-memory corpus **[ADR]**,
and may not ship as the default unless it meets the floor. Meets AN-004.

- MR-021.1 An evaluation run reports precision@3 and recall@10 for the adapter's
  default search path and for plain BM25 on the same corpus.
- MR-021.2 If the default path's precision@3 is below plain BM25's, the adapter's
  default is the lexical-only path, and the adopting ADR records the measured
  gap.

**MR-022 — Cheap orientation.** One read-only tool returns what the store
holds — kinds with counts, time range, class counts, index readiness — within a
fixed cap. Meets AN-005. **[engine]** (kind enumeration and per-kind and
per-class counts; no SDK verb provides them today).

- MR-022.1 On a fixture with known contents, the tool reports the exact kinds,
  per-kind counts, class counts, time bounds and index readiness.
- MR-022.2 On a store with more kinds than fit the cap *(ADR: 2,000 estimated
  tokens)*, the result stays within the cap and states how many kinds were
  omitted and how to list them.

**MR-023 — As-of queries.** A caller can run search and fetch at a stated
validity instant instead of now. Meets AN-006.

- MR-023.1 A record valid from T1 to T2 is returned at an instant inside the
  window and not at one outside it, for both search and fetch.
- MR-023.2 Responses state the validity instant they were answered at.

## Stopping

**MR-030 — Completeness signal.** Every searching or enumerating response states
whether its result set is complete, cut by limit, cut by budget, or empty; and,
when empty or cut, whether any index the query needed was not ready. Meets
AN-007. **[engine]** (bounded match count and saturation).

- MR-030.1 A query matching fewer records than the limit reports `complete`.
- MR-030.2 A query matching more than the limit reports cut-by-limit and a lower
  bound on matches *(ADR: count bounded at 1,000)*.
- MR-030.3 An empty result while a needed index is not ready reports `empty`
  with a not-ready reason naming the index, never a bare empty result.

**MR-031 — Confidence verdict.** Every search response carries a verdict of
`strong`, `weak`, `no_good_match` or `unavailable`, computed only from
similarity-calibrated scores. Rank-fusion scores are rank-only and never feed
the verdict. Meets AN-008. **[engine]** for raw per-arm scores; the rule and
rates are **[ADR]**.

- MR-031.1 Without a calibrated score (no cross-encoder rerank and no raw
  per-arm scores), the verdict is `unavailable`.
- MR-031.2 On the calibration corpus, the false-`strong` rate for absent
  answers and the false-`no_good_match` rate for present answers are each at or
  below the ADR rate.

**MR-032 — No silent degradation.** When an arm is skipped or declined, an index
is not ready, the embedder is unavailable, or a deadline is hit, the same
response names what ran, what did not, and why. Meets AN-007.

- MR-032.1 With the embedder disabled, a search returns lexical results and a
  degradation entry naming the vector arm and the reason.
- MR-032.2 An arm declined through the engine's typed soft fallback appears as
  a degradation entry.
- MR-032.3 No response reports `complete` while carrying a degradation entry
  for an arm that could have contributed.

**MR-033 — Bounded call time.** Every tool call returns within a deadline
**[ADR: 10 s]** with what it has, labelled per MR-032, or a typed timeout.
Meets AN-012.

- MR-033.1 With an engine call artificially blocked past the deadline, the tool
  returns a typed timeout at the deadline, and the adapter keeps serving other
  calls.
- MR-033.2 The number of engine calls in flight never exceeds the adapter's
  admission cap **[ADR]**, which is below the engine reader-pool size; excess
  calls wait or return a typed busy result.

## Context economy

**MR-040 — References and snippets by default.** Search results by default carry
a reference, kind, a bounded snippet, provenance and scores, not full content;
a fetch tool returns full content for references. Meets AN-009.

- MR-040.1 A default search over long records returns no content field over the
  snippet cap *(ADR: 300 characters)*.
- MR-040.2 Fetching the returned references yields content identical to the SDK
  read of the same records under the same view.

**MR-041 — Caller-set budget with continuation.** A caller can set a response
budget measured by the adapter's estimator **[ADR: serialized bytes ÷ 4,
versioned]**; the response fits, says it was cut, and carries a continuation.
Meets AN-009.

- MR-041.1 For any budget at or above the documented minimum, the estimator
  applied to the serialized structured output does not exceed the budget.
- MR-041.2 Following continuations of a search returns every ranked result up to
  the engine's ranked-result cap (100 today, `MAX_RANKED_LIMIT`), with no
  duplicates and no gaps, against one store state; past the cap the response
  says the cap was reached. **[engine]** for ranked pagination beyond the cap.
- MR-041.3 With no budget given, the default **[ADR: 4,000]** applies and is
  stated in the tool description.

**MR-042 — Uniform, legible references.** Every reference any tool returns is
accepted by every tool that takes a reference, and is stable for the life of
the record. Meets AN-010.

- MR-042.1 For a record with a caller-assigned logical id, the reference
  contains that logical id verbatim.
- MR-042.2 References for logical-id records are accepted by fetch, expand and
  explain. References for records without a logical id (content- and
  passage-addressed hits) are accepted by the same tools **[engine]**: today
  `read.get` takes logical ids only.
- MR-042.3 The same record yields the same reference across sessions and
  process restarts.

**MR-043 — Deterministic output.** Identical requests against identical store
state at an identical validity instant produce byte-identical output. Meets
AN-011.

- MR-043.1 Every read tool, called twice in two process lifetimes against an
  unchanged database with the validity instant pinned, yields byte-identical
  `structuredContent` and text.
- MR-043.2 With the validity instant unpinned, the two outputs differ only in
  the fields the interface document names as time-derived (the validity instant,
  and any identifier derived from it); the list of such fields is closed and
  tested.

## Multi-step work

**MR-050 — State identity on every response.** Every read response carries a
state identifier derived from the store state and validity instant only, so two
responses with equal identifiers read the same state. Meets AN-013.

- MR-050.1 Two reads with no intervening commit, with different filters, report
  the same state identifier.
- MR-050.2 After any commit, the next read reports a different identifier.
- MR-050.3 No single response contains results read from two store states.

**MR-051 — Drift is labelled, not hidden.** When the state moved since the
session's previous read, the response says so, whether caused by the agent's
own write or by another writer. Meets AN-013.

- MR-051.1 After an external commit, the next read is answered on the new state
  and carries a `state_advanced` label naming the previous identifier.
- MR-051.2 After a write through the adapter in the same session, the next read
  reflects that write and carries the label.
- MR-051.3 A continuation issued on an earlier state fails as drift with a
  remedy, rather than mixing states.

**MR-052 — Batched references.** A caller can fetch many references in one call,
against one state, with per-item results and errors. Meets AN-014.

- MR-052.1 Fetching N references returns N items in request order, each a record
  or a typed per-item error.
- MR-052.2 One invalid reference does not fail the others.

**MR-053 — Bounded traversal.** Graph expansion from a reference takes caps for
depth, nodes and edges, applies defaults when omitted, and reports what the
caps excluded. Meets AN-014.

- MR-053.1 Expansion never returns more nodes or edges than the caps.
- MR-053.2 When a cap is reached, the response says which cap.

**MR-054 — Actionable errors.** Every error belongs to one class from a closed
set **[ADR]** (proposed: invalid request, not found, not ready, drift, timeout,
busy, conflict, internal) and carries a stable code and one-line remedy;
errors for unknown kinds, fields and references list the nearest valid
candidates. Meets AN-015. **[engine]** for kind enumeration (MR-022).

- MR-054.1 A search naming kind `notes` where kind `note` is declared fails as
  invalid request and names `note`.
- MR-054.2 A well-formed reference to a missing record fails as not found,
  distinct from a malformed reference (invalid request).
- MR-054.3 Candidates are drawn only from operator- or registry-declared names
  (MR-012.2).

**MR-055 — Change queries.** A caller can ask for governed records created,
replaced, retracted or whose validity boundary passed, since a cursor an
earlier response returned. Meets AN-016. **[engine]**: `read.mutations` reads
the op-store, not governed records, and `read.crossed_boundary_since` is keyed
by wall-clock time, not by cursor.

- MR-055.1 After writing three records and replacing one, a change query from
  the earlier cursor returns exactly those four events in commit order.
- MR-055.2 Every read response carries a cursor usable for a change query.

## Writing

**MR-060 — Writes are opt-in.** The adapter is read-only unless the operator
enables writing at launch; with writing disabled, write tools are absent from
`tools/list`. Meets AN-017, AN-002.

- MR-060.1 With default launch options, `tools/list` lists no mutating tool;
  after a session that calls every listed tool, the canonical write cursor and
  a digest of canonical and op-store rows are unchanged.

**MR-061 — Remember, replace, retract.** With writing enabled, a caller can
record a fact, replace a record by reference, and retract a record by reference;
each returns the resulting reference and a cursor. Every record the adapter
writes has a logical id, so it can later be replaced or retracted. Meets
AN-017.

- MR-061.1 A remembered fact is returned by a following search in the same
  session (with the MR-051.2 label).
- MR-061.2 Replacing a record makes the old one report that it was superseded
  and the new one appear in its place.
- MR-061.3 Retracting moves the record to lifecycle state `Deleted`; default
  searches and fetches omit it; a fetch that requests inactive records returns
  it with that state.
- MR-061.4 Replace and retract on a reference without a logical id fail as
  invalid request with a remedy.

**MR-062 — No exact duplicates.** Recording content that exactly matches a
current record of the same kind, under the normalization rule **[ADR]**, returns
the existing reference and creates no new record. Near-duplicates are out of
scope. Meets AN-017. **[engine]** for the match rule.

- MR-062.1 Remembering the same content twice yields one current record and the
  same reference both times.

## Cross-cutting

**MR-070 — Governed semantics preserved.** Every tool is implemented over the
governed `fathomdb-sdk` surface; no tool returns a record that the equivalent
SDK call, under the same view, would hide.

- MR-070.1 For a fixture with erased, retracted, superseded and out-of-validity
  records, each read tool returns exactly the records the equivalent SDK call
  returns under the same `ReadView`.
- MR-070.2 The adapter crate's manifest depends on `fathomdb-sdk` and on no
  other FathomDB crate, nor on `rusqlite`, checked by a manifest lint.

**MR-071 — Typed contract.** Every tool declares input and output schemas, and
every structured output validates against its output schema.

- MR-071.1 Every `structuredContent` in the test suite validates against its
  tool's declared `outputSchema`.
- MR-071.2 All schemas are JSON Schema draft 2020-12.

## Traceability

| Need | Requirements |
| --- | --- |
| AN-001 | MR-010 |
| AN-002 | MR-010, MR-011, MR-012, MR-060 |
| AN-003 | MR-013, MR-061.2, MR-061.3 |
| AN-004 | MR-020, MR-021 |
| AN-005 | MR-022 |
| AN-006 | MR-023 |
| AN-007 | MR-030, MR-032 |
| AN-008 | MR-031 |
| AN-009 | MR-040, MR-041 |
| AN-010 | MR-042 |
| AN-011 | MR-043 |
| AN-012 | MR-003, MR-033 |
| AN-013 | MR-050, MR-051 |
| AN-014 | MR-052, MR-053 |
| AN-015 | MR-054 |
| AN-016 | MR-055 |
| AN-017 | MR-060, MR-061, MR-062 |
| AN-018 | MR-001, MR-002, MR-003, MR-004 |
| AN-019 | MR-014 |
