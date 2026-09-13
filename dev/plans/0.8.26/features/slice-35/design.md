---
title: FathomDB 0.8.26 Slice 35 — breaking V2 actuation spike design
status: DRAFT
---

# Slice 35 design — breaking V2 actuation spike

## Fixed decisions

- 0.8.26 is a breaking V2-only actuation release (`seq-282`).
- There is one actuation method per binding and one functional V2 grammar.
- V2 includes canonical node, derived node, derived edge, source dependency,
  and lifecycle transition operations.
- V1-shaped dynamic ingress receives only a loud non-executing V2 direction.
- No V1 request, digest, replay, receipt, integrity, data, operation-ID, upgrade,
  downgrade, or database-migration compatibility is retained.
- 0.8.26 accepts fresh databases only.

## Open inputs

D26-04 still selects strict complete-state endpoint refusal versus a V2
dangling-count receipt. D26-05 still selects the exact minimum V2 receipt
fields. No option may reintroduce historical compatibility.

## Prototype seams

### Ingress

The Rust facade exports V2 types only. Python, TypeScript, PyO3, and N-API
validate V2 as a closed schema. If a dynamic boundary recognizes the retired
top-level V1 discriminator, it returns the approved upgrade direction before
nested parsing and before opening a write transaction.

### Fresh database

Fresh creation may reuse internal ordered schema-construction functions. Open
must distinguish a new empty database from an existing non-current database;
the latter is refused before any DDL, metadata, WAL, receipt, or domain-row
mutation. The spike proves one representative earlier database, not every
historical version.

### V2 transaction

V2 validates its closed grammar and digest, simulates the bounded complete
batch, then applies it under the existing single-writer transaction. The edge
uses canonical `ProvenancedEdgeV1` validation, identity, provenance,
projection, lifecycle, and traversal machinery as implementation reuse, not as
historical database compatibility.

### V2 receipt and replay

Use one V2 digest domain, operation-ID namespace, receipt schema, source-reference
model, integrity checker, erasure behavior, and replay loader. Do not add a
request-version column or branch for V1. Exact V2 replay returns its V2 receipt;
changed V2 bytes conflict; erased V2 IDs remain reserved.

## Performance design

The edge path runs during simulation and committed application, increasing
serialized writer occupancy. Endpoint checks must use bounded indexed probes
over the complete prospective state. The spike measures before refactoring;
ordinary `Engine.write` is outside the optimization scope unless evidence
shows a shared defect that cannot be isolated.

## Stop conditions

Stop on functional V1 parsing or execution, V1 receipt/integrity loading,
earlier-database mutation, migration code, partial commit, ambiguous V2 replay,
unindexed endpoint work, cross-binding drift, or semantic policy entering the
engine.
