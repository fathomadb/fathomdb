---
title: FathomDB 0.8.26 Slice 8 — D26-01 graph-evidence impact spike
status: COMPLETE_READ_ONLY
observed_on: 2026-09-12
---

# D26-01 graph-evidence impact spike

## Question and method

This read-only scope spike answered whether an additive graph-evidence result
and point resolver can affect existing read, write, concurrency, erasure, and
performance contracts. It inspected the graph-expansion, evidence-resolution,
artifact-revision, erasure, reader-pool, and performance-gate implementations.
It made no code, schema, or contract change.

The evidence supports a dedicated implementation spike, named **Slice 15 —
graph-evidence performance and erasure linearization**, after Slice 10 and
before Slice 20. Slice 15 must have its own plan and design, design review,
human-authored RED tests, minimal GREEN prototype, code review, independent
verification, status record, and cleanup. Slice 20 may proceed only if Slice 15
accepts the design or records an approved narrower replacement.

## Findings

### Compatibility and ordinary-path impact

An opt-in successor graph result leaves existing `GraphExpandResultV1` bytes,
fixtures, and ordinary search paths unchanged. Adding required fields directly
to V1 would change a compatibility-sensitive serialized shape.

Existing AC-081a and AC-081b measure ordinary `Engine::search`; AC-081c proves
ordinary reader-pool independence. AC-072, AC-073, AC-075, and AC-076 likewise
exercise existing search behavior. A successor graph method and point resolver
do not invalidate those criteria unless their implementation changes shared
reader dispatch, caching, or ordinary search code. Slice 15 must nevertheless
run an identity-bound AC-081a/b observation and AC-081c as sentinels.

The graph path itself retains its existing bounds: result limit 50, work limit
10,000, and maximum depth 3. The current query-plan and RSS tests remain stop
gates.

### Bounded successor-result cost

The traversal already holds each target write cursor and each terminal-edge
write cursor. The lowest-risk seam is to retain the winning terminal-edge
cursor and hydrate immutable target and edge revision identities only after
the deterministic result set is truncated to 50.

The maximum additional result data is 100 caller identities: 50 target
revisions and 50 terminal-edge revisions. Caller identities are limited to 128
bytes, so raw identity payload is bounded by 12,800 bytes plus carrier and JSON
overhead. `_fathomdb_artifact_revisions` already has an indexed unique key on
artifact class and write cursor.

The target implementation is two bounded indexed hydration statements, one
for target cursors and one for terminal-edge cursors. Hydration must not be
joined into node or incident-edge loading, where work can grow to 10,000 rows.
That would change the new cost from `O(result_limit)` to `O(work_units)`.

### Sequential, concurrent, and write effects

The successor graph operation remains one WAL reader transaction. Two final
indexed hydration statements slightly extend its snapshot but do not acquire
the writer lock. Existing graph and ordinary search behavior remains unchanged
when the path is opt-in.

The initial point resolver should reuse the primary-connection serialization
of the existing evidence resolver. It then serializes with other point
resolutions, writes, and erasure, but does not occupy a reader-pool worker or
compete with ordinary search. Its work includes indexed provenance and
eligibility checks, source-body copying, and SHA-256, so cost scales with the
canonical source size. Fifty target resolutions are fifty sequential explicit
consumer calls; a batch API is out of scope until Memex-shaped evidence proves
it necessary.

Routing the first point resolver through the reader pool would increase the
concurrency and erasure blast radius. It would compete with search, extend WAL
snapshots, and require a new rule preventing erased bytes from overtaking a
successful erasure response. It is not the initial low-risk design.

### Erasure and secondary effects

An erased, expired, revoked, superseded, mismatched-context, ineligible, or
nonexistent revision must return one indistinguishable evidence-unavailable
refusal family. No persistent sidecar, evidence table, or cache may be added.

Primary-connection serialization prevents erasure from reporting success
while a resolver still owns the pre-erasure materialized result. A separately
held WAL reader may continue to cause the existing typed
`ErasureIncomplete`; the new path must never manufacture false erasure
success. Secondary effects are bounded response growth, revision-index cache
pages, and repeated source-body transfer when targets share a source. None is a
write-side persistence effect.

## Slice 15 required plan and design

Slice 15 is a bounded implementation and performance spike, not the product
implementation. It must preserve or discard prototype code explicitly at
close.

### Requirements and RED evidence

1. Prove existing V1 canonical bytes, permutations, work counts, errors, query
   plans, schema version, and RSS bounds are unchanged.
2. Prove at most 50 target and 50 terminal-edge revisions are hydrated after
   final selection, in two bounded indexed statements independent of graph
   work units.
3. Prove exact node and edge resolution and indistinguishable refusal for all
   unavailable and ineligible cases; do not synthesize rank or contribution.
4. Prove validation before and after materialization in one serialized
   transaction.
5. Pause before resolver return and prove erasure cannot report success first;
   every later resolution must be unavailable.
6. Prove an independent held WAL reader still produces typed
   `ErasureIncomplete`, never false success.

### GREEN prototype and measurements

- Compare V1 with the successor result for one target, 50 targets, and 10,000
  work units with 50 results. Use 1,000 measured release-mode iterations for
  the small cases and at least 30 for the 10,000-work case.
- Compare existing opaque-handle resolution with point resolution for node and
  edge artifacts backed by 1 KiB and 100 KiB sources. Run 1,000 sequential
  calls, eight callers by 200 calls, and one 50-resolution Memex-shaped run.
- Compare 1 KiB writer commits alone with commits concurrent with repeated
  point resolution.
- Record p50, p95, p99, throughput, SQL count, response bytes, bytes copied and
  hashed, reader-worker identity, mutex wait, RSS, WAL size, and erasure
  latency/outcome where applicable.

A repeatable writer-throughput loss above 10 percent or writer-p99 increase
above 25 percent is a design-review trigger, not a new product acceptance
criterion. Any existing hard-limit failure, V1 byte drift, unindexed scan, RSS
failure, or erasure overtaking is a stop. If AC-081 enters its warning band,
run its established multi-process campaign before attributing a regression.

## Recommendation to HITL

Approve D26-01 option A only with Slice 15 inserted before Slice 20. The
approved initial shape should be a successor graph-evidence result with
post-selection indexed hydration and a primary-connection point resolver.
Reader-pool and batch resolution remain later options requiring new evidence
and erasure-linearization design.
