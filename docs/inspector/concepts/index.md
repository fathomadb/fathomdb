# Inspector concepts

These concepts describe the data and observations in the implemented Inspector
0.2.0 candidate. Installed technical checks cover the selected pairing;
the unperformed human study is removed by owner decision.

## Records and representations

Canonical nodes and edges are records. A record's kind classifies it; structural
role is another axis. Edges have endpoints and their own identity and content.
A derived canonical record is still a distinct record, linked to a source by
recorded provenance where available.

Text indexes and vector materializations are retrieval representations of
content. Their declarations, stored materializations, and readiness are
separate facts. A graph relationship is not automatically a projection
created for every record; graph participation follows actual recorded edges.

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
lifecycle and temporal rules. Absence from search alone does not establish
which condition prevented selection.

Vector search can use a binary shortlist followed by float-distance reranking.
An exact distance within that selected set is not proof that the set contains
all globally nearest records. Lexical requests may use different indexed
fields and BM25/BM25F scoring routes. Views must identify the actual method,
not compare unlike scores on an unexplained common scale.
