---
title: 0.8.25 Slice 30 — core lifecycle and erasure closure design
status: DRAFT_SCOPE_RECONCILED_BLOCKED_ON_SLICE_7
design_version: 2
depends_on: 25
architecture: dev/design/fathomdb-data-plane-architecture-v2.md
---

# Slice 30 design

## Authority and boundary

Implements R25/AC25-30, N25-01/N25-02, Memex needs 5/6, and A25-05 for the
Slice 20 canonical-source-to-derived relation. Existing lifecycle, erasure,
and projection-registry contracts remain authoritative substrates. This design
adds direct-dependent closure without introducing semantic policy.

Recursive derived DAG closure, source-set liveness, prepared-request journal
recovery, and source-separable multi-source behavior are allocated after
0.8.25.

## Contract

```text
ClosurePhase = planned | barriered | propagating |
               projections_pending | proving | complete | incomplete
ClosureStatusV1 {
  schema_version: 1, operation_id, root_source_revision_id, root_action,
  admitted_boundary, phase, processed_count, pending_count,
  affected_revision_ids, projection_work_ids, blockers, proof?
}
ClosureProofV1 {
  schema_version: 1, operation_id, proof_write_boundary,
  active_dependent_count: 0,
  searchable_dependent_count: 0,
  projection_orphan_count: 0,
  post_admission_dependency_count: 0,
  checks
}
```

Supersede, invalidate, delete, and erase of a canonical source create a closure
intent and source barrier in the same writer transaction as the root lifecycle
change. The model-free transition matrix is fixed:

| Source action | Direct dependent action |
| --- | --- |
| supersede | invalidate; no replacement is inferred |
| invalidate | invalidate |
| delete | delete |
| erase | erase source-derived bytes and indexes |

Erasure retains only approved non-content tombstones. A derived artifact can
return only through a new caller-authored revision and dependency.

## Fencing and propagation

From barrier admission through successful proof, every governed list, search,
FTS, vector, graph, projection, and evidence path applies a direct dependency
guard before candidate/seed/frontier truncation. A derived revision whose
registered Slice 20 dependency names a barriered source is ineligible even before
its work row is processed. Missing/corrupt dependency indexes or inability to
evaluate the guard fails the read `closure_visibility_unavailable`.

The same barrier rejects new derived writes or dependencies against the source.
Propagation uses durable work rows keyed by `(closure_operation_id,
derived_revision_id, action)` and bounded transactions. It resumes
idempotently after restart. Because Slice 20 has no derived-to-derived edges,
this is a bounded direct lookup, not recursive ancestry traversal.

If more dependents exist than one transaction processes, the barrier and guard
remain. Resource exhaustion records `incomplete` and never lifts visibility
fencing. A concurrent dependency registration either commits before admission
and appears in the fixed work/proof set or observes the barrier and fails.

## Completion and proof

After dependent lifecycle work, drain relevant projections and inspect
canonical/dependency, FTS, vector, graph, evidence, and WAL state. Completion
requires zero active/searchable/projection dependents plus no post-admission
dependency. The zero proof and barrier retirement commit atomically at a later
write boundary. An empty queue alone is not completion.

Any closure transaction that deletes one or more Slice 20 registrations also
advances the separate dependency generation exactly once in that transaction.
Closure that changes no dependency leaves it unchanged.

Current lifecycle and erasure APIs remain compatible. Reactivation is the
existing transition to `active`, requires a currently valid source dependency,
and cannot restore erased bytes. All public/persisted types follow Slice 15
wire/SDK rules.

Failures include closure conflict, barrier conflict, visibility unavailable,
inactive reactivation, projection/proof incomplete, resource exhausted, and
erasure incomplete.

## RED/GREEN and verification

RED tests cover every root action, a dependent below the first work page,
read/write races, missing reverse index, injected projection orphan, restart at
each phase, exhausted resources, false empty-queue completion, and raw-byte/WAL
erasure canaries. GREEN proves immediate fail-closed visibility, idempotent
resume, no post-barrier registration, exact affected-state transitions, zero
proof, atomic barrier retirement, and clean replacement by a new revision.

Run fast, heavy, all/all-feature/operator, Windows Rust/Python/Node, and locally
packed artifact routes. CUDA and live-model are N/A. A formal independent READY
review remains required after Slice 7 and Slice 25 complete.
