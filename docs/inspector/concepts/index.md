# Inspector concepts

These concepts describe the data and observations in Inspector 0.3.0
(targeting FathomDB 0.8.31 / schema 38). Installed technical checks cover the
selected `D4 = B` Ubuntu 24.04 pairing; the unperformed human study remains
removed by owner decision.

## Records and representations

Canonical nodes and edges are records. A record's kind classifies it; structural
role is another axis. Edges have endpoints and their own identity and content.
In Schema 38, canonical nodes and edges can store structured JSON `properties`
outside `body` (alongside plain-text or JSON `body` documents), and canonical
and operational tables carry a reserved `scope` column (empty in `0.8.31`,
displayed read-only without synthesizing multi-tenant rules).
A derived canonical record is still a distinct record, linked to a source by
recorded provenance where available.

Text indexes, filterable property materializations (`canonical_property_values`
for both nodes and edges), and vector materializations are retrieval
representations of content. Their declarations (`_fathomdb_projection_registry`
carrying `entity_class` and `vector_kinds`), stored materializations, and
readiness are separate facts. A graph relationship is not automatically a
projection created for every record; graph participation follows actual
recorded edges.

Incomplete provenance is a valid state, including for some ordinary writes.
The Inspector must show the available role/completeness and identity rather
than turn an unknown link into a proven source relationship. Engine-owned
bookkeeping is labeled separately from caller data.

## Identity and observations

Equal text does not establish equal identity. Different revisions can share
one logical identity while containing different content. Search identities
can occupy logical, content, or passage namespaces; a content identity alone
need not resolve to one stored physical row.

A result is an observation of a request, not another independently authored
copy of the record. Its payload, score family, origin evidence, limits and
read context remain associated with it. A later record lookup must not silently
replace an earlier observed revision. In-session comparison retains each of two
observations' original request, population and context. A graph set can have
membership without a rank; unlike metric families do not share a scale.

## Operational collections

An append-only-log collection retains entries, including repeated keys.
A latest-state collection retains current values and does not automatically
retain overwritten payloads as mutation history. Equal keys in different
collections do not imply a shared history.

## Retrieval scope

Stored, eligible, representation-ready, runtime-available, and returned in one
bounded result set are different facts. Nodes and edges have class-specific
lifecycle and temporal rules. Historical `as_of` graph snapshots
(`as_of_write_cursor` or `as_of_epoch_ms`, mutually exclusive and resolved via
`_fathomdb_commit_timestamps`) observe the recorded graph state at a specific
commit point while remaining strictly subordinate to the active `D-01`
disclosure flags; historical queries never resurrect physically erased content
or bypass `--no-show-inactive` / `--show-erased`. Absence from search alone
does not establish which condition prevented selection.

Vector search can use a binary shortlist followed by float-distance reranking.
An exact distance within that selected set is not proof that the set contains
all globally nearest records. Lexical requests may use different indexed
fields and BM25/BM25F scoring routes. Views must identify the actual method,
not compare unlike scores on an unexplained common scale.
