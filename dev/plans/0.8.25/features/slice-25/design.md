---
title: 0.8.25 Slice 25 — bounded atomic actuation design
status: DRAFT_SCOPE_RECONCILED_BLOCKED_ON_SLICE_7
design_version: 2
depends_on: 20
architecture: dev/design/fathomdb-data-plane-architecture-v2.md
---

# Slice 25 design

## Authority and boundary

Implements the retained core of R25/AC25-25, N25-01, Memex needs 3/17 and the
compact portion of 18, plus A25-05. It is a model-free mechanism for applying
decisions already made by a caller. Typed-write, `PreparedWrite`, and the
single SQLite writer transaction remain the implementation substrate.

Facts, edges, merge/coexist verdict variants, full consequence journals,
public crash-journal administration, and omnibus batch sizes are outside this
slice and preserved in the post-0.8.25 design notes.

## Contract

```text
ActuationOperationV1 =
  PutCanonical { record }
  | PutDerived { record }
  | RegisterSourceDependency { dependency }
  | TransitionLifecycle { target_revision_id, transition }

ActuationBatchV1 {
  schema_version: 1,
  operation_id,
  decision_policy_id?,
  expected_write_boundary?,
  operations: [1..128]
}

ActuationReceiptV1 {
  schema_version: 1,
  operation_id,
  request_sha256,
  outcome: committed | committed_closure_pending | refused,
  reason_codes,
  affected_revision_ids,
  resulting_write_boundary?,
  resulting_dependency_generation?,
  projection_work_ids,
  closure_operation_ids
}
```

The lifecycle transition union is the existing governed active/superseded/
invalidated/deleted transition contract. Reactivation uses the existing
transition to `active`; erased bytes are never restored. A caller expresses a
supersession by creating the replacement revision and transitioning the old
revision in the same batch. FathomDB validates the decision but does not decide
whether the claims conflict.

## Atomicity and idempotency

The Engine canonicalizes and hashes the closed request, validates all
operations against one prospective state, then applies domain rows, synchronous
projection work, the compact operation record, and receipt in one SQLite writer
transaction. An operation may reference a record created earlier in the same
batch. Any invalid operation refuses the whole batch; per-operation reason
codes are diagnostic and never imply partial commit.

Slice 20 dependency membership has a separate monotonic generation because it
does not own projection work. A batch containing one or more successful
dependency additions advances that generation once and returns it as
`resulting_dependency_generation`; a batch without such a change returns
`None`. `resulting_write_boundary` retains its canonical/global meaning.

The compact operation table stores only operation ID, request digest, terminal
outcome/reason codes, opaque affected IDs, boundaries, and work/closure IDs. It
stores no record bodies, query text, source locators, or prepared request.

- Same operation ID and digest returns the identical terminal receipt.
- Same operation ID and different digest returns `operation_id_conflict`.
- A crash before transaction commit leaves neither mutation nor receipt; retry
  executes normally.
- A crash after commit replays the stored receipt.

There is no admitted/in-progress public state in 0.8.25. SQLite atomic commit,
not a second public recovery protocol, is the crash boundary.

## Validation, lifecycle, and limits

Validation checks Slice 15 identity/provenance, Slice 20 dependency roles,
allowed lifecycle transitions, expected boundary, projection declarations,
payload limits, and at most 128 operations/2,048 total references. The batch
cannot invoke a provider, model, network, GPU, query decomposition, semantic
merge, or truth judgment.

A lifecycle action that requires Slice 30 propagation commits its root barrier
and closure intent atomically and returns `committed_closure_pending`. Other
committed batches return `committed`. The receipt is compact; detailed
consequences remain queryable through the owning record, dependency,
projection, and closure APIs.

All types follow Slice 15 wire and SDK rules. Failures include invalid
operation/reference/transition, dependency conflict, boundary mismatch,
projection contract mismatch, bounds, barrier conflict, and idempotency
conflict.

## RED/GREEN and verification

RED tests inject failure at every operation position, invalid forward
references, mismatched boundaries, same-ID/different-digest races, limits, and
codec drift. GREEN proves no partial domain/receipt/projection state, legal
within-batch forward references, identical replay, before/after-commit crash
behavior, terminal non-content storage, and Slice 30 closure intent creation.

Run fast, heavy, all/all-feature, Windows Rust/Python/Node, and locally packed
artifact routes. Run the operator-feature route only if the reused lifecycle
transition is operator-gated; otherwise record it N/A at readiness. CUDA,
live-model, and pre-publication registry routes are N/A. A formal independent
READY review remains required after Slice 7 and Slice 20 complete.
