---
title: FathomDB 0.8.26 Slice 20 — exact graph artifact evidence
status: DRAFT
---

# Slice 20 plan — exact graph artifact evidence

## Outcome

A consumer can take a constrained graph target or terminal edge and resolve
its exact immutable artifact revision as evidence under the same frozen read
authority, without probabilistic body re-search.

## Requirements

- Expose immutable target revision and terminal-edge revision separately.
- Add a frozen point-evidence lookup by artifact revision.
- Reuse canonical bytes/span, identities, hash, lifecycle, dependency, and
  projection-origin materialization where applicable.
- Return intrinsic evidence without fabricated ranking contribution.
- Preserve authorization, eligibility, expiry, and non-disclosure behavior.
- Bind Rust, Python, TypeScript, and wire models consistently.

## TDD and delivery

1. Approve the successor ADR/interface design and response-version decision.
2. Add failing tests for target and edge success plus nonexistent, stale,
   expired, unauthorized, ineligible, and mismatched-context refusals.
3. Implement lookup and materialization in one reader transaction.
4. Add property/round-trip and cross-SDK fixtures.
5. Add installed-package probes and public examples.
6. Run focused, privacy, concurrency, restart, binding, package, and full gates.

## Acceptance

- A graph result contains enough immutable identity for exact point evidence.
- Resolution reproduces or fails; it never re-searches, silently refreshes, or
  reveals an ineligible artifact.
- Target and terminal-edge proof are distinct, and edge proof is not described
  as full-path proof.
- No logical-ID search predicate is introduced as a workaround.

## Stop gates

Stop on existence disclosure, split-transaction authorization/materialization,
rank synthesis, dynamic-binding incompatibility, or an unexpected schema
migration.
