---
title: FathomDB 0.8.28 Slice 70 — eligibility design
status: draft for Astra review
target_release: 0.8.28
---

# Native vector eligibility

Extend the shared format-35 vec0 builder with three plain INTEGER metadata
columns: `lifecycle`, `valid_from`, `valid_until`. Lifecycle encodes canonical
superseded (bit 2) and inactive (bit 1) flags; zero means current and active. Edges have no inactive
state. Bounds are inclusive: absent lower/upper use i64 MIN/MAX; finite
exclusive upper becomes end minus one. An end equal to MIN encodes an empty
interval using lower MAX and upper MIN. Edge lower is always MIN and upper
comes from `t_invalid`, matching existing edge reads.

A shared helper reads canonical state by record write cursor and updates
`vector_default` for all IDs selected through the indexed
`_fathomdb_vector_rows.write_cursor` mapping. Use it after canonical mutation
within its existing transaction in write commit, transition, actuation and
consolidation. Dependency closures erase their projections; verify that this
removal is record-keyed. No triggers or extra serving-authority tables are
introduced. Projection writes derive metadata inside their commit transaction,
so in-flight embedding cannot publish stale lifecycle state. Test-only vector
insertion without canonical ownership uses active, unbounded metadata.

Attribute reshapes stage/copy all three values. Recenter deletion/reinsertion
preserves them byte-for-byte alongside f32, spans and attribute values. Keep
the low-level single-vector deletion primitive for recentering; record erasure
collects all vector IDs before removing their mappings. This prepares upkeep
for future windows without enabling overflow production or historical reads.
Attribute refresh/backfill resolves record identity through the same mapping;
open orphan detection compares vector IDs against mapping rowids.

Compile `lifecycle=0`, `valid_from<=effective_at` and
`valid_until>=effective_at` into the inner vec0 KNN before ORDER/LIMIT.
Out-of-window view handling omits temporal predicates where already admitted;
public inactive/superseded flags remain refused until Slice 80. Internally,
compile admitted flag combinations as lifecycle equality or IN over the
allowed codes; sqlite-vec supports INTEGER metadata IN. Both flags admit
all codes and omit that predicate. Bind time
from the frozen reader view, never from another clock. Append eligibility
bindings after existing filter bindings to preserve their indexes. The public
test SQL seam must expose the actual production builder. Keep canonical
hydration guards as defense in depth, but remove stale comments claiming
post-cap filtering is the eligibility mechanism.

Remove the direct canonical/vector-row scan in
`vector_arm_requires_fallback`; retain its source-dependency/barrier query
and existing fallback reason. The plain no-dependency path performs no
store-wide vector eligibility scan. Canonical data remains the authority;
vec0 metadata is a transactional projection, not a second lifecycle identity.

Tests use real SQLite and deterministic embedders: shape and capacity,
mutation/rollback, all IDs per record, rebuild/reshape/recenter, a RED per
superseded/inactive/future/expired cause with more than 192 distractors,
reference selection/property comparisons, frozen views and retained source
barriers. Update the format ADR and interface contract for metadata/capacity;
no new public API is required. Keep test fixture updates separate from any
change to behavioral expectations.

## Design review refinements

Pin the internal flag matrix: default `lifecycle=0`, include-inactive
`lifecycle IN (0,1)`, include-superseded `lifecycle IN (0,2)`, both omit
lifecycle restriction. Exercise all four through the production builder.
Add RED fixtures with differing vector IDs/record cursors, multiple IDs per
record and a true orphan for attribute refresh, reshape backfill, erasure
and orphan detection. A pre-extension format-35 partition lacking the three
columns must fail open before serving or reshaping without partial mutation;
test the physical admission guard rather than relying on version alone.
