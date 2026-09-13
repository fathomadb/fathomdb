---
title: FathomDB 0.8.26 Slice 35 — breaking V1 actuation spike design
status: DRAFT
---

# Slice 35 design — breaking V1 actuation spike

## Fixed decisions

- 0.8.26 changes affected V1 contracts in place and introduces no parallel
  functional V1/V2 API pairs (`seq-283`).
- There is one actuation method per binding and one functional V1 grammar.
- Current V1 includes canonical node, derived node, derived edge, source dependency,
  and lifecycle transition operations.
- No V2 parser, method, router, redirect, or interaction mode is introduced.
- No historical request, digest, replay, receipt, integrity, data, operation-ID,
  upgrade, downgrade, or database-migration compatibility is retained.
- 0.8.26 accepts fresh databases only.

## Endpoint and receipt decisions

- Derived edges require both endpoints in the complete prospective batch
  state; endpoints later in the batch count (`seq-284`). Missing endpoints
  refuse the batch, and no dangling count is admitted.
- `ActuationReceiptV1` changes in place and stays compact (`seq-285`). Retain
  the current truthful outcome, operation identity and digest, refusal,
  affected revisions, transaction/projection boundaries, generation/closure,
  and internal source-reference concepts; add only edge fields the spike
  proves necessary. Do not add a graph-consequence manifest.

## Prototype seams

### Ingress

The Rust facade continues to export V1 types. Python, TypeScript, PyO3, and
N-API validate the changed V1 contract as one closed schema. The package
version identifies the breaking revision; the public schema-generation name
does not change.

Use one V1 parser and the existing actuation method. Do not add V2 types, a
V1/V2 union input, a version router, a redirect response, or separate
`actuate_v1`/`actuate_v2` methods.

### Fresh database

Fresh creation may reuse internal ordered schema-construction functions. Open
must distinguish a new empty database from an existing non-current database;
the latter is refused before any DDL, metadata, WAL, receipt, or domain-row
mutation. The spike proves one representative earlier database, not every
historical version.

### Current V1 transaction

Current V1 validates its closed grammar and digest, simulates the bounded
complete batch with the existing node/lifecycle ordering rules, and computes
the final active logical-identity set. It refuses any derived edge whose
endpoints are absent or non-active in that complete prospective state,
including when a same-batch lifecycle operation removes eligibility after a
node put. The edge's own position in the batch does not change this test. The edge
uses canonical `ProvenancedEdgeV1` validation, identity, provenance,
projection, lifecycle, and traversal machinery as implementation reuse, not as
historical database compatibility.

### Current V1 receipt and replay

Use one current V1 digest domain, operation-ID namespace, receipt schema, source-reference
model, integrity checker, erasure behavior, and replay loader. Do not add a
request-version column or cross-release branch. Exact current V1 replay returns
its current V1 receipt; changed bytes conflict; erased operation IDs remain
reserved.

## Performance design

The edge path runs during simulation and committed application, increasing
serialized writer occupancy. Endpoint checks must use bounded indexed probes
over the complete prospective state. The spike measures before refactoring;
ordinary `Engine.write` is outside the optimization scope unless evidence
shows a shared defect that cannot be isolated.

The spike crosses the edge's position with node-put and lifecycle-operation
orderings so the endpoint test cannot accidentally use the edge's insertion
position or pre-batch active state. Existing last-operation/lifecycle semantics
still determine the final prospective state.

## Stop conditions

Stop on a parallel V2 parser or execution path, historical receipt/integrity
loading, earlier-database mutation, migration code, partial commit, ambiguous replay,
unindexed endpoint work, cross-binding drift, or semantic policy entering the
engine.
