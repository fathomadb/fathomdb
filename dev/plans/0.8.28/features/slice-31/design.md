---
title: FathomDB 0.8.28 Slice 31 — design (W2)
status: revision 4 (2026-10-13), § 14 DRAFT FOR REVIEW (N1 and D-19, owner rulings Q29, Q30); revision 3 sections ready for W6
target_release: 0.8.28
observed_on: 2026-10-13
---

# Slice 31 design

This is W2 of [plan.md](plan.md) (revision 18): the FathomDB design topics
of plan § 6, each with its decision, evidence, consequences for W6 and the
tests that prove it. It answers every point that
[requirements.md](requirements.md) leaves to R-DES, and carries the two ADR
amendment drafts (appendix A, the Tegra pool ADR; appendix B, the CUDA
version support policy ADR). Owner ruling Q19 gives it its own Opus
review, R-DES, before W3 and W6.

**Revision 2** applies R-DES (Opus, APPROVE-WITH-FIXES, findings 1–10 and
the open-point rulings, [review.md](review.md)) and owner rulings Q22–Q25
(steward ledger seq 325–328): the compute-capability floor is a hermetic,
unconditional check (§ 9.2, Q22); every new `windchill3` process names an
RTX 3090 by UUID (§ 0.1); the lock-set guard covers the Linux-target
normal closure (§ 7.2, Q23); D-18 is `LibraryUnavailable`, reached
through Candle's re-exported cudarc, with a stderr line only on refusal
(§ 9.1, Q24); T-CUDA-COMPAT C3 is a GCP cell (§ 9.3, Q25); the embedder's
direct `cudarc` dependency is removed (§ 3.2); the guards' fixtures are
consistent and each fact has one checker (§§ 3.4, 3.5).

**Revision 3** applies what R-TEST (Opus, APPROVE-WITH-FIXES on
`test-plan.md` revision 1, [review.md](review.md)) and owner rulings
Q26–Q28 (steward ledger seq 329–331) change in the design: the release
contract keeps only the `candle-kernels` git patch at `K9`, read from the
pinned-override record (§ 1, § 11; R-TEST OP-1); a direct `cudarc-fathomdb`
dependency must not be declared in any form (§ 3.4); D-18 also refuses a
CUDA 12 cuBLAS or cuRAND that lacks a symbol FathomDB or Candle calls
(§ 9.1, Q26); the remedy names both the cuBLAS and the cuRAND wheel
directories (§ 9.1); the C3 test that needs a host without cuBLAS is
`#[ignore]` (§ 9.1); the C3 and S-13 cells drive embeds through the
embedder's live test binaries, because `fathomdb-cli` has no embed or
rerank verb, and the `libcuda.so.1`-only cell moves into the C3 session
(§ 9.3).

**Revision 4** adds § 14 after Orin window P: N1 is implemented (owner
ruling Q29) and D-19 gives a plain Rust process the private pool through
a first-use init at the allocator decision (owner ruling Q30).

Pool design topics D-1…D-9, the fork part of D-12, D-13 and D-F1…D-F4 are
the cudarc fork's (`fathomdb/design.md` revision 4 on `coreyt/cudarc`
`fathomdb/0.19.10`); DK-1…DK-4 are the Candle fork's
(`fathomdb/requirements.md` § 4 on `coreyt/candle-fathomdb`
`fathomdb/0.8.28-cudarc-fork`). § 10 lists where this design depends on
them.

## 0. Inputs and verified state (2026-10-13)

| Fact | Evidence |
| --- | --- |
| `cudarc-fathomdb` 0.19.10 is on crates.io, published from `C` = `b777f8d94eedca8334e9331cf520d554b22544b7`; `.crate` SHA-256 `8780d2fe3fbafc4660e6093abd7681cd9cec7b4d5549a20bc148308e6449caf2` | cudarc fork receipt `fathomdb/qualification/2026-10-13-FC10-publish-b777f8d.md` (record commit `9b0ef75`) |
| `candle-core-fathomdb`, `candle-nn-fathomdb`, `candle-transformers-fathomdb` 0.11.0 are on crates.io, published 2026-10-13 16:51Z from `K9` = `182ceab0547348993b6dbec4b1aed610dcc8b23f`; each registry `.crate` equals its dry run and its source files equal `K9` (104/104, 42/42, 206/206) | Candle fork receipt `fathomdb/qualification/2026-10-13-FK5-publish-182ceab0.md` (record commit `e9c6ed5e`); CK-AC-07 at `K9`: `2026-10-13-CK-AC-07-182ceab0.md` (record commit `e302d54c`) |
| The published Candle crates depend on upstream registry `candle-kernels` 0.10.2 (dynamic `libcudart`); `git diff 853324fc 182ceab0 -- candle-kernels` is empty | plan § 8.1 revision 17 update; requirements § 1 |
| `K9` names `cudarc-fathomdb` as `{ package = "cudarc-fathomdb", version = "0.19.10" }` with features `std`, `cublas`, `cublaslt`, `curand`, `driver`, `nvrtc`, `f16`, `f8`, `cuda-version-from-build-system`, `dynamic-loading`, `default-features = false` | `git show 182ceab0:Cargo.toml` lines 43-54 |
| The published `cudarc-fathomdb` keeps `[lib] name = "cudarc"` and declares `fathomdb-private-pool = []` | `git show b777f8d:Cargo.toml` lines 28-29, 64 |
| **The W6 lock, emulated.** In a scratch copy of the slice tree (tip `bb4ee7095`), with the W6 manifest edits of plan § 9 W6 step 2 applied by hand, `cargo metadata` changed `Cargo.lock` minimally (17 insertions, 12 deletions): it adds `cudarc-fathomdb` 0.19.10 (registry, checksum `8780d2fe…`, dependencies `float8`, `half`, `libloading 0.9.0`), the three Candle crates 0.11.0 (registry, checksums equal to the FK5 receipt) and `candle-kernels` 0.10.2 from `git+…?rev=182ceab0…`; it removes `cudarc` 0.19.7. The only `cudarc` left is 0.17.8 (`ug-cuda`, lock only). No `candle-flash-attn` entry exists. Scratch lock SHA-256 `a44490e42d32f38437c847c548befb5c6dcc571eda50489a92344cffea81449d` | measured 2026-10-13; scratch deleted afterwards. W6 regenerates the real lock; this is evidence for the guard designs, not the W6 lock |
| In that scratch tree, `cargo tree -e normal --target <t> -p fathomdb-embedder --features <f> --prefix none` shows exactly one `cudarc-fathomdb v0.19.10` node and no `cudarc v0.19` node for `t` in {`x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`} and `f` in {`embed-cuda`, `rerank-cuda`, `embed-cuda,tegra-pool`, `rerank-cuda,tegra-pool`} | measured 2026-10-13 |
| In that scratch tree, measured with the marker still enabled (revision 1's emulation kept `tegra-pool = ["dep:cudarc", "cudarc/fathomdb-private-pool"]`), the features resolved on `cudarc-fathomdb` (aarch64, `embed-cuda,rerank-cuda,tegra-pool`) were Candle's set plus `fathomdb-private-pool`. **After W6** (marker and direct dependency removed, § 3.2, § 3.3) the resolved set is Candle's alone: `cublas`, `cublaslt`, `cuda-version-from-build-system`, `curand`, `driver`, `dynamic-loading`, `f16`, `f8`, `nvrtc`, `std`; no concrete `cuda-NNNNN` feature | revision 1: `cargo tree -e features -i cudarc-fathomdb`, 2026-10-13. Revision 2: `cargo metadata --filter-platform` on a scratch crate depending on `cudarc-fathomdb` `=0.19.10` with exactly `K9`'s feature list (both Linux targets give that list), 2026-10-13; scratch deleted |
| Every `cudarc` symbol the product uses on x86_64 is reached through Candle's re-export `candle_core::cuda::cudarc::…` (`candle_bge.rs:194, 304, 352, 397`, `candle_reranker.rs:60, 234, 282`, `cuda_driver_init.rs:52`, `gpu_witness.rs:742-823`, `cuda_pool_policy.rs:1202, 1344`). No source file names the embedder's own `cudarc` dependency (`grep -rn 'cudarc::' src tests` finds only `candle_core::cuda::cudarc::` paths) | `grep`, 2026-10-13 |
| The embedder's direct `cudarc` dependency is aarch64-Linux-only (`Cargo.toml:66-72`) with features `std`, `driver`, `dynamic-loading`, `cuda-version-from-build-system`, a subset of `K9`'s set; it has no `cublas` or `curand` feature | embedder `Cargo.toml:59-72` |

Names used below: `C`, `K`, `K8`, `K9` as in requirements § 1. Paths
without a repository are FathomDB paths at `bb4ee7095`; embedder paths are
under `src/rust/crates/fathomdb-embedder/`.

### 0.1 Host rule for `windchill3` processes (Q22)

The Quadro K620 is never run, for any cell (owner ruling Q22, steward
ledger seq 325; plan § 15 Q11, Q22). Every new process this design starts
on `windchill3` that can reach CUDA (the D-18 decoy test, the report test,
T-CUDA-COMPAT C2, any local C3 step, D-14 or T-CUDA runs, the T-REGISTRY
resolution step if it runs a binary) sets `CUDA_VISIBLE_DEVICES` to one RTX
3090 **by UUID**, never `all`, never an index. The runner reads it with
`nvidia-smi --query-gpu=uuid,pci.bus_id --format=csv,noheader` and takes
the row whose bus id is `00000000:41:00.0` (the default 3090); the
receipt records the UUID. A visible ordinal 0 in such a process is
therefore that 3090: the decoy test's RED run, which reaches
`new_cuda_device(0)` today, builds its device on the 3090. Builds set
`CUDA_COMPUTE_CAP` explicitly (75), so Candle's kernel build never queries
`nvidia-smi` for a compute capability either.

## 1. D-8W Workspace CUDA version

**Decision.**

1. The workspace selects the CUDA binding version only through
   `CUDARC_CUDA_VERSION` or, when it is unset, cudarc's `nvcc --version`
   detection (feature `cuda-version-from-build-system`). No FathomDB
   manifest enables a concrete `cuda-NNNNN` feature on `cudarc-fathomdb`,
   and Candle 0.11.0 enables none either.
2. A guard forbids a concrete `cuda-NNNNN` feature on `cudarc-fathomdb` in
   the resolved graph (condition D of the single-cudarc guard, § 3.5).
3. `release.yml` keeps `CUDARC_CUDA_VERSION: "12060"` in its four CUDA jobs,
   and the release contract checker now requires it beside `CUDA_PATH`.
4. The 13.0 workspace cell (T-WS-BUILD 13000) runs on the owner-approved
   GCP VM (Q15) with `CUDARC_CUDA_VERSION=13000`, `CUDA_PATH`, `CUDA_HOME`
   and `PATH` all naming the VM's 13.0 toolkit, so the binding major and the
   `nvcc` that builds Candle's PTX agree. It is a build-and-run cell, never a
   release artifact (policy ADR, decision 2).

**Evidence.**

- `build.rs` at `C`: `detect_version_from_env` (lines 57-70) panics on an
  unsupported value and wins over features; with the env set it emits
  `cuda-{major}0{minor}0` as a cfg (line 113) in addition to any feature
  cfg, so a concrete feature of another version would give two version
  cfgs (plan § 2.3, "cudarc version selection"). With neither, it runs
  `nvcc --version` (lines 118-126, 158-183).
- The resolved feature set has no concrete version feature (§ 0, last row).
- `release.yml` lines 363, 626, 771 and 898 set `CUDARC_CUDA_VERSION:
  "12060"`; no FathomDB checker reads it (`git grep CUDARC_CUDA_VERSION --
  scripts` finds only `scripts/gcp-lab/remote-fork-cell.sh:86`, the GCP
  fork-cell runner that sets it).
- `scripts/check-cuda-release-contract.py`
  `require_x64_cuda_toolkit_environment` (lines 207-218) already requires
  `CUDA_PATH` and `LIBRARY_PATH` per CUDA job; the toolkit root is the
  contract literal `/usr/local/cuda-12.6`
  (`scripts/release/cuda-artifact-contract.sh:16-17`).
- Why 12.x at all (Q6): plan § 15 Q6; the Jetson Orin on JetPack 6 binds
  the shared 12.6 pin (`dev/tegra-platform-reference.md:80-83`).
- The 13.0 toolkit cell setup is Candle K-G2's
  (`fathomdb/scripts/vm-setup-cuda13.sh` in the Candle fork; receipt
  `2026-10-12-K-G2-853324fc.md`).

**Consequences (W6).**

- `scripts/check-cuda-release-contract.py`: `require_x64_cuda_toolkit_environment`
  also requires `CUDARC_CUDA_VERSION: "12060"`, derived from the toolkit
  root's `12.6` (one literal, not a second copy).
- `scripts/tests/test_cuda_release_contract.sh`: a mutation arm that removes
  the line from one job, RED first. It asserts the checker's message
  through a new helper `expect_fail_with <message>`, not only a nonzero
  exit (today's `expect_fail`, lines 73-80, checks only the exit status).
- **The release contract's Candle block (revision 3, R-TEST OP-1).**
  `check-cuda-release-contract.py` pins all four Candle packages to
  `859b8ea1` by `[patch.crates-io]` git and lock source (`CANDLE_GIT_REV`
  and `CANDLE_PACKAGES`, lines 120-126; `main`, lines 698-725). W6 reduces
  it to the static-CUDART fact of REQ31-16: the `candle-kernels` git patch
  at `K9` and its single git lock entry. The checker reads the `K9` rev
  from `scripts/pinned-override-rot.json` (`cargo_candle_exception.rev`,
  § 3.4) instead of holding a third literal, and leaves the three
  registry Candle crates to the pinned-override checker (one checker per
  fact). In `test_cuda_release_contract.sh` the arms `rejects a split
  Candle source selection` (line 540) and `rejects a Candle lockfile that
  falls back to crates.io` (line 554) are replaced by arms that assert
  the checker's message with `expect_fail_with`: a `candle-kernels` patch
  at another rev, a Candle fork crate reintroduced as a git patch, and a
  `candle-kernels` lock entry from crates.io.
- `scripts/check-single-cudarc.sh` condition D (§ 3.5).
- No manifest change for D-8W.

**Tests.** `test_cuda_release_contract.sh` arm
`release_cuda_jobs_pin_the_cudarc_binding_version` (RED: the checker
passes a workflow without the line); single-cudarc guard arm
`concrete_cuda_feature_on_cudarc_fathomdb_is_rejected` (§ 3.5); T-WS-BUILD
12060 and 13000 (AC31-03b).

## 2. D-10 Vendor excellence

**Decision.**

1. **Upstream APIs first.** The product uses upstream cudarc's result layer
   wherever one exists (`result::mem_pool::alloc_async`,
   `result::mem_pool::get_attribute` from upstream #544,
   `result::free_async`, `result::primary_ctx::*`, `result::ctx::*`,
   `result::device::*`) and the fork's item-5 names only for what upstream
   lacks: `CudaMemPool`, `MemPoolProps`, `CudaContext::new_with_mem_pool`,
   `AllocMode`, `alloc_mode()`. D-17 moves the pool counters to the fork's
   typed reads, which are item 5's proposed upstream shape (fork D-1).
2. **Candle's change stays minimal:** the product uses one fork-only Candle
   API, `Device::new_cuda_from_context` (Candle item K5); everything else is
   upstream Candle 0.10.2.
3. **Removal paths** live in each fork's item table, not in FathomDB:
   cudarc `FATHOMDB.md` items 1-9 (shipped in the crate) and Candle
   `FATHOMDB.md` items K1-K9 (repository root). FathomDB's
   `scripts/pinned-override-rot.json` records which fork items it relies
   on (§ 3.4), so a fork item cannot change under FathomDB without a
   reviewed checker change.
4. **`candle-kernels` stays a git patch at `K9`** (Q2): it is the only path
   by which the fork's static CUDART (K4, K6) reaches FathomDB-built
   artifacts, which REQ-068 requires (REQ31-16). There is no
   `candle-kernels-fathomdb` crate.
5. **Fork revisions are never rewritten**, and every fork version FathomDB
   uses is an exact registry version (D-11) whose `.cargo_vcs_info.json`
   names a qualified commit.

**Evidence.**

- Product imports of the fork API: `cuda_pool_policy.rs:1344-1346`; all
  other cudarc calls are result-layer (§ 8 table).
- `git show b777f8d:src/driver/result.rs` and the vendored
  `third_party/cudarc-0.19.7/src/driver/result.rs` export the same public
  function list (a `diff` of their `pub fn` / `pub unsafe fn` / `pub mod`
  lines is empty, 2026-10-13).
- Candle item table: `FATHOMDB.md` lines 42-53 at `e9c6ed5e`; cudarc item
  table: `FATHOMDB.md` lines 33-43 at `C`.
- K4/K6 static CUDART: Candle `FATHOMDB.md` K4 and K6 rows; CK-AC-04 at `K`
  and through CK-AC-07 at `K9`.
- `candle-kernels` reaches the workspace only by patch: requirements
  REQ31-16 rationale; R-FK-PUB finding 2.

**Consequences (W6).** Root `Cargo.toml` `[patch.crates-io]` keeps exactly
`libsqlite3-sys` (path) and `candle-kernels` (git, `rev` = `K9`);
`scripts/pinned-override-rot.json` and its checker change as § 3.4 says.

**Tests.** AC31-06 (fork-backed CF-AC-06, CK-AC-03, CK-AC-06); AC31-16
(`Cargo.lock` source of `candle-kernels`; the artifact inspectors); the
pinned-override test arms of § 3.4.

## 3. D-11 Packaging and consumption

### 3.1 Naming and versioning

**Decision.** FathomDB consumes the Candle fork crates under their
published names, renamed back to the upstream dependency keys in the
embedder manifest: `candle-core = { package = "candle-core-fathomdb", … }`
and the same for `candle-nn` and `candle-transformers`. It reaches
`cudarc-fathomdb` only through Candle (§ 3.2): source code keeps writing
`candle_core::cuda::cudarc::…`, because both forks keep their library
names (`[lib] name = "cudarc"`; Candle K1) and Candle 0.11.0 re-exports
cudarc (`pub use cudarc;`, Q21). The fork version scheme is the fork's:
`cudarc-fathomdb` uses the upstream base version (0.19.10), and the Candle
crates use the fork's own numbers (0.11.0, Q21).

**Evidence.** `git show b777f8d:Cargo.toml` lines 2-3, 28-29; Candle
`FATHOMDB.md` K1 and K9 rows; cudarc `FATHOMDB.md` "How to depend on it".

**Consequences.** None beyond § 3.2. How the cudarc fork numbers a
code-changing release on the same upstream base is the fork's decision
(R-DES open-point ruling 1); it is fork todo FT-1 (§ 13.1), non-blocking
for this slice.

### 3.2 Dependency form: exact Candle pins; no direct `cudarc` dependency

**Decision (R-DES finding 6: remove).** The embedder pins every Candle fork
crate exactly and **drops its direct `cudarc` dependency**:

```toml
candle-core         = { package = "candle-core-fathomdb", version = "=0.11.0", optional = true }
candle-nn           = { package = "candle-nn-fathomdb", version = "=0.11.0", optional = true }
candle-transformers = { package = "candle-transformers-fathomdb", version = "=0.11.0", optional = true, default-features = false }

[features]
tegra-pool = []
```

The `[target.'cfg(all(target_os = "linux", target_arch = "aarch64"))'.dependencies]`
table (lines 59-72, comment included) is deleted. `tegra-pool` stays a
feature with no dependency of its own: the driver part compiles only with
`tegra-pool`, aarch64 Linux and `embed-cuda` or `rerank-cuda`
(`cuda_pool_policy.rs:23-31`), and those features bring Candle, hence
`cudarc-fathomdb`.

**Why removal, not an exact-pin dependency with a comment.**

- **It is unused.** No source file imports it: every `cudarc` path in the
  product is `candle_core::cuda::cudarc::…` (§ 0). On x86_64 it does not
  exist at all, so the x86 product already compiles without it.
- **It adds nothing to the graph.** Its features (`std`, `driver`,
  `dynamic-loading`, `cuda-version-from-build-system`) are a subset of
  what Candle 0.11.0 enables (§ 0), so it changes neither the resolved
  features nor the node count.
- **Its only effect was the marker** (§ 3.3, retired) **and an exact
  version bound** on aarch64 `tegra-pool` builds. FathomDB's own builds
  get that bound from the lock: the pinned-override checker requires
  exactly one `cudarc-fathomdb` 0.19.10 with the crates.io source and the
  published checksum (§ 3.4), and the lock-set guard records its closure
  (§ 7.2). A bound that existed only for downstream consumers would cover
  one target and one feature only, while the non-pool and x86 builds of
  the same consumer resolve `cudarc-fathomdb` from Candle's caret
  (`^0.19.10`) either way.
- **Keeping it costs** a second declaration of the fork crate (two places
  to change at each fork bump), a target table the checkers must read, and
  feature-unification reasoning in every review.

The consequence, stated: a downstream consumer of the published FathomDB
crates resolves `cudarc-fathomdb` by Candle's caret, so it may get a later
`cudarc-fathomdb` 0.19.x that the fork publishes. The fork publishes only
qualified versions (fork plan § 8), and that consumer's lock is outside
FathomDB's guards in either design. Recorded, not changed.

The Candle pins are exact (`=0.11.0`), not caret, so a later Candle fork
version enters a FathomDB build only through a reviewed manifest change
(R-PIN, the re-qualification cells of § 7.1, the lock-set guard). The
embedder already pins Candle (`=0.10.3` today, lines 35-37) and `ort`
(`=2.0.0-rc.10`, line 57) exactly; this is the house rule.

**Evidence.** Embedder `Cargo.toml` lines 35-37, 57, 59-72, 148;
`cuda_pool_policy.rs:23-31`; the `grep` and feature rows of § 0; Candle
`git show 182ceab0:Cargo.toml` line 43 (`version = "0.19.10"`, caret).

**Consequences (W6).** `src/rust/crates/fathomdb-embedder/Cargo.toml`:
lines 35-37 (`=0.11.0`), the comment at lines 27-34 (Candle 0.11.0 from the
fork, reached from crates.io), lines 59-72 deleted, line 148
`tegra-pool = []` with its comment (lines 143-147) rewritten (no vendored
cudarc, no marker claim, AC31-06). Root `Cargo.toml` `[patch.crates-io]`
drops the `cudarc` path entry and the three Candle git entries (lines
93-94, 96-97) and moves `candle-kernels` (line 95) to `rev =
"182ceab0547348993b6dbec4b1aed610dcc8b23f"`. `Cargo.lock` is regenerated
with the smallest update (`cargo update -p` per changed package, or the
manifest edit followed by any `cargo` command), never a full
`cargo update`, so no unrelated version moves into the D-16 set.
`dev/interfaces/rust.md` (the `tegra-pool` feature text) says the feature
adds no dependency.

**Tests.** R-PIN checklist (plan § 9 W6 step 2); the pinned-override arms
(§ 3.4), including `a_direct_cudarc_fathomdb_dependency_with_a_caret_is_rejected`;
the single-cudarc guard (§ 3.5); AC31-16's lock condition.

### 3.3 The marker feature

**Decision.** FathomDB stops enabling `fathomdb-private-pool`; with the
direct dependency gone (§ 3.2) it has no way to name it. No FathomDB file
describes the marker as a guard.

- `cudarc-fathomdb` can be reached only by that package name; upstream
  `cudarc` cannot be substituted by a registry resolution, so the marker
  no longer guards anything a consumer can do by accident (requirements
  AC31-06, R-REQ open point 6).
- The one remaining substitution is a deliberate `[patch.crates-io]
  cudarc-fathomdb = …` in someone's own workspace. That substitute is
  either the fork (which has the pool) or a deliberate act.
- Cudarc fork item 6's removal path is "with item 5" (`FATHOMDB.md`
  line 40). Dropping it earlier, once no consumer enables it, needs the
  fork to change that row of its item table first. That is fork todo FT-2
  (§ 13.1), the fork's decision; the published 0.19.10 keeps the no-op
  feature, which is harmless.
- Candle never enabled it (CK-REQ-01: "the package name already fails
  closed against an unpatched cudarc").

**Evidence.** Embedder `Cargo.toml:148`; `git grep fathomdb-private-pool`
outside `third_party/` and `dev/plans/` finds the embedder manifest (lines
61, 148), `dev/adr/ADR-0.8.28-tegra-private-cuda-pool.md:109`,
`dev/design/release.md:138`, `dev/interfaces/rust.md:1243`,
`scripts/check-pinned-override-rot.py:102`,
`scripts/check-tegra-pool-registry-build.sh:11,14,56` and
`scripts/pinned-override-rot.json:57,65` (2026-10-13).

**Consequences (W6).** The embedder manifest (§ 3.2); every file listed
above is changed or deleted with its vendored-tree text (plan § 9 W6
"References to update"); the ADR amendment supersedes the marker sentence
(appendix A).

**Tests.** A T-PRODUCT assertion that W3 adds beside AC31-10's `git grep`:
after W6,

```sh
git grep -n -e "$marker" -- . ':!dev/plans' ':!dev/adr' ':!CHANGELOG.md' \
  ':!dev/todos-and-considerations-ledger.jsonl' ':!dev/steward/steward-ledger.jsonl'
```

is empty, where the test builds `marker` from two pieces
(`marker="fathomdb-private"; marker="$marker-pool"`) so the file that runs
the check never contains the literal (AC30-10 failed by matching itself).
The ledgers are excluded because fork todo FT-2 names the feature. The
same two rules (the ledger exclusions and a split literal) apply to
AC31-10's `git grep` of vendored-tree names wherever a test file runs it.
T-REGISTRY (b) builds `tegra-pool` from the registry (§ 3.6).

### 3.4 The pinned-override record and its checker

**Decision.** `scripts/pinned-override-rot.json` goes to schema version 5:

1. **`cargo_cudarc_exception` is deleted**, with everything that serves it
   in `scripts/check-pinned-override-rot.py`: the `CUDARC_*` constants
   (lines 84-103), `validate_cudarc_exception` (243-261), the cudarc
   branch of `validate_cargo_pins` (513-514, 530-533, 543-544, 580-606) and
   `cudarc_patch_note_failures` (610-622). The vendored tree, its digest
   and its `FATHOMDB-PATCH.md` status table go away (plan § 3).
2. **`cargo_candle_exception` shrinks to the `candle-kernels` patch:**
   `packages: ["candle-kernels"]`, `rev:
   "182ceab0547348993b6dbec4b1aed610dcc8b23f"`, `versions:
   {"candle-kernels": "0.10.2"}`; the rationale says the patch exists only
   to carry K4/K6 static CUDART into FathomDB-built artifacts (REQ-068,
   REQ31-16, Q2). `advisory_posture` is unchanged in form; the rev change
   needs the owner's human advisory review, as the posture text requires.
   Checker constants `CANDLE_PACKAGES`, `CANDLE_REV` and `CANDLE_VERSIONS`
   (lines 62-77) change to match.
3. **A new object `cargo_fork_registry_pins` replaces the item/status
   set.** It lists the exact registry versions of the fork crates and the
   fork items FathomDB relies on:

   ```json
   "cargo_fork_registry_pins": {
     "manifest": "src/rust/crates/fathomdb-embedder/Cargo.toml",
     "pins": [
       {
         "dependency": null,
         "package": "cudarc-fathomdb",
         "version": "0.19.10",
         "checksum": "8780d2fe3fbafc4660e6093abd7681cd9cec7b4d5549a20bc148308e6449caf2",
         "vcs_sha1": "b777f8d94eedca8334e9331cf520d554b22544b7",
         "repository": "https://github.com/coreyt/cudarc",
         "items_record": "FATHOMDB.md (shipped in the crate)",
         "items_relied_on": ["1", "2", "3", "5", "9"],
         "receipt": "fathomdb/qualification/2026-10-13-FC10-publish-b777f8d.md"
       },
       {
         "dependency": "candle-core",
         "package": "candle-core-fathomdb",
         "version": "0.11.0",
         "checksum": "b5feacc063f44c77a54cb8b43d03c5e32fa5421f4163f0524194783e21563341",
         "vcs_sha1": "182ceab0547348993b6dbec4b1aed610dcc8b23f",
         "repository": "https://github.com/coreyt/candle-fathomdb",
         "items_record": "FATHOMDB.md (repository root)",
         "items_relied_on": ["K1", "K2", "K3", "K5", "K7"],
         "receipt": "fathomdb/qualification/2026-10-13-FK5-publish-182ceab0.md"
       }
     ]
   }
   ```

   with `candle-nn` (checksum `69c7432a…6200`) and `candle-transformers`
   (checksum `513008c7…e222`) entries of the same shape. `"dependency":
   null` marks a pin that FathomDB reaches only through another pin (the
   embedder has no direct `cudarc` dependency, § 3.2): it is checked in
   the lock only. The checker owns the same values as constants (the
   file's existing pattern: metadata must equal checker-owned constants,
   lines 188-261) and verifies, offline:
   - the manifest's dependency tables, read as a whole: `[dependencies]`
     **and every `[target.'cfg(…)'.dependencies]` table** (the 0.8.27
     cudarc dependency lived in one). A pin with a `dependency` key is
     declared exactly once across them, in `[dependencies]`, with exactly
     `package` and `version = "=<version>"` (an exact requirement; a caret
     fails). A pin with `"dependency": null` must not be declared in any
     table, in any form: an exact, caret or path requirement, under its
     own name or under `package =`, all fail (revision 3, R-TEST finding
     9);
   - `Cargo.lock` has exactly one package of that name, with that version,
     source `registry+https://github.com/rust-lang/crates.io-index` and
     that checksum. **This is the only place that checks the number,
     version and source of `cudarc-fathomdb` lock entries** (R-DES
     finding 10: the single-cudarc guard's former condition B and the
     lock-set guard's former step 1 are dropped in favour of it);
   - no `[patch]`, `[replace]` or git dependency names the package (the
     existing unsupported-override rule, lines 506-528, already fails it).

   The checker cannot read the fork's item tables offline; the
   `items_relied_on` lists make a change of relied-on items a reviewed
   checker change, and R-PIN compares them with the fork's table.
   `vcs_sha1` and `receipt` are provenance that T-FORK-EVIDENCE checks
   against the registry and the fork (AC31-08, AC31-14).
4. **`scope.cargo`** text is rewritten for the three mechanisms that
   remain (the `candle-kernels` git patch, the SQLite path patch, the
   exact fork registry pins). The success line (line 713) drops "cudarc
   patches".

**Evidence.** `scripts/pinned-override-rot.json` lines 12, 16-39, 50-67;
`scripts/check-pinned-override-rot.py` lines named above; the checksums
are the registry `.crate` SHA-256s (FC10 and FK5 receipts), which Cargo
records as the lock `checksum` (§ 0 diff).

**Consequences (W6).** The two files above;
`scripts/tests/test_check_pinned_override_rot.sh` (an AC31-10 named
exception): the eight cudarc arms (lines 566-666: missing, tree, path,
lock, license, items, note, blank) are deleted with `seed_cudarc_fixture`
(lines 149-182); the Candle-cohort arms keep only
`candle-kernels`; new arms, RED first. Items relied on, and why: cudarc 1-3
(the 0.8.27 fallback path that `tegra-pool` falls back to on aarch64), 5
(the pool), 9 (D-18); Candle K1 (names), K2 (aarch64 CPU F16), K3
(`dynamic-loading`), K5 (`new_cuda_from_context`), K7 (the cudarc fork).

**Tests (new arms in `test_check_pinned_override_rot.sh`).**

- `fork_registry_pin_with_a_caret_requirement_is_rejected`;
- `a_direct_cudarc_fathomdb_dependency_is_rejected` (revision 3; replaces
  revision 2's caret-only arm): three fixtures, each rejected: a `cudarc`
  entry with `package = "cudarc-fathomdb"` and `version = "=0.19.10"` in
  `[dependencies]`; the same with `version = "0.19.10"` (a caret) in an
  aarch64 `[target.'cfg(…)'.dependencies]` table (the checker reads target
  tables); and `cudarc-fathomdb = { version = "=0.19.10" }` under its own
  name;
- `fork_registry_pin_with_a_non_registry_source_in_the_lock_is_rejected`
  (the former single-cudarc condition B: a second `cudarc-fathomdb` with a
  `git+https://github.com/coreyt/cudarc.git?rev=…` source);
- `fork_registry_pin_with_a_git_source_in_the_lock_is_rejected`;
- `fork_registry_pin_with_a_different_checksum_is_rejected`;
- `fork_registry_pin_with_a_second_version_in_the_lock_is_rejected`;
- `missing_fork_registry_pin_is_rejected`;
- `candle_kernels_patch_at_another_rev_is_rejected`;
- `a_cudarc_patch_entry_is_unsupported` (a reintroduced `cudarc` path
  patch fails as an unsupported override);
- GREEN: the real tree.

The marker feature's emptiness is checked beside the retired-reference
cases (`test_slice31_retired_cudarc_references.sh`, case
`the_tegra_pool_feature_is_empty`): `tomllib` reads the embedder manifest
and requires `features["tegra-pool"] == []` (§ 3.3).

### 3.5 The single-cudarc guard

**Decision.** `scripts/check-single-cudarc.sh` (bash entry point; the logic
in an embedded `python3 -I` block using `tomllib`, with the `tomli`
fallback AGENTS.md § 3 names). Fast tier: `run_tier_suite fast
test-check-single-cudarc bash scripts/tests/test_check_single_cudarc.sh`
in `scripts/agent-test.sh`, which runs the guard on the real tree and on
fixtures. Inputs: `--lock <file>` (default `<root>/Cargo.lock`) for the
lock condition and `--root <dir>` (default the repository root) for the
tree conditions, so the tests can point either at a fixture. It fails on
any of:

- **A. Lock, upstream cudarc 0.19** (parsed with `tomllib`, no `cargo`).
  The lock has a package named `cudarc` whose version starts `0.19.`.
  (`cudarc` 0.17.8 from `ug-cuda` passes: the `ug` chain is excluded by
  version, as Candle CK-AC-01 does.)
- **(Former B, moved.)** The number, version and source of
  `cudarc-fathomdb` lock entries are checked once, by the pinned-override
  checker (§ 3.4); this guard does not repeat it (R-DES finding 10).
- **C. Tree.** For each target triple in {`x86_64-unknown-linux-gnu`,
  `aarch64-unknown-linux-gnu`} and each feature set in {`embed-cuda`,
  `rerank-cuda`, `tegra-pool`, `embed-cuda,tegra-pool`,
  `rerank-cuda,tegra-pool`}, run in `<root>`:
  `cargo tree --offline --locked -e normal --target <triple> -p
  fathomdb-embedder --features <set> --prefix none`. Each output line has
  a trailing `(*)` with its leading space (Cargo's mark for a node already printed, R-DES
  finding 2) removed first. The guard then fails if a line starts `cudarc
  v0.19`, or a `cudarc-fathomdb` line is not exactly `cudarc-fathomdb
  v<version>` (a git or path source prints a parenthesised source and
  fails), or, for a set with `embed-cuda` or `rerank-cuda`, no
  `cudarc-fathomdb` line exists (non-vacuity). `flash-attn` and `ug` are
  never enabled by the guard, which is how their chains are excluded
  (CK-AC-01); FathomDB enables neither, and `flash-attn` is unsupported
  (R-FK-PUB finding 5). The expected version comes from
  `<root>/scripts/pinned-override-rot.json` `cargo_fork_registry_pins`
  (for fixtures, the fixture's own copy at that path), not a second
  literal.
- **D. Concrete version feature (D-8W).** For each target triple, run in
  `<root>`: `cargo tree --offline --locked -e features -i cudarc-fathomdb
  --target <triple> --features
  fathomdb-embedder/embed-cuda,fathomdb-embedder/rerank-cuda,fathomdb-embedder/tegra-pool
  --prefix none` has a line `cudarc-fathomdb feature "cuda-NNNNN"` (five
  digits).

`--locked` keeps the guard from rewriting the lock; a lock that does not
fit the manifests makes `cargo` fail ("cannot update the lock file"), and
a `cargo` failure is a guard failure, never a skip. On the real tree,
`--offline` needs the registry cache; the tier runner fetches it once with
`cargo fetch --locked` (crates.io, a permitted package registry) before
the guard.

**Fixtures** (`scripts/tests/fixtures/single-cudarc/`), split by condition
(R-DES finding 3):

- **Lock fixtures** (`lock/<name>.lock`) are lock files only. Condition A
  parses them without `cargo`, so they need no workspace and need not be
  resolvable.
- **Tree fixtures** (`tree/<name>/`) are complete, consistent workspaces
  that `cargo tree --offline --locked` accepts: a root `Cargo.toml`, a
  `fathomdb-embedder` crate with the features `embed-cuda`, `rerank-cuda`,
  `tegra-pool`, a `scripts/pinned-override-rot.json` copy, a
  `.cargo/config.toml` that replaces `crates-io` with a directory source
  `vendor/`, and stand-in crates there (`cudarc-fathomdb` 0.19.10 with
  `[lib] name = "cudarc"`, and `cudarc` 0.19.4 where a fixture needs it),
  each with a `.cargo-checksum.json`. With source replacement, the lock
  records these as `registry+https://github.com/rust-lang/crates.io-index`
  and `cargo tree` prints them as `cudarc-fathomdb v0.19.10`, exactly as
  for the real crates (measured 2026-10-13 in a scratch fixture: `cargo
  generate-lockfile --offline` then `cargo tree --offline --locked`
  printed `cudarc-fathomdb v0.19.10` with no source suffix). Each fixture's
  `Cargo.lock` is generated by `cargo generate-lockfile --offline` in the
  fixture and committed; the test suite first runs `cargo metadata
  --offline --locked` on every tree fixture and fails loudly if one is
  inconsistent, so a fixture cannot fail for the wrong reason.

| Fixture | Contents | Expected |
| --- | --- | --- |
| `lock/ok.lock` | a lock with one `cudarc-fathomdb` 0.19.10 registry entry and no `cudarc` | A passes |
| `lock/upstream-cudarc.lock` | `ok.lock` plus `cudarc` 0.19.4 (registry) | A fails |
| `lock/ug-cudarc.lock` | `ok.lock` plus `cudarc` 0.17.8 | A passes (the `ug` exclusion) |
| `tree/ok/` | the embedder depends on the `cudarc-fathomdb` stand-in under `embed-cuda` and `rerank-cuda` | C and D pass (the GREEN control) |
| `tree/upstream-cudarc/` | `ok/` plus the `cudarc` 0.19.4 stand-in, a dependency under `embed-cuda` | C fails |
| `tree/path-cudarc-fathomdb/` | the `cudarc-fathomdb` stand-in as a path dependency (prints a parenthesised path) | C fails |
| `tree/no-cudarc-fathomdb/` | `embed-cuda` reaches no `cudarc-fathomdb` | C fails (non-vacuity) |
| `tree/flash-attn-only/` | the `cudarc` 0.19.4 stand-in reachable only through a fixture feature `flash-attn` | C passes (the `flash-attn` exclusion) |
| `tree/concrete-feature/` | `ok/`, with `embed-cuda` enabling a `cuda-12060` feature on the stand-in | D fails |

`test-plan.md` (W3) fixes the fixture files byte for byte, including the
generation commands.

**Evidence.** § 0 (one node per target and feature set; no `cudarc` 0.19
in the lock; resolved features); the scratch fixture above. Candle CK-AC-01
(`fathomdb/requirements.md` lines 86-103 at `e9c6ed5e`) uses the same
tree-plus-exclusion form. The `ug` chain: `Cargo.lock` `ug-cuda` 0.5.0 →
`cudarc` 0.17.8 (lock only, plan § 2).

**Consequences (W6).** New `scripts/check-single-cudarc.sh`,
`scripts/tests/test_check_single_cudarc.sh`, the fixtures, one
`run_tier_suite fast` line in `scripts/agent-test.sh` where the
`test-vendored-cudarc` line (lines 323-326) is removed. Requirements
AC31-07 names the pinned-override checker for the former condition B.

**Tests.** AC31-07: one RED test per failure condition (A on its lock
fixture; C three ways; D) on its fixture, the two exclusion fixtures
passing, and the guard GREEN on the real tree (T-PRODUCT, "single-cudarc
guard red then green").

### 3.6 Registry consumption (T-REGISTRY)

**Decision.** `scripts/check-tegra-pool-registry-build.sh` is rewritten for
the opposite claim: a downstream consumer of the published crates, with
**no `[patch]` table**, builds and runs `tegra-pool`.

1. The scratch copy (unchanged method, lines 73-80, minus
   `third_party/libsqlite3-sys-0.38.1`, which the embedder does not reach)
   gets one consumer crate, its own workspace root, depending on the copied
   `fathomdb-embedder` by path with `features = ["embed-cuda",
   "tegra-pool"]`, and no `[patch]` table at all. `make_consumer`'s patch
   copying (lines 90-130) and the `cudarc` 0.19.7 pin (lines 137-149) are
   deleted.
2. **(a) Resolution.** `cargo generate-lockfile` in the consumer; its lock
   has `cudarc-fathomdb` 0.19.10, the three Candle crates 0.11.0 and
   `candle-kernels` 0.10.2, each with the crates.io registry source, and no
   `cudarc` 0.19.* package. Runs on any host (no CUDA needed for
   resolution).
3. **(b) Build and link on the Orin.** `CUDA_COMPUTE_CAP=87`,
   `CUDA_PATH=/usr/local/cuda-12.6`, `PATH` with its `bin`, and
   `LIBRARY_PATH=/usr/local/cuda/lib64` (R-FK-PUB finding 2: the registry
   `candle-kernels` links `-lcudart` dynamically); `cargo build` and
   `cargo test --no-run` succeed. The receipt states each value.
4. **(c) Runtime.** The consumer carries a copy of
   `tests/tegra_fragmented_va_cuda.rs` and `tests/support/live.rs` (which it
   includes by `#[path]`, lines 42-43) as its own integration test (the Slice 30 file is copied, not edited),
   with a consumer feature `embed-cuda` forwarding to
   `fathomdb-embedder/embed-cuda` (the test's `#![cfg]`, line 35, names
   that feature) and `candle-core = { package = "candle-core-fathomdb",
   version = "=0.11.0" }` as a dev-dependency (the test imports
   `candle_core`, line 39). `FATHOMDB_REQUIRE_LIVE=1 cargo test --test
   tegra_fragmented_va_cuda --
   forced_cuda_probe_succeeds_when_the_default_memory_pool_is_unavailable
   --exact` passes with one test run. `test-plan.md` fixes the invocation
   (AC31-08 (c)).
5. **(d)** The receipt and `docs/embedder.md` state that crates.io
   consumers get dynamic `libcudart` through upstream `candle-kernels`
   0.10.2, and that REQ-068's CPU-loadable guarantee applies only to
   FathomDB-built artifacts (REQ31-16).
6. The header comment (lines 2-49) is rewritten; it stays a manual release
   check (`dev/design/release.md` lines 131-142 change with it).

**Evidence.** Current script lines 2-49, 56, 85-130, 137-183; requirements
AC31-08; R-FK-PUB findings 2 and 5 (plan § 8.1 revision 17); the test's
`cfg` and imports (`tests/tegra_fragmented_va_cuda.rs:35-44`).

**Consequences (W6).** The script (AC31-10 named exception, AC30-07
inverted); `dev/design/release.md` lines 131-142; `docs/embedder.md`.

**Tests.** T-REGISTRY (a)-(d), RED first: the current script fails on the
W6 tree at its line 102 ("must carry exactly one cudarc entry"), which is
the recorded RED; GREEN is (a)-(c) passing on the Orin.

### 3.7 Kernel data types on the x86 build (BF16/F8)

**Decision.** FathomDB never selects BF16 or F8 on any CUDA device. The x86
release build compiles Candle's kernels once at `CUDA_COMPUTE_CAP=75`, and
the BF16 and F8 kernels exist only for `__CUDA_ARCH__ >= 800`, so in that
build they are absent on every GPU, the RTX 3090 included. The embedder and
reranker load and run F32 only today.

**No separate guard test (R-DES finding 10).** Revision 1 proposed a pure
test asserting the `DType` constants; it would be GREEN at once and could
not be shown RED by anything but editing the test's own expectation. The
behaviour is already guarded at run time: a BF16 or F8 tensor in the x86
build fails on the RTX 3090 (the kernel is absent from the sm_75 PTX the
driver JIT-compiles), so the T-CUDA RTX 3090 embed and rerank rows and
the C3 T4 cell (§ 9.3) fail on such a change. The test is dropped.

**Evidence.** `candle-kernels/src/fill.cu:70-87` in the Candle fork
(plan § 6 D-11); the T4 trial (`ones_gpu` fails pre-Ampere, plan § 15 Q13);
todo `TC-30847709`. The x86 compute capability: `cuda-artifact-contract.sh`
lines 71-78 (`CUDA_COMPUTE_CAP_X86_64='75'`).

**Consequences (W6).** None. Multi-architecture kernels (Candle DK-3, todo
`TC-323f15b0`) stay the way to add BF16/F8 later.

**Tests.** T-CUDA RTX 3090 rows; T-CUDA-COMPAT C3 (§ 9.3).

## 4. D-12 (product) Tegra

**Decision.**

1. **Selection, report and early `cuInit` are unchanged.** The gates,
   sizing, settings, reasons, `doctor cuda-allocator`
   (`fathomdb.doctor.cuda-allocator.v1`) and `run_module_load_early_init`
   stay Slice 30's (ADR rule and gates). The fork changes none of the calls
   they make (§ 8).
2. **The 3 GiB default stays.** Against the fork's Tegra facts:
   - **Usable ≈ 1/3 of the cap.** By the fork's fitted formula
     `ceil(ceil(cap / 32 MiB) / 3) × 32 MiB`, a 3072 MiB cap serves about
     1024 MiB before `CUDA_ERROR_OUT_OF_MEMORY` (3072 MiB itself was not
     measured; 1024 → 352 and 4096 → 1376 were). Slice 30 sized the pool
     with this fact already ("a pool holds ceil32(`maxSize`/3)", Slice 30
     design § 2.3), and measured `reserved_high` 288 MiB of 3 GiB for
     64-passage embedding batches (CB1), with exhaustion typed and
     recoverable (G4). The default therefore leaves about 3.5× headroom
     over the measured peak. The docs say so, and say that
     `FATHOMDB_POOL_MAXSIZE` buys about a third of its value on JetPack 6.
     T-ORIN records the usable capacity at 3 GiB once (§ 4, tests).
   - **About four uncapped pools per process.** The private pool is capped,
     so it does not take one of the four 20.47 GiB address ranges; a capped
     pool takes less (200 pools at 64 MiB coexisted with three uncapped).
     A private-pool process never calls `CudaContext::new`, so it never
     obtains the default pool. A process on the 0.8.27 path takes one
     (item 1). Co-resident libraries (default pools, graph memory) use the
     rest; when they exhaust them, item 1 picks `Synchronous`, which is the
     intended fallback. FathomDB uses no CUDA graphs (Candle 0.10.2 base,
     DK-1). `FATHOMDB_POOL_MAXSIZE` near device total is UNMEASURED for
     address-space cost and is not recommended in the docs.
   - **`cuMemGetInfo` is not a release signal.** The product never uses it
     to decide or report release; it reads the pool's counters. The one
     product use, the Slice 80.5 Tegra GPU allocation witness
     (`gpu_witness.rs:744`), is a release-evidence instrument that already
     accounts for the warm page pool with a 1 GiB control allocation held
     resident (`gpu_witness.rs:36-42, 96`). No change.
   - **`cuCtxCreate_v4(NULL)` segfaults on the Tegra 12.6 driver.**
     FathomDB never calls `CudaContext::new_non_primary` or
     `result::ctx::create_v4` (`grep` of `src/rust`, 2026-10-13). No
     change; the docs do not mention it (an upstream defect FathomDB does
     not reach).
3. **Tegra on CUDA 13 is UNMEASURED** (Q5, todo `TC-b267d940`).

**Evidence.** Cudarc fork `fathomdb/design.md` § 2 (free memory, uncapped
pools), § 4 (cap table), § 10 (`cuCtxCreate_v4`); probe receipt
`fathomdb/qualification/2026-10-12-uncapped-pools-probe.md` (record commit
`393f165`); G6 receipt `2026-10-12-G6-b777f8d.md`; cudarc `FATHOMDB.md`
"Tegra notes"; Slice 30 qualification
(`dev/plans/runs/0.8.28-slice-30/qualification.md` rows G1, G4; status
row S30-T10b for CB1); `cuda_pool_policy.rs:42-46, 411` (sizing
constants).

**Consequences (W6).** `docs/embedder.md` § "Jetson: private CUDA memory
pool": the usable-capacity sentence and the `FATHOMDB_POOL_MAXSIZE` note;
no code change.

**Tests.** T-ORIN (Slice 30 gate rows, `doctor cuda-allocator` reporting
`private`, 3 GiB, AC31-04); T-POOL-PRODUCT; a new non-gating measurement
`gpu_orin_64_records_the_usable_capacity_of_the_three_gib_pool` (in the new
unit-test file of § 6) that allocates 32 MiB tensors on the private pool
until `cuda_pool_exhausted` and prints the usable bytes for
`qualification.md`.

## 5. D-14 Allocations outside the pool

**Decision.** D-14 records, not bounds, the device memory that is not
allocated from the private pool. On the Orin it measures per-process
memory as `VmRSS` and its split, at fixed stages, beside the private
pool's counters; the default pool's counters serve only as a leak check.

- **What is outside the pool** (Candle DK-4, cudarc D-2 and D-F3): the
  cuBLAS handle and its workspace (Candle creates one `CudaBlas` per
  device; nothing calls `cublasSetWorkspace`); the cuRAND generator state
  (`CudaRng::new` at device creation); the driver's module load and JIT of
  the embedded PTX; static CUDART's registration; the context itself. The
  fork does not carry upstream PR #595 (a caller-owned cuBLAS workspace),
  so a pool-backed workspace is not possible in this slice and is not
  measured.
- **Why `VmRSS`.** On the Orin, `cuMemGetInfo` free memory is
  `MemAvailable` and does not move with pool reservations once the nvmap
  page pools are warm, while `VmRSS` rose by the full size of each
  allocation and the pool attributes were exact (cudarc D-2 measurements).
- **Method** (one process, `FATHOMDB_EMBED_DEVICE=cuda:0`, early `cuInit`
  run, cached model weights). At each stage, synchronize the decision's
  first context, then record from `/proc/self/status` `VmRSS`, `RssAnon`,
  `RssFile` and `RssShmem`, the private pool's `reserved_current`,
  `used_current`, `reserved_high`, and the device default pool's
  `reserved_current` and `used_high`:
  - S0 after `run_module_load_early_init`;
  - S1 after the first `new_cuda_device(0)` (decision, context, cuBLAS
    handle, cuRAND state);
  - S2 after `CandleBgeEmbedder::new()` (weights in the pool);
  - S3 after 5 warm-up embeds (PTX modules loaded and JIT-compiled, cuBLAS
    workspace in use);
  - S4 after 50 timed embeds;
  - S5 after the embedder is dropped.

  Outside-pool memory at stage n is reported as `ΔVmRSS(S0→Sn) −
  Δreserved_current(S0→Sn)`, with the `RssFile` delta shown separately
  (memory-mapped weight files count there). The same run with the CPU
  embedder gives the host-only control for S2-S5. The receipt reports the
  numbers; it does not gate on them.
- **Leak check:** the default pool's `reserved_current` and `used_high` stay
  0 at every stage (a private-pool process never touches the default pool,
  CK-AC-02's assertion in the product). This one is asserted.

**Evidence.** Candle `fathomdb/requirements.md` § 4 DK-4 (lines 292-318 at
`e9c6ed5e`, the RTX 3090 measurement: default-pool counters 0 through
device construction, a 512×512 `matmul` and `Tensor::rand`; the private
pool held only tensor bytes); cudarc `fathomdb/design.md` § 2 ("Tegra:
free memory is not a release signal") and § 12.3 (D-F3); Candle
`device.rs:40, 274, 275` at `K9` (one `CudaBlas`, one `CudaRng` per
device); todo `TC-283d4f3a`.

**Consequences (W6).** A new `#[ignore]` Orin test (below); the numbers go
into `qualification.md` (AC31-06). No product change.

**Tests.** `gpu_orin_64_memory_outside_the_private_pool_is_recorded` in the
new unit-test file of § 6 (`#[ignore]`, run under `flock
/tmp/fathomdb-gpu.lock` with `FATHOMDB_REQUIRE_LIVE=1`), printing one line
per stage, `fathomdb-d14 {json}`, and asserting the leak check. It reads
the default pool through `result::device::get_default_mem_pool` only
after S0, so
the read itself is part of every later stage alike.

## 6. D-15 Teardown and process lifecycle (product)

**Decision.**

1. **The Slice 30 teardown assertion stays in this slice.**
   `gpu_orin_64_a_primary_context_reset_is_context_lost_and_teardown_segfaults`
   keeps asserting `cuda_context_lost` on both embeds after a co-resident
   reset and SIGSEGV at teardown (`cuda_pool_policy_tests.rs:1518`). The
   fix stays 0.8.29's (todo `TC-281155a1`). Reasons:
   - the fault is in cuBLAS's `cublasDestroy_v2`, called by cudarc's
     `CudaBlas::drop` (`src/cublas/safe/mod.rs:90-97`, which `unwrap`s the
     destroy result), from the Candle device's drop. That code is
     identical in the vendored 0.19.7 and at `C` (a `diff` of the `impl
     Drop for CudaBlas` blocks is empty), so the new base does not change
     the fault;
   - a fix needs a change in a published crate (skipping the destroy, or
     not unwrapping it, when the context is gone: cudarc or Candle), or an
     intentional leak of the device in the product on detected loss. A
     crate change means a new `cudarc-fathomdb` or Candle version, fork
     re-qualification, R-FK-PUB or R-FC-PUB and an owner publication; the
     leak needs data on what is leaked. Slice 30's C7 instrumentation
     (todo `TC-99845d75`) exists to gather that data for 0.8.29;
   - the owner ruled a co-resident reset unsupported in 0.8.28 (ADR,
     component 5), and AC31-10 keeps the test unchanged unless D-15 fixes
     it.

   If the test fails on the new base (no SIGSEGV), that is a finding for
   R-VERIFY, not a test edit (test files are read-only, AGENTS.md § 5).
2. **Pools in statics at exit.** The decision's `OnceLock` holds the pool
   and the first context for the process lifetime and is never dropped;
   the OS reclaims them (Slice 30 § 4.4; cudarc D-2: a pool held by a
   static is never dropped). Module-level singletons hold pool memory until
   exit (ruling 32). Process exit with the pool alive must end with status
   0 and no signal, tested in the product.
3. **Release at close.** D-17's close release (§ 8) runs at drop of each
   FathomDB-built private-pool model, and is skipped when the context is
   lost, so it cannot add a teardown crash.
4. **Fork after init (hazard H-1)** keeps Slice 30's contract: a child
   forked after `import fathomdb` on the Tegra wheel cannot use CUDA and
   gets a typed error; the opt-out and `spawn`/`forkserver` are the
   remedies. Slice 31 tests these cases, each with a 120 s child timeout
   (a hang fails):
   - F1: hook on (default), the parent never touched the GPU, the child
     forces `cuda:0`: `EmbedDevicePolicyError`, kind `cuda_probe_failed`
     (Slice 30 G2: 5/5);
   - F2: `FATHOMDB_CUDA_EARLY_INIT=off`, same child: opens on `cuda`,
     path `default_pool` or `synchronous`, reason `cuinit_opted_out`, and
     embeds (Slice 30 G2: 5/5);
   - F3: hook on, the child under `auto`: opens on CPU, reason
     `cuda_probe_failed` (not run in Slice 30);
   - F4: hook on, the parent embedded on the private pool before forking;
     the child forces `cuda:0`: a typed error, no signal; the parent then
     embeds again on the private pool, and its pool counters stay below
     the cap (new);
   - F5: the `spawn` start method after the parent used the private pool:
     the child opens on `cuda`, path `private`, and embeds (new).

**Evidence.** `cuda_pool_policy_tests.rs:1494-1608` (the test and its
child); Slice 30 design § 3.5 and § 8.4; ADR lines 130-138; vendored
`third_party/cudarc-0.19.7/src/cublas/safe/mod.rs:90-97` and `git show
b777f8d:src/cublas/safe/mod.rs` lines 90-97; Slice 30 qualification § 4
(G2 fork rows); Slice 30 design § 2.5; ledger todos `TC-281155a1`,
`TC-99845d75`.

**Consequences (W6).** New test files only:

- `src/rust/crates/fathomdb-embedder/src/cuda_pool_slice31_tests.rs`,
  included from `cuda_pool_policy.rs` by `#[cfg(test)] #[path =
  "cuda_pool_slice31_tests.rs"] mod slice31_tests;` (the Slice 30 test file
  stays byte-identical, AC31-10), holding the Orin `#[ignore]` tests of
  §§ 4, 5, 6 and 8;
- `src/python/tests/test_tegra_pool_fork_after_init.py` (F1-F5; SKIP
  unless the Tegra wheel is installed on an Orin; it adopts the Rust GPU
  tests' `FATHOMDB_REQUIRE_LIVE=1` convention, which turns a skip into a
  failure and is new to the Python suite).

The docs keep Slice 30's H-1 text; F4 and F5 add one sentence each if the
results need it.

**Tests (T-TEARDOWN-PRODUCT, AC31-04).**

- `gpu_orin_64_a_primary_context_reset_is_context_lost_and_teardown_segfaults`
  (unchanged; re-run on the new base);
- `gpu_orin_64_process_exit_with_the_private_pool_alive_is_clean`: a child
  builds the embedder, embeds and calls `std::process::exit(0)` without
  dropping it; status 0, no signal;
- `gpu_orin_64_dropping_the_embedder_then_exiting_is_clean`: a child drops
  the embedder and returns; status 0, no signal, and the pool's
  `reserved_current` is 0 before exit;
- engine close: the Node/V8 end-to-end run of T-ORIN closes its engine and
  exits with status 0 (AC31-04 first bullet; W3 names the receipt check);
- engine close in Rust (R-DES open-point ruling 7, a recommendation for
  W3): a new `#[ignore]` Orin test in `fathomdb-engine` (for example
  `gpu_orin_64_engine_close_with_the_private_pool_then_exit_is_clean`, in a
  new `tests/slice31_engine_close_cuda.rs`) opens an engine with the CUDA
  embedder on the private pool, writes and searches once, closes the
  engine, then exits a child process; status 0, no signal, and (with N1)
  the pool's `reserved_current` is 0 after close. It gives engine-level
  close evidence that does not depend on the Node runtime; W3 fixes the
  name, file and feature set, and R-TEST may decline it with a reason;
- `test_forked_child_after_import_forced_cuda_raises_cuda_probe_failed`
  (F1), `test_forked_child_with_early_init_off_embeds_on_cuda` (F2),
  `test_forked_child_after_import_auto_falls_back_to_cpu` (F3),
  `test_forked_child_after_parent_gpu_use_gets_a_typed_error_and_the_parent_still_embeds`
  (F4), `test_spawned_child_embeds_on_the_private_pool` (F5).

These are characterization tests of existing behaviour, so they are GREEN
at once unless the new base changed it; R-TEST decides whether F4 needs a
RED mutation.

## 7. D-16 Fork evidence and re-qualification

### 7.1 When pins move

**Decision.** Any change of a fork pin is a manifest change reviewed by
R-PIN (Q20). The FathomDB cells that re-run:

| Change | FathomDB re-runs |
| --- | --- |
| new `cudarc-fathomdb` version (fork code change, after the fork's own re-qualification, fork plan § 8) | R-PIN; pinned-override checker; single-cudarc guard; lock-set guard (§ 7.2) with a new fork record; T-WS-BUILD 12060; T-POOL-PRODUCT and T-TEARDOWN-PRODUCT (Orin); T-CUDA RTX 3090 row; T-CUDA-COMPAT C2 if item 9 changed; T-REGISTRY; T-PERF if items 1-5 changed |
| new Candle crate version | R-PIN; pinned-override checker; single-cudarc guard; T-WS-BUILD; T-POOL-PRODUCT; T-CUDA; T-REGISTRY; T-PERF |
| `candle-kernels` `rev` moves | R-PIN (owner advisory review); pinned-override checker; AC31-16 artifact inspection; T-WS-BUILD 12060 (and 13000 when a VM session is approved) |
| FathomDB `Cargo.lock` changes the `cudarc-fathomdb` dependency set (§ 7.2) | the lock-set guard fails until the fork records a run with the new lock |

R-PIN confirms the chain with the plan § 9 W6 checklist: exact registry
versions; one `cudarc-fathomdb` in `Cargo.lock` with the crates.io
source; the pinned-override record updated in the same commit; the
`candle-kernels` `rev` = the Candle publication commit; and, new here,
each pin's `vcs_sha1` equals a commit that its fork's R-…-VERIFY (or
CK-AC-07) approved.

**Evidence.** Cudarc fork plan § 8 (re-qualification table, last row);
Candle `fathomdb/requirements.md` § 5.4; plan § 3.1 "The pin chain".

### 7.2 The lock-drift guard (Q8 option (b))

**What deleting `test_vendored_cudarc.sh` changes.** Today
`scripts/agent-test.sh:326` runs the vendored crate's `fathomdb_` tests in
the fast tier against FathomDB's own lock. After W6 the fork's hosted test
job runs them (x86_64 and aarch64), once with its baseline lock and once
with FathomDB's `Cargo.lock` fetched by the commit in `FATHOMDB_LOCK_SHA`
(fork `.github/workflows/fathomdb-ci.yaml` lines 32-35, 231-300), and G1/G6
run them on hardware. The run at `C` used FathomDB `4c327cd8a`'s lock, in
which the cudarc entry is the vendored `cudarc` 0.19.7, not
`cudarc-fathomdb` (`git show 4c327cd8:Cargo.lock`). So the fork has not yet
run with a lock that names `cudarc-fathomdb`; the guard below cannot go
GREEN until it has.

**Decision (revised for owner ruling Q23, steward ledger seq 326).**

- **The set: the Linux-target normal closure.** The guarded set is what
  the fork compiles for FathomDB on Linux, not everything the lock names.
  For each target `t` in {`x86_64-unknown-linux-gnu`,
  `aarch64-unknown-linux-gnu`}:
  `cargo metadata --offline --locked --format-version 1 --filter-platform
  <t> --features <F>`, where `<F>` is
  `fathomdb-embedder/embed-cuda,fathomdb-embedder/rerank-cuda,fathomdb-embedder/tegra-pool`
  in FathomDB. From the `resolve` graph, starting at the one
  `cudarc-fathomdb` node, follow only edges whose `dep_kinds` include a
  normal kind (`"kind": null`); build and dev edges are not followed, and
  `--filter-platform` has already removed Windows-, WASI- and other
  non-Linux-only dependencies. The set for `t` is the visited nodes,
  **including the root**. The record holds both targets' sets and the
  features resolved on the root (`resolve.nodes[root].features`), because
  the closure depends on them (`f16` brings `half`, `f8` brings `float8`,
  `dynamic-loading` brings `libloading`).
- **Measured size (2026-10-13, scratch crate with `K9`'s exact feature
  list, latest-compatible versions; scratch deleted).** The whole-lock
  closure was 26 packages **including the root**; the Linux normal closure
  is 20 including the root, identical on both targets: `cudarc-fathomdb`,
  `cfg-if`, `float8`, `getrandom`, `half`, `libc`, `libloading`, `libm`,
  `num-traits`, `ppv-lite86`, `proc-macro2`, `quote`, `rand`,
  `rand_chacha`, `rand_core`, `rand_distr`, `syn`, `unicode-ident`,
  `zerocopy`, `zerocopy-derive`. Q23 removes `autocfg` and `crunchy` (build
  dependencies), `r-efi`, `wasip2`, `wit-bindgen` (UEFI/WASI) and
  `windows-link`. Revision 1's "27" was the whole-lock closure of the
  emulated W6 lock and did not say whether it counted the root; W6 records
  the real count, root included.
- **Remaining cost.** The closure still holds widely shared crates
  (`libc`, `syn`, `proc-macro2`, `quote`, `zerocopy`, `rand*`, `getrandom`).
  Any FathomDB lock change that moves one of them, for any reason (a
  `cargo update -p` elsewhere, a new dependency that raises a minimum),
  fails the guard until the fork re-runs Run 2 with that lock and records
  the set: one fork CI run (two hosted jobs, minutes) plus a record commit
  in the fork and a copy commit in FathomDB. Changes to build-only,
  dev-only and Windows/WASI-only crates no longer trip it.
- **Canonical text and digest.** Per target, one line per package, the
  root first, then the members sorted by `(name, version)`:
  `<name> <version> <source> <checksum>\n`. The text is the targets in the
  order above, each preceded by `# <target>\n`, then `# features
  <comma-joined sorted root features>\n`. `set_sha256` is the SHA-256 of
  that text. Both the fork and FathomDB compute it with this definition.
- **The fork record (written in the fork).** After a hosted-test run with
  a FathomDB lock, the fork commits (class `D`)
  `fathomdb/qualification/<date>-lockset-<FathomDB commit, 12 hex>.json`:

  ```json
  {
    "schema": "fathomdb.cudarc-fathomdb-lock-set.v2",
    "fathomdb_lock_commit": "<40 hex>",
    "fathomdb_lock_sha256": "<sha256 of that Cargo.lock>",
    "ci_run_id": 0,
    "ci_head_sha": "<the fork commit the job ran at>",
    "ci_conclusion": "success",
    "closure": "linux-normal",
    "root_features": ["cublas", "cublaslt", "cuda-version-from-build-system", "curand", "driver", "dynamic-loading", "f16", "f8", "nvrtc", "std"],
    "targets": {
      "x86_64-unknown-linux-gnu": [{"name": "cudarc-fathomdb", "version": "0.19.10", "source": "registry+https://github.com/rust-lang/crates.io-index", "checksum": "8780d2fe…"}, {"name": "cfg-if", "version": "…", "source": "registry+…", "checksum": "…"}],
      "aarch64-unknown-linux-gnu": ["… the same shape …"]
    },
    "set_sha256": "<64 hex>"
  }
  ```

  In the fork, Run 2 computes it from its own root (the `cudarc-fathomdb`
  package) with `cargo metadata --filter-platform <t> --no-default-features
  -F <root_features>` after copying FathomDB's lock in. That requires Run
  2 to use FathomDB's resolved feature set, not today's `std, driver,
  dynamic-loading, cuda-12060` (workflow line 208): see fork todo FT-3
  (§ 13.1, R-DES open-point ruling 3). The workflow writes the JSON as a
  job artifact; the receipt commit copies it. This is a fork-CI (`W`)
  change plus records (`D`), so it does not move `C` (cudarc fork plan
  § 8: "fork CI workflow (`W`) only | fork CI").
- **FathomDB's copy.** `scripts/cudarc-fathomdb-lock-set.json` holds the
  same object plus `fork_record`: `{"repository":
  "https://github.com/coreyt/cudarc", "path": "<the record path>",
  "commit": "<the fork commit that added it>"}`.
- **The guard.** `scripts/check-cudarc-fathomdb-lock-set.py` (stdlib,
  `tomllib` or `tomli`), run in the fast tier through
  `scripts/tests/test_check_cudarc_fathomdb_lock_set.sh`
  (`run_tier_suite fast test-check-cudarc-fathomdb-lock-set …`). Inputs:
  `--root <dir>` (runs `cargo metadata` there) or `--metadata
  <x86_64.json> <aarch64.json>` (pre-computed metadata, for hermetic
  tests), and `--record <file>`. It:
  1. computes both targets' sets and the root features. It needs exactly
     one `cudarc-fathomdb` node; zero or several is a precondition error
     that names `scripts/check-pinned-override-rot.py`, which owns that
     fact (§ 3.4), and is not tested separately here;
  2. checks the record's own `set_sha256` against its contents (a
     hand-edited record fails);
  3. fails when the recomputed digest differs from the record's, printing
     per target the added, removed and changed packages and any change of
     root features, with the instruction: "push the commit that carries
     this Cargo.lock to origin so raw.githubusercontent.com serves it;
     the cudarc fork must set FATHOMDB_LOCK_SHA to it, run its hosted test
     job and record the set (cudarc fork plan § 8, last row); then copy
     the record to scripts/cudarc-fathomdb-lock-set.json".
  It does not read the fork. On the real tree it runs `cargo metadata`
  offline (the tier's `cargo fetch --locked` of § 3.5 fills the cache).
  T-FORK-EVIDENCE (R-VERIFY) checks that the fork record at
  `fork_record.commit` equals FathomDB's copy and that its `ci_run_id` is a
  successful run at `ci_head_sha`.

**The W6 sequence (no cycle).** RED: the guard and its fixture tests, with
no record (fails: "no record"). Then the W6 manifest and lock commit is
**pushed** on the slice branch: the fork's Run 2 fetches the lock with
`curl` from
`https://raw.githubusercontent.com/fathomadb/fathomdb/${FATHOMDB_LOCK_SHA}/Cargo.lock`
(fork workflow line 234 at `C`), so the commit must be on GitHub before the
fork runs. The fork sets `FATHOMDB_LOCK_SHA` to it, runs, and records the
set; FathomDB copies the record in a later commit that does not change
`Cargo.lock`, so the copy never invalidates itself. GREEN. If the fork run
fails, that is a fork finding (the pin chain restarts there). There is no
cycle: the record names the lock commit, and the lock does not depend on
the record.

**Evidence.** Fork workflow lines 32-35, 208 and 231-300 at `C` (Run 2
already fetches FathomDB's lock by commit, prints its SHA-256, runs `cargo
metadata --filter-platform "$host"` and asserts every resolved dependency
of cudarc equals FathomDB's lock); fork CI receipt `2026-10-12-CI-b777f8d.md`
(both runs passed, 55 and 56 tests; the set is not recorded); `git show
4c327cd8:Cargo.lock` (SHA-256
`5b9e680660c861f44c0476576adbfb686eb3cb3854a3fc928f0f03ed5d2fac52`,
vendored `cudarc` 0.19.7); the closure measurement above; requirements
AC31-14 (D-16 bullet); Q8, Q23.

**Consequences.** FathomDB (W6): the guard, its test, the record copy, its
fixtures, one tier line. The cudarc fork (fork work, its own plan and
review): Run 2's set emission with FathomDB's feature set, the
`FATHOMDB_LOCK_SHA` update, the record and receipt (fork todo FT-3).

**Tests** (`test_check_cudarc_fathomdb_lock_set.sh`; fixtures under
`scripts/tests/fixtures/cudarc-lock-set/`, each a pair of hand-minimal
`cargo metadata` JSON files plus a record, so the tests are hermetic and
need neither `cargo` nor a consistent lock; R-DES finding 3):

- `matching_metadata_passes` (`matching/`);
- `changed_libloading_without_a_record_fails` (`changed-libloading/`: the
  record of `matching/`, metadata with `libloading` 0.9.1), the RED named
  by AC31-14;
- `added_dependency_fails`, `removed_dependency_fails`;
- `changed_root_features_fail` (`f8` dropped from the root);
- `build_only_dependency_change_passes` (a `build`-kind edge's package
  changes) and `windows_only_dependency_change_passes` (a package absent
  from both filtered graphs changes): the Q23 exclusions;
- `record_with_a_wrong_set_sha256_fails`;
- `unrelated_lock_change_passes` (a package outside the closure changes);
- GREEN: the real tree (`--root`) with the copied fork record.

## 8. D-17 Product adaptation to the fork API

**Decision.** No product call site has to change to compile: the fork kept
every name and signature the product uses. Two call sites change by
choice, and two new sites are added.

**API difference, vendored 0.19.7 tree versus `C`** (read-only diff,
2026-10-13):

| Item | Vendored (`third_party/cudarc-0.19.7`) | Fork at `C` |
| --- | --- | --- |
| `MemPoolProps { max_size, release_threshold }` | `mem_pool.rs:11` | `mem_pool.rs:11`, same fields; a nonzero `max_size` now fails with `CUDA_ERROR_NOT_SUPPORTED` below bindings or driver 12020, and with `CUDA_ERROR_INVALID_VALUE` when a 13.2+ read-back is below the request (fork D-4) |
| `CudaMemPool::create`, `trim_to`, `raw`, `Drop` | lines 96, 146, 151, 156 | lines 276, 394, 399, 404; same signatures |
| `CudaMemPool::attribute(CUmemPool_attribute) -> u64` | line 120 | **removed** |
| typed reads `reserved_current`, `reserved_high`, `used_current`, `used_high`, `release_threshold`; `max_size() -> Option<u64>` | — | **added**, lines 330-382 |
| `AllocMode`, `new_with_mem_pool`, `alloc_mode`, `mem_pool`, `ordinal`, `cu_ctx`, `synchronize` | `core.rs:52, 342, 553, 559, 570, 630, 654` | `core.rs:52, 355, 566, 572, 583, 643, 667`; same signatures and bodies for `new_with_mem_pool` |
| `driver::result` public functions | — | identical list |
| loaded-library API (`try_culib`, `loaded_library`, `LibLoadError`, `LoadedLibrary`) | — | **added** (item 9) |

**Product call sites (all in `cuda_pool_policy.rs` unless named):**

| # | Site | Uses | W6 change |
| --- | --- | --- | --- |
| 1 | `cuda_pool_policy.rs:1344-1346` | imports `AllocMode`, `CudaContext`, `CudaMemPool`, `DriverError`, `MemPoolProps` | none |
| 2 | `:1356` | `DECISION` static type | none |
| 3 | `:1369-1384` `pool_attribute` | `result::mem_pool::get_attribute(pool.raw(), …)` in an `unsafe` block | **deleted** (site 10 no longer needs it) |
| 4 | `:1406` | `type Pool = Arc<CudaMemPool>` | none |
| 5 | `:1458-1470` `create_pool` | `MemPoolProps`, `CudaMemPool::create` | none; its new error codes map to `pool_create_failed` as every create failure does (Slice 30 § 8.1) |
| 6 | `:1472-1483` `probe_pool` | `result::mem_pool::alloc_async`, `result::free_async`, `result::ctx::synchronize` | none |
| 7 | `:1485-1491` `new_private_context` | `CudaContext::new_with_mem_pool` | none |
| 8 | `:1493-1498` `context_id` | `cu_ctx()` | none |
| 9 | `:1500-1502` `wrap_context` | Candle `Device::new_cuda_from_context` | none |
| 10 | `:1530-1537` `pool_counters` | four raw attribute reads through site 3 | **changed** to `pool.reserved_current()`, `reserved_high()`, `used_current()`, `used_high()`; same values, no `unsafe` |
| 11 | `:1544-1546` `live_private_contexts` | `Arc::strong_count` | none |
| 12 | `:1591-1598` `allocator_report` | `alloc_mode()`, `AllocMode`, `ordinal()` | none |
| 13 | `:1605-1612` `private_pool_counters` (test helper) | `synchronize()`, site 10 | none |
| 14 | `cuda_pool_policy_tests.rs:1423, 1430` | `AllocMode`, `alloc_mode()` | none (file frozen, AC31-10) |
| N1 | new, driver part | close release (below) | **added** |
| N2 | new, `candle_bge.rs:257-283` and `candle_reranker.rs:118-…` `probe_cuda` | D-18 check before `new_cuda_device` | **added** (§ 9.1) |

So the list has 14 existing sites: 2 change (3, 10) and 12 stay; there is
no rename. N1 and N2 are new.

**Error mapping.** Unchanged. A private-pool `DriverError`
`CUDA_ERROR_OUT_OF_MEMORY` (2) is `cuda_pool_exhausted`
(`cuda_pool_policy.rs:60-61, 497-501`); `CUDA_ERROR_CONTEXT_IS_DESTROYED`
(709), `CUDA_ERROR_INVALID_CONTEXT` (201), an inactive primary context or a
changed id is `cuda_context_lost` (`:62-65, 655`; Slice 30 § 3.5). The
fork adds no new error code on the allocation path. Its new `create`
codes (site 5) occur only at the decision and fall back. Item 9's
`LibLoadError` never reaches this mapping, because D-18 checks the
libraries first.

**Reporting.** `doctor cuda-allocator` and the snapshot keep their fields.
The pool counters keep their values. Since FathomDB frees pool memory only
with `cuMemFreeAsync` (Candle slices drop through cudarc's
`CudaSlice::drop`; the probe uses `free_async`), the fork's finding that
`cuMemFree` never decrements `USED_MEM_CURRENT` (cudarc D-2) does not make
FathomDB's counters stale. The report's `pool_max_size_bytes` stays the
decided size: `max_size()` is `None` on the 12.6 driver (read-back needs
bindings and driver 13020+, fork D-4). On JetPack 7.2 (UNMEASURED, Q5) a
read-back above the request would be reported by a later slice.

**Close (N1).** A new driver-part function
`release_unused_private_pool_memory()` runs when a FathomDB-built
private-pool model is dropped: when the decision is `Private` and
`classify`'s loss detection (`cuda_context_state` active and
`retained_primary_context_id` equal to the recorded id) finds the context
alive, it calls `first_context.synchronize()` and then
`pool.trim_to(0)`; on loss, or on any driver error, it does nothing. A
guard field, declared last in `CandleBgeEmbedder` and in the reranker so
that Rust drops it after the model and device fields, calls it. Why:
release threshold 0 returns unused memory only at a synchronization point
(cudarc D-5), and nothing synchronizes after an engine closes, so the
freed bytes stay reserved until the process's next CUDA work; with
`FATHOMDB_POOL_RELEASE_THRESHOLD=max` they are never released.
`trim_to(0)` releases only unused memory, so another engine on the same
pool is unaffected (it re-reserves on demand). This is a behaviour change:
the changelog records it.

N1 is implemented by owner ruling Q29; § 14.1 gives its final behaviour
and tests.

**Evidence.** Vendored `third_party/cudarc-0.19.7/src/driver/safe/mem_pool.rs`
and `core.rs` (lines above); `git show b777f8d:src/driver/safe/mem_pool.rs`
and `core.rs` (lines above); cudarc fork `fathomdb/design.md` § 1 table
("Changes from the vendored API") and its note that FathomDB reads counters
through `result::mem_pool::get_attribute`; § 2 (`USED_MEM_CURRENT` with
`cuMemFree`); § 4 (`create` rule, read-back); § 5 (threshold and
`trim_to`); Slice 30 status row S30-T10b (CB2 measured with a
synchronize).

**Consequences (W6).** `cuda_pool_policy.rs` sites 3 and 10; N1 in
`cuda_pool_policy.rs`, `candle_bge.rs` and `candle_reranker.rs`; the
changelog line.

**Tests.**

- Sites 3 and 10: the Slice 30 tests that read counters
  (`gpu_orin_64_pool_stays_under_max_size_and_is_empty_after_the_embedder_drops`
  and the context-reset snapshot assertion
  `"pool":{"reserved_mem_current":`) stay GREEN unchanged (a refactor
  under existing tests).
- N1, RED first:
  `gpu_orin_64_dropping_the_embedder_releases_pool_memory_without_a_caller_synchronize`
  (new unit-test file, § 6): drop the embedder, then read the counters
  **without** synchronizing (a new `#[cfg(test)]` reader beside
  `private_pool_counters`); `reserved_current` is 0. With
  `FATHOMDB_POOL_RELEASE_THRESHOLD=max` in a child process, the same
  assertion holds (`gpu_orin_64_close_release_also_trims_a_max_threshold_pool`).
- N1 skipped on loss: the context-reset test still reaches its SIGSEGV with
  the snapshot unchanged (the guard runs after the faulting drop and checks
  loss first).

## 9. D-18 Loaded-library check, the GPU floor and T-CUDA-COMPAT C3

### 9.1 D-18 Loaded-library check in production

**Decision.**

1. **Path to the fork API.** On x86_64 the embedder has no `cudarc`
   dependency of its own, and after W6 none on aarch64 either (§ 3.2); the
   removed aarch64 one had no `cublas` or `curand` feature anyway. The
   check therefore reaches the fork through Candle's re-export, the path
   every product `cudarc` call already uses (§ 0):
   `candle_core::cuda::cudarc::cublas::sys::try_culib()` and
   `candle_core::cuda::cudarc::curand::sys::try_culib()` (both `pub unsafe
   fn`, each called in an `unsafe` block whose `SAFETY` comment says it only
   loads and version-checks the library and calls no CUDA function),
   `…::cublas::sys::loaded_library()`, `…::curand::sys::loaded_library()`
   and `…::driver::sys::loaded_library()` (safe), and the types
   `candle_core::cuda::cudarc::{LibLoadError, LoadedLibrary, LibVersion,
   RejectReason}` (`src/lib.rs:237` at `C`), and (revision 3, Q26)
   `…::cublas::sys::is_symbol_present(name)` and
   `…::curand::sys::is_symbol_present(name)` (`pub unsafe fn … ->
   Result<bool, LibLoadError>`; `git show b777f8d:src/cublas/sys/mod.rs`
   lines 8838-8841, `src/curand/sys/mod.rs` line 602). Candle 0.11.0 enables
   `cublas`, `cublaslt`, `curand`, `nvrtc` and `dynamic-loading` on it
   (§ 0), so these items exist in every `embed-cuda` or `rerank-cuda`
   build.
2. **Check.** A new embedder module `cuda_libraries.rs` (pure part on every
   host; driver part under `embed-cuda` or `rerank-cuda`) exposes
   `require_usable_cuda_libraries()` (revision 2's
   `require_built_major_libraries()`, renamed because it now also checks
   symbols). Both concrete `probe_cuda`
   implementations call it before `new_cuda_device`
   (`candle_bge.rs:257-262`, `candle_reranker.rs:118-121`), after the
   resolver's floor check (§ 9.2). It:
   - calls the two `try_culib()` functions (the libraries Candle's device
     construction loads: one `CudaBlas` and one `CudaRng` per device), so a
     missing or wrong-major library is a typed error before any GPU work,
     instead of the `culib()` panic inside Candle's device construction;
   - then requires, for cuBLAS, `LoadedLibrary.version.major ==
     LoadedLibrary.cuda_major`. The fork version-checks cuBLAS only when it
     is opened by its unversioned name; a file named `libcublas.so.12` that
     reports another major is accepted by the fork (cudarc `FATHOMDB.md`
     item 9, "Unversioned names are version-checked") and refused here;
   - then (revision 3, owner ruling Q26, steward ledger seq 329) requires
     every symbol of the library's **required symbol set** to be present,
     by `is_symbol_present(name)` on the library already loaded. A CUDA 12
     cuBLAS or cuRAND that lacks one is refused with reason
     `MissingSymbol` and the list of missing names, before any device is
     built. Without this, a missing symbol panics at its first call
     (cudarc's `load`, `src/cublas/sys/mod.rs:10-13` at `C`: `panic!("Missing
     symbol {name}: {e}")`), which for a GEMM symbol is the first embed;
   - maps a failure to a new `CudaProbeError::LibraryUnavailable(
     CudaLibraryRefusal)` (renamed from revision 1's
     `LibraryMajorMismatch`, R-DES finding 5) with `library`, `reason`
     (`CudaLibraryRefusalReason::Missing` when no candidate of the built
     major loaded, `WrongMajor` when a loaded file reports another major,
     or `MissingSymbol` when a loaded file lacks a required symbol),
     `path` (`Option<String>`), `version` (`Option<String>`),
     `required_major` (`u32`; the library's own major, 12 for cuBLAS and
     10 for cuRAND, as the fork's `LibLoadError.cuda_major` gives it),
     `tried` (each name tried, with the fork's `RejectReason` text) and
     `missing_symbols` (`Vec<String>`, empty unless `MissingSymbol`). The pure function
     `refusal_from_lib_load_error(&LibLoadError) -> CudaLibraryRefusal`
     does the mapping (`LibLoadError`'s fields are public, so a test can
     build one).
   - **Message and remedy.** The message names the library, the reason,
     the file and version if one was found, and the major needed. For
     cuBLAS or cuRAND with reason `Missing` it gives one remedy that
     covers both libraries (revision 3, R-TEST finding 3): "install the
     CUDA 12 cuBLAS and cuRAND wheels (`pip install nvidia-cublas-cu12
     nvidia-curand-cu12`) and put both library directories on
     `LD_LIBRARY_PATH`, for example `export LD_LIBRARY_PATH=\"$(python3 -c
     'import nvidia.cublas as m; print(m.__path__[0])')/lib:$(python3 -c
     'import nvidia.curand as m; print(m.__path__[0])')/lib:$LD_LIBRARY_PATH\"`;
     the CUDA library loader does not search Python's site-packages on
     Linux". The wheel directories are `nvidia/cublas/lib` and
     `nvidia/curand/lib`. cuRAND's soname major is 10 (`libcurand.so.10`),
     so no text or test states major 12 for cuRAND. For `WrongMajor` it
     says which file to remove from the search path. For `MissingSymbol`
     it names the missing symbols and says to install a newer CUDA 12
     copy of that library (the same wheel remedy). The 13.0 artifact is named as the
     alternative only once it ships. This is T-CUDA-COMPAT C3's expected
     message (§ 9.3).
   - The library is never used: the probe fails, so no device is built.
   - **The required symbol set (Q26).** Every cuBLAS and cuRAND call
     FathomDB's CUDA path makes goes through Candle 0.11.0, and Candle
     calls these libraries only through cudarc's `cublas` and `curand`
     safe and `result` layers (`CudaBlas::new`, `Gemm` for GEMM and
     strided-batched GEMM, `gemm_strided_batched_ex`, `CudaRng`;
     `candle-core/src/cuda_backend/device.rs:274-275, 304`, `mod.rs:8,
     2233, 2539, 2598`). Those layers reach the `sys` functions called in
     cudarc's `src/cublas/result.rs` and `src/curand/result.rs`, plus the
     safe-layer calls `cublasSetPointerMode_v2`, `cublasGetPointerMode_v2`
     and `cublasGemmGroupedBatchedEx`, which Candle never calls
     (`src/cublas/safe/mod.rs:74, 84`, `safe/grouped_gemm.rs`). So the set
     is the `sys::cublas*` and `sys::curand*` **function** names called in
     those two `result.rs` files at `C`. Measured 2026-10-13, 13 cuBLAS
     names (`cublasCreate_v2`, `cublasDestroy_v2`, `cublasSetStream_v2`,
     `cublasSgemm_v2`, `cublasDgemm_v2`, `cublasSgemv_v2`,
     `cublasDgemv_v2`, `cublasSasum_v2`, `cublasDasum_v2`,
     `cublasGemmEx`, `cublasGemmStridedBatchedEx`,
     `cublasSgemmStridedBatched`, `cublasDgemmStridedBatched`) and 14
     cuRAND names (`curandCreateGenerator`, `curandDestroyGenerator`,
     `curandSetStream`, `curandSetPseudoRandomGeneratorSeed`,
     `curandSetGeneratorOffset`, `curandGenerate`, `curandGenerateLongLong`,
     `curandGenerateUniform`, `curandGenerateUniformDouble`,
     `curandGenerateNormal`, `curandGenerateNormalDouble`,
     `curandGenerateLogNormal`, `curandGenerateLogNormalDouble`,
     `curandGeneratePoisson`). These are long-standing entry points; C2's
     12.1 and 12.9 cells confirm that none of them narrows the 12.1–12.9
     range. The set is a checked-in constant in `cuda_libraries.rs`
     (`REQUIRED_CUBLAS_SYMBOLS`, `REQUIRED_CURAND_SYMBOLS`), not derived
     at run time.
   - **Drift guard.** A new fast-tier checker
     `scripts/check-cuda-required-symbols.py` finds the resolved
     `cudarc-fathomdb` and `candle-core-fathomdb` sources with `cargo
     metadata --locked --offline` (`manifest_path`) and fails if (a) the
     constants differ from the `sys::` function names called in
     cudarc's `cublas/result.rs` and `curand/result.rs`, (b) Candle's
     sources call a `cublas::sys` or `curand::sys` function directly or a
     safe-layer method outside the `result` layer (`set_pointer_mode`,
     `get_pointer_mode`, grouped GEMM), or (c) a FathomDB crate calls a
     `cublas::sys` or `curand::sys` function. A pin move that changes the
     set therefore fails the fast tier until the constant is updated.
   - **Where the check lives, and why not in the fork.** In FathomDB.
     The fork already exposes `is_symbol_present` at `C`, so no fork
     change, new `cudarc-fathomdb` version, re-qualification or
     publication is needed. The fork cannot enumerate missing symbols
     itself: each `sys` function resolves its symbol lazily in a
     per-function `OnceLock` and panics on failure (`load`, lines 10-13),
     and nothing lists the symbols a consumer will call. The consumer
     knows its call set; the fork does not.
3. **Resolution.** A new `DeviceResolutionReason::CudaLibraryUnavailable`
   (`"cuda_library_unavailable"`) and the reranker's
   `RerankerDeviceResolutionReason::CudaLibraryUnavailable`. Under `auto`
   the component runs on CPU with that reason; under forced `cuda:N`,
   `DeviceResolutionError::CudaLibraryUnavailable { ordinal, refusal }`,
   whose kind is `cuda_library_unavailable`, so the refusal's fields reach
   the caller (today's `ForcedCudaUnavailable` keeps only a reason). A
   private-pool decision is never reached, so rule 3 (fail closed) is not
   involved.
4. **Report.** `CudaDeviceInfo` (already `#[non_exhaustive]`) gains
   `cuda_libraries: Vec<CudaLibraryReport>`; `CudaLibraryReport { library:
   String, opened_as: String, path: Option<String>, version:
   Option<String>, built_major: u32 }`, from the fork's `loaded_library()`
   for the driver, cuBLAS and cuRAND, and for cuBLASLt and NVRTC only when
   cudarc loaded them. The napi and Python `CudaDeviceInfo` mirrors gain the
   field.
5. **Doctor.** A new sibling verb `fathomdb doctor cuda-libraries`
   (`fathomdb.doctor.cuda-libraries.v1`), following Slice 30's choice to
   leave `doctor gpu` v1 unchanged: `built_major`, one entry per library
   (`library`, `opened_as`, `path`, `version`, `status`: `loaded`,
   `refused` or `not_loaded`), `refusal` (the fields above, or `null`),
   and `maps` (the CUDA libraries in `/proc/self/maps`, by the existing
   `cuda_libraries_in_maps`). It always reports (Q24). Exit 70 when a
   refusal occurs, else 0.
6. **Logging (owner ruling Q24, steward ledger seq 327).** Only a refusal
   writes to stderr: one line per process, under a `Once`, at the refusing
   probe: `fathomdb-cuda-libraries {json}` with the refusal (the Slice 30
   `fathomdb-cuda-context-lost` precedent). A successful load writes
   nothing; the report and `doctor cuda-libraries` carry it. No new egress.
7. **Public surface** (AGENTS.md § 1; R-DES open-point ruling 5: keep the
   typed error, the report field and the doctor verb). W6 updates, in the
   same commits as the code: `dev/interfaces/rust.md` (new
   `CudaProbeError`, `DeviceResolutionReason`, `DeviceResolutionError`
   variants, `CudaDeviceInfo.cuda_libraries`, `CudaLibraryReport`,
   `CudaLibraryRefusal`; `tegra-pool` adds no dependency, § 3.2),
   `dev/interfaces/rust-sdk.md` (the SDK's error kind list and its
   vendored-cudarc text), `dev/interfaces/python.md` and
   `dev/interfaces/typescript.md` (the `CudaDeviceInfo` field and the
   error kind), `dev/interfaces/cli.md` (`doctor cuda-libraries`, its
   schema and exit codes; the `cuda_incompatible` rows of the `doctor gpu`
   table gain the compute-capability floor as listed architecture
   evidence, § 9.2). `dev/interfaces/wire.md` names no CUDA error kind
   today (`grep`, 2026-10-13); W6 confirms it needs no change and says so
   in `status.md`. User docs: `docs/embedder.md`, `docs/reference/cli.md`;
   `docs/reference/errors.md` lists no CUDA kind today, so W6 either adds
   the CUDA kinds there or records why `docs/embedder.md` stays their
   home. The policy ADR amendment (appendix B) records the kind.
8. `CudaProbeError` (`device_policy.rs:182-192`) and
   `DeviceResolutionError` (`:372-381`) are not `#[non_exhaustive]`, so the
   new variants are public breaks, recorded in the changelog (pre-1.0; no
   shim, AGENTS.md § 10).

**Evidence.**

- Fork API: `git show b777f8d:src/cublas/sys/mod.rs` lines 8821-8850
  (`try_culib` and `is_culib_present` are `pub unsafe fn`;
  `loaded_library` is safe); `src/lib.rs` lines 237 and 290-366 (the
  re-exports; `LibLoadError { library, cuda_major, tried }` with public
  fields; `RejectReason`); `src/driver/sys/mod.rs` line 16896
  (`loaded_library`); cudarc `fathomdb/design.md` § 12.4 and `FATHOMDB.md`
  item 9. The fork's own refusal message (`src/lib.rs:340-363`) ends
  "Ensure that `LD_LIBRARY_PATH` has the correct path to a CUDA 12 copy of
  the library", and the fork never searches site-packages on Linux, which
  is why the remedy names `LD_LIBRARY_PATH` and not only `pip install`.
- Candle at `K9`: `cudarc-fathomdb` features include `cublas`, `cublaslt`,
  `curand`, `nvrtc` (`Cargo.toml` lines 43-54); `device.rs:275`
  (`CudaRng::new` at device construction); `device.rs:185-189` (NVRTC only
  in a custom-kernel compile path FathomDB does not call; C2's
  `/proc/self/maps` cross-check confirms).
- Today's probe: `candle_bge.rs:257-283`, `candle_reranker.rs:118-…`;
  `CudaProbeError` (`device_policy.rs:182-192`) and `DeviceResolutionError`
  (`:372-381`).

**Consequences (W6).** `fathomdb-embedder`: `cuda_libraries.rs`,
`device_policy.rs`, `reranker_device_policy.rs`, `candle_bge.rs`,
`candle_reranker.rs`; `fathomdb-cli` (`doctor cuda-libraries`); napi and
py `CudaDeviceInfo` and error mapping; the docs and interface files above.

**Tests (RED first).**

- `a_lib_load_error_for_a_missing_cuda_12_cublas_is_a_typed_refusal_with_the_remedy`
  (hermetic, every host, `cuda_libraries.rs` unit test; R-DES finding 5):
  builds `LibLoadError { library: "cublas", cuda_major: 12, tried:
  [("libcublas.so", NotFound), ("libcublas.so.12", NotFound)] }`, passes it
  through `refusal_from_lib_load_error` and the resolution mapping, and
  asserts reason `Missing`, kind `cuda_library_unavailable`, and a message
  that contains `pip install nvidia-cublas-cu12 nvidia-curand-cu12`,
  `nvidia.cublas`, `nvidia.curand` and `LD_LIBRARY_PATH`. A
  second case with `WrongVersion(13.1.1)` gives `WrongMajor`. RED: the
  function and variant do not exist. It stays whatever happens to C3
  (Q25).
- `a_libcublas_so_12_that_reports_major_13_is_refused_before_gpu_work`
  (new `tests/slice31_cuda_library_major.rs`, `embed-cuda`, Linux): a child
  process with `LD_LIBRARY_PATH` naming a directory with a decoy cuBLAS,
  built at test time by `cc` from the fixture
  `tests/fixtures/decoy-cublas/decoy_cublas.c` (it exports
  `cublasGetProperty`, reporting major 13, and nothing else) and placed
  under both `libcublas.so.12` and `libcublas.so`, so that a loader that
  tries the unversioned name first (the vendored 0.19.7,
  `third_party/cudarc-0.19.7/src/lib.rs:219`) also opens the decoy rather
  than the 12.6 copy in `windchill3`'s loader cache. Expected:
  `LibraryUnavailable` with reason `WrongMajor`, the decoy's path,
  version 13 and required major 12. **RED (revision 3, R-TEST finding
  6):** the child panics in device construction (`cublasCreate_v2`
  'Expected function…' via dynamic loading); no `LibraryUnavailable`.
  (The cudarc source at `C` and at 0.19.7 words this panic `Missing
  symbol cublasCreate_v2: …`, from `load`; the RED is the panic and the
  absence of the refusal, whatever the wording.) The GREEN assertion
  requires the typed refusal in the child's output, so a panicking child
  can never pass. On `windchill3` the child runs with
  `CUDA_VISIBLE_DEVICES=<RTX 3090 UUID>` (§ 0.1); GREEN builds no device.
- `a_libcublas_so_12_missing_a_required_symbol_is_refused_before_gpu_work`
  (same file; revision 3, Q26): a second decoy, fixture
  `tests/fixtures/decoy-cublas/decoy_cublas_missing_symbol.c`, placed the
  same way, reports version 12.6 through `cublasGetProperty` and exports
  every name of `REQUIRED_CUBLAS_SYMBOLS` as a stub returning
  `CUBLAS_STATUS_SUCCESS` (with a non-null handle from
  `cublasCreate_v2`) **except `cublasSgemmStridedBatched`**. The child
  resolves forced `cuda:0` through the embedder's `probe_cuda` and, if a
  device comes back, runs one F32 batched `matmul` of two `2×2×2` Candle
  tensors on it (Candle's matmul reaches cudarc's
  `Gemm<f32>::gemm_strided_batched`, that is `cublasSgemmStridedBatched`;
  W6 confirms the call path in the RED log). **RED:** the probe selects
  the device and the matmul panics on the missing symbol (`Missing symbol
  cublasSgemmStridedBatched` on the pinned tree; on the vendored base the
  same `load` panic); no `LibraryUnavailable`. **GREEN:** the child's
  output contains `LibraryUnavailable` with reason `MissingSymbol`,
  `missing_symbols == ["cublasSgemmStridedBatched"]`, the decoy's path and
  version 12.6; no device built, no matmul run; the child exits 0. The
  GREEN assertion again requires the typed refusal in the output.
- `missing_cuda_12_cublas_is_a_typed_refusal_with_the_remedy` (same file):
  the process sees no CUDA 12 cuBLAS. It is `#[ignore = "C3 GCP cell"]`
  (revision 3, R-TEST finding 5), listed in `scripts/test-skip-allowlist.toml`
  as `ignored-by-design`, and run only in the C3 session with `--ignored
  --exact` (§ 9.3); it asserts its precondition (no CUDA 12 cuBLAS
  loadable) and fails if a copy is found. It covers forced `cuda:0`
  (error kind `cuda_library_unavailable`, reason `Missing`, the remedy
  text) and `auto` (CPU with reason `cuda_library_unavailable`).
- `the_cuda_library_report_names_the_loaded_files` (RTX 3090 by UUID, and
  the Orin): every `path` in the report appears in `/proc/self/maps`. RED:
  `CudaDeviceInfo` has no `cuda_libraries` field (compile failure).
- `a_refusal_writes_one_stderr_line` (child process with the major-13
  decoy): Q24. RED: no stderr line exists.
  `a_successful_load_writes_no_stderr_line` is a **regression guard**, not
  a RED (revision 3, R-TEST finding 7): it cannot fail on the base, which
  writes no such line.
- hermetic symbol-check tests in `cuda_libraries.rs` (Q26):
  `missing_required_symbols_lists_each_absent_name_in_set_order` (the pure
  function over an injected presence predicate) and
  `a_missing_symbol_refusal_names_the_symbols_and_the_remedy`.
- the drift guard's fixture arms (`scripts/tests/test_check_cuda_required_symbols.sh`):
  `matching_sets_pass`, `a_result_layer_symbol_missing_from_the_constant_fails`,
  `a_direct_candle_sys_call_fails`, `a_fathomdb_sys_call_fails`, and the
  real tree.
- pure tests of the mapping, the reason strings and the doctor JSON
  (`cuda_libraries.rs` unit tests; CLI tests beside the Slice 30
  `doctor cuda-allocator` tests).
- T-CUDA-COMPAT C2 receipts name the cuBLAS file from this report; C3
  (§ 9.3) and C5 as plan § 7 says.

### 9.2 The compute-capability floor (T-CUDA-COMPAT C4, owner ruling Q22)

**Decision (R-DES finding 1).** The K620 is never run (Q22), so C4 is a
hermetic test, and the floor check is **unconditional**: it no longer
waits on a K620 characterization.

1. **Where.** Every caller of `CudaProvider::probe_cuda` is one of three
   generic sites: `device_policy.rs:584` (`probe`, used by resolution),
   `device_policy.rs:702` (`diagnose_gpu`) and
   `reranker_device_policy.rs:404` (`probe`) (`grep '\.probe_cuda('`,
   2026-10-13). Each already holds the enumerated `CudaVisibleDevice`,
   whose `compute_capability` comes from the driver attributes read during
   enumeration (`candle_bge.rs:225-240`). The floor check runs at those
   three sites, on the enumerated device, **before** `provider.probe_cuda`,
   and so before `new_cuda_device`. Today the concrete `probe_cuda`
   builds the device first and enumerates after
   (`candle_bge.rs:260-275`); with the check ahead of the call, no path
   builds a device below the floor. One pure function holds the rule
   (`device_policy::below_compute_floor(compute_capability: Option<&str>,
   floor: Option<(u32, u32)>) -> bool`), used by all three sites.
2. **The floor value** comes from the build's `CUDA_COMPUTE_CAP`, the
   contract value the release jobs export
   (`scripts/release/cuda-artifact-contract.sh:71-78`: `75` on x86_64,
   `87` for the Tegra build; `check-cuda-release-contract.py:781-784, 999,
   1015` already require it). The embedder reads it at compile time with
   `option_env!("CUDA_COMPUTE_CAP")` (Cargo tracks environment variables
   read by `option_env!` and rebuilds when they change) and parses `NN`
   or `NNN` as major = all digits but the last, minor = the last. No
   per-architecture literal is written in Rust. The `CudaProvider` trait
   gains a provided method `fn kernel_compute_floor(&self) -> Option<(u32,
   u32)>` whose default returns that build value; the concrete providers
   keep the default and test fakes override it. A provided trait method is
   not a break for implementors.
3. **Outcome.** A device whose compute capability is below the floor gets
   `CudaProbeError::Incompatible`, so resolution gives
   `DeviceResolutionReason::CudaIncompatible` (`cuda_incompatible`;
   `device_policy.rs:597-598`): under forced `cuda:N`,
   `ForcedCudaUnavailable { reason: CudaIncompatible }`; under `auto`, CPU
   with that reason; `doctor gpu`, status `cuda_incompatible`, which
   `dev/interfaces/cli.md:218-236` already defines for "listed
   compatibility/architecture evidence". No new error kind.
4. **`auto`.** `auto` probes only the lowest visible ordinal
   (`device_policy.rs:464-466`, `min_by_key(visible_ordinal)`). If that
   device is below the floor, `auto` falls back to CPU with
   `cuda_incompatible`; it never tries the next ordinal (unchanged
   behaviour, documented in `docs/embedder.md`: set `CUDA_VISIBLE_DEVICES`
   to a supported GPU).
5. **Unknown values.** A device whose compute capability is `None` (the
   ORT provider's fakes report none, `ort_bge.rs:137, 368, 745`) is not
   refused: there is no evidence. A build without `CUDA_COMPUTE_CAP` (a
   developer build whose kernels Candle compiled for a detected GPU) has
   no floor and refuses nothing; the D-18 report records
   `kernel_compute_cap: null` for it. Release artifacts always carry the
   value (the contract checker requires it).

**Tests (RED first, hermetic, every host; `tests/device_policy.rs` and
`tests/slice71_reranker_policy.rs` are not frozen, but the new cases go in
a new file `tests/slice31_compute_floor.rs`).**

- `forced_cuda_on_a_device_below_the_kernel_floor_is_cuda_incompatible_without_a_probe`:
  a fake `CudaProvider` enumerates one device, ordinal 0, compute
  capability `"5.0"`, and reports `kernel_compute_floor() == Some((7,
  5))`; `cuda:0` resolves to `ForcedCudaUnavailable { reason:
  CudaIncompatible }` and the fake records **no** `probe_cuda` call. RED
  today: the resolver calls `probe_cuda(0)`, the fake's stand-in for
  device construction, and selects the device.
- `auto_with_the_lowest_ordinal_below_the_floor_falls_back_to_cpu`:
  devices 0 = `"5.0"`, 1 = `"8.6"`; CPU, reason `cuda_incompatible`, no
  `probe_cuda` call. RED today.
- `auto_with_the_lowest_ordinal_above_the_floor_selects_it`: devices 0 =
  `"8.6"`, 1 = `"5.0"`; selects 0 with one `probe_cuda(0)` call (the
  lowest-ordinal rule; GREEN, a characterization).
- `a_device_at_the_floor_is_probed` (`"7.5"` with floor (7, 5)) and
  `an_unknown_compute_capability_is_probed` (`None`).
- `doctor_gpu_reports_cuda_incompatible_below_the_floor`: `diagnose_gpu`
  with the same fake; no `probe_cuda` call. RED today.
- the reranker's `resolve` with the same fakes (two cases). RED today.
- `the_build_floor_parses_the_contract_values`: the parser maps `"75"` to
  (7, 5), `"87"` to (8, 7), `"120"` to (12, 0), and rejects empty or
  non-digit text.

C4 passes when these pass; no process on `windchill3` opens the K620.

### 9.3 T-CUDA-COMPAT C3 on a GCP VM (owner ruling Q25)

The owner asked (Q25, steward ledger seq 328) to explore running C3 (no
CUDA 12 cuBLAS present) on a GCP instance instead of pulling a container
image. The hermetic test of § 9.1 stays regardless; this cell adds the
real loader on a real host.

**Why a VM works for C3.** A GCP Ubuntu 24.04 image with only the NVIDIA
driver has no cuBLAS at all: `scripts/gcp-lab/remote-driver.sh` installs
`nvidia-headless-no-dkms-580-server`, `nvidia-utils-580-server` and the
prebuilt, signed module (research `cloud-gpu-test-options.md`, "The `t4`
profile"), which carry `libcuda` and the driver's own libraries but no
CUDA toolkit libraries. That is exactly C3's environment, without the
CUDA 13 runtime image. It does not show the "CUDA 13 libraries present"
half of the old C3 (the `-devel` image's unversioned `libcublas.so` →
`.so.13`); the decoy test (§ 9.1) covers a wrong-major file
hermetically, and the optional step 6 below can add `nvidia-cublas` 13.

**Build elsewhere, run on the VM.** Building needs `nvcc` (Candle's
kernels) and the 12.6 toolkit's `libcudart_static` (the `candle-kernels`
patch, REQ31-16). Installing a toolkit on the VM would also install a
cuBLAS (`libcublas-dev` is part of the fork-cell package set) and spoil
the cell. So the artifacts are built on `windchill3` from the W6 tree,
with `/usr/local/cuda` 12.6 (never changed), `CUDA_COMPUTE_CAP=75` and
`CUDARC_CUDA_VERSION=12060`, and no GPU used by the build (§ 0.1).
`windchill3` and the VM image are both Ubuntu 24.04 with glibc 2.39
(`/etc/os-release`, `ldd --version` on `windchill3`, 2026-10-13; the
script's image family is `ubuntu-2404-lts-amd64`), so the binaries run on
the VM. The VM gets (by `gcp-cuda-lab.sh push`):

- the `slice31_cuda_library_major` test binary (`cargo test --no-run
  --features embed-cuda -p fathomdb-embedder --test
  slice31_cuda_library_major`);
- the embedder's existing live test binaries, built `--no-run` (revision
  3, R-TEST finding 1: `fathomdb-cli` has only the `recover` and `doctor`
  roots, `fathomdb-cli/src/lib.rs:100-103`, and the CLI-scope ADR rejects
  an application query surface, so no CLI embed or rerank exists): the
  integration test `tests/candle_bge.rs` (`--features
  embed-cuda,loader-test-hooks --test candle_bge`), which builds
  `CandleBgeEmbedder::new()` under `FATHOMDB_EMBED_DEVICE`, and the
  library test binary with `candle_reranker::gpu_tests::gpu_loads_and_scores_finite`
  (`--features rerank-cuda --lib`), which loads the reranker on `cuda:0`;
- the CUDA CLI built as the release CLI archive
  (`scripts/release/seal-cuda-cli-archive.sh`), for `doctor
  cuda-libraries`, `doctor gpu` and `doctor reranker-gpu` only (the
  refusal and floor checks);
- the cached BGE and reranker model files, so the cell needs no model
  download.

**Session** (each needs owner approval, Q15, Q25; test and verification
only, no development on a Spot VM; at most 2 h; capacity order Spot T4,
on-demand T4, then L4):

1. `gcp-cuda-lab.sh create --profile t4 --name fdb-lab-c3 --zone <z>
   --max-run 2h` (Spot `n1-standard-4`, one T4, 100 GB boot disk,
   self-deleting at the cap), `wait-ssh`, then `driver --driver 580` (one
   reboot, measured in the pilot).
2. **Preconditions, recorded in the receipt:** `nvidia-smi` shows
   `Tesla T4`, driver 580.x, compute capability 7.5; `ldconfig -p | grep
   -c libcublas` is 0 and `find / -xdev -name 'libcublas*'` is empty; no
   `/usr/local/cuda*`; `LD_LIBRARY_PATH` unset.
3. **Refusal:** `FATHOMDB_REQUIRE_LIVE=1 <slice31_cuda_library_major
   binary> --ignored --exact
   missing_cuda_12_cublas_is_a_typed_refusal_with_the_remedy` (1 passed):
   forced `cuda:0` fails with kind `cuda_library_unavailable`, reason
   `missing`, and the remedy text; `auto` resolves to CPU with reason
   `cuda_library_unavailable`; the stderr line appears once (Q24).
   `fathomdb doctor cuda-libraries --json` exits 70 with `refusal.reason
   = "missing"`, `library = "cublas"`, the tried names and the remedy
   text; `doctor gpu` and `doctor reranker-gpu` are recorded. The
   live embed test under `FATHOMDB_EMBED_DEVICE=auto` (`<candle_bge
   binary> --exact embed_returns_unit_norm_vector`) passes on CPU.
4. **Floor boundary (free evidence):** the T4 is compute capability 7.5,
   the x86 floor, so § 9.2 must admit it: `doctor gpu` does not report
   `cuda_incompatible`.
5. **Remedy validated:** in a venv, `pip install nvidia-cublas-cu12
   nvidia-curand-cu12` (exact versions in the receipt; PyPI is a
   permitted registry), export `LD_LIBRARY_PATH` exactly as the remedy
   text says (both `nvidia/cublas/lib` and `nvidia/curand/lib`), then
   `doctor cuda-libraries` exits 0 with cuBLAS `loaded` from the venv's
   `nvidia/cublas/lib` and cuRAND (major 10) from its `nvidia/curand/lib`.
   The forced embed and rerank on the T4 are the live test binaries:
   `FATHOMDB_EMBED_DEVICE=cuda:0 FATHOMDB_REQUIRE_LIVE=1 <candle_bge
   binary> --exact embed_returns_unit_norm_vector` and
   `FATHOMDB_REQUIRE_LIVE=1 <lib test binary> --exact
   candle_reranker::gpu_tests::gpu_loads_and_scores_finite`, each 1
   passed. That also runs the F32-only kernels on sm_75 (§ 3.7).
   **Then the `libcuda.so.1`-only cell (CUDA-09, revision 3, R-TEST
   finding 2; moved from S-13, whose VM has only a 13.0 cuBLAS):** `sudo
   dpkg-divert --rename --add /usr/lib/x86_64-linux-gnu/libcuda.so` (lab
   VM only, Q15a), then the same forced embed again, with `grep libcuda
   /proc/<pid>/maps` (or the D-18 report) showing only `libcuda.so.1`.
6. **Optional (same session, if time remains):** `pip install
   nvidia-cublas` (the CUDA 13 wheel) instead, with its directory on
   `LD_LIBRARY_PATH`; the refusal must stay `missing` (or `wrong_major` if
   a `.so.12` name resolves to a 13.x file), never a load of the 13.x
   file.
7. `pull` the receipts, then `destroy` (the script deletes on any exit
   unless `--keep`).

**Cost and duration (estimate).** The 2026-10-12 T4 pilot installed
driver 580.178.04 with one reboot, and the research puts a boot plus
driver install at about 5-15 minutes (`cloud-gpu-test-options.md`); the
fork trial ran two T4 VMs for about 36 VM-minutes in total, building and
testing two forks; Candle K-G2 took 19 minutes (plan § 15 Q15). This cell pushes prebuilt artifacts and builds nothing, so
expect 30-45 minutes: about 10-15 for create, driver and reboot, 5 for
the push, 10-20 for steps 2-6. Rates (research § "Prices", us-central1,
2026-10-11): `n1-standard-4` Spot about $0.11/h plus a T4 Spot GPU about
$0.08-0.14/h plus a 100 GB disk about $0.01/h, so about $0.20-0.26/h
Spot; on-demand about $0.54/h. **Estimate: about $0.15-0.20 for a 45-minute
Spot session; at most about $0.55 at the 2 h cap, or about $1.10 on
demand.** The cost is negligible against the owner-approval step, which
is the real gate.

**If the owner declines the session**, C3 is UNMEASURED (AC31-15), and the
hermetic test and the decoy test still cover the refusal logic.

## 10. Fork-part topics this design depends on

| Topic | Owner | Used here |
| --- | --- | --- |
| D-1 API shape | cudarc design § 1 | § 8 (typed reads; names unchanged) |
| D-2 lifetime, leaked pointers, Tegra free memory and uncapped pools | cudarc design § 2 | §§ 4, 5, 6, 8 |
| D-3 stream ordering | cudarc design § 3 | none beyond Slice 30 |
| D-4 capacity, `maxSize`, Tegra cap table | cudarc design § 4 | §§ 4, 8 |
| D-5 threshold and `trim_to` | cudarc design § 5 | § 8 N1 |
| D-6 thread safety | cudarc design § 6 | none |
| D-7 observability | cudarc design § 7 | § 8 site 10 |
| D-8 CUDA 11.4-13.4 | cudarc design § 8 | § 1 |
| D-9 graph capture, PR #594 | cudarc design § 9 | § 4 (no graphs) |
| D-12 (fork part) | cudarc design § 10 | § 4 |
| D-13 multi-device | cudarc design § 11 | none (one GPU on Tegra) |
| D-F2 fork CI, FathomDB lock | cudarc design § 12.2 | § 7.2 |
| D-F3 cuBLAS workspace, PR #595 | cudarc design § 12.3 | § 5 |
| D-F4 built-major loading | cudarc design § 12.4 | § 9 |
| DK-1 base 0.10.2 | Candle requirements § 4 | § 4 (no CUDA graphs) |
| DK-2 dependency form | Candle requirements § 4 | § 3.2 (FathomDB chose registry pins, not the patch DK-2 suggested, because both forks are published) |
| DK-3 SASS | Candle requirements § 4 | § 3.7 |
| DK-4 cuBLAS workspace | Candle requirements § 4 | § 5 |

## 11. W6 change list (from §§ 1-9)

| File | Change | Topic |
| --- | --- | --- |
| `Cargo.toml` (root) | `[patch.crates-io]`: drop `cudarc` and three Candle entries; `candle-kernels` `rev` = `K9` | D-11 |
| `Cargo.lock` | smallest regeneration | D-11, D-16 |
| `src/rust/crates/fathomdb-embedder/Cargo.toml` | exact Candle registry pins; the aarch64 `cudarc` dependency table deleted; `tegra-pool = []`; comments | D-11 |
| `src/rust/crates/fathomdb-embedder/src/cuda_pool_policy.rs` | sites 3, 10; N1; `slice31_tests` module | D-17 |
| `src/rust/crates/fathomdb-embedder/src/cuda_pool_slice31_tests.rs` | new Orin tests | D-12, D-14, D-15, D-17 |
| `src/rust/crates/fathomdb-embedder/src/cuda_libraries.rs`, `device_policy.rs`, `reranker_device_policy.rs`, `candle_bge.rs`, `candle_reranker.rs` | D-18 check, report, reasons; the compute-capability floor and `CudaProvider::kernel_compute_floor`; N1 guard field | D-17, D-18, C4 |
| `src/rust/crates/fathomdb-embedder/tests/slice31_cuda_library_major.rs`, `tests/fixtures/decoy-cublas/decoy_cublas.c`, `tests/fixtures/decoy-cublas/decoy_cublas_missing_symbol.c` (revision 3, Q26), `tests/slice31_compute_floor.rs` | new tests | D-18, C4 |
| `scripts/check-cuda-required-symbols.py`, `scripts/tests/test_check_cuda_required_symbols.sh`, fixtures (revision 3, Q26) | new drift guard for the required symbol set | D-18 |
| `scripts/tests/test_slice31_retired_cudarc_references.sh` | new: retired vendored-tree and marker references; `tegra-pool = []` | D-11 |
| `scripts/test-skip-allowlist.toml`, `scripts/test-feature-matrix.toml` | entries for the new `#[ignore]` (`missing_cuda_12_cublas_is_a_typed_refusal_with_the_remedy`, `ignored-by-design`) and platform-excluded targets; the regenerated matrix | D-18, D-15 |
| `src/rust/crates/fathomdb-engine/tests/slice31_engine_close_cuda.rs` | new Orin test, if W3 and R-TEST keep it (§ 6) | D-15 |
| `src/rust/crates/fathomdb-cli`, `fathomdb-napi`, `fathomdb-py` | `doctor cuda-libraries`; report and error mapping | D-18 |
| `src/python/tests/test_tegra_pool_fork_after_init.py` | F1-F5 | D-15 |
| `scripts/pinned-override-rot.json`, `scripts/check-pinned-override-rot.py`, `scripts/tests/test_check_pinned_override_rot.sh` | schema 5 | D-11 |
| `scripts/check-single-cudarc.sh`, `scripts/tests/test_check_single_cudarc.sh`, fixtures | new guard | D-11, D-8W |
| `scripts/check-cudarc-fathomdb-lock-set.py`, `scripts/cudarc-fathomdb-lock-set.json`, `scripts/tests/test_check_cudarc_fathomdb_lock_set.sh`, fixtures | new guard | D-16 |
| `scripts/check-cuda-release-contract.py`, `scripts/tests/test_cuda_release_contract.sh` | `CUDARC_CUDA_VERSION`; the Candle block reduced to the `candle-kernels` git patch at `K9` and its single git lock entry, the rev read from `scripts/pinned-override-rot.json`; arms assert messages through `expect_fail_with` (revision 3, R-TEST OP-1) | D-8W, D-11 |
| `scripts/check-tegra-pool-registry-build.sh`, `dev/design/release.md` | T-REGISTRY | D-11 |
| `scripts/agent-test.sh` | drop `test-vendored-cudarc`; add the fast suites of the single-cudarc, lock-set, PTX-floor and required-symbol guards and the retired-reference test | D-11, D-16, D-18, C1 |
| `third_party/cudarc-0.19.7/`, `scripts/tests/test_vendored_cudarc.sh` | deleted | plan § 3 |
| `.gitattributes`, `.markdownlint-cli2.jsonc`, `scripts/agent-lint-md.sh`, `dev/tegra-platform-reference.md` | vendored-tree references removed | plan § 9 W6 |
| `dev/interfaces/rust.md`, `rust-sdk.md`, `python.md`, `typescript.md`, `cli.md` (`wire.md` checked, § 9.1 item 7); `docs/embedder.md`; `docs/reference/cli.md`; `docs/reference/errors.md` | vendored-cudarc text (rust.md, rust-sdk.md); `tegra-pool` adds no dependency; D-18 surface; the floor as `cuda_incompatible` evidence; floors (REQ31-15); D-12 note | D-11, D-12, D-18, C4 |
| `dev/adr/ADR-0.8.28-tegra-private-cuda-pool.md`; decision index row 62 | appendix A | all |
| `dev/adr/ADR-0.8.28-cuda-version-support-policy.md`; decision index row 63 | appendix B | D-18, C4 |
| `CHANGELOG.md` | registry fork crates; marker and direct `cudarc` dependency dropped; close release; `cuda_library_unavailable`; `doctor cuda-libraries`; the compute-capability floor refusal; floors | all |

## 12. Test index (new names, for W3)

| Test or check | File | Host | AC31 |
| --- | --- | --- | --- |
| `release_cuda_jobs_pin_the_cudarc_binding_version` | `scripts/tests/test_cuda_release_contract.sh` | any | 03b |
| pinned-override arms (§ 3.4) | `scripts/tests/test_check_pinned_override_rot.sh` | any | 06, 14 |
| single-cudarc arms A-D and exclusions (§ 3.5) | `scripts/tests/test_check_single_cudarc.sh` | any | 07 |
| lock-set arms (§ 7.2) | `scripts/tests/test_check_cudarc_fathomdb_lock_set.sh` | any | 14 |
| T-REGISTRY (a)-(d) | `scripts/check-tegra-pool-registry-build.sh` | any (a); Orin (b)-(c) | 08, 16 |
| compute-floor cases (§ 9.2) | `tests/slice31_compute_floor.rs` | any (hermetic) | 15 |
| `a_lib_load_error_for_a_missing_cuda_12_cublas_is_a_typed_refusal_with_the_remedy`, `missing_required_symbols_lists_each_absent_name_in_set_order`, `a_missing_symbol_refusal_names_the_symbols_and_the_remedy` | `src/cuda_libraries.rs` | `windchill3`, build only (`embed-cuda` needs `nvcc`); no GPU opened | 15 |
| required-symbol drift guard arms (§ 9.1, Q26) | `scripts/tests/test_check_cuda_required_symbols.sh` | any | 15 |
| `gpu_orin_64_engine_close_with_the_private_pool_then_exit_is_clean` (recommended) | `fathomdb-engine/tests/slice31_engine_close_cuda.rs` | Orin | 04 |
| `gpu_orin_64_records_the_usable_capacity_of_the_three_gib_pool` | `src/cuda_pool_slice31_tests.rs` | Orin | 04 |
| `gpu_orin_64_memory_outside_the_private_pool_is_recorded` | same | Orin | 06 |
| `gpu_orin_64_process_exit_with_the_private_pool_alive_is_clean`, `gpu_orin_64_dropping_the_embedder_then_exiting_is_clean` | same | Orin | 04 |
| `gpu_orin_64_dropping_the_embedder_releases_pool_memory_without_a_caller_synchronize`, `gpu_orin_64_close_release_also_trims_a_max_threshold_pool` | same | Orin | 02, 04 |
| F1-F5 | `src/python/tests/test_tegra_pool_fork_after_init.py` | Orin | 04 |
| `a_libcublas_so_12_that_reports_major_13_is_refused_before_gpu_work`, `a_libcublas_so_12_missing_a_required_symbol_is_refused_before_gpu_work` (Q26), `the_cuda_library_report_names_the_loaded_files`, the two stderr cases | `tests/slice31_cuda_library_major.rs` | `windchill3` (RTX 3090 by UUID), Orin | 15 |
| `missing_cuda_12_cublas_is_a_typed_refusal_with_the_remedy` (`#[ignore = "C3 GCP cell"]`) | same | C3 GCP T4 only | 15 |
| T-CUDA-COMPAT C3 session (§ 9.3) | receipts | GCP Spot T4, driver 580 only, owner-approved | 15 |

## 13. R-DES open points and their rulings

Revision 1 left seven points to R-DES; R-DES ruled on each
([review.md](review.md)):

1. **Fork version numbering after 0.19.10.** The fork decides; FathomDB
   needs only that each version's `FATHOMDB.md` names its upstream base.
   Fork todo FT-1, non-blocking.
2. **The 3 GiB cap's usable bytes.** Kept as designed: 1024 MiB is the
   fork's fitted formula, measured once at 3072 MiB by
   `gpu_orin_64_records_the_usable_capacity_of_the_three_gib_pool`
   (T-ORIN). If the result is below 576 MiB (twice the CB1 peak), sizing
   returns to the owner.
3. **The fork's FathomDB-lock run** should compile FathomDB's resolved
   `cudarc-fathomdb` feature set (§ 0, § 7.2 `root_features`), so the
   crates the lock-set guard records are the crates the fork compiled.
   Recommended; the fork decides (fork todo FT-3).
4. **N1 (close release)** was ruled to depend on its RED; owner ruling
   Q29 supersedes that: N1 is implemented (§ 14.1).
5. **D-18's surface:** keep the typed error, the report field and the
   doctor verb; renamed `LibraryUnavailable` / `cuda_library_unavailable`
   with a reason (R-DES finding 5); the stderr line only on refusal (Q24);
   the interface documents W6 updates are listed in § 9.1 item 7.
6. **C4** is hermetic and the floor check unconditional (Q22, § 9.2).
7. **Engine-close evidence on the Orin:** a Rust engine-level Orin close
   test is recommended to W3 (§ 6, tests).

### 13.1 Fork todos (non-blocking for Slice 31)

These are requests to the forks, recorded here and filed in
`dev/todos-and-considerations-ledger.jsonl` (with `ledgerwrite`) when W6
lands on `release/0.8.28`, so the ledger keeps one writer per branch. Each
fork decides in its own plan.

| Id | Fork | Request | Source |
| --- | --- | --- | --- |
| FT-1 | cudarc | State how `cudarc-fathomdb` numbers a code-changing release on an unchanged upstream base (crates.io will not take 0.19.10 again, and build metadata does not make a new version) | R-DES open point 1 |
| FT-2 | cudarc | Decide whether to drop item 6 (`fathomdb-private-pool`) before item 5's upstream removal now that no consumer enables it; that needs the item 6 row of `FATHOMDB.md` (removal path "with item 5") changed first | R-DES finding 10, § 3.3 |
| FT-3 | cudarc | Run 2 compiles FathomDB's resolved feature set and emits the Q23 Linux-normal lock-set record (§ 7.2) after `FATHOMDB_LOCK_SHA` moves to the pushed W6 lock commit | R-DES open point 3, finding 7, Q23 |

## 14. Revision 4: N1 and D-19 (owner rulings Q29, Q30)

Orin window P at `e9036a912` (status.md) found R14 test 1 GREEN without N1
and test 2 RED, and R16 failing with `path=default_pool`, reason
`cuinit_not_at_load`. The owner ruled (steward ledger seq 335-336): Q29,
implement N1; Q30, correct R16 and give plain Rust processes the private
pool without a caller-made init call. This section replaces § 8's "N1 is
conditional on its RED" and § 13 point 4.

### 14.1 N1, the close release (Q29)

- **Function.** `release_unused_private_pool_memory()` in
  `cuda_pool_policy.rs` (driver part, `tegra-pool` + aarch64 Linux + CUDA
  only). It does work only when all hold: the decision is `Private`; the
  dropped model's device has `alloc_mode()` `Private` (a CPU or
  `other_ordinal` model is skipped); the context is alive by `classify`'s check (primary context
  active and its id equal to the recorded id). Then it calls
  `first_context.synchronize()` and `pool.trim_to(0)`. On loss, a missing
  or fallback decision, or any driver error it does nothing; it never
  panics (body inside `catch_unwind`, errors discarded).
- **Guard.** A field declared last in `CandleBgeEmbedder` and
  `CandleTinyBertReranker`, so Rust drops it after `model` and `device`
  and the trim sees their buffers freed. Off the driver part it is a
  zero-sized no-op.
- **Close paths that reach it.** (1) Dropping a `CandleBgeEmbedder`;
  (2) `Engine::close`, which drops the released default-embedder provider
  after its guard (`runtime_lifecycle.rs:49`), and `Engine`'s `Drop`, which
  calls `close`; the napi and PyO3 `close` reach the same `Engine::close`;
  (3) dropping a directly built reranker. Not reached: the engine's
  reranker singleton (never dropped, ruling 32); a caller-supplied embedder
  still referenced elsewhere (it trims at its last drop); `close` returning
  `EngineError::Scheduler` (provider not released).
- **Threshold 0.** Window P shows the driver already returns freed memory
  without a synchronize, so N1 changes no counter; it runs anyway (one
  synchronize and an empty trim) so close does not depend on that
  unspecified driver timing.
- **Threshold `max`.** Close now returns all unused reserved memory.
  `max` therefore means "keep freed memory reserved while models are
  open; release it at each model close". Another open engine on the same
  pool keeps its live buffers (`trim_to` frees only unused memory) and
  re-reserves on demand. The synchronize waits for in-flight work on the
  device's primary context, so a close concurrent with another engine's
  embed can wait for that embed. Docs: `docs/embedder.md` (settings
  bullet), `dev/interfaces/rust.md`, `python.md`, `typescript.md`,
  changelog line.
- **Tests (Orin, `cuda_pool_slice31_tests.rs`).** RED:
  `gpu_orin_64_close_release_also_trims_a_max_threshold_pool`
  (failing at `e9036a912`, 301989888 bytes). Regression, unchanged:
  `gpu_orin_64_dropping_the_embedder_releases_pool_memory_without_a_caller_synchronize`
  (no reason to change it: its assertion is N1's contract at threshold 0).
  Skip on loss: the context-reset SIGSEGV test stays as is. Engine level:
  no counter clause (R-TEST OP-4 stands; the counters are `pub(crate)`
  test readers the engine crate cannot reach); R16 proves the engine close
  path exits cleanly on the private pool.

### 14.2 D-19, the private pool for plain Rust processes (Q30)

Today only the N-API registration hook, the PyO3 module init (Tegra
wheel) and `doctor cuda-allocator` call `run_module_load_early_init`; the
decision's `init_gate` (`cuda_pool_policy.rs:420`) refuses anything but
`Ran`, so every Rust process reports `cuinit_not_at_load` (Slice 30
§ 3.4, § 4.4).

**Option (a): load-time constructor** (`ctor` 0.2.9, already in the lock
through `napi`, or a hand-written `#[used] #[link_section =
".init_array"]` static), gated to `tegra-pool` + aarch64 Linux + CUDA.
Each placement breaks something:

- in `fathomdb-embedder`, it runs inside the Node addon and the Tegra
  wheel too: a second `cuInit` at `dlopen` with the embedder's unified
  feature flags, which Slice 30 § 3.4 forbids (each caller passes its own
  `cfg!` flags); and in the CLI, where `doctor cuda-allocator` promises no
  CUDA call before its early returns (`dev/interfaces/cli.md`);
- in `fathomdb` (facade), the CLI links it (same CLI break);
- in `fathomdb-sdk` only, engine and facade users (R16's own process)
  still miss the pool.

Wherever it sits, every linking process pays `cuInit` at start (study:
31.3 ms, +13 MiB RSS) even if it never opens an engine, and takes hazard
H-1 (Slice 30 § 2.5: a child forked after `cuInit` cannot use CUDA; F1)
before any FathomDB call. It also adds code before `main`, which Rust
does not support. Rejected.

**Option (b), recommended: first-use init at the allocator decision.**

- **Where.** `fathomdb-embedder`, at the decision's existing seam
  `PoolDriver::module_load_init()` (`CudarcDriver`,
  `cuda_pool_policy.rs:1398`). Every Rust caller (engine, facade, SDK,
  CLI) reaches it with no code of its own.
- **Rule** (a pure function `first_use_outcome(recorded, opted_out,
  init)` in `cuda_driver_init.rs`, tested on every host):
  1. a recorded load outcome wins and is returned unchanged; no second
     `cuInit` (so the N-API, PyO3 and CLI hooks, which record before any
     export or probe runs, keep their behaviour, including `Failed` and
     `SkippedCpuOnly`);
  2. none recorded and `FATHOMDB_CUDA_EARLY_INIT=off`: record `OptedOut`
     (reason `cuinit_opted_out`), so the opt-out means the same in Rust;
  3. none recorded: call `cuInit` (idempotent); on success record the new
     `ModuleLoadInit::RanAtFirstUse` (`"ran_at_first_use"`), which
     `init_gate` admits; on failure record nothing (`not_at_load`, reason
     `cuinit_not_at_load`). The write is compare-and-set under the existing
     `MODULE_LOAD_INIT` mutex, so the report, `doctor` and the decision see
     one value.
- **Tegra address space.** The constraint is on `cuInit` itself (a 4 GiB
  hole in [8 GiB, 128 GiB); study "The two failures"), and its known
  fragmenter is the V8 heap. In a Rust process the first `cuInit` is
  already the embedder probe's (`candle_bge.rs` `enumerate_visible_cuda_devices`),
  made before the first device build, so (b) adds no new `cuInit` timing
  and no new H-1 exposure: 0.8.27 Rust processes initialised at the same
  point. The one new mapping is the pool (about `maxSize`/3 = 1 GiB
  contiguous, study Phase 1); if a host has fragmented the window, pool
  creation fails and the process takes the 0.8.27 path with
  `pool_create_failed` (Slice 30 § 1 rule 2: never refuse). A Rust host
  with a fragmented window is UNMEASURED; the fallback covers it.
- **Policy envs.** The `FATHOMDB_POOL_*` settings are read as before;
  an exact `cpu` policy builds no CUDA device, so nothing runs.
- **Other builds.** Without `tegra-pool`, or off aarch64 Linux, the
  decision code is not compiled: no change (`not_built` / `None`).
- **Public surface.** `ModuleLoadInit` (new in 0.8.28, unpublished) gains
  `RanAtFirstUse` and becomes `#[non_exhaustive]`; `as_str` and the
  Python and TypeScript string sets gain `ran_at_first_use`.
  `run_module_load_early_init` stays `#[doc(hidden)]`. Doc updates:
  `dev/interfaces/rust.md` ("No load hook in Rust" becomes "First-use
  init in Rust"), `rust-sdk.md` (`cuinit_not_at_load` sentence),
  `cli.md`, `python.md` and `typescript.md` (`module_load_init` values),
  `docs/embedder.md`, the changelog (Rust and CLI engine processes on the
  Orin now get the private pool). ADR: Appendix A gains a paragraph
  amending component 4 ("Rust has no load hook") to the first-use rule,
  under ruling Q30.

**Option (c): a public, documented init API.** It adds AC-074 governed
surface and still requires the caller's call, which Q30 rules out. Not
needed with (b); not recommended.

**Tests, RED first, all GPU tests on the Orin.**

- Pure (any host): `first_use_outcome` table (a recorded value is
  returned and `init` is not called, for every variant; opted out;
  success; failure); `init_gate(RanAtFirstUse)` is `Ok`;
  `RanAtFirstUse.as_str()`; the napi, py and CLI mappings of the new
  string.
- R16 corrected (`slice31_engine_close_cuda.rs`, Q30 authorises the edit):
  the child also prints `module_load_init` and `reason`, and the parent
  asserts `ran_at_first_use` and `private_pool`; it still calls no init
  function. RED today on `path=private`.
- New `fathomdb-sdk/tests/slice31_sdk_first_use_cuda.rs`:
  `gpu_orin_64_sdk_process_reaches_the_private_pool_without_an_init_call`
  (child opens through the SDK with the default embedder on `cuda:0`;
  `path=private`, `ran_at_first_use`, embeds, closes, exit 0) and
  `gpu_orin_64_sdk_process_with_early_init_off_takes_the_0827_path`
  (`cuinit_opted_out`, not `private`).
- Unchanged and re-run: Slice 30's hook tests and window A's Node and
  Python rows (the hooks record first, so `ran` stays `ran`).

## Appendix A. ADR amendment draft

Text to append to `dev/adr/ADR-0.8.28-tegra-private-cuda-pool.md` in W6.
The front matter changes too: `blast_radius` begins "`cudarc-fathomdb`
0.19.10 (crates.io, from `coreyt/cudarc`); the Candle fork crates 0.11.0
and `candle-kernels` by git patch at `182ceab0`" in place of "vendored
cudarc 0.19.7; Fathom Candle fork 0.10.3"; `status` adds "amended
2026-10-<day> (Slice 31)". Decision index row 62 gains "amended by Slice
31: published fork crates".

> ## Amendment 2026-10-<day> — consumption through the published fork crates (0.8.28 Slice 31)
>
> **Context.** The decision above is unchanged: who gets the pool, the
> gates, sizing, settings, errors, report and early-`cuInit` contract stand.
> What changes is where the cudarc primitive and its evidence live. Slice 31
> moved the vendored cudarc delta to FathomDB's cudarc fork
> (`coreyt/cudarc`, branch `fathomdb/0.19.10`, rebased on upstream
> v0.19.10) and published it as `cudarc-fathomdb` 0.19.10 from commit
> `b777f8d9`; the Candle fork depends on it and is published as
> `candle-core-fathomdb`, `candle-nn-fathomdb` and
> `candle-transformers-fathomdb` 0.11.0 from `182ceab0` (owner rulings
> Q9, Q17, Q18, Q21).
>
> **Decision.**
>
> 1. **Consumption.** `fathomdb-embedder` depends on the Candle fork crates
>    `=0.11.0` and reaches `cudarc-fathomdb` 0.19.10 only through them
>    (`candle_core::cuda::cudarc`); it has no direct `cudarc` dependency, and
>    `tegra-pool` adds none. `Cargo.lock` holds exactly `cudarc-fathomdb`
>    0.19.10 from crates.io, checked by `scripts/check-pinned-override-rot.py`.
>    The root
>    `[patch.crates-io]` keeps only `candle-kernels` by git at `182ceab0`,
>    which carries the static CUDA runtime into FathomDB-built artifacts
>    (ADR-0.8.25-driverless-cuda-runtime-linkage), and `libsqlite3-sys`.
>    Exactly one `cudarc` 0.19 is in the graph, `cudarc-fathomdb`, enforced
>    by `scripts/check-single-cudarc.sh`.
> 2. **crates.io consumers get `tegra-pool`.** Because the fork crates are
>    published, a downstream build with `tegra-pool` and no `[patch]` table
>    resolves and builds. `scripts/check-tegra-pool-registry-build.sh`
>    proves it with a path copy of the FathomDB embedder from the tree under
>    test, depending on the published fork crates; it does not build
>    published FathomDB crates, which do not exist until the release. It
>    links the CUDA runtime dynamically through upstream `candle-kernels` 0.10.2 and needs
>    `LIBRARY_PATH` set to the CUDA library directory to link; the
>    CPU-loadable guarantee of REQ-068 applies only to FathomDB-built
>    artifacts.
> 3. **The marker feature is retired.** `tegra-pool` no longer enables
>    `fathomdb-private-pool`; the package name `cudarc-fathomdb` already
>    rules out upstream cudarc.
> 4. **Evidence for the primitive lives in the fork.** The fork's
>    requirements, design, tests, reviews and qualification receipts
>    (`fathomdb/` on `fathomdb/0.19.10`; G1-G6, G8 and the CI receipt at
>    `b777f8d9`) replace `scripts/tests/test_vendored_cudarc.sh`. The item
>    table with upstream status and removal paths is the published crate's
>    `FATHOMDB.md` (items 1-9). FathomDB guards lock drift between its
>    `Cargo.lock` and the fork's last recorded run
>    (`scripts/check-cudarc-fathomdb-lock-set.py`, owner ruling Q8), and
>    records the exact fork versions it relies on in
>    `scripts/pinned-override-rot.json`.
> 5. **Library loading.** The fork loads only CUDA libraries of the major
>    it was built for (item 9). FathomDB's check of the loaded libraries,
>    its typed refusal and `doctor cuda-libraries`, and the probe-time
>    compute-capability floor, are CUDA-version-policy decisions, recorded
>    in the amendment of 2026-10-<day> to
>    ADR-0.8.28-cuda-version-support-policy.
> 6. **Close.** Dropping a FathomDB-built private-pool model synchronizes
>    the decision's context and trims unused pool memory, unless the
>    context is lost. *(Kept only if Slice 31 design § 8 N1 lands.)*
> 7. **Tegra facts (fork measurements, JetPack 6).** A capped pool serves
>    about a third of its `maxSize`, so the 3 GiB default serves about
>    1 GiB; a process has room for about four uncapped pools; `cuMemGetInfo`
>    is not a release signal on Tegra. None changes the sizing rule.
> 8. **Why CUDA 12.x** is recorded in ADR-0.8.28-cuda-version-support-policy
>    (Slice 31 plan § 15 Q6).
>
> **Superseded statements in this ADR.**
>
> - Front matter `blast_radius`, "vendored cudarc 0.19.7; Fathom Candle
>   fork 0.10.3": superseded by `cudarc-fathomdb` 0.19.10 and the Candle
>   fork crates 0.11.0, with `candle-kernels` by git patch.
> - Components, item 1, "The vendored cudarc 0.19.7 gains `CudaMemPool`
>   (create, attribute, trim, raw, destroy on drop) …": superseded. The
>   primitive is in `cudarc-fathomdb` 0.19.10; `attribute` is replaced by
>   typed reads (`reserved_current`, `reserved_high`, `used_current`,
>   `used_high`, `release_threshold`, `max_size`).
> - Components, item 1, "The no-op feature `fathomdb-private-pool` marks
>   the vendored crate. `FATHOMDB-PATCH.md` gives each item an upstream
>   status and removal path, and `scripts/pinned-override-rot.json` lists
>   exactly that set": superseded by decisions 3 and 4 above.
> - Components, item 2, "core, nn and transformers move to 0.10.3 together
>   (candle-kernels stays 0.10.2)": superseded by 0.11.0 from the registry;
>   `candle-kernels` stays 0.10.2, by git patch at `182ceab0`.
> - Components, item 6, "It is supported only in repository and artifact
>   builds, which carry the vendored cudarc; against registry cudarc it
>   fails at resolution with the marker message
>   (`scripts/check-tegra-pool-registry-build.sh`)": superseded by decision
>   2.
> - Components, item 5, the co-resident reset: unchanged; still
>   unsupported in 0.8.28, fix planned for 0.8.29.
>
> **Consequences.** Upgrading cudarc or Candle is a new fork version,
> qualified in the fork, then an exact-version change in FathomDB under
> R-PIN. A fork item that FathomDB relies on cannot change without a
> reviewed change to the pinned-override checker. The new public surface
> of the library check is listed in the CUDA version support policy ADR's
> amendment.

## Appendix B. ADR amendment draft (CUDA version support policy)

Text to append to `dev/adr/ADR-0.8.28-cuda-version-support-policy.md` in
W6 (R-DES finding 9). Front matter: `status` adds "amended 2026-10-<day>
(Slice 31)"; `blast_radius` adds "`fathomdb-embedder` device probe; `doctor
cuda-libraries`". Decision index row 63 gains "amended by Slice 31: loaded-
library check and GPU floor".

> ## Amendment 2026-10-<day> — loaded-library check and the GPU floor (0.8.28 Slice 31)
>
> **Context.** Decisions 1 and 5 above set a CUDA 12.x library floor and a
> Turing-or-newer GPU floor; the consequences require a typed, plain
> failure below them. The cudarc fork loads only libraries of the major it
> was built for (`cudarc-fathomdb` item 9), but it accepts a file by its
> versioned name without asking its version, and Candle's device
> construction panics when a library is missing. Below the kernel floor, a
> GPU would have reached device construction and failed late. The owner
> ruled that the Quadro K620 on `windchill3` is never run (Slice 31 Q22),
> so the floor is tested with injected devices.
>
> **Decision.**
>
> 1. **Loaded-library check.** Before building a CUDA device, the embedder
>    and reranker load cuBLAS and cuRAND for the built major and refuse a
>    missing library, a file that reports another major, or a file that
>    lacks a symbol FathomDB or Candle calls (Slice 31 Q26), with the typed
>    kind `cuda_library_unavailable` and a reason (`missing`,
>    `wrong_major`, `missing_symbol`). Under `auto` the component runs on CPU with that
>    reason; under a forced `cuda:N` it is an error carrying the library,
>    path, version, required major and the names tried.
> 2. **Remedy text.** For a missing CUDA 12 cuBLAS or cuRAND the message
>    says to `pip install nvidia-cublas-cu12 nvidia-curand-cu12` and put
>    both packages' library directories on `LD_LIBRARY_PATH`, because the
>    loader does not search Python's site-packages on Linux.
> 3. **Report.** `CudaDeviceInfo.cuda_libraries` lists the files loaded;
>    `fathomdb doctor cuda-libraries` (`fathomdb.doctor.cuda-libraries.v1`)
>    always reports them; exit 70 on a refusal. Only a refusal writes one
>    stderr line per process (Slice 31 Q24).
> 4. **GPU floor at probe time.** A device whose compute capability is
>    below the build's `CUDA_COMPUTE_CAP` (the release contract value: 7.5
>    on x86_64, 8.7 for Jetson) is refused before any device is built, with
>    the existing kind `cuda_incompatible`. `auto` probes only the lowest
>    visible ordinal and falls back to CPU when it is below the floor.
> 5. **Tests.** The floor and the library-error mapping are tested
>    hermetically (injected devices and injected load errors); a real host
>    with no CUDA 12 cuBLAS is tested on an owner-approved GCP VM with the
>    driver only, or recorded UNMEASURED.
>
> **Consequences.** New public surface: the `cuda_library_unavailable`
> kind, `CudaDeviceInfo.cuda_libraries` and `doctor cuda-libraries`; the
> new error variants are a pre-1.0 break recorded in the changelog. The
> Tegra pool ADR's Slice 31 amendment (decision 5) points here.
