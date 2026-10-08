---
title: Slice 135 Phase 1 full workspace verification receipt
status: FAILED_PYTHON_SUITE
target_release: 0.8.27
---

# Phase 1 full workspace verification — 2026-10-08

The final `./scripts/agent-verify.sh` run started from committed checkout
`b25bd8a41`. Runtime product trees still matched exact measurement candidate
`224e44c593c13d86ece648adabe445723db04070`. The run used a gate-only
`core.excludesFile` pointing at [gate-excludes.txt](gate-excludes.txt), which
ignored only untracked local Slice 135 result files for the Python test-hook
clean-source precondition. It did not remove those files or change normal Git
status outside this run.

The command exited 1 after 1,132 seconds. Lint, typecheck, and strict security
passed; security reported zero violations, blockers, and downgrades. The test
step ran 186 suites: 185 passed, one failed. The failing `test-python` suite
reported 1,587 passed, 30 skipped, and three failed tests. The exact
[gate output](agent-verify.stdout) and [Python diagnostics](python-test.log)
are retained here. `SHA256SUMS` authenticates those files and the scoped
exclude pattern.

The failed declaration-count test compares a Slice 130 pre-move snapshot to
intentional later changes from `a8a0c74f0`: two changed `search_frozen`
signatures and the `DependencyTraceError` export. No golden file was
regenerated. The two `verify_embed_db` tests failed when their subprocess
could not import `eval`. A focused rerun with `PYTHONPATH` set to this
checkout's `src/python` passed both (`2 passed, 4 deselected`). The focused
rerun is diagnostic and does not change the full gate verdict.
