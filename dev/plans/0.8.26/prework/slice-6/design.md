---
title: FathomDB 0.8.26 Slice 6 — build failure evidence model
status: COMPLETE
---

# Slice 6 design — build failure evidence model

## Failure record

Each distinct root cause receives one record:

| Field | Meaning |
| --- | --- |
| evidence | transcript/run/log reference, date, commit, host, and exact command |
| stage | setup, preflight, build, lint, typecheck, test, or aggregate verify |
| symptom | exact diagnostic, preserved without paraphrasing compiler output |
| classification | product, build, script, environment, invocation, docs, sandbox, or transient |
| root-cause confidence | confirmed, probable, possible, or unknown |
| recurrence and impact | count/contexts and delay or blocked capability |
| workaround | what succeeded, including whether it weakened evidence |
| correction | smallest durable change and why it addresses the cause |
| proof | regression test, diagnostic contract, or target rerun |
| placement | Slice 9, reserved post-10 slice, Slice 50, postpone, or reject |

## Correct verification use

The canonical local aggregate is `./scripts/agent-verify.sh`, which runs lint,
typecheck, and test in order and short-circuits with the failing sub-gate. The
review must retain the sub-gate's original diagnostics and full-log pointer.
Focused commands may diagnose a failure but do not replace the final aggregate
gate. If a strict test requires host capability denied by the sandbox, rerun
the unchanged strict gate in an authorized capable environment; do not disable
the assertion or reinterpret the sandbox denial as product success.

Preflight is evaluated separately: a false refusal, missed stale base,
dependency-state error, disk check, or ambiguous diagnostic can prevent a
correct worktree workflow before product verification begins.

## Initial known evidence seed

The planning session itself observed that `agent-verify` reached the existing
`public-doc-truth` gate and failed because it still demanded a current 0.8.23
README statement on a 0.8.25 baseline. Slice 6 must reproduce and classify that
evidence rather than treating this note as a final root-cause ruling.

## Output ordering

Order recommendations by recurrence and release delay avoided, then by
stability risk, then effort. Separate an actionable repository correction from
an environment prerequisite and from a one-off transient event.

## Completed failure register

| ID | Finding and confidence | Smallest correction and proof | Draft placement |
| --- | --- | --- | --- |
| S6-01 | Confirmed: 0.8.26 preflight refuses the 0.8.26 plan because current-release selection still names 0.8.25. This is expected before activation, not a generic preflight defect. | After repairing historical closure, create the canonical 0.8.26 state/board and prove normal release-worktree preflight. Document that draft planning precedes activation. | Slice 9 prerequisite |
| S6-02 | Confirmed/deterministic: public-document truth demands 0.8.23 because 0.8.25 says publication complete while its canonical `published` receipt is null. | Correct the 0.8.25 state truth and make validation reject publication-complete/null-receipt combinations. Do not regress README to 0.8.23. | Slice 9, urgent |
| S6-03 | Confirmed: preflight reports stale local `main` in preference to newer `origin/main`; currently report-only but misleading. | Define the authoritative-main rule and add stale-local, remote-only, and offline-local fixtures. | Slice 9 |
| S6-04 | Confirmed historical, currently mitigated: stale-base worktrees and shared editable Python environments contaminated verification. | Preserve existing ancestry and no-editable-install guards; validate rather than add another mechanism. | Slice 9 verification |
| S6-05 | No defect: aggregate verification correctly short-circuits and preserves the failing subgate; ptrace denial requires the unchanged strict rerun on a capable host. | Keep current behavior and documentation. No generic retry or relaxed gate. | Reject new work |

## Evidence and scoring

| ID | Evidence locator / exact symptom | Recurrence and impact | Stability risk / effort |
| --- | --- | --- | --- |
| S6-01 | `scripts/preflight.sh --worktree /home/coreyt/projects/fathomdb-worktrees/release-0.8.26 --plan dev/plans/plan-0.8.26.md` → `--plan must match state plan dev/plans/plan-0.8.25.md`; selector is `scripts/release-current.py` | certain until activation; blocks normal preflight | low risk / low effort after S6-02 |
| S6-02 | `python3 scripts/check-public-doc-truth.py` and aggregate verifier → `README.md lacks a current published 0.8.23 statement`; `release-state-0.8.25.json` has null `published`; retained log `/tmp/fathomdb-agent-public-doc-truth-11.log` | deterministic on current tip; blocks lint/verify and leaves false lifecycle truth | low-medium risk because historical authority changes / low-medium effort |
| S6-03 | `scripts/preflight.sh` lines 75–79 choose local `main`; local `8b4bc1c6`, remote `a563362d` | recurs in stale clones; presently misleading report, potential future guard hazard | low risk / low effort with fixtures |
| S6-04 | memory `agent-worktree-stale-base-trap.md`; 0.8.25 stale-extension/ENOSPC records under `dev/plans/0.8.25/features/slice-55/` | costly historically; current controls mitigate recurrence | low risk / test-only effort |
| S6-05 | `scripts/agent-verify.sh`; AGENTS §§3–4; 0.8.25 ptrace records | normal use; no delay attributable to script design | no-change risk / no effort |

Risk and effort are provisional inputs for Slice 8, not implementation
estimates. S6-01 is a sequencing prerequisite, not a defect. S6-02 is the one
release-lifecycle root cause also observed downstream as S7-07.

The focused preflight invocation failed with `--plan must match state plan
dev/plans/plan-0.8.25.md`. The public-doc gate independently failed with
`README.md lacks a current published 0.8.23 statement`. These are separate
symptoms of incomplete release lifecycle truth. Historical ENOSPC and stale
native-extension episodes support isolated build roots and disk monitoring,
but do not justify broad automation in this release.

Compiler/job log bodies that were not retained are recorded as unavailable;
no root cause is promoted from missing transcript evidence.
