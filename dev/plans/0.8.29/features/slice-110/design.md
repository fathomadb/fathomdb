# Slice 110 lifetime design

Status: independently reviewed design (Astra-medium PASS). Owner seq-376 selects
option1 below; seq-377 holds actual Orin/3090 resets for .30 review. Source/seam
repairs are selected for .29, not implemented or runtime-qualified. Source baseline is accepted `a00bf9b9228318e7c236148551bb188f63ff66ee`.
No native or fault execution supports a new survival claim yet.

## Current-source implementation boundary

Source fit at Fathom `f7ad475fbf3ef273faa32dca6e0b82fccdc7efcb` and committed
cudarc repair `4510667590ff729adfdb140e53d645ce00a8acbd` distinguishes two steps.
The current product can preserve a loss it has already detected, without new
dependencies: `context_loss` currently forgets its result, so a later OOM becomes
exhaustion and `build_device` can create another private-context wrapper.
Add a once-published first-loss record to `PrivateDecision`, shared by its
existing policy calls. Publish identity/error metadata before snapshot output;
map each subsequent operation to that record before OOM classification, native
diagnosis or private-context construction. This is a product refusal/publication
cache, not a replacement native-generation state machine or cleanup guarantee.
Do not hold its publication synchronization across driver calls or output.
Concurrent calls already inside native diagnosis are not made safe by this
cache; shared native admission remains the qualified fork's responsibility.

Use the same published record in last-model release to refuse synchronization
and trim after a known loss. The first loss snapshot must not query the lost
pool; retain measured identity/probe provenance and emit null for unavailable
native counters. Preserve healthy and ordinary-OOM behavior. This narrow step
does not add leases to model execution or discover every native loss.

Actual shared-generation admission remains blocked on qualified140-A source.
The inspected cudarc commit exposes `PrimaryContextLifetime::new`, `admit` and
`CudaContext::new_tracked`, but no public cached-loss/recorded-identity accessor;
its existing `new_with_mem_pool` remains untracked. Qualified cached observation,
tracked pool attachment and complete descendant cleanup are required before
product token/pool wiring or pin changes. In particular, a token that first
becomes terminal with an OOM status cannot be integrated using enum-only mapping.
The eventual token supplies native loss authority to the product publication
path; there must be no independently recoverable product generation.

Candle already exposes `Device::new_cuda_from_context`; its existing
`CudaDevice::from_context_and_stream` creates BLAS then RNG. No new model/tensor
API is needed. Enclosing construction admission and safe partial unwind still
require qualified fork behavior. Default/synchronous tracking, device probes,
BGE/reranker construction/forward leases and native teardown remain integration
work after140, not claims completed by the product publication cache.

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

Cudarc owns a narrow tracked-primary-context facility. Proposed API shape is
`PrimaryContextLifetime` plus tracked variants of `CudaContext::new` and
`new_with_mem_pool`; existing untracked fork users retain their contract. The
tracked constructor returns both context and shared lifetime; subsequent Fathom
contexts on that device receive the same lifetime. Fathom holds one strong token
per used device for process lifetime, including default/synchronous decisions;
no token can be replaced after loss. Token creation records actual context id
before exposing native descendants. Constructor/model-load leases remain active
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
by the cited12.6 contract, once per successful retain. Keep it a distinct,
testable disposition; it must not implicitly retry, retain or query. If a chosen
platform cannot execute that documented operation safely, record the discrepancy
and return the exact release-retention policy to the owner. Non-primary contexts
are outside the selected fault contract; no speculative cuCtxDestroy repair.

A cleanup lease/check does not close the race with external reset. Owner seq-376
selects the quiescent envelope below. Reset after probe but before native
cleanup/launch, or during a lease, remains unsupported. Adding an internal lock
or extra synchronize does not control an uncooperative external reset.

## Fathom integration and errors

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
