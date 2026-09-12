---
title: FathomDB 0.8.26 Slice 10 — frozen evidence contract repair
status: DRAFT
---

# Slice 10 plan — frozen evidence contract repair

## Outcome

Explanation-enabled frozen retrieval returns a valid non-empty correlation ID,
the public guide explains the frozen/evidence APIs as one coherent workflow,
and a clean installed artifact proves the supported one-source Memex profile.

## Requirements

- Repair `search_frozen(explain=True)` and
  `search_with_evidence(include_explanation=True)` through the shared
  completion/finalization boundary.
- Preserve frozen eligibility, ranking, evidence references, and
  explanation-disabled behavior.
- Document `freeze_read_context → search_with_evidence → resolve_evidence` as
  the evidence-exposing route and `search_frozen` as a ranked diagnostic or
  non-exposing route.
- Document expiry, refusal, pagination, drift, correlation, and unsafe
  double-search/retry behavior.
- Run a clean installed-artifact witness over actuation/replay/restart, frozen
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
6. Run focused, binding, documentation, package, and full repository gates.
7. Obtain independent review before closing.

## Acceptance

- Both explanation-enabled methods validate and return non-empty correlation
  identity from source and installed artifacts.
- Explanation-disabled results remain valid and ranking/evidence identity is
  unchanged by requesting explanation.
- The published-surface guide is complete and executable.
- No fixed-version claim is made until a freshly built artifact passes.

## Stop gates

Stop if the repair changes ranking or eligibility, weakens frozen authority,
requires a breaking API change, or can pass only from a source checkout.
