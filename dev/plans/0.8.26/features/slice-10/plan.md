---
title: FathomDB 0.8.26 Slice 10 — frozen evidence contract repair
status: DRAFT
---

# Slice 10 plan — frozen evidence contract repair

## Slice-complete workflow

This plan adopts the full [lean slice execution contract](../../slice-execution-contract.md):
enumerate intervening changes and allocations, finalize needs/requirements/AC,
obtain design review, implement RED/GREEN, obtain code review and independent
verification, write status, and clean up temporary workspaces.

## Outcome

Explanation-enabled frozen retrieval returns a valid non-empty correlation ID,
the public guide explains the frozen/evidence APIs as one coherent workflow,
and a clean installed artifact proves the supported one-source Memex profile.

## Requirements

- **R26-10A:** Repair `search_frozen(explain=True)` and
  `search_with_evidence(include_explanation=True)` through the shared
  completion/finalization boundary.
- **R26-10B:** Preserve frozen eligibility, ranking, evidence references, and
  explanation-disabled behavior.
- **R26-10C:** Document
  `freeze_read_context → search_with_evidence → resolve_evidence` as
  the evidence-exposing route and `search_frozen` as a ranked diagnostic or
  non-exposing route.
- Document expiry, refusal, pagination, drift, correlation, and unsafe
  double-search/retry behavior.
- **R26-10D:** Verify the public contract through a clean installed-wheel
  black-box witness over actuation/replay/restart, frozen
  explained evidence, exact resolution, pagination, dependency lookup, and
  projection readiness.

## TDD and delivery

1. Add failing regressions for both frozen explanation methods and an
   explanation-disabled control.
2. Repair the smallest shared completion boundary.
3. Add equivalence assertions showing no rank/hit/evidence drift.
4. Write and execute the public recipe against built artifacts.
5. Add the black-box one-source profile without importing Memex or asserting
   Memex semantic policy.
6. Run focused, binding, documentation, and package checks, then the canonical
   `agent-verify` gate. Reserve broader platform release proof for Slice 50.
7. Obtain independent review before closing.

## Acceptance

- **AC26-10A:** Both explanation-enabled frozen methods validate with telemetry
  enabled and disabled and return valid, non-empty correlation identity.
- **AC26-10B:** Normalized explanation-on/off results preserve ordered hits,
  scores, identities, projections, and evidence sidecars.
- **AC26-10C:** API reference and executable recipe cover freeze, evidence
  search, resolution, expiry, and drift.
- **AC26-10D:** A freshly built wheel black-box probe passes without importing
  worktree sources; no fixed-version claim precedes that result.

## Stop gates

Stop if the repair changes ranking or eligibility, weakens frozen authority,
requires a breaking API change, or can pass only from a source checkout.
