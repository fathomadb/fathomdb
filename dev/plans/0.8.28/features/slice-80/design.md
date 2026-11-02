---
title: FathomDB 0.8.28 Slice 80 — search view design
status: approved with Astra refinements
target_release: 0.8.28
---

# Search views and historical projections

Use the existing ReadView contract: supersession and inactive state are
independent relaxations, while validity, content filters and source-dependency
eligibility continue to apply. Remove search admission rejection. Generate
canonical predicates from the view at all search retrieval sites, including
FTS ranking/streaming, vector hydration, property lookup and graph expansion.
Preserve intentional ownerless-FTS behavior and deterministic version selection
where an operation resolves a logical ID. Search history may return distinct
record versions; do not collapse them merely because their logical IDs match.
Native vec0 lifecycle IN predicates already implement the four combinations.

Retain edge vectors on both supersession keys and update their mapped rows'
lifecycle metadata inside the canonical write transaction. Do not fabricate
an `up_to_date` terminal for a superseded body-bearing edge. Extend existing
edge projection membership to include superseded versions consistently in
pending scans, commit-time membership, readiness aggregates/fast paths and
rebuild replay. Nodes already retain historical membership. Preserve existing
edge expiry and dependency eligibility rules; bodyless edges do not owe vectors.
In-flight commits derive current lifecycle metadata from canonical state.

At boot, reconcile historical edge rows whose earlier supersession created a
skip terminal with no physical vector tuple. Clear only false successful
terminals, rewind below the lowest pending historical cursor, and let the
ordinary bounded worker project them. Do not treat partial sidecar/vector/span
state as a legitimate skip, or clear genuine failed terminals. Rebuild uses
the same historical membership and reconstructs searchable historical FTS.
This repairs derived projections, with no canonical migration or new tables.

Search must not infer complete historical vectors from a stale terminal or
watermark. Use existing physical completion and soft-fallback reporting; when
a requested historical/temporal view admits records outside the scheduler's
current membership, conservatively fall back to text rather than silently
present a partial dense history as complete. Source-dependency restrictions
remain fail-closed. Default search retains its current native fast path.

Tests use deterministic embedders and real stores. Exercise each flag alone
and together across text and dense search, old/current bodies, node inactive
states, filter/validity combinations, graph/property paths and frozen/paged
entry points. Exercise both edge supersession keys before drain, after drain
and with a paused in-flight embed; verify retained spans and metadata, reopen
of old skipped terminals, rebuild, failures and unavailable runtime fallback.
Retain stale-body defaults and bounded scheduler/physical readiness checks.
Update Rust/Python/TypeScript interface and binding documentation together.

## Astra review refinements

Relaxed hybrid searches check completeness independently of query terms in the
same reader snapshot: every view-visible, vector-committable record must have
its successful terminal and matching physical tuple. Pending, failed or missing
history disables the whole vector arm and reports the existing vector soft
fallback, including when the runtime is absent or no lexical hit exists.
Direct text search does not report an unused vector fallback. Keep this probe
off the default hot path. Status/explain must not mistake legacy intentional
skips for complete history or turn a documented fallback into an unrelated
corruption failure. Reconciliation runs for edge-only stores too, independently
of a node vector declaration. Partial physical tuples remain corruption.

Attribute projections intentionally contain current active rows only. Preserve
that invariant. For relaxed attribute filters derive values from canonical body
using the same validated path and scalar conversion as `extract_scalar_attribute`;
apply predicates before candidate caps. For relaxed property FTS, populate a
connection-local temporary FTS5 table from view-eligible canonical scalar
values, using the existing property tokenizer. Rank the whole eligible corpus
before limits, never substrings or independently ranked rows. This is opt-in
query work, not a new persistent index. Strict queries retain existing indexes.

Graph traversal continues to operate on logical entities. Hydration resolves
the newest eligible revision deterministically by descending write cursor,
matching point-read semantics; body/dense search may independently retrieve
older versions. Avoid duplicate historical endpoint joins consuming graph
fanout limits. Preserve existing body-based fusion/deduplication: historical
search is ranked retrieval, not a complete enumeration of every revision.

Vector attribute reshapes and late declarations derive values for every mapped
historical/inactive vector from its canonical body and registry source, rather
than the current-only EAV cache. Preserve vector IDs and spans; test late
attribute declaration and source changes across reopen.

Rebuild replays FTS for every body-bearing canonical edge, including expired
history. Dense scheduling/completion separately retain their expiry predicate.
A past valid-as-of historical view must retain lexical fallback after rebuild.
