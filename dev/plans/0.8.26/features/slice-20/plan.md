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
  separately through an additive sidecar or successor result, preserving V1.
- **R26-20B:** Add a frozen point-evidence lookup by artifact revision.
- **R26-20C:** Return intrinsic canonical bytes/span, identities, hash,
  lifecycle, dependency, and projection origin where applicable, without
  inventing ranked contribution.
- **R26-20D:** Specify and preserve authorization, eligibility, nondisclosure,
  expiry, restart, and cross-binding behavior.

## TDD and delivery

1. Approve the successor ADR/interface design and response-version decision.
2. Add failing tests for target and edge success plus nonexistent, stale,
   expired, unauthorized, ineligible, and mismatched-context refusals.
3. Implement lookup and materialization in one reader transaction.
4. Add property/round-trip and cross-SDK fixtures.
5. Add installed-package probes and public examples.
6. Run focused privacy, concurrency, restart, binding, and package checks plus
   canonical `agent-verify`; reserve the full platform matrix for Slice 50.

## Acceptance

- **AC26-20A:** An additive sidecar or successor result returns distinct exact
  target and terminal-edge artifact identities across Rust, Python, and
  TypeScript without changing V1 incompatibly.
- **AC26-20B:** Exact resolution succeeds after restart and otherwise preserves
  nondisclosure for expired, mismatched-context, ineligible, revoked,
  superseded, and nonexistent revisions in one reader transaction.
- **AC26-20C:** Point evidence contains no fabricated rank or contribution; a
  target proves that artifact and a terminal edge makes no full-path claim.
- **AC26-20D:** Interfaces, successor ADR, shared fixtures, and bindings agree;
  no logical-ID search predicate is introduced as a workaround.

## Stop gates

Stop on existence disclosure, split-transaction authorization/materialization,
rank synthesis, dynamic-binding incompatibility, or an unexpected schema
migration.
