---
title: FathomDB 0.8.28 Slice 50 — plan
status: design investigation; built-in-provider replacement proposed; no implementation authorized
target_release: 0.8.28
observed_on: 2026-11-01
---

# Slice 50 plan (`DRAIN-SCAN-BOUND`)

> **Current direction (2026-11-01, scope correction `seq-357`):** close
> production provider escape paths and propose typed failed open on actual
> verification refusal. Preserve existing TC-68 persistent cache, probe cadence,
> gate, residual, mean and runtime behavior. The broader evidence-cache/runtime
> proposal is superseded. Read [requirements/AC](built-in-provider-requirements.md),
> [design/inventory](built-in-provider-design.md) and [plan](built-in-provider-plan.md).
> Unit S is unchanged. Unit D / D50-1 / D50-2 remain historical unselected
> options, held without implementation. Separate independent review and owner
> contract/release-placement approval are required before production changes.

Branch `slice/0.8.28-50-drain-scan-bound`, cut from `dev/0.8.28` at
`544bf74f8`; worktree `../fathomdb-worktrees/slice-0.8.28-50`.

- **Unit S** bounds each `UNION ALL` arm of the scheduler scan and the
  candidate-page query in `projection_worker.rs`
  ([design.md](design.md) § 1).
- **Unit D** stops `Engine::drain` waiting for vector work that a
  dense-disabled session cannot do (todo 289; design § 2). With owner
  approval (D50-2), it also covers the internal settles.

Requirements and AC: [requirements.md](requirements.md). Runs:
[test-plan.md](test-plan.md). Progress: [status.md](status.md).

## Steps

| Step | Who | What | Done when |
| --- | --- | --- | --- |
| P1 | implementer | Phase 1: requirements, design, test plan, this plan, status; release-state `design_refs` | Committed; `./scripts/check-release-state-views.sh` and `./scripts/agent-lint-md.sh` pass |
| R-DES | Fable reviewer (orchestrator runs it) | Design review of requirements, AC, design and test plan | Verdict recorded in `review.md`; findings applied |
| O | owner | Rulings D50-1 (design § 2.3) and D50-2 (design § 2.4) | Recorded in `status.md` and the design |
| H | implementer | H-RED, then H-GREEN: SQL builders and scan hook (design § 3) | RED fails to compile at its own sha; GREEN passes; lint verb passes |
| S | implementer | S-RED, then S-GREEN (design § 1.2) | AC50-01 and AC50-02 fail at S-RED and pass at S-GREEN; AC50-03 to AC50-05 pass at both; AC50-04 mutations recorded |
| T | implementer | Tool commit: scaling probe, driver, analysis and README (test-plan § 4.1) | Tool unit tests pass; the probe builds at base and head |
| D | implementer | D-RED, then D-GREEN; then D2 pairs if D50-2 approves | Each RED fails at its own sha for the stated reason (timing evidence in the body); GREEN passes |
| DOC | implementer | Interface-doc refused-drain passages, CHANGELOG, status, release state | `./scripts/agent-lint-md.sh` passes |
| R-CODE | codex (orchestrator runs it) | Code review of the slice branch | APPROVE, or findings fixed and re-reviewed |
| V | implementer | Scaling run (test-plan § 4), suites (§ 3.5), full gate (§ 5), `AGENT_LONG` perf gates once | AC50-06 to AC50-08, AC50-12 to AC50-14 recorded |
| L | orchestrator | Land on `dev/0.8.28`; update the release state and the board with the full landing sha | Landed sha recorded |

## Rules

- **TDD.** One RED commit per behaviour, failing at its own sha for the
  stated reason, with "RED evidence:" in its body; then its GREEN. No
  coverage commits that pass on arrival. The equivalence guards land
  inside S-RED, and the commit body says why they pass there
  (test-plan § 2). RED shas are audited in `git archive` exports.
- **Tests stay fixed.** Existing tests are not edited. The Slice 30
  refused-runtime fixture is copied into the new test file, not changed
  in place. Under D50-1 option B, the Slice 30 assertion at line 207
  would need an owner-approved change.
- **Scope.** No schema change, no env gate, no change to the arm
  predicates or the page loops (REQ50-S3).
- **Measurement tools** are committed (owner rule, 2026-10-29). BGE
  runs only on the RTX 3090 `cuda:0`; this slice's own tests do not
  use BGE (test-plan § 1).
- No push, no tag, no ledger writes from the implementer.
