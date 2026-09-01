---
title: 0.8.25 Slice 6 — independent review of Slice 7 plan
status: PASS
target_release: 0.8.25
observed_on: 2026-09-01
---

# Slice 6 — independent review of Slice 7 plan

## Review posture

The reviewer inspected the plan and repository read-only in the durable
`release/0.8.25` worktree. The reviewer did not edit files, invoke Steward or
Orchestrator, mutate external state, or implement Slice 7.

## Initial verdict — FIX-1

The reviewer found four material plan defects. The accepted proposal scope,
P25-07 boundary, retain-all evidence rule, narrow P25-20 rule, and draft 0.8.26
carry-forward were otherwise complete.

| Finding | Severity | Required correction |
| --- | --- | --- |
| R7-01 | P1 | Name exact new files/symbols and RED/GREEN commands for the wheel verifier, dependency policy, property tests, and traceability checker. |
| R7-02 | P1 | Treat bare `cargo audit` as an unfiltered receipt, then gate while ignoring only the deliberately postponed `paste` advisory; define the expected nonzero `cargo tree -i async-std` absence result. |
| R7-03 | P1 | Do not create an unverifiable 0.8.25 completion/`landed` claim while local commits are ahead of the remote release ref and no push is authorized. |
| R7-04 | P1 | State exact request-versus-response unknown field/variant behavior before promoting architecture v2. |

## FIX-1

The author changed only the plan:

- named checker, fixture, symbol, wiring, and focused command targets;
- separated the unfiltered RustSec receipt from the single-advisory gating
  policy and made `async-std` absence an expected exit-101 proof;
- retained 0.8.25 `COMPLETE_ON_RELEASE_BRANCH`/empty-`landed` truth until a
  separately authorized push makes completion verifiable; and
- defined fail-closed requests, additive response fields, typed unsupported
  response variants, and no older-writer mutation of uninterpretable persisted
  versions.

## FIX-1 re-review — FIX-2

The reviewer confirmed R7-02, R7-03, and R7-04 closed and found one remaining
R7-01 command ambiguity: both `event-listener` 2.x and 5.x are currently
resolved, so unqualified `cargo update -p event-listener` is not executable.

FIX-2 changes only that command to
`cargo update -p event-listener@5.4.1 --precise 5.4.2` when the old 5.4.x
instance remains; otherwise the dependency checker records that the 5.4.x
instance disappeared. No Slice 7 implementation or further FIX cycle has
occurred. The same reviewer must re-review FIX-2 before the plan returns to the
owner.

## FIX-2 re-review — PASS

The same independent reviewer confirmed the version-qualified
`event-listener@5.4.1` selector is executable and the disappearance alternative
is correctly bounded. R7-01 through R7-04 are closed. The accepted P25 scope,
P25-07 byte-unchanged oracle and stop condition, retain-all P25-17 constraint,
narrow P25-20 constraint, Slice 10+ exclusion, no-push/publication boundary,
and all six 0.8.26 carry-forwards remain intact.

This completes the independent review after the maximum two documented FIX-n
cycles. The remaining Slice 6 action is the owner's final approval or
correction of the reviewed Slice 7 plan; review PASS is not implementation
authorization.
