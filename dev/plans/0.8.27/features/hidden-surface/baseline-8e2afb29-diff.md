---
title: FathomDB 0.8.27 hidden-surface successor baseline 8e2afb29 - reviewed diff
status: COMPLETE
target_release: 0.8.27
previous_baseline: baseline-e3358800.json
---

# Successor baseline `8e2afb29` - reviewed diff

## Baseline identity

- File: `baseline-8e2afb29.json`.
- Capture source: `8e2afb293b79bb3922e7d5099096794ecae6f42e`, the
  implementation closeout `HEAD`.
- sha256: `a7ab93dc5f5ae92c3f708fc0b8954d3680b210e8203c6b6640ed50d6d9986812`.
- Captured with `python3 dev/tools/hidden_surface.py capture --source-sha
  8e2afb293b79bb3922e7d5099096794ecae6f42e` in 380 s.

## Comparison with `baseline-e3358800.json`

`compare` exits 1. `metadata_diffs` is empty, and the `release-probe` row is
equal (41 entries). Every difference below comes from the Slice 40 review fixes
that landed after `e3358800`. Each one widens a gate to add the `test` cfg,
narrows a test file to where it compiles, or repairs a test build. None moves
an item into or out of a default or release build.

| Row(s) | Entry | Before | After | Reason |
| --- | --- | --- | --- | --- |
| all 6 engine rustdoc rows | `Engine::execute_for_test` | `any(debug_assertions, feature = "test-hooks")` | `any(debug_assertions, feature = "test-hooks", test)` | `eecedfdf`: the crate's own `--release --tests` lib-test build (`test` true, `debug_assertions` false) must see the seam. Absent from shipped builds; the release probe still reports it `unresolved`. |
| all 6 engine rustdoc rows | `Engine::pause_reader_after_wal_snapshot_for_test` | same | same with `test` | Same reason as above (`eecedfdf`). |
| `test-targets` | `fathomdb-engine::reader_pool`, `slice21c_vector_role_gate`, `slice35_after_validation_races`, `tc91_projection_commit_hardening` | ungated | `debug_assertions` | `eecedfdf`: these files use debug-only seams, so they are gated to compile in release test builds. Dev builds, and so the workspace gate, still run them. |
| `test-targets` | `fathomdb-engine::slice40_projection_completion`, `slice40_projection_generation_races` | `feature = "test-hooks"` | `all(debug_assertions, feature = "test-hooks")` | `f3f9d72c` (C-4a): release test builds with `test-hooks` broke on debug-only seams. The feature-complete gate still runs them (dev profile). |
| 9 inventory rows (every engine feature set without `operator`) | `lifecycle_reliability` | `test-build: failed` | 6 tests (3 run, 3 ignored) | `eecedfdf` and `a80d5115`: at `e3358800` the target called operator-only `rebuild_projections` unconditionally and did not compile without `operator`. It now gates that path, so its tests build in every row. |

No test was removed, and no test became newly ignored.

The engine unit-test inventory is unchanged. The `#[cfg(debug_assertions)]`
added to three lib tests (`9d7d9797`) does not alter a dev-profile listing.
The private-helper and `pub(crate)` → private changes in `lib.rs`,
`lifecycle.rs`, and `temporal.rs` are not on any public path, so they do not
appear.
