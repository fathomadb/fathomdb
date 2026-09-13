---
title: FathomDB 0.8.26 Slice 8 — D26-03 through D26-05 actuation shape spike
status: COMPLETE_READ_ONLY
observed_on: 2026-09-12
last_reconciled: 2026-09-13
---

# D26-03 through D26-05 actuation shape spike

## Question and method

This read-only scope spike mapped the exact engine, persistence, binding,
contract, and test surfaces for changing the full V1 actuation grammar versus
adding an edge-specific request. It also quantified how endpoint semantics
affect receipt evolution and performance. It made no code, schema, or contract
change.

The evidence supports a dedicated implementation spike, named **Slice 35 —
actuation contract and performance spike**, after Slice 30 and before Slice 40.
Slice 35 must have its own plan and design, design review, human-authored RED
tests, minimal GREEN prototype, code review, independent verification, status
record, and cleanup. Slice 40 may proceed only from the accepted Slice 35
contract.

## D26-03 superseding final ruling: break V1 in place

The public request remains:

```text
ActuationBatchV1
  PutCanonicalNode(ProvenancedNodeV1)
  PutDerivedNode(ProvenancedNodeV1)
  PutDerivedEdge(ProvenancedEdgeV1)
  RegisterSourceDependency(SourceDependencyRegistrationV1)
  TransitionLifecycle(LifecycleActuationV1)
```

The changed V1 contract contains the four existing operation capabilities plus
`PutDerivedEdge`; it is not edge-only. An edge-only request cannot atomically
combine Memex's derived node, dependency registration, and semantic edge.

HITL `seq-283` supersedes `seq-282` as to V2 naming and redirects. V1 remains
the only functional grammar and uses one actuation method per binding. The V1
request and receipt types change in place. There is no V2 type, parser, method,
router, redirect response, or mixed-version interaction mode.

0.8.26 accepts fresh databases only. It carries no compatibility for earlier
receipt, integrity, provenance-row, lifecycle-row, dependency-row,
source-reference, operation-ID, upgrade, or downgrade data. Current V1 still
validates all of its corresponding invariants.

The expected scope remains approximately 15–20 code/contract files and 8–12
focused test or fixture files. Risk is medium and concentrated in the actuation
executor and bindings; storage risk remains low under the strict endpoint
policy below. An edge-only API touches fewer surfaces but fails the atomic
graph-unit requirement and is rejected.

## D26-04 and reframed D26-05 rulings

Ordinary `Engine::write` returns its dangling endpoint count in
`WriteReceipt.dangling_edge_endpoints`. `ActuationReceiptV1` and its durable
table have no equivalent field. Therefore D26-04 option A cannot truthfully
preserve ordinary flag/count semantics across replay without changing the V1
receipt/storage contract, bindings, erasure, integrity, and corruption checks.
Recomputing the count during replay would be wrong because endpoint state can
change.

HITL selected D26-04 option B at `seq-284`: refuse a derived edge
whose endpoints are absent from the complete prospective batch state. An edge
may precede endpoints that occur later in the same batch. Only endpoint
existence receives this complete-batch exception; provenance and dependency
ordering retain their existing semantics.

HITL selected D26-05 option A at `seq-285`: define a compact changed-in-place
`ActuationReceiptV1` without a dangling count. There is one fresh-database
current-contract operation-ID namespace and no cross-release behavior.

Missing endpoints can reuse `reference_unavailable` with deterministic first
paths: `/operations/{i}/record/from` before `/to`. This avoids a new public
refusal variant.

Choosing flag/count option A instead adds approximately 6–10 schema, integrity,
receipt, and erasure surfaces and raises stability risk to high. The choice is
warranted only if Memex explicitly requires admission of incomplete graphs and
replayable dangling counts.

## Performance assessment

There is no performance-preservation guarantee relative to 0.8.25. Slice 35
measures current V1 against the 0.8.25 V1 implementation only as
characterization, not as a compatibility gate.

Current actuation performs domain work once in rollback-only simulation and
again in the committed transaction. A derived edge therefore performs
provenance, projection, and endpoint-index work twice. Endpoint probes use the
partial active-logical-ID index and remain linear within the 128-operation
batch cap. The material effects are longer serialized writer-lock occupancy,
concurrent caller queueing, duplicate simulation/application work, and the need
to include edge projection cursors in the receipt and notifications.

Strict refusal does not require both the old dangling-count probes and a second
independent endpoint pass. The implementation should reuse a prepared, indexed
complete-state check and avoid speculative refactoring of ordinary
`commit_batch` until measurements justify it. A three-operation atomic Memex
unit is expected to cost less than separate transactional node/dependency and
edge calls while removing their crash boundary, but the release must measure
rather than assume that result.

## Slice 35 required plan and design

Slice 35 is a bounded implementation and performance spike, not the product
implementation. It must preserve or discard prototype code explicitly at
close.

### Requirements and RED evidence

1. Add current V1 digest golden and property tests; retain no historical
   encoder or replay compatibility path.
2. Prove every inherited operation capability produces its required current
   V1 domain effects without comparing historical persisted data.
3. Prove one derived node, dependency, and derived edge commit atomically with
   one operation ID, receipt, and transaction.
4. Prove an edge before endpoints later in the batch succeeds, while missing
   `from`, `to`, or both refuse deterministically without domain commits.
5. Prove invalid provenance/hash, revision collision, projection failure, and
   injected post-operation failures roll back the unit.
6. Prove exact current V1 replay across restart, changed-byte conflict,
   concurrent current-contract behavior, and erased operation-ID reservation.
7. Prove edge supersession, source-erasure receipt redaction, FTS/vector
   projection, traversal, and lifecycle behavior.
8. Add a cross-binding current V1 conformance fixture and exact closed-shape,
   precedence, Unicode, NUL, and JSON-pointer tests.
9. Prove no parallel V2 types, parser, router, redirect, method, or functional
   internals exist.
10. Prove fresh bootstrap and one representative earlier database refusal
    before mutation; do not build a historical migration matrix.

### GREEN prototype and measurements

- Implement only the changed-in-place V1 executor; add no version-dispatch
  path.
- Compare the 0.8.25 V1 baseline with 1,000 sequential current V1
  node/dependency/edge batches.
- Exercise the canonical three-operation Memex unit and a 128-operation
  edge-heavy bound.
- Run eight concurrent callers with unique IDs and separately with one shared
  exact ID.
- Record median, p95, total throughput, writer-lock and slow-event incidence,
  pending projection count, receipt size, and database growth.
- Use measurements to decide whether a narrowly scoped internal endpoint-probe
  change is justified; do not refactor the ordinary write path speculatively.

## Final disposition

D26-03 is accepted at `seq-283`, D26-04 at `seq-284`, and D26-05 at
`seq-285`. Slice 35 may now prove the changed V1 contract and performance
before Slice 40 implementation. The prior V2 and cross-version
recommendations are superseded.
