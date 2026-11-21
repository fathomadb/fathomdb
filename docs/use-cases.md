# FathomDB Use Cases

FathomDB fits applications that need compact discovery followed by exact,
revision-aware evidence verification. It keeps retrieval, records and explicit
relationships in an embedded database, close to the application using them.
The host decides what to store, who may access it and which actions to take.
These examples use the published 0.8.30 SDKs and, where identified, the
repository MCP adapter. Each keeps its source of truth explicit.

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
repository MCP adapter remains an evaluation build available from source. Its
retrieval tools provide budgeted search snippets and logical-ref fetch;
inspect truncation and adjust the budget when the needed text was cut.
[Logical fetch](guides/structured-search-hits.md#stored-versions-and-public-ids)
selects the latest eligible revision for its own view. Exact source-revision or
span verification belongs to the SDK evidence APIs.

Have the host maintain a registered `latest_state` checkpoint collection through
the SDK, recording a confirmed step and the evidence of its completion. Check
the live UI before a consequential retry. Database idempotency covers database
changes; confirming and safely retrying an external UI action remains the host's
responsibility. MCP feedback may persist agent labels, but the adapter exposes
no checkpoint-write or actuation tools. Coordinate
SDK writes and adapter reads under the documented
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

Use the [application SDK references](reference/index.md) for advanced provenance,
actuation and exact evidence. MCP supplies a compact read surface for discovery and logical fetch. Configure embeddings and supported dense kinds for
hybrid retrieval, and check the [extractor and provenance limits](guides/extractor-provenance.md)
when importing material. Measure relevance on your own records while keeping
host access policy and live external state explicit.
