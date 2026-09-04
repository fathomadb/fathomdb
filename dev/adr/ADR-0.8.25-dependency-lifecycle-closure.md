---
title: Dependency-aware lifecycle closure
date: 2026-09-04
status: accepted
target_release: 0.8.25
---

# ADR: dependency-aware lifecycle closure

## Context

Slice 20 records one immutable direct canonical-source-to-derived dependency.
Before Slice 30, superseding, deleting, purging, or erasing the source could
leave its registered dependent visible or searchable. Slice 25 temporarily
refused such actuation with `dependency_closure_required`, but refusal alone
does not provide lifecycle closure to a client such as Memex.

## Decision

FathomDB atomically admits a content-free closure operation with every
source-losing root mutation that has direct registered dependents. Soft loss
makes those dependents ineligible and removes their projections; physical loss
also removes their canonical rows, dependencies, projections, pending work,
and receipt references. A nonterminal operation is an unconditional read and
projection-publication barrier under every `ReadView`.

Soft closure is proved and recovered internally in bounded batches. Physical
closure commits its structural zero proof with deletion and completes the
existing telemetry-redaction and WAL-checkpoint boundary through an exact
retry of the originating purge or erasure operation. Other writes remain
fenced while physical completion is pending.

The Engine exposes one additive, keyed, bounded status read:
`read_dependency_closure` / `readDependencyClosure`. The identifier is opaque
and Engine-minted; an absent valid identifier returns no information. There is
no public closure listing, resume, repair, or semantic-policy method.

This release closes direct single-source dependencies only. Recursive and
multi-source closure, and scheduled propagation at a validity boundary, remain
0.8.26 work.

## Consequences

- Lifecycle and erasure can no longer report successful closure while a direct
  registered dependent remains active or searchable.
- Closure state adds schema step 30 and a monotonic internal sequence.
- No-dependency writes preserve their existing response shapes and create no
  closure rows.
- `dependency_closure_required` remains a reserved compatibility spelling but
  is no longer emitted by the completed Slice 30 path.
- Exact wire shapes, failure precedence, proof fields, and recovery invariants
  are owned by the [Slice 30 design](../plans/0.8.25/features/slice-30/design.md).
