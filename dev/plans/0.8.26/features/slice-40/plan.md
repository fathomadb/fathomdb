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

- Preserve `ActuationBatchV1` encoding, digest, replay, and behavior.
- Introduce a successor batch/operation contract containing
  `put_derived_edge(ProvenancedEdgeV1)` if approved in Slice 8.
- Validate edge endpoints against persisted plus earlier same-batch prospective
  state before any write.
- Reuse canonical edge persistence, lifecycle, provenance, projection, and
  traversal paths.
- Commit node, dependency, edge, replay record, receipt, and projection work
  atomically or not at all.
- Extend receipts only with directly computed information required for this
  operation.

## TDD and delivery

1. Accept a successor ADR and exact wire/binding contracts.
2. Commit failing tests for successful mixed batches and every validation,
   crash, duplicate, replay, lifecycle, erasure, and projection boundary.
3. Implement two-phase prospective validation and one writer transaction.
4. Add a V2 digest domain and prove V1 byte/replay preservation.
5. Bind Rust, Python, TypeScript, and wire surfaces with shared fixtures.
6. Run property, fault-injection, restart, concurrency, projection, package,
   and full repository gates.
7. Obtain independent high-risk implementation review.

## Acceptance

- The complete Memex graph-authoring unit has one transaction and one replay
  identity/receipt.
- Missing or invalid endpoints refuse before mutation; same-batch earlier
  endpoints are accepted deterministically.
- Replay of an identical request returns the original outcome; key reuse with
  different bytes refuses.
- V1 behavior and persisted identity are unchanged.
- Receipts contain no Memex semantic verdict or uncomputed consequence claim.

## Stop gates

Stop on partial commit, replay/digest ambiguity, V1 drift, unbounded work,
schema migration, unclear erasure behavior, projection divergence, or receipt
disclosure beyond the approved minimum.
