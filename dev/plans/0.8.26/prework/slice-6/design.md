---
title: FathomDB 0.8.26 Slice 6 — build failure evidence model
status: DRAFT
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
