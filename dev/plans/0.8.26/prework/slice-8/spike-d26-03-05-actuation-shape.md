---
title: FathomDB 0.8.26 Slice 8 — D26-03 through D26-05 actuation shape spike
status: COMPLETE_READ_ONLY
observed_on: 2026-09-12
---

# D26-03 through D26-05 actuation shape spike

## Question and method

This read-only scope spike mapped the exact engine, persistence, binding,
contract, and test surfaces for a full V2 actuation grammar versus an
edge-specific request. It also quantified how endpoint semantics affect
receipt evolution and performance. It made no code, schema, or contract
change.

The evidence supports a dedicated implementation spike, named **Slice 35 —
actuation V2 contract and performance spike**, after Slice 30 and before Slice
40. Slice 35 must have its own plan and design, design review, human-authored
RED tests, minimal GREEN prototype, code review, independent verification,
status record, and cleanup. Slice 40 may proceed only from the accepted Slice
35 contract.

## D26-03 final ruling: breaking V2-only grammar

The recommended public request is:

```text
ActuationBatchV2
  PutCanonicalNode(ProvenancedNodeV1)
  PutDerivedNode(ProvenancedNodeV1)
  PutDerivedEdge(ProvenancedEdgeV1)
  RegisterSourceDependency(SourceDependencyRegistrationV1)
  TransitionLifecycle(LifecycleActuationV1)
```

V2 contains the four operation capabilities introduced by V1 plus
`PutDerivedEdge`; it is not edge-only. An edge-only request cannot atomically
combine Memex's derived node, dependency registration, and semantic edge.

HITL `seq-282` rejects coexistence. V2 is the only functional grammar and uses
one actuation method per binding. Static V1 types are removed. Dynamic/native
V1-shaped ingress may only inspect the discriminator needed to return a loud
V2 direction; it cannot parse operations, translate, execute, digest, replay,
or load V1 receipts.

0.8.26 accepts fresh databases only. It carries no V1 receipt, integrity,
provenance-row, lifecycle-row, dependency-row, source-reference,
operation-ID, upgrade, or downgrade compatibility. V2 still validates all of
its own corresponding invariants.

The expected scope is approximately 15–20 code/contract files and 8–12 focused
test or fixture files. Risk is medium and concentrated in the actuation
executor and bindings; storage risk remains low under the strict endpoint
policy below. An edge-only API touches perhaps 8–12 surfaces but fails the
atomic graph-unit requirement and is rejected.

## D26-04 and reframed D26-05 remain open

Ordinary `Engine::write` returns its dangling endpoint count in
`WriteReceipt.dangling_edge_endpoints`. `ActuationReceiptV1` and its durable
table have no equivalent field. Therefore option A cannot truthfully preserve
ordinary flag/count semantics across replay without a nullable receipt-column
migration or a new receipt version, plus binding, erasure, integrity, and
corruption-check evolution. Recomputing the count during replay would be wrong
because endpoint state can change.

The lower-risk first implementation is D26-04 option B: refuse a derived edge
whose endpoints are absent from the complete prospective batch state. An edge
may precede endpoints that occur later in the same batch. Only endpoint
existence receives this complete-batch exception; provenance and dependency
ordering retain their existing semantics.

Under option B, D26-05 can define a compact `ActuationReceiptV2` without a
dangling count. There is one fresh-database V2 operation-ID namespace and no
cross-version behavior.

Missing endpoints can reuse `reference_unavailable` with deterministic first
paths: `/operations/{i}/record/from` before `/to`. This avoids a new public
refusal variant.

Choosing flag/count option A instead adds approximately 6–10 schema,
integrity, receipt, and erasure surfaces and raises stability risk to high.
The choice is warranted only if Memex explicitly requires admission of
incomplete graphs and replayable dangling counts.

## Performance assessment

There is no V1 performance-preservation requirement. Slice 35 measures V2
against the 0.8.25 V1 implementation only as characterization, not as a
compatibility gate.

Current actuation performs domain work once in rollback-only simulation and
again in the committed transaction. A V2 edge therefore performs provenance,
projection, and endpoint-index work twice. Endpoint probes use the partial
active-logical-ID index and remain linear within the 128-operation batch cap.
The material effects are longer serialized writer-lock occupancy, concurrent
caller queueing, duplicate simulation/application work, and the need to
include edge projection cursors in the receipt and notifications.

Strict refusal does not require both the old dangling-count probes and a
second independent endpoint pass. The implementation should reuse a prepared,
indexed complete-state check and avoid speculative refactoring of ordinary
`commit_batch` until measurements justify it. A three-operation atomic Memex
unit is expected to cost less than separate transactional node/dependency and
edge calls while removing their crash boundary, but the release must measure
rather than assume that result.

## Slice 35 required plan and design

Slice 35 is a bounded implementation and performance spike, not the product
implementation. It must preserve or discard prototype code explicitly at
close.

### Requirements and RED evidence

1. Add V2 digest golden and property tests; do not retain a functional V1
   encoder or replay path.
2. Prove every inherited operation capability produces its required V2 domain
   effects without comparing historical persisted data.
3. Prove one derived node, dependency, and derived edge commit atomically with
   one operation ID, receipt, and transaction.
4. Prove an edge before endpoints later in the batch succeeds, while missing
   `from`, `to`, or both refuse deterministically without domain commits.
5. Prove invalid provenance/hash, revision collision, projection failure, and
   injected post-operation failures roll back the unit.
6. Prove exact V2 replay across restart, changed-byte conflict, concurrent V2
   behavior, and erased V2 ID reservation.
7. Prove edge supersession, source-erasure receipt redaction, FTS/vector
   projection, traversal, and lifecycle behavior.
8. Add a cross-binding V2 conformance fixture and exact closed-shape,
   precedence, Unicode, NUL, and JSON-pointer tests.
9. Prove V1-shaped dynamic ingress returns only the approved V2 direction and
   causes no write; remove static public V1 types and functional V1 internals.
10. Prove fresh bootstrap and one representative earlier database refusal
    before mutation; do not build a historical migration matrix.

### GREEN prototype and measurements

- Implement only the V2 executor and minimal non-executing V1 discriminator
  refusal.
- Compare the 0.8.25 V1 baseline with 1,000 sequential V2
  node/dependency/edge batches.
- Exercise the canonical three-operation Memex unit and a 128-operation
  edge-heavy bound.
- Run eight concurrent callers with unique IDs and separately with one shared
  exact ID.
- Record median, p95, total throughput, writer-lock and slow-event incidence,
  pending projection count, receipt size, and database growth.
- Use measurements to decide whether a narrowly scoped internal endpoint-probe
  change is justified; do not refactor the ordinary write path speculatively.

## Recommendation to HITL

D26-03 is accepted at `seq-282`. Remaining HITL work is to rule D26-04's
endpoint policy and D26-05's minimum truthful V2-only receipt shape. The prior
current-receipt/cross-version recommendation is superseded.
