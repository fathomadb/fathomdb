# Slice 110 lifetime design

Status: independently reviewed design (Astra-medium PASS). Owner seq-376 selects
option1 below; seq-377 holds actual Orin/3090 resets for .30 review. Source/seam
repairs are selected for .29. The current-pin loss-policy subset is implemented
at `8247f11056bc2deb3e0c6ec0472abbd4b217e857`, independently reviewed and
integrated; all98 CPU policy tests pass. Shared native lifetime/pool/model wiring
and final qualification remain pending. The original source baseline is accepted
`a00bf9b9228318e7c236148551bb188f63ff66ee`.
No native or fault execution supports a new survival claim yet.

## Current-source implementation boundary

The current-pin subset at `8247f11056bc2deb3e0c6ec0472abbd4b217e857` is
integrated in the inspected release source `98a9787ea10786e4602ecf586b610c09c7a8bb4d`.
Five observed RED/GREEN behaviors now preserve the first published loss over
later OOM, return one winning concurrent observation with each request's current
operation, refuse private-device construction before creating/wrapping ownership,
emit unavailable pool counters without native reads, and skip known-loss
last-model queries/sync/trim while draining model counts. Independent review and
all98 CPU policy tests passed. This is product publication/refusal metadata,
not a second native generation or proof of untracked/default-device safety.
Concurrent calls already inside native diagnosis still need fork admission.
The reviewed counter successor changes only old non-null pool assertions;
identity, operation, winner, one-line/once output and historical expected-signal
teardown assertions remain. No healthy counter query or whole-model quarantine
was added.

Selected cudarc source `cb36642fad90c1796693c36ee59e3d3d162f4410` has fresh
independent Astra-medium acceptance of312 selected passing CPU controls,
formatting and production-library Clippy-Werror. This includes tracked pool
creation/attachment, `CudaContext::new_with_mem_pool_tracked`, lifetime/admission,
cached diagnostics and BLAS/RNG. The available `recorded_id()` and `loss()`
observations perform no native dispatch. Reuse this exact source/result boundary;
do not regenerate fork proof or a source census merely for product adoption.
It does not qualify GPU/NVRTC runtime, all452 listed tests, product pins or resets.
The eventual token supplies native loss authority to the existing product
publication path; no independently recoverable product generation is introduced.

Candle's `Device::new_cuda_from_context` suffices for context injection, but does
not finish the required repair. Exact consumed K9's private
`gemm_strided_batched_f32` prepares infallible device pointers and then calls raw
GEMM with `*cublas.handle()`. Pointer preparation or an earlier temporary Drop
can record driver loss inside the outer110 lease. The selected Candle repair
must declare a borrowed `CudaBlas::admit()` guard before the pointer guards and
use its checked `handle()` after pointer preparation, immediately before the
unchanged raw GEMM. Preserve all dimensions, strides, datatypes, scalars,
compute type and algorithm; do not replace it with generic Gemm dispatch.
Candle's accepted design at `434e8096dd0f30adb8fcf6094b4b27927b9a3ee0` has prepared
private-helper tests, but production and actual CUDA RED/GREEN qualification
remain pending. This necessary Candle qualification is a hard adoption edge,
even though no new public model/tensor API is required. Reconcile the product API
migration/test map against these exact interfaces now and obtain independent design review; it need not wait for Candle runtime
qualification. Actual coherent pin adoption and affected native/consumer proof
still require the selected Candle qualification.

## Existing source and primary guarantees

Product imports `cudarc-fathomdb =0.19.10` through Candle's re-export, and
`candle-{core,nn,transformers}-fathomdb =0.11.0` from the registry. Kernel patch
is K=`182ceab0547348993b6dbec4b1aed610dcc8b23f`; cudarc publication source is
C=`b777f8d94eedca8334e9331cf520d554b22544b7`. Registry VCS metadata and lock
identities agree with [.28 provenance](../../../0.8.28/features/slice-95/status.md).
This investigation byte-compared consumed `driver/safe/{core,mem_pool}.rs`,
`cublas/safe/mod.rs`, `curand/safe.rs`, Candle `cuda_backend/{device,mod}.rs`
and `tensor.rs` to exact C/K Git objects: all seven matched. This authenticates
these inspected files, not a new all-file or installed-artifact qualification.
Caches are read-only inputs; fork checkout HEAD never supplies pin authority.

CUDA12.6.3 [primary-context reference](https://docs.nvidia.com/cuda/archive/12.6.3/cuda-driver-api/group__CUDA__PRIMARY__CTX.html)
says reset destroys context resources, requires caller coordination with other
users, and permits balanced primary release afterward. It does not guarantee
stale cuBLAS/event/pool cleanup or uncoordinated-reset survival. CUDA12.6.3
[cuBLAS reference](https://docs.nvidia.com/cuda/archive/12.6.3/cublas/index.html#cublasdestroy)
describes implicit device synchronization during destruction. The observed .28
SIGSEGV is platform evidence, not a promise that every CUDA library faults the
same way. The design below is an inference requiring tests, not NVIDIA approval.

## Complete reachable ownership and cleanup

Paths below are relative to product source or exact C/K, as marked. A struct
field still drops after a custom Drop returns: returning early from CudaSlice
alone cannot suppress its event/stream/context descendants.

| Owner/creator | Retention and native actions | Required behavior |
| --- | --- | --- |
| Product engine `runtime_lifecycle.rs`, `embed_dispatch/core.rs`, projection runtime | Dispatcher/provider/projection owners hold `Arc<dyn Embedder>`; close stops admission, quiesces SQLite, drains workers and drops released provider outside close lock. Late worker/caller clones can outlive a close attempt. | Keep lifecycle order/deadline; every actual CUDA owner uses the same generation independently of engine lifetime/thread. Never hold a generation mutex while joining workers or dropping user callbacks. |
| Product `candle_bge.rs` and `candle_reranker.rs` | BERT tensors, reranker linear heads, Device, tokenizer; pool guard drops last after model/device. Model object itself is not Clone, but provider Arcs, Candle Device and Tensor storage are shared. | Acquire generation admission before load/forward; final-owner destructor discovery protects no-forward close. Do not rely on rearranging fields or last guard. |
| K `tensor.rs`, `cuda_backend/mod.rs` | Tensor Arc→storage/RwLock; CudaStorage owns typed CudaSlice and cloned CudaDevice. Views borrow storage; temporary/returned tensors can outlive a model. | Lost storage and all nested handles disarm independently; normal Rust allocations/Arcs drop normally. No whole-model leak. |
| K `cuda_backend/device.rs` | Cloned device owns context, stream, shared module stores/custom modules, `Arc<CudaBlas>`, RNG mutex and seed. `from_context` uses default stream; `new_with_stream` owns a stream. | All descendants inherit the context generation. Include RNG even when model does not call random kernels. |
| C `driver/safe/core.rs`: CudaSlice/views/SyncOnDrop | Slice read/write CudaEvents, stream Arc; Drop waits events, async-frees or synchronizes/sync-frees. Pointer guards record events/synchronize during error unwind. | No wait/record/sync/free on terminal generation; nested event/stream drops also refuse. Borrowed views/guards cannot escape their allocation lifetime. |
| C CudaEvent/CudaStream/CudaModule/CudaFunction | Event destroy, owned-stream bind/destroy, module bind/unload; functions retain module, launch arguments retain borrowed pointers/guards. Default stream is non-owning but retains context. | Each destructor takes generation cleanup admission before first native action, including binding. No stale cleanup merely because bind returned an error. |
| C `cublas/safe/mod.rs`, `curand/safe.rs` | Native create→set stream→completed wrapper; current Drop destroys then unwraps. Errors before full wrapping can leave raw handles. | Wrap immediately after create success, before fallible initialization. Healthy errors clean once, without destructor unwind; lost handles disarm without destroy. No BLAS-only patch. |
| C CudaContext | Each wrapper retains primary independently, has transient clear-on-read error_state and optional pool Arc. Drop currently binds stale context then releases. | Share one generation across wrappers; never clear terminal state with transient errors. Lost path skips bind/destroy. Balanced primary release may be the only permitted native cleanup, as below. |
| C `driver/safe/mem_pool.rs` | Pool can outlive context; raw destroy/counter/trim calls have no context owner today. Product decision holds pool and first context for process lifetime. | Attach generation before exposing pool to model allocations. Lost pool skips trim/destroy/counters; no stale diagnostics. Process statics must not be the sole safety mechanism. |
| C PinnedHostSlice | Event→context; Drop synchronizes event and calls free_host. | Include in fork cleanup rule although current BERT path is not proved to allocate it. Skip stale sync/free after loss; native pinned bytes join frozen residue. |
| Product load/probe and singleton errors | CUDA probe/device can die before model guard; mmap VarBuilder/BERT/head failure unwinds partial tensors. Reranker singleton leaks one successful model process-wide; typed pool errors currently retry load. | Protect from first native handle, not only completed model. After loss retry must refuse before allocation; existing singleton stays usable only as a typed refusal carrier. |

Other CUDA libraries/graphs/external memory wrappers not reached by these models
are not automatically in scope. Public raw-handle escapes remain their unsafe
caller's responsibility. Enabling another backend/handle family requires a new
ownership review before claiming it is covered.

## Recommended architecture: shared terminal generation

Cudarc owns the tracked-primary-context facility. The actual
`PrimaryContextLifetime::new(ordinal)` retains one primary anchor and records its
identity; `CudaContext::new_tracked(Arc<PrimaryContextLifetime>)` borrows that
anchor without another retain. Fathom keeps one strong token per used ordinal
for process lifetime, including default/synchronous decisions. Every later
wrapper receives that token; none replaces it after loss. Tracked pool support
remains a qualification prerequisite. Constructor/model-load leases remain active
through completion or unwind; pausing a thread does not release a lease or make
its partial ownership a supported real-reset boundary.
Pool attachment is mandatory before any tracked context allocates through it;
a pool already attached to a different generation is refused. Every context,
stream/event/module/library handle reaches that token; pool owns its own Arc.

`Healthy(recorded_id) → Lost(first_observation)` is irreversible. A short mutex
protects state, probe and active-lease count; first observation/payload is stable.
Do not hold it across CUDA FFI, native Drop, callback, worker join or output.
On admission when no lease is active, perform the identity/liveness probe before
incrementing the count. Nested/concurrent leases may share an already admitted
healthy epoch; callers must coordinate reset only when every lease/work item has
finished. On Lost, fail admission without driver access. A destructor lease when
no operation is active performs the same probe, discovering no-forward loss
before its first stale bind/free/destroy. A failed probe conservatively terminally
refuses cleanup; record its actual error rather than claiming a measured id.

Probe primary active state without retaining; if active query `cuCtxGetId` on
the already-owned recorded context, not a new primary retain. Inactive, changed
id, invalid/destroyed context or inability to establish liveness latches loss.
Do not create a new context during diagnosis. Fork tests must exercise the actual
probe route and classify query failures; if this stale-handle query itself is
unsafe on a selected platform, stop that support claim and revisit isolation.

Each destructor either obtains healthy cleanup admission and performs existing
cleanup once, or clears/disarms its raw handle and lets Rust fields drop. Its
nested fields repeat the same rule and see Lost. Transient error_state remains
separate. CudaBlas/CudaRng healthy destructor failures are recorded/discarded,
never unwrapped. Loss must not first be discovered after trying stale destroy.

On Lost, balanced `cuDevicePrimaryCtxRelease` without stale bind is permitted
by the cited12.6 contract, once for the shared tracked anchor's successful
retain, not once per borrowing wrapper. Keep it a distinct, testable disposition; it must not implicitly retry, retain or query. If a chosen
platform cannot execute that documented operation safely, record the discrepancy
and return the exact release-retention policy to the owner. Non-primary contexts
are outside the selected fault contract; no speculative cuCtxDestroy repair.

A cleanup lease/check does not close the race with external reset. Owner seq-376
selects the quiescent envelope below. Reset after probe but before native
cleanup/launch, or during a lease, remains unsupported. Adding an internal lock
or extra synchronize does not control an uncooperative external reset.

## Remaining Fathom integration and errors

`cuda_pool_policy.rs` retains immutable allocator choice, but stores the shared
lifetime for each constructed CUDA device. Both private and fallback-only
constructors use tracked contexts and Candle `new_cuda_from_context`; this
protects default/synchronous ownership without changing the chosen allocator.
Existing ordinal/device policy remains authoritative. Product support may still
be limited to actually qualified cells.

Acquire admission before BGE single/batch, reranker single/chunk/batch and model
load, retaining that lease through completion/unwind. Partial-load loss uses only
production-seam injection, not a live reset during this lease. Promoting it to a
real fault case requires a separately reviewed handoff retaining fully wrapped
partial owners, synchronizing all submitted GPU work, releasing all admission
leases, and closing global reset admission against every constructor, operation
and destructor. Resume must reacquire admission, see terminal loss and unwind
without unsafe native work (apart from separately permitted balanced primary
release). No such handoff is selected or implemented here.
A low-level drop discovering loss publishes the same terminal token;
subsequent Fathom calls construct existing CudaContextLost payloads from that
record, without performing another native operation. Preserve operation names,
recorded/current id semantics and stable SDK codes. Generalize typed mapping
beyond the private-only classifier only for tracked CUDA devices, not CPU.
No successful result may escape if that call establishes terminal loss.

At qualified pin adoption, migrate the old library-status tuple matches in
`candle_bge.rs` and `candle_reranker.rs`, and the three BGE test constructors,
to the actual `CublasError`/`CurandError` variants: `Native(status)`,
`Driver(DriverError)`, `Lost(PrimaryContextLoss)` and `ContextMismatch`.
Only `Native` library statuses use library compatibility classification; `Driver`
uses driver policy. The native `DriverError` tuple itself is unchanged. A cached
token loss wins before either projected status or OOM classification, including when its first native status is OOM. An ordinary
OOM without a loss record stays nonterminal. Context mismatch is an association
refusal, not proof of reset. This API migration is mechanical preparation after
the qualified interfaces exist, not a missing-API compilation RED.

Preserve `PrimaryContextLoss`'s first `recorded_id`, optional `observed_id` and
`cause` when publishing existing typed payloads. An actual
`PrimaryContextRefusal::DriverStatus` is distinct from successful-probe
`PrimaryInactive`/`IdentityChanged` projecting synthetic709, or
`NativeLibraryCleanup { library, status }`/`SyntheticUnknown` projecting
synthetic UNKNOWN. Keep the library domain/status and label synthetic projections
in existing diagnostic text; do not report them as measured driver errors.
Never call native error-name/string queries, primary retains or pool reads to
enrich a cached loss. The selected fork's driver/library formatting is cached.

### Concrete product delta against `c900a9a21`

Today `fallback_only::new_cuda_device` calls `Device::new_cuda` and its
`forward_failure` returns `None`. Only the aarch64 private driver owns the
accepted publication policy. Neither model has an outer admission lease.
The following is net-new product wiring, using the inspected fork APIs; it does
not repeat the accepted cudarc cleanup implementation.

1. Compile common tracked construction/admission whenever `embed-cuda` or
   `rerank-cuda` is enabled, independently of architecture and `tegra-pool`.
   Keep pool selection, configuration, reasons and reports architecture-gated.
   CPU/Metal construction and metadata access never enter the CUDA registry.
2. Keep a strong `Arc<PrimaryContextLifetime>` per used ordinal for process life.
   A small per-ordinal initialization slot serializes `PrimaryContextLifetime::new`
   without holding registry/slot locks across native work. Publish the token
   before any context/pool wrapper can escape. Failed anchor creation before a
   token exists permits retry; it must not permanently memoize ordinary OOM.
   Once published, including after wrapper/load failure, never replace the token.
   Constructor unwind releases its initialization slot; waiters cannot deadlock.
3. Default/synchronous construction uses `CudaContext::new_tracked(token.clone())`
   followed by `Device::new_cuda_from_context(context)`. Private construction uses
   `CudaMemPool::create_tracked(token.clone(), &props)` and
   `CudaContext::new_with_mem_pool_tracked(token.clone(), pool.clone())`.
   Obtain `recorded_id()` from the token, not a new `cuCtxGetId`/retain.
   Replace the current `with_primary_retained` decision/probe with this tracked
   ownership. Probe the four-byte allocation through the tracked context's
   stream/slice APIs, drop the temporary and synchronize while admitted; avoid
   raw pool allocation/free that bypasses terminal-status recording. Preserve
   normal create/probe-failure fallback reasons, but check token loss before
   deciding fallback or wrapping another device. Terminal loss always returns
   typed refusal, including during the first allocator decision.
4. Read the attached token through
   `device.as_cuda_device()?.cuda_stream().context().primary_lifetime()`;
   cloning its Arc makes an owned `PrimaryContextLease` possible without borrowing
   the model/device across the operation. Product constructors must never accept
   an untracked CUDA device as a silently supported substitute. This remains an
   internal construction invariant, not a new public device-injection API.
5. Use one small admission/result wrapper in `cuda_pool_policy.rs`, with an inert
   CPU/Metal case. Acquire `token.admit()` before the operation, invoke the real
   production closure, then inspect cached `token.loss()` before returning either
   success or error. If loss exists, drop any candidate successful value while
   the lease is still alive and return the existing typed loss instead. Map a
   native admission error without loss through the existing initialization/error
   policy; never manufacture terminal OOM from its numeric status alone.
6. Scope model construction inside that closure. In BGE
   `new_from_weights_on_device`, clone the token first and move the owned device
   into the admitted closure: a local lease declared inside the existing function
   would otherwise drop before its device parameter on early return. Keep mmap
   builders, BERT, reranker pooler/head and partial ownership inside the closure;
   they unwind before admission is released. Completed-model rejection also
   drops all fields inside the admitted scope. Keep `_pool_guard` last and retain
   existing provider/model ownership; do not leak or quarantine the model.
7. BGE `embed`, `embed_batch`, `embed_dual_for_test`; reranker `score`,
   `score_batch` and `score_chunk`; both `probe_cuda` implementations and the
   explicit local-asset attestation path run their CUDA work in that wrapper.
   A batch keeps its outer lease across chunks; chunks can acquire nested leases.
   Preserve the current empty-batch fast return because it submits no CUDA work.
   For a nonempty call, loss refuses before tensor creation and a later cached
   loss defeats even a host result already computed. Move transient tensors into
   the inner scope so their Drop completes before the final cached-loss check.
8. Reranker public methods retain `Result<_, candle_core::Error>`. Carry an
   existing `CudaPoolFailure` in `Error::Cuda(Box::new(failure))`, with
   wrapper-aware downcast extraction
   in `cuda_pool_error`; do not convert it to a message-only error. The engine's
   existing `CeScorer`/`score_pool` typed propagation and singleton stay intact.
   Preserve `forward`, `batch forward`, `rerank forward`, `model load`,
   `device probe` and `device build` operation labels. Extend recursive Candle
   error extraction for BLAS/RNG `Driver` and `Lost` variants; only `Native`
   BLAS statuses enter architecture/library compatibility handling.
9. Replace `context_loss`/`context_alive` diagnostic re-retains with token
   admission and cached first loss. The product publication record only caches
   rendering/first operation and is never a health authority. Loss can originate
   in descendant Drop without prior product publication. Map the token before
   private OOM classification, including first terminal driver OOM. Last-model
   decrement always runs; final healthy sync/trim shares token admission after
   model buffers drop. Lost release does no primary/pool/native diagnostic read.
10. Complete the existing nondefault `cuda-pool-observer` branch only after this
    token is adopted. Observe an already-published private decision; never create
    a token/context or synchronize/trim to obtain a sample. Healthy reads take
    the same admission and use checked pool counters. Cache UUID/pool identity
    during admitted construction. Recheck cached loss after reads: loss/error
    returns unavailable counters and its actual cause, never partial/zero-filled
    success. Keep model-count samples before/after and their existing ABA limit.
    Without a decision, token, or feature, retain the existing unavailable states.
    Update the observer's interface wording with this actual checked-read contract.

### Exact-revision local consumption

The prepared unpublished versions are `cudarc-fathomdb 0.20.0` and the three
`candle-{core,nn,transformers}-fathomdb 0.12.0`; kernels stay `0.10.2` at K9.
Cudarc's version/notices successor is
`343b04fa73f85ee78754d6ee2baa07b959cc9ccc`, with production source identical to
accepted `cb36642fad90c1796693c36ee59e3d3d162f4410`. Candle's current preparation
base `0c1be99388db9c48440fb1ae854cf883f595cd30` is not the qualified helper repair:
use the eventual independently accepted GREEN revision, never this placeholder.

After Candle qualification, update all four exact version requirements together
and supply a local Cargo configuration with `[patch.crates-io]` entries for
cudarc and all three Candle packages. Each entry uses `git = "file:///..."`
pointing to its existing local fork repository and the full qualified `rev`.
Use Git revisions, not mutable worktree `path` overrides. Resolve/lock through
that configuration in owned storage; no registry checksum exists for these Git
packages and none may be fabricated. This is a local integration route, not a
registry-ready publish claim; final release dependency publication/normalization
still needs its separately authorized route.

Candle's workspace `path` dependency on kernels would otherwise resolve kernels
from the new Candle Git source. Keep the existing root crates.io K9 patch and
add a second local-configuration patch table keyed by the exact Candle file Git
URL, for example `[patch."file:///home/coreyt/projects/fathomdb-forks/candle-fathomdb"]`
(matching the exact Git URL used for the three local Candle patches). In that table
set `candle-kernels` to the existing
`https://github.com/coreyt/candle-fathomdb.git` revision
`182ceab0547348993b6dbec4b1aed610dcc8b23f`.
A tiny file-only Cargo fixture verified that a Git-source patch redirects a
workspace path dependency to a distinct Git source; its cached
`metadata --offline --locked` passed without compilation. No fork manifest change
is needed. The owning Candle workspace qualifies its unchanged K9 kernel bytes;
its path-source lock is not product K9 source proof.

Before a product build, actual resolved metadata/lock must show one coherent
0.12.0 Candle trio, cudarc0.20.0 at the full qualified revision, and kernel0.10.2
at the original remote K9 source. Preserve feature and static-CUDART boundaries.
If Cargo cannot resolve this exact graph from existing local objects/cache, stop
at that dependency error; do not publish, fetch unrelated source, downgrade or
silently change kernel source. The first affected checks/builds require the
parent's released resource window. No native build ran during this design pass.

Projection/query/ordinary/frozen search preserve current typed-error propagation;
rerank never converts loss to0.0 or unavailable. Existing singleton retry can
remain structurally unchanged only if every retry first sees terminal admission
and makes zero native allocations. Already-leaked singleton models need no new
leak, but every score refuses. Do not introduce new indefinite background work.

First-loss snapshot stays once per process. It uses recorded ids, actual probe
result, cached library paths and optional cached pool counters; after loss native
pool counters are null/unavailable. Publish loss before emitting, outside token
mutex. Existing .28 characterization's post-loss counter assertion needs an
explicit successor alongside its positive-exit oracle, not a silent relaxation.
Healthy last-model guard still synchronizes/trims after buffers free. It takes
the same generation admission; Lost returns immediately. No unsafe native query
is required to enrich a diagnostic or count a dropped owner.

## Cleanup bound and alternatives

Recommended: reclaim Rust owners normally, abandon only the finite native handle/
allocation set existing at the first loss and require process restart before
further GPU admission on that device. No quarantine vector/list grows per failed
request. Known bytes/counts are frozen from checked allocation accounting;
no allocations admitted after loss, including failed retries/probes. Late drops
reduce Rust live ownership, never create more abandoned objects. Pool maxSize
is not an opaque-host-memory bound; Orin usable pool memory is not its cap.

Opaque cuBLAS/cuRAND host state has no measured or documented hard byte cap here.
This does not satisfy a literal universal byte bound. Owner seq-376 explicitly
accepts the frozen-live-set policy with opaque host bytes unbounded, and .30
reviews that choice. Neither test
RSS ceilings nor a count alone establishes that product guarantee. No automatic
process kill or restart is added to a library; the caller must restart before
GPU reuse. A future reset-survival claim requires normal process exit tests, including runtime unload hooks;
OS reclaim after forced kill is not teardown-survival evidence.

| Alternative | Decision rationale |
| --- | --- |
| Shared generation and native-handle disarm, recommended | Protects clones/partial ownership at actual native boundary; preserves healthy paths. Needs narrow fork-A repair and explicit finite-live-set residue acceptance. |
| Whole-model ManuallyDrop/quarantine | Reject: misses partial loads, temporary tensors, external clones and native descendants; retains tokenizer/model host memory and can grow per request. |
| BLAS-only skip or last-model trim guard | Reject: RNG, slice/event/module/pool/context hazards remain; field order cannot protect them. |
| GPU worker process isolation | Choose if arbitrary asynchronous-reset survival or a hard opaque-byte ceiling is required. New IPC/provider/lifecycle architecture and OS-enforced worker resource bounds need separate scope/design review. A killed worker is process containment, not in-process successful teardown. |
| Same-process context reconstruction | Reject for110: mixes stale handles/identities and the immutable allocator decision; no valid recovery proof. |

## Owner selection and future live-fault boundary

Owner 2026-11-14 seq-376 selects quiescent loss after completed construction:
all participants finish/synchronize GPU work; none constructs, operates or drops
CUDA ownership during reset. Partial-load loss remains production-seam-only.
After loss, reclaim Rust ownership, abandon at most the frozen native live set,
admit no further GPU allocation/use, and require process restart. Known counts/
bytes do not grow; opaque native host bytes have no hard numerical cap. .30
reviews this policy. Worker isolation/arbitrary-timing support is not selected.

Owner seq-377 holds actual Orin AGX64/CUDA12.6 private-pool and selected3090
CUDA12.6 child reset execution until .30 review. .29 implements and verifies the
source/seam repairs with healthy/OOM/lifecycle controls; there is no mandatory
.29 live-fault gate after this explicit deferral. Source/seam success cannot
qualify reset survival or close original TC-281155a1. Keep .28 unsupported-reset
claims and historical signal11 evidence until a qualified successor exists.

The future proposed cells are three completed-construction cases per selected
Orin/3090 cell: reset then typed refusals, reset→no-forward close, and last-clone
drop. Proposed limits remain120 seconds per ready child,30 minutes per venue,
8GiB GPU and16GiB owned host memory, subject to actual later venue/session
permission. These limits do not bound opaque product bytes. Missing Orin leaves
original-defect closure unqualified. No host-wide reset, driver/toolkit mutation,
K620, unowned process attachment or partial-load live reset is permitted.
See [.30 receiving allocation](../../../0.8.30/release-followups.md).
