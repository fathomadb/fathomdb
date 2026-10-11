---
title: Slice 135 Python S01 runner verification
status: FULL_GATE_FAILED_FOCUSED_CHECKS_PASSED
target_release: 0.8.27
---

# Python S01 runner verification

The [S01 runner](../../../../../../../scripts/slice135_python_s01.py) was added
test-first. Its focused test failed at collection before the runner existed,
then passed after implementation: three tests cover stable corpus shape and
negative guards for wrong text IDs, absent vector branch, unknown IDs and an
empty hybrid result. The final [focused pytest](slice135-s01-focused-pytest.log),
[Ruff check](slice135-s01-ruff-check.log),
[Ruff format check](slice135-s01-ruff-format.log), and
`./scripts/agent-lint-md.sh` passed. The raw installed-wheel workload and
deliberately wrong wheel-hash refusal are in the
[baseline S01 probe](../2026-10-07-python-s01-baseline-probe/README.md).

The first `./scripts/agent-verify.sh` attempt was interrupted with exit 130
after the worktree's generated `target/debug` grew to about 21 GB and free
disk fell to 8.5 GB while Rust test compilation was still underway. Its
[partial log](default-interrupted.log) is retained. Only this worktree's
generated `target/debug` was then removed.

The complete rerun used the same `./scripts/agent-verify.sh` gate and tests
with `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_TEST_DEBUG=0` and
`CARGO_PROFILE_DEV_DEBUG=0` to reduce generated artifact size. It completed
with exit 1: 184 registered suites, 182 ran, 179 passed, three failed, two
skipped. Rust workspace tests passed; the TypeScript suite was skipped because
the checkout has no installed TypeScript dependencies. See the exact
[full gate log](compact-full-gate.log) and [hash manifest](SHA256SUMS).

The three failures were:

- [`test-steward-orient`](steward-orient.log): the live checkout's 4354 byte
  briefing exceeded the test's 4096 byte runtime cap. This also failed before
  the S01 runner change.
- [`test-python`](python-suite.log): test-hook setup refused to build a
  candidate artifact from the dirty worktree, so the Python suite did not
  collect. This is a verification setup failure, not a Python test pass.
- [`test-python-native-receipt`](python-native-receipt.log): the existing
  checkout's native receipt nonce mismatched the shared virtual environment.

The complete gate is **not green**. The S01 runner's focused checks and
installed-wheel probe do not substitute for those gates. TypeScript dependency
setup and rerunning the affected release gates remain for the Phase 1
checkpoint.

The subsequent [clean-worktree Python rerun](../2026-10-07-python-suite-clean/README.md)
verified a fresh native receipt and narrowed the Python suite to one frozen
declaration-pin failure; the full gate above was not rerun after that change
of test environment.
