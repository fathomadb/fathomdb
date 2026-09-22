---
title: FathomDB 0.8.27 prework verification
status: COMPLETE
target_release: 0.8.27
verified_on: 2026-09-21
---

# 0.8.27 prework verification

## Verdict

**PASS.** An independent read-only verification subagent checked the reviewed
planning baseline, activated release state/board, final plan anchors, and
focused repository gates. No unresolved P1-P4 finding remains.

## Resolved finding

The first staged traceability run found one P2: the active plan contained the
required content but lacked five exact role headings consumed by
`commission-manifest.sh`. The plan now has canonical goal/scope, immediate-next,
slice-ladder, reserved-gap, and cross-cutting-DoD headings. The manifest and
staged design-reference gate pass after the correction.

## Passing evidence

- diff/JSON validity; plan and design status; plan anchors; design lifecycle;
  architecture authority; and staged design-reference traceability;
- release-state generated views and `release-current.py`, which selects
  0.8.27;
- normal, linked-worktree landing, and Slice 9 dependency-closure preflights;
- exact 25-slice ladder, dependencies, schema 34, Slices 0-9 planning SHA,
  Slice 10 `READY_UNCOMMISSIONED`, two open decisions, and existing design
  references;
- supporting Markdown lint over 1,221 files with zero findings and 47/47
  focused offline links; and
- no product, dependency, test, CI, package, environment, temporary branch, or
  temporary worktree change.

## Expected red and evidence boundary

`scripts/check-public-doc-truth.py` remains RED only because root `README.md`
still claims published 0.8.25. That known stale truth is allocated to Slice 10
and was neither hidden nor weakened. Canonical `agent-verify` was not run as a
full regression for this planning-only change: it cannot be green until that
truth correction lands, and this checkout does not yet own the exact Markdown
tool dependency. A read-only primary-checkout Markdown run used version 0.23.0
instead of the repository's 0.23.2 pin, so it is supporting evidence only.
