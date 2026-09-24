---
title: FathomDB 0.8.27 hidden-surface oracle - implementation status
status: ACTIVE
implemented_on: 2026-09-23
baseline_entry_sha: 0ea86e55
design_revision: 7
---

# Hidden-surface oracle implementation status

Status is ACTIVE, not COMPLETE: ACH-14 fails on one test
(`nomic_smoke`, below), and ACH-5 has not been run because the implementation
closeout `HEAD` depends on how that test is resolved.

## Commits

| Commit | Kind | Content |
| --- | --- | --- |
| `f5bc0875` | RED | First self-tests and fixtures (per-site design). |
| `146ee08d` | GREEN | First tool (per-site signing). |
| `bd549d9c` | fix | Manifests use umask permissions. |
| `dc2aab69` | RED | Revision 7 self-tests and fixtures, plus the coverage-gate self-tests. |
| `00fa36d0` | GREEN | Effective signing, test inventories, `test_targets.py`, gate, coverage check, matrix, allowlist. |
| `b6e155e2` | docs | Cadence text in `plan-0.8.27.md`. |
| `18ccf146` | chore | `baseline-e3358800.json`. |
| `4f65e6b6` | fix | Gate runs Slice 72 live and records the reasoned allowlist entries. |

RED evidence: `dc2aab69` failed with a `signature` key assertion in
`test_walk_hazards` and with `FileNotFoundError` for `scripts/lib/test_targets.py`.
GREEN evidence: both self-test files pass (see Final checks).

## Captures

Every capture used `python3 dev/tools/hidden_surface.py capture` with the
pinned `nightly-2026-04-24` (`rustc 1.97.0-nightly (36ba2c771 2026-04-23)`),
`x86_64-unknown-linux-gnu`. Each capture has 33 rows:

- 8 rustdoc rows;
- `release-probe`;
- `test-targets` (260 targets);
- 8 `tests-<row>` rows;
- 15 `tests-req-*` rows.

| Capture | Source | Output sha256 | Seconds |
| --- | --- | --- | --- |
| X1 (cold cache) | `e3358800378a8e6ad9e1dac24c18990c20c58dc7` | `4923cb16db56f21767ae70267e1dcce88ce6eeaa6eea18d7291658723ec47707` | 373 |
| Y | `5f5c1798a3cffc1416467fd707587954ea75d9c6` | `b20137b6837363e0d1fb85d387ca2f347256b7c9849122e5f8eb2025c55a1172` | 235 |
| X2 → `baseline-e3358800.json` | `e3358800…` | `4923cb16…c47707` | 229 |
| X3, dirty scratch clone | `e3358800…` | `4923cb16…c47707` | 221 |
| Injected (`--source-dir`) | edited export of `e3358800…` | `310b714ab052a0324560d6c97871b6e3bc3ac3a316b22fcdf053713eba96bedb` | 236 |

## Acceptance

### ACH-1: PASS at `e3358800`

The check was run against `baseline-e3358800.json`:

- Rows: exactly the 8 rustdoc rows, `release-probe`, `test-targets`, the 8
  `tests-<row>` rows, and 15 derived `tests-req-*` rows.
- `arm_reader_search_hook_for_test` is present in all 6 engine rows.
- `fathomdb_engine::slice72_test_hooks` is only in `engine-slice72-test-hooks`.
- `Engine::open_with_migrations_for_test` is only in
  `engine-migration-test-hooks`.
- `fathomdb_engine::tc5_benchmark` is only in `engine-tc5-benchmark`.
- `Engine::execute_for_test` is in every engine row, with effective `cfg`
  `any(debug_assertions, feature = "test-hooks")` in all 6. This is the
  expected value at `e3358800`.
- `governed_surface_method_absence_proof` is only in `facade-default`.

The closeout-`HEAD` half of the claim, the predicate
`any(debug_assertions, feature = "test-hooks", test)`, belongs to ACH-5 and has
not been run.

### ACH-10: PASS

The `release-probe` row of the baseline has 41 entries. The facade's
`release_surface_raw_sql_absence_proof` is `resolved`. The 40 engine items are
`unresolved`:

- `Engine::execute_for_test` and the other 21 `Engine::*_for_test` seams gated
  on `debug_assertions`;
- `CacheStatusReply` and its 2 impls;
- `ProjectionWorkerPauseReadyError` and its 9 impls;
- `ProjectionWorkerTransactionPauseForTest` with its 2 impls and 2 methods.

No status contradicted its predicate. A contradiction would fail the capture
with exit 2.

### ACH-2: PASS

The self-tests reject short, unknown, and non-commit SHAs before any non-git
command runs. The heavy checks passed:

- **X-Y-X.** X1 and X2 are byte-identical (`cmp`); the same cache was used
  throughout.
- **Dirty checkout.** A scratch clone at `/tmp/claude-1000/hs-dirty-clone`
  (`b6e155e2`) was prepared with:
  - a staged edit to `CHANGELOG.md`;
  - an unstaged edit to `README.md`;
  - an untracked file;
  - one stash entry (`ach2-clone-stash`).

  Its own tool captured the non-`HEAD` commit `e3358800`. Before and after that
  capture, `git status --porcelain --untracked-files=all`, `git worktree list`,
  `git stash list`, `git for-each-ref`, and `HEAD` were identical (`diff`). The
  `git diff HEAD` sha256 was also unchanged (`689d5847…`). The clone's manifest
  equals the baseline byte for byte.

### ACH-3 (heavy): PASS

X1, X2, and X3 are byte-identical captures of one SHA.

### ACH-6: PASS

**Comparison with `5f5c1798`.** Compare exit 1; `metadata_diffs` is empty. All
8 rustdoc rows, `release-probe`, and `test-targets` are equal. Only inventory
rows differ, which ACH-11 permits. They differ by 3 test renames from Slice 40,
shown in each engine inventory row that builds the test:

| Before (`5f5c1798`) | After (`e3358800`) | Cause |
| --- | --- | --- |
| `error_taxonomy::open_stage_enum_is_exactly_four_members` | `…_five_members` | `caffbca6` made the taxonomy exhaustive (`OpenStage::ProjectionGeneration`) |
| `error_taxonomy::corruption_kind_enum_is_exactly_four_members` | `…_five_members` | `caffbca6` (`CorruptionKind::ProjectionGenerationDrift`) |
| `lib::slice72_test_hooks::contract_fixture_records_actual_forward_overlap` | `lib::test_hooks::slice72_test_hooks::…` | Slice 40 moved the module into `test_hooks.rs` |

No test was removed or newly ignored.

**Defect injection.** The injection was made in an export of `e3358800`
(`hidden_surface.py export`):

- the `test-hooks` gate was removed from the definition of
  `decode_dependency_trace_root_for_test`;
- that name was split out of the gated root `pub use` group into an ungated
  `pub use`.

`capture --source-dir` exited 0. `compare` exited 1:

- `engine-default` has exactly one added entry,
  `fathomdb_engine::decode_dependency_trace_root_for_test` (`function`,
  `"cfg":null,"hidden":false`), with nothing removed or changed.
- The same item is added in `engine-slice72-test-hooks`,
  `engine-migration-test-hooks`, and `engine-tc5-benchmark`.
- It changes from `feature = "test-hooks"` to ungated in `engine-test-hooks`
  and `engine-operator-test-hooks`.
- The metadata differences are `source_modified` (`false` → `true`) and
  `source_tree_sha256`.

Writing that capture to `dev/plans/0.8.27/features/hidden-surface/injected.json`
was refused with exit 2 (`a source_modified capture cannot be written under
…/hidden-surface`). The export was then removed with `discard`.

### ACH-11: PASS for Slice 40

The inventory at `5f5c1798` equals the baseline's inventory apart from the 3
renames above. The baseline also records a real build defect as
`test-build: failed` entries: at `e3358800` the engine target
`lifecycle_reliability` does not compile without `operator` (E0599,
`rebuild_projections`). This affects 9 inventory rows, including
`tests-engine-default`. Later Slice 40 review commits fixed it
(`#[cfg(feature = "operator")]` in the test).

### ACH-13: PASS with one host exclusion

259 of 260 `test-targets` entries appear in at least one inventory row. The
exception is `fathomdb-embedder::aarch64_candle_cpu`, whose file-level
`#![cfg(all(target_arch = "aarch64", target_os = "linux", …))]` compiles to no
tests on x86_64. It is recorded in the allowlist under class
`platform-excluded`, a fourth class this implementation added (see
Deviations).

### ACH-14: FAIL on one test (every other target passes)

The command was `bash scripts/test-feature-complete.sh --scratch <scratch>/fc3`
(193 s, after warm builds):

- Coverage passed and the preflight found both 3090s (indices 0 and 1 in PCI
  bus order; the K620 is index 2 and was not selected).
- `doctor warm-cache` found the embedder cached (0 bytes downloaded).
- It ran 14 feature sets and 53 targets. Every cargo run exited 0: 289 tests
  passed, 0 failed, 7 ignored (all on the allowlist).
- Slice 72 ran live on `GPU-5f9cfc90…` (RTX 3090 index 0). The `basic` and
  `moderate` receipts report outcome `success`.

The one failure is `skip marker '[skip]' from
fathomdb-engine::nomic_smoke::nomic_loads_and_embeds is not allowlisted`.

The test reads its weights from the hard-coded path
`/root/.cache/fathomdb/embedders/nomic-v1.5` and prints `[skip] nomic weights
absent` unless that directory exists. It cannot run as a non-root user, so it
has skipped silently on every gate until now. It was deliberately not
allowlisted, because the skip comes from a test defect, not from design.
Resolving it needs an owner decision: fix the test's weight location, or
allowlist it with that reason.

Earlier runs:

- `fc1` exposed a gate defect: `fathomdb-cli` has two binaries.
- `fc2` exposed 16 unexcused skips or ignores, which were triaged. Slice 72's
  `PENDING_EXTERNAL` was fixed by staging its asset root and giving it one
  visible device. The others were allowlisted with their source reasons.

### ACH-5: NOT RUN

This waits on the ACH-14 decision. Between `e3358800` and the current `HEAD`,
the engine sources differ by known changes that a closeout capture will show:

- the `test` term on `execute_for_test` and
  `pause_reader_after_wal_snapshot_for_test`;
- `#[cfg(debug_assertions)]` on 3 private helpers and 3 tests;
- `pub(crate)` → private in `temporal.rs`;
- the operator gate in `lifecycle_reliability`.

They are to be recorded as `baseline-<HEAD>.json` with `baseline-<HEAD>-diff.md`.

### ACH-4, ACH-8, ACH-9, ACH-12, ACH-15, ACH-7: PASS (fast tier and text)

- ACH-4, ACH-8, and ACH-9 are covered by `scripts/tests/test_hidden_surface.py`.
- ACH-12 and ACH-7 are the cadence and retirement text in
  `dev/plans/plan-0.8.27.md` (`b6e155e2`).
- ACH-15 is covered by `scripts/check-test-target-coverage.py` and
  `scripts/tests/test_test_targets.py`, both in the fast tier.
- `regenerate.sh` reproduced every fixture byte-for-byte when run twice
  (sha256 check).

## Deviations

- **Inventory build command.** Inventories are built with `cargo build --tests
  --keep-going` rather than `cargo test --no-run`. The latter stops at the first
  target that fails to compile, which happened at `e3358800`. Failed targets
  are recorded as `test-build: failed` entries. Inventory builds also set
  `CARGO_PROFILE_{DEV,TEST}_DEBUG=0` and `CARGO_INCREMENTAL=0`, and delete the
  test executables after listing, because disk on this host is limited (about
  50 GB free). All of this is recorded in each row's identity.
- **Matrix scope.** The matrix holds every derived requirement set. The gate
  runs, from each set, only the targets that `cargo test --workspace` does not
  already run on this host. The coverage check counts a target as
  gate-covered only under its own requirement set, matching what the gate
  runs.
- **`platform-excluded` class.** A fourth allowlist class, for whole targets
  whose file-level cfg excludes this host. It is valid only where the cfg has a
  platform atom and the target is uncovered on the host; otherwise it is stale.
- **Slice 72 device.** The gate narrows `CUDA_VISIBLE_DEVICES` to `0` for the
  Slice 72 target, because the Slice 72 design requires exactly one visible
  device. It also stages `FATHOMDB_SLICE72_ASSET_ROOT`.
- **Owner-gated impls.** The release probe names a trait impl through its
  owner when the owner itself is absent in release, because the canonical
  trait paths are private or unstable.

## Final checks (2026-09-23)

| Command | Result |
| --- | --- |
| `python3 scripts/tests/test_hidden_surface.py` | `ok hidden-surface` (17 tests) |
| `python3 scripts/tests/test_test_targets.py` | `ok test-targets` (9 tests) |
| `python3 scripts/check-test-target-coverage.py` | `ok`: 260 targets; 53 run only by the feature-complete gate |
| `bash scripts/agent-lint.sh` | exit 0 |
| `bash scripts/agent-typecheck.sh` | exit 0 |
| `bash scripts/agent-lint-shell.sh` | exit 0 |
| `python3 scripts/tests/test_slice30_surface_comparator.py` | fails on host disk only |

The Slice 30 test failure is environmental. `_prepare_scratch` requires
100 GB free and the host had about 38 GB free. The failing assertion expected
the ownership-marker error and got the capacity error. This unit does not
modify that tool, and the same test passed earlier this session with 96 GB free.
