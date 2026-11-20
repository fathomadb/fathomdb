---
title: Slice 60 execution design
status: APPROVED
slice: 60
target_release: 0.8.30
---

# Slice 60 execution design

## Reconciled inputs and scope

Entry is `bfee102dd06a879875af42e19fa81b073af22134` in the task-owned
implementation checkout. The [whole plan](plan.md) remains selected under
seq409. Parent owns canonical state, integration, ledgers and cleanup. Changes
since `f7fb1da9a`:10 supplies owned noneditable setup and tooling/security fixes;
20 supplies delivery controls;30 supplies attribution/error fixes;40 supplies
reviewed latency treatment;50 supplies partial runtime/logging;55 supplies
reviewed composed integrity proof at3595192b. None changes the fork source.
Do not replay their passing gates or silently complete50/70/75 here.

Adopt prework3's reproducible dependency/native contract,4's actual owned
constructor/model boundaries,5's deep owning tests plus thin installed controls,
and9's selected fork placement. Adjust the old upstream leads to the actual
registry source below. Keep format37, F32 model precision, current public
numerical/security/support policy, healthy OOM recovery and driverless loading.
No CUDA13, cutile/ug/flash-attn, new backend/dtype, ANN, model re-export,
publication, host reset or partial-construction live reset is selected.
The [quality roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md)
applies through first/persistent native errors, known-output controls and exact
installed ownership. Defer exhaustive GPU fuzzing and architecture campaigns.

## Source and fork migration

The current cudarc0.20.0 source343b04fa is based on upstreamv0.19.10
ec1cac71. Published upstream0.19.10 is still the comparison base; preserve the
qualified lifetime/library/allocator implementation. Evaluate/remove only its
unused no-op marker, independently of every private-pool primitive and product
tegra-pool selection/accounting. Before removal update FATHOMDB item6's removal
condition; qualified package identity already rejects upstream substitution.
Removal is feature metadata cleanup; preservation tests are not fabricated RED.

Select upstream Candle0.11.1 at
`59f8e903ca80a32ea7f8514b39748fc979e69e82`, authenticated by published
candle-core0.11.1 `.cargo_vcs_info.json`. Current upstream main1a49dc27
is a comparison input, not the candidate. Preserve the unpublished fork0.12.0
trio names and direct cudarc0.20.0 dependency; migrate directly to the selected
upstream API. The consumed kernels become upstream0.11.1 plus required linkage
and selected image changes, replacing K9's0.10.2 bytes only after qualification.
A common qualified code commit may serve both trio and kernels; do not assume
their present split has already disappeared. No registry delivery is claimed.

| Delta | Selected disposition and current need |
| --- | --- |
| cudarc1–4 | Retain aarch64 default-pool fallback, per-device decision, zero-length behavior and owning tests. |
| cudarc5 | Retain CudaMemPool/tracked context ownership, counters, allocator isolation and cleanup. Upstream has no equivalent. |
| cudarc6 | Remove unused marker after independent policy/feature-resolution review; no allocator behavior changes. |
| cudarc7–9 | Retain fork CI, package identity and checked built-major loading. No upstream writes. |
| .29 lifetime | Retain shared per-device token, monotonic first cause, all owning destructor guards, checked BLAS/RNG dispatch and balanced primary release. |
| K1 | Retain renamed trimmed trio/CPU-safe defaults; keep unsupported optional upstream features disabled. |
| K2 | Retain Linux AArch64 gemm no-F16 build and typed CPU F16 matmul refusal. New upstream NEON work does not prove baseline fullfp16 safety. |
| K3 | Retain dynamic loading; upstream0.11.1 still uses dynamic-linking. |
| K4/K6 | Retain static whole-archive CUDART, CUDA_PATH search/rerun; include source/header rebuild tracking. |
| K5 | Retain public existing-context constructor and its default-stream semantics; upstream constructor remains private and its ordinary new uses per-thread stream. |
| K7/K7t | Retain renamed qualified cudarc edge and actual private-pool isolation test. Upstream removes ug, closing its old lock-only cudarc0.17.8 path for selected graphs. |
| K8/K9 | Replace old registry cohort assumptions with exact unpublished code pins; qualify complete kernel source and standalone SDK graph. |
| .29 F32 helper | Reapply borrowed BLAS admission before pointer guards and checked handle immediately before unchanged raw GEMM. Keep original dimension/stride/datatype/compute/algo behavior. |

Upstream0.11.1 also advances zip to9.1.3 with defaults disabled and tokenizer
to0.23.1/fancy-regex. Inspect actual model/safetensor/pickle/zip reachability;
keep the direct product tokenizer separately pinned unless an actual compatibility
fix requires migration. Check authored exact token IDs and existing model
numerical controls; no automatic oracle/model regeneration. Fork-local documents
own requirements, design, tests and exact native receipts; product records their
qualified subjects rather than duplicating fork suites.

## Native image candidate and prospective adoption rule

Baseline: current PTX-only11 module families (Affine, Binary, Cast, Conv, Fill,
Indexing, Quantized, Reduce, Sort, Ternary, Unary), compute75 x86/87 Tegra,
CUDA12.6. Treatment: selected upstream kernel source, all11 families packaged
as driver-loadable fatbinary images with PTX fallback. Use explicit nvcc
`--generate-code` rows: x86 native sm75 and sm86 plus compute75 PTX;
Tegra native sm87 plus compute87 PTX. No auto-detected build target.
Candle's existing module cache loads binary bytes through cudarc Ptx::from_binary;
custom user PTX remains unchanged. Native/PTX dispatch is the NVIDIA image
loader's selection; errors propagate through the existing typed driver policy.
No fallback after a real native load error or terminal loss is introduced.

Keep PTX text available for API compatibility/inspection. A build-only candidate
switch distinguishes identical-source PTX baseline and image treatment; it is
not a new product runtime switch. Native75 is inspected but unmeasured because
only3090(sm86) and Orin(sm87) are selected venues. x86 PTX75 omits BF16/F8
symbols guarded at80; native86 may contain them, but no new dtype support is
promised or used by product. MoE/static quantized archive is outside product
F32 paths: preserve its explicit current build-target-only native behavior and
no PTX fallback, inspect it, and retain unsupported-family limits. Do not infer
MoE support on a newer device from successful F32 module fallback.

Before trials, select these adoption bounds: all exact-output/typed/lifetime
controls pass; median cold first-valid-output improves at least10% on each
selected venue against the same-source PTX baseline; median steady latency
regresses at most5%; clean kernel build wall time at most3x baseline; compressed
and unpacked linked artifact bytes each at most2x baseline. Five fresh processes
per arm, interleaved ABBAABBAAB, same drivers/toolkit/model/input/features,
empty owned CUDA_CACHE_PATH for cold and one isolated prewarmed cache for warm.
Measure startup through validated output, empty-cache/warm-cache difference as
JIT-associated cost (not a perfect causal JIT timer), steady20 operations,
clean build wall time and gzip/raw artifact size. Keep all attempts; no tuning
or posterior limits. A missed/inconclusive criterion preserves PTX as product
default and records native treatment non-adoption, without discarding its result.
Compare old consumed source to selected upstream separately; do not attribute
upstream changes to native images. Existing numerical tolerances remain fixed;
small integer F32 kernel controls have exact hand-computed results.

Native control: CUDA_DISABLE_PTX_JIT=1, CUDA_FORCE_PTX_JIT absent; execute known
F32 fill/arithmetic/reduction/matmul outputs and actual BGE/rerank. PTX control:
CUDA_FORCE_PTX_JIT=1, CUDA_DISABLE_PTX_JIT absent; same inputs/output oracle,
with native images ignored. Inspect each module image's SASS targets, PTX ISA,
targets and symbols with CUDA12.6 cuobjdump; listing alone proves no execution.
Below-floor/wrong-library/missing-family cases retain current typed refusal or
documented auto fallback. Do not run K620 or claim generic ARM GPU support.

## FIX-1 qualification selections

All11 image families receive binary target/ISA/symbol inspection. Actual
native/PTX launches are selected for these seven families, on both3090 andOrin:

| Module / symbol | Authored operation and exact result |
| --- | --- |
| Fill / fill_f32 | Four values filled with1 produce [1,1,1,1]. |
| Affine / affine_f32 | [1,2,3,4]×2+1 produces [3,5,7,9]. |
| Binary / badd_f32 | [1,2,3,4]+[4,3,2,1] produces [5,5,5,5]. |
| Cast / cast_u32_f32 | Existing token-index representation [1,2,3,4] converts to exact F32 [1,2,3,4]. |
| Indexing / is_u32_f32 | Select indices [3,1] from F32 [1,2,3,4] and require [4,2]. |
| Reduce / fast_sum_f32 | Sum [1,2,3,4] and require10. |
| Unary / usqr_f32 | Square [1,2,3,4] and require [1,4,9,16]. |

Use actual Candle operations plus explicit owning module/function lookup, with
an actual result readback after each launch. Read source-selected symbol names
before writing the test; names above identify intended existing operations,
not permission to alter the oracle if production returns a wrong value.
Conv/Quantized/Sort/Ternary, native75 and MoE are inspected-only here; they
receive no new runtime support claim. Existing typed below-floor and wrong
library checks remain actual policy tests. Runtime wrong-image control loads
an owned CUDA12.6 sm90-only cubin without PTX on the selected86/87 device and
requires actual NO_BINARY_FOR_GPU before launch. Missing-family control loads
a valid Fill module, requests a deliberately absent function, and requires
NOT_FOUND without a launch. Feed actual returned errors through the real
product Candle/driver classifier and resolution seam: NO_BINARY is
Incompatible (auto CPU report; forced existing typed incompatibility), NOT_FOUND
is ProbeFailed (auto existing probe-failed report; forced typed probe failure).
Neither is context loss or permission to select another allocator/device.

The decisive RED isolates Candle Fill: first healthy GPU/context and fill_f32
lookup/launch under PTX-allowed matched baseline must produce [1,1,1,1]; a
separate PTX-disabled process fails specifically loading that Candle module
with the native-image-unavailable driver status. It must reach neither cuBLAS
nor MoE and have no missing-device/library precondition. The positive image
successor launches and reads the exact same values. The other six owning
controls then establish their actual routes, with raw driver status and module
identity retained. Process-global JIT switches alone are not attribution proof.

Model oracles remain human-set: tests/candle_bge.rs EU-4 dimension384,
unit norm error<1e-5 and authored cat/finance near-versus-far cosine gap>0.15;
candle_reranker::gpu_tests::cpu_gpu_logits_close requires per-pair abs<1e-2,
with existing batch/per-pair abs<1e-3 preservation. Use the actual source test
names after listing; require FATHOMDB_REQUIRE_LIVE=1 and no skip/fallback for
selected GPU bodies. Record existing loader HF_REVISION and per-file digests
plus reranker RERANKER_REVISION/three SHA256 constants; validate cached bytes
before trials. No model output snapshots or new numeric tolerance.

Adoption workloads are embedding AND reranking separately, each required to
meet the preselected10% cold/5% steady criteria on EACH selected venue. Embedding
uses EU-4's fixed three unit-norm inputs; reranking uses the existing
cpu_gpu_logits_close authored query/passages, unchanged order. Both arms use
release profile, same source/lock/features/model bytes and artifact consumers;
JIT force/disable variables are absent in adoption trials. Run ABBAABBAAB
separately per workload, venue and cold/warm condition, so neither workload or
cache condition can be cherry-picked. A=PTX andB=images. Cold uses a new empty
cache per process; warm uses a separately prewarmed cache per arm (never reuse
another arm's cache). Record first-valid-output including model load, then20
fixed repeated operations and their median. Clean kernel build wall time uses
new empty owned target per arm, same dependency cache and fixed job count;
record dependency/preparation time separately. Size measures corresponding
release Python installed native module and wheel, Node native module and
platform tarball, and CLI executable/archive: raw linked bytes and gzip-level6
(or existing package compression) bytes with exact same packaging options.
Every selected artifact must satisfy2x, not an average hiding an oversized one.
The historical source comparison is separate from same-source treatment.

Upstream's new enable_cuda_graph_htod_cache API and TLS owned CudaSlice cache
are opt-in and have no selected product caller (verify source search). Keep it
inactive; review any future graph-cache caller before claiming loss safety.
Kernel0.11.1 logical source, archive files and normal/features/target IDs must
match the selected exact code pin. If trio/kernels coalesce, update root and
standalone logical file patches, HTTPS replacement and lock identities together;
otherwise preserve two explicit qualified code pins. A local path build cannot
close the portable archive or independent downstream graph row.

## Exact-pin safe-drop and live protocol

First qualify actual owning cudarc nested-drop/library admission seams and
healthy controls, then Candle checked raw F32 GEMM/private pool/ARM CPU controls,
then product construction/forward/drop and installed consumers. The312 selected
.29 CPU controls and current source identity are reusable for unchanged code;
new source edges receive affected tests. Passing preservation is not RED.
F30 native loading gets a genuine semantic RED: known F32 execution with PTX JIT
disabled fails on the actual PTX-only baseline, then passes the image candidate.
Any newly found product defect requires its own failing test/commit before fix.
Do not alter the historical expected-SIGSEGV test to manufacture green.

Selected live cases are separate owned completed-construction children:
(1) BGE single/batch and reranker forwards, synchronize, reset, two typed refusals,
close/drop marker and exit0; (2) same completed model but reset then immediate
engine/model close with no forward; (3) tracked Candle tensor clone outliving
device/model, reset then all owners drop and final-clone marker/exit0. Pair each
venue with healthy matched forward/drop, actual ordinary OOM cleanup/retry and
a fresh-process model forward/drop after every fault child. Existing engine
close API is used; new module-release API belongs50 and is not a prerequisite.
One snapshot, cached cause, no subsequent allocation/unsafe native cleanup and
normal process exit are mandatory. Missing GPU, timeout, signal or absent marker
fails positive teardown. No same-process CUDA recovery.

The reset call is cuDevicePrimaryCtxReset_v2 of ordinal0 inside that child,
after all model construction has completed, all submitted work synchronized,
and all participating constructor/operation/destructor/clone holders are
quiescent under one test-owned rendezvous. No other CUDA module in that process
may use that device. NVIDIA12.6.3 documents process-primary resource reset and
requires caller coordination; it does not release retained usage. The fork's
balanced final primary release remains permitted. No device/host reset.

Use existing Orin10.83.10.13 AGX64 JetPack6/R36.5.2/CUDA12.6 and selected3090
UUID GPU-5f9cfc90-2be1-06a7-ce39-5a6d294b209b/R580.178.04/CUDA12.6.
Open existing lock inodes without create/unlink: Orin /tmp/fathomdb-gpu.lock;
3090 /mnt/data/tmp2/fathomdb-0829-continuation/slice30-gd/locks/
GPU-5f9cfc90-2be1-06a7-ce39-5a6d294b209b.lock (inode8532374).
Recheck exclusive lease, no other GPU actor and floors before execution.
Select max120s after readiness per child, max30min runtime per venue, max8GiB
actual device bytes/16GiB owned host RSS; start floors12GiB MemAvailable,
2GiB SwapFree,32GiB task disk and8GiB GPU free (Orin unified memory uses host
accounting); stop floors8GiB/1GiB/16GiB. Bound build separately; do not impose
runtime fault watchdog on compile. Sample owned PID/start-time/resource data;
on bounds failure terminate/reap only owned process group and retain failure.
Concrete reset commands/artifact hashes, actor/lease and healthy evidence must
be ready and reviewed before parent surfaces the remaining seq377 authority.
Healthy/source/CPU work continues independently while actual reset is held.

## Frozen residue and last-owner evidence

Retain seq376's selected policy: after loss native residue may remain until
process restart, but it is bounded by the finite live allocations/handles at
first loss and cannot grow on repeated refused requests. Retain known count/
byte accounting from the owning token/model observers; opaque native host bytes
have no established hard cap.16GiB RSS is a session stop, not a product bound.
Revisit this policy by recording actual observed residue and no-growth after
32 refused calls; do not infer native reclamation from close success or RSS
alone. Post-loss snapshots use only cached metadata/unavailable native counters.
Owning spies establish zero new alloc/free/trim/event/stream/module/library
native work; real children establish typed requests, no-forward first detection,
clone retention/drop, known model counts, passive RSS/device samples and exit.
Real children cannot directly prove every hidden native call or opaque byte.
No healthy pool query runs after loss to measure the abandoned pool.

The existing cuda_private_pool test proves tensor USED release/default/current
pool isolation, not final-pool destruction. Separately run cudarc
tracked_pool_cleanup_tests::cleanup_healthy_last_owner_holds_admission_through_one_native_destroy
and cleanup_known_loss_disarms_last_pool_owner_without_native_destroy; their
owning native dispatcher counts require exactly1 healthy destroy versus0 lost,
strong-count/Weak reclamation, same lifetime and balanced primary release.
Pair native destroy_with_outstanding_allocations_is_deferred (existing real
256MiB native allocation, final pool drop, independent outstanding-allocation
survival and release/RSS oracle) with a positive final-owner Candle child:
retain Weak pool/context/tensor owners, release in declared clone order, require
Weak upgrade=None only at each actual last owner, pre-drop healthy USED return,
post-final-drop marker, no tracked cleanup loss and natural exit0. Do not call
a destroyed pool handle. This joins owning dispatch proof with actual native
resource/normal-exit observations; it is not a hard opaque-native-byte bound.
Ordinary OOM uses existing cuda_ordinary_oom_tests finite3GiB private cap loop
or exactly one rejected physical-total+32MiB default request, actual typed
status2, full healthy USED release and successful4MiB retry on SAME token,
allocator and recorded generation. Native query remains under healthy admission.
Historical expected-SIGSEGV characterization remains unexecuted and retained;
new positive cases are separate. No live reset before concrete seq377 authority.

## Product adoption and verification order

Use one exact qualified cudarc, trio and kernels in normal/features/target
x86_64/aarch64 graphs for embed, rerank, combined and CPU. Record normalized
source IDs, HTTPS replacement identity, versions and features; prove a fresh
source archive/clean Cargo home resolves without local fork filesystem access.
Update root and standalone experiments/sdk_drivers/rust patches together.
Root patches do not propagate to independent downstream workspaces. Archive
and downstream graph tests must reject second active cudarc, wrong pin/mirror,
wrong CUDA feature and accidental kernel substitution. Git sources have no
registry checksum; no publication follows from successful local package tests.

Build owned Python/Node/CLI CUDA artifacts and fresh installed consumers. Each
runs positive F32 embed/rerank, device attribution, authored persisted write/read,
close and separate-process reopen/exit. Explicit CPU/auto driverless consumers
load CUDA-capable bytes without NVIDIA libraries/devices; forced CUDA refuses
with the existing typed error. ARM CPU preserves F32 and F16 refusal. Retain
exact source/lock/features/toolchain/artifact/native resolution identities.
Use owned >=3.11 venv with exact tools (select3.12 for final native candidate),
Rust1.95 and read-only Node26.13.1/npm11.20.0 executables; public support waits75.
No editable install, borrowed native module or shared target/venv.

Astra-medium reviews this coherent plan/design and fork-local amendments with
distinct verdicts; at most one consolidated FIX-1 before re-review. Commit only
after PASS. Direct Sol6.2-high reviews final code and direct Terra independently
runs affected actual native/artifact tests. Scoped lint/type/tests while working;
prepare derived files and clean commit before one strict full source gate with
ptrace, no arbitrary outer40min timeout and no writes during gate. Narrow
corrections rerun failed/affected/unrun scopes. Preserve every original failure,
skip, adverse measurement and unsatisfied mandatory row. Final status hands exact
safe-drop pins to50, graph/source proof to70, installed/native applicability to75.

## Primary source basis

- [Published Candle0.11.1](https://docs.rs/crate/candle-core/0.11.1/source/.cargo_vcs_info.json) and [selected source](https://github.com/huggingface/candle/tree/59f8e903ca80a32ea7f8514b39748fc979e69e82).
- [NVIDIA12.6.3 primary context reset](https://docs.nvidia.com/cuda/archive/12.6.3/cuda-driver-api/group__CUDA__PRIMARY__CTX.html) requires caller coordination and retains usage counts.
- [NVIDIA12.6.3 compilation/JIT](https://docs.nvidia.com/cuda/archive/12.6.3/cuda-c-programming-guide/index.html#application-compatibility) and its JIT environment controls distinguish native/PTX.
- [Cargo patch scope](https://doc.rust-lang.org/cargo/reference/overriding-dependencies.html#the-patch-section) requires downstream workspace patches.

Direct Astra-medium returned PLAN PASS and DESIGN PASS after one consolidated
FIX-1, including the cudarc and both Candle owning amendments. This is source
planning acceptance; native/cost/safe-drop/installed rows remain unexecuted.
