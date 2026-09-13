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
and a truthful bounded V2 receipt in a fresh 0.8.26 database.

## Requirements

- **R26-40A:** Replace functional V1 actuation with one
  `ActuationBatchV2`/`ActuationOperationV2` contract containing the four
  inherited capabilities plus `put_derived_edge(ProvenancedEdgeV1)`. V1-shaped
  dynamic ingress may only return the approved non-executing V2 direction.
- **R26-40B:** Commit node, dependency, edge, replay record, receipt, and
  projection work atomically or not at all.
- **R26-40C:** Apply the Slice 8-approved endpoint policy. Either preserve
  ordinary flag-and-count dangling behavior or introduce a separately
  documented governed-edge refusal; both choices evaluate the complete
  prospective batch state unless order sensitivity is explicitly approved.
- Reuse canonical edge persistence, lifecycle, provenance, projection, and
  traversal paths.
- **R26-40D:** Implement one V2 receipt, digest, replay, operation-ID, and
  integrity contract. Do not retain V1 request, replay, receipt, integrity, or
  cross-version operation-ID behavior.
- **R26-40E:** Accept only fresh 0.8.26 databases. Refuse a representative
  earlier database before mutation and implement no historical migration
  matrix.

## TDD and delivery

1. Apply accepted ADR-0.8.26 and finalize the exact V2 wire/binding contracts.
2. Commit failing tests for successful mixed batches and every validation,
   crash, duplicate, replay, lifecycle, erasure, and projection boundary.
3. Implement the approved prospective-state policy and one writer transaction.
4. Add one V2 digest domain and remove functional V1 digest/replay paths.
5. Bind Rust, Python, TypeScript, and wire surfaces with shared fixtures.
6. Run focused property, changed-boundary fault, restart, concurrency,
   projection, and package checks plus canonical `agent-verify`; reserve the
   broad platform matrix for Slice 50.
7. Obtain independent high-risk implementation review.

## Acceptance

- **AC26-40A:** V2 construction is compatible across bindings; V1-shaped
  dynamic ingress returns the approved direction without parsing operations or
  mutating state, and no static V1 request type remains public.
- **AC26-40B:** The complete Memex graph-authoring unit has one transaction and one replay
  identity/receipt.
- **AC26-40C:** The approved dangling/endpoint policy is documented and tested
  against complete same-batch state, including later operations, unless Slice
  8 explicitly approves order sensitivity.
- Replay of an identical request returns the original outcome; key reuse with
  different bytes refuses.
- **AC26-40D:** V2 receipt/storage, replay, integrity, and operation-ID behavior
  are tested on a fresh database; receipts contain no Memex semantic verdict or
  uncomputed consequence claim, and no V1 compatibility path remains.
- **AC26-40E:** Fresh bootstrap succeeds and an earlier database is refused
  before mutation without version-by-version migration testing.

## Stop gates

Stop on partial commit, replay/digest ambiguity, functional V1 execution,
translation or receipt support, migration of an earlier database, unbounded
work, unclear erasure behavior, projection divergence, or receipt disclosure
beyond the approved minimum.
