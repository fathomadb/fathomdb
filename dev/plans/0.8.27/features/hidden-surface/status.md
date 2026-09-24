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
| `05d3099c` | docs | `fc8` recorded; `fc7` claims corrected. |
| `375d4816` | RED | Code review FIX-3 contracts: child scopes, injected `plan_runs` listings, `--write-matrix` keeps `[[extra]]`, the live-only skip contract. |
| `c894ca97` | fix | FIX-3 W-1 to W-3 and ledger item 252: child scopes, `list_tests` injection, `FATHOMDB_REQUIRE_LIVE=1` with per-crate `require_live_or_skip`, opt-in tests excluded, markers excused only as `benign-message`. |
| `584eced5`, `544e3e40` | docs | `fc9` recorded; ledger item TC-a0fb71fa closed. |
| `abb0c1d6` | fix | FIX-4 X-1: `candle_reranker` test modules moved after the loader items (clippy `items_after_test_module`). |
| `efad28e5` | fix | FIX-4 X-2: deprecated `httpmock` `assert_hits` replaced by `assert_calls` in the loader tests. |
| `f7b847fb` | test | FIX-4 X-3: `harness_skips_unavailable_backends_cleanly` fails its ONNX skips under `FATHOMDB_REQUIRE_LIVE=1`. |
| `c202c27c` | test | FIX-4 X-4: child-scope cases (crash then next binary, nested child, scopeless child, prose `running` line). |
| `36941495` | fix | FIX-4 X-5, X-6: `scan_output` documents its fail-closed limits; the microbench exclusion states its true basis. |

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

### ACH-14: PASS (at `36941495`, run `fc10`)

**Code review FIX-4.**

- **X-1.** `cargo clippy -p fathomdb-embedder --features default-reranker
  --all-targets -- -D warnings` failed with `items_after_test_module`. The
  `tests` and `gpu_tests` modules of `candle_reranker.rs` now sit at the end of
  the file (a move only, same lines); the command is clean.
- **X-2.** The loader tests used the deprecated `httpmock::Mock::assert_hits`,
  which in httpmock 0.8.3 only calls `assert_calls`. The four sites use
  `assert_calls(0)`; the 12 loader tests pass. Literal `--all-features` cannot
  build on Linux (the Candle Metal backends need `objc2`, and `cudarc` needs a
  CUDA toolkit), so the reproduction is `cargo clippy -p fathomdb-embedder
  --features default-embedder,default-reranker,onnx-embedder,loader-test-hooks,tc5-benchmark
  --all-targets -- -D warnings`, which is now clean.
- **X-3.** `harness_skips_unavailable_backends_cleanly` accepted an unset ONNX
  environment and an ONNX construction skip (`effective == "n/a"`) even under
  `FATHOMDB_REQUIRE_LIVE=1`. Both now go through `require_live_or_skip`.
  Before the change, all five scenarios below passed. After it: the default
  run passes (printing the skip); `FATHOMDB_REQUIRE_LIVE=1` with the ONNX
  environment unset panics; `FATHOMDB_REQUIRE_LIVE=1` with the gate's ONNX
  environment passes; with a missing model file, the construction skip panics
  under `FATHOMDB_REQUIRE_LIVE=1` and passes without it.
- **X-4.** Four `test_child_scopes` cases now cover: a child that crashes
  before its result line, followed by the next binary; a nested child; a
  scopeless child; and a test whose output merely begins with `running 1
  test`. Verifier mutations M3 (no depth reset on `Running`), M6
  (first-status-wins), M12 (`_CHILD_RUN` without `$`), and M15 (`depth = 1`)
  each now fail `test_test_targets.py`. All 16 verifier mutations are killed.
- **X-5.** `scan_output` documents that its child scoping fails closed. A
  planned test that prints a bare `running N test(s)` line, or a child that
  crashes without a result line, is reported as `no result` or `did not run`,
  never as a pass.
- **X-6.** The `pr9_microbench_watchdog_overhead` exclusion now states its true
  basis: the test is an assertion-free timing diagnostic. The gate could
  provision its inputs, but running it would measure nothing.

`fc10`, `bash scripts/test-feature-complete.sh --scratch <scratch>/fc10`,
exited 0. It covered 20 runs and 345 planned tests: 337 passed, 0 failed, and
8 ignored. There were 0 failures and 0 skip markers. Every per-run count
matches `fc9`, and `harness_skips_unavailable_backends_cleanly` ran in
`fathomdb-embedder[default-embedder,onnx-embedder]` (15/15). After the gate,
`git status` was clean.

One non-gate event: a standalone unfiltered `cross_backend_calibration` run
done for X-3 also ran the excluded record writer
`calibration_reports_p1_flips_and_p2_l2`. That writer rewrote
`dev/plans/runs/0.8.18-slice-0-cross-backend-calibration.md` 4 minutes
before `fc10` started, and the file was restored from git. The gate itself
never runs the writer.

### ACH-14: earlier run (at `c894ca97`, run `fc9`)

**Live-test hardening (FIX-3, ledger item 252).** The gate now runs every
test with `FATHOMDB_REQUIRE_LIVE=1`. Every self-skip in the workspace was
classified:

- **Provisioned prerequisite (31 sites):** reranker and nomic weights, the
  ONNX Runtime assets, network access (`FATHOMDB_SKIP_NETWORK_TESTS`), and the
  Slice 72 runner environment. Each calls its crate's test-only
  `require_live_or_skip`, which panics under `FATHOMDB_REQUIRE_LIVE=1` and
  otherwise prints the same message as before.
- **Opt-in:** `IRC_RUN`, `AGENT_LONG`, `EU_DUMP`, `SLICE6_EXPERIMENT`, the
  Slice 72 stress switch, the Jetson-only witness, and gitignored gold and
  corpus files. These keep their skips. The 10 gate-run ones are now
  `opt-in-experiment` entries with `exclude = true`, and the gate never runs
  them. The two whole-target entries (`ir_c_fusion_experiment`,
  `ir_c_recall_run`) became single-test entries.

A skip marker is now excused only by a `benign-message` entry (there are
none), and an ignored test only by `ignored-by-design`. With the nomic cache
empty, `nomic_loads_and_embeds` prints `[skip] nomic weights absent` and passes
by default, and fails with `FATHOMDB_REQUIRE_LIVE=1 and a live prerequisite is
missing` when the variable is set.

**Child scopes (FIX-3 W-1).** Child runs of `calibration_leg_worker` print
their `test calibration_leg_worker ... ok` lines before the worker's own
top-level line, so the earlier first-status-wins rule could count a failed
top-level worker as passed. The scanner now treats a `running N test(s)` line
printed while a planned test is running as a child scope, closed by the
child's `test result:` line, and records nothing from inside it.

`fc9`, `bash scripts/test-feature-complete.sh --scratch <scratch>/fc9`, exited
0:

- 20 runs, 345 planned tests, counted per planned test from `summary.json`:
  337 passed, 0 failed, and 8 ignored (all `ignored-by-design`). There were
  0 failures, 0 skip markers in any run, and no extra-set drift.
- Per run (planned / passed / ignored): `fathomdb[none]` 3/3/0;
  `fathomdb-cli[default-embedder]` 1/1/0; `fathomdb-cli[default-reranker]`
  1/1/0; `fathomdb-embedder[default-embedder]` 6/6/0;
  `fathomdb-embedder[default-reranker]` 5/5/0; `fathomdb-embedder[embed-cuda]`
  2/2/0; `fathomdb-embedder[loader-test-hooks]` 22/22/0;
  `fathomdb-embedder[rerank-cuda]` 2/2/0; `fathomdb-embedder[tc5-benchmark]`
  2/2/0; `fathomdb-embedder[default-embedder,onnx-embedder]` 15/15/0;
  `fathomdb-engine[default-embedder]` 9/8/1;
  `fathomdb-engine[default-reranker]` 19/19/0;
  `fathomdb-engine[migration-test-hooks]` 6/5/1;
  `fathomdb-engine[slice72-gpu-tests]` 19/17/2;
  `fathomdb-engine[tc5-benchmark]` 4/4/0; `fathomdb-engine[test-hooks]`
  130/127/3; `fathomdb-engine[default-embedder,operator]` 1/0/1;
  `fathomdb-engine[operator,test-hooks]` 82/82/0;
  `fathomdb-engine[migration-test-hooks,operator,test-hooks]` 3/3/0;
  `fathomdb-tc5-benchmark[tc5-benchmark]` 13/13/0.
- `fc8` planned 355 tests; the 10 fewer are the excluded opt-in tests, which
  in `fc8` ran and printed their skip markers.
- Every gate-run test with a provisioned-prerequisite site ran and passed:
  `ort_bge_embeds_384_dim_finite_deterministic_vector`, the 5
  `candle_reranker::tests` and 2 `candle_reranker::gpu_tests` tests,
  `candle_onnx_equivalence_measurement`, `cpu_legs_reproduce_0816_baseline`,
  `calibration_cpu_baseline_components_hold`,
  `rerank_passages_threads_alpha_and_pool_n`, the 6 CE-dependent
  `pr_g10_reranker_ce` tests, `nomic_loads_and_embeds`, the 4 network-gated
  `eu5b_lockflip` tests, `cli_doctor_warm_cache_succeeds`, and the Slice 72
  `basic` and `moderate` runs on RTX 3090 index 0.
- After the gate, `git status` showed only this unit's uncommitted doc edits;
  no committed record was rewritten.

### ACH-14: earlier run (at `fe947288`, run `fc8`)

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
- **ONNX export pins.** The versions that first exported the cached model are
  unrecoverable: the venv that built it no longer exists. The exact pins in
  `dev/tools/onnx/export-requirements.txt` (torch 2.4.1+cpu, transformers
  4.44.2, numpy 1.26.4, onnx 1.23.0, and every transitive dependency, each
  with its sha256) are the canonical pins. They were verified genuine and
  complete, and a re-export through the gate's own path into a scratch
  directory reproduced the pinned model sha256 `c92689ec…` byte for byte,
  leaving no `model.onnx.onnx` behind.
- **Opt-in tests excluded, not excused.** An `opt-in-experiment` entry must now
  set `exclude = true` for a single test; the coverage check rejects any other
  form. The embedder's unit tests include `tests/support/live.rs` by path,
  because the helper must stay test-only and the crate has no test support
  module in `src`.
- **Owner-gated impls.** The release probe names a trait impl through its
  owner when the owner itself is absent in release, because the canonical
  trait paths are private or unstable.

## Final checks (2026-09-23, at `36941495` plus this record)

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
| `cargo clippy -p fathomdb-embedder --features default-reranker --all-targets -- -D warnings` | exit 0 |
| `cargo clippy -p fathomdb-embedder --features default-embedder,default-reranker,onnx-embedder,loader-test-hooks,tc5-benchmark --all-targets -- -D warnings` | exit 0 |
| `cargo test -p fathomdb-embedder` with the same features, `--test loader` | 12 passed |
| `cargo test -p fathomdb-embedder --features default-embedder,onnx-embedder --test cross_backend_calibration`, gate ONNX env, `FATHOMDB_REQUIRE_LIVE=1` | 5 passed |
| `bash scripts/test-feature-complete.sh --scratch <scratch>/fc10` | exit 0; 345 planned, 337 passed, 0 failed, 8 ignored |

The Slice 30 test previously failed only on its 100 GB free-space guard. Code
review H-4 (`df9cbf9f`) pins `disk_usage` in that test's scratch-ownership
checks, so it no longer depends on host disk. The Slice 30 tool is unchanged.
