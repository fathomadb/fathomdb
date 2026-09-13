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
2. Build a bounded prospective identity set from persisted endpoints and all
   same-batch node operations without making order semantically significant.
3. Apply the Slice 8 endpoint decision: preserve ordinary flag-and-count
   dangling semantics, or refuse dangling derived edges under an explicitly
   different governed-edge contract.
4. In one writer transaction, apply nodes, dependencies, derived edges,
   lifecycle effects, replay record, receipt, and projection work.
5. Commit once. Any error or injected interruption rolls back the complete
   unit; restart uses the durable replay record.

## Current V1 receipt boundary

Define one changed-in-place V1 receipt/storage contract containing only fields known in the
transaction. It does not compute Memex support/refutation meaning, downstream
semantic consequences, or a general dependency manifest. There is no
historical receipt reader, integrity path, replay path, or shared cross-release
operation-ID rule.

## Persistence and fresh-database boundary

Current V1 may reuse the node, edge, provenance, dependency, and projection
designs as its fresh schema. It does not interpret their earlier-version rows.
Fresh-database creation may reuse internal schema-construction code, but open
must refuse an existing non-current database before mutation. No upgrade or
downgrade path, historical migration matrix, historical receipt table reader,
or historical integrity compatibility is implemented.

Current V1 uses one explicit digest domain and operation-ID namespace. The exact
changed-in-place V1 receipt shape and D26-04 endpoint policy remain subject to
the unfinished Slice 8 review.
