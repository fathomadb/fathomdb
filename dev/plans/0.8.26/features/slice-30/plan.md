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

- Distribute `fathomdb doctor data-plane-integrity --json` for the target
  matrix approved in Slice 8.
- Define executable/binding/database version matching and fail closed on an
  incompatible combination.
- Specify quiescence, locking, bounds, privacy, JSON schema, and exit codes.
- Guarantee no repair, rebuild, mutation, projection advance, or raw content
  disclosure.
- Document the operator workflow and preserve the Python/TypeScript recovery
  denylist.

## TDD and delivery

1. Inventory the existing CLI and release artifact topology; decide the
   smallest distribution change.
2. Add failing artifact-level tests for install/discovery, version mismatch,
   non-quiescent use, malformed/corrupt state, bounds, JSON, exit codes, and
   zero mutation.
3. Implement packaging or metadata changes without duplicating engine logic.
4. Add target-specific build/install/run evidence.
5. Verify that governed SDKs expose no doctor/recovery route.
6. Run focused, real-database, package, platform, and full repository gates.

## Acceptance

- A clean operator environment can install, invoke, parse, and version-check
  the integrity tool.
- The tool returns stable machine-readable results and documented exit codes.
- Before/after database and projection state prove the operation is read-only.
- Memex needs neither raw SQL nor in-process repair privileges.

## Stop gates

Stop if safe operation requires an SDK doctor method, concurrent inspection
cannot be made explicit and safe, packaging creates ambiguous version pairing,
or the route can mutate state.
