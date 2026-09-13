---
title: FathomDB 0.8.26 Slice 40 — atomic derived-edge actuation
status: DRAFT
---

# Slice 40 plan — atomic derived-edge actuation

## Slice-complete workflow

This plan adopts the full [lean slice execution contract](../../slice-execution-contract.md):
enumerate intervening changes and allocations, finalize needs/requirements/AC,
obtain design review, implement RED/GREEN, obtain code review and independent
verification, write status, and clean up temporary workspaces.

## Outcome

One governed actuation transaction can commit a derived node, its canonical
dependency, and a provenance-bearing semantic edge, with deterministic replay
and a truthful bounded changed-in-place V1 receipt in a fresh 0.8.26 database.

## Requirements

- **R26-40A:** Change `ActuationBatchV1`/`ActuationOperationV1` in place to
  contain the four inherited capabilities plus
  `put_derived_edge(ProvenancedEdgeV1)`. Add no parallel V2 types, parser,
  router, redirect, or method.
- **R26-40B:** Commit node, dependency, edge, replay record, receipt, and
  projection work atomically or not at all.
- **R26-40C:** Require both derived-edge endpoints in the complete prospective
  batch state, including endpoints introduced later in the batch. Refuse the
  whole batch when either endpoint is absent; do not admit or count dangling
  derived edges. Evaluate the final active state after all same-batch node and
  lifecycle effects. The edge's own position is irrelevant; existing
  node/lifecycle ordering rules determine the final state.
- Reuse canonical edge persistence, lifecycle, provenance, projection, and
  traversal paths.
- **R26-40D:** Implement one changed-in-place V1 receipt, digest, replay,
  operation-ID, and integrity contract. Do not retain historical request,
  replay, receipt, integrity, or cross-release operation-ID behavior.
- **R26-40E:** Accept only fresh 0.8.26 databases. Refuse a representative
  earlier database before mutation and implement no historical migration
  matrix.

## TDD and delivery

1. Apply accepted ADR-0.8.26 and finalize the exact current V1 wire/binding contracts.
2. Commit failing tests for successful mixed batches and every validation,
   crash, duplicate, replay, lifecycle, erasure, and projection boundary.
3. Implement the approved prospective-state policy and one writer transaction.
4. Update the one V1 digest contract in place and remove historical compatibility paths.
5. Bind Rust, Python, TypeScript, and wire surfaces with shared fixtures.
6. Run focused property, changed-boundary fault, restart, concurrency,
   projection, and package checks plus canonical `agent-verify`; reserve the
   broad platform matrix for Slice 50.
7. Obtain independent high-risk implementation review.

## Acceptance

- **AC26-40A:** Current V1 construction is compatible across bindings; no
  public or internal parallel V2 parser, method, or router exists.
- **AC26-40B:** The complete Memex graph-authoring unit has one transaction and one replay
  identity/receipt.
- **AC26-40C:** Missing `from`, `to`, or both refuse deterministically without
  domain commits; an edge whose endpoints occur later in the same batch
  succeeds; a same-batch lifecycle transition that leaves an endpoint
  non-active refuses regardless of whether the edge appears before or after it.
- Replay of an identical request returns the original outcome; key reuse with
  different bytes refuses.
- **AC26-40D:** Current V1 receipt/storage, replay, integrity, and operation-ID behavior
  are tested on a fresh database; receipts contain no Memex semantic verdict or
  uncomputed consequence claim, and no historical compatibility path remains.
- **AC26-40E:** Fresh bootstrap succeeds and an earlier database is refused
  before mutation without version-by-version migration testing.

## Stop gates

Stop on partial commit, replay/digest ambiguity, a parallel V2 surface,
historical translation or receipt support, migration of an earlier database, unbounded
work, unclear erasure behavior, projection divergence, or receipt disclosure
beyond the approved minimum.
