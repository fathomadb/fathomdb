# Slice 140 current-source census and provenance

Observed2026-11-10; immutable product source
`02e30d861beddc8061af0dc22f12106d5e09dd32` at
`/mnt/data/worktrees/fathomdb-0.8.29`, initially clean. The canonical release
selector returns0.8.29, its board and JSON. JSON140 is NOT_STARTED, depends_on9,
with concrete required110/120 design/fork-first condition. This Phase A does not
edit that state. Actual10 is closed;20 is actively separate and was not read.
130's review-log is terminal Astra design PASS after FIX1; its older status prose
retains dated pending text. No130 campaign or source qualification follows.

## Existing files versus new proposals

Before this pass, tracked slice140 contained **plan.md only**. This proposal
replaces that plan and adds requirements.md, acceptance.md, design.md,
test-map.md, source-census.md and status.md; supplemental machine receipts are
source-census.json and phase-a-validation.json. No local140 implementation,
fork repair test or qualification receipt exists. Existing110/120 tracked
directories each have plan.md only; richer findings were read from their
explicit `/tmp/fathomdb-0829-direct/slice-110/phase-a/` and sibling120 trees,
without mutation or promotion into accepted source.

## Exact consumed source

| Input | Identity and current fact | Bound |
| --- | --- | --- |
| C cudarc registry crate | cudarc-fathomdb0.19.10; C=`b777f8d94eedca8334e9331cf520d554b22544b7`; upstream v0.19.10 base=`ec1cac71dab29498240704a9e4a264fbbec19495`. | Actual lock/registry pins, not live clone HEAD. FATHOMDB items1/2/3/5/9 relied on; marker6 remains. |
| K Candle registry crates | core/nn/transformers-fathomdb0.11.0; K=`182ceab0547348993b6dbec4b1aed610dcc8b23f`; upstream0.10.2 base=`7c7a8c570e6a16ea24cc30a7501c8ffbbdb51680`. | Fork0.11.0 is not upstream0.11.0. Product F32 BERT paths do not select BF16/F8 or MoE expansion. |
| Kernel root patch | candle-kernels0.10.2 at exact K; build.rs emits PTX plus libmoe.a and whole-archive cudart_static. Locked builder cudaforge0.1.5. | Root patch not inherited by registry consumers; native MoE/static registration is part of build census even if model does not call MoE. Soname10 is not toolkit-major evidence. |
| Product | Exact fork dependencies in embedder manifest; both CUDA features imply tegra-pool, whose pool policy is aarch64/Linux only. Current reset parent intentionally expects SIGSEGV; product library Display always prescribes CUDA12 remedy. | No shared terminal-generation lifetime state or positive reset teardown proof established by current tests. Remedy gap belongs120. |

110's authenticated receipt compared101 C,63 core,24 nn and204 transformers
source/build/original-manifest files (392 total), zero mismatches, and matched
four cached archive SHA256 values to its candidate lock/VCS identities.120
additionally compared25 K kernel files and authenticated locked cudaforge0.1.5
archive/10 source files. This pass reuses those immutable receipts and compares
their relevant product inputs to actual02e30d: no diff for Cargo.lock/Cargo.toml,
embedder tree, TS loader, artifact contract or pin registry. The supplementary
[receipt](source-census.json) records current hashes and comparison results.
No downloads, builds or native execution were duplicated. Source identity is
not installed-runtime qualification.

## Exact-object hazards inspected

At C, `src/driver/safe/core.rs` CudaSlice waits/free/sync; its event fields later
drop. CudaEvent binds/destroys, owned CudaStream binds/destroys, SyncOnDrop
records/synchronizes, CudaModule binds/unloads, and check_err clears ordinary
error_state. `src/cublas/safe/mod.rs` and `src/curand/safe.rs` create handles,
set streams before complete ownership and unwrap destruction. At K,
`candle-core/src/cuda_backend/device.rs` owns stream/modules/BLAS/RNG; Tensor/
Storage clones and partial construction extend cleanup beyond a top model.
110's ownership census additionally covers context/pool, policy retained-primary
helpers, dispatcher/MCP admission and singletons. These are candidate hazards,
not dynamic counts or an approved suppression/quarantine policy.

110's pinned CUDA12.6 primary-context evidence permits balanced release after
reset; it does not authorize stale binding/destruction or arbitrary reset races.
No new external fact verification was necessary: this pass uses exact source
and already dated primary evidence. Compiler6111901 exposure remains UNKNOWN;
120 must disposition exact PTX/JIT/native paths before13 delivery.

## Read-only fork discovery and authority

Real local clones are `/home/coreyt/projects/fathomdb-forks/cudarc` and
`/home/coreyt/projects/fathomdb-forks/candle-fathomdb`. No tracked/root AGENTS.md
was found in either. Exact C has `fathomdb/{plan,requirements,design,test-plan,
review,status}.md` plus qualification records. Exact K has
`fathomdb/{plan,requirements,review,status}.md` and qualification records;
its requirements file includes the detailed design/test plan. No nonexistent
K design.md/test-plan.md is assumed. Read full identities from Git objects;
working checkout HEAD never supplies published-pin authority.

Memory read: actual canonical MEMORY.md and real entries
forks-coreyt-cudarc-and-candle-fathomdb.md,
cudarc-fork-qualification-and-tegra-pool-facts.md,
windchill3-never-change-driver-or-primary-cuda.md,
tdd-red-must-fail-per-finding.md, full-gate-run-frequency.md,
release-dod-requires-full-workspace-gate.md,
verify-design-against-code-not-just-architecture.md,
conformance-rewrite-vacuous-green-trap.md, slice-prompt-verify-test-claims.md
and acceptance-md-locked-no-feature-acs.md. Canonical current ACs override the
old memory's historical highest-number statement. Relevant actual requirements,
acceptance, ADR index/pool/policy/driverless/witness contracts, master/JSON/board,
direct execution, quality roadmap, CUDA review and .30 receiving plan were read.
