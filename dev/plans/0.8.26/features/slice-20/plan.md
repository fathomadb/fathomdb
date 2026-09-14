---
title: FathomDB 0.8.26 Slice 20 — exact graph artifact evidence
status: DRAFT
---

# Slice 20 plan — exact graph artifact evidence

## Slice-complete workflow

This plan adopts the full [lean slice execution contract](../../slice-execution-contract.md):
enumerate intervening changes and allocations, finalize needs/requirements/AC,
obtain design review, implement RED/GREEN, obtain code review and independent
verification, write status, and clean up temporary workspaces.

## Outcome

A consumer can take a constrained graph target or terminal edge and resolve
its exact immutable artifact revision as evidence under the same frozen read
authority, without probabilistic body re-search.

## Requirements

- **R26-20A:** Expose immutable target revision and terminal-edge revision
  separately through the V1 shape selected by D26-01. If a sidecar is selected,
  pair stable identities with authenticated resolution references. Introduce no
  parallel V2 graph result or method.
- **R26-20B:** Resolve authenticated graph-evidence references under an
  equivalent frozen context. A raw revision ID is identity, not authority and
  is not an independent enumeration surface.
- **R26-20C:** Return intrinsic canonical bytes/span, identities, hash,
  lifecycle, and direct dependency when applicable, without inventing ranked
  contribution or projection origin.
- **R26-20D:** Require frozen graph context and preserve authorization, class-
  aware node/terminal-edge eligibility, nondisclosure, state drift, restart,
  incomplete-provenance atomicity, and cross-binding behavior.

## TDD and delivery

1. Approve the ADR/interface design and exact V1 response shape.
2. Add failing tests for target and edge success plus nonexistent, state-
   drifted, out-of-window, unauthorized, ineligible, incomplete-provenance,
   and mismatched-context refusals.
3. Implement lookup and materialization in one reader transaction.
4. Add property/round-trip and cross-SDK fixtures.
5. Add installed-package probes and public examples.
6. Run focused privacy, concurrency, restart, binding, and package checks plus
   canonical `agent-verify`; reserve the full platform matrix for Slice 50.

## Acceptance

- **AC26-20A:** The selected V1 result or first-generation V1 sidecar returns
  distinct exact target and terminal-edge artifact identities across Rust,
  Python, and TypeScript, with no parallel V2 pair.
- **AC26-20B:** Exact resolution succeeds after unchanged-state restart and
  otherwise preserves nondisclosure for state-drifted, out-of-window,
  mismatched-context, ineligible, revoked, superseded, erased, closure-fenced,
  and nonexistent revisions in one reader transaction.
- **AC26-20C:** Point evidence contains no fabricated rank or contribution; a
  target proves that artifact and a terminal edge makes no full-path claim.
- **AC26-20D:** Interfaces, accepted ADR/addendum, shared fixtures, and bindings agree;
  no logical-ID search predicate is introduced as a workaround.

## Stop gates

Stop on existence disclosure, split-transaction authorization/materialization,
rank synthesis, dynamic-binding incompatibility, or an unexpected schema
migration.
