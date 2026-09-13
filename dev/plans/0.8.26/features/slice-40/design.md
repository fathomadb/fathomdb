---
title: FathomDB 0.8.26 Slice 40 — atomic derived-edge design
status: DRAFT
---

# Slice 40 design — atomic derived-edge actuation

## Accepted breaking boundary and proposed remaining contract

Per accepted
[`ADR-0.8.26-breaking-v1-contract-and-fresh-database-boundary.md`](../../../../adr/ADR-0.8.26-breaking-v1-contract-and-fresh-database-boundary.md),
change `ActuationBatchV1` in place and add
`ActuationOperationV1::PutDerivedEdge` carrying the existing
`ProvenancedEdgeV1` shape. Keep one actuation method per binding. Do not add a
parallel V1/V2 method family, parser, router, or redirect response.

The operation is deliberately `put_derived_edge`, not arbitrary `put_edge`:
immutable revision identity and source/dependency provenance are mandatory for
governed derived graph authoring.

## Transaction algorithm

1. Validate current V1 batch size, operation grammar, identities, provenance, and
   idempotency digest without writing.
2. Build the final bounded prospective active-identity set from persisted
   endpoints plus all same-batch node and lifecycle operations, applying the
   contract's existing last-operation and lifecycle rules. The edge's own
   position does not limit which same-batch endpoint operations are visible.
3. Refuse any derived edge whose endpoints are absent from the complete
   prospective active state. Endpoints introduced later in the batch satisfy
   this check unless a same-batch lifecycle transition leaves them non-active;
   the edge's own position is not semantically significant.
4. In one writer transaction, apply nodes, dependencies, derived edges,
   lifecycle effects, replay record, receipt, and projection work.
5. Commit once. Any error or injected interruption rolls back the complete
   unit; restart uses the durable replay record.

## Current V1 receipt boundary

Define one changed-in-place V1 receipt/storage contract containing the current
truthful outcome, operation identity and digest, refusal, affected revisions,
transaction/projection boundaries, generation/closure, internal
source-reference concepts, and only edge fields proven necessary. It has no
dangling count and does not compute Memex support/refutation meaning,
downstream semantic consequences, or a general dependency manifest. There is no
historical receipt reader, integrity path, replay path, or shared cross-release
operation-ID rule.

## Persistence and fresh-database boundary

Current V1 may reuse the node, edge, provenance, dependency, and projection
designs as its fresh schema. It does not interpret their earlier-version rows.
Fresh-database creation may reuse internal schema-construction code, but open
must refuse an existing non-current database before mutation. No upgrade or
downgrade path, historical migration matrix, historical receipt table reader,
or historical integrity compatibility is implemented.

Current V1 uses one explicit digest domain and operation-ID namespace. These
endpoint and receipt boundaries are ruled at `seq-284` and `seq-285`.
