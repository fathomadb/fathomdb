---
title: FathomDB 0.8.26 Slice 40 — atomic derived-edge design
status: DRAFT
---

# Slice 40 design — atomic derived-edge actuation

## Proposed contract

Preserve the exhaustive V1 grammar. Introduce `ActuationBatchV2` and an
`ActuationOperationV2::PutDerivedEdge` carrying the existing
`ProvenancedEdgeV1` shape. Provide versioned engine and binding entry points;
do not silently reinterpret V1 request bytes.

The operation is deliberately `put_derived_edge`, not arbitrary `put_edge`:
immutable revision identity and source/dependency provenance are mandatory for
governed derived graph authoring.

## Transaction algorithm

1. Validate batch size, operation grammar, identities, provenance, and V2
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

## Receipt boundary

First prove whether the existing V1 receipt columns can truthfully represent a
V2 request. A successor receipt/storage version is added only for a concrete
audit requirement and may contain only fields already known in the
transaction. It does not compute Memex support/refutation meaning, downstream
semantic consequences, or a general dependency manifest. Shared operation-ID
storage requires an explicit cross-version collision rule.

## Persistence and compatibility

Reuse existing node, edge, provenance, dependency, mutation, and projection
tables. A new schema or migration is not assumed and triggers a stop. V2 uses
an explicit digest domain/version; all V1 golden bytes and replay fixtures must
remain unchanged.
