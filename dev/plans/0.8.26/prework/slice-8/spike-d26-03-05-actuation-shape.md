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

## D26-03: V2 is a full successor grammar

The recommended public request is:

```text
ActuationBatchV2
  PutCanonicalNode(ProvenancedNodeV1)
  PutDerivedNode(ProvenancedNodeV1)
  PutDerivedEdge(ProvenancedEdgeV1)
  RegisterSourceDependency(SourceDependencyRegistrationV1)
  TransitionLifecycle(LifecycleActuationV1)
```

V2 includes the complete V1 operation surface plus `PutDerivedEdge`; it is not
an edge-only request. An edge-only request cannot atomically combine Memex's
derived node, dependency registration, and semantic edge. A composite graph
unit would duplicate node and dependency contracts and unnecessarily constrain
multi-edge batches.

V1 remains unchanged. V2 receives separate Rust, Python, and TypeScript entry
points and the digest domain `fathomdb.actuation.v2\0`. Both public versions
should normalize to one private executor representation rather than copy the
existing actuation implementation.

## Proposed V1 and V2 compatibility contract

V2 is comprehensive only within the actuation-batch family. It is not a new
version of the entire Engine, SDK, database, read surface, search surface, or
graph surface. It introduces one additional batch type and one corresponding
entry point per binding. Existing clients that use only V1 require no change.

At the capability level, V2 is a strict superset of V1: it contains equivalent
forms of every V1 operation plus `PutDerivedEdge`. At the type and wire level,
the versions remain separate closed schemas. A V1 batch is not silently parsed
as V2, a V2 batch is not accepted by the V1 entry point, and no V1 digest or
canonical byte sequence changes. A caller may explicitly construct the V2
equivalent of a V1 request when it wants the successor surface.

There is no session-wide version negotiation or version mode. One Engine or
SDK client may issue calls in any order, including:

```text
actuate(V1) -> actuate_v2(V2) -> actuate(V1) -> actuate(V1) -> actuate_v2(V2)
```

Sequential mixed-version calls operate on the same database state and obey
the same transaction, provenance, lifecycle, projection, and erasure rules.
Concurrent V1 and V2 calls are also allowed, but the existing single-writer
boundary serializes their commits. Concurrency changes arrival and completion
order, not atomicity.

The operation-ID namespace is shared across versions:

- different operation IDs work in any sequential or concurrent V1/V2 order;
- an exact V1 replay uses the V1 digest and returns its stored receipt;
- an exact V2 replay uses the V2 digest and returns its stored receipt;
- reusing one operation ID across V1 and V2 conflicts, even when the listed
  domain operations appear equivalent, because the versioned digest domains
  differ; and
- an erased operation ID remains reserved against both versions.

The proposal returns the existing `ActuationReceiptV1` from both entry points
under strict endpoint refusal. This is intentional: the receipt describes a
committed or refused operation outcome and already carries the required
revision, cursor, dependency, lifecycle, and source-reference truth. Request
schema version and receipt schema version are independent. A RED audit must
prove that claim before Slice 40; otherwise receipt evolution returns to HITL.

Thus V2 is best described as **a separate, full successor grammar for one API
family**, not “an updated V1” and not “a comprehensive new FathomDB surface.”
Internally the two versions share execution logic; publicly they remain
unambiguous and independently replayable.

The expected scope is approximately 15–20 code/contract files and 8–12 focused
test or fixture files. Risk is medium and concentrated in the actuation
executor and bindings; storage risk remains low under the strict endpoint
policy below. An edge-only API touches perhaps 8–12 surfaces but fails the
atomic graph-unit requirement and is rejected.

## D26-04 and D26-05 are coupled

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

Under option B, D26-05 option A can return the existing
`ActuationReceiptV1`, retain existing receipt tables, and treat receipt version
and request version as independent. The V1 and V2 operation-ID namespace is
shared. Exact V2 replay returns the stored receipt; reuse of one ID with V1 and
V2 conflicts because the digest domains differ; erased IDs remain reserved for
both versions.

Missing endpoints can reuse `reference_unavailable` with deterministic first
paths: `/operations/{i}/record/from` before `/to`. This avoids a new public
refusal variant.

Choosing flag/count option A instead adds approximately 6–10 schema,
integrity, receipt, and erasure surfaces and raises stability risk to high.
The choice is warranted only if Memex explicitly requires admission of
incomplete graphs and replayable dangling counts.

## Performance assessment

V1 keeps its entry point, digest, encoding, and execution behavior. Its
sequential and concurrent performance should therefore remain stable unless
shared executor refactoring changes the path; Slice 35 must measure this
sentinel.

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

1. Add V2 digest golden and property tests while proving every V1 golden byte
   remains unchanged.
2. Prove every inherited V1 operation produces equivalent domain effects
   through V2.
3. Prove one derived node, dependency, and derived edge commit atomically with
   one operation ID, receipt, and transaction.
4. Prove an edge before endpoints later in the batch succeeds, while missing
   `from`, `to`, or both refuse deterministically without domain commits.
5. Prove invalid provenance/hash, revision collision, projection failure, and
   injected post-operation failures roll back the unit.
6. Prove exact V2 replay across restart, changed-byte conflict, sequential and
   concurrent V1/V2 ID collision behavior, and erased-ID reservation.
7. Prove edge supersession, source-erasure receipt redaction, FTS/vector
   projection, traversal, and lifecycle behavior.
8. Add a cross-binding V2 conformance fixture and exact closed-shape,
   precedence, Unicode, NUL, and JSON-pointer tests.

### GREEN prototype and measurements

- Normalize V1 and V2 to one private executor representation without changing
  V1 construction, digest, public entry point, or behavior.
- Compare 1,000 sequential V1 node/dependency batches with V2
  node/dependency/edge batches.
- Exercise the canonical three-operation Memex unit and a 128-operation
  edge-heavy bound.
- Run eight concurrent callers with unique IDs and separately with one shared
  exact ID.
- Record median, p95, total throughput, writer-lock and slow-event incidence,
  pending projection count, receipt size, and database growth.
- Require no repeatable V1 regression beyond normal noise. Use measured
  evidence to decide whether a narrowly scoped internal endpoint-probe change
  is justified; do not refactor the ordinary write path speculatively.

## Recommendation to HITL

Approve D26-03 option A as the full successor grammar above. Rule D26-04 and
D26-05 together: approve strict complete-state endpoint refusal and current
receipt storage. Preserve flag/count only if Memex supplies an affirmative
incomplete-graph requirement and accepts the added durable-schema work.
