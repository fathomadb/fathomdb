---
title: 0.8.26 Slice 1 — dependency and pinning sweep
status: DRAFT
target_release: 0.8.26
depends_on: 0
---

# Slice 1 plan

## Slice-complete workflow

This plan adopts the [lean slice execution contract](../../slice-execution-contract.md).
It is evidence-only: reconcile draft deltas, complete and independently review
the dependency findings, verify them, write status, and mark behavioral
TDD/code review not applicable unless scope changes.

## Outcome

Produce a complete, current dependency/advisory/pin register and an ordered
response proposal without changing manifests, locks, workflows, sources, or
installed environments.

## Work

1. Query open Dependabot PRs and alerts; note that PR creation is currently
   paused for all five configured ecosystems.
2. Run current registry-backed Cargo audit/outdated, Python metadata, root npm,
   TypeScript npm, and GitHub Actions SHA/version checks in isolated writable
   caches.
3. Enumerate every required/candidate upgrade with current/candidate version,
   direct/transitive role, motivation, artifacts, and affected platforms.
4. Trace every exact pin, Git revision, prerelease, patch/replace, and upper
   bound to its ADR, compatibility test, packaging constraint, or incident.
5. Classify update-now, retain, investigate, or postpone; design separate
   verification for each accepted candidate.

## Acceptance

- All tracked dependency sources are covered.
- Live results are dated and reproducible; cache/egress failures are not green.
- No pin is labeled stale without its compatibility rationale.
- Coupled native stacks are not split into blind lockfile changes.
- No dependency or environment mutation occurs.
