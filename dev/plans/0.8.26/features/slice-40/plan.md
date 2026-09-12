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
and a truthful bounded receipt.

## Requirements

- **R26-40A:** Preserve `ActuationBatchV1` encoding, digest, replay, and behavior.
  Introduce a successor batch/operation contract containing
  `put_derived_edge(ProvenancedEdgeV1)` if approved in Slice 8.
- **R26-40B:** Commit node, dependency, edge, replay record, receipt, and
  projection work atomically or not at all.
- **R26-40C:** Apply the Slice 8-approved endpoint policy. Either preserve
  ordinary flag-and-count dangling behavior or introduce a separately
  documented governed-edge refusal; both choices evaluate the complete
  prospective batch state unless order sensitivity is explicitly approved.
- Reuse canonical edge persistence, lifecycle, provenance, projection, and
  traversal paths.
- **R26-40D:** First test whether existing receipt storage truthfully represents
  V2. Extend it only for a demonstrated audit requirement, with explicit
  integrity/replay and shared operation-ID semantics.

## TDD and delivery

1. Accept a successor ADR and exact wire/binding contracts.
2. Commit failing tests for successful mixed batches and every validation,
   crash, duplicate, replay, lifecycle, erasure, and projection boundary.
3. Implement the approved prospective-state policy and one writer transaction.
4. Add a V2 digest domain and prove V1 byte/replay preservation.
5. Bind Rust, Python, TypeScript, and wire surfaces with shared fixtures.
6. Run focused property, changed-boundary fault, restart, concurrency,
   projection, and package checks plus canonical `agent-verify`; reserve the
   broad platform matrix for Slice 50.
7. Obtain independent high-risk implementation review.

## Acceptance

- **AC26-40A:** V1 behavior and persisted identity remain unchanged and V2
  construction is compatible across bindings.
- **AC26-40B:** The complete Memex graph-authoring unit has one transaction and one replay
  identity/receipt.
- **AC26-40C:** The approved dangling/endpoint policy is documented and tested
  against complete same-batch state, including later operations, unless Slice
  8 explicitly approves order sensitivity.
- Replay of an identical request returns the original outcome; key reuse with
  different bytes refuses.
- **AC26-40D:** Receipt/storage compatibility and cross-version operation-ID
  collisions are tested; receipts contain no Memex semantic verdict or
  uncomputed consequence claim.

## Stop gates

Stop on partial commit, replay/digest ambiguity, V1 drift, unbounded work,
schema migration, unclear erasure behavior, projection divergence, or receipt
disclosure beyond the approved minimum.
