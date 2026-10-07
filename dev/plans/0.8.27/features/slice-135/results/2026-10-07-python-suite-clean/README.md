---
title: Slice 135 clean-worktree Python suite and native receipt
status: ONE_KNOWN_DECLARATION_PIN_FAILURE
target_release: 0.8.27
---

# Clean-worktree Python suite and native receipt

After committing the S01 runner at
`2d2ccefef4092b36419b777f0399d2c61735eb3f`, the worktree was clean.
The Python suite ran with `FATHOMDB_TESTS_ALLOW_REBUILD=1`, a fresh nonce,
`FATHOMDB_SKIP_NETWORK_TESTS=1`, and the checkout-owned `.venv`. The native
test-hook wheel rebuilt and its [receipt](slice135-clean-python-test-hooks-receipt.json)
verified against this exact candidate. The first
[suite run](slice135-clean-python-suite.log) produced 1582 passed, 30 skipped,
and three failed: one declaration pin and two child imports of `eval`.

The two child-import cases passed when `PYTHONPATH` was set to this checkout's
absolute `src/python` path; see the [focused check](slice135-python-embed-db-path-check.log).
A second [full Python suite](slice135-python-path-suite.log) with that path,
a fresh [verified native receipt](slice135-python-path-suite-receipt.json),
and the same network-skip setting produced **1587 passed, 27 skipped, one
failed**. This is a narrower Python-suite result, not a full-workspace green
claim. The path correction increased the number of exercised tests; the
source-independent installed-wheel S01 probe remains separately identified.

The sole remaining failure is
`test_all_python_declarations_match_pre_move_baseline`: the current parser
finds 1132 declarations while the frozen Slice 130 pre-move fixture pins
1131. The [exact diff](slice135-python-declaration-diff.json) shows two
`search_frozen` signatures changed from `pool_n: int=0` to
`pool_n: int | None=None` and one added `errors.DependencyTraceError` alias.
The [Slice 132 design review](../../../slice-132/design-review.md) had already
identified both Python surface differences. This result is a stale-fixture
lead, not evidence that the current API is wrong. The test and baseline
fixture were left unchanged; a separately reviewed contract decision is
needed before replacing the frozen oracle.

All retained logs and reports are bound by [SHA256SUMS](SHA256SUMS). The
remaining declaration test and the separate steward-orient gate failure
prevent a full green claim for Slice 135.
