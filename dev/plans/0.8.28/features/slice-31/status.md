# Slice 31 status

Step log for [plan.md](plan.md) (revision 18; its § 8.1 and the
"Update 2026-10-13" parts for revisions 16 to 18 give the slice state on
2026-10-13). Red and green shas per
FathomDB step, merge-forward shas, host guards, the fork commits `C` and
`K`, and the pin-chain checks (plan § 4). Fork-side step logs are in each
fork's `fathomdb/status.md`.

## Plan and fork plans

| Item | Commit |
| --- | --- |
| Slice 31 plan revision 14, `GOAL.md` | `80ae2a18e` on `release/0.8.28` |
| Slice 31 plan revision 15 (§ 8.1 state) | `eb75e31d5` on `release/0.8.28`, merged into this branch |
| Merge-forward of `release/0.8.27` (W0 step 2) | `9fa71e93a` |
| cudarc fork plan | `coreyt/cudarc` `fathomdb/0.19.10`, `fathomdb/plan.md` (tip `d3a5a055`) |
| Candle fork plan | `coreyt/candle-fathomdb` `fathomdb/0.8.28-cudarc-fork`, `fathomdb/plan.md` (tip `549f3548`) |

## W0 (prerequisites)

| Step | State (2026-10-13) |
| --- | --- |
| 1 Plan, GOAL, merge-forward plan | done (revision 14 at `80ae2a18e`) |
| 2 Merge-forward | done at `9fa71e93a`; repeat before R-CODE-3 if `release/0.8.27` advances |
| 3 Preflight, slice worktree, `status.md`, `review.md` | done: worktree `../fathomdb-worktrees/slice-0.8.28-31` on `slice/0.8.28-31-cudarc-fork` from `origin/release/0.8.28` at `80ae2a18e`. `scripts/preflight.sh --worktree … --min-disk-gb 15`: disk 76 GB free and the checks pass except one hard failure, "release-state discovery failed: no live release": no 0.8.28 release-state file exists yet (plan header: this plan is D28-13's working plan until the 0.8.28 plan, state file and board exist). Recorded, not worked around |
| 4 Toolchains; CUDA 13.0 | `windchill3` guard: driver 580.178.04, `/usr/local/cuda` → 12.6 (every fork receipt records it before and after). The runfile download was not approved; the 13.0 toolkit cells run on a GCP VM (ruling Q15) |
| 5 Orin check | done during the fork cells (L4T R36.5.2, 12.6, `~/fathomdb-forks/`) |
| 6 Slice `.venv` via `scripts/bootstrap.sh` | done 2026-10-13 in the slice worktree: `bash scripts/bootstrap.sh` exit 0, `.venv` on Python 3.12.3 with `src/python[dev]` editable; gitleaks, actionlint 1.7.12, shellcheck 0.11.0 and lychee already present, no apt step. Its `npm install` in `src/ts` reordered two keys in `src/ts/package-lock.json`; restored, not committed |
| 7 Ledger | done in `80ae2a18e`: steward ledger seq 303–317, todos seq 292–299 |

## Fork commits and the pin chain

| Fork | Qualified commit | Reviews | Pin-chain check (2026-10-13) |
| --- | --- | --- | --- |
| cudarc | `C` = `b777f8d94eedca8334e9331cf520d554b22544b7` | R-FC-VERIFY APPROVE-WITH-NOTES, no open finding | `git diff C a27a9b9 -- . ':(exclude)fathomdb'` is only `.github/workflows/fathomdb-ci.yaml` (FT-3, class W, CI-only; run 38491361439 success at `a27a9b9`; record `7990a37`); CI receipt `2026-10-12-CI-b777f8d.md` has `head_sha` = `C` |
| Candle | `K` = `853324fc2b9f813eedceb75bef660a06a413ff5b` | R-FK-VERIFY APPROVE-WITH-FIXES ("fit to pin"), all findings disposed | `K`'s root `Cargo.toml` names `cudarc-fathomdb` at `https://github.com/coreyt/cudarc.git`, `rev = "b777f8d94eedca8334e9331cf520d554b22544b7"`. `git diff K 549f3548 -- . ':(exclude)fathomdb'` touches only the root `FATHOMDB.md` (`887e5781`, R-FK-VERIFY F1: its commit table), which no published crate packages; no code changed after `K` |

Receipts: cudarc `fathomdb/qualification/` (G1–G6, G8, CI, CF-AC-07a and
07b); Candle `fathomdb/qualification/2026-10-12-K-G1-K-G3-853324fc.md` and
`2026-10-12-K-G2-853324fc.md` with their `.log` files.

## Publication preparation

- cudarc FC10 (CF-AC-07b) was run early on the owner's request: receipt
  `2026-10-13-CF-AC-07b-b777f8d.md` (`a6d49c41`, fixes `d3a5a055`).
  R-FC-PUB: APPROVE-WITH-FIXES; finding 1 closed by making
  `fathomdb/0.19.10` the fork's default branch (Q17). By owner direction
  (Q17) R-PUB was approved before R-VERIFY, and `cudarc-fathomdb` 0.19.10
  was published from `C` on 2026-10-13 at 14:02Z (receipt
  `2026-10-13-FC10-publish-b777f8d.md`, fork commit `9b0ef75`).
- Candle FK5: K8 `cd64715e` (0.10.4 on registry `cudarc-fathomdb` 0.19.10)
  passed CK-AC-07 on the RTX 3090 and the Orin on 2026-10-13 (fork receipt
  `fathomdb/qualification/2026-10-13-CK-AC-07-cd64715e.md`, fork commit
  `ddff4cbd`). R-FK-PUB approved it apart from finding 1: 0.10.3 → 0.10.4
  replaces candle-core's re-exported cudarc with `cudarc-fathomdb`, a
  semver-breaking change. Owner ruling Q21: publish 0.11.0. `K9` =
  `182ceab0547348993b6dbec4b1aed610dcc8b23f` (record `4abb8b38`) is `K8`
  with versions 0.11.0 and a fork `description`; it is the publication
  commit. CK-AC-07 cells at `K9` (control `K8`) are running; the R-FK-PUB
  re-check of `K9` and the owner's publication of 0.11.0 follow. `K8`'s
  receipt stands as evidence for the same code. `git diff 853324fc
  182ceab0 -- candle-kernels` is empty.

## W6 (implementer, 2026-10-13)

Test-plan § 2 order. RED is the observed failure; the commit bodies hold
the full output.

| Row | RED | GREEN | Observed RED |
| --- | --- | --- | --- |
| R1 single-cudarc guard | `0fe295edd` (arm rewired `f1a27631d`) | `d3381cc3b`; real-tree arm GREEN at `aa898677f` | `FAIL guard script scripts/check-single-cudarc.sh is absent` |
| R2 pinned-override schema 5 | `36f48f95b` | `aa898677f` (pin commit) | `UNVERIFIED pinned-override-rot: metadata schema_version must be 4`; each new arm's message absent |
| R3 lock-set guard | `0ef134ea0` | `f8a2ad147` (hermetic arms); real-tree arm GREEN after the record copy `c8234c927` | `FAIL guard scripts/check-cudarc-fathomdb-lock-set.py is absent` |
| R4 `CUDARC_CUDA_VERSION` | `f243750e4` | `229cb0890` | `FAIL rejects a CUDA job without CUDARC_CUDA_VERSION 12060 (cuda-contract-preflight) (the checker passed)` |
| R5 release-contract Candle block | arms `f243750e4`; RED of record at `aa898677f` | `b9da8c591` | `cuda-release-contract: [patch.crates-io] lacks a source pin for 'candle-core-fathomdb'` (baseline) |
| R6 retired references | `88e871a57` | partly at `aa898677f`; references commit pending | 15 files, 10 marker lines, `tegra-pool` = `['dep:cudarc', 'cudarc/fathomdb-private-pool']` |
| R7 PTX floor | `8346b4661` | `fffc6dd0f` | `FAIL checker scripts/check-ptx-driver-floor.py is absent` |
| R8 compute floor (C4) | `2cc4e9ae0` (compile), `9166d660c` (behaviour) | `26f917eae` | E0432/E0407; then 5 failed: `left: Ok(... compute_capability: Some("5.0") ...) right: Err(ForcedCudaUnavailable { ordinal: 0, reason: CudaIncompatible })` |
| R9 D-18 pure part | `943e5c9c1` | `a48ef0bc3` | E0425/E0433 (types and constants absent) |
| R11 T-REGISTRY | old script at `aa898677f` | `68f3233b0` (REG-a GREEN on windchill3) | `[patch.crates-io] must carry exactly one cudarc entry`, rc=1 |
| R18 required-symbol guard | `9f0e58219` | `b95bcedc2`; real tree `c9d67f2f5` | `FAIL checker scripts/check-cuda-required-symbols.py is absent` |

**Pin commit (Phase B, R-PIN): `aa898677f3eff2fb8fb40e2108a1b29072f0e7fa`,
pushed 2026-10-13** so the cudarc fork's Run 2 can fetch its `Cargo.lock`
from `raw.githubusercontent.com` (design § 7.2). `Cargo.lock` SHA-256
`b887a101a1ea9684a8e64d0bb8a849da6ef4e61a777bdc2ff62062126a8d142a`.
R-PIN evidence (commit body): one `cudarc-fathomdb` 0.19.10 from crates.io,
checksum `8780d2fe…caf2`; `candle-{core,nn,transformers}-fathomdb` 0.11.0
from crates.io with the FK5 checksums; `candle-kernels` 0.10.2 from
`git+https://github.com/coreyt/candle-fathomdb.git?rev=182ceab0…`; the
only other `cudarc` is 0.17.8 (ug-cuda); patch table `['candle-kernels',
'libsqlite3-sys']`; `tegra-pool = []`; no target dependency tables;
`.cargo_vcs_info.json` sha1 `b777f8d9…` and `182ceab0…`; relied-on items
1, 2, 3, 5, 9 and K1, K2, K3, K5, K7 present in the fork tables;
`check-pinned-override-rot.sh` and `check-single-cudarc.sh` ok;
workspace and `embed-cuda,rerank-cuda` `cargo check` pass (build only).

**Deviations so far.**

- R1: the flash-attn exclusion arm passes `--lock lock/ok.lock`; the
  fixture's own lock must hold the optional `cudarc` 0.19.4, which trips
  condition A (`f1a27631d`). The expectation is unchanged.
- R8: an intermediate commit (`9166d660c`) adds only the trait method and
  parser, so the behavioural REDs are observed, not only the compile
  failure.
- D-18 `required_major`: the design says the fork's
  `LibLoadError.cuda_major` gives 10 for cuRAND; at `b777f8d` it is the
  CUDA major (12) for every library, so FathomDB maps cuRAND to its soname
  major 10 itself.
- R9's two `LibLoadError` tests and R10 (CLI) need the fork types and the
  driver part, so they are written RED on the pinned tree (Phase C).
- Existing policy-test fakes that report compute capability 8.6 and do not
  override `kernel_compute_floor()` would be refused in a build with
  `CUDA_COMPUTE_CAP=87`; no Orin window runs those suites.

### W6 Phase C (on the pinned tree)

| Row | RED | GREEN | Observed RED |
| --- | --- | --- | --- |
| R12, R12b decoy cuBLAS (3090) | `b564c9671` | `3116da864` | R12: the child panics in the fork's loader (`Unable to dynamically load the "cublas" shared library for CUDA 12. Tried: libcublas.so.12 (wrong version 13.1.1), libcublas.so (wrong version 13.1.1)`); R12b: no panic, the device is built and the F32 matmul runs on the decoy's stub (`matmul ran: [[[0.0, 0.0], …]]`), no refusal |
| R9 `LibLoadError` mapping, R13 report and stderr line | `42ff7d865` | `3116da864` | E0425/E0599/E0609 (`refusal_from_lib_load_error`, `LibraryUnavailable`, `CudaLibraryUnavailable`, `cuda_libraries` absent) |
| R10 `doctor cuda-libraries` | `270d9595f` | `e8b0ef299` | E0599 (`DoctorCommand::CudaLibraries`), E0425 (`cuda_libraries_output`, `cuda_libraries_exit_code`) |
| D-18 report in the bindings | `28671f3b6` | `ae07740ff` | unresolved SDK imports; no `cuda_libraries` on the PyO3 and N-API mirrors; Python `ImportError`; TypeScript TS2305/TS2353 |
| CUDA-07 driver entry | `a63bd5317` | `c82b18750` | `no driver entry in [CudaLibraryReport { library: "cublas", … }, …]` |
| D-17 sites 3 and 10 | refactor, no RED | `179af293c` | aarch64-only code, first compiled on the Orin |
| R6 references | `88e871a57` | `710aa0a23` (after the self-match fix `9b0527efb`) | see Phase A |
| R16 engine close | characterization | `c678eeb51`, `b220deeaa` | written, `#[ignore]`, compiled on windchill3; first run in Orin window P |
| R15 Python F1–F5 | characterization | `f4d057136` | written; skip off a Tegra wheel on a Jetson |

Other Phase C commits: docs and interface docs `0440d430a`; ADR
amendments, index rows 62 and 63 and the changelog `76e992da4`;
contributor map `2ef33f5ae`; T-CUDA receipt `595dc6a04`; the ONNX
embedder's `CudaDeviceInfo` literals `b3e39e9bd` (found by the
feature-complete gate); the feature matrix `9b43d29c1`.

**T-CUDA on the RTX 3090** (receipt
[2026-10-13-T-CUDA-3090-c82b187505a9.md](qualification/2026-10-13-T-CUDA-3090-c82b187505a9.md);
GPU `GPU-5f9cfc90-2be1-06a7-ce39-5a6d294b209b`; guard unchanged in every
line of [windchill3-guard.log](qualification/windchill3-guard.log)):
WSB-01 pass; CMP-C1b 11 PTX modules, `.version` ≤ 8.5, `sm_75`; PRD-24
6 passed; CUDA-01 and CUDA-02 preflight passed (wheel and N-API need no
`libcudart`); CUDA-03 `not_applicable`; CUDA-04/05/06/11 6 passed, 1
ignored; CUDA-07 driver, cuBLAS 12.6.1 and cuRAND 10.3.7 loaded from the
expected real paths; CMP-C2 cuBLAS 12.1.3 and 12.9.2 each load, report,
embed and rerank on `cuda:0`.

**Further deviations and findings (Phase C).**

- R12b: Candle 0.11.0's F32 matmul calls `cublasGemmStridedBatchedEx`,
  not `cublasSgemmStridedBatched` (no Candle matmul reaches it), so the
  test plan's RED path (a matmul panic) did not occur; the GREEN
  assertion is as specified.
- R12: the fork version-checks versioned names at `b777f8d` (its `judge`
  runs for every candidate), contrary to design § 9.1's premise; the
  refusal comes from the `LibLoadError` (`WrongMajor`), whose file is
  found on `LD_LIBRARY_PATH`.
- `CudaLibraryRefusal.library` is `&'static str` and `path`/`version` are
  `Option<Box<str>>`, to keep the by-value error variants under clippy's
  `result_large_err` limit.
- `c-libmajor-wc3` counts 6 passed, 1 ignored (plan: 5 and 1): the CUDA-07
  driver test was added.
- CUDA-03 reports `module_load_init` `not_at_load` on x86 (plan: `ran`);
  the early-`cuInit` hook exists only on aarch64 Linux.
- CMP-C2 ran without the wheel (the preflight deletes it): the release
  CLI and the embedder's live test binaries with each cuBLAS wheel on
  `LD_LIBRARY_PATH`; `c2-embed-rerank.py` was not written.
- `tests/device_policy.rs` (not frozen) gained the new `CudaProbeError`
  variant in its fixture's exhaustive match; `ort_bge.rs`'s in-module
  test literal gained `cuda_libraries`. Compile adaptations, no assertion
  changed.
- The preflight witness JSON files are not committed (gitleaks flags
  their SHA-256 values); the receipt quotes the digests.
- `docs/reference/errors.md` lists error classes, not kinds; the CUDA
  kinds stay in `docs/embedder.md`. `dev/interfaces/wire.md` names no CUDA
  kind and needs no change.
- Not written: the Rust Orin tests of `src/cuda_pool_slice31_tests.rs`
  (R14 N1 RED, R15 TDN-01/02, R17 D-12 and D-14 measurements) and N1. They
  compile only on aarch64 Linux, and windchill3 has no aarch64 Rust target
  or `rust-src`; they belong to Orin window P. D-17 is likewise compiled
  first on the Orin.

### Left for the Orin, GCP and the main thread

- **Orin windows P and A**: done at `e9036a912` (below). N1 and the R16
  finding are ruled (Q29, Q30); design § 14 awaits review. **Window B** (G1–G8) and **window C**
  (G9): runners ready on the Orin, not run. PERF-03 on the RTX 3090:
  runner written (`run-perf-3090.sh`), not run.
- **GCP**: S-C3 (CMP-C3, CUDA-09) and S-13 (WSB-02, CUDA-08, CMP-C5), each
  on the owner's go-ahead.
- **cudarc fork FT-3**: Run 2 with `FATHOMDB_LOCK_SHA` =
  `aa898677f3eff2fb8fb40e2108a1b29072f0e7fa` and the Linux-normal record;
  then copy it to `scripts/cudarc-fathomdb-lock-set.json` (R3's real-tree
  arm, and the fast tier, stay RED until then).
- **Main thread**: the owner's advisory review of the `candle-kernels` rev
  change (pinned-override posture); FT-1…FT-3 to the todos ledger when W6
  lands; the R-TEST OP-3 todo (feature-complete gate by UUID).

### W6 Orin windows P and A (2026-10-13, Jetson AGX Orin 64 GB)

Receipts [2026-10-13-ORIN-P-e9036a912442.md](qualification/2026-10-13-ORIN-P-e9036a912442.md)
and [2026-10-13-ORIN-A-e9036a912442.md](qualification/2026-10-13-ORIN-A-e9036a912442.md);
guard unchanged in all 52 lines of
[orin-guard.log](qualification/orin-guard.log).

| Row | Commit | Result |
| --- | --- | --- |
| R14, R15, R17 tests written | `e9036a912` | `src/cuda_pool_slice31_tests.rs` and two test-only counter readers; the first aarch64 compile of D-17 and the `tegra-pool` paths passed with no warning |
| R14 threshold 0 | `e9036a912` | GREEN without N1: `reserved_current` 0 after the drop, no synchronize. The predicted RED did not occur |
| R14 threshold `max` | `e9036a912` | RED as predicted: `reserved_current` 301989888 after close |
| R15 TDN-01, TDN-02 | `e9036a912` | characterization, GREEN at `e9036a912` |
| R16 TDN-03 | `e9036a912` | FAILED: clean close and exit 0, but `path=default_pool`: the test never runs the module-load early `cuInit` (reason `cuinit_not_at_load`). Finding for R-VERIFY; test not edited |
| R17 ORIN-13 | `e9036a912` | usable 1073741824 bytes (32 × 32 MiB) of the 3 GiB pool: above 576 MiB, no owner trigger |
| R17 ORIN-14 | `e9036a912` | six stages recorded; default pool 0 at every stage (asserted) |
| Window A | `e9036a912` | POOL-01…04 pass; TDN-06…10 (F1–F5) 5 passed; ORIN-12 5/5; ORIN-15 exit 0 (wheel and `.node`); CMP-C1b 11 PTX modules `sm_87`; REG-b/REG-c exit 0 with the mode `off` and with `auto` |
| Harness | `6d73dc19b` and the window-runner commit | Slice 30 harness copied and adapted; PERF-03 runner; window B and C runners |

**N1: ruled.** R14's conditional had one test passing and one failing
(threshold 0 releases without a synchronize; `max` keeps 301989888
bytes). Owner ruling Q29 (steward ledger seq 335): implement N1, so close
also trims a `max` pool. **R16: ruled.** Owner ruling Q30 (seq 336):
correct R16 and give plain Rust processes the private pool without a
caller-made init call (design § 14.2, D-19). Design revision 4, § 14,
is drafted for review; Phase 2 (implementation) waits for it.

**Deviations.** maturin 1.14.1 came from an existing build venv on the
Orin (the host default is 1.12.6); F1–F5 ran from a copy of the test
file, because `src/python`'s `pythonpath = ["."]` would import the source
package instead of the wheel, with `pytest` from the user site on
`PYTHONPATH`; the D-14 CPU control is the same test with
`FATHOMDB_EMBED_DEVICE=cpu`. The FT-3 lock-set record copy
(`scripts/cudarc-fathomdb-lock-set.json`) landed later in `c8234c927`.

### W6 design revision 6: N1, D-19, the implication and the pool settings (2026-10-15)

Implemented against design revision 6 (`831255b9e`; owner rulings Q33-Q38).
The frozen `cuda_pool_policy_tests.rs` is byte-identical and passes.

| Topic | RED | GREEN | Evidence |
| --- | --- | --- | --- |
| § 14.3 implication (Q37) | `2af2826b4` (script test fails on 14 features) | `b2654de76` | `scripts/tests/test_tegra_pool_implied_by_cuda_features.sh`; release contract adjusted to the cfg reality (only `embed-cuda`/`rerank-cuda` may enable `tegra-pool` in the N-API manifest); `ENABLE=embed-cuda-only` registry row |
| § 14.1 N1 (Q34) | `514e9dca1` | `0bfc3e783` | `cuda_pool_release_tests.rs` (any host); Orin R14 `max` and the two-model test GREEN |
| § 14.2 D-19 probe read (Q36) | `514e9dca1` | `0bfc3e783` | `cuda_pool_probe_read_tests.rs`; Orin R16 RED at `514e9dca1` (`cuinit_not_at_load`), GREEN at `32295692f`; SDK children GREEN |
| § 15 / § 15.9 settings (Q33, Q35) | `514e9dca1` | `0bfc3e783` | `cuda_pool_mode_tests.rs`, `slice31_cuda_pool_settings.rs` (engine), Python, TypeScript and CLI tests; Orin parity matrix |
| Docs | — | `8aa5b68e7` | interface docs, user docs, ADR amendment decisions 2, 6, 9-11, changelog |

Orin windows P and A re-run at `32295692f`: every cell passed, guard
unchanged in 104 lines
([2026-10-15-ORIN-PA-32295692f56b.md](qualification/2026-10-15-ORIN-PA-32295692f56b.md)).
`venv-tip/`, `node-tip/` and `artifacts/fathomdb` were rebuilt in place;
`node-base/` is unchanged.

Open: per-SDK post-close pool counters (no binding exposes them; the
embedder-level tests cover the release); the § 15.8 RTX 3090 row passed at `392832c72`
(`slice31_cuda_pool_x86.rs`, 1 passed, RTX 3090 by UUID); Orin build directories `src-*`, `target-0bfc3e783713`,
`target-32295692f56b`, `target-red` and the window scripts and bundles
under `~/fathomdb-slice31/` are left for removal (about 16 GB).

### Orin G8 soak characterization (2026-10-15)

Window B's Node G8 drift (+12.1 %) was characterized with 60-minute soaks,
pool on and pool off. The owner accepted G8 as measured (Q39, steward ledger
seq 345) on the justification in
[2026-10-15-ORIN-G8-SOAK-32295692f56b.md](qualification/2026-10-15-ORIN-G8-SOAK-32295692f56b.md):

- Python's +1.0 MiB per minute is the harness's per-minute allocator churn,
  identical with the pool off.
- Node rises to a bounded plateau with or without the pool; the pool adds
  a fixed about 40 MiB of unified memory.
- No errors, mismatches, swap or system memory loss.

The 0.8.29 todo TC-84f7f62d replaces the drift bound with a plateau
criterion. The harness's G3 median bound now matches the ruled 0.400 MiB
(`9844eeba8`); window B's Python G3 (median 0.380) passes.

### Full gate, R-CODE-3 and R-VERIFY (2026-10-15)

- Full gate `agent-verify.sh` at `392832c72`: 198 of 202 suites passed.
  The four failures were inventory drift (feature matrix, slice90 root
  blob, the EARP config-field classification and package-boundary deltas),
  fixed in `836af60a1`; those suites and the touched tests were re-run and
  pass (owner rule: re-run only the failed suites).
- R-CODE-3 (Opus) fix steps: LOW-4 RED `9e835cf09` → GREEN `51d5df79e`
  (with LOW-6); LOW-5 `836af60a1`; MEDIUM-1, MEDIUM-2, LOW-3, LOW-7
  `fc60245a4`. MEDIUM-1 strengthens the Orin two-model test to
  `assert_eq!` on `reserved_current`; it passed on the Orin at
  `fc60245a4` (`qualification/2026-10-15-ORIN-M1-fc60245a4.log`).
- AC31-10 named exception: `slice30_cuda_allocator_api.rs` changed in
  `0bfc3e783`, forced by the § 15.4 report fields; no assertion changed:

  ```diff
  -    ReleaseThreshold, RerankerDevicePolicyError,
  +    PoolMode, PoolSettingSource, ReleaseThreshold, RerankerDevicePolicyError,
  @@ fn a_constructed_device_info_has_no_allocator_report_until_one_is_attached
           ModuleLoadInit::Ran,
  +        Some(PoolMode::Auto),
  +        Some(PoolSettingSource::Default),
  +        Some(PoolSettingSource::Default),
  +        Some(PoolSettingSource::Default),
       );
  ```

- R-VERIFY (Opus) at `1f1646f74`: record gaps B1-B7 fixed in the next
  commit; ACs met except AC31-03b, AC31-13 and AC31-15, which wait on
  PERF-03, S-C3 and S-13.

## Next

PERF-03, S-C3 and S-13, then merge into `release/0.8.28`.

## Log

- 2026-10-13: plan revision 16 (registry consumption after the early
  `cudarc-fathomdb` 0.19.10 publication; rulings Q17–Q20); `GOAL.md`
  requires revision 16.
- 2026-10-13: W1 drafted: [requirements.md](requirements.md) (DRAFT FOR
  R-REQ), REQ31-01…15 with AC31 sets per review and seven open points.
  Next: R-REQ.
- 2026-10-13: plan revision 17: the Candle fork crates publish as 0.11.0
  from `K9` (`182ceab0`), not 0.10.4 from `K8` (owner ruling Q21, R-FK-PUB
  finding 1); W6 pins the Candle crates `=0.11.0` and `candle-kernels` at
  `K9`; `GOAL.md` and `requirements.md` follow.
- 2026-10-13: R-REQ (Opus) on `requirements.md` at `f41563903`:
  APPROVE-WITH-FIXES, F1–F12 and seven open-point rulings, all applied
  ([review.md](review.md)); new REQ31-16 (REQ-068 through the
  `candle-kernels` patch); T-REGISTRY moves into W6. `requirements.md` is
  REVISED AFTER R-REQ, ready for W2. Next: W2 → R-DES.
- 2026-10-13: W2 drafted: [design.md](design.md) (DRAFT FOR R-DES,
  revision 1): D-8W, D-10, D-11 (exact `=0.19.10`/`=0.11.0` registry pins,
  marker feature dropped, pinned-override schema 5, single-cudarc guard,
  T-REGISTRY), D-12, D-14, D-15 (Slice 30 teardown assertion kept for
  0.8.29), D-16 (lock-set guard and fork record format), D-17 (14 call
  sites, 2 changed), D-18, the ADR amendment draft and seven open points.
  The Candle crates 0.11.0 are on crates.io (Candle fork receipt
  `2026-10-13-FK5-publish-182ceab0.md`, record commit `e9c6ed5e`). Next:
  R-DES.
- 2026-10-13: R-DES (Opus) on `design.md` revision 1 at `1e0537856`:
  APPROVE-WITH-FIXES, findings 1–10 and seven open-point rulings, all
  applied with owner rulings Q22–Q25 (steward ledger seq 325–328) in
  `design.md` revision 2 (REVISED AFTER R-DES, ready for W3),
  `requirements.md` (REQ31-10, AC31-07, AC31-10, AC31-14, AC31-15) and
  plan revision 18 ([review.md](review.md)). Main changes: hermetic C4 with
  an unconditional compute-capability floor; RTX 3090 by UUID for every
  new `windchill3` process; the embedder's direct `cudarc` dependency
  removed; `LibraryUnavailable` reached through Candle's re-exported
  cudarc; the Linux-normal lock-set closure; C3 as a GCP T4 cell (design
  § 9.3); fork todos FT-1–FT-3 to file when W6 lands. Next: W3 → R-TEST.
- 2026-10-13: W3 drafted: [test-plan.md](test-plan.md) (DRAFT FOR R-TEST,
  revision 1): hosts and guards (RTX 3090 by UUID, Orin by git bundle, GCP
  sessions S-13 and S-C3), the W6 RED-first order R1–R17, a catalogue of
  every AC31 and plan § 7 topic with exact commands, the qualification
  runs (G1–G10, G9 in full, the RTX 3090 arm, D-14, usable capacity, C3),
  traceability, the single-cudarc fixtures verified in a scratch
  directory, and eleven open points (OP-1: the release contract still pins
  the Candle git cohort at `859b8ea1`; OP-2: AC31-10's `git grep` must
  exclude the steward ledger; OP-3: the feature-complete gate selects GPUs
  by index). T-REGISTRY resolution measured in a scratch consumer with no
  `[patch]` table. Next: R-TEST.
- 2026-10-13: R-TEST (Opus) on `test-plan.md` revision 1 at `72a471508`:
  APPROVE-WITH-FIXES, findings 1–17, rulings on OP-1 to OP-11 and
  proportionality cuts, all applied with owner rulings Q26–Q28 (steward
  ledger seq 329–331) in `test-plan.md` revision 2 (REVISED AFTER R-TEST,
  ready for W6), `design.md` revision 3, `requirements.md` (AC31-02,
  AC31-04, AC31-10, REQ31-10, REQ31-15, AC31-15) and plan revision 19
  ([review.md](review.md)). Main changes: embeds and reranks through the
  CUDA wheel and the embedder's live test binaries (no CLI embed verb);
  the `libcuda.so.1`-only cell moved to S-C3; D-18 refuses a cuBLAS or
  cuRAND missing a required symbol (Q26; FathomDB-side check through the
  fork's `is_symbol_present`, no fork change); the cuBLAS and cuRAND
  remedy; message-asserting release-contract arms; Orin windows P, A, B
  and C; `QUAL_QUIET_STRICT=1` for G9; S-13 sizing (Q27); G5's Slice 30
  equality report-only (Q28). Main-thread todo to ledger: move the
  feature-complete gate from GPU indices to UUID selection (OP-3). Next:
  W6.
- 2026-10-13: design revision 4, § 14 (DRAFT FOR REVIEW): N1 implemented
  per Q29 (close trims the pool at any threshold; no-op on loss or a
  non-private model); D-19 per Q30 recommends a first-use init at the
  allocator decision in `fathomdb-embedder` (new `ModuleLoadInit`
  `ran_at_first_use`; load hooks win; opt-out honoured), rejecting a
  load-time constructor and a public init API. Next: review, then Phase 2
  (RED: R14 `max`, corrected R16, new SDK Orin tests).
- 2026-10-15: W6 design revision 6 implemented (§§ 14.1-14.3, § 15), RED
  first; Orin windows P and A and the parity matrix pass at `32295692f`.
  Next: the main thread's full gate, then windows B and C.
