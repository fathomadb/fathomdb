---
title: 0.8.26 Slice 1 — dependency and pinning sweep
status: COMPLETE
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
2. Run current registry-backed Cargo audit/update, Python metadata, root npm,
   TypeScript npm, standalone tooling, and targeted GitHub Actions SHA/version
   checks in isolated writable caches. Record a failed or unavailable broad
   SBOM/action sweep as a bounded evidence limit, not green.
3. Enumerate every manifest-level or security-relevant candidate with
   current/candidate version, role, motivation, artifacts, and affected
   platforms. A compatible transitive lock refresh may be summarized by count
   and risk rather than copying every resolver line when no item has a security
   or feature driver.
4. Trace every exact pin, Git revision, prerelease, patch/replace, and upper
   bound to its ADR, compatibility test, packaging constraint, or incident.
5. Classify update-now, retain, investigate, or postpone; design separate
   verification for each accepted candidate.

## Acceptance

- All shipped manifest classes and tracked standalone developer tools are
  covered by a live result or an explicit fail-closed limitation requiring
  Slice 8 disposition.
- Live results are dated and reproducible; cache/egress failures are not green.
- No pin is labeled stale without its compatibility rationale.
- Coupled native stacks are not split into blind lockfile changes.
- No dependency or environment mutation occurs.

The delta review approved these proportionality adjustments: an action-wide
latest-version catalog and a 126-line transitive lock proposal would not add
decision value comparable to targeted official-pin and advisory evidence.
