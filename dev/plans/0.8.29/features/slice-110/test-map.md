# Slice 110 test-first implementation map

The current-pin subset below has accepted observed RED/GREEN and CPU proof.
Remaining proposed test names are specifications, not execution claims.
Keep current .28 crash evidence historical. Add positive successor tests; do not
change the existing expected-signal test merely to make a code change green.
After the selected contract is proved, retire that active characterization with
an explicit source/ADR successor in the reviewed implementation.

## Active .29 and deferred live tests

Owner seq-376 selects quiescent completed-construction/frozen-live-set residue
until restart, with opaque native host bytes unbounded. Seq-377 holds actual
Orin/3090 reset execution for [.30 review](../../../0.8.30/release-followups.md).
F1–F8 and F11's partial-load source seam remain .29 work with applicable healthy/
OOM/installed lifecycle controls. F9/F10 and reset-dependent F11 consumer/fresh-
process cells below are future held tests, not mandatory .29 gates or observed
REDs. Their specification is retained for future proof; no real reset survival
or original-defect closure is claimed. Do not retire the crash characterization
until actual qualified successor evidence exists.

## RED/GREEN order and actual oracles

The five current-pin publication/classification/build/snapshot/release oracles
are implemented and independently accepted; all98 CPU policy tests passed.
Their assertions remain preservation controls for affected integration changes.
The reviewed snapshot successor changes only native pool counters to unavailable;
it retains identity, operation, winner, one-line/once and historical signal11
assertions. These tests do not cover a first terminal token OOM, tracked default
admission, model leases or native descendants.

Remaining product TDD starts only with the qualified cudarc pool/library and
Candle checked-F32 artifacts available through an approved consumption route:

1. Mechanically migrate old Cublas tuple matches and three BGE test constructors
   to qualified `Native`/`Driver`/`Lost`/`ContextMismatch` interfaces. Preserve
   Native compatibility handling and Driver status identity. Compilation against
   an absent proposed API is never behavioral RED.
2. Exercise the actual product mapping with cached first causes: native driver
   OOM is terminal only when the token has a loss record; ordinary OOM remains
   retryable. Inactive/changed-id successful probes must retain synthetic709
   provenance, and library cleanup must retain its domain/status despite UNKNOWN
   projection. Concurrent callers/snapshot must share the same winning cause;
   repeat formatting/admission makes zero native diagnostic calls.
3. Exercise tracked default/private constructors, probes, BGE single/batch/dual
   pooling, reranker single/chunk/batch, failed-load retry and cached singleton.
   Before allocation require shared per-device admission and exact existing typed
   errors; terminal state refuses new initialization until process restart.
   Different-device and CPU/driverless controls remain healthy and isolated.
4. Exercise partial-load/forward unwind and last-model release through production
   seams with the outer lease still held. Require no post-loss native cleanup,
   normal Rust ownership release and healthy/OOM preservation. No live partial
   reset or whole-model quarantine is introduced. A successful result cannot
   escape after the call establishes terminal loss.

Candle separately proves its actual private F32 helper refuses before raw BLAS
and preserves numerical dispatch with `CudaBlas::admit()` declared before pointer
guards and checked `handle()` after preparation. Its current prepared tests are
not observed RED/GREEN or artifact qualification. Product tests do not substitute
for that necessary fork test, and Fathom does not rerun cudarc's owning suites.

Fork tests live in each fork's `fathomdb/` plan and native Rust modules. Product
tests live in existing pool-policy/model/engine/SDK suites. A semantic-preserving
refactor may expose an existing cleanup call through a small per-module test seam;
it must precede RED and preserve current behavior. The assertion exercises the
production branch and its native-call dispatch, not a second lifecycle model.
Do not create a generic fake-CUDA/operator/audit framework. No database mocks.

Each row gets its own actual failing RED commit, followed by the smallest GREEN.
Record actual output/exit/failing assertion in the RED commit body. Compilation
failure from a proposed API, intentional production break, prior failure from
another test and passing-on-arrival coverage do not count as that row's RED.
If a row is already satisfied after earlier work, identify it as preservation;
do not invent a defect or claim a new RED. Preserve tests during fix-to-spec.

| Order / behavior | RED oracle against actual behavior | GREEN scope / acceptance |
| --- | --- | --- |
| F1 terminal error | `destroyed_context_error_stays_terminal_after_check_err`: record a context-destroyed driver failure, check twice, require both refusals. Current clear-on-read state lets the second check succeed. Separate OOM control requires nonterminal behavior. | C: separate irreversible generation loss from ordinary error_state. A1/A4. |
| F2 shared wrappers | `loss_on_one_primary_wrapper_refuses_its_sibling`: separately constructed wrappers sharing one retained lifetime anchor, report loss on one, require sibling admission refusal before native call. Drop/reopen a wrapper and require same refusal while product token lives. | C: shared lifetime and tracked constructor, no replacement. Different-device control stays healthy. A2/A4. |
| F3 discover before cleanup | `last_owner_drop_discovers_reset_without_forward`: production probe reports inactive/changed id on final drop, require zero stale bind/destroy calls and normal Rust owner release; unknown query conservatively refuses without replacement retain. Native private-pool child is separate F9. | C: cleanup admission probes before first native cleanup; no diagnostic re-retain. A2/A3. |
| F4a BLAS cleanup | `lost_blas_drop_never_calls_destroy`, plus healthy destroy-error nonpanic: actual Drop dispatch counts0 versus1 as appropriate. Baseline invokes destroy/unwraps. | C: BLAS raw-handle disarm and nonunwinding healthy error disposition. A2/A3. |
| F4b RNG cleanup | `lost_rng_drop_never_calls_destroy`: independently proves RNG branch, not only device drop stopped at BLAS. Healthy destroy remains once/nonpanicking. | C: RNG counterpart. A2/A3. |
| F4c slices and nested fields | `lost_slice_drop_skips_free_and_event_cleanup`: live read/write events plus owned stream; assert zero wait/free/event-destroy/stream-destroy, including final nested field destruction. Repeat async and synchronous allocation paths. | C: slice and nested owner cleanup rules; normal counters/control still release. A2/A5. |
| F4d temporary guards/modules | `lost_pointer_guard_never_records_or_waits`, `lost_module_never_unloads`, `lost_pinned_host_never_syncs_or_frees`: fail/repair each independent uncovered native cleanup branch separately. | C: production SyncOnDrop, module and pinned-host dispatches. A2. |
| F4e pool and context | `lost_pool_never_trims_destroys_or_queries`, `lost_context_releases_once_without_bind`: attach same generation, drop final pool/context in both orders and with clones. Require balanced documented primary release only; no cuCtxDestroy/native diagnostic read. | C: pool lifetime attachment/disarm and context special release rule. A2/A5. |
| F5 partial handle creation | `blas_set_stream_failure_cleans_created_handle_once`, corresponding RNG create/set-stream/set-seed stages: force actual fallible stage, require exactly-once healthy cleanup, no unwrap panic. Then production-seam loss before unwind requires no stale destroy; no real reset during the constructor lease. | C: wrap raw ownership immediately after native create. Add one genuine RED per presently faulty constructor stage. A2/A3. |
| F6 product admission | `lost_generation_refuses_build_before_allocation`, `lost_embed_and_batch_are_typed`, `lost_rerank_is_never_neutral`: before native work require exact typed payload/codes and zero allocation calls. Retry twice after failed load, and cached successful singleton score. | Product policy/BGE/rerank admission and mapping, one behavior at a time. A1/A4. |
| F7 close diagnostics | `loss_snapshot_does_not_query_lost_pool`, `concurrent_loss_emits_one_snapshot`, `last_model_loss_skips_sync_and_trim`: actual once-gate and native-call spy, not expected-text-only tests. Cached/current id provenance is checked. | Product snapshot, last guard integration; never hide cleanup faults under catch_unwind. A1/A2/A5. |
| F8 state/property cases | `generation_loss_is_monotonic_across_clone_drop_sequences`: proptest bounded create/clone/admit/drop/transient-OOM/detect sequences against a small human-specified state machine. Require no post-loss native work, at-most-once cleanup, no live-set growth; reconcile wrappers released. | Owning fork recovery-layer properties. New behavior defects found get their own RED/GREEN; broad passing property coverage is not retroactive RED. A2/A4/A5. |
| F9 original defect — HELD .30 | `gpu_orin_private_reset_refuses_and_exits_normally`: real cached CUDA forward, coordinated sync/reset, two typed failures, one snapshot, explicit torn-down marker and exit0. Existing implementation fails with signal11; no missing-GPU skip counts as RED. | Qualified C→necessary K→product pins; real Orin child closes the original unsafe seam only after all above protections. A1–A3/A6. |
| F10 no-forward/clone lifetime — HELD .30 | `gpu_reset_immediate_close_exits_normally` and `gpu_reset_last_tensor_clone_drop_exits_normally`: reset with live objects then no forward; close/drop thread order varied, final clone deliberately outlives model/engine. Explicit completion markers/exit0 required. | Actual destructor discovery/whole-graph proof; independently expose paths earlier case cannot reach. A2/A3. |
| F11 consumers HELD .30 / partial unwind active .29 | `installed_reset_lifecycle_{rust,python,node}`: completed native object load before coordinated reset, explicit readiness/reset/drop completion, exit0 and real DB reopen where engine attached. Separately, `partial_load_loss_unwinds_without_native_work` exercises production cleanup through a loss seam. | Product/native lifecycle parity; partial-load loss is seam-only and cannot count as a real reset case. A1–A3/A6. |

## Fault/control matrix and deadlines

Fault children are opt-in and require actual venue/session permission. Parent
uses a real bounded subprocess wait/reap, not the historical unbounded `.output()`.
Timeout, signal, absent post-drop marker or killed child fails positive teardown;
parent containment is cleanup of a failed test, not a pass. Readiness and selected
GPU identity must precede injection; retain the actual status and minimal output.

| Cell | Case and observable result | Claim limit |
| --- | --- | --- |
| Orin AGX64/CUDA12.6 private pool |1: reset then typed single/batch/rerank refusals;2: reset→no-forward close;3: last-clone drop after completed construction. Post-drop/close markers and normal exit0. | Future original-defect proof, HELD .30; not a .29 gate. Cached weights and current source-built native consumers. Partial-load real reset excluded; missing venue blocks closure. |
| Selected3090/CUDA12.6 default pool | Same three cases only if explicitly permitted, selected UUID; default allocator identity checked. | Separate control; never substitutes for Orin private pool. |
| Synchronous allocation | Native cleanup branch seams plus real cell only where this allocator can actually be selected without altering product policy. | No fake allocator report or forced product fallback to manufacture coverage. Unmeasured stays unclaimed. |
| Healthy matching build/device | Actual load/forward/close/drop, exact live allocation/pool USED release and last-model sync/trim; clones delay release until last owner. | Preserve accepted healthy allocator/release behavior, including static first context. Do not use Tegra cuMemGetInfo alone as release proof. |
| Ordinary pool OOM | Reach actual cap with bounded allocation, exact exhaustion type, free healthy temporary/live buffers, retry after pressure removal succeeds. | Must not mark generation lost, disable healthy cleanup or mutate allocator choice. No unlimited OOM sweep. |
| Concurrent/queued after reset | Release waiting calls after coordinated reset, every GPU request fails typed, one process snapshot, repeated/concurrent close honors existing drain/lock semantics. | Reset never overlaps an active lease or native destructor; no arbitrary asynchronous-safety claim. |
| Fresh process | Parent reaps lost child, new child builds same device/model and performs real forward then normal close/exit. | Process-restart recovery only; no inference of old-generation reconstruction. |
| CPU/driverless/forced device | Existing REQ-068/AC-082 preservation tests with changed features/pins, actual CPU nonuse and forced typed failure. | No CUDA initialization through lifetime registry on CPU paths. |

Future proposed caps, not granted execution, are120 seconds per ready child,30 minutes per
venue,8GiB GPU and16GiB owned host memory; no new public deadline or product
opaque-memory bound follows. Install/build only in a released heavy window with
checkout-owned environments. SDK process-level residue must not be inferred
from a Rust-only child or fork counter seam.

Partial-load real reset remains outside the supported matrix. A future separately
reviewed test must retain fully wrapped partial owners, synchronize all submitted
GPU work, release all admission leases, and close global reset admission against
other constructors, operations and destructors. A paused thread alone is invalid.
Resume must reacquire admission, observe terminal loss and unwind without unsafe
native work, apart from the separately permitted balanced primary release. The
current constructor lease spans completion/unwind, so no such live-reset handoff
is asserted by F5/F11.

## Checks, review and exit

Run the specific changed fork/product tests and applicable lint/typecheck first;
then required final clean source gate once, never concurrent with tree edits.
Fathom consumes fork proof rather than rerunning fork suites. A fresh independent
Astra implementation/results review covers raw child statuses, actual cleanup,
error parity, healthy/OOM preservation and any missing support cells. Canonical
Markdown/view checks run on parent import; no build/full gate for this draft.
No supplementary audit packets, receipt inventories or status appendices.

For the selected .29 source exit, A1 joins F1/F6/F7; A2 joins F2–F5/F8 and
source cleanup seams; A3 joins F3/F4/F5 and F11 partial unwind; A4 joins F1/F2/
F6/F8; A5 joins F4/F7/F8 plus healthy/OOM controls and the explicit seq-376
residue policy; A6 joins qualified consumed pins and applicable actual healthy
consumer identities. The historical full-support join additionally needs F9–F11
live/fresh-process cells, now HELD .30 under seq-377. A count test proves neither
a universal hard byte bound nor real reset survival. No native test ran here.
