---
title: 0.8.25 Slice 20 — dependency registration and liveness design
status: REVIEWED_BLOCKED_ON_SLICE_7
design_version: 1
review_fix: 2
depends_on: 15
---

# Slice 20 design

## Authority and disposition

Implements R25/AC25-20, N25-01, Memex needs 4/14, and A25-04/A25-05.
Projection linkage is precedent, not the generic store; the lifecycle protocol
is historical evidence. This is the new architecture-v2 dependency authority.
FathomDB enforces structure, never semantic truth. READY remains blocked.

## Public/wire contract

```text
ArtifactRefV1 =
  LogicalCurrent { id_space, logical_id }
  | PinnedRevision { artifact_revision_id }
DependencySetV1 {
  schema_version: 1, set_id, dependency_set_revision_id, dependent,
  members[1..256], liveness: all_required | any_surviving,
  lifecycle: active | retired, metadata
}
DependencyLivenessViewV1 {
  schema_version: 1, effective_valid_at, canonical_write_boundary,
  lifecycle_mode: strict_current
}
```

`LogicalCurrent` follows future active revisions; `PinnedRevision` never
retargets. Set ID is stable and set revision immutable. Replacement appends an
active revision and retires the old atomically; delete retires without
destroying history. Historical reads name a set revision. A dependent may own
multiple active sets: all sets must be live; each applies its closed rule.

## Structural liveness and validity boundaries

Dependency liveness ignores caller eligibility/access and historical view
relaxations. A closure operation fixes one strict-current effective instant.
A logical-current member survives when its current revision is active/valid at
that instant; a pinned member survives only when that revision is active/valid.
`all_required` needs every member and `any_surviving` at least one.

Registration or activation of an **active** dependent requires every set to be
live at the transaction's fixed instant; otherwise it rejects
`dependency_not_live`. A caller may instead create/transition the dependent
inactive or pending. A future member `valid_from` never schedules closure and
never auto-activates a dependent. The caller may later request the existing
transition to active, which re-evaluates strict liveness at that later instant.

Only a boundary that can change an active set from live to non-live is queued.
For validity alone this is an applicable member `valid_until`; immediate write,
supersession, invalidation, and erasure losses create closure intent in their
writer transaction. For `all_required`, every finite member `valid_until` is a
loss boundary. For `any_surviving`, the queued loss boundary is recomputed after
each mutation as the latest finite `valid_until` among surviving members, or no
boundary when a member has no finite end. Future `valid_from` is excluded.

The writer scheduler admits Slice 30 closure when a queued loss becomes due.
Every governed retrieval checks due-loss rows before candidates; until the due
closure is barriered/completed it fails `dependency_closure_due` rather than
show stale derived state. Writes recompute the queue atomically.

## Persistence, cycles, and failures

Persist stable sets, immutable revisions, members, current pointer, lifecycle,
and next loss boundary, indexed both directions. Validate prospective active
set revisions in the writer transaction. Logical-current references resolve
against the prospective current map; pinned references remain fixed. Reject
self/reachable cycles, then append/retire and queue atomically.

Queries order set/revision/member. Until Slice 45, return at most 100 sets and
refuse overflow. Caps: 256 members/set, 1,024 sets/mutation, 10,000 reachable
nodes. Failures include missing/ambiguous reference, cycle, unknown rule,
set-revision conflict, dependency not live, closure due, and bounds.

## Tests and verification

Tests cover both reference modes, supersession, immutable set history,
replacement/delete, cycles, both rules/multiple sets, caller-view independence,
active registration refusal, inactive/pending creation, `valid_from` without
closure/reactivation, `valid_until` loss for each rule, queue recomputation,
scheduler/read refusal, restart, rollback, limits, reciprocity, codecs and
three-SDK/Windows/installed parity. Run fast, heavy, all/all-feature, Windows
and registry; CUDA/model N/A.
