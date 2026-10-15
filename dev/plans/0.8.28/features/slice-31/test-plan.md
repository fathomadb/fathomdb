---
title: FathomDB 0.8.28 Slice 31 — test plan (W3)
status: REVISED AFTER R-TEST (APPROVE-WITH-FIXES); ready for W6; revision 2 (2026-10-13)
target_release: 0.8.28
observed_on: 2026-10-13
---

# Slice 31 test plan

This is W3 of [plan.md](plan.md) (revision 19): every FathomDB test of the
slice, traced to an AC31 of [requirements.md](requirements.md), with its
host, exact command and expected result (plan § 4). It expands the test
topics of plan § 7 and the "Tests" part of every decision in
[design.md](design.md) revision 3. R-TEST (Opus, Q19) reviewed revision 1
before the first W6 commit.

**Revision 2** applies R-TEST (Opus, APPROVE-WITH-FIXES, findings 1–17,
the rulings on open points OP-1 to OP-11 and the proportionality cuts,
[review.md](review.md)) and owner rulings Q26–Q28 (steward ledger
seq 329–331). The main changes: no test drives an embed through the CLI,
which has no embed or rerank verb (the CUDA wheel and the embedder's live
test binaries do, § 3.13); the `libcuda.so.1`-only cell moves to the C3
session; D-18 refuses a cuBLAS or cuRAND that lacks a required symbol
(Q26, R12b, R18); the C3-only test is `#[ignore]`; the release-contract
arms assert messages; the Orin runs in a pin window and three windows A,
B and C (§ 4.2); G9 uses `QUAL_QUIET_STRICT=1`; S-13 may use a larger
machine for one build attempt (Q27); G5's equality to Slice 30's hash is
report-only (Q28). § 6 records each open point's ruling.

The forks' own tests are planned, run and reviewed in the forks (plan § 7,
I-18). For a fork-backed criterion this plan names the fork receipt that is
checked and how (§ 3.12); it never re-runs a fork test.

**Names.** `C`, `K`, `K8`, `K9` as in requirements § 1. `<slice base>` is
`git merge-base origin/release/0.8.28 <tip>`. `<tip>` is the slice-branch
commit under test, `<sha12>` its first 12 hex digits. Embedder paths are
under `src/rust/crates/fathomdb-embedder/`. A file or test marked
**new (W6)** does not exist at `c441b927e`; every other file and test name
cited here was checked to exist on 2026-10-13 (`ls`, `grep`).

**Tiers.** *fast*: `./scripts/agent-test.sh --tier=fast` (each suite is a
`run_tier_suite fast …` line). *full*: `./scripts/agent-verify.sh`, which
runs every tier, the Rust workspace tests included, but **without**
`embed-cuda` or `rerank-cuda` (`scripts/test-rust-workspace.sh` builds no
CUDA feature, and those features need `nvcc`). *build-only*: `windchill3`
with § 1.1's build environment, a CUDA-feature build whose tests open no
GPU; the receipt quotes the exact command. *feature-complete*: one run of
`scripts/test-feature-complete.sh` on `windchill3` (cmd `c-fc`), which
runs every test the workspace run cannot, the CUDA-feature sets included,
and catches `scripts/test-skip-allowlist.toml` gaps. *GPU*: `windchill3`,
one RTX 3090 by UUID (§ 1.1). *Orin*: the Jetson AGX Orin 64 GB (§ 1.2).
*GCP*: an owner-approved lab VM session (§ 1.3). *review*: a command the
reviewer runs at R-VERIFY or R-CODE-4; no product code.

**Receipts.** Qualification receipts go to
`dev/plans/0.8.28/features/slice-31/qualification/` as
`<YYYY-MM-DD>-<cell>-<sha12>.md` with the raw `.log` beside it, the fork
convention; `qualification.md` (W6) summarizes them. Harness scripts are a
lint-clean copy of Slice 30's in `dev/plans/runs/0.8.28-slice-31/harness/`
(new (W6)). Every receipt states: commit, host, guard lines before and
after, environment, `Cargo.lock` SHA-256, and each test by name with its
run count ("1 passed"), never only a pass summary (plan § 7
T-POOL-PRODUCT).

## 1. Hosts and guards

### 1.1 `windchill3` (x86_64; RTX 3090 by UUID)

- **Never changed:** NVIDIA driver 580.178.04 and `/usr/local/cuda` →
  `/usr/local/cuda-12.6` (Q4, Q6, Q11). No install, update or removal of a
  driver or toolkit; no CUDA 13.0 toolkit on this host (Q15).
- **The Quadro K620 is never run** (Q22). Every process this plan starts
  on `windchill3` that can reach CUDA sets `CUDA_VISIBLE_DEVICES` to the
  UUID of the RTX 3090 at bus `00000000:41:00.0` (design § 0.1): never
  `all`, never an index. The UUID is recorded in the receipt. The one
  exception is the existing feature-complete gate (cmd `c-fc`), which
  selects `CUDA_DEVICE_ORDER=PCI_BUS_ID CUDA_VISIBLE_DEVICES=0,1`: under
  PCI bus order the K620 at `61:00.0` is index 2 and excluded (R-TEST
  ruling OP-3). Moving that gate to UUIDs is a main-thread todo, not W6
  work.
- **Guard and selection** (`dev/plans/runs/0.8.28-slice-31/harness/wc3-env.sh`,
  new (W6); sourced by every `windchill3` GPU run):

  ```sh
  GUARD_LOG="${GUARD_LOG:?}"
  wc3_guard() {
    driver="$(nvidia-smi --query-gpu=driver_version --format=csv,noheader | sort -u | paste -sd, -)"
    cuda="$(readlink -f /usr/local/cuda)"
    printf '%s %s driver=%s cuda=%s\n' "$(date -u +%FT%TZ)" "$1" "$driver" "$cuda" >>"$GUARD_LOG"
    [ "$driver" = 580.178.04 ] && [ "$cuda" = /usr/local/cuda-12.6 ] || {
      echo "wc3 guard changed: driver=$driver cuda=$cuda; stop the slice" >&2; exit 3; }
  }
  RTX3090_UUID="$(nvidia-smi --query-gpu=uuid,pci.bus_id --format=csv,noheader \
    | awk -F', ' '$2 == "00000000:41:00.0" {print $1}')"
  [[ "$RTX3090_UUID" =~ ^GPU-[0-9a-f-]{36}$ ]] || { echo "no RTX 3090 at 41:00.0" >&2; exit 3; }
  export CUDA_VISIBLE_DEVICES="$RTX3090_UUID" FATHOMDB_CUDA_GPU_UUID="$RTX3090_UUID"
  export CUDA_COMPUTE_CAP=75 CUDARC_CUDA_VERSION=12060 CUDA_PATH=/usr/local/cuda-12.6
  export PATH="/usr/local/cuda-12.6/bin:$PATH"
  ```

  Each run calls `wc3_guard before-<cell>` first and `wc3_guard
  after-<cell>` last; any change stops the slice (plan § 2.2). Builds set
  `CUDA_COMPUTE_CAP=75` so Candle's kernel build never asks `nvidia-smi`
  for a compute capability.
- **GPU lock:** `flock /tmp/fathomdb-gpu.lock` around every GPU process,
  as the Slice 30 harness does.
- **Python:** the slice worktree's own `.venv` from `scripts/bootstrap.sh`
  (W0 step 6) for the gate; scratch venvs under the session scratchpad for
  C2 and wheel smokes. Never `pip install -e` or `maturin develop` against
  any checkout's `.venv`.

### 1.2 Jetson AGX Orin 64 GB (`10.83.10.13`; JetPack 6, CUDA 12.6)

- **Guard** before and after every test process, as the fork cells do:
  `head -1 /etc/nv_tegra_release` (expected `# R36 (release), REVISION:
  5.2, …`) and `readlink -f /usr/local/cuda` (expected
  `/usr/local/cuda-12.6`). Nothing is installed or reflashed (Q5).
- **How code reaches it** (the Candle K-G3 method, receipt
  `2026-10-13-CK-AC-07-182ceab0.md`: "a git bundle of the branch"):

  ```sh
  # windchill3, from the slice worktree; S is the session scratchpad
  git bundle create "$S/slice31-<sha12>.bundle" slice/0.8.28-31-cudarc-fork
  scp "$S/slice31-<sha12>.bundle" coreyt@10.83.10.13:fathomdb-slice31/
  # Orin
  git clone -q ~/fathomdb-slice31/slice31-<sha12>.bundle ~/fathomdb-slice31/src-<sha12>
  git -C ~/fathomdb-slice31/src-<sha12> checkout -q --detach <tip>
  git -C ~/fathomdb-slice31/src-<sha12> rev-parse HEAD   # recorded; must equal <tip>
  ```

  The bundle carries `<slice base>` too, for the T-PERF baseline
  (`src-base`). Each tree builds in its own `CARGO_TARGET_DIR`. Build
  environment: `CUDA_COMPUTE_CAP=87 CUDARC_CUDA_VERSION=12060
  CUDA_PATH=/usr/local/cuda-12.6 PATH=/usr/local/cuda-12.6/bin:$PATH
  LIBRARY_PATH=/usr/local/cuda/targets/aarch64-linux/lib` (the Tegra build
  scripts' value). Receipts and logs return by `scp`; the main thread
  verifies and commits them (plan § 11). The work directory is removed at
  the end of the window.
- **GPU lock:** `flock /tmp/fathomdb-gpu.lock`; one locked window per
  cell group; `FATHOMDB_REQUIRE_LIVE=1` on every GPU test so a skip fails.
- **Model weights:** the cached `~/.cache/huggingface/hub` copies Slice 30
  used; nothing is downloaded during a timed series.

### 1.3 GCP VM cells (`scripts/gcp-cuda-lab.sh`)

Each session needs the owner's approval, is test and verification only
(no development on a Spot VM; a source bundle or prebuilt artifacts in,
receipts out), is capped at 2 h wall clock (`--max-run 2h`), and follows
the capacity order Spot T4, on-demand T4, then L4 (Q15, Q25). The VM gets
no service account and deletes itself at the cap. `FDB_LAB_PROJECT` names
the project; the active gcloud configuration is never used.

| Session | Cells | Setup | Owner ruling |
| --- | --- | --- | --- |
| **S-13** | T-WS-BUILD 13000 (WSB-02), T-CUDA 13.0 (CUDA-08), T-CUDA-COMPAT C5 (CMP-C5) | `create --profile t4 --name fdb-lab-s13 … --machine-type n1-standard-8` (Q27, below), `driver --driver 580`, the 13.0 apt toolkit on the VM only (Candle `fathomdb/scripts/vm-setup-cuda13.sh`, as K-G2), `push` of a `git bundle` | Q15, Q27 |
| **S-C3** | T-CUDA-COMPAT C3 (CMP-C3), then the `libcuda.so.1`-only cell (CUDA-09) after the remedy step | `create --profile t4 --name fdb-lab-c3 …`, `driver --driver 580` only: no toolkit, no cuBLAS; prebuilt artifacts pushed (design § 9.3) | Q25, Q15a |

**S-13 sizing and attempts (owner ruling Q27, steward ledger seq 330).**
The S-13 session may use `n1-standard-8` or a larger `n1` machine for
**one** build attempt. `scripts/gcp-cuda-lab.sh create` takes
`--machine-type n1-<…>` for the `t4` profile (usage lines 16-17; parsed at
line 71; checked against `^n1-(standard|highmem|highcpu)-[0-9]+$` at lines
114-116; default `n1-standard-4`); the `l4` profile takes `g2-standard-<N>`
instead (lines 122-124). Many attempts to obtain an available VM (zones,
Spot then on-demand, then L4) are allowed. The 2-hour limit per session
(`--max-run 2h`) is still enforced, receipts are pulled (`gcp-cuda-lab.sh
pull`) after every cell, not only at the end, and each session still needs
the owner's go-ahead. S-C3 builds nothing and keeps the default
`n1-standard-4`.

If the owner declines a session, its cells are UNMEASURED by owner
decision, recorded in `status.md` and with `ledgerwrite` (GOAL.md).

### 1.4 Cells UNMEASURED by owner ruling

| Cell | Ruling | AC31 |
| --- | --- | --- |
| Workspace runtime for CUDA 13.1–13.4 builds (R590–R615 drivers; never on `windchill3`) | Q4 | 03b |
| RTX 3090 JIT of 13.0 PTX | not measured (plan § 7 T-CUDA); an L4 in S-13 would cover a post-Turing JIT | 03b |
| Tegra on CUDA 13 (JetPack 7.2.x) | Q5, todo `TC-b267d940` | 04 |
| C3 and C5, only if the owner declines S-C3 or S-13 | Q25, Q15 | 15 |
| T-WS-BUILD 13000 and T-CUDA 13.0, only if the owner declines S-13 (K-G2 at `K` then carries to `K9`, R-REQ ruling 2) | Q15 | 03b |
| Product-level `reserved_cur` after the last close in G3, and G8's reserved-memory bound (the product exposes no pool counters; Slice 30 deviation D-1) | carried from Slice 30 | 04 |

No other cell may be UNMEASURED (AC31-15).

## 2. TDD order for W6

Plan § 10: the failing test first, committed or its red run recorded in
`status.md`, then the code; test files are read-only during fix-to-spec;
the reviewer re-runs each RED sha. W6 runs in three phases so that each RED
fails for its stated reason:

- **Phase A (before the pin commit):** guards, contracts, C1, C4, the
  required-symbol drift guard and the hermetic D-18 tests.
- **Phase B (the pin commit, R-PIN):** manifests, `Cargo.lock`, the
  vendored tree deleted, the pinned-override record; the Phase A guard
  REDs go GREEN here or in the next commit. **Orin window P** (about
  20 min, at the pin commit, before N1 is written; R-TEST finding 14):
  the N1 RED (R14) and the first characterization runs of R15 and R16,
  under § 1.2's guard and lock.
- **Phase C (on the pinned tree):** T-REGISTRY, the device-level D-18
  tests, N1, D-14, D-15, the engine close test.

| # | Test (file) | Fails on the current base because | GREEN change |
| --- | --- | --- | --- |
| R1 | `test_check_single_cudarc.sh` fixture arms A, C (×3), D, the two exclusions and the real tree (`scripts/tests/test_check_single_cudarc.sh`, fixtures `scripts/tests/fixtures/single-cudarc/`, appendix A; all new (W6)) | the guard script `scripts/check-single-cudarc.sh` is absent | new guard (design § 3.5) and its `run_tier_suite fast test-check-single-cudarc …` line; the real-tree arm goes GREEN only in Phase B (today's lock has `cudarc` 0.19.7) |
| R2 | pinned-override schema 5 arms (`scripts/tests/test_check_pinned_override_rot.sh`, AC31-10 named exception): `fork_registry_pin_with_a_caret_requirement_is_rejected`, `a_direct_cudarc_fathomdb_dependency_is_rejected` (three fixtures: exact in `[dependencies]`, caret in an aarch64 target table, own name; design § 3.4 revision 3), `fork_registry_pin_with_a_non_registry_source_in_the_lock_is_rejected`, `fork_registry_pin_with_a_git_source_in_the_lock_is_rejected`, `fork_registry_pin_with_a_different_checksum_is_rejected`, `fork_registry_pin_with_a_second_version_in_the_lock_is_rejected`, `missing_fork_registry_pin_is_rejected`, `candle_kernels_patch_at_another_rev_is_rejected`, `a_cudarc_patch_entry_is_unsupported` (all new (W6)) | the checker has no `cargo_fork_registry_pins` rule and still requires `cargo_cudarc_exception` and the four-package Candle cohort at `859b8ea1` (`check-pinned-override-rot.py` lines 62-103), so each arm's expected message is absent | `scripts/check-pinned-override-rot.py` and `scripts/pinned-override-rot.json` to schema 5 (design § 3.4); the eight cudarc arms and `seed_cudarc_fixture` (lines 149-182, 571-665) deleted; GREEN control `real repository has exact governed npm and Cargo pin provenance` (line 752) passes on the Phase B tree |
| R3 | lock-set arms `matching_metadata_passes`, `changed_libloading_without_a_record_fails`, `added_dependency_fails`, `removed_dependency_fails`, `changed_root_features_fail`, `build_only_dependency_change_passes`, `windows_only_dependency_change_passes`, `record_with_a_wrong_set_sha256_fails`, `unrelated_lock_change_passes` (`scripts/tests/test_check_cudarc_fathomdb_lock_set.sh`, fixtures `scripts/tests/fixtures/cudarc-lock-set/`, each with `metadata.json` and a minimal `Cargo.lock` read by `--lock`, OP-10; new (W6)) | `scripts/check-cudarc-fathomdb-lock-set.py` is absent; after it exists the real-tree arm fails with "no record" until the fork records a run (design § 7.2 sequence) | the guard; its fast-tier line; `scripts/cudarc-fathomdb-lock-set.json` copied from the fork record in a later commit that does not change `Cargo.lock` |
| R4 | `release_cuda_jobs_pin_the_cudarc_binding_version`: label `rejects a CUDA job without CUDARC_CUDA_VERSION 12060`, asserted with the new helper `expect_fail_with <message>` (`scripts/tests/test_cuda_release_contract.sh`; new arm and helper (W6)) | `check-cuda-release-contract.py` `require_x64_cuda_toolkit_environment` (line 207) does not read `CUDARC_CUDA_VERSION`, so a workflow without the line passes and the expected message is absent | the checker requires `CUDARC_CUDA_VERSION: "12060"` in each CUDA job, derived from the toolkit root's `12.6`, and names the job in its message (design § 1) |
| R5 | release-contract Candle arms, rewritten (OP-1 ruling): `rejects a candle-kernels patch at a rev other than K9`, `rejects a Candle fork crate reintroduced as a git patch`, `rejects a candle-kernels lock entry from crates.io`, each through `expect_fail_with <message>` (same file; new arms (W6)) | **RED: `baseline CUDA contract agrees` (line 108) fails on the Phase B sha**: the checker still pins all four Candle packages to `859b8ea1` by git (`check-cuda-release-contract.py` lines 120-126, 698-725), and today's arms `rejects a split Candle source selection` (line 540) and `rejects a Candle lockfile that falls back to crates.io` (line 554) assert that form | the checker's Candle block reduced to the `candle-kernels` git patch at `K9` and its single git lock entry (REQ31-16), the rev read from `scripts/pinned-override-rot.json` (`cargo_candle_exception.rev`), not a third literal (design § 1 revision 3); the two old arms deleted |
| R6 | `test_slice31_retired_cudarc_references.sh` (`scripts/tests/`; new (W6); fast tier) with three cases: `no_vendored_cudarc_reference_outside_history` (AC31-10's `git grep`), `no_private_pool_marker_reference_outside_history` (design § 3.3), both building their patterns from pieces, and `the_tegra_pool_feature_is_empty` (cmd `c-tegra-pool-feature`) | outside `dev/plans`, `dev/adr`, the ledgers and `CHANGELOG.md`, 16 files besides the vendored tree match the vendored-tree patterns and 6 name the marker (measured 2026-10-13 with `c-grep-vendored` and `c-grep-marker`); `features["tegra-pool"]` is `["dep:cudarc", "cudarc/fathomdb-private-pool"]` (embedder `Cargo.toml:148`) | Phase B deletions, `tegra-pool = []`, and the "References to update" list (plan § 9 W6 step 2) |
| R7 | C1 `scripts/tests/test_check_ptx_driver_floor.sh` arms `ptx_8_5_sm_75_passes`, `ptx_8_6_fails`, `ptx_target_sm_80_fails_for_x86`, `ptx_target_sm_87_passes_for_tegra`, `no_ptx_found_fails` (fixtures `scripts/tests/fixtures/ptx-floor/`; new (W6)) | `scripts/check-ptx-driver-floor.py` is absent | new checker: scans every PTX module (`.ptx` files under a Candle `OUT_DIR`, or PTX text embedded in an artifact) and requires `.version` ≤ 8.5 and `.target` equal to the contract's `sm_<CUDA_COMPUTE_CAP>` (OP-6) |
| R8 | C4 floor tests (`tests/slice31_compute_floor.rs`, new (W6); design § 9.2): `forced_cuda_on_a_device_below_the_kernel_floor_is_cuda_incompatible_without_a_probe`, `auto_with_the_lowest_ordinal_below_the_floor_falls_back_to_cpu`, `doctor_gpu_reports_cuda_incompatible_below_the_floor`, `reranker_forced_cuda_below_the_floor_is_cuda_incompatible_without_a_probe`, `reranker_auto_below_the_floor_falls_back_to_cpu`, `the_build_floor_parses_the_contract_values` | the resolver calls `probe_cuda(0)` on the 5.0 device and selects it; `CudaProvider::kernel_compute_floor` and the parser do not exist (compile failure is the RED for the last) | `device_policy::below_compute_floor`, the provided trait method, the check at `device_policy.rs:584`, `:702` and `reranker_device_policy.rs:404` |
| R9 | `a_lib_load_error_for_a_missing_cuda_12_cublas_is_a_typed_refusal_with_the_remedy`, `a_lib_load_error_for_a_wrong_major_cublas_is_wrong_major`, and (Q26) `missing_required_symbols_lists_each_absent_name_in_set_order`, `a_missing_symbol_refusal_names_the_symbols_and_the_remedy` (`src/cuda_libraries.rs` unit tests; new (W6); `embed-cuda`, build-only on `windchill3`, no GPU) | `refusal_from_lib_load_error`, the symbol-check function, `CudaProbeError::LibraryUnavailable` and the kind `cuda_library_unavailable` do not exist (compile failure) | `cuda_libraries.rs`, the variants and the resolution mapping (design § 9.1 items 2-3) |
| R10 | `doctor_cuda_libraries_parses_as_a_doctor_verb`, `doctor_cuda_libraries_json_has_the_v1_schema_and_every_library`, `doctor_cuda_libraries_exits_70_on_a_refusal` (`src/rust/crates/fathomdb-cli/src/lib.rs` `mod tests`; new (W6)) | `Cli::try_parse_from(["fathomdb", "doctor", "cuda-libraries", "--json"])` fails: no such verb | the verb, schema `fathomdb.doctor.cuda-libraries.v1`, exit 70 (design § 9.1 item 5) |
| R11 | T-REGISTRY: `scripts/check-tegra-pool-registry-build.sh` on the Phase B tree (AC31-10 named exception) | the old script stops at line 102, "[patch.crates-io] must carry exactly one cudarc entry", once the cudarc patch entry is gone: the recorded RED (design § 3.6) | the rewritten script, (a)-(d) (§ 3.10) |
| R12 | `a_libcublas_so_12_that_reports_major_13_is_refused_before_gpu_work` (`tests/slice31_cuda_library_major.rs`, new (W6); fixture `tests/fixtures/decoy-cublas/decoy_cublas.c`, new (W6), installed as `libcublas.so.12` and `libcublas.so`) on `windchill3`, RTX 3090 by UUID | **the child panics in device construction (`cublasCreate_v2` 'Expected function…' via dynamic loading); no `LibraryUnavailable`** (R-TEST finding 6; the cudarc sources word the panic `Missing symbol cublasCreate_v2: …`, design § 9.1) | `require_usable_cuda_libraries()` before `new_cuda_device` in both `probe_cuda` implementations. The GREEN assertion requires the typed refusal (`LibraryUnavailable`, `WrongMajor`) in the child's output, so a panicking child never passes |
| R12b | `a_libcublas_so_12_missing_a_required_symbol_is_refused_before_gpu_work` (same file; fixture `tests/fixtures/decoy-cublas/decoy_cublas_missing_symbol.c`, new (W6); owner ruling Q26) on `windchill3`, RTX 3090 by UUID | the probe selects the device, and the child's F32 batched matmul panics on the missing `cublasSgemmStridedBatched` (cudarc `load`); no `LibraryUnavailable` (design § 9.1) | the required-symbol check in `require_usable_cuda_libraries()`; GREEN requires `LibraryUnavailable`, reason `MissingSymbol`, `missing_symbols == ["cublasSgemmStridedBatched"]` in the child's output, no device built, exit 0 |
| R13 | `the_cuda_library_report_names_the_loaded_files` and `a_refusal_writes_one_stderr_line` (same file) | `CudaDeviceInfo` has no `cuda_libraries` field (compile failure); no stderr line exists | the report field, the `Once`-guarded `fathomdb-cuda-libraries {json}` line (Q24). `a_successful_load_writes_no_stderr_line` (same file) is a **regression guard**, not a RED: it cannot fail on the base (R-TEST finding 7) |
| R14 | N1, **conditional**: `gpu_orin_64_dropping_the_embedder_releases_pool_memory_without_a_caller_synchronize` and `gpu_orin_64_close_release_also_trims_a_max_threshold_pool` (`src/cuda_pool_slice31_tests.rs`, new (W6); Orin window P at the pin commit, OP-8) | expected: after the drop, without a synchronize, the pool's `reserved_current` is above 0 (release threshold 0 releases only at a synchronization point, cudarc D-5); with `FATHOMDB_POOL_RELEASE_THRESHOLD=max` it is never released | `release_unused_private_pool_memory()` and the guard field (design § 8). **If the RED passes on the pinned tree, N1 is dropped**, the result goes to `status.md`, and the two tests stay as characterization |
| R15 | D-15 characterization (Orin; first run in window P): `gpu_orin_64_process_exit_with_the_private_pool_alive_is_clean`, `gpu_orin_64_dropping_the_embedder_then_exiting_is_clean` (`src/cuda_pool_slice31_tests.rs`); F1-F5 (`src/python/tests/test_tegra_pool_fork_after_init.py`; in window A, which has the Tegra wheel); the engine close test R16 | these characterize existing behaviour, so they are expected GREEN on first run (design § 6); the first run is recorded in `status.md` as "characterization, GREEN at <sha>", not as a RED. A failure is a finding for R-VERIFY, never a test edit | none, unless a run fails |
| R16 | `gpu_orin_64_engine_close_with_the_private_pool_then_exit_is_clean` (`src/rust/crates/fathomdb-engine/tests/slice31_engine_close_cuda.rs`, new (W6); R-DES ruling 7; first run in window P) | characterization (as R15) | none |
| R17 | D-12, D-14 measurements: `gpu_orin_64_records_the_usable_capacity_of_the_three_gib_pool`, `gpu_orin_64_memory_outside_the_private_pool_is_recorded` (`src/cuda_pool_slice31_tests.rs`) | measurements, not REDs; the D-14 leak check (default pool counters 0) is asserted and expected GREEN | none |
| R18 | required-symbol drift guard arms `matching_sets_pass`, `a_result_layer_symbol_missing_from_the_constant_fails`, `a_direct_candle_sys_call_fails`, `a_fathomdb_sys_call_fails` and the real tree (`scripts/tests/test_check_cuda_required_symbols.sh`, fixtures `scripts/tests/fixtures/cuda-required-symbols/`; new (W6); Q26) | `scripts/check-cuda-required-symbols.py` is absent | the checker and its fast-tier line (design § 9.1 "Drift guard"); the real-tree arm goes GREEN once `cuda_libraries.rs` holds the constants |

**Not REDs, by nature:** the AC31-10 frozen-file diff (§ 3.1, PRD-30) is a
review check, GREEN at every commit that obeys the rule; the D-17 sites 3
and 10 change is a refactor under the unchanged Slice 30 tests (design
§ 8). Both are listed so no one mistakes their GREEN for a missing RED.

**Generated-file updates W6 makes with the new tests** (or the fast tier
fails): `python3 scripts/lib/test_targets.py --write-matrix` for the new
test targets (`scripts/check-test-target-coverage.py` checks
`scripts/test-feature-matrix.toml`); `scripts/test-skip-allowlist.toml`
entries for the new `#[ignore]` and platform-excluded targets
(`slice31_cuda_library_major` compiles to nothing off Linux;
`missing_cuda_12_cublas_is_a_typed_refusal_with_the_remedy` is
`#[ignore = "C3 GCP cell"]` and listed as `ignored-by-design`;
`slice31_engine_close_cuda` is `#[ignore]` by design); the tier totality
check `scripts/check-agent-test-tier-totality.sh` for the five new fast
suites (single-cudarc, lock-set, PTX floor, required symbols, retired
references). The
feature-complete run (cmd `c-fc`) is the check that the allowlist is
complete.

## 3. Test catalogue

Commands too long for a table cell, or containing a pipe, are given by
label in § 3.13. Every *GPU* row runs under § 1.1's `wc3-env.sh` and lock;
every *Orin* row under § 1.2's guard, lock and `FATHOMDB_REQUIRE_LIVE=1`.

### 3.1 T-PRODUCT (workspace, guards, references)

| ID | Test | File | Tier | Host | Command | Expected | AC31 | Design |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| PRD-01 | full gate | — | full | `windchill3`, slice worktree with its own `.venv` | `./scripts/agent-verify.sh` (rerun unconfined if AC-036's `strace` is denied) | exit 0 | 02, 10 | — |
| PRD-02 | single-cudarc A: `lock/upstream-cudarc.lock` | `scripts/tests/test_check_single_cudarc.sh` (new) | fast | any | `bash scripts/tests/test_check_single_cudarc.sh` | guard fails, message names `cudarc 0.19.4` | 07 | § 3.5 |
| PRD-03 | single-cudarc A exclusion: `lock/ug-cudarc.lock` | same | fast | any | same | guard passes (`cudarc` 0.17.8) | 07 | § 3.5 |
| PRD-04 | single-cudarc C: `tree/upstream-cudarc/` | same | fast | any | same | guard fails on the line `cudarc v0.19.4` | 07 | § 3.5 |
| PRD-05 | single-cudarc C: `tree/path-cudarc-fathomdb/` | same | fast | any | same | guard fails: `cudarc-fathomdb v0.19.10 (<path>)` is not exactly `cudarc-fathomdb v0.19.10` | 07 | § 3.5 |
| PRD-06 | single-cudarc C non-vacuity: `tree/no-cudarc-fathomdb/` | same | fast | any | same | guard fails: no `cudarc-fathomdb` line for `embed-cuda` | 07 | § 3.5 |
| PRD-07 | single-cudarc C exclusion: `tree/flash-attn-only/` | same | fast | any | same | guard passes | 07 | § 3.5 |
| PRD-08 | single-cudarc D: `tree/concrete-feature/` | same | fast | any | same | guard fails on `cudarc-fathomdb feature "cuda-12060"` | 07, 03b | § 1, § 3.5 |
| PRD-09 | single-cudarc GREEN controls: `lock/ok.lock`, `tree/ok/`, and the real tree | same | fast | any (registry cache filled by `cargo fetch --locked`) | same | guard passes; fixtures consistent (`cargo metadata --offline --locked` on each tree fixture first) | 07 | § 3.5 |
| PRD-10 | `Cargo.lock` has exactly one `cudarc-fathomdb`, 0.19.10, crates.io source, checksum `8780d2fe…` | pinned-override checker | fast | any | `python3 scripts/check-pinned-override-rot.py` | exit 0 | 07, 14 | § 3.4 |
| PRD-11 | pinned-override arms R2 (nine names) and the GREEN control | `scripts/tests/test_check_pinned_override_rot.sh` | fast | any | `bash scripts/tests/test_check_pinned_override_rot.sh` | each `PASS` line printed; exit 0 | 06, 07, 14, 16 | § 3.4 |
| PRD-12 | lock-set arms R3 (nine names) | `scripts/tests/test_check_cudarc_fathomdb_lock_set.sh` (new) | fast | any | `bash scripts/tests/test_check_cudarc_fathomdb_lock_set.sh` | each arm as named; exit 0 | 14 | § 7.2 |
| PRD-13 | lock-set guard on the real tree | `scripts/check-cudarc-fathomdb-lock-set.py` (new) | fast | any | `python3 scripts/check-cudarc-fathomdb-lock-set.py --root . --record scripts/cudarc-fathomdb-lock-set.json` | exit 0; the record's `fathomdb_lock_commit` is the W6 lock commit | 14 | § 7.2 |
| PRD-14 | `release_cuda_jobs_pin_the_cudarc_binding_version` | `scripts/tests/test_cuda_release_contract.sh` | fast | any | `bash scripts/tests/test_cuda_release_contract.sh` | the arm passes: the checker exits nonzero **and** its stderr contains the expected message naming `CUDARC_CUDA_VERSION` and the job (`expect_fail_with`) | 03b | § 1 |
| PRD-15 | release-contract Candle arms R5 and `baseline CUDA contract agrees` | same | fast | any | same | all pass on the Phase B tree; each R5 arm matches its message through `expect_fail_with`; the checker reads `K9` from `scripts/pinned-override-rot.json` (`grep -c 182ceab0 scripts/check-cuda-release-contract.py` is 0) | 16 | § 1 (OP-1) |
| PRD-16 | `no_vendored_cudarc_reference_outside_history` | `scripts/tests/test_slice31_retired_cudarc_references.sh` (new) | fast | any | `bash scripts/tests/test_slice31_retired_cudarc_references.sh` | `git grep` (cmd `c-grep-vendored`) empty | 10 | § 3.3 |
| PRD-17 | `no_private_pool_marker_reference_outside_history` | same | fast | any | same | `git grep` (cmd `c-grep-marker`) empty | 06 | § 3.3 |
| PRD-18 | the vendored tier entry is gone | `scripts/agent-test.sh` | review | any | cmd `c-vendored-gone` | all three hold | 10 | plan § 9 W6 |
| PRD-19 | root patch table holds exactly `libsqlite3-sys` (path) and `candle-kernels` (git, `rev` = `K9`) | `Cargo.toml` | review | any | cmd `c-patch-table` | the two entries, nothing else | 06, 16 | § 2, § 3.2 |
| PRD-20 | `Cargo.lock` has one `candle-kernels`, source `git+https://github.com/coreyt/candle-fathomdb.git?rev=182ceab0…#182ceab0…`, no registry copy | pinned-override checker (and the release contract per OP-1) | fast | any | `python3 scripts/check-pinned-override-rot.py` | exit 0 | 16 | § 3.4 |
| PRD-21 | embedder manifest: Candle crates `=0.11.0`; no direct `cudarc-fathomdb` dependency in any form or table; `tegra-pool = []` | pinned-override checker arms `fork_registry_pin_with_a_caret_requirement_is_rejected`, `a_direct_cudarc_fathomdb_dependency_is_rejected` (three fixtures); R6 case `the_tegra_pool_feature_is_empty` | fast | any | as PRD-11; cmd `c-tegra-pool-feature` | pass; `features["tegra-pool"] == []` | 06, 14 | § 3.2, § 3.4 |
| PRD-22 | C1 fixture arms (R7) | `scripts/tests/test_check_ptx_driver_floor.sh` (new) | fast | any | `bash scripts/tests/test_check_ptx_driver_floor.sh` | each arm as named | 15 | plan § 7 C1 |
| PRD-23 | C4 floor tests (R8, six names) plus `auto_with_the_lowest_ordinal_above_the_floor_selects_it`, `a_device_at_the_floor_is_probed`, `an_unknown_compute_capability_is_probed` | `tests/slice31_compute_floor.rs` (new) | full | any, no GPU | `cargo test -p fathomdb-embedder --test slice31_compute_floor` | 9 passed; each fake records the expected `probe_cuda` calls | 15 | § 9.2 |
| PRD-24 | D-18 hermetic tests (R9, four names) and the pure reason-string and message tests `refusal_reason_strings_are_missing_wrong_major_and_missing_symbol`, `the_refusal_message_names_library_file_version_and_major` | `src/cuda_libraries.rs` (new) | build-only; feature-complete (`embed-cuda` set) | `windchill3`, § 1.1 build environment; no GPU opened. Not in the full gate: `embed-cuda` needs `nvcc` and `scripts/test-rust-workspace.sh` builds no CUDA feature | cmd `c-prd24` (quoted in the receipt) | 6 passed; the missing case's message contains `pip install nvidia-cublas-cu12 nvidia-curand-cu12`, `nvidia.cublas`, `nvidia.curand` and `LD_LIBRARY_PATH`; no expectation names major 12 for cuRAND | 15 | § 9.1 |
| PRD-25 | CLI verb tests (R10, three names) | `src/rust/crates/fathomdb-cli/src/lib.rs` | full | any | `cargo test -p fathomdb-cli --lib doctor_cuda_libraries` | 3 passed | 15 | § 9.1 |
| PRD-26 | the seven Slice 30 `doctor cuda-allocator` tests (`lib.rs:3170-3301`: `doctor_cuda_allocator_explicit_cpu_returns_before_any_cuda_call`, `…_affirmed_sbsa_returns_before_any_cuda_call`, `…_without_a_cuda_feature_returns_before_any_cuda_call`, `…_runs_early_init_then_probe_and_reports_the_decision`, `…_reports_a_cpu_resolution_and_a_missing_report_as_not_applicable`, `…_probe_failure_is_an_internal_failure`, `…_parses_as_a_doctor_verb`) unchanged and passing | `src/rust/crates/fathomdb-cli/src/lib.rs` | full | any | `cargo test -p fathomdb-cli --lib doctor_cuda_allocator` | 7 passed | 10 | § 4 |
| PRD-27 | interface and user docs carry the D-18 surface and the floors | `dev/interfaces/{rust,rust-sdk,python,typescript,cli}.md`, `docs/embedder.md`, `docs/reference/cli.md` | review | any | cmd `c-docs` | each grep finds its text; `./scripts/agent-lint-md.sh` and `./scripts/agent-lint-docs.sh` pass | 10, 15, 08 | § 9.1 item 7 |
| PRD-28 | ADR amendments and changelog | `dev/adr/ADR-0.8.28-tegra-private-cuda-pool.md`, `dev/adr/ADR-0.8.28-cuda-version-support-policy.md`, decision index rows 62, 63, `CHANGELOG.md` | review | any | cmd `c-adr` | amendment headings present; every vendored-tree statement (lines 6, 101, 142 and the item-6 text) marked superseded; changelog entries for the five deliberate changes (REQ31-10) | 10 | appendices A, B |
| PRD-30 | AC31-10 frozen files unchanged | nine files | review | any | cmd `c-frozen` | empty diff, or only the named exceptions, each listed with its diff in `status.md` | 10 | § 6 |
| PRD-31 | `scripts/check-pinned-override-rot.sh` wrapper and the agent-lint verb pass on the tree | `scripts/agent-lint.sh` | full | any | `./scripts/agent-lint.sh` | exit 0 | 06 | § 3.4 |
| PRD-32 | required-symbol drift guard arms (R18) and the real tree | `scripts/tests/test_check_cuda_required_symbols.sh` (new) | fast | any (registry cache filled by `cargo fetch --locked`) | `bash scripts/tests/test_check_cuda_required_symbols.sh` | each arm as named; the real tree reports 13 cuBLAS and 14 cuRAND names (or W6's recorded counts) | 15 | § 9.1 (Q26) |
| PRD-33 | feature-complete gate, once | `scripts/test-feature-complete.sh` | feature-complete | `windchill3` (RTX 3090s at indices 0 and 1 under `CUDA_DEVICE_ORDER=PCI_BUS_ID`; OP-3) | cmd `c-fc` | exit 0; the receipt lists at least the `fathomdb-embedder` (`embed-cuda` among them) and `fathomdb-cli` sets with their test counts; no allowlist gap, no stale entry | 10, 15 | § 9.1 |

**Cross-references (no separate run; proportionality).** The Slice 30
product tests of the nine AC31-10 files run inside PRD-01 (revision 1's
PRD-29).

### 3.2 T-WS-BUILD

| ID | Test | File | Tier | Host | Command | Expected | AC31 | Design |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| WSB-01 | workspace build, 12060 | — | GPU (build only) | `windchill3` | cmd `c-wsb-12060` | each build exits 0; `nvcc --version` 12.6 recorded; `CUDARC_CUDA_VERSION=12060` | 03b | § 1 |
| WSB-02 | workspace build and test binaries, 13000 | — | GCP S-13 | T4 (or L4), driver 580, 13.0 apt toolkit on the VM | cmd `c-wsb-13000` | each build exits 0; `nvcc --version` 13.0; `cudarc-fathomdb` 0.19.10 registry, Candle 0.11.0 (the registry form replaces K-G2's carry, R-REQ ruling 2) | 03b | § 1 item 4 |

### 3.3 T-CUDA (product runtime cells)

| ID | Test | File | Tier | Host | Command | Expected | AC31 | Design |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CUDA-01 | release CUDA preflight: x86 wheel and N-API tarball built, inspected, driverless CPU smokes, GPU Python and N-API embed | `scripts/release/cuda-preflight.sh` | GPU | `windchill3`, RTX 3090 by UUID (`FATHOMDB_CUDA_GPU_UUID`) | `bash scripts/release/cuda-preflight.sh "$S/preflight-embed"` | exit 0; `inspect-cuda-artifacts.py` passes; GPU smokes report `cuda` | 02, 16 | — |
| CUDA-02 | the same with the reranker | same | GPU | same | `bash scripts/release/cuda-preflight.sh "$S/preflight-rerank" --rerank-cuda` | exit 0; rerank on `cuda` | 02 | — |
| CUDA-03 | `doctor cuda-allocator` reports the pool path not built on x86 | release CLI | GPU | same | cmd `c-doctor-x86` | `"built":false` (plan § 7's "`not_built`": `TEGRA_POOL_BUILT` is false off aarch64 Linux), `"path":null`, `"reason":"not_applicable"` (on x86 the device report carries no allocator record, and `cuda_allocator_json` maps a missing report to `not_applicable`, `fathomdb-cli/src/lib.rs:1194`; `not_built` as a reason exists only on aarch64 Linux CUDA builds without `tegra-pool`, Slice 30 design § 3.3; AC31-02 says this since revision 2), `"module_load_init":"ran"` | 02 | plan § 2 |
| CUDA-04 | decoy `libcublas.so.12` reporting major 13 is refused (R12) | `tests/slice31_cuda_library_major.rs` (new) | GPU | same | cmd `c-libmajor-wc3` | 1 passed: `LibraryUnavailable`, reason `WrongMajor`, the decoy's path, version 13, required major 12; no device built; the typed refusal is read from the child's output (a panic fails) | 15 | § 9.1 |
| CUDA-05 | `the_cuda_library_report_names_the_loaded_files` | same | GPU | same | same | 1 passed; every `path` in the report is in `/proc/self/maps`; cuRAND reported with major 10 | 15 | § 9.1 |
| CUDA-06 | `a_refusal_writes_one_stderr_line` (RED) and `a_successful_load_writes_no_stderr_line` (regression guard) | same | GPU | same | same | 2 passed (Q24) | 15 | § 9.1 |
| CUDA-07 | `doctor cuda-libraries` on the host | release CLI | GPU | same | cmd `c-doctor-libs-wc3` | exit 0; `built_major` 12; cuBLAS `loaded`, its `path` equal to `realpath /usr/local/cuda/targets/x86_64-linux/lib/libcublas.so.12` (a file under `/usr/local/cuda-12.6/`; the loader reaches it through the `/usr/local/cuda` symlink, so real paths are compared); cuRAND `loaded`, major 10, `path` equal to `realpath …/libcurand.so.10`; `refusal` null | 15 | § 9.1 |
| CUDA-08 | T-CUDA 13.0: embed and rerank end to end, `doctor cuda-allocator` | 13.0 builds on the VM (WSB-02) | GCP S-13 | T4, driver 580, 13.0 toolkit | cmd `c-s13-run` | the live embed test and the reranker GPU test each 1 passed on `cuda:0` (`FATHOMDB_REQUIRE_LIVE=1`); `doctor cuda-allocator` as CUDA-03 (`"built":false`, `"reason":"not_applicable"`) | 03b | plan § 7 |
| CUDA-09 | only `libcuda.so.1` present: the 12.6 build embeds | the 12.6 live embed test binary built on `windchill3` | GCP S-C3, after the remedy step (R-TEST finding 2) | same VM, `dpkg-divert` of the unversioned `libcuda.so` (Q15a) | cmd `c-c3-libcuda1` | the forced embed passes again (1 passed); `/proc/<pid>/maps` shows `libcuda.so.1` and no `libcuda.so` (Candle R-FK-VERIFY F5) | 03b | design § 9.3 step 5 |
| CUDA-11 | decoy `libcublas.so.12` missing `cublasSgemmStridedBatched` is refused (R12b, Q26) | `tests/slice31_cuda_library_major.rs` (new) | GPU | `windchill3`, RTX 3090 by UUID | cmd `c-libmajor-wc3` | 1 passed: `LibraryUnavailable`, reason `MissingSymbol`, `missing_symbols` `["cublasSgemmStridedBatched"]`; no device built; no matmul run | 15 | § 9.1 |

**Cross-references (no separate run).** The Orin product cells that
revision 1 listed as CUDA-10 are § 3.4 and § 3.5.

### 3.4 T-POOL-PRODUCT (Orin only; pool path built only on aarch64 Linux)

| ID | Test | File | Tier | Host | Command | Expected | AC31 | Design |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| POOL-01 | `forced_cuda_probe_succeeds_when_the_default_memory_pool_is_unavailable` | `tests/tegra_fragmented_va_cuda.rs` | Orin | Orin | cmd `c-pool-va` | 1 passed (run without `tegra-pool`, Slice 30 F-14) | 02 | § 4 |
| POOL-02 | `gpu_orin_64_builds_every_device_on_a_three_gib_private_pool` | `src/cuda_pool_policy_tests.rs` | Orin | Orin | cmd `c-pool-lib` with that name | 1 passed | 02 | § 4 |
| POOL-03 | `gpu_orin_64_pool_stays_under_max_size_and_is_empty_after_the_embedder_drops` | same | Orin | Orin | same | 1 passed (sites 3 and 10 refactor stays GREEN, design § 8) | 02 | § 8 |
| POOL-04 | `gpu_orin_64_a_primary_context_reset_is_context_lost_and_teardown_segfaults` (runs `gpu_orin_64_context_reset_child`) | same | Orin | Orin | same | 1 passed: `cuda_context_lost` on both embeds, one snapshot line, SIGSEGV at teardown (D-15 item 1; no SIGSEGV is an R-VERIFY finding, not a test edit) | 02, 04 | § 6 |
| POOL-05 | N1 RED/GREEN (R14) | `src/cuda_pool_slice31_tests.rs` (new) | Orin | Orin | cmd `c-pool-s31` with `gpu_orin_64_dropping_the_embedder_releases_pool_memory_without_a_caller_synchronize`, then `gpu_orin_64_close_release_also_trims_a_max_threshold_pool` | GREEN after N1: `reserved_current` 0 without a caller synchronize, and with threshold `max` | 02, 04 | § 8 |
| POOL-06 | Slice 30 G10 tests (pure, every host) | `src/cuda_pool_policy_tests.rs` (names in Slice 30 qualification § 11) | full | any | PRD-01 | pass | 04 | § 4 |

### 3.5 T-ORIN

| ID | Test | File | Tier | Host | Command | Expected | AC31 | Design |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ORIN-01…09 | Slice 30 gate rows G1–G9 on the new base; G1 includes the `doctor cuda-allocator` check (revision 1's ORIN-11, merged: cmd `c-doctor-orin` ×20, 20/20 identical `"path":"private"`, `"reason":"private_pool"`, `"pool_max_size_bytes":3221225472`, `"release_threshold":"0"`, `"module_load_init":"ran"`) | harness `run-gates.sh` (copy) | Orin | Orin | § 4.2 | § 4.2 (Slice 30 plan § 4.2 criteria, with the owner rulings of 2026-10-08) | 04, 13 (G9) | § 4 |
| ORIN-12 | Node/V8 fragmented end to end, engine closed, exit 0 | `scripts/tests/test_tegra_node_early_cuinit.sh` | Orin | Orin | cmd `c-node-e2e` | every run `PASS`; the consumer closes its engine (`tegra_node_early_cuinit_consumer.mjs` step `close`) and exits 0 with no signal (rc < 128); receipt names the run count | 04 | § 6 (engine close) |
| ORIN-13 | usable capacity of the 3 GiB pool | `gpu_orin_64_records_the_usable_capacity_of_the_three_gib_pool` (`src/cuda_pool_slice31_tests.rs`, new) | Orin | Orin | cmd `c-pool-s31` | prints usable bytes; non-gating; below 576 MiB returns sizing to the owner (§ 4.5) | 04 | § 4, § 13 ruling 2 |
| ORIN-14 | D-14 memory outside the pool | `gpu_orin_64_memory_outside_the_private_pool_is_recorded` (same file) | Orin | Orin | cmd `c-pool-s31` | six `fathomdb-d14 {json}` lines S0–S5; default pool `reserved_current` and `used_high` 0 at every stage (asserted) | 06 | § 5 |
| ORIN-15 | AC31-16 Tegra linkage on the Orin-built artifacts | `scripts/release/check-tegra-wheel-linkage.sh` | Orin | Orin (window A) | cmd `c-tegra-linkage` | exit 0 on the Tegra wheel's extension and on the Tegra N-API binary; the receipt quotes the output | 16 | REQ31-16 |

**Cross-references (no separate run).** G10 (revision 1's ORIN-10) is
POOL-06 in PRD-01 plus the fork's G6 receipt (FEV-05).

### 3.6 T-TEARDOWN-PRODUCT (D-15, Orin)

| ID | Test | File | Tier | Host | Command | Expected | AC31 | Design |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| TDN-01 | `gpu_orin_64_process_exit_with_the_private_pool_alive_is_clean` | `src/cuda_pool_slice31_tests.rs` (new) | Orin | Orin | cmd `c-pool-s31` | 1 passed: child exit status 0, no signal | 04 | § 6 item 2 |
| TDN-02 | `gpu_orin_64_dropping_the_embedder_then_exiting_is_clean` | same | Orin | Orin | same | 1 passed: status 0, no signal; `reserved_current` 0 before exit | 04 | § 6 |
| TDN-03 | `gpu_orin_64_engine_close_with_the_private_pool_then_exit_is_clean` | `src/rust/crates/fathomdb-engine/tests/slice31_engine_close_cuda.rs` (new) | Orin | Orin | cmd `c-engine-close` | 1 passed: a child opens an engine with the default embedder on `cuda:0`, report path `private`, writes, searches, closes the engine, exits; status 0, no signal (OP-4: no counter clause) | 04 | § 6, R-DES ruling 7 |
| TDN-06 | F1 `test_forked_child_after_import_forced_cuda_raises_cuda_probe_failed` | `src/python/tests/test_tegra_pool_fork_after_init.py` (new) | Orin | Orin, Tegra wheel in a scratch venv | cmd `c-fork` | `EmbedDevicePolicyError`, kind `cuda_probe_failed`; child ends within 120 s | 04 | § 6 item 4 |
| TDN-07 | F2 `test_forked_child_with_early_init_off_embeds_on_cuda` | same | Orin | same | same | opens on `cuda`, path `default_pool` or `synchronous`, reason `cuinit_opted_out`, embeds | 04 | § 6 |
| TDN-08 | F3 `test_forked_child_after_import_auto_falls_back_to_cpu` | same | Orin | same | same | opens on CPU, reason `cuda_probe_failed` | 04 | § 6 |
| TDN-09 | F4 `test_forked_child_after_parent_gpu_use_gets_a_typed_error_and_the_parent_still_embeds` | same | Orin | same | same | child: a typed error, no signal; parent: embeds again on `private`, counters below the cap | 04 | § 6 |
| TDN-10 | F5 `test_spawned_child_embeds_on_the_private_pool` | same | Orin | same | same | child opens on `cuda`, path `private`, embeds | 04 | § 6 |

**Cross-references (no separate run).** The lost-context case (revision
1's TDN-04) is POOL-04; the engine close in Node/V8 (TDN-05) is ORIN-12.
F4 needs no RED mutation (OP-7 ruling).

### 3.7 T-CUDA-COMPAT (REQ31-15)

| ID | Test | File | Tier | Host | Command | Expected | AC31 | Design |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CMP-C1b | PTX floor on the real 12.6 builds | `scripts/check-ptx-driver-floor.py` (new) | build-only; Orin | `windchill3` (`sm_75`); Orin (`sm_87`, window A) | cmd `c-c1` | every PTX module `.version` ≤ 8.5, `.target` `sm_75` (x86) and `sm_87` (Orin); module count > 0 recorded | 15 | plan § 7 C1 |
| CMP-C2 | older and newer 12.x cuBLAS | the x86 CUDA wheel from CUDA-02 in a scratch venv, the embed-and-rerank script `dev/plans/runs/0.8.28-slice-31/harness/c2-embed-rerank.py` (new (W6)), the release CLI's `doctor cuda-libraries`, and the CUDA-05 test | GPU | `windchill3`, RTX 3090 by UUID; one scratch venv per version: `nvidia-cublas-cu12==12.1.3.1` and `nvidia-cublas-cu12==12.9.2.10` (12.4 dropped, proportionality) | cmd `c-c2` | per version: the script reports embed and rerank on `cuda`; `doctor cuda-libraries` names the venv's `nvidia/cublas/lib/libcublas.so.12` and its version, equal to the `/proc/self/maps` entry; cuRAND (major 10) comes from the loader cache, `/usr/local/cuda/targets/x86_64-linux/lib/libcurand.so.10` (real path under `/usr/local/cuda-12.6/`), and the receipt says so; `pip freeze` of the venv recorded. A missing required symbol is a typed `MissingSymbol` refusal (Q26), never a crash; a refusal fails the cell for that version and goes to the owner, because REQ31-15 promises 12.1–12.9 | 15 | § 9.1 |
| CMP-C3 | no CUDA 12 cuBLAS (GCP) | `missing_cuda_12_cublas_is_a_typed_refusal_with_the_remedy` (`tests/slice31_cuda_library_major.rs`, new, `#[ignore = "C3 GCP cell"]`), the live test binaries and the release CLI's doctor verbs | GCP S-C3 | Spot T4, driver 580 only | § 4.6 | refusal: the ignored test 1 passed (forced `cuda:0` fails `cuda_library_unavailable`, reason `missing`, the remedy text; `auto` on CPU with that reason; one stderr line); `doctor cuda-libraries` exit 70, `reason` `missing`, `library` `cublas`, tried names and remedy; the `auto` live embed passes on CPU. Floor: `doctor gpu` not `cuda_incompatible` on the 7.5 T4. Remedy (both wheel directories on `LD_LIBRARY_PATH`): exit 0, cuBLAS `loaded` from the venv's `nvidia/cublas/lib`, cuRAND (major 10) from `nvidia/curand/lib`; the forced live embed and reranker tests each 1 passed on the T4 | 15 | § 9.3 |
| CMP-C5 | 13.0 build with 13.0 libraries | CUDA-08 + `doctor cuda-libraries` | GCP S-13 | T4, 13.0 toolkit | cmd `c-s13-run` | CUDA-08, plus `built_major` 13 and cuBLAS 13.x `loaded` from the toolkit | 15 | plan § 7 C5 |
| CMP-DOC | floors in `docs/embedder.md` | PRD-27 | review | any | cmd `c-docs` | CUDA library floor 12.x (tested 12.1–12.9), driver floor 560.28.03, GPU floor Turing (7.5) on x86, Orin on Jetson | 15 | plan § 9 W6 |

**Cross-references (no separate run).** C1's fixture arms (revision 1's
CMP-C1a) are PRD-22; C4 (CMP-C4) is PRD-23, 9 passed, with the K620 never
run; the D-18 hermetic tests (CMP-D18) are PRD-24, which stand whatever
C3's outcome (Q25).

### 3.8 T-PERF (AC31-13)

| ID | Test | File | Tier | Host | Command | Expected | AC31 | Design |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| PERF-01 | G9 Node 25: new, baseline, S-new | harness `run-gates.sh g9` (copy, cells changed) | Orin | Orin | § 4.3 | per measure ratio new / baseline ≤ 1.05 with CIs (CI upper > 1.10 doubles N once); speed-up new over S-new CI lower > 1.5, computed on the S-sync subset where present, else all S (Slice 30's rule, `dev/plans/runs/0.8.28-slice-30/summaries/g9.txt` line 17; rerank reported, non-gating); import ≤ S + 5 ms; RSS ≤ S + 32 MiB | 13 | — |
| PERF-02 | G9 Python: P, S | same | Orin | Orin | § 4.3 | P / S ≤ 1.15 per measure; import ≤ S + measured `cuInit` cost + 5 ms (SD-4) | 13 | — |
| PERF-03 | RTX 3090 steady embed: new vs baseline | x86 harness adaptation, Node addon only (new (W6), OP-9 ruling) | GPU | `windchill3`, RTX 3090 by UUID | § 4.4 | median ratio new / baseline ≤ 1.05; bootstrap 95 % CI reported; N = 20 processes per arm, interleaved | 13 | — |

### 3.9 T-REGISTRY (AC31-08)

| ID | Test | File | Tier | Host | Command | Expected | AC31 | Design |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| REG-a | resolution, no `[patch]` table | `scripts/check-tegra-pool-registry-build.sh` (rewritten) | review (W6, R-VERIFY; R-CODE-4 re-checks) | any (`windchill3`) | `bash scripts/check-tegra-pool-registry-build.sh --resolve-only` | consumer lock: `cudarc-fathomdb` 0.19.10, the three Candle crates 0.11.0, `candle-kernels` 0.10.2, each `registry+https://github.com/rust-lang/crates.io-index`; no `cudarc` 0.19.* (`cudarc` 0.17.8 from `ug-cuda` allowed) | 08 | § 3.6 |
| REG-b | build and link on the Orin | same | Orin | Orin | cmd `c-reg-orin` | `cargo build` and `cargo test --no-run` exit 0 with `LIBRARY_PATH=/usr/local/cuda/lib64`; receipt states `CUDA_COMPUTE_CAP`, `CUDA_PATH`, `PATH`, `LIBRARY_PATH` | 08 | § 3.6 |
| REG-c | smoke against that resolution | consumer copy of `tests/tegra_fragmented_va_cuda.rs` + `tests/support/live.rs` | Orin | Orin (window A) | cmd `c-reg-orin` | `forced_cuda_probe_succeeds_when_the_default_memory_pool_is_unavailable`: 1 passed, `FATHOMDB_REQUIRE_LIVE=1`, `FATHOMDB_POOL_MODE=off` (OP-5 ruling); one run with the mode left at `auto` is recorded as information, pass or fail | 08 | § 3.6 |
| REG-d | dynamic `libcudart` statement | receipt; `docs/embedder.md` | review | any | cmd `c-docs` | both state that crates.io consumers get dynamic `libcudart` through upstream `candle-kernels` 0.10.2 and that REQ-068 applies only to FathomDB-built artifacts | 08, 16 | § 3.6 |
| REG-e | Candle half of AC31-08 from the registry | — | review: R-VERIFY cites the Candle FK5 receipt `2026-10-13-FK5-publish-182ceab0.md` (104, 42 and 206 packaged files checked for `candle-core-fathomdb`, `candle-nn-fathomdb` and `candle-transformers-fathomdb`); the command runs once, at R-CODE-4 (proportionality) | any | cmd `c-reg-crates` | each of the three 0.11.0 `.crate` files: every packaged source file equals `git show K9:<path>`; `.cargo_vcs_info.json` sha1 = `K9`, no `dirty`; SHA-256 equal to the `K9` dry run (a mismatch with the file check passing is recorded, not failed) | 08 | requirements AC31-08 |
| REG-f | cudarc half of AC31-08 from the registry | — | review (R-CODE-4) | any | cmd `c-reg-crates` for `cudarc-fathomdb` 0.19.10 | `.cargo_vcs_info.json` sha1 = `C`; SHA-256 `8780d2fe…caf2` | 08 | requirements AC31-08 |

### 3.10 T-REGISTRY script behaviour (rewritten script)

The rewritten `scripts/check-tegra-pool-registry-build.sh` keeps the
tracked-files copy method (lines 73-80, minus `third_party/libsqlite3-sys-0.38.1`)
and builds one consumer, its own workspace root, with **no `[patch]`
table**. Measured on 2026-10-13 in a scratch copy of this tree with the
W6 manifest edits applied by hand (the consumer below; scratch deleted):

```toml
[package]
name = "tegra-pool-registry-consumer"
version = "0.0.0"
edition = "2021"
publish = false

[workspace]

[features]
embed-cuda = ["fathomdb-embedder/embed-cuda"]

[dependencies]
fathomdb-embedder = { path = "../src/rust/crates/fathomdb-embedder", features = ["tegra-pool"] }

[dev-dependencies]
candle-core = { package = "candle-core-fathomdb", version = "=0.11.0" }
```

- `cargo generate-lockfile` resolved `candle-core-fathomdb`,
  `candle-nn-fathomdb`, `candle-transformers-fathomdb` 0.11.0 (checksums
  `b5feacc0…`, `69c7432a…`, `513008c7…`, equal to the FK5 receipt),
  `cudarc-fathomdb` 0.19.10 (`8780d2fe…`) and `candle-kernels` 0.10.2, all
  from crates.io; the only `cudarc` was 0.17.8. That is (a).
- With the copied test's `#![cfg]` switched to `x86_64` in the scratch
  copy only, `cargo check --locked --tests --features embed-cuda` built
  the consumer's integration test against that resolution (39 s, no GPU
  used). The copy compiles as the consumer's own test; on the Orin the
  unchanged `aarch64` file builds in REG-b.
- `cargo test -p fathomdb-embedder --no-run` in the consumer fails:
  "package `fathomdb-embedder` cannot be tested because it requires
  dev-dependencies and is not a member of the workspace", and `--features
  fathomdb-embedder/…` fails with "cannot specify features for packages
  outside of workspace". A non-member path dependency cannot run its own
  tests in the consumer, which is why design § 3.6 copies the test (OP-5).

The script's modes: no argument runs (a), and on an aarch64 Linux Tegra
host also (b) and (c); `--resolve-only` runs (a) alone; it exits 1 on any
failure, never skips silently, and prints the four environment values.

### 3.11 T-FORK-EVIDENCE (R-VERIFY; no code)

Each row: the reviewer checks that the receipt exists at the named fork
commit, names the pinned commit, records a lockfile SHA-256 where it built,
shows unchanged host guards, and that the fork's review approves it with no
open finding. Paths are in each fork's `fathomdb/qualification/` and are
linked by permalink at the receipt commit in `status.md`.

| ID | AC31 | Fork criterion | Receipt (fork) | Check |
| --- | --- | --- | --- | --- |
| FEV-01 | 01 | CF-AC-01 | cudarc `fathomdb/review.md` "R-FC-VERIFY, 2026-10-12"; `fathomdb/status.md` "Code commits since `ec1cac71`" | Tree A `diff -r` empty; 46 code commits; verdict APPROVE-WITH-NOTES, no open note |
| FEV-02 | 02 | CF-AC-02, CF-AC-03, CK-AC-02 | cudarc `2026-10-12-G1-b777f8d.md`, `2026-10-12-G3-13000-b777f8d.md`, `2026-10-12-G6-b777f8d.md`; Candle `2026-10-12-K-G1-K-G3-853324fc.md`, `2026-10-13-CK-AC-07-cd64715e.md`, `2026-10-13-CK-AC-07-182ceab0.md` | `device_on_a_private_pool_context_allocates_from_that_pool` (T1 `cuda_private_pool`) 1/0/0 at `K`, `K8`, `K9` on RTX 3090 and Orin |
| FEV-03 | 03a | CF-AC-04a, 04b, 04c | `2026-10-12-CI-b777f8d.md`; `2026-10-12-G3-<v>-b777f8d.md` for 12060…13040; `2026-10-12-G5a-<v>-b29c41c.md` (carried by R-FC-VERIFY); `2026-10-12-G5b-<v>-b777f8d.md` | CI `head_sha` = `C`; per-test RUN/UNMEASURED accounting present |
| FEV-04 | 03b | CK-AC-05 (K-G2) | Candle `2026-10-12-K-G2-853324fc.md` | carries to `K9`: `git diff 853324fc 182ceab0 -- . ':(exclude)fathomdb' ':(exclude)FATHOMDB.md'` changes only the manifest lines named in AC31-03b; replaced by WSB-02 if S-13 runs |
| FEV-05 | 04 | CF-AC-05b, CK-AC-05 (K-G3), CK-AC-07 | cudarc `2026-10-12-G6-b777f8d.md`; Candle `2026-10-12-K-G1-K-G3-853324fc.md`, `2026-10-13-K-G3-cd64715e.log`, `2026-10-13-K-G3-182ceab0.log` | Orin guard lines unchanged; pass at each commit |
| FEV-06 | 05 | CF-AC-05 | R-FC-VERIFY in cudarc `fathomdb/review.md` | approved |
| FEV-07 | 06 | CF-AC-06, CK-AC-03, CK-AC-06 | cudarc `FATHOMDB.md` items 1-9 at `C`; Candle `FATHOMDB.md` K1-K9; R-FK-VERIFY | items relied on (`pinned-override-rot.json` `items_relied_on`) exist in each table |
| FEV-08 | 07 | CK-AC-01 (through CK-AC-07) | Candle `2026-10-13-CK-AC-07-182ceab0.md` | one `cudarc-fathomdb v0.19.10` node per crate, registry form |
| FEV-09 | 14 | CF-AC-07a, 07b, 09, CK-AC-07, CK-AC-08 | `package-list-b777f8d.txt`, `2026-10-12-CF-AC-07a-b777f8d.md`, `2026-10-13-CF-AC-07b-b777f8d.md`, `2026-10-13-FC10-publish-b777f8d.md`; Candle `2026-10-13-FK5-publish-182ceab0.md` | cmd `c-pin-chain`; no `fathomdb/` path in the package list; 07b equals 07a; `vcs_sha1` of each pin in `pinned-override-rot.json` equals the receipt's commit |
| FEV-10 | 14 | lock-set record (D-16) | cudarc `fathomdb/qualification/<date>-lockset-<FathomDB sha12>.json` (fork todo FT-3) | the record at `fork_record.commit` equals `scripts/cudarc-fathomdb-lock-set.json` minus `fork_record`; `ci_run_id` is a successful run at `ci_head_sha` (`gh run view <id> -R coreyt/cudarc --json conclusion,headSha`) |
| FEV-11 | 15 | CF-AC-11 | `2026-10-12-CI-b777f8d.md` (T-UNIT counts per feature), `2026-10-12-G1-TDEV-b29c41c.md`, `2026-10-12-G5b-TDEV-b777f8d.md` | nonzero counts equal to the fork test plan's; T-DEV (a), (b) pass |
| FEV-12 | 16 | CK-AC-04 (through CK-AC-07) | Candle `2026-10-12-K-G1-K-G3-853324fc.md`, `2026-10-13-CK-AC-07-182ceab0.md` | static CUDART evidence at `K` and `K9` |
| FEV-13 | 09 | CF-AC-08, 08b | — | **held by owner direction (Q17)**: not checked until FC9/FC9b are released; post-merge (R-PROP, R-PROP2) |

### 3.12 T-PROCESS (AC31-11, AC31-12)

| ID | Test | Tier | Host | Command | Expected | AC31 |
| --- | --- | --- | --- | --- | --- | --- |
| PROC-01 | red and green shas per W6 step; each RED re-run by the reviewer | review | `windchill3` | `status.md` W6 table; reviewer re-runs each RED sha's named test and records its failure text | every R1–R14 row has a red sha (or N1's "dropped" record) and a green sha | 11 |
| PROC-02 | reviews recorded with model `opus`; R-REQ, R-DES, R-TEST before the first W6 commit | review | any | cmd `c-review-order` | the commit that introduced the R-TEST row is an ancestor of the first W6 commit (Q19) | 11 |
| PROC-03 | no steward or orchestrator agent | review | any | every agent in `status.md` and `review.md` is an implementer, research, review or Orin agent | holds | 11 |
| PROC-04 | fork process criteria | review | any | cudarc CF-AC-09 (status, review, class check `ec1cac71..C`), Candle CK-AC-08 | recorded passing | 11 |
| PROC-05 | no PR or issue objects on either fork | review | any | `gh pr list -R coreyt/cudarc --state all`; `gh issue list -R coreyt/cudarc --state all`; the same two for `coreyt/candle-fathomdb` | each prints nothing | 12 |
| PROC-06 | upstream push URLs disabled | review | `windchill3` | `git -C /home/coreyt/projects/fathomdb-forks/cudarc remote get-url --push upstream`; `git -C /home/coreyt/projects/fathomdb-forks/candle-fathomdb remote get-url --push upstream` | `DISABLED-never-push-upstream` | 12 |
| PROC-07 | nothing from this slice on `release/0.8.27` | review | any | cmd `c-ancestry` | every slice commit: not an ancestor of `origin/release/0.8.27` | 12 |
| PROC-08 | host guards unchanged in every log | review | any | `grep -h 'driver=' <every guard log>` and the Orin guard lines of every receipt | only `driver=580.178.04 cuda=/usr/local/cuda-12.6`; only R36 REVISION 5.2 and `/usr/local/cuda-12.6`; fork receipts per CF-AC-10, CK-AC-08 | 12 |

### 3.13 Commands

`$S` is the session scratchpad; `<tip>` and `<sha12>` as above. Every
`windchill3` GPU command is preceded by `. dev/plans/runs/0.8.28-slice-31/harness/wc3-env.sh; wc3_guard before-<cell>`
and followed by `wc3_guard after-<cell>`.

- **`c-grep-vendored`** (AC31-10; both ledgers excluded, OP-2; the test
  builds each literal from pieces so it never matches itself):

  ```sh
  git grep -il -e "third_party/""cudarc" -e "test_vendored""_cudarc" -e "vendored"" cudarc" -e "cudarc"" 0.19.7" -- . \
    ':!dev/plans' ':!dev/adr' ':!CHANGELOG.md' \
    ':!dev/todos-and-considerations-ledger.jsonl' ':!dev/steward/steward-ledger.jsonl'
  ```

- **`c-grep-marker`** (design § 3.3):

  ```sh
  marker="fathomdb-private"; marker="$marker-pool"
  git grep -n -e "$marker" -- . ':!dev/plans' ':!dev/adr' ':!CHANGELOG.md' \
    ':!dev/todos-and-considerations-ledger.jsonl' ':!dev/steward/steward-ledger.jsonl'
  ```

- **`c-vendored-gone`:**

  ```sh
  ! grep -n -e 'test-vendored-cudarc' -e 'test_vendored_cudarc' scripts/agent-test.sh \
    && ! test -e scripts/tests/test_vendored_cudarc.sh && ! test -e third_party/cudarc-0.19.7
  ```

- **`c-patch-table`:**

  ```sh
  python3 -I -c 'import tomllib; p=tomllib.load(open("Cargo.toml","rb"))["patch"]["crates-io"]; print(sorted(p)); print(p["candle-kernels"])'
  # expect ['candle-kernels', 'libsqlite3-sys'] and
  # {'git': 'https://github.com/coreyt/candle-fathomdb.git', 'rev': '182ceab0547348993b6dbec4b1aed610dcc8b23f'}
  ```

- **`c-tegra-pool-feature`** (PRD-21; R6's third case):

  ```sh
  python3 -I -c 'import tomllib; f=tomllib.load(open("src/rust/crates/fathomdb-embedder/Cargo.toml","rb"))["features"]; assert f["tegra-pool"] == [], f["tegra-pool"]'
  ```

- **`c-frozen`** (AC31-10):

  ```sh
  base="$(git merge-base origin/release/0.8.28 <tip>)"
  git diff --stat "$base" <tip> -- \
    src/rust/crates/fathomdb-embedder/tests/tegra_fragmented_va_cuda.rs \
    src/rust/crates/fathomdb-embedder/src/cuda_pool_policy_tests.rs \
    src/rust/crates/fathomdb-embedder/tests/slice30_cuda_allocator_api.rs \
    src/rust/crates/fathomdb-engine/tests/slice30_cuda_error_kinds.rs \
    src/rust/crates/fathomdb-engine/src/rerank/tests.rs \
    src/rust/crates/fathomdb-sdk/tests/cuda_errors.rs \
    src/python/tests/test_tegra_pool_early_init.py \
    src/python/tests/test_typed_errors.py \
    src/ts/tests/typed-errors.test.ts
  # expect no output; the named exceptions are other files
  ```

- **`c-docs`:** `grep -n 'cuda_library_unavailable' dev/interfaces/rust.md dev/interfaces/rust-sdk.md dev/interfaces/python.md dev/interfaces/typescript.md docs/embedder.md`;
  `grep -n 'cuda-libraries' dev/interfaces/cli.md docs/reference/cli.md`;
  `grep -n -e '560.28.03' -e 'compute capability 7.5' -e 'nvidia-cublas-cu12' -e 'libcudart' docs/embedder.md`;
  `! grep -n -i 'vendored cudarc' dev/interfaces/rust.md dev/interfaces/rust-sdk.md`.
  Each `grep` must find its text.
- **`c-adr`:** `grep -n 'Amendment 2026-10' dev/adr/ADR-0.8.28-tegra-private-cuda-pool.md dev/adr/ADR-0.8.28-cuda-version-support-policy.md`;
  `grep -n 'Slice 31' dev/adr/ADR-0.6.0-decision-index.md CHANGELOG.md`.
- **`c-wsb-12060`** (`windchill3`, build only; the GPU is not opened):

  ```sh
  cargo build --locked -p fathomdb-embedder --features embed-cuda,rerank-cuda
  cargo build --locked --release -p fathomdb-cli --features embed-cuda,rerank-cuda
  cargo test --locked --no-run -p fathomdb-embedder --features embed-cuda,rerank-cuda
  cargo test --locked --no-run -p fathomdb-engine --features embed-cuda,rerank-cuda
  ```

- **`c-wsb-13000`** (S-13 VM, after `vm-setup-cuda13.sh`):

  ```sh
  export CUDARC_CUDA_VERSION=13000 CUDA_PATH=/usr/local/cuda-13.0 CUDA_HOME=/usr/local/cuda-13.0 \
    PATH=/usr/local/cuda-13.0/bin:$PATH CUDA_COMPUTE_CAP=75
  cargo build --locked -p fathomdb-embedder --features embed-cuda,rerank-cuda
  cargo build --locked --release -p fathomdb-cli --features embed-cuda,rerank-cuda
  cargo test --locked --no-run -p fathomdb-embedder --features embed-cuda,rerank-cuda
  ```

- **`c-s13-run`** (S-13, after `c-wsb-13000`; there is no CLI embed or
  rerank verb, R-TEST finding 1). The live test binaries come from that
  build (`cargo test --locked --no-run -p fathomdb-embedder --features
  embed-cuda,rerank-cuda,loader-test-hooks --message-format=json`; paths
  from the JSON `executable` fields), with the cached model files pushed:

  ```sh
  FATHOMDB_EMBED_DEVICE=cuda:0 target/release/fathomdb doctor cuda-allocator --json
  target/release/fathomdb doctor cuda-libraries --json; echo "exit=$?"
  FATHOMDB_EMBED_DEVICE=cuda:0 FATHOMDB_REQUIRE_LIVE=1 "$CANDLE_BGE_BIN" \
    --exact embed_returns_unit_norm_vector --nocapture --test-threads=1
  FATHOMDB_REQUIRE_LIVE=1 "$EMBEDDER_LIB_BIN" \
    --exact candle_reranker::gpu_tests::gpu_loads_and_scores_finite --nocapture --test-threads=1
  ```

  Each test 1 passed; each process's `/proc/self/maps` CUDA libraries are
  recorded (the D-18 report prints them).
- **`c-c3-libcuda1`** (S-C3, last, after the remedy step of § 4.6; R-TEST
  finding 2): `sudo dpkg-divert --rename --add /usr/lib/x86_64-linux-gnu/libcuda.so`
  (lab VM only, Q15a), then, with the remedy's `LD_LIBRARY_PATH` still
  set, `FATHOMDB_EMBED_DEVICE=cuda:0 FATHOMDB_REQUIRE_LIVE=1 "$CANDLE_BGE_BIN" --exact embed_returns_unit_norm_vector --nocapture`
  (the 12.6 binary built on `windchill3`); `grep libcuda /proc/<pid>/maps`
  during the run (or the D-18 report) shows only `libcuda.so.1`.
- **`c-doctor-x86`:** `FATHOMDB_EMBED_DEVICE=cuda:0 target/release/fathomdb doctor cuda-allocator --json`.
- **`c-doctor-libs-wc3`:**

  ```sh
  target/release/fathomdb doctor cuda-libraries --json >"$S/doctor-libs.json"; echo "exit=$?"
  realpath /usr/local/cuda/targets/x86_64-linux/lib/libcublas.so.12 \
    /usr/local/cuda/targets/x86_64-linux/lib/libcurand.so.10
  # each report path must equal the matching realpath (both under /usr/local/cuda-12.6/)
  ```

- **`c-libmajor-wc3`:**

  ```sh
  flock /tmp/fathomdb-gpu.lock cargo test --locked -p fathomdb-embedder --features embed-cuda \
    --test slice31_cuda_library_major -- --test-threads=1 --nocapture
  ```

  Expected: "5 passed; 0 failed; 1 ignored" (R-TEST's "4 passed; 1
  ignored" plus R12b, added by Q26); the ignored test is
  `missing_cuda_12_cublas_is_a_typed_refusal_with_the_remedy`
  (`#[ignore = "C3 GCP cell"]`, run only in S-C3).
- **`c-prd24`** (PRD-24, build-only on `windchill3`, § 1.1 build
  environment; the GPU is not opened):

  ```sh
  cargo test --locked -p fathomdb-embedder --features embed-cuda --lib cuda_libraries:: -- --test-threads=1
  # expect: test result: ok. 6 passed
  ```

- **`c-fc`** (PRD-33; `windchill3`, once, after the Phase C commits; it
  selects GPUs 0,1 by PCI bus order, OP-3):

  ```sh
  . dev/plans/runs/0.8.28-slice-31/harness/wc3-env.sh; wc3_guard before-fc
  unset CUDA_VISIBLE_DEVICES FATHOMDB_CUDA_GPU_UUID   # the gate sets its own selection
  flock /tmp/fathomdb-gpu.lock bash scripts/test-feature-complete.sh --scratch "$S/fc"
  wc3_guard after-fc
  ```

  The script has no per-crate option (`scripts/lib/feature_complete.py`
  takes only `--scratch` and `--write-matrix`), so it runs every set; the
  receipt lists the `fathomdb-embedder` and `fathomdb-cli` sets at least.
- **`c-review-order`** (PROC-02):

  ```sh
  r=dev/plans/0.8.28/features/slice-31/review.md
  rtest="$(git log --reverse --format=%H -S '| R-TEST |' -- "$r" | head -1)"
  w6="<first W6 commit, from status.md>"
  test -n "$rtest" && git merge-base --is-ancestor "$rtest" "$w6" && echo ordered
  ```

  The pickaxe string is the table row `| R-TEST |`, because the bare text
  `R-TEST` already appears in earlier `review.md` commits ("Next: … →
  R-TEST").
- **`c-tegra-linkage`** (ORIN-15, Orin window A):
  `bash scripts/release/check-tegra-wheel-linkage.sh <extension>` on the
  Tegra wheel's unpacked extension `.so` and on the Tegra N-API `.node`.
- **`c-pool-va`** (Orin):

  ```sh
  flock /tmp/fathomdb-gpu.lock env FATHOMDB_REQUIRE_LIVE=1 cargo test --locked -p fathomdb-embedder \
    --features embed-cuda --test tegra_fragmented_va_cuda -- \
    --exact forced_cuda_probe_succeeds_when_the_default_memory_pool_is_unavailable --nocapture
  ```

- **`c-pool-lib`** (Orin; one process per test name `<t>`):

  ```sh
  flock /tmp/fathomdb-gpu.lock env FATHOMDB_REQUIRE_LIVE=1 FATHOMDB_EMBED_DEVICE=cuda:0 cargo test --locked -p fathomdb-embedder \
    --features embed-cuda,tegra-pool --lib -- --ignored --exact "cuda_pool_policy::tests::<t>" \
    --nocapture --test-threads=1
  ```

- **`c-pool-s31`** (Orin): as `c-pool-lib`, with
  `FATHOMDB_EMBED_DEVICE=cuda:0` as there, and the module path
  `cuda_pool_policy::slice31_tests::<t>`.
- **`c-engine-close`** (Orin):

  ```sh
  flock /tmp/fathomdb-gpu.lock env FATHOMDB_REQUIRE_LIVE=1 cargo test --locked -p fathomdb-engine \
    --features embed-cuda,tegra-pool --test slice31_engine_close_cuda -- --ignored \
    --exact gpu_orin_64_engine_close_with_the_private_pool_then_exit_is_clean --nocapture --test-threads=1
  ```

- **`c-doctor-orin`:** the Tegra CLI (`cargo build --release -p fathomdb-cli --features embed-cuda,rerank-cuda,tegra-pool`),
  `target/release/fathomdb doctor cuda-allocator --json`, 20 processes.
- **`c-node-e2e`** (Orin): build the addon with the recipe in the header
  of `scripts/tests/test_tegra_node_early_cuinit.sh` (`--features
  embed-cuda,rerank-cuda,tegra-pool`), then
  `FATHOMDB_TEGRA_NODE_PACKAGE=<pkg> FATHOMDB_TEGRA_EARLY_CUINIT_RUNS=5 FATHOMDB_REQUIRE_LIVE=1 flock /tmp/fathomdb-gpu.lock bash scripts/tests/test_tegra_node_early_cuinit.sh`.
- **`c-fork`** (Orin): a scratch venv with the Tegra wheel from
  `scripts/release/build-python-cuda-tegra.sh` (features
  `pyo3/extension-module,embed-cuda,rerank-cuda,tegra-pool`), then
  `FATHOMDB_REQUIRE_LIVE=1 flock /tmp/fathomdb-gpu.lock <venv>/bin/python -m pytest -v src/python/tests/test_tegra_pool_fork_after_init.py`
  (5 passed).
- **`c-c1`** (the `OUT_DIR` from Cargo's JSON messages, not a directory
  listing; the build is fresh after `c-wsb-12060`, and Cargo still emits
  `build-script-executed` for it):

  ```sh
  cargo build --locked --release -p fathomdb-cli --features embed-cuda,rerank-cuda --message-format=json \
    | python3 -I -c 'import json,sys; d={m["out_dir"] for m in map(json.loads,sys.stdin) if m.get("reason")=="build-script-executed" and "candle-kernels" in m["package_id"]}; assert len(d)==1, d; print(d.pop())' \
    >"$S/c1-out-dir.txt"
  python3 scripts/check-ptx-driver-floor.py --ptx-dir "$(cat "$S/c1-out-dir.txt")" --target sm_75
  ```

  On the Orin the same with `--features embed-cuda,rerank-cuda,tegra-pool`
  and `--target sm_87`.
- **`c-c2`** (per version `V` in `12.1.3.1` and `12.9.2.10`, the exact
  PyPI versions of `nvidia-cublas-cu12`, listed by `pip index versions`
  on 2026-10-13; scratch venv `$S/c2-$V`; the wheel is CUDA-02's, which
  carries embed and rerank CUDA features):

  ```sh
  python3 -m venv "$S/c2-$V"
  "$S/c2-$V/bin/pip" install "nvidia-cublas-cu12==$V" "$S/preflight-rerank/python-dist/"*.whl
  "$S/c2-$V/bin/pip" freeze >"$S/c2-$V.freeze.txt"
  export LD_LIBRARY_PATH="$("$S/c2-$V/bin/python" -c 'import nvidia.cublas as m; print(m.__path__[0])')/lib"
  target/release/fathomdb doctor cuda-libraries --json; echo "exit=$?"
  FATHOMDB_EMBED_DEVICE=cuda:0 FATHOMDB_RERANK_DEVICE=cuda:0 \
    "$S/c2-$V/bin/python" dev/plans/runs/0.8.28-slice-31/harness/c2-embed-rerank.py
  flock /tmp/fathomdb-gpu.lock cargo test --locked -p fathomdb-embedder --features embed-cuda \
    --test slice31_cuda_library_major -- --exact the_cuda_library_report_names_the_loaded_files --nocapture
  ```

  `c2-embed-rerank.py` (new (W6), in the harness directory, lint-clean
  under `ruff`) opens an engine on a scratch database with the default
  embedder and reranker, writes two documents, runs one search with
  rerank, and prints one `fathomdb-c2 {json}` line with the embedder's and
  reranker's device reports (including `cuda_libraries`) and the CUDA
  lines of `/proc/self/maps`; it exits 1 unless both components report
  `cuda`. cuRAND is not put on `LD_LIBRARY_PATH` here, so it comes from
  the loader cache; the receipt records its path and major 10.
- **`c-reg-orin`** (Orin, on the bundle checkout):

  ```sh
  export CUDA_COMPUTE_CAP=87 CUDA_PATH=/usr/local/cuda-12.6 PATH=/usr/local/cuda-12.6/bin:$PATH \
    LIBRARY_PATH=/usr/local/cuda/lib64 FATHOMDB_REQUIRE_LIVE=1 FATHOMDB_POOL_MODE=off
  flock /tmp/fathomdb-gpu.lock bash scripts/check-tegra-pool-registry-build.sh
  # (c) inside the script: cargo test --locked --features embed-cuda --test tegra_fragmented_va_cuda -- \
  #   --exact forced_cuda_probe_succeeds_when_the_default_memory_pool_is_unavailable --nocapture
  ```

- **`c-reg-crates`:** `curl -sSfL https://crates.io/api/v1/crates/<name>/<version>/download -o <name>-<version>.crate`
  (crates.io, a permitted registry); `sha256sum`; `tar -xzf`;
  `cat <name>-<version>/.cargo_vcs_info.json`; for each packaged file,
  `diff <(git -C /home/coreyt/projects/fathomdb-forks/candle-fathomdb show K9:<path>) <file>`
  (cudarc: `C` in `/home/coreyt/projects/fathomdb-forks/cudarc`). `Cargo.toml` and
  `Cargo.toml.orig` are compared as `cargo package` normalizes them
  (`.orig` against the source).
- **`c-pin-chain`:** in the cudarc clone `/home/coreyt/projects/fathomdb-forks/cudarc`,
  `git diff C origin/fathomdb/0.19.10 -- . ':(exclude)fathomdb'`
  and `git merge-base --is-ancestor C origin/fathomdb/0.19.10`; in the
  Candle clone `/home/coreyt/projects/fathomdb-forks/candle-fathomdb`,
  `git diff K9 origin/fathomdb/0.8.28-cudarc-fork -- . ':(exclude)fathomdb' ':(exclude)FATHOMDB.md'`
  and `git merge-base --is-ancestor <X> origin/fathomdb/0.8.28-cudarc-fork`
  for `X` in `K`, `K8`, `K9`; the commit-class checks of each fork plan.
- **`c-ancestry`** (the slice's own commits, `<slice base>..<tip>`):

  ```sh
  base="$(git merge-base origin/release/0.8.28 <tip>)"
  for c in $(git rev-list "$base..<tip>"); do
    git merge-base --is-ancestor "$c" origin/release/0.8.27 && echo "LEAK $c"
  done
  ```

  prints nothing (run for the slice and post branches).

## 4. Qualification runs (`qualification.md`, W6)

All on the merged-forward tip (plan § 9 W6 step 4). One receipt per cell
group; `qualification.md` gives each gate's verdict against the criterion
as written, with the numbers, as Slice 30's did.

### 4.1 T-CUDA on the RTX 3090 (`windchill3`)

Receipt `<date>-T-CUDA-3090-<sha12>.md`: CUDA-01 to CUDA-07 and CUDA-11,
CMP-C1b (`sm_75`), CMP-C2 (cuBLAS 12.1.3.1 and 12.9.2.10, each venv's
`pip freeze`), WSB-01, PRD-24 (cmd `c-prd24`, quoted), the guard log, the
3090 UUID, the artifact SHA-256s (`inspect-cuda-artifacts.py` output for
AC31-16's x86 half). The feature-complete run (PRD-33, cmd `c-fc`) has
its own receipt `<date>-FC-<sha12>.md`.

### 4.2 T-ORIN: Slice 30 gate rows G1–G10 on the new base

Artifacts built on the Orin from the bundle checkout of `<tip>`
(`slice/0.8.28-31-cudarc-fork`): the Node addon (`embed-cuda,rerank-cuda,tegra-pool`,
packed as Slice 30 did), the Tegra Python wheel from
`scripts/release/build-python-cuda-tegra.sh` (its feature set now
includes `rerank-cuda`, so Slice 30's second wheel W2 is no longer needed:
`PY_W1` and `PY_W2` name the same venv), and the Tegra CLI. AC31-16's
Tegra half is ORIN-15 (cmd `c-tegra-linkage`).

Harness: `dev/plans/runs/0.8.28-slice-31/harness/` (a copy of Slice 30's;
lint-clean under `scripts/agent-lint-shell.sh`), `SCRATCH`, `NODE_PROD`,
`PY_W1`, `PY_W2`, `FDB_CLI` set as in its `run-gates.sh` header.

**Windows (R-TEST ruling OP-11, and finding 14 for window P).** Each
window is one locked GPU session under § 1.2's guard, and its receipt
names the window.

| Window | When | Cells | Budget |
| --- | --- | --- | --- |
| P | at the pin commit (Phase B), before N1 is written | R14 (N1 RED), first characterization runs of R15 (TDN-01, TDN-02) and R16 (TDN-03) | about 20 min |
| A | after every Orin build of a W6 tip (the merged-forward tip last) | POOL-01…05, TDN-01…10 (F1–F5 included), ORIN-12, ORIN-13, ORIN-14, ORIN-15, REG-b, REG-c, CMP-C1b (`sm_87`) | about 40 min |
| B | on the merged-forward tip | G1–G8 (G1 includes the doctor check, revision 1's ORIN-11) | about 1.5 h |
| C | last, on the merged-forward tip | G9 (§ 4.3) with `QUAL_QUIET_STRICT=1` and `NO_SWAP=1`; no `cargo` or `rustc` process running on the Orin (Slice 30 deviation D-4) | about 1.3 h, plus about 1 h if a CI upper bound doubles N |

| Gate | Command | Pass (Slice 30 plan § 4.2, with the 2026-10-08 rulings) | Receipt |
| --- | --- | --- | --- |
| G1 | `bash run-gates.sh g1`; cmd `c-doctor-orin` ×20 | 20/20 Node and Python private, 3 GiB, threshold `"0"`, `ran`; doctor 20/20 identical (`"path":"private"`, `"reason":"private_pool"`, `"pool_max_size_bytes":3221225472`, `"release_threshold":"0"`, `"module_load_init":"ran"`) | `<date>-T-ORIN-G1-<sha12>.md` |
| G2 | `bash run-gates.sh g2` | never refuses; 0.8.27 path; reason reported (H-1 now TDN-06…10) | `…-G2-…` |
| G3 | `bash run-gates.sh g3` and `g3s` | all cycles private; median VmRSS growth ≤ 0.400 MiB/cycle, max ≤ 0.44; `reserved_cur` clause UNMEASURED at product level (§ 1.4), with TDN-02 and POOL-05 as the in-process evidence | `…-G3-…` |
| G4 | `bash run-gates.sh g4` | typed `CudaPoolExhaustedError`; next embed on `cuda` with the pre-error hash | `…-G4-…` |
| G5 | `bash run-gates.sh g5` | **gating:** identical hash and rerank scores P vs S. **Report-only (owner ruling Q28, steward ledger seq 331):** equality to Slice 30's values (`d9dafb8c410005f3`, rerank `[9.954336737870562e-06, 0.9979814299391018]`) is recorded and evaluated after the work, in `qualification.md`; a difference is reported, not a failure | `…-G5-…` |
| G6 | `bash run-gates.sh g6` | private at 4M and 8M objects, hash unchanged | `…-G6-…` |
| G7 | `bash run-gates.sh g7` | 2/4/8 processes × 5, no failure or OOM kill | `…-G7-…` |
| G8 | `bash run-gates.sh g8` and `g8s` | no allocator error; RSS within 10 % (Node judged against the pool-off control as the owner ruled for Slice 30) | `…-G8-…` |
| G9 | § 4.3 | AC31-13 | `…-T-PERF-ORIN-…` |
| G10 | POOL-06 (in PRD-01) and FEV-05 | tests pass | (gate log) |

GPU budget: about 4.5 h locked in all (P 20 min, A about 40 min per
build, B about 1.5 h, C about 1.3 h), plus about 1 h of contingency in C
for doubling N.

### 4.3 T-PERF on the Orin (AC31-13 = G9 in full)

**Arms.** *new*: the Node addon and Tegra wheel built from `<tip>`;
*baseline*: the same artifacts built from `<slice base>` (the Slice 30
product on vendored cudarc 0.19.7; the pre-slice build); *S*: each build
with `FATHOMDB_POOL_MODE=off`. The Slice 30 study-P cell and its
`FATHOMDB_POOL_VARIANT` env are removed from the cell file; baseline runs
with no pool settings.

`g9-node.tsv` (harness copy):

```text
newP node 25.9.0 $NODE_NEW $NODE_NEW/node_modules/fathomdb-linux-arm64-gnu/fathomdb.linux-arm64-gnu.node CONSUMER_MODE=perf
baseP node 25.9.0 $NODE_BASE $NODE_BASE/node_modules/fathomdb-linux-arm64-gnu/fathomdb.linux-arm64-gnu.node CONSUMER_MODE=perf
Snew node 25.9.0 $NODE_NEW $NODE_NEW/node_modules/fathomdb-linux-arm64-gnu/fathomdb.linux-arm64-gnu.node CONSUMER_MODE=perf,FATHOMDB_POOL_MODE=off
```

Python: `pyP` and `pyS` (pool off and early `cuInit` off) on the new
wheel; import cells `pyimpP`, `pyimpS`, `pyimpI` as Slice 30.

**Design.** N = 20 processes per arm in interleaved randomised blocks
(`interleave.sh <cells> 20 <seed> <outdir>`, seeds recorded), `NO_SWAP=1`,
strict quiet check `QUAL_QUIET_STRICT=1` (the harness variable;
`run-gates.sh:21` defaults it to 0, so window C sets it), and no `cargo`
or `rustc` running on the Orin; per process: import time, first embed,
5 warm-up + 50 timed steady embeds, batches 1/8/32/128 (5 + 50 each),
5 + 50 reranks. Statistic: median over processes of each process's median;
interval: 95 % percentile bootstrap, 10 000 resamples, process as the unit
(`analyze.py g9`).

**Thresholds (gating unless marked).**

| Measure | Criterion |
| --- | --- |
| Node new / baseline, per measure (embed, batches 1/8/32/128, rerank) | point ratio ≤ 1.05; CIs reported; if a CI upper bound > 1.10, double N once (to 40) and report, not doubled again |
| Node speed-up new over Snew, embed and batches | CI lower bound > 1.5, on the S-sync subset of Snew where present, else all of Snew (Slice 30's rule, `summaries/g9.txt` line 17: "S-sync subset where present, else all S") |
| Node speed-up new over Snew, rerank | measured and reported, **non-gating** (Slice 30 revision 3.3) |
| Node import | new ≤ Snew + 5 ms |
| Node peak RSS | new ≤ Snew + 32 MiB |
| Python P / S, per measure | ≤ 1.15 |
| Python import | P ≤ S + measured `cuInit` cost (`pyimpI` − `pyimpS`) + 5 ms (SD-4) |

Receipt `<date>-T-PERF-ORIN-<sha12>.md` with `summaries/g9.txt`.

### 4.4 T-PERF on the RTX 3090 (`windchill3`)

Arms *new* (`<tip>`) and *baseline* (`<slice base>`): the x86 Node CUDA
addon built from each commit in its own copy with `npm run
build:native:cuda` in `src/ts` (`scripts/release/build-napi-cuda.sh`, the
release contract's features and toolkit; OP-9 ruling: Node addon only, no
Python arm). Cells `newP` and `baseP`, `CONSUMER_MODE=perf`,
`FATHOMDB_EMBED_DEVICE=cuda:0`, `CUDA_VISIBLE_DEVICES=<3090 UUID>`; N = 20
processes per arm in interleaved randomised blocks; statistic and bootstrap
as § 4.3. Pass: median steady-embed ratio new / baseline ≤ 1.05, with the
95 % CI reported. The harness's host-quiet and GPU-idle checks read
Tegra sysfs today (`lib-host.sh:12, 38`); W6's adaptation checks GPU
idleness with `nvidia-smi --id=<3090 UUID>` (utilization and no compute
process) and drops the nvgpu file-descriptor check, which has no meaning
on a discrete GPU (OP-9 ruling). Receipt `<date>-T-PERF-3090-<sha12>.md`.

### 4.5 D-14 stages and the usable capacity (Orin)

- **D-14** (ORIN-14): one process, `FATHOMDB_EMBED_DEVICE=cuda:0`, early
  `cuInit` run, cached weights. At S0 (after `run_module_load_early_init`),
  S1 (first `new_cuda_device(0)`), S2 (`CandleBgeEmbedder::new()`), S3
  (5 warm-up embeds), S4 (50 timed embeds), S5 (embedder dropped): after a
  synchronize of the decision's first context, one line `fathomdb-d14
  {"stage":"S<n>","vm_rss":…,"rss_anon":…,"rss_file":…,"rss_shmem":…,"pool":{"reserved_current":…,"used_current":…,"reserved_high":…},"default_pool":{"reserved_current":…,"used_high":…}}`.
  The receipt tabulates outside-pool memory `ΔVmRSS(S0→Sn) −
  Δreserved_current(S0→Sn)` with `ΔRssFile` shown separately, and the
  CPU-embedder control run for S2–S5. Reported, not gated; the default-pool
  leak check is asserted. Receipt `<date>-D14-<sha12>.md`.
- **Usable capacity** (ORIN-13): allocate 32 MiB tensors on the private
  pool until `cuda_pool_exhausted`; print `fathomdb-capacity
  {"max_size":3221225472,"usable_bytes":…,"chunks":…}`. Expected near
  1024 MiB (the fork's fitted formula). **Below 576 MiB (twice Slice 30's
  CB1 peak of 288 MiB), sizing returns to the owner** (design § 13 ruling
  2); `qualification.md` states the number and whether the trigger fired.
  Receipt `<date>-CAP-<sha12>.md`.

### 4.6 T-CUDA-COMPAT C3 session (S-C3, owner-approved)

Built on `windchill3` from `<tip>` (§ 1.1 environment, no GPU opened),
paths from `--message-format=json` (`executable`):

- `cargo test --locked --no-run -p fathomdb-embedder --features embed-cuda --test slice31_cuda_library_major`;
- the live test binaries: `cargo test --locked --no-run -p fathomdb-embedder --features embed-cuda,loader-test-hooks --test candle_bge`
  and `cargo test --locked --no-run -p fathomdb-embedder --features rerank-cuda --lib`
  (for `candle_reranker::gpu_tests::gpu_loads_and_scores_finite`);
- the release CLI sealed with `scripts/release/seal-cuda-cli-archive.sh`
  (doctor verbs only: no CLI embed or rerank exists);
- the cached BGE and reranker model files.

```sh
FDB_LAB_PROJECT=<project> scripts/gcp-cuda-lab.sh create --profile t4 --name fdb-lab-c3 --zone <z> --max-run 2h
scripts/gcp-cuda-lab.sh wait-ssh --name fdb-lab-c3 --zone <z>
scripts/gcp-cuda-lab.sh driver --name fdb-lab-c3 --zone <z> --driver 580
scripts/gcp-cuda-lab.sh push --name fdb-lab-c3 --zone <z> "$S/c3-bundle" c3
scripts/gcp-cuda-lab.sh run --name fdb-lab-c3 --zone <z> -- bash c3/run-c3.sh refusal
scripts/gcp-cuda-lab.sh pull --name fdb-lab-c3 --zone <z> c3/out "$S/c3-out"
scripts/gcp-cuda-lab.sh run --name fdb-lab-c3 --zone <z> -- bash c3/run-c3.sh remedy
scripts/gcp-cuda-lab.sh pull --name fdb-lab-c3 --zone <z> c3/out "$S/c3-out"
scripts/gcp-cuda-lab.sh run --name fdb-lab-c3 --zone <z> -- bash c3/run-c3.sh libcuda1
scripts/gcp-cuda-lab.sh pull --name fdb-lab-c3 --zone <z> c3/out "$S/c3-out"
scripts/gcp-cuda-lab.sh destroy --name fdb-lab-c3 --zone <z>
```

`run-c3.sh` (new (W6), in the harness directory) takes one step name and
records the preconditions on every step (`nvidia-smi` shows `Tesla T4`,
driver 580.x, compute capability 7.5; `ldconfig -p | grep -c libcublas`
is 0; `find / -xdev -name 'libcublas*'` empty before the remedy; no
`/usr/local/cuda*`; `LD_LIBRARY_PATH` unset before the remedy). Receipts
are pulled after each step (Q27's rule applies to every session).

- **`refusal`** (design § 9.3 steps 3-4): `FATHOMDB_REQUIRE_LIVE=1
  <slice31_cuda_library_major binary> --ignored --exact
  missing_cuda_12_cublas_is_a_typed_refusal_with_the_remedy` (1 passed:
  forced `cuda:0` refused with `cuda_library_unavailable`, `auto` on CPU
  with that reason, one stderr line); `fathomdb doctor cuda-libraries
  --json` exit 70; `fathomdb doctor gpu --json` and `fathomdb doctor
  reranker-gpu --json` are recorded, and neither reports
  `cuda_incompatible` (the floor boundary on the 7.5 T4); `FATHOMDB_EMBED_DEVICE=auto
  FATHOMDB_REQUIRE_LIVE=1 <candle_bge binary> --exact
  embed_returns_unit_norm_vector` passes on CPU.
- **`remedy`** (step 5): a venv with `pip install
  nvidia-cublas-cu12==12.6.4.1 nvidia-curand-cu12==10.3.7.77` and its
  `pip freeze`; `LD_LIBRARY_PATH` exactly as the remedy text (both
  `nvidia/cublas/lib` and `nvidia/curand/lib`); `doctor cuda-libraries`
  exit 0, cuBLAS `loaded` from the venv's `nvidia/cublas/lib`, cuRAND
  (major 10) from `nvidia/curand/lib`; then the forced live tests:
  `FATHOMDB_EMBED_DEVICE=cuda:0 FATHOMDB_REQUIRE_LIVE=1 <candle_bge
  binary> --exact embed_returns_unit_norm_vector` and
  `FATHOMDB_REQUIRE_LIVE=1 <lib test binary> --exact
  candle_reranker::gpu_tests::gpu_loads_and_scores_finite`, each 1 passed
  on the T4.
- **`libcuda1`** (CUDA-09, cmd `c-c3-libcuda1`; last, because
  `dpkg-divert` changes the VM): divert `libcuda.so`, repeat the forced
  embed with the remedy's `LD_LIBRARY_PATH`, and record that only
  `libcuda.so.1` is mapped.
- Optionally, if time remains before the cap, the CUDA 13 wheel step of
  design § 9.3 step 6, in a separate venv.

Receipt `<date>-C3-<sha12>.md`.

### 4.7 S-13 session (owner-approved)

As § 4.6 with `--name fdb-lab-s13` and `--machine-type n1-standard-8`
(or a larger `n1`) for **one** build attempt (owner ruling Q27; many
attempts to obtain an available VM are allowed; the 2 h cap holds; each
session needs the owner's go-ahead). The 13.0 toolkit is installed on the
VM by the Candle fork's `vm-setup-cuda13.sh` and the source bundle is
pushed and built there (test and verification only). Cells in order, a
`pull` of the receipts after each: WSB-02, CUDA-08, CMP-C5 (cmd
`c-s13-run` for the last two). The `libcuda.so.1`-only cell runs in S-C3
(CUDA-09), because this VM has only the 13.0 cuBLAS and the 12.6 build
would refuse. Receipt `<date>-S13-<sha12>.md`.

## 5. Traceability

### 5.1 AC31 → tests → review

| AC31 | Tests | Review |
| --- | --- | --- |
| 01 | FEV-01 | R-VERIFY |
| 02 | FEV-02, POOL-01…05, PRD-01 (the Slice 30 product tests), CUDA-01, CUDA-02, CUDA-03 | R-VERIFY |
| 03a | FEV-03 | R-VERIFY |
| 03b | FEV-04, WSB-01, WSB-02, CUDA-08, CUDA-09, PRD-08, PRD-14 | R-VERIFY |
| 04 | FEV-05, ORIN-01…09 (G5's Slice 30 equality report-only, Q28), ORIN-12, ORIN-13, POOL-04, POOL-05, POOL-06, TDN-01…03, TDN-06…10 | R-VERIFY |
| 05 | FEV-06 | R-VERIFY |
| 06 | FEV-07, PRD-11, PRD-17, PRD-19, PRD-21, PRD-31, ORIN-14 | R-DES (done), R-VERIFY |
| 07 | PRD-02…10, FEV-08 | R-PIN, R-VERIFY |
| 08 | REG-a…f | R-VERIFY (a-e), R-CODE-4 (a, e, f re-checked from the registry) |
| 09 | FEV-13 (held, Q17) | R-PROP, R-PROP2 |
| 10 | PRD-01, PRD-16, PRD-18, PRD-26, PRD-27, PRD-28, PRD-30, PRD-33 | R-CODE-3, R-VERIFY |
| 11 | PROC-01…04 | R-VERIFY |
| 12 | PROC-05…08 | R-VERIFY (so far), R-CODE-4 (final) |
| 13 | PERF-01, PERF-02, PERF-03 | R-VERIFY |
| 14 | FEV-09, FEV-10, PRD-10, PRD-12, PRD-13, PRD-21 | R-PIN, R-VERIFY |
| 15 | CMP-C1b, CMP-C2, CMP-C3, CMP-C5, CMP-DOC, FEV-11, PRD-22…25, PRD-32, PRD-33, CUDA-04…07, CUDA-11 | R-DES (done), R-TEST (done), R-VERIFY |
| 16 | PRD-15, PRD-19, PRD-20, CUDA-01 (x86 inspector), ORIN-15 (Tegra linkage), FEV-12, REG-d | R-PIN, R-VERIFY |

### 5.2 Plan § 7 topic → tests

| Topic | Tests |
| --- | --- |
| T-WS-BUILD | WSB-01, WSB-02 |
| T-POOL-PRODUCT | POOL-01…06 |
| T-CUDA | CUDA-01…09, CUDA-11 |
| T-CUDA-COMPAT C1–C5 | CMP-C1b, CMP-C2, CMP-C3, CMP-C5, CMP-DOC; cross-referenced: PRD-22 (C1 fixtures), PRD-23 (C4), PRD-24 (D-18 hermetic) |
| T-PRODUCT | PRD-01…28, PRD-30…33 |
| T-ORIN | ORIN-01…09, ORIN-12…15 |
| T-TEARDOWN-PRODUCT | TDN-01…03, TDN-06…10; cross-referenced: POOL-04, ORIN-12 |
| T-PERF | PERF-01…03 |
| T-FORK-EVIDENCE | FEV-01…13 |
| T-PROCESS | PROC-01…08 |
| T-REGISTRY | REG-a…f |

Every test ID above appears in § 5.1 under at least one AC31, and every
AC31 has at least one test; AC31-09 is covered only by the held fork
criteria (Q17). Revision 1's IDs PRD-29, CUDA-10, ORIN-10, ORIN-11,
TDN-04, TDN-05, CMP-C1a, CMP-C4 and CMP-D18 are retired (merged or
cross-referenced, R-TEST proportionality) and are not reused.

### 5.3 Thin coverage, stated

- **AC31-01, 03a, 05** are fork-backed only (FEV rows): FathomDB runs no
  test of its own, by design (I-18).
- **AC31-09** is held (Q17): no check runs in this slice before FC9/FC9b
  are released.
- **AC31-03b's 13.0 half and AC31-15's C3 and C5** rest on owner-approved
  VM sessions; without them only the carried K-G2 and the hermetic tests
  remain.
- **AC31-13's RTX 3090 arm** needs a harness adaptation that W6 writes
  (in scope by the OP-9 ruling, Node addon only).
- **REQ31-15's 12.1–12.9 range** rests on two C2 cells (12.1.3.1 and
  12.9.2.10) and the 12.6 toolkit; 12.4 was dropped for proportionality.
- **AC31-04's engine-close evidence** has no pool-counter clause at the
  engine level (OP-4); TDN-02 and POOL-05 carry the counter evidence at the
  embedder level.

## 6. Open points and their R-TEST rulings

Revision 1 raised eleven open points; R-TEST ruled on each, and this
revision applies the rulings.

1. **OP-1 Release-contract Candle cohort.** Ruled: the release contract
   keeps only the `candle-kernels` git patch at `K9` and its single git
   lock entry; it reads the `K9` rev from `scripts/pinned-override-rot.json`,
   not a third literal; its arms assert messages (`expect_fail_with`).
   Applied in R5, PRD-15 and design § 1 and § 11 (revision 3).
2. **OP-2 AC31-10's `git grep` excludes the steward ledger.** Ruled:
   amend AC31-10 to exclude `dev/steward/steward-ledger.jsonl`. Applied in
   `requirements.md` and `c-grep-vendored`.
3. **OP-3 The feature-complete gate and Q22.** Ruled: the gate keeps
   indices 0 and 1 with `CUDA_DEVICE_ORDER=PCI_BUS_ID`; the K620 at
   `61:00.0` is index 2 and excluded. The C3-only test is `#[ignore = "C3
   GCP cell"]` and allowlisted `ignored-by-design` (R-TEST finding 5), so
   no opt-in variable is needed. Moving the gate to UUID selection is a
   todo for the main thread to ledger, not W6 work (§ 1.1).
4. **OP-4 Engine-level counters.** Ruled: TDN-03 has no counter clause;
   TDN-02 and POOL-05 carry the counter evidence.
5. **OP-5 T-REGISTRY smoke.** Ruled: REG-c runs with
   `FATHOMDB_POOL_MODE=off`; one run with `auto` is recorded as
   information.
6. **OP-6 Where C1 runs.** Confirmed: the fixture arms in the fast tier
   (PRD-22) and the real PTX check on the `windchill3` and Orin build
   output (CMP-C1b).
7. **OP-7 F4 and a RED mutation.** Ruled: no F4 mutation.
8. **OP-8 N1's RED condition.** Ruled: on the pinned tree, as written,
   plus the Orin window P at the pin commit (finding 14, § 4.2).
9. **OP-9 The x86 T-PERF harness.** Ruled: in scope, Node addon only;
   each arm built with `npm run build:native:cuda`
   (`scripts/release/build-napi-cuda.sh`); the GPU-idle check uses
   `nvidia-smi --id=<UUID>`; the nvgpu file-descriptor check is dropped
   (§ 4.4).
10. **OP-10 Lock-set guard needs the lock.** Ruled: `--lock` (R3).
11. **OP-11 Orin budget and order.** Ruled: three windows, A (after every
    build: POOL, TDN, F1–F5, ORIN-12–14, REG-b/c, C1b), B (G1–G8) and C
    (last: G9 with `QUAL_QUIET_STRICT=1`, `NO_SWAP=1`, no `cargo` or
    `rustc` running), with about 1 h extra for doubling N (§ 4.2).
    Revision 1's ORIN-11 is merged into G1 (proportionality), so it runs
    in window B, not A.

## Appendix A. Single-cudarc guard fixtures

Verified on 2026-10-13 with Cargo 1.95.0 in a scratch directory (deleted):
each tree fixture's lock was generated by `cargo generate-lockfile
--offline`, `cargo metadata --offline --locked` accepted each, and `cargo
tree --offline --locked -e normal --target x86_64-unknown-linux-gnu -p
fathomdb-embedder --features <set> --prefix none` printed the lines quoted
below (the workspace member's own line, which carries its path, is not
inspected by the guard).

### A.1 Shared files of every tree fixture `tree/<name>/`

`Cargo.toml`:

```toml
[workspace]
resolver = "2"
members = ["crates/fathomdb-embedder"]
```

`.cargo/config.toml`:

```toml
[source.crates-io]
replace-with = "fixture-vendor"

[source.fixture-vendor]
directory = "vendor"
```

`crates/fathomdb-embedder/src/lib.rs`: empty.

`scripts/pinned-override-rot.json`:

```json
{
  "cargo_fork_registry_pins": {
    "pins": [
      {"dependency": null, "package": "cudarc-fathomdb", "version": "0.19.10"}
    ]
  }
}
```

`vendor/cudarc-fathomdb-0.19.10/Cargo.toml` (and an empty `src/lib.rs`):

```toml
[package]
name = "cudarc-fathomdb"
version = "0.19.10"
edition = "2021"

[lib]
name = "cudarc"
path = "src/lib.rs"

[features]
cuda-12060 = []
```

`vendor/cudarc-fathomdb-0.19.10/.cargo-checksum.json`:

```json
{"files":{},"package":"8780d2fe3fbafc4660e6093abd7681cd9cec7b4d5549a20bc148308e6449caf2"}
```

Where a fixture needs upstream `cudarc`, `vendor/cudarc-0.19.4/` has the
same shape with `name = "cudarc"`, `version = "0.19.4"` and the package
checksum `1111111111111111111111111111111111111111111111111111111111111111`
(a stand-in; directory sources check only the listed files).

The embedder manifest `crates/fathomdb-embedder/Cargo.toml` starts:

```toml
[package]
name = "fathomdb-embedder"
version = "0.0.0"
edition = "2021"
publish = false
```

### A.2 Per-fixture embedder dependencies and features

| Fixture | `[dependencies]` | `[features]` | `embed-cuda` tree lines (trailing `(*)` mark removed) | Guard |
| --- | --- | --- | --- | --- |
| `tree/ok/` | `cudarc = { package = "cudarc-fathomdb", version = "=0.19.10", optional = true }` | `embed-cuda = ["dep:cudarc"]`, `rerank-cuda = ["dep:cudarc"]`, `tegra-pool = []` | `cudarc-fathomdb v0.19.10` | C, D pass |
| `tree/upstream-cudarc/` | as `ok` plus `upstream-cudarc = { package = "cudarc", version = "=0.19.4", optional = true }` | `embed-cuda = ["dep:cudarc", "dep:upstream-cudarc"]`, others as `ok` | `cudarc v0.19.4`, `cudarc-fathomdb v0.19.10` | C fails |
| `tree/path-cudarc-fathomdb/` | `cudarc = { package = "cudarc-fathomdb", path = "../../path/cudarc-fathomdb", optional = true }`; the stand-in at `path/cudarc-fathomdb/` (no `.cargo-checksum.json`); no vendored `cudarc-fathomdb` | as `ok` | `cudarc-fathomdb v0.19.10 (<fixture>/path/cudarc-fathomdb)` | C fails |
| `tree/no-cudarc-fathomdb/` | as `ok` | `embed-cuda = []`, `rerank-cuda = ["dep:cudarc"]`, `tegra-pool = []` | none | C fails (non-vacuity) |
| `tree/flash-attn-only/` | as `upstream-cudarc` | as `ok` plus `flash-attn = ["dep:upstream-cudarc"]` | `cudarc-fathomdb v0.19.10` | C passes |
| `tree/concrete-feature/` | as `ok` | `embed-cuda = ["dep:cudarc", "cudarc/cuda-12060"]`, others as `ok` | `cudarc-fathomdb v0.19.10` | D fails: `cudarc-fathomdb feature "cuda-12060"` |

For `tree/ok/` the D command (`cargo tree --offline --locked -e features
-i cudarc-fathomdb --target <t> --features
fathomdb-embedder/embed-cuda,fathomdb-embedder/rerank-cuda,fathomdb-embedder/tegra-pool
--prefix none`) printed only `cudarc-fathomdb feature "default"` as a
feature line of the root; for `tree/concrete-feature/` it added
`cudarc-fathomdb feature "cuda-12060"`.

Generation, for each tree fixture: `(cd scripts/tests/fixtures/single-cudarc/tree/<name> && cargo generate-lockfile --offline)`;
the `Cargo.lock` is committed. The generated `tree/ok/Cargo.lock` was:

```toml
# This file is automatically @generated by Cargo.
# It is not intended for manual editing.
version = 4

[[package]]
name = "cudarc-fathomdb"
version = "0.19.10"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "8780d2fe3fbafc4660e6093abd7681cd9cec7b4d5549a20bc148308e6449caf2"

[[package]]
name = "fathomdb-embedder"
version = "0.0.0"
dependencies = [
 "cudarc-fathomdb",
]
```

### A.3 Lock fixtures `lock/<name>.lock`

`lock/ok.lock` is byte-identical to `tree/ok/Cargo.lock` above.
`lock/upstream-cudarc.lock` is `ok.lock` with this block appended (and
`"cudarc"` added to the embedder's `dependencies` list before
`"cudarc-fathomdb"`):

```toml

[[package]]
name = "cudarc"
version = "0.19.4"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "1111111111111111111111111111111111111111111111111111111111111111"
```

`lock/ug-cudarc.lock` is the same with `version = "0.17.8"`. Condition A
parses them with `tomllib` and needs no workspace.
