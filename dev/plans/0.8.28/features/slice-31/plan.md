---
title: FathomDB 0.8.28 Slice 31 — the private CUDA memory pool on forked cudarc, CUDA 12.6 and 13.x, through crates.io (D28-13) plan
status: APPROVED SLICE SCOPE (owner, 2026-10-11); revision 19 — R-TEST (Opus) APPROVE-WITH-FIXES on `test-plan.md` revision 1, all seventeen findings, the eleven open-point rulings and the proportionality cuts applied in `test-plan.md` revision 2 (REVISED AFTER R-TEST, ready for W6) and `design.md` revision 3; owner rulings Q26 (D-18 refuses a CUDA 12 cuBLAS or cuRAND that lacks a required symbol, a typed error, never a panic), Q27 (S-13 may use `n1-standard-8` or larger for one build attempt; many VM attempts; 2 h per session; receipts after every cell; owner go-ahead per session) and Q28 (G5's equality to the Slice 30 hash is report-only, evaluated after the work); T-CUDA-COMPAT C2 drops the 12.4 cell; revision 18 — R-DES (Opus) APPROVE-WITH-FIXES on `design.md` revision 1, all ten findings applied in `design.md` revision 2 (REVISED AFTER R-DES, ready for W3); owner rulings Q22 (the Quadro K620 is never run; C4 is a hermetic compute-capability floor test; every new `windchill3` process names an RTX 3090 by UUID), Q23 (the D-16 lock-set guard covers the Linux-target normal closure), Q24 (the D-18 stderr line only on refusal; `doctor cuda-libraries` always reports) and Q25 (T-CUDA-COMPAT C3 as a GCP T4 cell with the driver only); the embedder's unused direct `cudarc` dependency is removed; REQ31-10 states the deliberate product changes; revision 17 — the Candle fork crates publish as 0.11.0 from `K9` (`182ceab0`), not 0.10.4 from `K8` (Q21: replacing candle-core's publicly re-exported cudarc is semver-breaking, R-FK-PUB finding 1); W6 pins the Candle crates `=0.11.0` and the `candle-kernels` git patch at `K9`; R-REQ fixes applied in `requirements.md` (T-REGISTRY moves into W6; § 7 T-POOL-PRODUCT names the Slice 30 GPU tests); revision 16 — `cudarc-fathomdb` 0.19.10 published from `C` on 2026-10-13, before W1–W3, W6 and R-VERIFY, by owner direction (Q17); FathomDB consumes the published registry crates in W6 (`cudarc-fathomdb` 0.19.10 and the Candle crates 0.10.4 from `K8`, Q18) instead of git pins, and `candle-kernels` stays a git patch at a Candle commit; `fathomdb/0.19.10` is `coreyt/cudarc`'s default branch and the upstream proposals are not started (Q17); W1–W3 run in full (Q19); R-PIN approved (Q20); the 0.8.28 release state, board and plan exist; revision 15 — state of the slice on 2026-10-13 (§ 8.1): fork development and qualification done for the pinned commits, remaining fork steps, W0 step 3 done; revision 14 — fork-first order (Q14), CUDA 13.0 toolkit cells on a GCP VM and Spot VMs for test only (Q15), Candle base stays 0.10.2 (Q16), both forks qualified (cudarc `C` = b777f8d, Candle `K` = 853324fc), Candle R-FK-PLAN M6 fixes, the x86 BF16/F8 caveat, FC10 dry run done early, W0 step 7 ledger entries filed; revision 13 — Q13 test-venue rulings (GCP pilot first; AWS arm64 cells to 0.8.29; Jetson on CUDA 13 via CloudJetson); revision 12 — second cudarc proposal (built-major library loading, FC9b, R-PROP2), production loaded-library check (D-18), cloud test venue research pending (Q13); revision 11 — CUDA version support policy (ADR-0.8.28-cuda-version-support-policy.md), REQ31-15 and T-CUDA-COMPAT, rulings Q10–Q12; revision 10 — owner rulings Q8 (D-16 lock-drift guard (b)) and Q9 (crate name `cudarc-fathomdb`); revision 9 — R0-rev8 fixes; fork-side work planned, performed, reviewed and qualified in each fork (cudarc and Candle fork plans revision 2); W0 merge-forward done at 9fa71e93a
target_release: 0.8.28
observed_on: 2026-10-11
---

# 0.8.28 Slice 31: the private CUDA memory pool on forked cudarc

This slice makes FathomDB's CUDA memory pool excellent end to end:

- **On the cudarc fork:** the pool primitive and the Jetson allocator
  fallback, rebased onto upstream v0.19.10 and correct for every cudarc
  CUDA feature (11.4 through 13.4).
- **Consumption:** Candle and FathomDB consume that fork, and the fork is
  published so crates.io consumers of `fathomdb` get it.
- **Proposal:** a tidy cudarc memory-pool proposal is prepared, not posted.

The fork work is planned, coded, reviewed and qualified inside each fork, as
recorded in that fork's own `fathomdb/plan.md` (§ 3.1). This plan tracks that
work, consumes its result, and owns everything on the FathomDB side.

It is the working plan for D28-13 in `dev/plans/0.8.28-draft-scope.md` until
the 0.8.28 plan, release-state file and board exist; those must copy it.
They exist since 2026-10-13 (`4565f23d3` on `release/0.8.28`:
`dev/plans/plan-0.8.28.md`, `dev/plans/release-state-0.8.28.json`,
`dev/plans/runs/STATUS-0.8.28.md`), and point back to this plan.
Slice number 31 (owner, 2026-10-11). The session goal is [GOAL.md](GOAL.md).
Nothing in this slice writes to `release/0.8.27`.

## 0. Owner instructions and where this plan covers them

| # | Owner instruction (2026-10-11) | Covered by |
| --- | --- | --- |
| I-1 | Slice 31 is approved | front matter; § 12 R0 … R0-rev8 |
| I-2 | Slice 31 gets its own worktree; the `release-0.8.28` worktree used for the merge-forward is removed (done) | § 9 W0; § 13 merge method; [merge-forward plan](merge-forward-plan.md) |
| I-3 | A goal so the work can continue and complete | [GOAL.md](GOAL.md) |
| I-4 | Bring the cudarc fork to v0.19.10, then apply our changes | § 9 W4; cudarc fork plan FC0, FC4 |
| I-5 | Compatible with the latest CUDA 13.x; read release notes and published docs | § 2.3; REQ31-03; cudarc fork plan CF-REQ-04, D-8, D-F1; § 7 T-CUDA |
| I-6 | Subagents as appropriate | § 11; fork plans § 9 / § 8 |
| I-7 | Appropriate reviews | § 12; fork plans |
| I-8 | Requirements + acceptance criteria, design, test plans reviewed | § 12 R-REQ, R-DES, R-TEST; cudarc R-FC-REQ, R-FC-DES, R-FC-TEST; Candle R-FK-PLAN |
| I-9 | Opus reviews, not codex | § 11 review spawn; § 12 |
| I-10 | No steward or orchestrator | § 11 |
| I-11 | TDD (RED/GREEN) | § 10; fork plans |
| I-12 | Code is reviewed | § 12 R-FC-CODE, R-FK-CODE, R-FK-PUB, R-CODE-3, R-CODE-4, R-PROP |
| I-13a | Implement a memory pool | REQ31-02, REQ31-13; cudarc fork plan |
| I-13b | Vendor libraries and forks used with excellence | REQ31-06, REQ31-14; § 3.1; D-10 |
| I-13c | A tidy cudarc memory-pool proposal | REQ31-09; cudarc fork plan FC9, R-PROP; second proposal (built-major library loading) FC9b, R-PROP2 |
| I-13d | FathomDB 0.8.28 with excellence and pool support, including Jetson Tegra | REQ31-04, REQ31-10, REQ31-13; § 9 W6; T-ORIN |
| I-14 | Orin `10.83.10.13`, ssh/scp approved; a Claude agent may run there | § 2.2; § 11 |
| I-15 | Check outside PATH for libraries | § 2.2; § 9 W0 step 4; GOAL.md |
| I-16 | Excellence; this is a memory pool | cudarc fork plan § 5–6; § 6; § 7 |
| I-17 | Fork work is planned, performed, reviewed, tested and coded from within each fork, and tracked by Slice 31 | § 3.1; § 9 W4, W5; REQ31-14; both fork plans |
| I-18 | FathomDB need not run cudarc's own tests (including the driver-only 13.x builds); the fork must | § 7; cudarc fork plan § 6.2 G3; REQ31-03 |
| Q7 | Fork work is planned and run in each fork (I-17, I-18) | § 15 Q7 |
| Q8 | Close lock drift with D-16 option (b): a FathomDB guard that fails until the fork re-runs with the new lock | § 6 D-16; § 9 W6; § 15 Q8 |
| Q9 | The published cudarc fork is named `cudarc-fathomdb` | § 3.1; § 9 W8; § 15 Q9 |
| Q10 | CUDA version importance and actions accepted; the floor is CUDA 12.x libraries; CUDA usage and importance are re-reviewed every release | REQ31-15; § 9 W6; § 15 Q10; `ADR-0.8.28-cuda-version-support-policy.md`; `dev/design/cuda-version-support-review.md` |
| Q11 | `windchill3`'s Quadro K620 must keep its driver support: no driver branch after R580 | § 1; § 2.2; T-CUDA; § 15 Q11 |
| Q13 | Where CUDA compatibility is tested: `windchill3` Docker passthrough is acceptable; cloud options (GCP first) and cloud Jetson are evaluated; the Docker Hub image pull is held until then | T-CUDA-COMPAT C3; § 15 Q13 |
| Q12 | Check the 12.6 driver floor and a CUDA-13-libraries-only environment, with web evidence on compatibility | REQ31-15; T-CUDA-COMPAT; § 15 Q12 |
| Q4 | Never install the R615 driver; it would disable `windchill3` | § 1; § 2.2; § 9 W0 step 4 guard; § 15; fork plans § 1.6 |
| Q5 | Tegra on CUDA 13 is UNMEASURED in Slice 31; propose doing it before the 0.8.28 release | REQ31-04; § 15; § 16 follow-up |
| Q6 | Build against CUDA 13.0 without changing `windchill3`'s primary CUDA 12.6; find why 12.x was chosen | § 2.2; § 9 W0 step 4; T-CUDA; § 15 Q6 |
| Q14 | (2026-10-12) The forked libraries are FathomDB's upstream dependencies, so the fork work is done first | § 8; § 15 Q14 |
| Q15 | (2026-10-12) The CUDA 13.0 toolkit cells run on a GCP VM with driver branch 580; Spot VMs are for test and verification only, never development; no CUDA 13.0 runfile on `windchill3` | § 2.2; AC31-03b; § 7 T-WS-BUILD, T-CUDA, T-CUDA-COMPAT C5; § 9 W0 step 4; § 15 Q15 |
| Q16 | (2026-10-12) The Candle fork stays on upstream Candle 0.10.2 (Candle DK-1) | § 14; § 15 Q16; § 16 |
| Q17 | (2026-10-13) `fathomdb/0.19.10` is `coreyt/cudarc`'s default branch; publish `cudarc-fathomdb` 0.19.10 now; do not start the upstream proposals yet | § 8.1; § 9 W7, W8; § 15 Q17 |
| Q18 | (2026-10-13) Publish the three Candle crates 0.10.4 from `K8` once R-FK-PUB approves (version and commit superseded by Q21) | § 8.1; § 9 W8; § 15 Q18 |
| Q19 | (2026-10-13) W1–W3 run in full, each with its own Opus review, before W6 | § 9 W1–W3; § 15 Q19 |
| Q20 | (2026-10-13) R-PIN approved | § 9 W6; § 12 R-PIN; § 15 Q20 |
| Q21 | (2026-10-13) The Candle fork crates publish as 0.11.0, not 0.10.4: 0.10.3 → 0.10.4 would replace candle-core's publicly re-exported cudarc (`pub use cudarc;`, upstream registry `cudarc` 0.19.4 in 0.10.3) with a different crate, `cudarc-fathomdb`, a semver-breaking change (R-FK-PUB finding 1) | § 3; § 8.1; § 9 W6, W8; § 15 Q21 |
| Q22 | (2026-10-13) The `windchill3` Quadro K620 is never run, including for T-CUDA-COMPAT C4: C4 becomes a hermetic fake-provider test of a probe-time compute-capability floor from the build's `CUDA_COMPUTE_CAP`, checked before any device is built; every new `windchill3` test process names an RTX 3090 by UUID in `CUDA_VISIBLE_DEVICES` | § 7 T-CUDA, T-CUDA-COMPAT C4; § 9 W6; § 15 Q11, Q22; design § 0.1, § 9.2 |
| Q23 | (2026-10-13) The D-16 `cudarc-fathomdb` lock-set guard covers only the Linux-target normal dependency closure (what the fork compiles), not build/dev or Windows/WASI-only crates | § 15 Q23; design § 7.2 |
| Q24 | (2026-10-13) The D-18 loaded-library check writes a stderr line only when FathomDB refuses (missing or wrong-major cuBLAS); `doctor cuda-libraries` always reports | § 15 Q24; design § 9.1 |
| Q25 | (2026-10-13) Explore T-CUDA-COMPAT C3 (no CUDA 12 cuBLAS) on a GCP instance instead of a container image pull: driver only, no CUDA toolkit libraries, with its cost; each VM session needs owner approval; the hermetic injected-load-error test stays | § 7 T-CUDA-COMPAT C3; § 15 Q25; design § 9.3 |
| Q26 | (2026-10-13) Require a code fix so a CUDA 12 cuBLAS or cuRAND that lacks a symbol FathomDB or Candle needs gives a typed refusal, never a panic: D-18 extends from a major-version check to a required-symbol check; the C2 floor cells then test it | § 7 T-CUDA-COMPAT C2; § 15 Q26; design § 9.1; REQ31-15, AC31-15; test plan R12b, R18, CUDA-11 |
| Q27 | (2026-10-13) The S-13 GCP CUDA 13.0 build session may use `n1-standard-8` or larger for one build attempt; many attempts to obtain an available VM are allowed; the 2-hour per-session limit is still enforced; receipts are pulled after every cell; each session still needs owner go-ahead | § 15 Q27; test plan § 1.3, § 4.7 |
| Q28 | (2026-10-13) The G5 check that the embedding output hash on the new crates equals the Slice 30 hash is report-only, evaluated after the work, not a gate | § 15 Q28; AC31-04; test plan § 4.2 G5 |

## 1. Owner direction (2026-10-11)

- **Fork-first.** cudarc work happens on `coreyt/cudarc` and Candle work on
  `coreyt/candle-fathomdb`. Nothing is posted upstream until in-product use
  and owner sign-off. Ruling 38's "upstream-first in upstream style" governs
  the commit shape.
- **Fork work lives in the fork (I-17, I-18).** Each fork's plan,
  requirements, design, test plan, code, reviews and qualification receipts
  are in that fork. FathomDB records the fork verdicts and pins the
  qualified commits. It runs only product-level tests.
- **Consumption and base.** Consumption is a git dependency on the fork
  (2b), and all six vendored items move to the fork. The end state is
  crates.io carrying the patch, from the fork. The base is upstream cudarc
  v0.19.10. There is no `candle-kernels-fathomdb` for now. **Revision 16:**
  `cudarc-fathomdb` was published before W6 (Q17), so FathomDB consumes the
  registry crates directly (§ 9 W6); only `candle-kernels` stays a git
  patch.
- **Process.** The main thread runs the slice, with no steward and no
  orchestrator. Every review is an independent Opus subagent, not codex.
  TDD applies to all code.
- **CUDA.** The shipping target stays CUDA 12.6 (x86 and Jetson Orin on
  JetPack 6). The cudarc fork proves CUDA 13 compatibility with its full
  compile matrix through 13.4 and its driver-only 13.x GPU tests. FathomDB
  and Candle prove it with CUDA 13.0 toolkit runs on a GCP VM with driver
  branch 580 (Q15; revision 13 said `windchill3`).
- **`windchill3` is never changed.** Its driver stays 580.178.04, with no
  R615 (Q4). R580 is the last driver branch that supports its Quadro K620
  (Maxwell), so no later branch is ever installed (Q11). Its primary CUDA stays `/usr/local/cuda` → 12.6 (Q6). No
  CUDA 13.0 toolkit is installed on it (Q15).
- **Tegra on CUDA 13 is UNMEASURED in Slice 31** (Q5). A follow-up before
  the 0.8.28 release is proposed (§ 16).
- **Push policy.** Fork and FathomDB branch pushes need no prompt. Never
  push to Memex or any upstream repository. crates.io publication is the
  owner's.

## 2. Current state (verified 2026-10-11)

| Fact | Evidence |
| --- | --- |
| The workspace patches `cudarc` to the vendored 0.19.7 tree | root `Cargo.toml` `[patch.crates-io]` |
| Vendored delta: items 1–4 (598 lines), items 5–6 with tests (676 lines); FathomDB runs their unit tests through `scripts/tests/test_vendored_cudarc.sh` in the fast tier of `scripts/agent-test.sh:326` | `third_party/cudarc-0.19.7/FATHOMDB-PATCH.md` and patch files |
| Candle (fork rev `859b8ea1`) depends on `cudarc = "0.19.4"` | fork `Cargo.toml` |
| `fathomdb-embedder` depends on `cudarc = "=0.19.7"` on aarch64 Linux with marker feature `fathomdb-private-pool` | embedder `Cargo.toml` |
| `ug-cuda` → `cudarc` 0.17.8 (max CUDA 13.0) is lock-only; `cargo tree -i ug-cuda --features embed-cuda` is empty | `Cargo.lock`; research |
| Published `candle-core-fathomdb` 0.10.3 → upstream `candle-kernels` 0.10.2; the fork's static-CUDART kernels reach workspace builds only by git patch | fork `Cargo.toml`; `cargo info` |
| `release/0.8.27` merged forward into `release/0.8.28` at `9fa71e93a` (parents `91e190a65`, `c82faa95d`); todo `TC-d71a9e36` closed; the `release-0.8.28` worktree was removed afterwards, keeping the branch | `git log`; [merge-forward plan](merge-forward-plan.md) § Execution record |
| **Latent defect in today's vendored code:** `upgrade_device_ptr`'s safety doc (`core.rs:2797-2799`) says a pointer leaked from any context of the device is valid because all contexts share one decision; a `Private` context does not | vendored `core.rs` |
| x86 and Jetson builds share one pinned toolkit, `/usr/local/cuda-12.6` / nvcc 12.6.68 | `scripts/release/cuda-artifact-contract.sh:16-17` (rationale 11-15); `release.yml` GPU jobs |
| Slice 30's design, ADR (`ADR-0.8.28-tegra-private-cuda-pool.md`, index row 62) and qualification are the baseline not to regress | `slice-30/` |
| **Forks (revision 8):** both are GitHub forks, so a PR opened from them defaults to the upstream base; Actions are enabled on both; cudarc's upstream workflows trigger only on `pull_request` | `gh api repos/coreyt/{cudarc,candle-fathomdb}` |
| Fork clones `../fathomdb-forks/cudarc` and `../fathomdb-forks/candle-fathomdb` exist, with `upstream` remotes whose push URL is disabled; branches `fathomdb/0.19.10` (from `ec1cac71`) and `fathomdb/0.8.28-cudarc-fork` (from `859b8ea1`) carry the fork plans | `git remote -v`; fork plans |
| `coreyt/cudarc` `main` = upstream tag `v0.19.10` (`ec1cac71`); 19 CUDA features `cuda-11040`…`13040`; driver functions resolve lazily per function, so a newer-binding build runs on driver 580 until it calls a missing function | cudarc fork plan § 2 |
| Candle fork: upstream tag `0.10.2` (`7c7a8c57`) + 10 fork commits (10 files, +139/−41). Corrected in revision 14 (Candle fork plan revision 3): upstream 0.11.0 (`31f35b14`) depends on cudarc `^0.19.8`; upstream `main` (`c68b2499`, 2026-10-02) on `^0.19.10` | Candle fork plan § 2 |
| FathomDB's private-pool path compiles only on aarch64 Linux with `tegra-pool`: the embedder's cudarc dependency is aarch64-Linux-only, `cuda_pool_policy.rs` selects `fallback_only` elsewhere, and `tests/tegra_fragmented_va_cuda.rs` is aarch64-gated; on x86 `doctor cuda-allocator` reports `not_built` | `src/rust/crates/fathomdb-embedder/Cargo.toml:66-72`; `cuda_pool_policy.rs:139,165,1255-1340`; test file line 35 |
| `cudarc-fathomdb` is unclaimed on crates.io (2026-10-11, API 404) | crates.io API |

### 2.1 Evaluation of v0.19.7 → v0.19.10

Verified in a scratch clone unless marked [I].

- 15 upstream commits: fixes #591 (`CudaSlice` drop/leak event waits gated on
  managed multi-stream sync) and #602 (`alloc_pinned` leak); additions
  `alloc_pinned_with_flags`, `CudaEvent::try_is_complete`,
  `CudaContext::wrap_borrowed_stream`, `CudaFunction::cu_function`,
  `CudaViewMut::into_view`; one break (`CUgraphInstantiate_flags` bitmask,
  #600); CUDA 13.3/13.4 features. No upstream pool work; #536 open; PR #594
  (graph memory pool) would conflict with items 1–3 and 5 [I].
- The delta applies as text; the private-pool `Cargo.toml` hunk fails on any
  git checkout and the marker feature is added by hand; the `CudaSlice::drop`
  hunk sits under #591's independent gate.
- Existing defects on both bases: `mem_pool.rs` fails with `cuda-13020`+
  (`CUmemLocation.id` in a union); `set_max_size`'s cfg list stops at
  `cuda-13020`; below 12020 `max_size` is silently ignored; x86 dead-code
  warning (`in_fresh_process`).
- RTX 3090: 62/62 non-ignored `driver::` tests incl. 19 FathomDB tests with
  `cuda-12060`; Candle `859b8ea1` checks clean. aarch64 paths not yet run on
  the rebased tree.

### 2.2 Hosts and toolchains (searched outside PATH)

| Host | Hardware | Driver | CUDA |
| --- | --- | --- | --- |
| `windchill3` (x86_64, this host) | 2× RTX 3090 (sm_86), Quadro K620 (sm_50) | 580.178.04 — **never changed** (Q4) | Primary toolkit `/usr/local/cuda` → 12.6 — **never changed** (Q6). CUDA 13.0 runtime/NVRTC/cuBLAS libraries only in pip wheels (`.venv/.../nvidia/cu13/`); no CUDA 13 `nvcc` yet. |
| Jetson AGX Orin 64 GB, `10.83.10.13` (`coreyt`; ssh/scp approved) | Orin iGPU (sm_87) | 540.5.0 | L4T R36.5.2 (JetPack 6), `/usr/local/cuda-12.6`; JetPack 6 cannot run CUDA 13 |
| `win-vm` | — | — | not used by Slice 31 (the cudarc fork's hosted CI covers Windows) |

**CUDA 13.0 on `windchill3` without touching the driver or primary CUDA
(Q6).** The NVIDIA runfile installs only the toolkit with `--toolkit`, and a
non-root install into a user directory needs no sudo
([installation guide](https://docs.nvidia.com/cuda/cuda-installation-guide-linux/index.html)):

```bash
sh cuda_13.0.<u>_<driver>_linux.run --silent --toolkit \
   --toolkitpath="$HOME/opt/cuda-13.0" --no-man-page
```

Run as `coreyt` without sudo, it cannot write `/usr/local/cuda`, the driver
or the package database. Builds select it per command only:
`CUDA_PATH=$HOME/opt/cuda-13.0 PATH=$HOME/opt/cuda-13.0/bin:$PATH
CUDARC_CUDA_VERSION=13000`. Driver 580 is CUDA 13.0's own branch (R580), so
13.0 builds — including Candle's PTX — run on it. **Guard:** W0 and every
qualification run record the `nvidia-smi` driver version and
`readlink -f /usr/local/cuda` before and after; any change stops the slice.
Downloading the runfile is network access beyond the package registries, so
the owner approves the download (W0 step 4). **Revision 14 (Q15):** the
download was not approved and is not done. The CUDA 13.0 toolkit cells run
on a GCP VM instead (W0 step 4); the recipe above is kept only as history.

### 2.3 CUDA 13.x (primary NVIDIA documentation)

Research subagent, 2026-10-11; the driver table and the Jetson upgrade table
were re-checked by the main thread. [I] = inference.

- **Latest: CUDA 13.4.2 (September 2026)**
  ([archive](https://developer.nvidia.com/cuda-toolkit-archive);
  [release notes, "13.4 Update 1"](https://docs.nvidia.com/cuda/cuda-toolkit-release-notes/index.html)).
- **Drivers:** 13.0 → R580, 13.1 → R590, 13.2 → R595, 13.3 → R610,
  13.4 → R615; any 13.x application runs on ≥ 580 under minor-version
  compatibility, but "applications that compile device code to PTX will not
  work on older drivers"
  ([minor-version compatibility](https://docs.nvidia.com/deploy/cuda-compatibility/minor-version-compatibility.html)).
  Candle's kernels are PTX, so on driver 580 only CUDA 12.x and 13.0
  workspace builds can run; 13.1+ builds compile but are runtime-UNMEASURED
  on `windchill3` (Q4). Forward compatibility does not apply to GeForce
  ([forward compatibility](https://docs.nvidia.com/deploy/cuda-compatibility/forward-compatibility.html)).
- **Jetson:** JetPack 6 cannot run CUDA 13; JetPack 7.2 (L4T 39.2) adds Orin
  with CUDA 13.2, upgradable to 13.3/13.4 via `cuda-compat-orin-13-4`
  ([CUDA for Tegra § 8.2.1](https://docs.nvidia.com/cuda/cuda-for-tegra-appnote/index.html)).
  Known issue 5699079 (L4T 39.2): an excessively large allocation may reboot
  the device. NVML is Thor-only; Orin compute capability comes from
  `CUDA_COMPUTE_CAP=87`.
- **Architectures:** CUDA 13 supports Turing onward; sm_86, sm_87 remain;
  sm_50 (K620) is not a CUDA 13 target
  ([nvcc](https://docs.nvidia.com/cuda/cuda-compiler-driver-nvcc/index.html)).
- **Pool-relevant API changes:** `CUmemLocation.id` in a union from 13.2;
  13.4 adds `localized` and location type `DEVICE_LOCALITY_DOMAIN`
  ([CUmemLocation 13.2](https://docs.nvidia.com/cuda/archive/13.2.0/cuda-driver-api/structCUmemLocation__v1.html));
  `CUmemPoolProps`: no `maxSize` ≤ 12.1, `maxSize` 12.2+, `usage` 12.6+,
  `gpuDirectRDMACapable` and `reserved[53]` in 13.4
  ([CUmemPoolProps](https://docs.nvidia.com/cuda/cuda-driver-api/structCUmemPoolProps__v1.html));
  new pool attributes (13.2: 9–14 incl. `MAX_POOL_SIZE`; 13.4: 15); 13.0
  host and managed pools; `cuCtxCreate` takes `CUctxCreateParams` in 13.0
  (cudarc keeps `cuCtxCreate_v4`, 12.5+)
  ([13.0 context API](https://docs.nvidia.com/cuda/archive/13.0.0/cuda-driver-api/group__CUDA__CTX.html)).
  Pool function signatures are unchanged between 12.x and 13.x; `maxSize` 0
  means system-dependent
  ([stream-ordered allocator](https://docs.nvidia.com/cuda/cuda-driver-api/group__CUDA__MALLOC__ASYNC.html)).
- **cudarc version selection:** `CUDARC_CUDA_VERSION`, then the highest
  feature, then `nvcc --version` (exact match); issue
  [#525](https://github.com/chelsea0x3b/cudarc/issues/525) (Jetson built
  against 13.x bindings until `CUDARC_CUDA_VERSION=12060`). `build.rs`
  emits the env-pinned cfg in addition to any feature cfg (two version cfgs
  if both differ).
- **Candle:** upstream CI builds kernels in `nvidia/cuda:13.0.2`
  (`CUDA_COMPUTE_CAP=86`), PTX, `-std=c++17`; CCCL 3 needs C++17 [I: fine].
- **Artifacts:** 12.x and 13.x builds are separate artifacts (sonames and
  binding layouts differ) [I]. 0.8.28 artifacts stay on 12.6 (§ 15 Q6).

## 3. Target state

```text
fathomdb-embedder ──(cudarc = { package = "cudarc-fathomdb" })──┐
candle-core-fathomdb 0.11.0 ──(cudarc = { package = "cudarc-fathomdb" })──┤
                                                  cudarc-fathomdb (crates.io,
                                                  from coreyt/cudarc fathomdb/0.19.10)
```

- **One cudarc.** `cudarc-fathomdb` keeps `[lib] name = "cudarc"`, and a
  guard enforces exactly one cudarc 0.19 in the graph.
- **After publication:** no `[patch.crates-io]` entry remains for cudarc or
  Candle core/nn/transformers; `candle-kernels` keeps its fork git patch.
  Revision 16: both publications precede W6's pins, so W6 reaches this
  state directly.
- **The vendored tree is gone.** `third_party/cudarc-0.19.7`,
  `scripts/tests/test_vendored_cudarc.sh` and every reference to them are
  removed. The item table now lives in each fork's `FATHOMDB.md`.
- **Fork records.** Each fork carries its own engineering record under
  `fathomdb/`.
- **CUDA pin.** `release.yml` stays on CUDA 12.6 with
  `CUDARC_CUDA_VERSION=12060` pinned.

### 3.1 Fork management

| | cudarc fork | Candle fork |
| --- | --- | --- |
| Repository, branch | `coreyt/cudarc`, `fathomdb/0.19.10` (from `ec1cac71`) | `coreyt/candle-fathomdb`, `fathomdb/0.8.28-cudarc-fork` (from `859b8ea1`) |
| Plan (authoritative for fork work) | `fathomdb/plan.md` | `fathomdb/plan.md` |
| Requirements, design, test plan | `fathomdb/requirements.md`, `design.md`, `test-plan.md` (FC1–FC3) | `fathomdb/requirements.md` (requirements, design decisions and tests; FK1) |
| Reviews, status, receipts | `fathomdb/review.md`, `status.md`, `qualification/` | same |
| Item table (ships with the crate for cudarc) | `FATHOMDB.md` | `FATHOMDB.md` |
| Proposal | `proposal/mem-pool` branch; drafts in `fathomdb/proposal/` | none |
| Local clone (one writer at a time) | `../fathomdb-forks/cudarc`; proposal worktree `../fathomdb-forks/cudarc-proposal` | `../fathomdb-forks/candle-fathomdb` |
| CI | fork-only workflow on push (`fathomdb-ci.yaml`, class `W`): upstream's 19-feature × {Linux, Windows} × {default, `no-std`} check matrix, aarch64 Linux check and clippy, a hosted `fathomdb_` test job (also with FathomDB's lock), fmt and a publish dry run; CI receipts in `fathomdb/qualification/` | none hosted (CUDA needs a GPU runner); local GPU cells |

Rules shared by both forks, which their plans state in full:

- **No PR or issue objects.** Never `gh pr create`, `gh issue create`,
  `gh repo set-default` or a web-UI PR from a fork. Upstream push URLs are
  disabled in every clone.
- **Commit classes** `U`, `L`, `P`, `W` (fork CI), `D` (`Fork-Class:`
  trailer on `fathomdb/*` branches; none on `proposal/*`). Engineering
  records are `D` commits and touch only `fathomdb/` (and `FATHOMDB.md` in
  the Candle fork; in the cudarc fork `FATHOMDB.md` ships in the crate, so
  it is `P`).
- **Pins name qualified code commits.** Receipts follow as `D` commits
  naming them. Every pin of `cudarc-fathomdb` (Candle's and FathomDB's)
  uses the identical URL string `https://github.com/coreyt/cudarc.git` and
  the full 40-character `rev`, so Cargo sees one source. **Revision 16:**
  FathomDB pins no fork by git except `candle-kernels`; it consumes the
  published crates, whose `.cargo_vcs_info.json` names the qualified
  commit (`C` for `cudarc-fathomdb`, `K9` for the Candle crates; revision
  17, Q21), so Cargo sees one `cudarc-fathomdb` source, the registry.
- **Publication from the qualified commits.** `cudarc-fathomdb` is
  published from `C` itself; the Candle crates from `K9` (revision 17,
  Q21), which is re-qualified (Candle CK-AC-07 at `K9`, control `K8`).
- **The pin chain.** cudarc commit `C` passes R-FC-VERIFY. The Candle fork
  pins `C`, giving Candle commit `K`, which passes R-FK-VERIFY. FathomDB
  then pins `C` and `K` together, each pin passing R-PIN. Revision 16:
  `C` is published as `cudarc-fathomdb` 0.19.10; revision 17: `K9` (`K`
  with the registry dependency, version 0.11.0, Q21) passes CK-AC-07 and
  R-FK-PUB and is published; FathomDB pins those registry versions, each passing R-PIN. Any later cudarc
  code change restarts the chain for the affected cells (each fork plan's
  re-qualification table), and FathomDB re-runs § 7's product cells that
  the change can reach (D-16).
- **Fork branches are never rewritten or force-pushed.** The main thread
  pushes explicit refs after verifying from git.

What FathomDB records (`status.md`, `review.md`, linking fork records by
permalink at the receipt commit's sha), and R-VERIFY checks (AC31-14):

- each fork's plan sha and review verdicts (R-FC-PLAN … R-FC-VERIFY,
  R-FK-PLAN … R-FK-VERIFY), each with no open finding;
- the qualified commits `C` and `K`, and the receipts covering them;
- the UNMEASURED cells with the owner ruling that makes them so.

## 4. Artifacts (`dev/plans/0.8.28/features/slice-31/`)

Before the slice merges these live on `slice/0.8.28-31-cudarc-fork`. After
it merges they live on `slice/0.8.28-31-post` (§ 13), which carries the
later `status.md` and `review.md` entries and W8 step 3. Fork-side artifacts
live in the forks (§ 3.1) and are not copied here.

| Artifact | Written in | Reviewed by |
| --- | --- | --- |
| `requirements.md` — final REQ31/AC31 (from § 5), each fork-backed criterion naming its CF/CK criterion | W1 | R-REQ |
| `design.md` — FathomDB topics (§ 6) with citations; ADR amendment draft | W2 | R-DES |
| `test-plan.md` — every FathomDB test traced to an AC31, host, command, expected result | W3 | R-TEST |
| `review.md` — each FathomDB review's model, findings, dispositions; one line per fork review (fork, sha, verdict, link) | each review | — |
| `status.md` — step log, red/green shas, merge-forward shas, host guards, fork commits `C`/`K` and pin-chain checks | W0 onward | R-VERIFY |
| `qualification.md` — FathomDB product qualification (T-CUDA, T-ORIN, T-PERF) | W6 | R-VERIFY |

The Slice 30 draft `slice-30/upstream-cudarc-package.md` gains a superseded
pointer to the cudarc fork's `fathomdb/proposal/` (written on
`slice/0.8.28-31-post` after R-PROP).

## 5. Requirements and acceptance criteria (draft; finalized in W1, reviewed by R-REQ)

A **fork-backed** criterion passes when the fork's named criterion is
recorded passing at the pinned commit, with that fork's R-…-VERIFY approving
it and no open finding. FathomDB does not re-run the fork's tests.

| Requirement | Acceptance criteria |
| --- | --- |
| **REQ31-01 Fork base.** `fathomdb/0.19.10` is cut from upstream `v0.19.10` (`ec1cac71`; no tag pushed to the fork); the delta is small upstream-style commits. | **AC31-01 (fork-backed):** cudarc CF-AC-01 at `C`. |
| **REQ31-02 Pool correctness.** The private pool never touches the default or current pool; frees are stream-ordered; `CudaSlice`-owned allocations never outlive the pool; leaked pointers are a documented precondition and the `upgrade_device_ptr` doc is corrected; `maxSize`, threshold and trim behave; zero length works; `Send`/`Sync` justified; multi-stream correct. Candle devices built from a private-pool context use it. | **AC31-02 (fork-backed + product):** cudarc CF-AC-02 and CF-AC-03 at `C`; Candle CK-AC-02 at `K` (this is the x86 runtime evidence of a private-pool context driven through Candle, on RTX 3090); FathomDB's private-pool GPU tests (T-POOL-PRODUCT) pass on the Orin (12.6) against `C`/`K`. FathomDB's pool path is not built on x86 (§ 2), so on RTX 3090 FathomDB asserts only the default allocator path and `doctor cuda-allocator` reporting `not_built` (T-CUDA). |
| **REQ31-03 CUDA compatibility.** cudarc compiles for all 19 features with default and `no-std` feature sets on Linux and Windows and runs its GPU tests on driver 580 for 12.6, 13.0, 13.2 and 13.4 bindings; Candle and the FathomDB workspace build and run with the 12.6 toolkit and, if approved, the 13.0 toolkit; no removed or deprecated Driver API. | **AC31-03a (fork-backed):** cudarc CF-AC-04a, CF-AC-04b and CF-AC-04c at `C` (G3 on driver 580, and G5 on a GCP T4 with a newer driver installed on the lab VM only (cudarc ruling Q-F1), are the runtime evidence for the 13.1+ builds, including the 13.2+ `CUmemLocation` union path; FathomDB does not run them, I-18). **AC31-03b (fork-backed + product):** Candle CK-AC-05 at `K`, whose 13.0 toolkit cell K-G2 ran on a GCP T4 with driver 580.178.04 (Q15); FathomDB T-WS-BUILD and T-CUDA cells for 12.6 on `windchill3`, and for 13.0 on a GCP VM with driver branch 580 and the 13.0 toolkit installed on the VM only (Q15); if the owner declines that VM session, the 13.0 cells are UNMEASURED by owner decision. Workspace runtime for 13.1–13.4 is UNMEASURED by owner ruling Q4. |
| **REQ31-04 Jetson Tegra.** Every Slice 30 gate holds on the Orin (JetPack 6, CUDA 12.6) on the new base. Tegra on CUDA 13 is UNMEASURED in Slice 31 (Q5). | **AC31-04:** cudarc CF-AC-05b and Candle CK-AC-05 (its Orin cell) at `C`/`K` (fork-backed); `qualification.md` re-runs the Slice 30 gate rows on the Orin; `doctor cuda-allocator` reports `private`, 3 GiB; the Tegra/CUDA 13 row reads UNMEASURED citing Q5 and todo `TC-b267d940`. |
| **REQ31-05 Fallback.** Items 1–4 act only on aarch64 Linux. | **AC31-05 (fork-backed):** cudarc CF-AC-05 at `C`. |
| **REQ31-06 Vendor excellence.** Upstream APIs used (result-layer `mem_pool`, #544) instead of new bindings; minimal fork deltas; every item has upstream status and removal path in each fork's `FATHOMDB.md`; fork revs never rewritten after a pin; library allocations outside the pool are known (D-14). | **AC31-06:** cudarc CF-AC-06 and Candle CK-AC-03, CK-AC-06 (fork-backed); R-DES records D-10 and D-14; pinned-override gate passes. |
| **REQ31-07 One cudarc.** | **AC31-07:** the guard passes; tests that introduce a registry `cudarc 0.19.*` node, or a second `cudarc-fathomdb` source (a different URL string or `rev`), each turn it red; Candle CK-AC-01 at `K`. |
| **REQ31-08 crates.io.** | **AC31-08 (post-publication):** `scripts/check-tegra-pool-registry-build.sh` with no patch table builds `tegra-pool` and passes the Orin GPU smoke; the published `cudarc-fathomdb` `.cargo_vcs_info.json` sha equals `C`, and the published Candle crates' (0.11.0, Q21) equals `K9` (re-qualified, Candle CK-AC-07). |
| **REQ31-09 Proposals.** (1) Pool primitive only, on `proposal/mem-pool` from upstream `main`, upstream CI clean, tests skip without a device, docs, example and benchmark, issue and PR drafts, coexists with #594. (2) Built-major library loading with the loaded-library report, on `proposal/lib-major` from upstream `main`, independent of (1). Neither is posted. | **AC31-09 (post-merge, fork-backed):** cudarc CF-AC-08 and CF-AC-08b; R-PROP and R-PROP2 approve. |
| **REQ31-10 FathomDB product.** `tegra-pool`, typed errors (`cuda_pool_exhausted`, `cuda_context_lost`), `doctor cuda-allocator`, early-`cuInit` contract unchanged; ADR amended for 2b and for fork-held evidence; docs and changelog; no reference to the vendored tree or its test script remains. | **AC31-10:** full `./scripts/agent-verify.sh` green in the slice worktree with its own `.venv` (§ 9 W0 step 6); Slice 30 product tests unchanged; `git grep -il -e third_party/cudarc -e test_vendored_cudarc -e 'vendored cudarc' -e 'cudarc 0.19.7' -- . ':!dev/plans' ':!dev/adr' ':!dev/todos-and-considerations-ledger.jsonl' ':!CHANGELOG.md'` is empty (plans, ADRs, the ledger and the changelog are history; the ADR amendment marks every vendored-tree statement, including `blast_radius`, as superseded); ADR amendment and changelog present. |
| **REQ31-11 Process.** | **AC31-11:** `status.md` lists red and green shas per FathomDB step, each red re-run by the reviewer; `review.md` records every FathomDB review with model `opus` and every fork review by link; fork CF-AC-09 and CK-AC-08 pass. |
| **REQ31-12 Scope safety.** | **AC31-12:** no upstream issue, comment, PR or push; no PR or issue object on either fork; no commit from this slice on `release/0.8.27`; `windchill3` driver and `/usr/local/cuda` unchanged in every guard log, FathomDB and fork. |
| **REQ31-13 Performance.** No regression from the rebase. | **AC31-13:** Slice 30 gate G9 in full (`slice-30/plan.md` § 4.2) re-run on the Orin against the Slice 30 product: Node ratio ≤ 1.05 per measure with bootstrap 95 % CIs (CI upper > 1.10 → double N once), speed-up over synchronous > 1.5 (CI lower bound), import ≤ S + 5 ms, RSS ≤ S + 32 MiB; Python P / S ≤ 1.15; RTX 3090: median ratio new/0.19.7 ≤ 1.05 for embed steady state. The proposal benchmark is the cudarc fork's T-PERF. |
| **REQ31-15 CUDA version support** (`ADR-0.8.28-cuda-version-support-policy.md`). The 12.6 artifact works with any CUDA 12.x cuBLAS from 12.1 to 12.9 on driver 560 or later and Turing or newer GPUs; it never loads a cuBLAS of another major; an environment with no CUDA 12 cuBLAS, or an unsupported GPU, gets a typed, plain error; the user docs state the floors. | **AC31-15:** T-CUDA-COMPAT C1–C5 pass or are UNMEASURED by owner decision; cudarc CF-REQ-11 (built-major library loading) at `C`; D-18's production check and report pass their RED-first tests; `docs/embedder.md` states the CUDA library floor (12.x), the driver floor (560.28.03 for the 12.6 artifact) and the GPU floor (Turing, compute capability 7.5, on x86; Orin on Jetson). |
| **REQ31-14 Fork management.** Fork work is planned, coded, reviewed and qualified in each fork per § 3.1; FathomDB pins only qualified code commits; the pin chain is consistent; published crates contain no fork engineering records. | **AC31-14:** both fork plans reviewed (R-FC-PLAN, R-FK-PLAN) and every fork review recorded with no open finding; `C` is the cudarc commit pinned by both the Candle fork at `K` and FathomDB; `git diff C <cudarc tip> -- . ':(exclude)fathomdb'` and the Candle equivalent are empty or the tips were re-qualified; the commit-class checks pass (cudarc fork plan § 1.3 over `ec1cac71..`, Candle fork plan § 1 over `859b8ea1..`); cudarc CF-AC-07a at `C` shows no `fathomdb/` path in the package; a fork CI receipt has `head_sha` = `C`. |

**AC sets.** R-VERIFY before merge checks AC31-01…07, 10, 11, 12 (so far),
13, 14 and 15. AC31-09 is checked at R-PROP and R-PROP2 after merge. AC31-08 and AC31-12
(final) are checked at R-CODE-4 after publication. Revision 17 (R-REQ):
R-VERIFY also checks AC31-08 (T-REGISTRY runs in W6) and the new AC31-16
(REQ-068 through the `candle-kernels` patch); R-CODE-4 re-checks AC31-08
from the registry.

**Revision 16.** `requirements.md` (W1) carries these
criteria in final form for the registry pins (AC31-02, 07, 08, 14 and
T-FORK-EVIDENCE name registry versions where this table names git URL and
`rev`). After R-REQ it governs where the two differ. AC31-08's cudarc half
is already evidenced by the cudarc fork's FC10 receipt
`fathomdb/qualification/2026-10-13-FC10-publish-b777f8d.md` (fork commit
`9b0ef75`).

**Revision 17.** `requirements.md` is revised after R-REQ
(APPROVE-WITH-FIXES) for the Candle crates 0.11.0 from `K9` (Q21) and
governs where it differs from this table.

## 6. Design topics (`design.md` answers each with citations)

The pool design topics D-1…D-9, the fork part of D-12, D-13 and the new
D-F1/D-F2 moved to the cudarc fork plan § 5, keeping their numbers. Candle
decisions DK-1…DK-4 are in the Candle fork plan § 5. FathomDB's `design.md`
covers:

- **D-8W Workspace CUDA version.** The workspace pins
  `CUDARC_CUDA_VERSION` and forbids combining it with a different concrete
  `cuda-*` feature. `release.yml` stays at 12060. Covers the workspace build
  with the 13.0 toolkit.
- **D-10 Vendor excellence.** Fork versus upstream APIs, Candle's minimal
  change, removal paths, and `candle-kernels` staying git-pinned.
- **D-11 Packaging and consumption.**
  - **Naming:** naming, `[lib] name` and versioning.
  - **Dependency form:** the embedder's
    `cudarc = { package = "cudarc-fathomdb" }` and the git-to-registry
    switch.
  - **Guards:** the single-cudarc guard.
  - **Marker feature:** its future.
  - **Kernel data types (revision 14, Candle R-FK-VERIFY F2):** the x86
    release build compiles Candle's kernels once at `CUDA_COMPUTE_CAP=75`,
    and the BF16 and F8 kernels exist only for `__CUDA_ARCH__ >= 800`
    (`candle-kernels/src/fill.cu:70-87`). So with that build BF16 and F8
    are unavailable on every GPU, the RTX 3090 included; F16 is present.
    FathomDB must never select BF16 or F8 on x86 (todo
    `TC-30847709`). The embedder uses F32 only today. A multi-architecture
    kernel build (Candle DK-3, todo `TC-323f15b0`) is the way to add them.
- **D-12 (product) Tegra.** `tegra-pool` selection, `doctor
  cuda-allocator`, and the early-`cuInit` contract on the new base.
- **D-14 Allocations outside the pool.** cuBLAS/cuBLASLt workspaces,
  static CUDART internals and module loads, measured in the product. Takes
  input from the cudarc fork's D-2, D-7 and D-F3 (upstream PR #595, a
  retained caller-owned cuBLAS workspace) and Candle's DK-4, so R-DES
  follows R-FK-PLAN. **Candle DK-4 (measured):** the cuBLAS/cuBLASLt
  workspace and cuRAND state are not allocated from any memory pool, so
  D-14 measures per-process device memory (VmRSS on the Orin); default-pool
  counters serve only as a leak check (todo `TC-283d4f3a`).
- **D-15 Teardown and process lifecycle (product).** A lost context or
  device reset before drop (Slice 30 saw SIGSEGV in `cublasDestroy_v2`),
  pools in statics at exit, and fork after init (H-1).
- **D-16 Fork evidence and re-qualification.**
  - **When pins move:** which FathomDB checks and § 7 cells re-run when `C`
    or `K` moves, and how R-PIN confirms the pin chain.
  - **What deleting `test_vendored_cudarc.sh` changes:** today it runs the
    `fathomdb_` tests in FathomDB's hosted CI (`ci.yml` fast tier) against
    FathomDB's own `Cargo.lock`. After the slice the cudarc fork's hosted
    test job runs them (x86_64 and aarch64, once with FathomDB's lock), and
    GPU cells G1/G6 run them on hardware, with receipts at `C`.
  - **The remaining gap is lock drift.** A FathomDB `Cargo.lock` change to
    `cudarc-fathomdb`'s resolved dependencies (for example a Dependabot
    bump of `libloading` or `half`) is not exercised by the fork until the
    fork re-runs with the new lock.
  - **D-16 decides how to close it, either way:**
    - (a) a FathomDB fast-tier check that runs only the fork's pure
      `fathomdb_` tests against FathomDB's lock (hosted, no GPU), or
    - (b) a FathomDB guard that fails when the lock changes that set until
      the fork records a re-run with the new lock (cudarc fork plan § 8).

    **Ruled (owner, 2026-10-11, Q8): (b).** FathomDB does not run cudarc's
    own tests (I-18); the design specifies the guard's inputs (the
    resolved dependency set of `cudarc-fathomdb` in `Cargo.lock`), where
    the fork records the re-run, and the guard's tests (RED on a changed
    set with no matching fork record).
- **D-17 Product adaptation to the fork API.**
  - **Error mapping:** map cudarc's `DriverError` from the private pool to
    FathomDB's typed `cuda_pool_exhausted` and `cuda_context_lost`.
  - **Reporting:** what `doctor cuda-allocator` and the product's pool
    counters (`pool_counters`) report, given the fork's typed and raw
    attribute reads (cudarc D-7, D-8).
  - **Close:** `trim_to` and release at engine close (formerly D-5's "after
    close").
  - **Rename:** any rename the fork's D-1 API choice forces on the embedder.
- **D-18 Loaded-library check in production (REQ31-15).** When CUDA is
  selected, the embedder reads the fork's loaded-library report (cudarc
  D-F4) for every CUDA library it uses (cuBLAS, cuBLASLt, NVRTC if used).
  - **Check:** a library whose reported major differs from the build's is
    refused with a typed error naming the file, its version and the major
    needed, before any GPU work. It is never used.
  - **Report:** the resolved file path and version of each library are
    recorded in the CUDA device report and shown by `doctor` (which verb,
    and the field names, are decided here). This is public surface:
    `dev/interfaces/` and `docs/reference/` are updated in the same change.
  - **Logging:** one structured line per process at CUDA selection, with
    no new egress.
  - **Tests:** RED first, with a decoy wrong-major library (fixture), the
    CUDA-13-only container (T-CUDA-COMPAT C3) and the receipts of C2.

## 7. Test plan topics (`test-plan.md` expands each, traced to AC31)

cudarc's own tests (T-UNIT, T-DEV, T-PROP, T-CONC, T-FAULT, T-CAPTURE,
T-MULTI, T-TEARDOWN, T-PROC, the 19-feature matrix, the driver-only 13.x
cells and the proposal benchmark) are planned, run and reviewed in the
cudarc fork (fork plan § 6). Candle's tests are in the Candle fork (fork
plan § 6). FathomDB runs:

- **T-WS-BUILD** workspace builds with `embed-cuda`/`rerank-cuda`:
  `CUDARC_CUDA_VERSION=12060` with the 12.6 toolkit on `windchill3`;
  `13000` on a GCP VM with driver branch 580 and the CUDA 13.0 apt toolkit
  installed on the VM only (Q15), the same setup as Candle K-G2
  (`fathomdb/scripts/vm-setup-cuda13.sh` in the Candle fork). The VM gets a
  source bundle and returns receipts; nothing is developed on it. Workspace cells needing a 13.1+
  `nvcc` are out of scope (Q4/Q6).
- **T-POOL-PRODUCT** (Orin only: the pool path is built only on aarch64
  Linux with `tegra-pool`, § 2) FathomDB's four Slice 30 Orin GPU tests in
  `fathomdb-embedder`: `forced_cuda_probe_succeeds_when_the_default_memory_pool_is_unavailable`
  (`tests/tegra_fragmented_va_cuda.rs`), and the `#[ignore]` tests
  `gpu_orin_64_builds_every_device_on_a_three_gib_private_pool`,
  `gpu_orin_64_pool_stays_under_max_size_and_is_empty_after_the_embedder_drops`
  and `gpu_orin_64_a_primary_context_reset_is_context_lost_and_teardown_segfaults`
  (`src/cuda_pool_policy_tests.rs`; it runs `gpu_orin_64_context_reset_child`),
  and the Slice 30 product tests, against the W6 registry pins (revision
  16; `requirements.md` AC31-02). A receipt must list each test by
  name with a nonzero run count, never just a pass summary.
- **T-CUDA** product runtime cells:

  | Host / toolkit | Needs | Must pass |
  | --- | --- | --- |
  | RTX 3090, driver 580, 12.6 toolkit | present | embed and rerank end to end on the default allocator path; `doctor cuda-allocator` reports `not_built`; full gate |
  | GCP T4 (or L4), driver branch 580, CUDA 13.0 toolkit on the VM | owner approval of the VM session (Q15) | T-WS-BUILD (13000), embed end to end, `doctor cuda-allocator` reports `not_built`. An RTX 3090 (sm_86) JIT of 13.0 PTX is not measured; an L4 (sm_89) would cover a post-Turing JIT |
  | GCP VM, driver 580, only `libcuda.so.1` present (the unversioned `libcuda.so` diverted on the VM, as for cudarc T-DEV (e), ruling Q15a) | the C3 session, after its remedy step (revision 19, R-TEST finding 2) | the 12.6 build embeds end to end (Candle R-FK-VERIFY F5: every fork receipt loaded the driver as the unversioned `libcuda.so`) |
  | RTX 3090, 13.1–13.4 builds | R590–R615 drivers, never on `windchill3` (Q4, Q11) | UNMEASURED |
  | Orin, JetPack 6, 12.6 | present | T-POOL-PRODUCT, T-ORIN |
  | Orin, JetPack 7.2.x, 13.2/13.4 | reflash | UNMEASURED (Q5) |

  Every run pins `CUDA_VISIBLE_DEVICES` to one RTX 3090 **by UUID**
  (never `all` or an index; Q22, design § 0.1) and logs the
  driver/`/usr/local/cuda` guard.
- **T-CUDA-COMPAT** CUDA version support (REQ31-15; policy ADR):

  | Cell | Where | Must show |
  | --- | --- | --- |
  | C1 PTX driver floor | fast tier, no GPU | every PTX module in the 12.6 build declares `.version` ≤ 8.5 and `.target sm_75` (x86) or `sm_87` (Orin). A release build measured `.version 8.5`, `.target sm_75` on 2026-10-11. NVIDIA lists driver 560.28.03 for CUDA 12.6, which the docs state as the floor. PTX 8.5 maps to the CUDA 12.5 driver (555.42.02) on secondary evidence, untested and not promised. Older drivers cannot be installed on `windchill3`, so no older-driver runtime cell exists |
  | C2 older and newer 12.x cuBLAS | RTX 3090; scratch venv with `nvidia-cublas-cu12` 12.1.3.1 and 12.9.2.10 (PyPI; revision 19 drops the 12.4 cell for proportionality, R-TEST), reached by `LD_LIBRARY_PATH`; embeds and reranks through the x86 CUDA wheel and a short script, because `fathomdb-cli` has no embed or rerank verb | embed and rerank end to end; the receipt names the cuBLAS file actually loaded, from FathomDB's own D-18 report and cross-checked against `/proc/self/maps`, and its reported version, because the system's 12.6 `libcublas.so` is on the loader cache and could be picked silently; cuRAND (major 10) comes from the loader cache and the receipt says so. A missing symbol is a typed error, never a crash: since revision 19 this holds by owner ruling Q26, because D-18 checks the required symbol set before any device is built (reason `missing_symbol`) |
  | C3 no CUDA 12 cuBLAS (revision 18, Q25) | a GCP Spot T4 VM (`scripts/gcp-cuda-lab.sh`, profile `t4`, at most 2 h, one owner-approved session) with NVIDIA driver 580 only: no CUDA toolkit, no cuBLAS. Artifacts (the D-18 test binary, the CUDA CLI, model files) are built on `windchill3` from the W6 tree and pushed; nothing is built on the VM (design § 9.3). Replaces revision 17's `windchill3` Docker image pull | the 12.6 build refuses with the typed `cuda_library_unavailable` error naming the missing CUDA 12 cuBLAS and the remedy (`pip install nvidia-cublas-cu12` plus its `lib` directory on `LD_LIBRARY_PATH`); `doctor cuda-libraries` exits 70 with the refusal; then, after the remedy (both the `nvidia-cublas-cu12` and `nvidia-curand-cu12` library directories on `LD_LIBRARY_PATH`), `doctor cuda-libraries` exits 0 and embed and rerank run on the T4 through the embedder's live test binaries. Since revision 19 the session also runs the `libcuda.so.1`-only cell after the remedy (it would refuse on the S-13 VM, which has only the 13.0 cuBLAS). Estimate: 30-45 minutes, about $0.15-0.20 Spot (at most about $0.55 at the cap, about $1.10 on demand). UNMEASURED if the owner declines the session; the hermetic injected-`LibLoadError` test and the decoy test still cover the refusal |
  | C4 below the GPU floor (revision 18, Q22) | any host, hermetic; the Quadro K620 is never run | a fake `CudaProvider` reporting a compute capability 5.0 device with the floor 7.5: forced `cuda:N` gives `cuda_incompatible` and `auto` falls back to CPU with `cuda_incompatible`, both without a `probe_cuda` call (no device built); `auto` probes only the lowest visible ordinal; the floor comes from the build's `CUDA_COMPUTE_CAP` (design § 9.2). No new error kind |
  | C5 13.0 build, 13.0 libraries | the T-CUDA 13.0 GCP VM (Q15) | the T-CUDA 13.0 row, plus the loaded-library check of C2 |

  **Web evidence (2026-10-11) behind these cells:**
  - NVIDIA's minor-version compatibility page lists CUDA 12.x at driver
    525 or later and 13.x at 580 or later. It says applications that
    compile device code to PTX will not work on older drivers.
  - Applications built with older toolkits keep working on newer drivers
    (binary backward compatibility), so a 12.6 build runs on R580.
  - CUDA libraries change soname with the major (`libcublas.so.12` /
    `.so.13`), and cuBLAS, cuFFT and cuSPARSE 13 drop pre-Turing GPUs. A
    12.x build therefore needs a 12.x cuBLAS, and never a 13.x one.
  - cudarc 0.19.10's `get_lib_name_candidates` (`src/lib.rs:204-243`)
    tries the unversioned name first and also tries `.so.12`, `.so.11`,
    `.so.10`, `.so.9` and `.so.1` whatever the built major. A 13.x build
    can load cuBLAS 12, and a 12.x build can load cuBLAS 13 through a
    `libcublas.so` dev symlink. The cudarc fork fixes this (CF-REQ-11).
- **T-PRODUCT** full `./scripts/agent-verify.sh`; single-cudarc guard red
  then green; the `git grep` check of AC31-10.
- **T-ORIN** Slice 30 gate rows; `doctor cuda-allocator`; one
  Node/V8-fragmented end-to-end run (private pool selected, fallback works).
- **T-TEARDOWN-PRODUCT** (D-15) on the Orin: engine close and process exit
  with the pool alive; a lost context before drop reports
  `cuda_context_lost` without SIGSEGV; behaviour of a child forked after
  init is documented and tested to the extent D-15 decides.
- **T-PERF** AC31-13 measurements.
- **T-FORK-EVIDENCE** (R-VERIFY, no code): for `C` and `K`, the receipts
  named by AC31-01…07 and 14 exist, name that commit, record a lockfile
  SHA-256, and show unchanged host guards; the CI receipt's `head_sha` is
  `C`; the pin-chain diffs are empty; both pins use the identical URL and
  full `rev`. Revision 16: for the registry pins, the published
  `.cargo_vcs_info.json` sha of `cudarc-fathomdb` 0.19.10 is `C` and of
  the Candle crates 0.11.0 is `K9` (revision 17, Q21), and
  `candle-kernels`' git patch `rev` is `K9`.
- **T-PROCESS** (revision 17, R-REQ) the record and guard checks of
  AC31-11 and AC31-12: `status.md` and `review.md` entries, the `gh`
  listings on both forks, the fork push URLs, the ancestry checks against
  `release/0.8.27`, and the host guard logs.
- **T-REGISTRY** registry-consumer check (AC31-08). Revision 17 (R-REQ):
  both publications precede W6, so it runs in W6 and at R-VERIFY, and
  R-CODE-4 re-checks it.

## 8. Order of the whole slice

```text
R0 … R0-rev8 (done; rev8 also served as R-FC-PLAN and R-FK-PLAN part 1)
→ W0 steps 4, 5, 7 (done, except the step-4 download: Q15)
→ cudarc FC1 → R-FC-REQ → FC2 → R-FC-DES → FC3 → R-FC-TEST → FC4–FC7 → R-FC-CODE → FC8 → R-FC-VERIFY
→ Candle FK2 → R-FK-CODE → FK3 → R-FK-VERIFY        (FK2 needs R-FC-VERIFY and R-FK-PLAN)
→ W6 (R-PIN for C and K; needs R-TEST too) → merge-forward → qualification
→ R-CODE-3 + R-VERIFY → merge
→ cudarc FC9 → R-PROP          (FC9 may start once R-FC-CODE approves)
→ cudarc FC9b → R-PROP2        (after FC9, same proposal worktree)
→ cudarc FC10 → R-FC-PUB → owner R-PUB → owner publishes cudarc-fathomdb
→ Candle FK5 → R-FK-PUB → owner R-PUB → owner publishes the Candle crates
→ W8 step 3 → R-CODE-4 → merge
```

**Fork-first (Q14, owner 2026-10-12).** The forks are FathomDB's upstream
dependencies, so their tracks ran first. State on 2026-10-13: the cudarc
fork is qualified at `C = b777f8d` (R-FC-VERIFY), the Candle fork at
`K = 853324fc`, which pins `C` (R-FK-VERIFY). § 8.1 gives the full state.
Revision 16 moves the publications (FC10, FK5) before W6 and holds FC9 and
FC9b; § 8.1 "Update 2026-10-13 (revision 16)" gives the order now in force.

### 8.1 State on 2026-10-13 (revision 15)

**Fork work for the pinned commits is complete.** Both forks are
developed, tested, reviewed and qualified at the commits FathomDB will pin:

| Fork | Qualified commit | Evidence | Reviews |
| --- | --- | --- | --- |
| cudarc (`coreyt/cudarc` `fathomdb/0.19.10`) | `C` = `b777f8d94eedca8334e9331cf520d554b22544b7` | G1–G4 and G5a on `windchill3`, G5b on a GCP T4 (13.1–13.4 builds on a newer driver), G6 on the Orin, G8, a CI receipt with `head_sha` = `C`, CF-AC-07a | R-FC-REQ, R-FC-DES, R-FC-TEST, R-FC-CODE, R-FC-CODE-2, R-FC-VERIFY (APPROVE-WITH-NOTES); no open finding |
| Candle (`coreyt/candle-fathomdb` `fathomdb/0.8.28-cudarc-fork`) | `K` = `853324fc2b9f813eedceb75bef660a06a413ff5b`, pinning `C` | K-G1 (RTX 3090, 12.6, at 86 and 75), K-G2 (GCP T4, driver 580, CUDA 13.0 toolkit), K-G3 (Orin) | R-FK-PLAN, R-FK-CODE and its re-check, R-FK-VERIFY ("fit to pin"); no open finding |

No code changed after `C` or `K`; later fork commits are records only
(the slice `status.md` records the pin-chain checks).

**Fork steps still to come:**

| Step | Kind | Waits for | Then |
| --- | --- | --- | --- |
| cudarc FC10 | publication preparation | done early on the owner's request (CF-AC-07b, R-FC-PUB APPROVE-WITH-FIXES, findings 2–4 fixed) | owner decision on R-FC-PUB finding 1 (the README's relative `FATHOMDB.md` link resolves against the default branch `main`: change the fork's default branch or accept it); owner R-PUB after R-VERIFY; owner publishes `cudarc-fathomdb` 0.19.10 from `C` |
| cudarc FC9 | development (proposal branch `proposal/mem-pool` from upstream `main`, RED first, example, benchmark, issue and PR drafts) | the slice merge (§ 8 order) | R-PROP; never posted without owner sign-off |
| cudarc FC9b | development (`proposal/lib-major`, built-major library loading) | FC9 | R-PROP2; not posted |
| Candle FK5 | development and re-qualification (K8: registry `cudarc-fathomdb` 0.19.10, version 0.10.4; K-G1, K-G3, CK-AC-01, CK-AC-04, dry runs) | the owner's publication of `cudarc-fathomdb` (crates.io rejects git dependencies) | R-FK-PUB; owner R-PUB; owner publishes the three Candle crates from K8 |
| cudarc G7 (Tegra on CUDA 13) | test | CloudJetson vetting (Q5, Q13; todo `TC-b267d940`) | UNMEASURED in Slice 31 |

If FathomDB's W6 or its reviews find a defect in a fork, the fix is made in
that fork and restarts the pin chain there (§ 3.1): fork code review,
re-qualification, then a new R-PIN.

**FathomDB side:** W0 steps 1, 2, 4 (GCP venue, Q15), 5 and 7 are done; step
3 is done (worktree `../fathomdb-worktrees/slice-0.8.28-31`, branch
`slice/0.8.28-31-cudarc-fork`, `status.md` and `review.md` started;
preflight's one hard failure is that no 0.8.28 release-state file exists
yet). Step 6 (the slice `.venv`) comes before the first full gate. W1–W3
have not started.

**Next:** W1 → R-REQ, W2 → R-DES, W3 → R-TEST, then W6 with the owner's
R-PIN of `C` and `K`, qualification, R-CODE-3 and R-VERIFY, the merge,
then FC9 and FC9b, then publication (§ 9 W8).

#### Update 2026-10-13 (revision 16)

The owner reordered publication (Q17, Q18). The tables above stay as the
revision 15 record; this part supersedes them where they differ.

| Step | State (2026-10-13) |
| --- | --- |
| cudarc FC10 | **Done.** R-FC-PUB finding 1 closed by making `fathomdb/0.19.10` the default branch of `coreyt/cudarc` (Q17). The owner approved R-PUB and published `cudarc-fathomdb` 0.19.10 from `C` at 14:02Z, before W1–W3, W6 and R-VERIFY. The registry `.crate` SHA-256 `8780d2fe3fbafc4660e6093abd7681cd9cec7b4d5549a20bc148308e6449caf2` equals the dry-run `.crate`; its `.cargo_vcs_info.json` sha1 is `C`; all 131 source files match `C`. Receipt: cudarc fork `fathomdb/qualification/2026-10-13-FC10-publish-b777f8d.md` (fork commit `9b0ef75`) |
| Candle FK5 | **In progress.** `K8` = `cd64715ead4f86470b195a9010c4f60f7a4c0833` on `fathomdb/0.8.28-cudarc-fork` (record `6b350638`): `candle-core-fathomdb`, `candle-nn-fathomdb` and `candle-transformers-fathomdb` at 0.10.4 on `cudarc = { package = "cudarc-fathomdb", version = "0.19.10", … }`; `candle-kernels` stays a path dependency at 0.10.2, so the published crates name upstream registry `candle-kernels` 0.10.2, as 0.10.3 did (Q2). Local checks pass (one `cudarc-fathomdb v0.19.10` node per crate; the multi-package publish dry run). Next: CK-AC-07's GPU cells (K-G1 on the RTX 3090, K-G3 on the Orin, running), R-FK-PUB, then the owner publishes the three crates in dependency order (Q18) |
| cudarc FC9, FC9b | **Not started, by owner direction** (Q17). The proposal branches will carry no FathomDB material |

`git diff 853324fc cd64715e -- candle-kernels` is empty, and
`candle-kernels/Cargo.toml` inherits nothing from the workspace, so
`candle-kernels` is the same at `K` and `K8`.

The 0.8.28 release state, board and plan exist (`4565f23d3`), so W0 step
3's preflight failure ("no live release") no longer applies.

#### Update 2026-10-13 (revision 19)

W3 is done: R-TEST (Opus) approved `test-plan.md` revision 1 with fixes;
its seventeen findings, its rulings on open points OP-1 to OP-11, its
proportionality cuts and owner rulings Q26–Q28 are applied in
`test-plan.md` revision 2 (REVISED AFTER R-TEST, ready for W6),
`design.md` revision 3 and `requirements.md` ([review.md](review.md)).
Next: W6.

#### Update 2026-10-13 (revision 18)

W1 and W2 are done: `requirements.md` (REVISED AFTER R-REQ, amended after
R-DES) and `design.md` revision 2 (REVISED AFTER R-DES). R-DES (Opus)
approved revision 1 with fixes; its ten findings, its open-point rulings
and owner rulings Q22–Q25 are applied ([review.md](review.md)). Next: W3
(`test-plan.md`) → R-TEST, then W6.

#### Update 2026-10-13 (revision 17)

This part supersedes the revision 16 Candle FK5 row where they differ.

| Step | State (2026-10-13) |
| --- | --- |
| Candle FK5, `K8` | **Passed, superseded for publication.** `K8` (`cd64715e`, 0.10.4) passed CK-AC-07 on the RTX 3090 and the Orin (fork receipt `fathomdb/qualification/2026-10-13-CK-AC-07-cd64715e.md`, fork commit `ddff4cbd`); the receipt stands as evidence for the same code. R-FK-PUB otherwise approved `K8`, but its finding 1 ruled out 0.10.4 (Q21) |
| Candle FK5, `K9` | **In progress.** `K9` = `182ceab0547348993b6dbec4b1aed610dcc8b23f` (record `4abb8b38`): the three crates at 0.11.0, each with a fork `description`; otherwise identical to `K8`, and its `Cargo.lock` differs from `K8`'s only in the three version lines. `K9` is the publication commit. Next: CK-AC-07's cells at `K9` (control `K8`, running), the R-FK-PUB re-check of `K9`, then the owner publishes 0.11.0 in dependency order (Q18, Q21) |

`git diff 853324fc 182ceab0 -- candle-kernels` is empty, so FathomDB's
`candle-kernels` git patch pins `K9`, the publication commit (R-REQ open
point 1). `git diff 853324fc 182ceab0 -- . ':(exclude)fathomdb'
':(exclude)FATHOMDB.md'` changes only the root `Cargo.toml` (the three
fork crates' workspace versions and the `cudarc-fathomdb` source, git to
registry) and the three crates' `version` and `description` lines, so
Candle K-G2 (CUDA 13.0, run at `K` only) carries to `K9` (R-REQ open
point 2).

R-FK-PUB also found that a consumer of the published Candle crates with
no patch table links `-lcudart` dynamically through upstream registry
`candle-kernels` 0.10.2 and needs `LIBRARY_PATH=<cuda>/lib64` to link
(finding 2; T-REGISTRY), and that `flash-attn` on
`candle-transformers-fathomdb` pulls upstream `candle-core` 0.10.2 and a
second cudarc (finding 5; unsupported, FathomDB does not enable it).

**Order now in force:**

```text
FC10 (done) → FK5: CK-AC-07 at K9 → R-FK-PUB re-check → owner publishes Candle 0.11.0
W1 → R-REQ → W2 → R-DES → W3 → R-TEST            (Q19; may run during FK5)
→ W6 (registry pins, R-PIN; GREEN pins wait for the Candle publication)
→ merge-forward → qualification → R-CODE-3 + R-VERIFY → merge
→ W8 step 3 → R-CODE-4 → merge
FC9 → R-PROP, FC9b → R-PROP2: held until the owner releases them (Q17)
```

Each step joins only where its gate says:

- Candle FK1 → R-FK-PLAN may start after R-FC-DES (it uses the pool API); it
  gates only FK2.
- FathomDB W1 → R-REQ (after R-FC-REQ) → W2 → R-DES (after R-FK-PLAN, for
  D-14) → W3 → R-TEST; it gates only W6. FC4 waits only for R-FC-TEST.

The main thread writes documents one checkout at a time and never two
writers in one checkout. Owner-gated steps
(the W0 step 4 download, R-PIN, R-PUB, publication) block only what depends
on them.

## 9. Work packages (each ends at its review; findings fixed before moving on)

### W0. Prerequisites (main thread)

1. Done: plan revisions 6–9, `GOAL.md` and the
   [merge-forward plan](merge-forward-plan.md) committed on `release/0.8.28`;
   revision 8 and both fork plans reviewed by R0-rev8 (APPROVE-WITH-FIXES),
   fixed in revision 9 and fork plan revisions 2. The temporary docs
   worktree `../fathomdb-worktrees/release-0.8.28-plan` was removed after
   pushing revision 9, keeping the branch; no checkout of `release/0.8.28`
   remains.
2. Done: merge-forward at `9fa71e93a`; todo `TC-d71a9e36` closed. Repeat
   before R-CODE-3/R-VERIFY if 0.8.27 advances, per the merge-forward plan,
   in a temporary worktree of `release/0.8.28` (§ 13). Then, with no
   implementer running, merge updated `release/0.8.28` into the slice branch
   and re-run the affected checks.
3. `scripts/preflight.sh`; create worktree
   `../fathomdb-worktrees/slice-0.8.28-31` on `slice/0.8.28-31-cudarc-fork`
   from `origin/release/0.8.28`; start `status.md` on the slice branch with
   the plan and fork-plan shas, and copy the R0-rev8 summary into
   `review.md` (its fork dispositions are in each fork's
   `fathomdb/review.md`). The fork clones and branches already exist and are
   pushed; add the proposal worktree only at FC9.
4. Toolchains, searched outside PATH. Record the guard (driver 580.178.04,
   `readlink -f /usr/local/cuda` → 12.6). **Revision 14 (Q15):** the
   download below was not approved and is not done. The 13.0 toolkit cells
   (Candle K-G2, done; T-WS-BUILD 13000 and the T-CUDA 13.0 rows) run on a
   GCP VM with driver branch 580 and the 13.0 toolkit installed on the VM
   only; each VM session needs owner approval, and Spot VMs are used only
   for test and verification. Revision 13 text, kept as history:
   **Owner approval:** download the CUDA 13.0 runfile; install toolkit-only, user-local, no sudo (§ 2.2).
   Never install or update a driver on `windchill3`; never run the runfile
   with sudo or without `--toolkit`; never install apt `cuda`/driver
   packages. Re-check the guard. The download serves cudarc G4, Candle K-G2
   and FathomDB T-CUDA alike; if it is declined, cudarc G4 can still use
   the cu13 PyPI wheels in a scratch virtual environment (cudarc fork plan
   G4), and the toolkit cells (K-G2, T-WS-BUILD 13000) are UNMEASURED by
   owner decision. The wheels are pinned to 13.0.x (for example
   `nvidia-cuda-runtime==13.0.96`, `nvidia-cuda-nvrtc==13.0.88`,
   `nvidia-cublas==13.0.2.14`), matching the 13.0 driver, and installed
   only into that scratch environment; the primary `.venv`'s `nvidia/cu13`
   libraries (cuBLAS 13.1.1) are never used. PyPI also carries
   `nvidia-cuda-nvcc==13.0.88`; whether it can build Candle's kernels in
   place of the runfile is unmeasured and is not relied on.
5. Orin check over ssh (`10.83.10.13`): disk, GPU lock, toolchain, a clone
   location for the fork tests (`~/fathomdb-forks/`).
6. Python gate environment: create a `.venv` owned by the slice worktree
   with `scripts/bootstrap.sh` run inside it (AGENTS.md § 3: the full gate
   preflights a checkout-owned `.venv`), never touching or borrowing the
   primary checkout's `.venv` (`agent-worktree-stale-base-trap.md`). The
   full gate must be green there; the Python parity oracle of
   `0.8.27-slice132-rust-sdk-lessons.md` only supplements it.
7. Ledger: record the 2026-10-11 directions (I-17, I-18) and rulings Q4–Q12
   with `ledgerwrite`.

### W1. Requirements and acceptance criteria → R-REQ

`requirements.md` from § 5, traced to D28-13, Slice 30's R30/AC30, REQ-068
and the fork CF/CK criteria (after R-FC-REQ, so the IDs are stable).

### W2. Design → R-DES

`design.md` answering § 6 (D-8W, D-10, D-11, D-12, D-14 … D-18) with
citations. ADR amendment draft: 2b consumption (revision 16: through the
published registry crates, with `candle-kernels` by git patch); evidence for the cudarc
primitive now held in the fork rather than in `test_vendored_cudarc.sh`;
every vendored-tree statement, including `blast_radius`
(`ADR-0.8.28-tegra-private-cuda-pool.md:6, 101, 142`), marked superseded.
Records § 15 Q6's finding and the D-16 lock-drift guard (Q8, option (b)).
Revision 18: `design.md` revision 2 (REVISED AFTER R-DES) applies R-DES's
findings and Q22–Q25, adds a second amendment draft for
`ADR-0.8.28-cuda-version-support-policy.md` (D-18 and the GPU floor) and
records fork todos FT-1–FT-3 (design § 13.1).

### W3. Test plan → R-TEST

`test-plan.md`: each FathomDB test, host, command, expected result, AC31.

### W4. cudarc fork track (in `coreyt/cudarc`; cudarc fork plan FC0–FC10)

Performed in the fork per its plan. FathomDB-side duties of the main thread:

- run the fork's reviews (R-FC-PLAN, R-FC-REQ, R-FC-DES, R-FC-TEST,
  R-FC-CODE, R-FC-VERIFY, R-PROP, R-FC-PUB) and add one line per verdict to
  `review.md`;
- record `C`, the receipts and the pin-chain check in `status.md`;
- keep implementer briefs pointed at the fork clone and the fork plan, never
  at the FathomDB worktree.

**Gate to W6 pins:** R-FC-VERIFY approved at `C` with no open finding.

### W5. Candle fork track (in `coreyt/candle-fathomdb`; Candle fork plan FK0–FK5)

Same duties for R-FK-PLAN, R-FK-CODE, R-FK-VERIFY and R-FK-PUB.

**Gate to W6 pins:** R-FK-VERIFY approved at `K`, and `K` pins `C`.

### W6. FathomDB slice (`slice/0.8.28-31-cudarc-fork`), TDD → R-CODE-3 + R-VERIFY

1. **RED:** the pinned-override checker and its test, the single-cudarc
   guard and the CUDA release contract each expect fork sources, no
   vendored tree and no `test_vendored_cudarc.sh` tier entry. T-CUDA-COMPAT
   C1 (the PTX `.version`/`.target` check) and C4 (the hermetic
   compute-capability floor test with a fake provider, design § 9.2, Q22)
   are written red first.
2. **GREEN** (each pin commit gets R-PIN, approved by the owner, Q20):
   - **Pins and dependency (revision 16: registry form).** Revision 15
     pinned `C` and `K` by git URL and full `rev`; both forks are now
     published (or will be, Q18), so FathomDB consumes the registry crates:
     - the embedder's aarch64-Linux `cudarc` dependency is **removed**
       (revision 18, R-DES finding 6, design § 3.2): it is unused (every
       product `cudarc` path is `candle_core::cuda::cudarc::…`) and its
       features are a subset of Candle's; `tegra-pool = []`, and
       `cudarc-fathomdb` 0.19.10 is reached through Candle and checked in
       `Cargo.lock` by the pinned-override checker;
     - `candle-core`, `candle-nn` and `candle-transformers` move from
       `=0.10.3` to `=0.11.0` (packages `candle-*-fathomdb`, registry;
       revision 17, Q21);
     - the root `[patch.crates-io]` drops the `cudarc` path entry and the
       three Candle core/nn/transformers git entries and keeps only
       `candle-kernels` by git
       (`https://github.com/coreyt/candle-fathomdb.git`, full 40-character
       `rev` of `K9`, `182ceab0547348993b6dbec4b1aed610dcc8b23f`), beside
       `libsqlite3-sys`;
     - regenerate `Cargo.lock`.

     **Exact pins (D-11, design § 3.2):** the Candle crates stay exact
     (`=0.11.0`), so a later Candle fork version enters FathomDB builds
     only through R-PIN. `cudarc-fathomdb` has no manifest pin after
     revision 18; FathomDB's lock is held at exactly 0.19.10 (crates.io
     source, published checksum) by the pinned-override checker, and a
     later version enters only through a reviewed lock change, the
     lock-set guard and R-PIN (D-16).

     **`candle-kernels` at `K9` (ruled by R-REQ, revision 17).** The
     `candle-kernels` tree is the same at `K`, `K8` and `K9` (§ 8.1
     revision 17 update). `K9` names one Candle commit for the whole cohort
     and is the commit the crates are published from; the empty
     `candle-kernels` diff `K..K9` is the reason K-G2 (CUDA 13.0, run at
     `K` only) still applies.

     **R-PIN checklist:** the registry versions are exactly
     `cudarc-fathomdb` 0.19.10 in `Cargo.lock` (no manifest pin, revision
     18) and the three Candle crates `=0.11.0`;
     `Cargo.lock` has exactly one `cudarc-fathomdb` package, with the
     crates.io registry source, and no registry `cudarc 0.19` on the normal
     dependency tree; `scripts/pinned-override-rot.json` (and its checker)
     updated in the same commit; the `candle-kernels` patch `rev` equals
     `K9`.
   - **Deletions:** delete `third_party/cudarc-0.19.7`; delete
     `scripts/tests/test_vendored_cudarc.sh` and its tier entry and comment
     (`scripts/agent-test.sh:323-326`); add D-16's lock-drift guard.
   - **Registry build (revision 17, R-REQ):** RED then GREEN
     `scripts/check-tegra-pool-registry-build.sh` for T-REGISTRY
     (`requirements.md` AC31-08). It breaks as soon as the cudarc patch
     entry is removed, at its line 102 ("`[patch.crates-io]` must carry
     exactly one cudarc entry"), so it changes in the same step.
   - **Contract docs:** `dev/interfaces/rust.md:1240` and
     `dev/interfaces/rust-sdk.md:392` (both say "the vendored cudarc").
   - **References to update:** the embedder `Cargo.toml` comments (lines
     59-65 and 143-147), `.gitattributes`,
     `.markdownlint-cli2.jsonc`, root `Cargo.toml`,
     `dev/tegra-platform-reference.md`, `scripts/agent-lint-md.sh`,
     `scripts/check-pinned-override-rot.py`,
     `scripts/pinned-override-rot.json`,
     `scripts/tests/test_check_pinned_override_rot.sh`,
     `scripts/check-tegra-pool-registry-build.sh` (its assertion changes in
     this step; revision 17), and `dev/design/release.md` lines 131-142
     ("registry cudarc 0.19.7", § "Registry-cudarc check").
   - **Kept as is:** `release.yml` keeps `CUDARC_CUDA_VERSION=12060`.
   - **CUDA support docs (REQ31-15):** `docs/embedder.md` states the CUDA
     library floor (12.x; tested with cuBLAS 12.1–12.9), the driver floor
     (560.28.03 for the 12.6 artifact), the GPU floor (Turing on x86:
     GeForce RTX 20 and GTX 16, Quadro RTX and T series, Tesla T4, and
     later), and the CUDA-13-only remedy.
   - **Records:** ADR amendment; changelog.
3. Full `./scripts/agent-verify.sh` (AC-036 needs ptrace; rerun unconfined if
   denied; W0 step 6 for Python receipts).
4. Merge-forward again, including into the slice branch (W0 step 2);
   product qualification (T-WS-BUILD, T-POOL-PRODUCT, T-CUDA, T-ORIN,
   T-PERF) into `qualification.md`, optionally run by a Claude agent on the
   Orin.
5. R-CODE-3 and R-VERIFY (including T-FORK-EVIDENCE), then merge (§ 13).

### W7. Upstream proposal (cudarc fork FC9) → R-PROP

Performed in the fork (`proposal/mem-pool`, drafts in `fathomdb/proposal/`).
On `slice/0.8.28-31-post` the main thread records R-PROP and adds the
superseded pointer to `slice-30/upstream-cudarc-package.md`. Then FC9b
(`proposal/lib-major`, drafts in `fathomdb/proposal-lib-major/`) and
R-PROP2. Neither proposal is posted. Revision 16: FC9 and FC9b are not
started until the owner releases them (Q17); their branches carry no
FathomDB material.

### W8. Publication (owner) → R-FC-PUB, R-FK-PUB, R-PUB, R-CODE-4

1. cudarc fork FC10: dry runs from a detached checkout of `C` (CF-AC-07b;
   crates.io token for the new crate name first) → R-FC-PUB → owner R-PUB →
   owner publishes `cudarc-fathomdb` 0.19.10 from `C`. **Done 2026-10-13,
   before W6 (Q17; § 8.1 update).** AC31-08's cudarc half is evidenced.
2. Candle fork FK5: `K9` (registry `cudarc-fathomdb`, 0.11.0; revision 17,
   Q21), re-qualified per Candle CK-AC-07 (control `K8`) → R-FK-PUB
   re-check → owner R-PUB → owner publishes the three Candle crates from
   `K9`. **In progress; publication approved once R-FK-PUB approves (Q18,
   Q21).** It precedes W6's GREEN pins.
3. On `slice/0.8.28-31-post`: R-CODE-4 re-checks AC31-08 from the
   registry (both halves, and T-REGISTRY) and checks AC31-12 final, then
   merge. Revision 16 moved the patch-entry drop and the registry pins to
   W6; revision 17 (R-REQ) moves the registry-build script change and
   T-REGISTRY to W6 too, so this step holds no code unless R-CODE-4 finds
   a defect.

## 10. TDD rule (all code: FathomDB W6 and W8, and every fork code package)

Failing test first (committed, or the red run recorded in `status.md` of the
repository where the code lives), then the code, then refactor. Test files
are read-only during fix-to-spec. Ported tests count as RED only if they fail
on the base (cudarc FC4). Two failures of the same kind stop the step for a
re-think (AGENTS.md § 5). Reviewers re-run each RED sha and confirm it
fails.

## 11. Roles and subagents

- **Main thread** (session running [GOAL.md](GOAL.md)): writes FathomDB and
  fork documents; creates every tag, branch, worktree and clone; briefs
  implementers; runs reviews; verifies results from git and logs; pushes
  explicit refs; merges. Not the steward or orchestrator; neither is
  spawned.
- **Implementers** (`implementer` type, `.claude/agents/implementer.md`): one
  per checkout — the FathomDB slice worktree, the cudarc clone, the cudarc
  proposal worktree, or the Candle clone — TDD, never pushing. Briefs give
  the absolute path, the branch, the governing plan (the fork plan for fork
  work), the output path for the closure `output.json` (outside the fork
  tree, in the scratchpad), and override the definition's codex mention:
  the reviewer is an Opus subagent run by the main thread. Briefs repeat the
  fork bans: no `gh pr`/`gh issue`, no push.
- **One writer per checkout:** the main thread writes documents in a
  checkout only when no implementer is running there; implementers in
  different checkouts may run concurrently only if their work does not
  depend on each other (in practice the pin chain serializes them).
- **Research subagents** (general-purpose) read NVIDIA, cudarc, Candle and
  Cargo sources and return cited findings.
- **Review spawn:** Agent call with `model: "opus"`, effort high,
  `subagent_type` general-purpose (never steward or orchestrator), read-only
  instruction, the artifact plus its requirements (REQ31/AC31 or CF/CK), and
  a demand for findings with file:line and a failure scenario. Recorded in
  the `review.md` of the repository reviewed.
- **Orin agent:** a Claude agent may run on `10.83.10.13` for long
  qualification runs (fork G6/K-G3 or FathomDB T-ORIN); results come back by
  `scp`; the main thread verifies and commits the receipts.

## 12. Reviews (all Opus subagents unless marked owner)

| # | Repository | When | What | Blocks |
| --- | --- | --- | --- | --- |
| R0 (owner) | FathomDB | done 2026-10-11 | plan revision 3 | — |
| R0-rev4, R0-rev5 | FathomDB | done 2026-10-11 | revisions 4 and 5 + GOAL (fixes in revisions 5 and 6) | — |
| R0-rev8 | FathomDB + forks | done 2026-10-11 | plan revision 8, GOAL and both fork plans (APPROVE-WITH-FIXES: H1–H2, M1–M12, L1–L14; all fixed in revision 9 and fork plan revisions 2); served as R-FC-PLAN and R-FK-PLAN part 1, recorded in each fork's `fathomdb/review.md` | — |
| R-FC-REQ | cudarc | end of FC1 | `requirements.md`: coverage, testability | FC2; FathomDB W1 |
| R-FC-DES | cudarc | end of FC2 | `design.md`, citations checked | FC3; FK1 |
| R-FC-TEST | cudarc | end of FC3 | `test-plan.md`: every CF-AC covered, D-F1 accounting, commands | FC4 |
| R-FK-PLAN | Candle | end of FK1 | `requirements.md` (requirements, DK-1…DK-4, tests), `FATHOMDB.md` | FK2 |
| R-REQ | FathomDB | end of W1 | `requirements.md` | W2 |
| R-DES | FathomDB | end of W2 | `design.md` + ADR amendment draft | W3 |
| R-TEST | FathomDB | end of W3 | `test-plan.md` | W6 |
| R-FC-CODE | cudarc | end of FC7 | whole branch, commit classes, RED re-runs, fork CI vs upstream workflows | FC8; FC9 |
| R-FC-VERIFY | cudarc | end of FC8 | CF-AC set at `C`, receipts | FK2; R-PIN for `C` |
| R-FK-CODE | Candle | end of FK2 | diff from `859b8ea1`, RED re-run | FK3 |
| R-FK-VERIFY | Candle | end of FK3 | CK-AC set at `K`, receipts | R-PIN for `K` |
| R-PIN (owner; approved 2026-10-13, Q20) | FathomDB | each pinned fork rev or registry version | the change (`scripts/pinned-override-rot.json` rule), with the fork verdicts; revision 16 checklist in § 9 W6 step 2 | that pin |
| R-CODE-3 | FathomDB | end of W6 | the slice branch | merge |
| R-VERIFY | FathomDB | with R-CODE-3 | pre-merge AC set incl. T-FORK-EVIDENCE, `qualification.md`, `status.md` | merge |
| R-PROP | cudarc | end of FC9 | proposal branch and drafts (CF-AC-08) | owner sign-off (later) |
| R-PROP2 | cudarc | end of FC9b | second proposal branch and drafts (CF-AC-08b) | owner sign-off (later) |
| R-FC-PUB | cudarc | FC10 | package list and dry run at `C` (CF-AC-07b equals 07a) | owner R-PUB |
| R-FK-PUB | Candle | FK5 | `K8` (APPROVE apart from finding 1, Q21), then the re-check of `K9`: its re-qualification receipts and dry runs (CK-AC-07; formerly R-CODE-2b) | owner R-PUB |
| R-PUB (owner) | forks | before each `cargo publish` | dry-run output, packaged file list | each publish |
| R-CODE-4 | FathomDB | W8 step 3 | AC31-08 re-check from the registry (T-REGISTRY ran in W6, revision 17), AC31-12 final | merge |

Findings go into the reviewed repository's `review.md`; each is fixed RED
then GREEN or given a recorded disposition before the blocked step starts.
FathomDB's `review.md` carries one line per fork review.

## 13. Branch management

| Repo | Branch | Cut from (by the main thread) | Writer | Lands by |
| --- | --- | --- | --- | --- |
| `coreyt/cudarc` | `main` | upstream mirror | none (fast-forward to upstream only) | — |
| `coreyt/cudarc` | `fathomdb/0.19.10` (default branch since 2026-10-13, Q17) | `ec1cac71` (tag `v0.19.10`; no tag pushed) | main thread (docs) or one implementer, `../fathomdb-forks/cudarc` | stays; published 0.19.10 from `C` |
| `coreyt/cudarc` | `proposal/mem-pool` | `upstream/main` | implementer, worktree `../fathomdb-forks/cudarc-proposal` | stays; posted later only with owner sign-off |
| `coreyt/cudarc` | `ci/proposal-mem-pool`, `ci/qualify-<sha>` | proposal + the fork-CI (`W`) commit; or the qualified commit `C` | main thread | deleted after each CI run and receipt |
| `coreyt/candle-fathomdb` | `fathomdb/0.8.28-cudarc-fork` | `859b8ea1` | main thread (docs) or one implementer, `../fathomdb-forks/candle-fathomdb` | stays; publishes 0.11.0 from `K9` (Q21) |
| `fathomadb/fathomdb` | `release/0.8.28` | existing branch | main thread, temporary worktree per merge | holds merges |
| `fathomadb/fathomdb` | `slice/0.8.28-31-cudarc-fork` | `origin/release/0.8.28` | implementer and main thread, serialized; worktree `../fathomdb-worktrees/slice-0.8.28-31` | R-CODE-3 + R-VERIFY |
| `fathomadb/fathomdb` | `slice/0.8.28-31-post` | `origin/release/0.8.28` right after the slice merges | main thread and implementer (W8 step 3), serialized; worktree `../fathomdb-worktrees/slice-0.8.28-31-post` | R-CODE-4, merge |

- **Merge method:** the main thread creates a temporary worktree of
  `release/0.8.28` (no other checkout of it exists), runs
  `git merge --no-ff <slice branch>`, runs the checks the merge touches,
  pushes the explicit branch ref, and removes the temporary worktree,
  keeping the branch.
- Fork branches are never rewritten or force-pushed. FathomDB worktrees are
  removed after their branch lands; fork clones stay until W8 closes.

## 14. Risks

- **Two cudarc copies** (guard, AC31-07, CK-AC-01). **`ug-cuda` → cudarc
  0.17.8** (max CUDA 13.0) is lock-only and must stay disabled.
- **Accidental upstream PR or push from a fork** (both are GitHub forks):
  upstream push URLs disabled; PR and issue objects banned outright (§ 3.1).
- **Evidence drift between fork and FathomDB:** a pin naming a commit the
  receipts do not cover. Pins name qualified code commits; T-FORK-EVIDENCE
  and AC31-14 check the chain.
- **Upstream PR #594** overlaps items 1–3 and 5 (cudarc D-9, FC9); **PR
  #595** (retained caller-owned cuBLAS workspace) bears on D-14, cudarc
  D-F3 and Candle DK-4.
- **Vacuous product evidence on x86:** the pool path is not built there;
  T-POOL-PRODUCT is Orin-only and receipts name each test with a run count.
- **Lock drift after `test_vendored_cudarc.sh` is retired:** closed by the
  D-16 guard (Q8, option (b)).
- **`windchill3` safety:** a driver change would disable the host (Q4). Only
  toolkit-only, user-local installs; every guard stops the slice on change.
- **CUDA 13.1+ runtime** cannot be measured on driver 580 for PTX builds;
  the cudarc fork's driver-only 13.2/13.4 cells cover the binding layouts
  (with per-test RUN/UNMEASURED accounting); the rest is UNMEASURED.
- **Tegra on CUDA 13** UNMEASURED (Q5); follow-up proposed (§ 16).
- **Candle upstream drift:** upstream 0.11.0 (`31f35b14`) is on cudarc
  `^0.19.8` and upstream `main` (`c68b2499`) on `^0.19.10` (revision 13 said
  0.11.0 was on 0.19.10). Candle DK-1 keeps 0.10.2 (Q16); the rebase is
  todo `TC-509dc8a6`. The fork crates' version 0.11.0 (Q21) is a semver
  step for the changed public cudarc dependency, not a rebase: their code
  is still upstream 0.10.2's, and their names (`candle-*-fathomdb`) never
  collide with upstream `candle-core` 0.11.0.
- **`flash-attn` on `candle-transformers-fathomdb`** pulls upstream
  `candle-core` 0.10.2 and a second cudarc (R-FK-PUB finding 5). FathomDB
  does not enable it; the single-cudarc guard excludes the chain, as
  CK-AC-01 does.
- **First publish of a new crate name:** crates.io token bootstrap (FC10).
- **Licensing:** cudarc MIT/Apache-2.0; attribution kept.

## 15. Questions and rulings

- **Q1.** Version — ruled: base v0.19.10, first publication `0.19.10`.
- **Q2.** `candle-kernels-fathomdb` — ruled: no, not now (static CUDART is
  required for FathomDB-built artifacts per
  `ADR-0.8.25-driverless-cuda-runtime-linkage.md`, decision index row 52,
  REQ-068; those are built in the workspace with the git patch).
- **Q3.** Slice number — ruled: 31.
- **Q4.** R615 driver on `windchill3` — ruled: **never**; it would disable
  the host. CUDA 13.1+ runtime cells are UNMEASURED.
- **Q5.** Tegra on CUDA 13 — ruled: UNMEASURED in Slice 31; propose the work
  before the 0.8.28 release (§ 16).
- **Q6.** CUDA 13 on `windchill3` — ruled: build against CUDA 13.0 only,
  without changing the primary 12.6 (§ 2.2); 0.8.28 artifacts stay on 12.6.
  **Why 12.x (history search, 2026-10-11):**
  - **No recorded decision.** No ADR, design, ledger, commit or memory
    records a choice of 12.x over 13.x.
  - **12.6 was what was installed.** CUDA 12.6 was already on `windchill3`
    in June 2026 (`dev/design/0.8.1-embedder-gpu-and-portability.md:24`;
    0.8.7 status: "Driver 580.x (CUDA 13 capable, runs 12.6 builds)").
  - **It was then frozen.** The 0.8.23 release contract froze it as one
    literal shared by the x86 and Jetson builds
    (`scripts/release/cuda-artifact-contract.sh:11-17`).
  - **The binding dependency is the Jetson Orin.** JetPack 6 cannot run
    CUDA 13, and NVIDIA excludes Orin from CUDA 13's Arm toolkit unification
    (`dev/tegra-platform-reference.md:80-83`), so moving the x86 toolkit to
    13.x would split the shared pin.
  - **Incidental:** the 0.8.18 ONNX-CUDA eval used `onnxruntime-gpu`
    1.26.0 with CUDA 12 / cuDNN 9 wheels (not a shipped path).
- **Q7.** Where fork work is planned and run — ruled (owner, 2026-10-11): in
  each fork, tracked by Slice 31 (I-17); FathomDB need not run cudarc's own
  tests (I-18).
- **Q8.** Lock drift after `test_vendored_cudarc.sh` is retired — ruled
  (owner, 2026-10-11): D-16 option (b), a FathomDB guard that fails when
  the lock changes `cudarc-fathomdb`'s resolved dependencies until the fork
  records a re-run with the new lock.
- **Q9.** Crate name — ruled (owner, 2026-10-11): `cudarc-fathomdb`
  (unclaimed on crates.io, 2026-10-11; first publish needs the token
  bootstrap).
- **Q10.** CUDA version support — ruled (owner, 2026-10-11): the proposal's
  importance ratings and actions are accepted, and the floor is CUDA 12.x
  libraries (11.x unsupported). Recorded in
  `ADR-0.8.28-cuda-version-support-policy.md` (decision index row 63).
  Every release re-runs `dev/design/cuda-version-support-review.md`;
  `AGENTS.md` § 1 and the plan template point at it. The 13.0 artifact is
  follow-up work for the release after 0.8.28 (§ 16).
- **Q11.** `windchill3`'s Quadro K620 (Maxwell, sm_50) — ruled (owner,
  2026-10-11): no driver that ends its support. R580 is the last such
  branch, so `windchill3` stays on R580. The K620 is below the x86 kernel
  target (compute capability 7.5), so FathomDB GPU tests use the RTX 3090s.
  Revision 18 (Q22): the K620 is never run, not even for C4, which is a
  hermetic floor test; every new `windchill3` test process names an RTX
  3090 by UUID.
- **Q12.** 12.6 driver floor and CUDA-13-only environments — investigated
  (2026-10-11):
  - **PTX:** a release build's PTX is ISA 8.5 for `sm_75`. NVIDIA lists
    driver 560.28.03 for CUDA 12.6; PTX 8.5 maps to the CUDA 12.5 driver
    (555.42.02) on secondary evidence.
  - **Docs:** they state 560.28.03 (C1).
  - **CUDA-13-only environment:** a 12.6 build needs a CUDA 12 cuBLAS.
    cudarc's cross-major library names are a defect, fixed in the fork
    (CF-REQ-11) and tested by C2 and C3.
- **Q13.** Test venue for CUDA compatibility — open (owner, 2026-10-11):
  - `windchill3` Docker with GPU passthrough is acceptable.
  - Cloud options are being evaluated: GCP first, including whether an MCP
    server exists; x86 and arm64 NVIDIA; driver-branch control for the
    12-on-13 and 13-on-12 runs; cloud Jetson.
  - The research goes to `research/cloud-gpu-test-options.md`.
  - The Docker Hub pull for C3 is held until the owner rules.
  - **Rulings (owner, 2026-10-11):**
    - **`windchill3` Docker:** not chosen yet, because disk is tight.
    - **GCP:** explored first, with a one-VM pilot of the workflow
      (credentials, a Spot T4 VM, driver install, containers pulled on the
      VM, receipts, teardown) before any matrix run.
    - **Agent- or human-driven VM creation** is decided after the pilot.
    - **Pilot (done, 2026-10-12):** CPU and T4 workflow pilots passed
      (`scripts/gcp-cuda-lab.sh`; driver 580.178.04 on a Spot T4).
    - **Fork build trial (done, 2026-10-12, owner request):** each fork
      built and tested on a T4 in two environments, four efforts in total,
      within the 2-hour T4 wall-clock limit (03:11:10Z–03:48:08Z). Driver
      580 throughout; environment 1 is the CUDA 12.6 toolkit, environment
      2 adds the 13.0 toolkit and selects it.
      - cudarc `c075191`: builds pass for `cuda-12060` and `cuda-13000`;
        68/70 and 66/68 tests pass. The two failures in both environments
        are upstream timing assertions (pinned versus pageable copy speed)
        that this host does not meet, failing in all 8 runs. FC3 must
        classify them as host-dependent.
      - Candle `7e55a57`: builds pass with both toolkits; 250 of 251
        `candle-core-fathomdb` tests pass in each, including
        `cuda_from_context`. `ones_gpu` fails on any pre-Ampere GPU because
        the BF16/F8 fill kernels are built only for sm_80+. FathomDB must
        never select BF16 on Turing, and the FK cells must expect this
        test on sm_75.
      - Every CUDA library each test opened came from the selected toolkit
        (`LD_DEBUG=files`).
      - Not qualification: the fork plans' GPU cells still run on
        `windchill3` and the Orin. Details and receipts:
        [research](research/cloud-gpu-test-options.md) § "Fork build trial
        on a T4".
    - **AWS `g5g` arm64 cells:** a todo for 0.8.29.
    - **Jetson on CUDA 13 (Q5):** use CloudJetson
      ([cloudjetson.com](https://cloudjetson.com/)), not a local reflash.
      Vet the vendor first: JetPack 7.2 availability, root access, and data
      handling.
- **Q14.** Order of work — ruled (owner, 2026-10-12): "To me, it seems we
  should get the 'upstream' work done first, and the forked libraries are
  upstream dependencies to fathomdb", then "I agree with this sequence of
  work". The cudarc fork (FC1–FC8) and the Candle fork (FK1–FK3) are
  qualified first; FathomDB's W1–W3 and W6 follow (§ 8). Steward ledger
  seq 313.
- **Q15.** CUDA 13.0 toolkit venue — ruled (owner, 2026-10-12): the 13.0
  toolkit cells run on a GCP VM with driver branch 580 and NVIDIA's 13.0
  apt toolkit installed on the VM only; the `windchill3` runfile download
  is not approved. "Spot instances can only be test and verification, no
  development (potential code loss)": a VM gets a source bundle and returns
  receipts. Each new VM session needs owner approval, at most 2 hours wall
  clock; capacity order Spot T4, then on-demand T4, then L4. Candle K-G2
  ran this way on 2026-10-13 (19 minutes). **Q15a:** on the lab VM only,
  the unversioned `libcuda.so` may be moved aside with `dpkg-divert` to
  test loading with only `libcuda.so.1`. Steward ledger seq 315, 316.
- **Q16.** Candle base — ruled (owner, 2026-10-12, Candle DK-1): stay on
  upstream Candle 0.10.2; the rebase onto 0.11.x/main is todo
  `TC-509dc8a6`. Steward ledger seq 317.
- **Q17.** Default branch, publication and proposals — ruled (owner,
  2026-10-13): make `fathomdb/0.19.10` the default branch of
  `coreyt/cudarc`, so the fork README's `FATHOMDB.md` link resolves
  (R-FC-PUB finding 1); the `cudarc-fathomdb` package is unchanged, and the
  upstream proposal branches keep upstream's README and carry no FathomDB
  material. Publish `cudarc-fathomdb` 0.19.10 now, before W1–W3, W6 and
  R-VERIFY (§ 8 had FC10 after R-VERIFY); R-PUB approved. Do not prepare
  FC9 or FC9b yet. Published 2026-10-13 14:02Z from `C` (§ 8.1 update).
  Steward ledger seq 318, 319.
- **Q18.** Candle publication — ruled (owner, 2026-10-13): publish
  `candle-core-fathomdb`, `candle-nn-fathomdb` and
  `candle-transformers-fathomdb` 0.10.4 (`K8`, registry `cudarc-fathomdb`
  0.19.10) in dependency order once R-FK-PUB approves. Steward ledger
  seq 320. The version and commit are superseded by Q21 (0.11.0 from
  `K9`); the dependency order and the approval stand.
- **Q19.** W1–W3 — ruled (owner, 2026-10-13): run in full as planned,
  `requirements.md` (R-REQ), `design.md` (R-DES) and `test-plan.md`
  (R-TEST), each with its own Opus review, before W6. Steward ledger
  seq 321.
- **Q20.** R-PIN — ruled (owner, 2026-10-13): approved for the cudarc and
  Candle pins of W6 (identical URL and full revs or registry versions, one
  `cudarc-fathomdb` in `Cargo.lock`, pinned-override rules updated in the
  same commit, pin equals the qualified commit). Steward ledger seq 322.
- **Q21.** Candle fork version — ruled (owner, 2026-10-13): the three
  Candle fork crates publish as 0.11.0, not 0.10.4 (supersedes Q18's
  version and commit). Reason: R-FK-PUB finding 1. Moving 0.10.3 to 0.10.4
  replaces candle-core's publicly re-exported dependency (`pub use
  cudarc;`; 0.10.3 used upstream registry `cudarc` 0.19.4) with a
  different crate, `cudarc-fathomdb`, which is semver-breaking. R-FK-PUB
  otherwise approved `K8`. The publication commit is `K9` =
  `182ceab0547348993b6dbec4b1aed610dcc8b23f` (record `4abb8b38`): `K8`
  with versions 0.11.0 and a fork `description` on the three crates.
  FathomDB pins the Candle crates `=0.11.0` and the `candle-kernels` git
  patch at `K9`. Steward ledger seq 324.

- **Q22.** The Quadro K620 and C4 — ruled (owner, 2026-10-13, R-DES
  Q-A): the `windchill3` Quadro K620 is never run, including for
  T-CUDA-COMPAT C4. C4 becomes hermetic: a fake-provider RED test of a
  probe-time compute-capability floor taken from the build's
  `CUDA_COMPUTE_CAP`, checked before any device is built. Every new
  `windchill3` test process names an RTX 3090 by UUID in
  `CUDA_VISIBLE_DEVICES`. Design § 0.1 and § 9.2; AC31-15. Steward ledger
  seq 325.
- **Q23.** Lock-set closure — ruled (owner, 2026-10-13, R-DES Q-B): the
  D-16 `cudarc-fathomdb` lock-set guard covers only the Linux-target
  normal dependency closure (what the fork compiles), not build/dev or
  Windows/WASI-only crates. Design § 7.2; AC31-14. Steward ledger seq 326.
- **Q24.** D-18 logging — ruled (owner, 2026-10-13, R-DES Q-C): the
  loaded-CUDA-library check writes a stderr line only when FathomDB
  refuses (missing or wrong-major cuBLAS); `doctor cuda-libraries` always
  reports. Design § 9.1. Steward ledger seq 327.
- **Q25.** C3 venue — ruled (owner, 2026-10-13, R-DES Q-D): explore
  running T-CUDA-COMPAT C3 (no CUDA 12 cuBLAS present) on a GCP instance
  instead of a container image pull; design the cell (driver only, no CUDA
  toolkit libraries) and its cost; each VM session still needs owner
  approval; the hermetic injected-load-error test stays. Designed in
  design § 9.3 (Spot T4, driver 580 only, artifacts built on `windchill3`,
  30-45 minutes, about $0.15-0.20); AC31-15 makes C3 the GCP cell or
  UNMEASURED if the owner declines. Steward ledger seq 328.
- **Q26.** Missing symbols — ruled (owner, 2026-10-13, R-TEST owner
  question 1): require a code fix so that a CUDA 12 cuBLAS or cuRAND that
  lacks a symbol FathomDB or Candle needs gives a typed refusal
  (`LibraryUnavailable`, reason `MissingSymbol`), never a panic. D-18
  extends from a major-version check to a required-symbol check; the C2
  floor cells then test it. Designed in design § 9.1 (revision 3): the
  check is in FathomDB, through the fork's existing `is_symbol_present`,
  over a checked-in set taken from cudarc's `cublas` and `curand`
  `result` layers, with a fast-tier drift guard; no fork change. Steward
  ledger seq 329.
- **Q27.** S-13 sizing — ruled (owner, 2026-10-13, R-TEST owner question
  2): the S-13 GCP CUDA 13.0 build session may use `n1-standard-8` or
  larger for one build attempt (`scripts/gcp-cuda-lab.sh create
  --machine-type`); many attempts to obtain an available VM are allowed;
  the 2-hour per-session limit is still enforced; receipts are pulled
  after every cell; each session still needs owner go-ahead. Test plan
  § 1.3 and § 4.7. Steward ledger seq 330.
- **Q28.** G5 and Slice 30 — ruled (owner, 2026-10-13, R-TEST owner
  question 3): the G5 check that the embedding output hash on the new
  crates equals the Slice 30 hash is report-only, evaluated after the
  work, not a gate; G5's P-versus-S identity still gates. AC31-04; test
  plan § 4.2. Steward ledger seq 331.

## 16. Ledger and follow-ups

- Todo `TC-d71a9e36-4509-470e-a677-525ea91193e4`: done in the merge-forward.
- Todo `TC-b267d940-1add-4d08-a596-a7669ae4a500`: the Q5 Tegra/CUDA 13
  follow-up below.
- **W0 step 7 (done, 2026-10-13):** I-17, I-18 and rulings Q4–Q16 plus
  the cudarc fork's Q-F1 are steward ledger seq 303–317 (Q7 is I-17; Q12
  was an investigation, recorded in § 15). `FATHOMDB-PATCH.md` item 5 reads
  "on the fork, not posted" until W6 deletes the file.
- **Candle DK-1 (Q16):** the rebase onto upstream 0.11.x/main is todo
  `TC-509dc8a6-7d4a-415c-9f4b-f3bb1bd07021` (0.8.29).
- **Todo for 0.8.29 (Q13):** the AWS `g5g` (Graviton2 + T4G) arm64 cells
  for the non-Tegra aarch64 path: `TC-ff476f59-727e-4e5b-abc8-5cf438c32143`.
- **Follow-up todo (Q10):** a CUDA 13.0 artifact beside 12.6 in the release
  after 0.8.28 (x86, and Orin on JetPack 7.2 once Q5 is measured), with
  artifact naming and `auto` selection designed there:
  `TC-a5afe8b8-3731-4cf6-9150-8e050bc2d313`.
- **Candle DK-3 (consideration):** multi-architecture kernels (SASS + PTX)
  for x86, the only way to bring BF16/F8 into the x86 artifact:
  `TC-323f15b0-4474-490e-8abc-638861d67368`.
- **Facts recorded for design (watching):** no BF16/F8 in the x86 cc 75
  build (`TC-30847709-9ccd-4b68-a2ec-c027aa469ae8`); Tegra pool sizing
  facts (`TC-aea347e9-f3b3-4dbb-b671-4be4b6010012`); Candle allocations
  outside the pool and how D-14 measures them
  (`TC-283d4f3a-83fa-4192-a078-32b71e790c9b`).
- **Proposed follow-up (Q5):** before the 0.8.28 release, qualify Tegra on
  CUDA 13:
  - on a CloudJetson AGX Orin running JetPack 7.2.x (CUDA 13.2, then 13.4
    via `cuda-compat-orin-13-4`), not a reflash of our Orin (Q13);
  - re-measure whether the default-pool failure persists, bounding
    exhaustion tests per NVIDIA known issue 5699079 (a large allocation may
    reboot an L4T 39.2 device);
  - run the cudarc fork's G7 cells and FathomDB's T-ORIN;
  - restore JetPack 6 for the shipping target.

  Under Q13 this runs on a CloudJetson device after vendor vetting, so no
  reflash of our Orin is needed. Todo
  `TC-b267d940-1add-4d08-a596-a7669ae4a500` (opened 2026-10-11, updated
  2026-10-13; not started).
