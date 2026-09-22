---
title: FathomDB 0.8.27 Slice 20 - correction-safe erasure design
status: IMPLEMENTED
target_release: 0.8.27
observed_on: 2026-09-21
---

# Slice 20 - correction-safe erasure design

## Existing failure and invariants

The existing `erase_source_shared` ordering remains the outer contract:
validate -> finish any durable retry -> drain -> freeze -> one immediate
SQLite transaction -> physical-closure finalization -> telemetry redaction ->
WAL checkpoint -> success. The correction changes only the transaction body.

Within `excise_source_inner`, a correction actuation can leave a completed
`source_revision` closure referenced by its durable actuation receipt. Current
code deletes that closure before `redact_actuation_receipts_for_refs` loads and
validates the receipt. Receipt validation requires every referenced closure to
exist, so the operation returns `Storage` and the transaction rolls back. This
matches the exact Memex failure without requiring a new public concept.

The following invariants are load-bearing:

- one-source, direct-dependency semantics only;
- one immediate transaction for primary deletion and durable retry admission;
- requested-bucket report counts retain their existing meaning;
- pre-commit failure rolls back every plane;
- post-commit at-rest failure returns `ErasureIncomplete` with retry state;
- one committed-deletion audit row, no additional scrub-only retry audit;
- raw `source_id` survives only in the accepted audit row and, when physical
  dependents exist, one `source_bucket` closure root;
- no schema, wire, error taxonomy, method, or report shape changes.

## Transaction design

### 1. Freeze the requested inventory

Collect and retain the requested bucket's node and edge cursors; they determine
`projections_invalidated`. `nodes_excised` and `edges_excised` are the
rowcounts of the requested bucket's `DELETE … WHERE source_id` statements. No
earlier step in the transaction deletes canonical rows, so those rowcounts equal
the pre-delete inventory without a separate freeze step.

Resolve canonical source revisions from those cursors and use the existing
persisted-chain validator to discover their direct physical dependents. RED
fixture construction proved the accepted one-source invariant rejects a
dependent whose row `source_id` differs from its canonical source. Validated
direct dependents are therefore already in the requested cursor inventory.
Do not expand outside it and do not recurse beyond the shipped model.

### 2. Capture complete erasure evidence before mutation

Using the requested cursor sets, collect stable telemetry ids, actuation receipt
references, and the physical-proof scope before deleting any row. Include the
requested `source_id` in receipt-reference cleanup even when the bucket is
empty. Record the existing `source_bucket` physical closure only when the
validated plan contains physical dependents.

### 3. Preserve receipt validation order

Redact actuation receipts for the complete requested reference set while their
referenced completed soft-closure rows still exist. Only after validation and
redaction succeeds may the transaction delete completed `source_revision`
closure rows for every erased requested source revision. Cleanup is not limited
to roots whose current direct-dependent plan is nonempty, because a completed
closure referenced by a receipt remains validation-significant even after its
dependent set is empty.

Both actions remain in the same immediate transaction. A corrupt receipt,
closure, dependency, or trigger therefore rolls the whole primary mutation
back; no partial source deletion becomes visible.

### 4. Delete requested state and preserve its counts

Erase artifact identity, dependency registration, actuation references,
row-owned projections, and canonical rows for the requested cursor sets. The
existing requested-bucket deletes remain the idempotent catch-all for raw rows
lacking artifact identity.

Accumulate projection deletion counts from the requested cursors, matching the
existing requested-bucket contract.

The telemetry pending-redaction obligation contains stable ids from the
requested set. The physical proof also measures that set, dependency
rows, source links/versions, and receipt references. Finalization retains the
current exact-zero proof.

## Failure and retry behavior

- Persisted-chain validation, receipt validation, physical-closure admission,
  or any transaction write failure occurs before commit and returns the
  existing error while SQLite restores the exact pre-call state.
- Telemetry redaction or WAL checkpoint failure occurs after the primary
  transaction commits and returns the existing `ErasureIncomplete` variant.
- The retry path detects the durable physical closure/pending redaction,
  performs only outstanding at-rest work, returns zero report counts, does not
  append another audit row, and completes the same closure idempotently.

## Tests and public documentation

The Rust matrix is the deep owner. It seeds unique bodies and identities across
canonical rows, derived dependents, unrelated survivors, projection/vector
rows, actuation receipts, closures, telemetry, and WAL-visible state. Exact
SQL assertions run after engine close and independent reopen. A broad string
allowlist is prohibited; accepted audit/closure identity rows are asserted by
table, key, field, and count.

Python and TypeScript exercise only public corrected success and typed
`ErasureIncomplete` mapping. They do not duplicate the physical matrix.
Maintained Rust/Python/TypeScript interface and operations documentation will
clarify that already-closed direct dependents are physically erased while the
report remains scoped to the requested bucket.

No ADR change is needed: this restores the accepted erasure and dependency-
closure contracts rather than choosing a new architecture.
