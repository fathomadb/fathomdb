---
title: FathomDB 0.8.26 Slice 10 — frozen evidence contract repair
status: ACTIVE
execution_authorized: repository owner 2026-09-13
---

# Slice 10 plan — frozen evidence contract repair

## Slice-complete workflow

This plan adopts the full [lean slice execution contract](../../slice-execution-contract.md):
reconcile intervening changes and allocations, finalize release-local needs,
requirements, and acceptance criteria, obtain design review, implement with
committed RED/GREEN evidence, obtain code review and independent verification,
write status, and clean up temporary workspaces.

## Reconciliation since the draft

1. Slice 9 landed at `f1de5f6e`, repaired release authority, and made Slice 10
   the next unblocked slice. It changed no Slice 10 product behavior.
2. The reported 0.8.25 defect remains present: `Engine::search_frozen` and
   `Engine::search_with_evidence` return explained results without calling the
   ordinary-search observability finalizer, leaving `correlation_id` empty and
   causing strict Python/TypeScript mapping failure.
3. Existing Slice 50 tests use explanation data internally but do not assert a
   non-empty correlation identity. Slice 55 pins that identity only for
   ordinary explained search.
4. Frozen authority has no wall-clock expiry or retained-snapshot lease. It is
   restart-portable while bound state remains available and unchanged; the
   draft's “expiry” wording is rejected in favor of authentication,
   state-unavailable, and state-drift semantics.
5. Opaque evidence references contain fresh randomness. Equivalence therefore
   compares positional artifact identities and resolved evidence semantics,
   not reference bytes across independent calls.
6. `scripts/verify-release-python-wheel.sh` already supplies a clean,
   non-editable package boundary. Extend it with a defect-specific profile
   rather than creating another package harness.
7. D26-07 assigns the combined P0–P2 artifact profile to Slice 50. Slice 10
   seeds only the minimal one-source prerequisite and proves the repaired
   frozen explanation/evidence workflow. Slice 20 retains graph evidence lookup.

The result is intentionally narrow: one Engine completion defect,
cross-binding regressions, maintained interface/public guidance, and one clean
installed-wheel witness. No schema, public method, wire shape, ranking,
evidence-token, actuation, or persisted-format change is needed.

## Accepted release-local need, requirements, and acceptance

**N26-01:** A frozen evidence query can return validated query-level and
per-hit explanation, including a non-empty FathomDB correlation identity,
without weakening its frozen authority or changing its ranked/evidence result.
This release-local need refines existing global NEED-006, NEED-012, and
NEED-024; those stable global statements require no edit. The locked global
`dev/acceptance.md` is not changed.

- **R26-10A:** `search_frozen(explain=True)` and
  `search_with_evidence(include_explanation=True)` finalize one valid,
  non-empty correlation identity through the existing observability finalizer.
- **R26-10B:** Explanation enablement changes only requested observability:
  ordered hits, scores, identities, projection cursor/fallback, positional
  evidence identity, frozen authority, and explanation-disabled telemetry
  behavior stay unchanged.
- **R26-10C:** Maintained Rust/Python/TypeScript interfaces and public guidance
  document the canonical evidence workflow, diagnostic/non-evidence role of
  `search_frozen`, frozen refusal/drift semantics, top-K versus pagination,
  equivalent-context resolution, and invalid non-frozen/body-research
  substitutions.
- **R26-10D:** A freshly built, non-editably installed Python wheel proves both
  explained frozen paths, disabled controls, exact one-source evidence
  resolution, current dependency metadata, restart, import provenance, clean
  close, and process exit.

Acceptance criteria:

- **AC26-10A:** Both explained frozen methods return mapper-valid non-empty
  `q...` identities with telemetry enabled and unique `x...` fallback
  identities with telemetry disabled; finalization occurs exactly once.
- **AC26-10B:** Normalized explanation-on/off calls preserve all non-requested
  result fields, positional evidence identities, and resolved evidence
  semantics; explanation-disabled frozen calls emit no new telemetry.
- **AC26-10C:** Interface references and a Python guide cover the complete
  supported workflow, including whole-attempt restart after drift, and
  accurately state there is no elapsed-time expiry contract. The guide's
  canonical flow is mirrored and executed by the source-independent installed-
  wheel profile.
- **AC26-10D:** The clean wheel verifier proves module/native imports come from
  its fresh environment and the focused one-source profile passes without
  importing worktree sources. Slice 50 retains the combined release profile.

## TDD and delivery

1. Commit RED Rust Engine tests for the complete exact-once,
   telemetry-on/off, disabled-control, and normalized-equivalence matrix. Add
   real Python and TypeScript public-call regressions for both affected
   explained methods plus disabled controls, proving their strict mappers
   accept the finalized native response. Do not duplicate Engine telemetry
   internals in every binding. Confirm the RED failure is the empty correlation
   ID.
2. GREEN the smallest Engine change: after successful frozen result assembly,
   call the existing finalizer exactly once only when the returned explanation
   is present. Do not route through ordinary search or add another finalizer.
3. Correct stale explanation-presence comments and update
   `dev/interfaces/{rust,python,typescript,wire}.md`, public API references,
   and a new `docs/guides/frozen-evidence.md`; link it from `mkdocs.yml` and
   `docs/guides/index.md`.
4. Extend the existing wheel verifier with an external executable profile and
   strengthen its fixture test so the profile cannot be skipped. Build and
   install a fresh wheel into an external environment with `PYTHONPATH` unset.
5. Run focused engine/binding tests, docs and wheel-verifier contract tests,
   the real installed-wheel witness, `agent-verify`, and full-workspace
   clippy/check. Do not run the unrelated long performance/platform matrix.
6. Obtain independent code review, allow at most two focused FIX cycles, then
   independent verification. Close state against the reviewed implementation
   commit and advance to Slice 15 without a self-referential SHA.

## Stop gates

Stop if the repair changes ranking or eligibility, weakens frozen authority,
asserts equality of randomized evidence references, introduces telemetry on
explanation-disabled frozen calls, requires a schema/public/wire-version
change, adds graph evidence lookup early, or passes only from source checkout.
