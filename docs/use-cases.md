# FathomDB Use Cases

FathomDB fits applications that need compact discovery followed by exact,
revision-aware evidence verification. It keeps retrieval, records and explicit
relationships in an embedded database, close to the application using them.
The host decides what to store, who may access it and which actions to take.
These examples reflect FathomDB **0.8.32** and, where identified, the
`fathomdb-mcp` server. Each keeps its source of truth explicit.

## Single-scope default and multi-agent isolation

**By default a FathomDB database is single-scope / single-agent** (one agent per
database, `scope = ""`, using the default `ScopePolicy.unscoped()` /
`ScopePolicy::Unscoped`). Most single-agent applications can open a database and
write or search without specifying any scope. When multiple agents or tenants
share one Schema **39** database file, **opt into** engine-enforced caller
isolation with `ScopePolicy` (`single`, `read_only` / `readOnly`, or
`read_write` / `readWrite`) at open time so read queries, frozen contexts, and
writes are confined to authorized scopes (`ScopeViolationError` on unauthorized
writes; cross-scope nondisclosure on lookups).

## Coding agents: searchable engineering memory

Keep architecture decisions, incident notes and test evidence searchable across
sessions. Full-text search can find an exact error or decision ID; hybrid
retrieval can help find a passage described in different words. Declare
filterable attributes for repository, revision and path, then apply them to the
relevant query. Those attributes come from your application; filters alone do
not establish tenant access control.

For an implementation decision, resolve selected source spans through the SDK's
[frozen evidence flow](guides/frozen-evidence.md), using versioned provenance
written at ingestion. The agent can cite the stored revision and passage.
Git, the live checkout and language tools establish what is currently implemented;
the stored note provides supporting evidence for that inspection.

## Computer agents: runbooks and workflow checkpoints

Store addressable runbook sections or individual procedure steps so an agent
can discover a small relevant excerpt before fetching a selected record. The
`fathomdb-mcp` server (production read+write in `0.8.32`, read-only by default)
provides budgeted search snippets,
logical-ref fetch, and integrated bounded frozen evidence (`search` with
`freeze: true`, `expand` with `include_evidence_refs`, and `fetch` with
`evidence_refs` / `graph_evidence_refs`); inspect truncation and adjust the
budget when the needed text was cut.

Maintain a registered `latest_state` checkpoint collection through the SDK or,
on a store explicitly opted into writes with a bound caller `scope`
(`allow_writes: true` + `scope`), via MCP `put_latest_state` or `actuate` with
idempotent `operation_id` replay and CAS `expected_write_boundary`
preconditions. Check the live UI before a consequential retry. Database
idempotency covers database changes; confirming and safely retrying an external
UI action remains the host's responsibility. Coordinate concurrent host SDK
and MCP access by embedding `fathomdb_mcp::Adapter` or respecting
[one-Engine-per-file ownership](operations/index.md#concurrency-and-durability-limits).

## Business processes: case records with applicable evidence

Connect a case to the policy, requirement or procedure revision that applies
to it. Validity and eligibility controls help select the intended view;
versioned provenance and exact evidence let a reviewer inspect the supporting
passage. For example, a case note can retain an attributed link to the procedure
used when the decision was made, while a later policy revision remains distinct.

The SDK's [atomic actuation](reference/python-api.md#dependencies-actuation-and-closure)
can apply related database changes with idempotent replay, such as a case-record
update, a derived note and its registered source dependency. Its transaction
boundary covers the declared database operations. External approvals, messages
and other side effects require host coordination.

For a design controls pilot, keep requirements, verification records and
approvals in their existing systems of record. Use FathomDB to retrieve the
supporting revision and passage, with a route back to the authoritative record.
The existing approval workflow still establishes the decision.

## Knowledge management: traceable decisions and claims

Stable IDs, supersession and versioned provenance help explain how a recorded
claim changed and which source supported each version. Add explicit,
caller-written relationships between decisions, claims and sources, then use
[bounded graph expansion](guides/graph-retrieval.md) to inspect relevant neighbors.
Traceability questions are a particularly direct fit for this combination.

When exact graph evidence is required, use the SDK's frozen, opt-in graph
evidence flow to resolve the returned target and winning terminal edge. Read
work bounds and degradation alongside the result. That proof covers those
artifacts; it does not authenticate every edge in an entire path or perform
entity resolution. Your application authors and interprets the relationships.

## Personal agents: embedded, correctable memory

Keep preferences and observations in an application's local database. Hybrid
search can help recover a preference when a later request paraphrases it.
Stable IDs and supersession let the host record a correction while retaining
the distinction between earlier and current versions.

The host decides what deserves persistence, separates user statements from
agent inference, and controls which context each caller may read. For removal,
use the documented [source erasure and maintenance procedures](operations/erasure.md)
for governed records and their derived indexes. Treat completion and incomplete
outcomes according to that contract. Exports, backups and copies held elsewhere
need their own handling; database erasure is not an instant physical wipe of
every storage remnant.

## Build the evidence flow

Start with a narrow collection, a clear record identity and explicit source
attribution. For exact citations, author versioned provenance and source
locators, freeze the intended read context, search with evidence and resolve
only selected references from that attempt. Ordinary search or logical fetch
can select a different eligible revision; a source ID alone does not prove
source bytes. If frozen state drifts, restart the evidence sequence.

Use the [application SDK references](reference/index.md) or the
[MCP server guide](guides/mcp-server.md) for governed reads, frozen evidence,
and opt-in scoped actuation. Configure embeddings and supported dense kinds for
hybrid retrieval, and check the [extractor and provenance limits](guides/extractor-provenance.md)
when importing material. Measure relevance on your own records while keeping
host access policy and live external state explicit.
