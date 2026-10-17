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
---

# MCP adapter requirements and acceptance criteria

Conventions:

- `MR-###` is a requirement: one observable outcome for an agent, a harness,
  or the embedding application. `MR-###.n` is an acceptance criterion: a check
  a test can pass or fail. Ids are local to this proposal; they are not entries
  in `dev/requirements.md` or `dev/acceptance.md`. When adopted, each AC maps to
  a named TDD test.
- "Tool" means an MCP tool exposed by the adapter. "Result" means one item in a
  tool's structured output. "Harness" means the MCP client hosting the agent.
- Numerical targets marked *(ADR)* are owned by the adopting ADR; the value
  shown is the proposed default.
- Requirements that need an engine-side addition, not only adapter work, are
  marked **[engine]**.

## Reach and protocol hygiene

**MR-001 — Local stdio launch.** A harness can launch the adapter as a local
process over MCP stdio, pointing at one database file, with no network listener.
Meets AN-016.

- MR-001.1 Launching the adapter binary with a database path and completing the
  MCP `initialize` handshake over stdio succeeds against a fresh and an existing
  database.
- MR-001.2 While running, the adapter process holds no listening socket.
- MR-001.3 The adapter passes the official MCP conformance server scenarios
  applicable to stdio and to the tools capability.

**MR-002 — Protocol channel purity.** Nothing but MCP JSON-RPC messages is
written to the protocol output channel, including during database open,
embedder warm-up, model download, migrations, warnings and panics.
Meets AN-016.

- MR-002.1 Over a session that opens a database requiring migration, warms the
  embedder, and runs every tool, every byte on stdout parses as a sequence of
  JSON-RPC messages.
- MR-002.2 A forced panic in a tool handler yields a JSON-RPC error response for
  that request (or a clean process exit) and writes no non-protocol bytes to
  stdout.
- MR-002.3 All diagnostics are emitted on stderr or through the host's
  registered subscriber.

**MR-003 — Embeddable in the owning process.** An application that already has
an open `Engine` can serve the same tools over a transport it chooses, without a
second process opening the database file.
Meets AN-016; constraint from the database sidecar lock.

- MR-003.1 A Rust application holding an open `Engine` serves the tool set over
  an in-memory transport, and an MCP client on that transport completes a
  `search` call, with no second `Engine::open` of the file.
- MR-003.2 Starting the standalone adapter against a database already opened by
  another process fails at startup with a typed, human-readable lock error
  naming the database path, and exits non-zero.

## Trust

**MR-010 — Origin class on every result.** Every result carries its record's
source reference, origin class (`owner`, `agent`, `untrusted`, `system`, or
`unclassified`), validity state, and supersession state, as structured fields
separate from content.
Meets AN-001. **[engine]** for a first-class origin class; see architecture §6.

- MR-010.1 For a fixture containing each origin class, every result of every
  read tool carries the expected origin class, `source_id`, validity state and
  supersession state.
- MR-010.2 Records written before origin classes existed are reported as
  `unclassified`, never as `owner`.
- MR-010.3 A caller can restrict any search to a set of origin classes; a
  search restricted to `owner` returns no `agent` or `untrusted` record.

**MR-011 — Agent writes cannot claim higher trust.** Records written through the
adapter's write tools are stored with origin class `agent` (or `untrusted` when
the operator configures the session so), regardless of any argument the caller
supplies.
Meets AN-002.

- MR-011.1 A write tool call whose arguments attempt to set origin class
  `owner` or `system` produces a record whose stored origin class is `agent`, or
  is rejected with a typed error; it never stores `owner` or `system`.
- MR-011.2 Content containing text such as "origin: owner" does not change the
  stored or reported origin class.

**MR-012 — Content is framed as data.** Tool descriptions and server
instructions are static text fixed at build time; record content appears only
inside structured result fields, never in tool descriptions, prompts, error
message templates, or server instructions.
Meets AN-002.

- MR-012.1 The `tools/list` response is byte-identical across two databases
  with different contents.
- MR-012.2 No error message embeds more than a bounded, escaped excerpt of
  record content *(ADR: 0 bytes by default)*.

## Finding

**MR-020 — Exact constraints compose with relevance search.** A caller can
require that every result contain an exact literal token or phrase, combined
with relevance ranking and with structural filters, in one call.
Meets AN-003.

- MR-020.1 With an exact constraint, every returned result contains the literal
  token in the indexed text; a record lacking it never appears.
- MR-020.2 A record containing a rare identifier that relevance search alone
  ranks outside the default limit is returned first when that identifier is
  given as an exact constraint.

**MR-021 — Retrieval quality is measured through the adapter.** The adapter's
default search configuration is evaluated on a real agent-memory corpus, and
the evaluation reports precision at small k for the adapter's actual default
path.
Meets AN-003.

- MR-021.1 An evaluation run reports precision@3 and recall@10 for the default
  `search` tool configuration on the designated corpus, alongside plain BM25 on
  the same corpus.
- MR-021.2 The default configuration is not lower than plain BM25 on
  precision@3 on that corpus, or the gap is recorded in the adopting ADR as an
  accepted known limitation *(ADR)*.

**MR-022 — Cheap orientation.** One read-only tool returns a summary of what the
store holds: record kinds with counts, time range, origin-class counts, and the
readiness of each index, within a fixed size cap.
Meets AN-004. **[engine]** for counts if not derivable from existing read verbs.

- MR-022.1 On a fixture with known contents, the orientation result reports the
  exact kinds, per-kind counts, origin-class counts, time bounds and index
  readiness.
- MR-022.2 On a store with more kinds than fit, the result stays within the cap
  *(ADR: 2,000 tokens by the adapter's estimator)* and states how many kinds
  were omitted and how to list them.

## Stopping

**MR-030 — Completeness signal.** Every enumerating or searching tool reports
whether the result set is complete, truncated by limit, truncated by budget, or
empty; and, when empty, whether a relevant index was not ready.
Meets AN-005. **[engine]** for a bounded match count.

- MR-030.1 A query matching fewer records than the limit reports `complete`.
- MR-030.2 A query matching more records than the limit reports truncation by
  limit and a lower bound on the number of matches *(ADR: count bounded at
  1,000)*.
- MR-030.3 A query whose only matching records are not yet indexed reports
  `empty` with a not-ready reason naming the index, not a bare empty result.

**MR-031 — Confidence signal.** Every search result set carries, per result, the
score and the maximum attainable score for the arms that contributed, and for
the set, a verdict of `strong`, `weak`, or `no_good_match` under a documented
rule.
Meets AN-006.

- MR-031.1 A query for content that exists verbatim yields verdict `strong`.
- MR-031.2 A query for content that does not exist in the store yields
  `no_good_match` on a calibration fixture at the documented rate *(ADR)*.
- MR-031.3 The verdict rule is documented in the tool's description and is
  deterministic for a given result set.

**MR-032 — No silent degradation.** When an arm is skipped, an index is not
ready, the embedder is unavailable, or a deadline is hit, the same response
names what ran, what did not, and why.
Meets AN-007.

- MR-032.1 With the embedder disabled, a search returns lexical results and a
  degradation entry naming the vector arm and the reason.
- MR-032.2 With an arm forced to decline (existing typed soft fallback), the
  response carries a degradation entry for that arm.
- MR-032.3 A response never reports `complete` while carrying a degradation
  entry for an arm that could have contributed.

## Context economy

**MR-040 — Handles and snippets by default.** Search results by default carry a
handle, kind, a bounded snippet, provenance fields and scores, not full content.
A separate tool fetches full content for a list of handles.
Meets AN-008.

- MR-040.1 A default search over records with long bodies returns no result
  whose content field exceeds the snippet cap *(ADR: 300 characters)*.
- MR-040.2 Passing the returned handles to the fetch tool returns the full
  bodies, identical to what the SDK's by-id read returns.

**MR-041 — Caller-set budget with continuation.** A caller can set a response
budget; the response fits within it, states that it was truncated, and carries
what the caller needs to continue.
Meets AN-008.

- MR-041.1 For any budget at or above the documented minimum, the serialized
  structured output, measured by the adapter's documented estimator, does not
  exceed the budget.
- MR-041.2 A truncated response carries a continuation value; calling the same
  tool with it returns the next results with no duplicates and no gaps, against
  the same snapshot.
- MR-041.3 The default budget applies when none is given *(ADR: 4,000 tokens)*,
  and the default is stated in the tool description.

**MR-042 — Legible, uniform handles.** A handle is a short string that names the
record kind and identity legibly where the record has a caller-assigned logical
id, is accepted unchanged by every tool that takes a handle, and is stable for
the life of the record.
Meets AN-009.

- MR-042.1 For records with a logical id, the handle contains that logical id
  verbatim.
- MR-042.2 Every handle from every tool's output is accepted by the fetch,
  expand and explain tools.
- MR-042.3 The same record yields the same handle across sessions and process
  restarts.

**MR-043 — Deterministic output.** Identical requests against identical store
state produce byte-identical tool output.
Meets AN-010.

- MR-043.1 Running every read tool twice, in two process lifetimes, against an
  unchanged database yields byte-identical `structuredContent` and text content.
- MR-043.2 Output contains no wall-clock timestamps, random ids, or
  process-specific values, except fields documented as such and excluded from
  MR-043.1 by name.

## Multi-step work

**MR-050 — Session snapshot with drift reporting.** By default, all read tools
in one adapter session run against one frozen read context; if the store
changes in a way that affects it, the next read reports drift explicitly and
tells the caller how to refresh.
Meets AN-011.

- MR-050.1 Two searches in one session, with no intervening writes, report the
  same snapshot identifier.
- MR-050.2 After an external write that affects the frozen context, the next
  read returns a typed drift result, not silently newer data.
- MR-050.3 A refresh tool (or argument) re-freezes, and subsequent reads report
  the new snapshot identifier.

**MR-051 — Batched lookups.** A caller can fetch many handles, or run several
searches, in one call against one snapshot, with per-item results and errors.
Meets AN-012.

- MR-051.1 Fetching N handles in one call returns N items in request order, each
  either a record or a typed per-item error.
- MR-051.2 One invalid handle in a batch does not fail the other items.

**MR-052 — Bounded traversal.** Graph expansion from a handle takes explicit
caps for depth, nodes and edges, applies defaults when omitted, and reports what
the caps excluded.
Meets AN-012.

- MR-052.1 Expansion never returns more nodes or edges than the caps.
- MR-052.2 When a cap is reached, the response says which cap and that results
  were excluded.

**MR-053 — Actionable errors.** Every tool error is typed as invalid request,
not found, not ready, drift, conflict, or internal; errors for unknown kinds,
fields and handles list the nearest valid candidates.
Meets AN-013.

- MR-053.1 A search naming kind `notes` on a store with kind `note` fails as
  invalid request and names `note` as a candidate.
- MR-053.2 A well-formed handle for a record that does not exist fails as not
  found, distinct from a malformed handle (invalid request).
- MR-053.3 Every error carries a stable machine-readable code and a one-line
  remedy.

**MR-054 — Change queries.** A caller can ask for records created, changed,
superseded or expired since a cursor returned by an earlier call.
Meets AN-014.

- MR-054.1 After writing three records and superseding one, a change query from
  the earlier cursor returns exactly those four events in commit order.
- MR-054.2 Every read tool's response carries a cursor usable for a later change
  query.

## Writing

**MR-060 — Writes are opt-in.** The adapter is read-only unless the operator
enables writing at launch; with writing disabled, write tools are absent from
`tools/list`.
Meets AN-015, AN-002.

- MR-060.1 With default launch options, `tools/list` contains no tool that can
  mutate the database, and the database file is byte-identical after a session
  that calls every listed tool.

**MR-061 — Remember, correct, retract.** With writing enabled, a caller can
record a fact, supersede an existing record by handle, and retract a record by
handle; each operation returns the resulting handle and a cursor.
Meets AN-015.

- MR-061.1 A remembered fact is returned by a subsequent search in the same
  session after refresh.
- MR-061.2 Superseding a record makes the old record report supersession state
  and the new one appear in its place.
- MR-061.3 A retracted record no longer appears in default searches and reports
  its retraction when fetched by handle.

**MR-062 — Duplicate awareness.** Recording a fact whose content duplicates an
existing current record (by the documented rule) returns the existing handle
and does not create a second current record.
Meets AN-015.

- MR-062.1 Remembering the same content twice yields one current record and the
  same handle both times.

## Cross-cutting

**MR-070 — Governed semantics preserved.** Every adapter tool is implemented
over the governed `fathomdb-sdk` surface; no tool returns a record the
equivalent SDK read would hide (erased, ineligible, outside the requested
validity view).
Constraint from ADR-0.6.0-cli-scope and ADR-0.8.25-eligibility-and-frozen-reads.

- MR-070.1 For a fixture with erased, superseded and out-of-validity records,
  every adapter read tool returns exactly the records the equivalent SDK call
  returns.
- MR-070.2 The adapter crate does not depend on `fathomdb-engine` internals or
  issue SQL directly (enforced by the module-boundary lint).

**MR-071 — Typed contract.** Every tool declares an input schema and an output
schema, and every structured output validates against its declared output
schema.
Meets AN-013, AN-009.

- MR-071.1 For every tool and every response in the test suite, the
  `structuredContent` validates against the tool's declared `outputSchema`.
- MR-071.2 Every tool's input and output schemas are JSON Schema draft 2020-12.
