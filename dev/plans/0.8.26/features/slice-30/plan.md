---
title: FathomDB 0.8.26 Slice 30 — distributable operator integrity route
status: DRAFT
---

# Slice 30 plan — distributable operator integrity route

## Slice-complete workflow

This plan adopts the full [lean slice execution contract](../../slice-execution-contract.md):
enumerate intervening changes and allocations, finalize needs/requirements/AC,
obtain design review, implement RED/GREEN, obtain code review and independent
verification, write status, and clean up temporary workspaces.

## Outcome

Memex operators can install and run FathomDB's existing bounded read-only data
plane integrity check as a version-matched external tool, without raw SQL and
without adding doctor or recovery authority to governed SDKs.

## Requirements

- **R26-30A:** Qualify the published crates.io
  `fathomdb doctor data-plane-integrity --json` route for Memex's selected
  deployment target, including named artifact installation/discovery, exact
  CLI identity, compatible database schema, and fail-closed mismatch behavior.
  Add packaging only if that route is unusable.
- **R26-30B:** Specify quiescence, locking, bounds, privacy, JSON schema, and
  exit codes. Guarantee no repair, rebuild, mutation, projection advance, or raw content
  disclosure.
- **R26-30C:** Document the operator workflow and preserve the Python/TypeScript recovery
  denylist.

## TDD and delivery

1. Inventory the existing CLI and release artifact topology; decide the
   smallest distribution change.
2. Add failing artifact-level tests for install/discovery, version mismatch,
   non-quiescent use, malformed/corrupt state, bounds, JSON, exit codes, and
   zero mutation.
3. Implement only the qualification, non-mutation, documentation, or
   conditional packaging delta demonstrated by the RED tests.
4. Add target-specific build/install/run evidence.
5. Verify that governed SDKs expose no doctor/recovery route.
6. Run focused real-database and selected-package checks plus canonical
   `agent-verify`; reserve the broad platform matrix for Slice 50.

## Acceptance

- **AC26-30A:** A clean operator environment can install/discover the named
  exact CLI artifact, invoke and parse it, and inspect a database with the
  selected compatible schema while rejecting incompatible combinations.
- **AC26-30B:** The tool returns stable bounded results and documented exit
  codes; before/after database and projection state prove the whole invocation
  is read-only under its locking contract.
- **AC26-30C:** Memex needs neither raw SQL nor in-process repair privileges,
  and governed SDKs expose no doctor/recovery route.

## Stop gates

Stop if safe operation requires an SDK doctor method, concurrent inspection
cannot be made explicit and safe, packaging creates ambiguous version pairing,
or the route can mutate state.
