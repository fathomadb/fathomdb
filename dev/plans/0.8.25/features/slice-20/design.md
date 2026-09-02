---
title: 0.8.25 Slice 20 — core dependency registration design
status: DRAFT_SCOPE_RECONCILED_BLOCKED_ON_SLICE_7
design_version: 2
depends_on: 15
architecture: dev/design/fathomdb-data-plane-architecture-v2.md
---

# Slice 20 design

## Authority and boundary

Implements the retained core of R25/AC25-20, N25-01, Memex need 4, and
A25-05. It adds one provenance-safe dependency relation from an immutable
canonical source revision to an immutable caller-derived artifact revision.
FathomDB validates and indexes the relation; it does not infer it or assign
semantic truth.

Multi-source sets, derived-to-derived dependencies, logical-current
retargeting, and configurable liveness are allocated to
[`0.8.x-after-0.8.25-design-notes.md`](../../../../design/0.8.x-after-0.8.25-design-notes.md).
They are not dormant variants in this contract.

## Contract

```text
DependencyId(string)
SourceDependencyV1 {
  schema_version: 1,
  dependency_id,
  source_revision_id: SourceRevisionId,
  derived_revision_id: ArtifactRevisionId,
  state: active | retired,
  created_write_boundary,
  retired_write_boundary?
}
DependencyPageV1 { schema_version: 1, items, truncated: false }
```

The source must be a complete Slice 15 canonical source revision. The
dependent must be a distinct caller-derived revision. Both references are
pinned and never retarget when either logical record receives a new revision.
A derived revision has at most one active source dependency in 0.8.25. The
stored representation permits that bound to be relaxed additively later.

Public operations are `register_source_dependency`,
`retire_source_dependency`, `dependencies_for_source`, and
`source_for_derived`. Registration is idempotent for an identical dependency
ID and endpoints. Reuse with different endpoints fails. Retirement preserves
the non-content identity row for audit and idempotency.

## Persistence and validation

Persist one normalized relation table with unique indexes on dependency ID,
active derived revision, and `(source_revision_id, derived_revision_id)`, plus
an index ordered by `(source_revision_id, derived_revision_id, dependency_id)`.
Registration validates the prospective transaction before writing:

1. both immutable revisions exist and have the required roles;
2. the source provenance is complete and currently valid for registration;
3. the endpoints differ and no active dependency already owns the derived
   revision;
4. the derived revision does not identify a canonical source; and
5. IDs and versions satisfy Slice 15 wire rules.

The role restriction makes a cycle structurally impossible. Self-reference,
derived-as-source, and canonical-as-dependent requests return typed
`dependency_cycle_or_role_invalid`; the Engine must not accept a generic edge
and rely on a later closure pass to discover the error.

Until Slice 45, source lookup returns at most 100 ordered entries. More than
100 returns `dependency_lookup_bound_exceeded` with no partial result.
`source_for_derived` returns zero or one relation. No client-side scan or
shadow reverse index is part of the contract.

## Lifecycle and compatibility

Slice 30 consumes the reverse index for lifecycle and erasure closure. A
retired relation cannot make a dependent eligible; retirement does not by
itself restore or activate either artifact. Source supersession never
silently retargets a dependency.

The feature is additive. Existing records remain readable. Legacy derived
records without registered dependencies remain explicitly unlinked and cannot
claim dependency-complete evidence. All public and persisted objects follow
Slice 15 version, unknown-field/variant, typed-error, Rust/Python/TypeScript,
Windows CPU/native, and locally packaged parity rules.

Failures are `dependency_reference_missing`,
`dependency_provenance_incomplete`, `dependency_cycle_or_role_invalid`,
`dependency_conflict`, `dependency_not_active`, and
`dependency_lookup_bound_exceeded`.

## RED/GREEN and verification

RED fixtures cover missing/wrong-role/self references, a second active source,
ID replay with changed endpoints, overflow, and cross-SDK codec disagreement.
GREEN properties cover stable forward/reverse ordering, exact reciprocity,
restart/reindex preservation, atomic rollback, retirement/replay, source
supersession without retargeting, and legacy-unlinked behavior. Real-database
lifecycle tests prove Slice 30 can discover every direct dependent without an
application shadow index.

Run fast, heavy, all/all-feature, Windows Rust/Python/Node, and locally packed
artifact routes. CUDA, model, operator, and pre-publication registry routes are
N/A. A formal independent READY review remains required after Slice 7 and
Slice 15 complete.
