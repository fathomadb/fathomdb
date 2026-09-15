---
title: FathomDB 0.8.26 Slice 40 — independent verification
status: PASS
reviewed_on: 2026-09-15
---

# Slice 40 independent verification

The read-only verification subagent first returned FAIL at `3850c93e`. The
default-parallel fresh-cutover target reported 11 passes and one failure, and
the older-wins test failed when run alone with
`RuntimeConfiguration(TooLate)`. The test created schema 33 through raw SQLite
in the tested process before its first product open, so its evidence depended
on another parallel test configuring the process first.

The remediation at `bfb2132b` moves schema-33 creation and the held product
lock into a coordinated child process. The tested parent performs no SQLite
operation before its first `Engine::open`: it waits for child readiness,
observes `DatabaseLocked`, signals release, reaps the child, then verifies the
typed schema-33 refusal. The child has a bounded ten-second release wait, and
same-process raw fixture helpers explicitly configure the test runtime before
opening SQLite.

The same verifier reran the corrected evidence and returned **PASS**:

- formerly failing older-wins test alone: 1 passed, 0 failed, 12 filtered;
- full fresh-cutover target in default parallel mode: 13 passed, 0 failed;
- child lifecycle: bounded synchronization, normal wait/reap, no lingering
  child;
- `git diff --check`: PASS; worktree clean at the reviewed commit;
- no remaining P1/P2 finding.

The main-thread canonical gate at `7b1dc0a8` had already passed lint,
typecheck, security, and all 110 registered suites. The later change is confined
to process isolation in this focused test target; the verifier reran the exact
affected target rather than repeating the full canonical gate.
