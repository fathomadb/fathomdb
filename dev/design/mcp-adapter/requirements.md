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
- "Store state" means the engine's read-visibility generation. Triggers advance
  it on canonical, source, dependency, projection, vector and op-store tables,
  so it moves both when records change and when a projection commit changes
  what is searchable. It is monotonic. The canonical write boundary
  (`MAX(write_cursor)` over four tables) and the projection cursor are read
  beside it only to classify the cause. They are not monotonic proofs: the write
  boundary can fall after a purge.
- Test seams for delays and blocked calls (MR-003, MR-033) live in the
  adapter's own bridge, behind a test-only feature of the adapter crate; they
  need no engine hook.

## Reach and protocol hygiene

**MR-001 — Local stdio launch.** A harness can launch the adapter as a local
process over MCP stdio, configured with a store registry (MR-005) rather than
one database file, with no network listener. Meets AN-018.

- MR-001.1 Launching the binary with a registry naming a fresh and an existing
  database, and completing the MCP `initialize` handshake, succeeds; both
  stores are then usable.
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

**MR-003 — Responsive before a store is ready.** The handshake and
`tools/list` complete without waiting for any engine open, migration or embedder
warm-up; tool calls made to a store before it is ready return a typed not-ready
result for that store only. Meets AN-012, AN-018.

- MR-003.1 With engine open delayed indefinitely by the bridge's test seam,
  `initialize` and `tools/list` complete within 1 s. (Harness handshake
  timeouts are the harness's; 10 s is the assumed minimum.)
- MR-003.2 A search issued during warm-up returns not-ready with the phase
  named, not an error and not an empty result.
- MR-003.3 When a store's open fails (lock held, corruption, migration refusal),
  every call to that store returns the typed open error with its remedy, and
  stderr carries the same; calls to other stores are unaffected, and the process
  does not exit until the harness closes it.

**MR-004 — Embeddable in a Rust host.** A Rust application that holds open
`Engine`s can register them in the adapter's store registry and serve the tools
over a transport it chooses, so the application and its agents share stores
without a second opener. Embedded mode is the same server used as a Rust
library (owner decision, 2026-10-17). Meets AN-018.

- MR-004.1 A Rust application holding two open `Engine`s registers both under
  names, serves the tool set over an in-memory transport, and a client completes
  a search against each; no second `Engine::open` of either file occurs.
- MR-004.2 A standalone call to a store whose file another process holds yields
  the typed lock error per MR-003.3, naming the store, and other stores keep
  working.

**MR-005 — Addressable stores.** The adapter serves any number of FathomDB
database files. The operator defines which files are addressable, either as
named entries or as a root directory whose database files are addressable by
relative name. The agent addresses a store by that name on every call, or
through a session default, and can list the addressable stores. The agent never
supplies a filesystem path. Meets AN-020.

- MR-005.1 With a registry of three stores, the same tool called with each store
  name returns that store's records only.
- MR-005.2 A listing tool returns every addressable store name, with its
  open/closed and readiness state, and no filesystem paths.
- MR-005.3 A name that is not addressable, including one containing path
  separators, `..`, or an absolute path, fails as invalid request, and no file
  outside the registry is opened or created.
- MR-005.4 A store is opened on first use and closed after it has been idle for
  the configured timeout, which releases its sidecar lock. The timeout is
  configurable, defaults to 60 seconds, and 0 means never close (owner
  decision, 2026-10-17). Any call to the store resets the idle clock. A store
  with calls in flight, including abandoned ones still running, is never idle.
- MR-005.7 **One open store at a time** (owner decision, 2026-10-17). A call to
  store B while store A is open closes A, once A has no calls in flight, and
  then opens B. Calls to A that arrive during the switch wait or return busy;
  they never run against B. The open-store limit is a registry setting fixed at
  1 in this release; the registry manages a set of open stores internally, so
  raising the limit later changes configuration, not structure.
- MR-005.5 The agent cannot create a new database file unless the operator's
  registry explicitly allows creation for that name.
- MR-005.6 All state identity, cursors and continuations (MR-041, MR-050,
  MR-055) are per store. A continuation or cursor issued for one store fails as
  invalid request on another.

**MR-006 — Clean reconnection.** When a closed store is used again, the adapter
reopens it and restores every piece of per-engine and per-session state, so the
agent sees no difference except labelled changes the store itself underwent.
Reconnection needs no MCP-level refresh. Meets AN-013, AN-018.

- MR-006.1 After an idle close and reopen, every tool behaves as before the
  close, against the same database, with the same launch policy.
- MR-006.2 Engine-level settings the adapter applied at open — telemetry sink,
  diagnostics subscriber, profiling and slow-query thresholds — are re-applied
  on every reopen; a test asserts each is active after reopen.
- MR-006.3 A continuation, cursor or state identifier issued before the close
  is honoured after reopen exactly as if the store had stayed open: if the store
  is unchanged, the next page continues; if another process changed it while
  closed, the response carries `state_advanced` (MR-051).
- MR-006.4 During reopen the store reports not-ready with phase `reopening`
  (MR-003.2), and a call that carries an MCP progress token receives progress
  notifications until the store is ready.
- MR-006.5 Feedback (MR-024) naming a response issued before the close returns
  `not_recorded` with reason `store_reopened`, never an error, and nothing is
  written to any sink. The adapter enforces this itself: each response context
  records the store's open epoch, and feedback from an earlier epoch is
  refused. The engine cannot enforce it: its telemetry nonce is always 0 and
  its sequence restarts on every enable, so a pre-close query id is accepted by
  `record_feedback` and would be credited to whichever new search reused it.
- MR-006.6 A failed reopen (for example, another process took the lock while
  the store was closed) yields the typed open error for that store, and the next
  call retries the open; it does not stay failed for the life of the process.
- MR-006.7 `tools/list` and server capabilities are byte-identical before and
  after any close, reopen or store switch, so no `notifications/tools/list_changed`
  is sent or needed.

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
- MR-012.2 No error message contains record content. Kind names in errors come
  only from the engine's kind enumeration (MR-022), whose kinds are set by
  writers, never by content; field names come only from the projection
  registry. In write mode, adapter writes may use only kinds on the operator's
  launch allow-list, so an agent cannot mint a kind name that other sessions'
  errors would echo.

**MR-013 — Currency on every result.** Every result states its validity window
relative to the response's validity instant and whether a newer record
superseded it. Meets AN-003. **[engine]** for search hits and for fetch:
`SearchHit` carries id, kind, body, scores and `source_id`, and `NodeRecord`
carries `logical_id`, `kind`, `body` and `write_cursor`; neither carries
validity or supersession fields, and `read.get` materialises active rows only.

- MR-013.1 On a fixture with a superseded record and an expired record, every
  read tool that returns them, under a view that includes them, reports the
  supersession and the expiry.
- MR-013.2 Under the default view, neither appears in search results.

**MR-014 — Operator-bound scope.** The operator can bind an adapter session to
a scope; every tool honours it inside the engine, and no tool argument can
widen it. Meets AN-019. **[engine]**: today eligibility reaches only the search
family; `read.get`, `read.get_many`, `read.list` predicates, `graph.neighbors`,
aggregates and change queries take none. Until the engine applies scope to
every read path, the adapter refuses to start with a scope configured (fails
closed), rather than filtering in the adapter.

- MR-014.1 With a session bound to scope A, no read tool returns a record
  outside A, including by direct reference and through neighbours of an
  in-scope record.
- MR-014.2 A reference or filter outside A yields not found, indistinguishable
  from a record that does not exist. This is a deliberate exception to AN-015's
  "valid, no match" distinction.
- MR-014.3 Counts (MR-022) and change queries (MR-055) in a scoped session
  reflect only records in A.
- MR-014.4 Launching with a scope configured, on an engine without scoped point
  reads, fails at startup with a typed error naming the missing capability. Two
  checks apply: a build-time capability of the linked SDK, which runs before
  the handshake, and a schema check after open (from the open report), whose
  failure is reported through the MR-003.3 path, since a file can predate the
  schema step even when the SDK supports it.
- MR-014.5 Records written through the adapter carry the session's scope as
  stamped by the engine from launch policy, not from the body; an agent cannot
  write into another scope.

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

**MR-024 — "This hit was used" feedback.** An agent can report which results
from a search it actually used. The adapter records the report and does
nothing else with it: no ranking, filtering or default changes because of it.
What FathomDB later does with the signal is undecided (owner, 2026-10-17).
Meets AN-021.

- MR-024.1 A feedback call that names a search response and some of its
  references is accepted. When the operator has enabled telemetry and the
  response came from a hybrid search, the report reaches the engine's existing
  feedback record (`record_feedback`) with label source `agent`. Text-only
  searches are never captured by engine telemetry, so feedback on them returns
  `not_recorded` with reason `text_mode`.
- MR-024.2 With telemetry disabled, the call returns `not_recorded` with that
  reason, not an error.
- MR-024.3 Search output for the same request on the same state is
  byte-identical before and after any amount of feedback (MR-043).
- MR-024.4 Feedback is available in read-only mode, because it writes only to
  the telemetry sink and not to the database (MR-060.1 still holds).
- MR-024.5 Feedback naming a response from another store, or references not in
  that response, fails as invalid request.
- MR-024.6 Under concurrent hybrid searches, feedback is credited to the search
  that produced the named response. While telemetry is enabled, the adapter runs
  hybrid searches with `explain=true` and takes the query id from that search's own
  `explanation.correlation_id`, which the engine assigns under the sink lock. It
  never uses `last_telemetry_query_id`, which is engine-wide and can be swapped
  between concurrent searches. The explanation is dropped from the response
  unless the caller asked for it, so output is unchanged (MR-043).
- MR-024.7 Every telemetry query id the adapter records is unique within its
  sink file. The adapter enables a fresh sink file on every open, named from the
  operator's telemetry directory, the store and the open epoch, because
  re-enabling on the same file would repeat ids (sequence restarts at 0). A
  test runs two opens of one store with feedback in each and asserts that every
  feedback record joins to exactly one capture event.

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

- MR-033.1 The deadline bounds waiting and starting; a started call runs to
  completion and returns its result (owner decision 2026-10-18;
  `dev/design/agent-access/queue-admission.md` § 6.1). With an engine call
  blocked past the run limit by the bridge's test seam, the tool returns a
  typed timeout at the run limit, measured from the call's admission, and the
  adapter keeps serving other calls. A client cancel answers the call at once
  with a typed result: a waiting call leaves the queue, and a running call is
  abandoned while its read runs to its end and keeps its admission slot.
- MR-033.2 The number of engine calls in flight never exceeds the adapter's
  admission cap **[ADR]**; excess calls wait, in arrival order, while their
  deadline allows, then return a typed busy result (owner decision
  2026-10-18; `dev/design/agent-access/queue-admission.md` § 6.1). In
  standalone mode the default cap is below the engine reader-pool size; in
  embedded mode the host sets it, because the host shares the same pool.

## Context economy

**MR-040 — References and snippets by default.** Search results by default carry
a reference, kind, a bounded snippet, provenance and scores, not full content;
a fetch tool returns full content for references. Meets AN-009.

- MR-040.1 A default search over long records returns no content field over the
  snippet cap *(ADR: 300 characters)*.
- MR-040.2 Fetching the returned references yields content identical to the SDK
  read of the same records under the same view.

**MR-041 — Caller-set budget with continuation.** A caller can set a response
budget measured by the adapter's estimator, serialized bytes ÷ 4 (owner
decision, 2026-10-17, as a starting rule of thumb; versioned so it can change).
The response fits, says it was cut, and carries a continuation.
Meets AN-009.

- MR-041.1 For any budget at or above the documented minimum, the estimator
  applied to the serialized structured output does not exceed the budget.
- MR-041.2 With no state advance between pages, following continuations of a
  search returns every ranked result up to the engine's ranked-result cap (100
  today, `MAX_RANKED_LIMIT`) with no duplicates and no gaps; past the cap the
  response says the cap was reached. **[engine]** for ranked pagination beyond
  the cap.
- MR-041.4 When the state advanced between pages, the next page is re-derived on
  the new state, contains no reference already returned, and is labelled
  `state_advanced` with the caveat that results which moved earlier may be
  missing (MR-051.3).
- MR-041.5 A continuation is an opaque, adapter-authenticated value that the
  agent passes back unchanged. Its size is bounded because it records at most
  the ranked-result cap of references. It contains no record content. A
  tampered continuation fails as invalid request.
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
  the validity-instant field, and in results whose validity window boundary
  fell between the two calls. No identifier is derived from the instant.

## Multi-step work

**MR-050 — State identity on every response.** Every read response carries a
state identifier derived from the store state only, and the validity instant as
a separate field. Two responses with equal identifiers read the same visible
store, including what was searchable. Meets AN-013. **[engine]**: a
content-free read, on a reader connection, of the visibility generation, the
canonical write boundary and the projection cursor. `freeze_read_context` cannot
serve: it takes the writer-connection lock and counts every canonical row.
Until the read exists, responses report the identifier as `unavailable`.

- MR-050.1 Two reads with no intervening commit of any kind, with different
  filters and an unpinned validity instant, report the same identifier.
- MR-050.2 After a canonical commit, or a projection commit that changes
  visibility, the next read reports a different identifier.
- MR-050.3 No response contains results read from two store states. When the
  adapter cannot show a single state, it returns a drift error and no results.

**MR-051 — Drift is labelled, not hidden.** When the store state moved since the
session's previous read, the response says so, and gives the cause where it can:
`canonical` when the write boundary changed, `index` when only the projection
cursor changed, `unspecified` otherwise. This holds whether the agent's own
write or another writer moved the state. Meets AN-013.

- MR-051.1 After an external canonical commit, the next read is answered on the
  new state and carries a `state_advanced` label naming the previous identifier,
  with cause `canonical`. After a background embedding batch alone, the label's
  cause is `index`.
- MR-051.2 After a write through the adapter in the same session, the next read
  reflects that write and carries the label.
- MR-051.3 A continuation issued on an earlier state is re-derived per MR-041.4.
  It fails as drift only when it cannot be re-derived, for example because the
  request's own parameters became invalid.

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

- MR-054.1 A search naming kind `notes` on a store whose kind enumeration
  contains `note` fails as invalid request and names `note`.
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
writes has a logical id, so it can later be replaced or retracted, and a kind
from the operator's launch allow-list. Meets AN-017. The write family is not in
the first release: it waits for engine-stamped origin class (MR-011) and scope
(MR-014.5).

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
SDK call, under the same view and eligibility, would hide. The adapter never
filters records itself.

- MR-070.1 For a fixture with erased, retracted, superseded and out-of-validity
  records, each read tool returns exactly the records the equivalent SDK call
  returns under the same `ReadView` and eligibility.
- MR-070.2 The adapter crate's manifest depends on `fathomdb-sdk` and on no
  other FathomDB crate, nor on `rusqlite`, checked by a manifest lint.

**MR-071 — Typed contract.** Every tool declares input and output schemas, and
every structured output validates against its output schema.

- MR-071.1 Every `structuredContent` in the test suite validates against its
  tool's declared `outputSchema`.
- MR-071.2 All schemas are JSON Schema draft 2020-12.

**MR-072 — Hints sidecar.** Results carry a `hints` array of suggested next
calls authored by FathomDB, so an agent can act on a cut, an empty or degraded
search, or a default store without inferring it from flags
(ADR-0.8.29-agent-hints-sidecar; `dev/design/agent-access/hints-sidecar.md`).
Meets AN-009.

- MR-072.1 Data-free: every hint string is a closed-enum word, an adapter
  handle, an operator store name or a ref from the same response; no record
  content, kind names, `source_id` or SDK error text.
- MR-072.2 Separate and advisory: hints live only in `hints`, never replace a
  typed field, and ignoring them never makes a result wrong.
- MR-072.3 Derived: computed only from values the call already holds; no
  extra query, read, embed or lock.
- MR-072.4 Deterministic: identical requests on identical state with the
  same hints setting give byte-identical hints (MR-043).
- MR-072.5 Bounds: at most one hint per reason; a cut-ref hint lists every
  cut ref up to 50 and counts the rest, and refs over 128 bytes are left out
  and counted.
- MR-072.6 Configuration: per call, then the `hints` config key, then on.
  With hints off, every output is byte-identical to the output without
  hints.
- MR-072.7 Budget: hints count against `max_tokens` (MR-041.1 holds with
  them). A `search` page may keep fewer results, but never none because of
  hints. A `fetch` whose items fit only without the `body_cut` hint is still
  refused, as when they cannot fit at all.

## First-release scope

The first release ships only what the 0.8.27 SDK supports plus one engine
addition, the state read (MR-050). It is read-only, with no scope
binding.

| Group | Requirements |
| --- | --- |
| In the first release, SDK as-is | MR-001, MR-002, MR-003, MR-004, MR-005, MR-006 (except .3), MR-012.1, MR-021, MR-023, MR-024, MR-032, MR-033, MR-040, MR-041.1/.3/.5, MR-042.1/.3, MR-043, MR-052, MR-053, MR-054.2, MR-060, MR-070, MR-071 |
| In the first release, needs the state read | MR-006.3, MR-041.2/.4, MR-050, MR-051.1/.3 |
| Reported `unavailable` until its engine addition lands | MR-010, MR-012.2 and MR-054.1/.3 (no kind candidates without the kind enumeration), MR-013, MR-020, MR-022, MR-030, MR-031, MR-042.2, MR-055 |
| Startup refused until its engine addition lands | MR-014 |
| Not shipped until origin and scope stamping land | MR-011, MR-051.2, MR-061, MR-062 |

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
| AN-009 | MR-040, MR-041, MR-072 |
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
| AN-020 | MR-005, MR-006 |
| AN-021 | MR-024 |
