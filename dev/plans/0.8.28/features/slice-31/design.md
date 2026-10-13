---
title: FathomDB 0.8.28 Slice 31 — design (W2)
status: DRAFT FOR R-DES; revision 1 (2026-10-13)
target_release: 0.8.28
observed_on: 2026-10-13
---

# Slice 31 design

This is W2 of [plan.md](plan.md) (revision 17): the FathomDB design topics
of plan § 6, each with its decision, evidence, consequences for W6 and the
tests that prove it. It answers every point that
[requirements.md](requirements.md) (REVISED AFTER R-REQ) leaves to R-DES,
and carries the ADR amendment draft (appendix A). Owner ruling Q19 gives
it its own Opus review, R-DES, before W3 and W6.

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
| In that scratch tree the features resolved on `cudarc-fathomdb` (aarch64, `embed-cuda,rerank-cuda,tegra-pool`) are `cublas`, `cublaslt`, `cuda-version-from-build-system`, `curand`, `driver`, `dynamic-loading`, `f16`, `f8`, `fathomdb-private-pool`, `nvrtc`, `std`; no concrete `cuda-NNNNN` feature | `cargo tree -e features -i cudarc-fathomdb`, measured 2026-10-13 |

Names used below: `C`, `K`, `K8`, `K9` as in requirements § 1. Paths
without a repository are FathomDB paths at `bb4ee7095`; embedder paths are
under `src/rust/crates/fathomdb-embedder/`.

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
  "12060"`; no FathomDB script checks it (`git grep CUDARC_CUDA_VERSION --
  scripts` finds only `scripts/gcp-lab/remote-fork-cell.sh:86` and the
  vendored test script, which W6 deletes).
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
  the line from one job, RED first.
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

**Decision.** FathomDB consumes the fork crates under their published
names, renamed back to the upstream dependency keys in the embedder
manifest:
`cudarc = { package = "cudarc-fathomdb", … }`,
`candle-core = { package = "candle-core-fathomdb", … }` and the same for
`candle-nn` and `candle-transformers`. Source code keeps writing
`cudarc::…` and `candle_core::…`, because both forks keep their library
names (`[lib] name = "cudarc"`; Candle K1). The fork version scheme is the
fork's: `cudarc-fathomdb` uses the upstream base version (0.19.10), and the
Candle crates use the fork's own numbers (0.11.0, Q21).

**Evidence.** `git show b777f8d:Cargo.toml` lines 2-3, 28-29; Candle
`FATHOMDB.md` K1 and K9 rows; cudarc `FATHOMDB.md` "How to depend on it".

**Consequences.** None beyond § 3.2. How the cudarc fork numbers a
code-changing release on the same upstream base is open (§ 13, point 1).

### 3.2 Dependency form: exact pins

**Decision.** The embedder pins every fork crate exactly:

```toml
candle-core         = { package = "candle-core-fathomdb", version = "=0.11.0", optional = true }
candle-nn           = { package = "candle-nn-fathomdb", version = "=0.11.0", optional = true }
candle-transformers = { package = "candle-transformers-fathomdb", version = "=0.11.0", optional = true, default-features = false }

[target.'cfg(all(target_os = "linux", target_arch = "aarch64"))'.dependencies]
cudarc = { package = "cudarc-fathomdb", version = "=0.19.10", optional = true, default-features = false, features = [
    "std", "driver", "dynamic-loading", "cuda-version-from-build-system",
] }
```

Exact (`=0.19.10`), not caret:

- a later `cudarc-fathomdb` 0.19.x can then enter a FathomDB build only
  through a reviewed manifest change, which brings R-PIN, the D-16
  re-qualification cells and the lock-set guard (§ 7);
- Candle 0.11.0 requires `cudarc-fathomdb` `^0.19.10` (caret), which
  unifies with `=0.19.10`, so the graph keeps one node (§ 0);
- the embedder already pins Candle (`=0.10.3` today, lines 35-37) and
  `ort` (`=2.0.0-rc.10`, line 57) exactly, so this is the existing house
  rule, not a new one.

The cost: a downstream crate that also needs another `cudarc-fathomdb`
0.19.x cannot build with `fathomdb-embedder`. No such consumer exists,
and the crate exists only for FathomDB.

Without `tegra-pool`, the embedder's own `cudarc` edge is off, and
`cudarc-fathomdb`'s version comes from Candle's `=0.11.0` pin, whose
manifest names `0.19.10` (caret). A downstream consumer could therefore
resolve a later `cudarc-fathomdb` 0.19.x for the non-pool build. That is
the consumer's lock, outside FathomDB's guards; FathomDB's own lock and
artifacts are covered by the single-cudarc and lock-set guards. Recorded,
not changed.

**Evidence.** Embedder `Cargo.toml` lines 35-37, 57, 66-72; Candle
`git show 182ceab0:Cargo.toml` line 43; the scratch resolution (§ 0).

**Consequences (W6).** `src/rust/crates/fathomdb-embedder/Cargo.toml`
lines 35-37 and 67; the comment at lines 27-34 says Candle 0.11.0 from the
fork, and the comment at lines 59-65 is rewritten (no vendored cudarc, no
marker claim, AC31-06). Root `Cargo.toml` `[patch.crates-io]` drops the
`cudarc` path entry and the three Candle git entries (lines 93-94, 96-97)
and moves `candle-kernels` (line 95) to `rev =
"182ceab0547348993b6dbec4b1aed610dcc8b23f"`. `Cargo.lock` is regenerated
with the smallest update (`cargo update -p` per changed package, or the
manifest edit followed by any `cargo` command), never a full
`cargo update`, so no unrelated version moves into the D-16 set.

**Tests.** R-PIN checklist (plan § 9 W6 step 2); the pinned-override arms
(§ 3.4); the single-cudarc guard (§ 3.5); AC31-16's lock condition.

### 3.3 The marker feature

**Decision.** FathomDB stops enabling `fathomdb-private-pool`:
`tegra-pool = ["dep:cudarc"]`. No FathomDB file describes the marker as a
guard.

- With `package = "cudarc-fathomdb"`, the dependency can resolve only to
  `cudarc-fathomdb`. Upstream `cudarc` cannot be substituted by a registry
  resolution, so the marker no longer guards anything a consumer can do
  by accident (requirements AC31-06, R-REQ open point 6).
- The one remaining substitution is a deliberate `[patch.crates-io]
  cudarc-fathomdb = …` in someone's own workspace. That substitute is
  either the fork (which has the pool) or a deliberate act.
- Dropping the use makes cudarc fork item 6 removable: its removal path is
  "with item 5" (`FATHOMDB.md` line 40), and with no consumer enabling it,
  the fork may drop it at its next version. The published 0.19.10 keeps
  it, which is harmless (a no-op feature).
- Candle never enabled it (CK-REQ-01: "the package name already fails
  closed against an unpatched cudarc").

**Evidence.** Embedder `Cargo.toml:148`; `git grep fathomdb-private-pool`
outside `third_party/` and `dev/plans/` finds the embedder manifest (lines
61, 148), `dev/adr/ADR-0.8.28-tegra-private-cuda-pool.md:109`,
`dev/design/release.md:138`, `dev/interfaces/rust.md:1243`,
`scripts/check-pinned-override-rot.py:102`,
`scripts/check-tegra-pool-registry-build.sh:11,14,56` and
`scripts/pinned-override-rot.json:57,65` (2026-10-13).

**Consequences (W6).** The embedder manifest line 148; every file listed
above is changed or deleted with its vendored-tree text (plan § 9 W6
"References to update"); the ADR amendment supersedes the marker sentence
(appendix A). A ledger todo asks the cudarc fork to drop item 6 at its next
version (owner's call; not this slice).

**Tests.** `git grep -n fathomdb-private-pool -- . ':!dev/plans' ':!dev/adr'
':!CHANGELOG.md'` is empty after W6 (a T-PRODUCT assertion that W3 adds
beside AC31-10's `git grep`); T-REGISTRY (b) builds `tegra-pool` from the
registry (§ 3.6).

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
         "dependency": "cudarc",
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
   (checksum `513008c7…e222`) entries of the same shape. The checker owns
   the same values as constants (the file's existing pattern: metadata must
   equal checker-owned constants, lines 188-261) and verifies, offline:
   - the manifest declares `dependency` with exactly `package` and
     `version = "=<version>"` (an exact requirement; a caret fails);
   - `Cargo.lock` has exactly one package of that name, with that version,
     source `registry+https://github.com/rust-lang/crates.io-index` and
     that checksum;
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
- `fork_registry_pin_with_a_git_source_in_the_lock_is_rejected`;
- `fork_registry_pin_with_a_different_checksum_is_rejected`;
- `fork_registry_pin_with_a_second_version_in_the_lock_is_rejected`;
- `missing_fork_registry_pin_is_rejected`;
- `candle_kernels_patch_at_another_rev_is_rejected`;
- `a_cudarc_patch_entry_is_unsupported` (a reintroduced `cudarc` path
  patch fails as an unsupported override);
- GREEN: the real tree.

### 3.5 The single-cudarc guard

**Decision.** `scripts/check-single-cudarc.sh` (bash entry point; the logic
in an embedded `python3 -I` block using `tomllib`, with the `tomli`
fallback AGENTS.md § 3 names). Fast tier: `run_tier_suite fast
test-check-single-cudarc bash scripts/tests/test_check_single_cudarc.sh`
in `scripts/agent-test.sh`, which runs the guard on the real tree and on
fixtures. Inputs: `--root <dir>` (default the repository root), so the
tests can point it at a fixture. It fails on any of:

- **A. Lock, upstream cudarc 0.19.** `Cargo.lock` has a package named
  `cudarc` whose version starts `0.19.`. (`cudarc` 0.17.8 from `ug-cuda`
  passes: the `ug` chain is excluded by version, as Candle CK-AC-01 does.)
- **B. Lock, more than one fork source.** `Cargo.lock` has no
  `cudarc-fathomdb` package, more than one, or one whose source is not
  `registry+https://github.com/rust-lang/crates.io-index`.
- **C. Tree.** For each target triple in {`x86_64-unknown-linux-gnu`,
  `aarch64-unknown-linux-gnu`} and each feature set in {`embed-cuda`,
  `rerank-cuda`, `tegra-pool`, `embed-cuda,tegra-pool`,
  `rerank-cuda,tegra-pool`}:
  `cargo tree --locked -e normal --target <triple> -p fathomdb-embedder
  --features <set> --prefix none` has a line starting `cudarc v0.19`, or a
  `cudarc-fathomdb` line other than exactly `cudarc-fathomdb v0.19.10` (a
  git or path source prints a parenthesised source and fails), or, for a
  set with `embed-cuda` or `rerank-cuda`, no `cudarc-fathomdb` line at all
  (non-vacuity). `flash-attn` and `ug` are never enabled by the guard,
  which is how their chains are excluded (CK-AC-01); FathomDB does not
  enable either, and `flash-attn` is unsupported (R-FK-PUB finding 5).
  The expected version comes from `scripts/pinned-override-rot.json`
  `cargo_fork_registry_pins`, not a second literal.
- **D. Concrete version feature (D-8W).** For each target triple, with
  `--features embed-cuda,rerank-cuda,tegra-pool`, `cargo tree --locked -e
  features -i cudarc-fathomdb --target <triple> --prefix none` lists a
  feature matching `cuda-[0-9]{5}`.

`--locked` keeps the guard from rewriting the lock. The tree conditions
need the registry index cache; on a host without it, the first run fetches
from crates.io (a package registry, permitted). A `cargo` failure is a
guard failure, never a skip.

**Fixtures** (`scripts/tests/fixtures/single-cudarc/`), each a minimal
workspace whose crates are path crates only, so `cargo tree --offline
--locked` runs with no network:

| Fixture | Contents | Expected |
| --- | --- | --- |
| `ok/` | a `fathomdb-embedder` crate with features `embed-cuda`, `rerank-cuda`, `tegra-pool`; its lock names one `cudarc-fathomdb` 0.19.10 registry package (a lock-only entry the tree does not need) and a path crate standing in for the tree node | pass (the GREEN control for A, B, D) |
| `lock-upstream-cudarc/` | `ok/` plus a lock entry `cudarc` 0.19.4 (registry) | A fails |
| `lock-ug-cudarc/` | `ok/` plus `cudarc` 0.17.8 | pass (the `ug` exclusion) |
| `lock-two-sources/` | `ok/` plus a second `cudarc-fathomdb` 0.19.10 with a `git+https://github.com/coreyt/cudarc.git?rev=…` source | B fails |
| `lock-second-version/` | `ok/` plus `cudarc-fathomdb` 0.19.11 | B fails |
| `tree-upstream-cudarc/` | a path crate named `cudarc` version 0.19.4 that the fixture `fathomdb-embedder` depends on under `embed-cuda` | C fails |
| `tree-flash-attn-only/` | the same `cudarc` path crate reachable only through a fixture feature `flash-attn` | pass (the `flash-attn` exclusion) |
| `tree-concrete-feature/` | the fixture's cudarc stand-in has a `cuda-12060` feature that `embed-cuda` enables | D fails |

Because the tree conditions in fixtures read path crates, condition C's
version check in fixtures reads the fixture's own pin record
(`scripts/tests/fixtures/single-cudarc/<fixture>/pinned-override-rot.json`).
`test-plan.md` (W3) fixes the fixture files byte for byte.

**Evidence.** § 0 (one node per target and feature set; no `cudarc` 0.19
in the lock; resolved features). Candle CK-AC-01 (`fathomdb/requirements.md`
lines 86-103 at `e9c6ed5e`) uses the same tree-plus-exclusion form. The
`ug` chain: `Cargo.lock` `ug-cuda` 0.5.0 → `cudarc` 0.17.8 (lock only, plan
§ 2).

**Consequences (W6).** New `scripts/check-single-cudarc.sh`,
`scripts/tests/test_check_single_cudarc.sh`, the fixtures, one
`run_tier_suite fast` line in `scripts/agent-test.sh` where the
`test-vendored-cudarc` line (lines 323-326) is removed.

**Tests.** AC31-07: one RED test per failure condition A-D on its fixture,
the two exclusion fixtures passing, and the guard GREEN on the real tree
(T-PRODUCT, "single-cudarc guard red then green").

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
reranker load and run F32 only today; a guard test keeps it so.

**Evidence.** `candle-kernels/src/fill.cu:70-87` in the Candle fork
(plan § 6 D-11); the T4 trial (`ones_gpu` fails pre-Ampere, plan § 15 Q13);
todo `TC-30847709`. The x86 compute capability: `cuda-artifact-contract.sh`
(`CUDA_COMPUTE_CAP` per target).

**Consequences (W6).** A pure test in a new embedder test file; no product
change. Multi-architecture kernels (Candle DK-3, todo `TC-323f15b0`) stay
the way to add BF16/F8 later.

**Tests.** `cuda_models_load_and_run_in_f32_only` (new file
`src/rust/crates/fathomdb-embedder/tests/slice31_cuda_dtype.rs`, no GPU):
asserts the `DType` constants that `candle_bge.rs` and
`candle_reranker.rs` pass to the model loaders and tensors are `F32`
(W3 names the exact symbols it reads). It is a regression guard, so it is
GREEN at once; R-TEST decides whether a mutation (a BF16 constant) must be
shown RED.

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

**Decision.**

- **The set.** The resolved dependency set of `cudarc-fathomdb` is the
  transitive closure, in `Cargo.lock`, of the `dependencies` lists starting
  at the single `cudarc-fathomdb` package (a bare name resolves to the one
  package of that name; `name version` to that version). Each member is
  recorded as `name`, `version`, `source` and `checksum` (empty for a
  package without one). The closure is taken from the lock, not from one
  platform or feature set, so it is a superset of what any build compiles.
  On the emulated W6 lock it has 27 packages (§ 0 scratch tree), led by
  `float8` 0.7.0, `half` 2.7.1 and `libloading` 0.9.0.
- **Canonical text and digest.** One line per package, the root first,
  then the members sorted by `(name, version)`:
  `<name> <version> <source> <checksum>\n`. `set_sha256` is the SHA-256 of
  that text. Both the fork and FathomDB compute it with this definition.
- **The fork record (written in the fork).** After a hosted-test run with
  a FathomDB lock, the fork commits (class `D`)
  `fathomdb/qualification/<date>-lockset-<FathomDB commit, 12 hex>.json`:

  ```json
  {
    "schema": "fathomdb.cudarc-fathomdb-lock-set.v1",
    "fathomdb_lock_commit": "<40 hex>",
    "fathomdb_lock_sha256": "<sha256 of that Cargo.lock>",
    "ci_run_id": 0,
    "ci_head_sha": "<the fork commit the job ran at>",
    "ci_conclusion": "success",
    "root": {"name": "cudarc-fathomdb", "version": "0.19.10", "source": "registry+https://github.com/rust-lang/crates.io-index", "checksum": "8780d2fe…"},
    "dependencies": [{"name": "float8", "version": "0.7.0", "source": "registry+…", "checksum": "…"}],
    "set_sha256": "<64 hex>"
  }
  ```

  The workflow's Run 2 computes the set from the fetched lock and writes
  the JSON as a job artifact; the receipt commit copies it. This is a
  fork-CI (`W`) change plus records (`D`), so it does not move `C`
  (cudarc fork plan § 8: "fork CI workflow (`W`) only | fork CI").
- **FathomDB's copy.** `scripts/cudarc-fathomdb-lock-set.json` holds the
  same object plus `fork_record`: `{"repository":
  "https://github.com/coreyt/cudarc", "path": "<the record path>",
  "commit": "<the fork commit that added it>"}`.
- **The guard.** `scripts/check-cudarc-fathomdb-lock-set.py` (stdlib,
  `tomllib` or `tomli`), run in the fast tier through
  `scripts/tests/test_check_cudarc_fathomdb_lock_set.sh`
  (`run_tier_suite fast test-check-cudarc-fathomdb-lock-set …`). It:
  1. recomputes the set from `Cargo.lock` and fails if the lock has zero
     or several `cudarc-fathomdb` packages;
  2. checks the record's own `set_sha256` against its `root` and
     `dependencies` (a hand-edited record fails);
  3. fails when the recomputed digest differs from the record's, printing
     the added, removed and changed packages and the instruction: "the
     cudarc fork must run its hosted test job with this Cargo.lock
     (FATHOMDB_LOCK_SHA = the commit that carries it) and record the set
     (cudarc fork plan § 8, last row); then copy the record to
     scripts/cudarc-fathomdb-lock-set.json".
  It is offline and does not read the fork. T-FORK-EVIDENCE (R-VERIFY)
  checks that the fork record at `fork_record.commit` equals FathomDB's
  copy and that its `ci_run_id` is a successful run at `ci_head_sha`.

**The W6 sequence.** RED: the guard and its fixture tests, with no record
(fails: "no record"). Then the W6 manifest and lock commit is pushed on
the slice branch; the fork sets `FATHOMDB_LOCK_SHA` to it, runs, and
records the set; FathomDB copies the record; GREEN. If the fork run fails,
that is a fork finding (the pin chain restarts there).

**Evidence.** Fork workflow lines 32-35 and 231-300 (Run 2 already fetches
FathomDB's lock by commit, prints its SHA-256 and asserts every resolved
dependency of cudarc equals FathomDB's lock); fork CI receipt
`2026-10-12-CI-b777f8d.md` (both runs passed, 55 and 56 tests; the set is
not recorded); `git show 4c327cd8:Cargo.lock` (SHA-256
`5b9e680660c861f44c0476576adbfb686eb3cb3854a3fc928f0f03ed5d2fac52`,
vendored `cudarc` 0.19.7); requirements AC31-14 (D-16 bullet); Q8.

**Consequences.** FathomDB (W6): the guard, its test, the record copy, its
fixtures, one tier line. The cudarc fork (fork work, its own plan and
review): the Run 2 set emission, the `FATHOMDB_LOCK_SHA` update, the
record and receipt. The fork's Run 2 compiles only `std, driver,
dynamic-loading, cuda-12060` (workflow line 208), not the `cublas`,
`curand`, `nvrtc`, `f16` and `f8` code FathomDB's build enables (§ 0), so
`half` and `float8` are in the set without being compiled there; § 13
point 3 asks the fork to decide.

**Tests** (`test_check_cudarc_fathomdb_lock_set.sh`, fixtures under
`scripts/tests/fixtures/cudarc-lock-set/`):

- `matching_lock_passes` (`matching/`);
- `changed_libloading_without_a_record_fails` (`changed-libloading/`: the
  record of `matching/`, a lock with `libloading` 0.9.1), the RED named by
  AC31-14;
- `added_dependency_fails`, `removed_dependency_fails`;
- `two_cudarc_fathomdb_packages_fail`, `no_cudarc_fathomdb_package_fails`;
- `record_with_a_wrong_set_sha256_fails`;
- `unrelated_lock_change_passes` (a package outside the closure changes);
- GREEN: the real tree with the copied fork record.

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
| N2 | new, `candle_bge.rs:257-283` and `candle_reranker.rs:118-…` `probe_cuda` | D-18 check before `new_cuda_device` | **added** (§ 9) |

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

N1 is conditional on its RED. If the RED test passes on the base (the
driver releases without a synchronize), N1 is dropped and the result is
recorded in `status.md`.

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

## 9. D-18 Loaded-library check in production

**Decision.**

1. **Check.** A new embedder module `cuda_libraries.rs` (pure part on every
   host; driver part under `embed-cuda` or `rerank-cuda`) exposes
   `require_built_major_libraries()`. Both `probe_cuda` implementations
   call it before `new_cuda_device` (`candle_bge.rs:257-262`,
   `candle_reranker.rs:118-121`). It:
   - calls `cudarc::cublas::sys::try_culib()` and
     `cudarc::curand::sys::try_culib()` (the libraries Candle's device
     construction loads: one `CudaBlas` and one `CudaRng` per device), so a
     missing or wrong-major library is a typed error before any GPU work,
     instead of the `culib()` panic inside Candle's device construction;
   - then requires, for cuBLAS, `LoadedLibrary.version.major ==
     LoadedLibrary.cuda_major`. The fork version-checks cuBLAS only when it
     is opened by its unversioned name; a file named `libcublas.so.12` that
     reports another major is accepted by the fork (cudarc `FATHOMDB.md`
     item 9, "Unversioned names are version-checked") and refused here;
   - maps a failure to a new `CudaProbeError::LibraryMajorMismatch(
     CudaLibraryRefusal)` with `library`, `path` (`Option<String>`),
     `version` (`Option<String>`), `required_major` (`u32`) and `tried`
     (each name tried, with the fork's reason). Its message names the file,
     its version and the major needed, and for a missing CUDA 12 cuBLAS the
     remedy (`pip install nvidia-cublas-cu12`, or the 13.0 artifact once it
     ships), which is T-CUDA-COMPAT C3's expected message.
   - The library is never used: the probe fails, so no device is built.
2. **Resolution.** A new `DeviceResolutionReason::CudaLibraryMismatch`
   (`"cuda_library_mismatch"`) and the reranker's
   `RerankerDeviceResolutionReason::CudaLibraryMismatch`. Under `auto` the
   component runs on CPU with that reason; under forced `cuda:N`,
   `DeviceResolutionError::CudaLibraryMismatch { ordinal, refusal }`, whose
   kind is `cuda_library_mismatch`, so the refusal's fields reach the
   caller (today's `ForcedCudaUnavailable` keeps only a reason). A
   private-pool decision is never reached, so rule 3 (fail closed) is not
   involved.
3. **Report.** `CudaDeviceInfo` (already `#[non_exhaustive]`) gains
   `cuda_libraries: Vec<CudaLibraryReport>`; `CudaLibraryReport { library:
   String, opened_as: String, path: Option<String>, version:
   Option<String>, built_major: u32 }`, from the fork's `loaded_library()`
   for the driver (`cudarc::driver::sys::loaded_library()`), cuBLAS and
   cuRAND, and for cuBLASLt and NVRTC only when cudarc loaded them. The
   napi and Python `CudaDeviceInfo` mirrors gain the field.
4. **Doctor.** A new sibling verb `fathomdb doctor cuda-libraries`
   (`fathomdb.doctor.cuda-libraries.v1`), following Slice 30's choice to
   leave `doctor gpu` v1 unchanged: `built_major`, one entry per library
   (`library`, `opened_as`, `path`, `version`, `status`: `loaded`,
   `refused` or `not_loaded`), `refusal` (the fields above, or `null`),
   and `maps` (the CUDA libraries in `/proc/self/maps`, by the existing
   `cuda_libraries_in_maps`). Exit 70 when a refusal occurs, else 0.
5. **Logging.** One line per process at the first CUDA probe, under a
   `Once`, to stderr: `fathomdb-cuda-libraries {json}` with the report or
   the refusal (the Slice 30 `fathomdb-cuda-context-lost` precedent). No
   new egress.
6. **Public surface** (AGENTS.md § 1): `dev/interfaces/rust.md`,
   `python.md`, `typescript.md`, `cli.md`, `docs/reference/errors.md` and
   the CLI reference change in the same commits; the ADR amendment records
   the new kind.
7. **Below the GPU floor (T-CUDA-COMPAT C4, REQ31-15).** If C4 shows that
   an explicit `cuda:N` on a GPU below the build's kernel target (the K620,
   compute capability 5.0) does not already fail typed at the probe, the
   probe refuses a device whose compute capability is below (7, 5) on
   x86_64 or (8, 7) on aarch64 with the existing `CudaProbeError::Incompatible`
   (`cuda_incompatible`), which `enumerate` and `probe` already map
   (`device_policy.rs:570-571, 597-598`). No new error kind. `auto` already
   picks the lowest visible ordinal (`device_policy.rs:464-466`).

**Evidence.**

- Fork API: `git show b777f8d:src/cublas/sys/mod.rs` lines 8821-8850
  (`try_culib`, `is_culib_present`, `culib`, `is_symbol_present`,
  `loaded_library`); `src/lib.rs` line 237 (`LibLoadError`, `LibVersion`,
  `LoadedLibrary`, `RejectReason` re-exported); `src/driver/sys/mod.rs`
  line 16896 (`loaded_library`); cudarc `fathomdb/design.md` § 12.4 and
  `FATHOMDB.md` item 9.
- Candle at `K9`: `cudarc-fathomdb` features include `cublas`, `cublaslt`,
  `curand`, `nvrtc` (`Cargo.toml` lines 43-54); `device.rs:275`
  (`CudaRng::new` at device construction); `device.rs:185-189` (NVRTC only
  in a custom-kernel compile path FathomDB does not call; C2's
  `/proc/self/maps` cross-check confirms).
- Today's probe: `candle_bge.rs:257-283`, `candle_reranker.rs:118-…`;
  `CudaProbeError` (`device_policy.rs:182-192`) and `DeviceResolutionError`
  (`:372-381`) are not `#[non_exhaustive]`, so the new variants are public
  breaks, recorded in the changelog (pre-1.0; no shim, AGENTS.md § 10).
- C3's expected message: plan § 7 T-CUDA-COMPAT C3.

**Consequences (W6).** `fathomdb-embedder`: `cuda_libraries.rs`,
`device_policy.rs`, `reranker_device_policy.rs`, `candle_bge.rs`,
`candle_reranker.rs`; `fathomdb-cli` (`doctor cuda-libraries`); napi and
py `CudaDeviceInfo` and error mapping; the docs and interface files above.

**Tests (RED first).**

- `a_libcublas_so_12_that_reports_major_13_is_refused_before_gpu_work`
  (new `tests/slice31_cuda_library_major.rs`, `embed-cuda`, Linux): a child
  process with `LD_LIBRARY_PATH` naming a directory with a decoy
  `libcublas.so.12`, built at test time by `cc` from the fixture
  `tests/fixtures/decoy-cublas/decoy_cublas.c` (it exports
  `cublasGetProperty`, reporting major 13, and nothing else). Expected:
  `LibraryMajorMismatch` naming the decoy's path, version 13 and required
  major 12. RED today: the fork accepts the versioned name and the probe
  proceeds. No GPU needed for the refusal.
- `missing_cuda_12_cublas_is_a_typed_refusal_with_the_remedy` (same file):
  `LD_LIBRARY_PATH` empty of cuBLAS on a host whose loader cache has none
  (skips on `windchill3`, whose `/usr/local/cuda` cuBLAS is in the cache;
  runs in the C3 container).
- `the_cuda_library_report_names_the_loaded_files` (RTX 3090 and Orin):
  every `path` in the report appears in `/proc/self/maps`.
- pure tests of the mapping, the reason strings and the doctor JSON
  (`cuda_libraries.rs` unit tests; CLI tests beside the Slice 30
  `doctor cuda-allocator` tests).
- T-CUDA-COMPAT C2 receipts name the cuBLAS file from this report; C3 and
  C5 as plan § 7 says.

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
| `src/rust/crates/fathomdb-embedder/Cargo.toml` | exact registry pins; `tegra-pool = ["dep:cudarc"]`; comments | D-11 |
| `src/rust/crates/fathomdb-embedder/src/cuda_pool_policy.rs` | sites 3, 10; N1; `slice31_tests` module | D-17 |
| `src/rust/crates/fathomdb-embedder/src/cuda_pool_slice31_tests.rs` | new Orin tests | D-12, D-14, D-15, D-17 |
| `src/rust/crates/fathomdb-embedder/src/cuda_libraries.rs`, `device_policy.rs`, `reranker_device_policy.rs`, `candle_bge.rs`, `candle_reranker.rs` | D-18 check, report, reasons, N1 guard field | D-17, D-18 |
| `src/rust/crates/fathomdb-embedder/tests/slice31_cuda_library_major.rs`, `tests/fixtures/decoy-cublas/decoy_cublas.c`, `tests/slice31_cuda_dtype.rs` | new tests | D-18, D-11 |
| `src/rust/crates/fathomdb-cli`, `fathomdb-napi`, `fathomdb-py` | `doctor cuda-libraries`; report and error mapping | D-18 |
| `src/python/tests/test_tegra_pool_fork_after_init.py` | F1-F5 | D-15 |
| `scripts/pinned-override-rot.json`, `scripts/check-pinned-override-rot.py`, `scripts/tests/test_check_pinned_override_rot.sh` | schema 5 | D-11 |
| `scripts/check-single-cudarc.sh`, `scripts/tests/test_check_single_cudarc.sh`, fixtures | new guard | D-11, D-8W |
| `scripts/check-cudarc-fathomdb-lock-set.py`, `scripts/cudarc-fathomdb-lock-set.json`, `scripts/tests/test_check_cudarc_fathomdb_lock_set.sh`, fixtures | new guard | D-16 |
| `scripts/check-cuda-release-contract.py`, `scripts/tests/test_cuda_release_contract.sh` | `CUDARC_CUDA_VERSION` | D-8W |
| `scripts/check-tegra-pool-registry-build.sh`, `dev/design/release.md` | T-REGISTRY | D-11 |
| `scripts/agent-test.sh` | drop `test-vendored-cudarc`; add the two guard suites | D-11, D-16 |
| `third_party/cudarc-0.19.7/`, `scripts/tests/test_vendored_cudarc.sh` | deleted | plan § 3 |
| `.gitattributes`, `.markdownlint-cli2.jsonc`, `scripts/agent-lint-md.sh`, `dev/tegra-platform-reference.md` | vendored-tree references removed | plan § 9 W6 |
| `dev/interfaces/rust.md`, `rust-sdk.md`, `python.md`, `typescript.md`, `cli.md`; `docs/embedder.md`; `docs/reference/errors.md`; CLI reference | vendored-cudarc text (rust.md, rust-sdk.md); D-18 surface; floors (REQ31-15); D-12 note | D-11, D-12, D-18 |
| `dev/adr/ADR-0.8.28-tegra-private-cuda-pool.md`; decision index row 62 | appendix A | all |
| `CHANGELOG.md` | registry fork crates; marker dropped; close release; `cuda_library_mismatch`; `doctor cuda-libraries`; floors | all |

## 12. Test index (new names, for W3)

| Test or check | File | Host | AC31 |
| --- | --- | --- | --- |
| `release_cuda_jobs_pin_the_cudarc_binding_version` | `scripts/tests/test_cuda_release_contract.sh` | any | 03b |
| pinned-override arms (§ 3.4) | `scripts/tests/test_check_pinned_override_rot.sh` | any | 06, 14 |
| single-cudarc arms A-D and exclusions (§ 3.5) | `scripts/tests/test_check_single_cudarc.sh` | any | 07 |
| lock-set arms (§ 7.2) | `scripts/tests/test_check_cudarc_fathomdb_lock_set.sh` | any | 14 |
| T-REGISTRY (a)-(d) | `scripts/check-tegra-pool-registry-build.sh` | any (a); Orin (b)-(c) | 08, 16 |
| `cuda_models_load_and_run_in_f32_only` | `tests/slice31_cuda_dtype.rs` | any | 06 |
| `gpu_orin_64_records_the_usable_capacity_of_the_three_gib_pool` | `src/cuda_pool_slice31_tests.rs` | Orin | 04 |
| `gpu_orin_64_memory_outside_the_private_pool_is_recorded` | same | Orin | 06 |
| `gpu_orin_64_process_exit_with_the_private_pool_alive_is_clean`, `gpu_orin_64_dropping_the_embedder_then_exiting_is_clean` | same | Orin | 04 |
| `gpu_orin_64_dropping_the_embedder_releases_pool_memory_without_a_caller_synchronize`, `gpu_orin_64_close_release_also_trims_a_max_threshold_pool` | same | Orin | 02, 04 |
| F1-F5 | `src/python/tests/test_tegra_pool_fork_after_init.py` | Orin | 04 |
| `a_libcublas_so_12_that_reports_major_13_is_refused_before_gpu_work`, `missing_cuda_12_cublas_is_a_typed_refusal_with_the_remedy`, `the_cuda_library_report_names_the_loaded_files` | `tests/slice31_cuda_library_major.rs` | `windchill3`, Orin, C3 container | 15 |

## 13. Open points for R-DES

1. **Fork version numbering after 0.19.10.** If `cudarc-fathomdb` needs a
   code change on the same upstream base, crates.io will not accept
   0.19.10 again, and `0.19.10+build` metadata does not make a new version.
   The fork plan does not say which number it takes (no rule in its
   `plan.md`, `requirements.md` or `FATHOMDB.md`, `grep` 2026-10-13).
   FathomDB needs only that each version's `FATHOMDB.md` names its base;
   the fork decides. Settled by a fork plan revision.
2. **The 3 GiB cap's usable bytes.** 1024 MiB is the fork's fitted
   formula, not a measurement at 3072 MiB. Settled by
   `gpu_orin_64_records_the_usable_capacity_of_the_three_gib_pool` in
   T-ORIN; if the result is far below 1024 MiB (for example under 576 MiB,
   twice the CB1 peak), sizing returns to the owner.
3. **The fork's FathomDB-lock run compiles a narrower feature set** than
   FathomDB builds (§ 7.2): `half` and `float8` changes would be recorded
   by the lock-set guard but not compiled in the fork's run. Whether Run 2
   should use FathomDB's resolved feature set is the fork's decision
   (fork plan § 8); settled by a fork plan or design revision.
4. **N1 (close release) depends on its RED.** Whether reserved bytes stay
   after drop without a synchronize on the Orin is not measured; Slice 30
   measured CB2 only after a synchronize. Settled by the RED run (§ 8).
5. **D-18's surface shape** (a new error variant, a new reason, a new
   doctor verb, an always-on stderr line) is a design choice with public
   breaks. Evidence cannot settle it; R-DES (and the owner, if R-DES asks)
   confirms or narrows it. The alternative is to keep
   `ForcedCudaUnavailable { reason: CudaIncompatible }` and put the details
   only in the log line and the doctor verb.
6. **C4 behaviour on the K620 today** is uncharacterized; § 9 item 7 is
   conditional on it. Settled by T-CUDA-COMPAT C4.
7. **Engine-close evidence on the Orin** relies on the Node/V8 end-to-end
   run of T-ORIN (§ 6). Whether `fathomdb-engine` should carry its own
   Orin close test is left to W3 and R-TEST.

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
> 1. **Consumption.** `fathomdb-embedder` depends on
>    `cudarc = { package = "cudarc-fathomdb", version = "=0.19.10" }` (aarch64
>    Linux, `tegra-pool`) and on the Candle fork crates `=0.11.0`. The root
>    `[patch.crates-io]` keeps only `candle-kernels` by git at `182ceab0`,
>    which carries the static CUDA runtime into FathomDB-built artifacts
>    (ADR-0.8.25-driverless-cuda-runtime-linkage), and `libsqlite3-sys`.
>    Exactly one `cudarc` 0.19 is in the graph, `cudarc-fathomdb`, enforced
>    by `scripts/check-single-cudarc.sh`.
> 2. **crates.io consumers get `tegra-pool`.** Because the fork crates are
>    published, a downstream build of the published FathomDB crates with
>    `tegra-pool` resolves and builds
>    (`scripts/check-tegra-pool-registry-build.sh`). It links the CUDA
>    runtime dynamically through upstream `candle-kernels` 0.10.2 and needs
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
>    it was built for (item 9). FathomDB checks the loaded cuBLAS and cuRAND
>    before any GPU work and refuses a wrong major with the typed kind
>    `cuda_library_mismatch`; `doctor cuda-libraries` reports the loaded
>    files (ADR-0.8.28-cuda-version-support-policy).
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
> reviewed change to the pinned-override checker. New public surface: the
> `cuda_library_mismatch` kind, `CudaDeviceInfo.cuda_libraries` and `doctor
> cuda-libraries` (Slice 31 design § 9).
