---
title: FathomDB 0.8.27 hidden-surface oracle - implementation status
status: COMPLETE
implemented_on: 2026-09-23
baseline_entry_sha: 0ea86e55
design_revision: 7
---

# Hidden-surface oracle implementation status

Every ACH passed. The successor baseline `baseline-8e2afb29.json` records
the implementation closeout `HEAD`.

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
| `e29efcad`, `70b904a2` | docs | This file (first pass); the Slice 40 review cites ACH-6. |
| `f8f7bd62` | fix | `nomic_smoke` resolves its weights under the embedder cache root (`dirs::cache_dir()`). |
| `44fba58c` | RED | Self-test for pinned-weight provisioning in the gate. |
| `8e2afb29` | GREEN | Gate provisions pinned nomic-embed-text-v1.5 weights. |
| `df9cbf9f` | test | Code review H-4: the Slice 30 scratch-ownership test pins disk usage. |
| `553f47eb` | RED | Code review FIX-1 contracts for the gate and the oracle. |
| `04ef2772` | fix | H-6, H-7: release-only cfgs evaluated, `prune` locked, effective docstring. |
| `88b180bd` | fix | H-1, H-2: bare `SKIP` caught, item-level feature tests gated, ONNX provisioned. |
| `4c00d0f6` | fix | Every gate binary runs with `--no-fail-fast`; newly surfaced skips reasoned. |
| `6e9c2f6e` | docs | Design text for provisioning and the listing contract; ledger entry for live-test hardening. |
| `b3e32500` | fix | The legacy-upgrade test configures the SQLite runtime before its raw connection. |
| `e5d2df13` | docs | `fc7` recorded (its counts are corrected below). |
| `9537bbc6` | RED | Code review FIX-2 contracts: extra feature sets, parent counts, calibration split, failure accounting, release-only scan, export pins. |
| `fe947288` | fix | FIX-2 V-1 to V-6: extra feature sets, per-parent counts, `calibration_cpu_baseline_components_hold`, `run_failures`, item-gated release-only parsing, hash-pinned ONNX export. |

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
| Closeout → `baseline-8e2afb29.json` | `8e2afb293b79bb3922e7d5099096794ecae6f42e` | `a7ab93dc5f5ae92c3f708fc0b8954d3680b210e8203c6b6640ed50d6d9986812` | 380 |

## Acceptance

### ACH-1: PASS at `e3358800` and at closeout `HEAD`

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

The same checks against `baseline-8e2afb29.json` (closeout `HEAD`) all pass.
There, `Engine::execute_for_test` carries
`any(debug_assertions, feature = "test-hooks", test)` in all 6 engine rows.

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

### ACH-14: PASS (at `fe947288`, run `fc8`)

**Corrections.** Two earlier versions of this section made false claims.

- `fc4` was recorded as a pass with "nothing skipped outside the allowlist".
  The skip matcher missed bare `SKIP`, so three ONNX-equivalence tests skipped
  in `fc4` and were counted as passed. Code review FIX-1 fixed the matcher
  (`88b180bd`, `4c00d0f6`).
- `fc7` was recorded as running every test gated by an item-level
  `#[cfg(feature = ...)]`. That was false. The matrix held only target-level
  requirement sets, so tests gated on a feature no target requires never ran.
  Examples are `cli_doctor_warm_cache_succeeds`,
  `doctor_reranker_gpu_cpu_subprocess_has_exact_database_free_contract`, the
  embedder's `cuda_probe_error_tests` and `candle_reranker::gpu_tests`, and
  the `fathomdb-tc5-benchmark` binary's unit tests. The `fc7` counts were also
  wrong: "326 passed" summed `test result` lines, which include the
  `calibration_leg_worker` child runs. Per planned test, `fc7` was 321 passed
  and 8 ignored, 329 in all. The `cross_backend_calibration` binary's 3 tests
  took 118.45 s in total. The 37.0 s figure was one child worker.

FIX-2 (`fe947288`) closed both gaps. The gate now lists each crate under the
union of its host-buildable features and derives an extra set for every test
that no other set lists. The committed matrix records the 6 derived sets as
`[[extra]]`. The gate also counts each planned test from its own status.

The FIX-1 gate had surfaced one genuine failure in `fc5`:
`slice40_projection_generation::upgraded_nonempty_database_bootstraps_as_legacy_degraded`
failed with `RuntimeConfiguration(TooLate)`, because it opened a raw SQLite
connection before configuring the runtime. It was fixed test-first
(`b3e32500`), with the assertions unchanged.

The final run is `fc8`, `bash scripts/test-feature-complete.sh --scratch
<scratch>/fc8`, which exited 0:

- 20 runs, 355 planned tests, counted per planned test: 347 passed, 0 failed,
  and 8 ignored, all on the allowlist. There were 0 contract failures and no
  extra-set drift.
- The preflight found both 3090s (indices 0 and 1 in PCI bus order). The K620
  was not selected.
- Pinned assets were verified from the cache: nomic-embed-text-v1.5, ONNX
  Runtime 1.26.0, and the exported bge-small ONNX graph.
- The tests gated on features that no target requires ran:
  - `fathomdb-cli[default-embedder]`: `cli_doctor_warm_cache_succeeds ... ok`.
  - `fathomdb-cli[default-reranker]`:
    `doctor_reranker_gpu_cpu_subprocess_has_exact_database_free_contract ... ok`.
  - `fathomdb-embedder[embed-cuda]`: both
    `candle_bge::cuda_probe_error_tests` tests passed.
  - `fathomdb-embedder[rerank-cuda]`: `gpu_loads_and_scores_finite` (a
    `cuda:0` logit) and `cpu_gpu_logits_close` (max abs diff 1.43e-6) passed.
  - `fathomdb-tc5-benchmark[tc5-benchmark]`: 13 tests passed.
- `tegra_gpu_allocation_witness_on_real_hardware` compiles under `embed-cuda`
  and printed `SKIP tegra-gpu-allocation-witness: not_opted_in`. It is a
  Jetson-only arm and is allowlisted as `opt-in-experiment`, with its reason.
- The calibration's assertions ran in the new
  `calibration_cpu_baseline_components_hold`, which passed. The writer
  `calibration_reports_p1_flips_and_p2_l2` stays excluded. After the gate,
  `git status` was clean, so the committed calibration record was not
  modified. The `cross_backend_calibration` binary's 4 tests took 160.18 s.
- Slice 72 ran live on RTX 3090 index 0.

History:

- `fc1` exposed a gate defect: `fathomdb-cli` has two binaries.
- `fc2` exposed 16 unexcused skips or ignores, which were triaged. Slice 72's
  `PENDING_EXTERNAL` was fixed by staging its asset root and giving it one
  visible device. The others were allowlisted with their source reasons.
- `fc3` failed only on `nomic_smoke` (RED). It printed `[skip] nomic weights
  absent` because it read the hard-coded path
  `/root/.cache/fathomdb/embedders/nomic-v1.5`. Following the owner's decision,
  the test was fixed rather than allowlisted (`f8f7bd62`): it now resolves the
  embedder loader's cache root, with every assertion unchanged. The gate now
  provisions the weights (`44fba58c` RED, `8e2afb29` GREEN). The embedder crate
  has no nomic fetcher, so the gate downloads revision `e9b67630` and verifies
  the LFS sha256 of `model.safetensors` and the git blob sha1 of
  `tokenizer.json`.

### ACH-5: PASS (successor baseline)

The closeout `HEAD` `8e2afb29` was captured and compared with
`baseline-e3358800.json`: compare exit 1. There are no metadata differences,
and `release-probe` is equal. The differences are the reviewed Slice 40
review-fix changes, each justified in
`baseline-8e2afb29-diff.md`:

- the `test` term on 2 engine seams;
- `debug_assertions` file gates on 6 test targets;
- `lifecycle_reliability` building again in 9 inventory rows.

The capture is committed as the successor `baseline-8e2afb29.json`. At `HEAD`,
259 of 260 targets appear in an inventory row (aarch64 excluded) and there are
no `test-build: failed` entries. The exclusive-create guard is covered by
`test_output_guards`.

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
- **Extra feature sets.** The matrix has a second, gate-derived table,
  `[[extra]]`, for tests behind an item-level feature cfg that no target
  requires. The gate derives it from test listings (a build per candidate
  feature), so only the gate checks that it is complete; the fast-tier coverage
  check validates only its entries. Host-buildable features exclude the Candle
  Metal backends through the table `PLATFORM_DEPENDENCY_FEATURES` in
  `test_targets.py`, because Cargo cannot express that they build only on
  macOS. The gate also covers crates without test targets (`fathomdb-napi`,
  `fathomdb-py`, `fathomdb-tc5-benchmark`) when their features add tests.
- **ONNX export pins.** The venv that built the cached model no longer exists,
  so its versions could not be recovered. The lock
  `dev/tools/onnx/export-requirements.txt` pins the current exact versions
  (torch 2.4.1+cpu, transformers 4.44.2, numpy 1.26.4, onnx 1.23.0, and all
  transitive dependencies) with sha256 hashes. It was checked by exporting
  through the gate's own path into a scratch directory: the result matched the
  pinned `c92689ec…` sha256, and no `model.onnx.onnx` was left behind.
- **Owner-gated impls.** The release probe names a trait impl through its
  owner when the owner itself is absent in release, because the canonical
  trait paths are private or unstable.

## Final checks (2026-09-23, at `fe947288`)

| Command | Result |
| --- | --- |
| `python3 scripts/tests/test_hidden_surface.py` | `ok hidden-surface` |
| `python3 scripts/tests/test_test_targets.py` | `ok test-targets` |
| `python3 scripts/check-test-target-coverage.py` | `ok`: 260 targets; 53 run only by the feature-complete gate |
| `python3 scripts/tests/test_slice30_surface_comparator.py` | `ok slice30-surface-comparator` |
| `bash scripts/agent-lint.sh` | exit 0 |
| `bash scripts/agent-typecheck.sh` | exit 0 |
| `bash scripts/agent-lint-shell.sh` | exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |

The Slice 30 test previously failed only on its 100 GB free-space guard. Code
review H-4 (`df9cbf9f`) pins `disk_usage` in that test's scratch-ownership
checks, so it no longer depends on host disk. The Slice 30 tool is unchanged.
