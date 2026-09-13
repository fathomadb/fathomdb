---
title: FathomDB 0.8.26 Slice 15 — graph-evidence impact design
status: DRAFT — REVIEW REQUIRED
---

# Slice 15 design — graph-evidence impact spike

## Alternatives under test

### A. Optional V1 result sidecar — provisional recommendation

Change the existing `GraphExpandRequestV1` in place with an opt-in evidence-ref
request and change `GraphExpandResultV1` in place with an optional evidence
sidecar keyed deterministically to returned targets and terminal edges.
`GraphTargetV1` remains unchanged. When the option is absent, the field is
omitted and the ordinary response remains byte-for-byte unchanged.

This is a sidecar in response structure only. It is not a second graph method,
API generation, router, persistence table, cache, or background service.

### B. Required target fields

Change `GraphTargetV1` in place to carry the needed immutable revision
identities on every graph call. This is allowed by the breaking-release rule,
but makes response growth and hydration part of the ordinary path and changes
every target fixture and binding shape.

Both alternatives retain V1 names. A V2 result, graph method, parser, or router
is prohibited by `seq-283`.

## Shared prototype architecture

Traversal keeps each selected target and terminal-edge write cursor. Only
after deterministic result truncation does the prototype issue two bounded,
indexed hydration statements against `_fathomdb_artifact_revisions`: one for
target cursors and one for terminal-edge cursors. Hydration must remain
`O(result_limit)`, never `O(work_units)`.

The point resolver is a first-generation V1 operation returning intrinsic
`ResolvedArtifactEvidenceV1`. On the primary connection, one transaction
validates frozen authority, artifact identity, lifecycle, eligibility, and
nondisclosure before and after materializing canonical bytes and hash. It does
not invent rank or contribution and does not fall back to logical-ID or body
search.

## Concurrency and erasure

Primary-connection serialization is the initial safety boundary. Resolution
serializes with writes and erasure so erasure cannot report success while the
resolver owns materialized pre-erasure bytes. Existing held WAL readers may
still cause typed `ErasureIncomplete`; the prototype must not weaken that
contract. Reader-pool and batch-resolution designs are excluded.

## Decision evidence

Prefer A only if the absent-option path remains byte-identical, indexed
hydration is bounded, point resolution preserves erasure linearization, and
write/read measurements remain within existing hard limits without material
ordinary-path regression. Prefer B only if A adds greater measurable cost or
complexity and required target fields do not create a worse consumer and
fixture blast radius. If neither is safe within this boundary, recommend
deferral rather than broadening Slice 20.
