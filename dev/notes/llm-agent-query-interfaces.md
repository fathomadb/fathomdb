---
title: How LLM agents query for data, and what interfaces would serve them better
date: 2026-10-16
status: research note (non-canonical; see dev/notes/README.md)
purpose: Answer "how do LLMs and agents query for data?" from the agent's side of the tool boundary, then name the interface properties that would improve an agent's ability to query, and map each against the 0.8.27 FathomDB surface.
inputs:
  - dev/interfaces/python.md (0.8.27 governed surface)
  - src/python/fathomdb/engine.py, read.py, graph.py, filter.py, types.py
  - dev/design/0.8.0-agent-memory-fit.md (earlier read-surface analysis)
  - dev/design/0.8.8-explain-and-telemetry-adr.md (explain sidecar)
  - dev/notes/context-research-agentic-best-practices.md (context-engineering synthesis)
---

# How LLM agents query for data, and what interfaces would serve them better

> This note is written from the perspective of an agent that spends most of its
> working life issuing queries through a tool boundary. It is an argument about
> interface shape, not a plan. Nothing here commissions work; §5 lists candidate
> gaps so a future release plan can weigh them (`ROADMAP.md` posture).

## 1. The question, restated

Every existing lookup method — `grep`, `sed`/`awk`, SQL, vector search, graph
query, warehouse, web search, REST/RPC APIs — was designed for a human operator
who has persistent working memory, cheap eyes, millisecond-latency iteration,
and tacit knowledge of the schema. An agent can drive all of them, and does.
The question is: given the ways an agent is *not* a human operator, which
interface properties would most improve its ability to find the right data,
know that it found it, and stop?

## 2. How agents actually query today

The table records what each method is good at, why agents reach for it, and
where it hurts an agent specifically. The "hurts" column is the design input.

| Method | What it gives | Why agents reach for it | Where it hurts an agent |
| --- | --- | --- | --- |
| `grep` / `rg` / `glob` | Exact lexical or regex match; returns *locations* (`file:line`), not content | Zero schema; deterministic; cheap; addressable results; composable in a pipe | No ranking or semantics; floods context on common tokens; needs iteration when the exact token is unknown |
| `sed` / `awk` / `jq` | Stream projection and transformation | Shapes output to a token budget; extracts one field from many | Easy to get wrong silently; no feedback that the projection dropped what mattered |
| Read with offset/limit | A window over a file | The only way to "hydrate" after a locate step | The agent chooses the window blind; no signal where the interesting part is |
| SQL | Declarative set semantics; aggregation; joins; `EXPLAIN` | Models are strong at SQL *when the schema is in context*; exactness; counts | Schema must be known or hallucinated; join paths and dialects drift; unbounded result sizes; no semantic match |
| Vector / kNN search | Semantic nearest neighbours | Finds things the agent cannot name exactly | No completeness signal ("is that all?"), no "nothing matches" signal, scores not comparable across queries, `k` is arbitrary, results are bodies not handles |
| Graph (Cypher, SPARQL, BFS) | Multi-hop relations; typed edges | "Neighbours of X" is a natural agent question | Query languages are rare in training data; traversals are unbounded unless the interface bounds them; schema of edge kinds is implicit |
| Warehouse / OLAP | Aggregates over large sets | Answers "how many / how distributed" without enumeration | Latency and cost are invisible until the query runs |
| Web search | Ranked snippets, then fetch | Two-phase locate → hydrate is exactly the right shape for a token budget | Noisy; freshness and provenance are weak; no structured constraints |
| Typed tool / API call (MCP, function calling) | Fixed verbs with JSON-schema arguments | The dominant agent interface; the schema doc *is* the agent's model of the system | Each API is bespoke; many narrow verbs are hard to compose; errors rarely say what to do next |
| Language server / tree-sitter | Structure-aware navigation (definitions, references) | Beats text search for code because it follows typed relations, not strings | Code-only; the analogue for data is a typed edge graph |
| Key-value / by-id | A record for a handle | Round-trips a handle from an earlier result | Only useful if earlier results carried handles |

Three lessons fall out of the table and recur in every recommendation below:

- **The grep lesson.** Agents love `grep` because it is exact, returns handles
  rather than payloads, is cheap enough to iterate, and needs no schema. Most
  vector interfaces invert every one of those properties.
- **The SQL lesson.** Declarative set semantics plus `EXPLAIN` plus a
  discoverable catalogue is a near-ideal agent interface, *provided the
  catalogue fits in the context window*. Text-to-SQL failures are mostly
  catalogue failures, not reasoning failures.
- **The web-search lesson.** A ranked list of snippets with handles, followed by
  selective fetch, is the most token-efficient retrieval shape in common use.
  Returning full bodies for the top-k is the expensive default.

## 3. How an agent differs from the operator these tools were built for

- **Context is the scarce resource, not latency.** Every returned byte costs
  tokens now and attention later ("lost in the middle"). A human scrolls past
  an over-fetch; an agent pays for it twice.
- **No working memory outside the window.** Schema, earlier result sets, and
  intermediate state must either be re-fetched or held in context. Humans keep
  these in their heads.
- **Iteration is a round trip.** Each refinement costs a tool call, tokens, and
  a chance to derail. Interfaces that fold a likely second query into the first
  response pay for themselves.
- **Identifiers get hallucinated.** Column names, kinds, ids, paths. An
  interface that validates and suggests converts a hallucination into a
  one-step recovery; one that silently returns empty converts it into a false
  "not found".
- **Structured output is a strength; rare syntax is a weakness.** Agents emit
  JSON arguments reliably and SQL well; they emit Cypher or SPARQL poorly.
  Natural-language-in is *not* the best agent interface either: it adds a
  second interpretation layer with its own ambiguity.
- **Stopping is the hard part.** An agent needs to know whether a result set is
  complete, whether a miss means "absent" or "not indexed yet", and whether to
  reformulate or give up. Humans infer this from the shape of the results.
- **Plans span many calls.** Pagination, follow-up reads, and citations must
  refer to the same snapshot, or the plan's premises shift underneath it.
- **Results must be actionable.** A body string is a dead end. A handle can be
  passed to get, expand, explain, or cite.

## 4. Interface properties that would improve an agent's ability to query

Ordered by how much the author believes each would change agent behaviour.
Each item names the human-tool analogue it generalises.

### 4.1 A cheap, compact, self-describing catalogue

One read-only verb that answers "what can I ask here?": kinds and their counts,
attribute and projection names with example values and cardinality, edge kinds,
which indexes exist and their readiness, the id spaces, the time bounds of the
data. Token-budgeted and rankable by relevance to a hint string so it fits in
context even for large stores. This is the `\d` of `psql` and the schema
listing every text-to-SQL harness prepends, made first-class. It is the single
largest lever on query correctness because it removes the hallucinated
identifier at the source.

### 4.2 Two-phase retrieval with handles, and a token budget instead of a row limit

Return handles plus a short snippet by default; let the agent hydrate the ones
it wants. Accept `max_tokens` (or `max_bytes`) alongside `limit`, and let the
engine decide what to shed: body truncation with an explicit marker and the
handle to fetch the rest, fewer hits, or snippet-only. This is web search's
results page and `grep -l`, applied to semantic retrieval.

### 4.3 Completeness, confidence, and no-match signals

Per query: a total or bounded match count; whether the result saturated the
limit; which retrieval arms contributed and which returned nothing; a score
distribution or top-gap so the agent can tell a confident hit from a
best-of-noise; and an explicit "no good match" verdict rather than a weak top-k.
Per store: whether the index the query hit is ready, stale, or partial, so a
miss can be classified as *absent* versus *not yet visible*. Vector search
offers none of this today; SQL offers counts; `grep` offers exit code 1.

### 4.4 One query, engine-chosen plan, with explain in both directions

The agent states what it is looking for — free text, exact tokens it is sure
of, closed structural constraints, a time window — and the engine routes across
lexical, vector, filter, and graph arms, then reports which arms fired and why
each hit ranked. Add the negative direction: "why did handle X not appear?"
(filtered out, out of validity window, not indexed, below cut). Positive explain
tells the agent whether to trust; negative explain tells it whether to
reformulate or stop. This is SQL's `EXPLAIN` extended to retrieval.

### 4.5 Exactness alongside semantics

Agents frequently know an exact token (an error string, an identifier, a quoted
phrase) and want it as a hard constraint while the rest of the query stays
fuzzy. A closed predicate grammar — equality, prefix, substring on named
attributes, range on time — composed with semantic search is the `grep | fzf`
pipeline collapsed into one call. Negative and absence queries ("confirm there
is no record of X") need the exact arm and a count; they are unanswerable by
nearest-neighbour alone.

### 4.6 Stable snapshots across a plan

A frozen read context that pagination, by-id reads, graph expansion, and
citations all run against, with a correlation id, so that a ten-call plan sees
one world. Humans tolerate drift because they notice it; agents do not notice.

### 4.7 Handles, provenance, and validity on every result

Every hit carries a typed id that round-trips into every other verb, its source
reference, creation and validity bounds, and supersession state. This is what
makes a hit citable and makes the next call possible without a second search.

### 4.8 Typed errors that name the recovery, and suggestions on miss

"Unknown kind `notes`; known kinds are `note`, `task`" is worth more than a
clean empty result. Fuzzy-matching identifiers in the *error* path, not the
query path, keeps the query path exact while making hallucinated identifiers
recoverable in one step. Errors should also distinguish *invalid request* from
*valid request, nothing found* from *valid request, not ready*.

### 4.9 Estimate before execute

A `count` or `estimate` form of every enumerating verb, and a dry-run that
reports the plan and expected size without materialising results. Token
budgets make this more valuable to an agent than cost-based `EXPLAIN` is to a
DBA.

### 4.10 Batched and compound requests

Several lookups in one round trip: a list of ids to get, two handles to expand,
three queries to search, all against one snapshot, returned in a uniform
envelope. Each avoided tool call saves latency, the per-call token overhead, and
a derailment opportunity. GraphQL's composition without GraphQL's language.

### 4.11 Named working sets

Let the agent name a result set and refine it server-side ("filter the previous
results to kind `task`", "expand the top five of set A") instead of carrying
intermediate ids in context and re-sending them. This is the temp table or the
shell pipeline, made persistent across tool calls and scoped to a snapshot.

### 4.12 Bounded, deterministic traversal

Graph expansion with explicit node, edge, and depth caps, all-or-nothing, with a
report of what the cap excluded. An agent cannot be trusted to pick a safe
depth; the interface must make the budget a first-class argument.

### 4.13 Change-oriented queries

"What changed since cursor X", "what crossed a validity boundary since T",
"what is different between snapshot A and B". For an agent maintaining memory
across sessions, delta queries are more common than full searches and are far
cheaper in tokens.

### 4.14 Aggregates and query-by-example

Counts and distributions per kind, source, or time bucket, so "how many" never
requires enumeration; and "more like handle X", so a follow-up does not require
the agent to re-express a hit as a text query.

### 4.15 Output designed for a reader that pays per token

Stable field names and ordering across verbs; homogeneous rows as compact
tables rather than repeated keys; no redundant echo of the request; explicit
truncation markers with the handle needed to fetch the remainder; and a small
set of verbs with rich typed options rather than many narrow verbs. The
tool-schema text and the result envelope are both part of the interface.

### 4.16 A constrained escape hatch

A small verb set covers most queries, but the long tail is real. A read-only,
time- and row-limited SQL (or equivalent) surface over documented views, paired
with the catalogue in §4.1, lets a capable model express the query it has in
mind without the system guessing. The constraint set is what makes it safe to
hand to an agent.

## 5. Where FathomDB 0.8.27 stands against these properties

Grounded in `dev/interfaces/python.md` and the Python SDK source; the Rust and
TypeScript peers mirror the same governed set. "Has" means the property is on
the public surface today; "partial" means an adjacent primitive exists; "gap"
means nothing on the surface serves it. Gaps are candidates for a future plan,
not commitments.

| § | Property | Status | Evidence / nearest primitive |
| --- | --- | --- | --- |
| 4.1 | Self-describing catalogue | gap | `read.projections`, `read.projection_status`, `read.embedding_readiness` describe configuration and readiness, not the data (kinds, counts, attribute values, edge kinds). `engine.open_report()` is open-time only. |
| 4.2 | Handles + snippet, token budget | partial | `SearchHit` carries typed `id`, `kind`, `source_id`; `limit` is rows only and `body` is always full. No snippet or budget argument. |
| 4.3 | Completeness / no-match signals | partial | `SoftFallback` names an arm that could not contribute; `explain=True` carries per-arm ranks and scores. No match count, saturation flag, or calibrated no-match verdict. |
| 4.4 | Engine-chosen plan + explain | partial | Hybrid RRF across text, vector, and optional graph arm with CE rerank is engine-routed; `Explanation` sidecar is positive-direction only. No negative explain. |
| 4.5 | Exactness alongside semantics | partial | `Filter` is a closed implicit-AND grammar (`SourceType`, `Kind`, `CreatedAfter`, `Status`, `Json`); `Json` is typed-rejected on `search` by design (ADR-0.8.11 D3). `search_text_only` gives the lexical arm alone. No exact-substring predicate composed with semantic search. |
| 4.6 | Stable snapshots across a plan | has | `freeze_read_context`, `search_frozen`, `search_expand_frozen`, `read.canonical_page`; frozen contexts are authenticated and carry a correlation identity. |
| 4.7 | Handles, provenance, validity | has | Typed `IdSpace` ids, `source_id` on every hit path, `ReadView` validity axis, supersession on `read.*`, `search_with_evidence` / `resolve_evidence`. |
| 4.8 | Typed errors + suggestions | partial | Typed error classes with structured attributes across all three SDKs; no did-you-mean on unknown kinds, projections, or ids. |
| 4.9 | Estimate before execute | gap | No count or dry-run form of `search`, `read.list`, or `graph.expand`. |
| 4.10 | Batched / compound requests | partial | `read.get_many` batches ids; nothing batches heterogeneous verbs in one call against one snapshot. |
| 4.11 | Named working sets | gap | Frozen contexts snapshot the store, not a result set. |
| 4.12 | Bounded deterministic traversal | has | `graph.expand(GraphExpandRequestV1)` is bounded, deterministic, all-or-nothing; `graph.neighbors` takes an explicit depth. |
| 4.13 | Change-oriented queries | has | `read.mutations`, `read.crossed_boundary_since`, `read.mutation_projection_status`, write cursors on receipts. |
| 4.14 | Aggregates, query-by-example | gap | No count or distribution verb; no more-like-this by id (the agent must re-issue text). |
| 4.15 | Token-aware output shape | partial | Stable dataclass shapes with the "new optional field with default" evolution rule; no truncation markers or compact tabular form; the surface is in-process SDK, not a tool schema. |
| 4.16 | Constrained escape hatch | gap by decision | `ADR-0.6.0-cli-scope` rejects application query verbs on the CLI; no read-only SQL surface. `read.list` with `Json` predicates is the closest general predicate path. |

Two observations on the mapping:

- The properties FathomDB already has (4.6, 4.7, 4.12, 4.13) are exactly the
  ones that are hardest to retrofit and that most retrieval systems lack.
  Snapshots, provenance, bounded traversal, and deltas are the substrate the
  remaining items would be built on.
- The largest remaining gaps (4.1 catalogue, 4.2 snippet/budget, 4.3
  completeness, 4.9 estimate) are all *read-side shaping* over data the engine
  already holds, which matches the finding in
  `dev/design/0.8.0-agent-memory-fit.md` that agent fit is mostly a read-surface
  problem. None requires a data-model change.

## 6. Agent-facing adapter, not just SDK verbs

The governed surface is an in-process SDK. The way an agent would actually
reach it is through a tool boundary (MCP or function calling) where the tool
schema text and result envelope are the whole interface the model sees. §4.15
and §4.16 therefore apply to a hypothetical adapter as much as to the SDK, and
an adapter is where §4.2 budgets, §4.10 compound requests, and §4.11 working
sets would most naturally live without changing the governed verb set. Whether
such an adapter belongs in this repository is a scope decision for a release
plan; this note only records that the interface properties above are the
requirements it would have to meet.

## 7. Summary

An agent does not need a new query *language*. It needs the properties that
`grep`, SQL, and web search each got partly right, assembled behind one typed
boundary: a catalogue it can afford to read, handles instead of payloads, a
token budget instead of a row count, explicit signals for complete / absent /
not-ready, exact constraints composed with semantic match, one snapshot per
plan, explain in both directions, and errors that name the fix. FathomDB holds
the hard substrate for this; what remains is read-side shaping.
