---
title: FathomDB 0.8.27 Slice 90 — runtime and root closure design
status: PLANNED
target_release: 0.8.27
---

# Slice 90 runtime and root closure design

This is prospective design, not a verification receipt. Release state binds
Slice 85 at `7a2f9bf9` with closeout `8cd3389d`, but the current
`release/0.8.27` source at `e689000d4` also contains the later Slice 85
recovery commits through `294af94b5`. Recovery qualification and release-state
rebinding remain open. Slice 90's source inventory uses the current source;
`7a2f9bf9` remains the distinct historical D27 performance reference. This
record and the master plan own the complete Slice
90 obligation. Slice 100
cannot start with any requirement below incomplete. No Slice 91 is allocated:
runtime implementation and verification can precede structural moves inside
Slice 90. `D27-runtime-topology` is ruled as Option B by HITL decision
`seq-293`, and `seq-295` accepted and formally codified the successor contract
in `ADR-0.8.27-engine-owned-runtime-topology.md`. AC27-90B remains blocked on
implementation and verification; neither ruling commissions production work.

## Evidence and scope

Historical design review baseline: `ab8f43be2c9ceaa9ad19b23b23f40e9d2c484513`
(the substrate table below is bound to the same commit; engine source is
identical to the earlier `459d528f` review base). The operational Slice 90
entry inventory must instead use current release source at `e689000d4` or
its direct descendant before the first production edit. The current
engine root still owns open/admission, runtime configuration, connection
helpers, WAL and operator facades, index projectors, runtime carriers, and
test seams. `projection_runtime.rs` owns the shared runtime allocation and
hard-codes `PROJECTION_WORKERS`; its timeout starts at
`DEFAULT_EMBED_TIMEOUT_MS`. Python `Engine.open` resolves configuration but
does not pass it to `_NativeEngine.open`. TypeScript passes its options to
NAPI, but NAPI's open implementation consumes only the embedder choice.
The SDK/native config declarations contain five knobs.

Authority is the accepted
[`scheduler-shape ADR`](../../../../adr/ADR-0.6.0-scheduler-shape.md),
[`async-surface ADR`](../../../../adr/ADR-0.6.0-async-surface.md),
[`embedder protocol ADR`](../../../../adr/ADR-0.6.0-embedder-protocol.md),
[`bindings design`](../../../../design/bindings.md), and the applicable
[`Rust`](../../../../interfaces/rust.md),
[`Python`](../../../../interfaces/python.md), and
[`TypeScript`](../../../../interfaces/typescript.md) interfaces. The ADR
requires an engine-owned embedder pool with a configurable size (CPU-count
default) and a configurable per-call timeout (30-second default), with
late results discarded rather than forcibly cancelling a running thread.
The scheduler ADR also requires a distinct dedicated Tokio orchestration pool
(default two), per-job tasks bounded at four times embedder-pool size, and
channel submission to a dedicated writer. The current engine has no Tokio
dependency or such pools: projection runtime starts an OS dispatcher and two
OS workers; workers serialize healthy embedding via `embed_serialize`, call
detached per-call watchdog threads, and commit through worker connections.
`Engine` writes use its mutex-protected connection. Query embedding in
`search.rs` and `embedding::Engine::embed_text` invokes the embedder directly,
without that watchdog. Therefore this is a runtime architecture correction,
not just option forwarding or replacing `PROJECTION_WORKERS`.

Keep one engine crate, rooted `Engine`, private implementation modules, public
root paths and exact cfg gates. Preserve SQL, transaction/lock/commit order,
startup and shutdown order, error precedence, and wire shapes during moves.
Only the explicitly reviewed configuration correction changes behavior.
Slice 40 permitted root navigation during extraction; Slice 50 deliberately
retained subsystem collaboration; Slice 70 deferred open/runtime work. Their
historical reviews are not rewritten. Whole-crate import normalization and
unrelated cycle elimination are outside this slice.

## Ownership and complete inventory

New names below are prospective private modules. Existing public root paths
remain re-exports. Move existing bodies, not wrapper duplicates. Split a large
owner into cohesive private submodules only when its reviewed item map shows
the split preserves ownership; line count alone is not a reason.

| Owner | Final disposition |
| --- | --- |
| `open` | All `Engine::open*` paths and their common admission/open implementation; `OpenReport`, `OpenedEngine`, `EmbedderChoice`; `check_embedder_profile`, `default_embedder_identity`, embedder/reranker open gates, GPU-allocation witness helpers, database-path/lock/admission/header/schema/WAL-sidecar probes, migration error conversion, and open-only startup wiring. Preserve each failure's precedence and cleanup. |
| `runtime_configuration` | `RuntimeConfiguration`, `RuntimeConfigurationError`, `RuntimeSqliteMode`, `configure_runtime`, its locking/effective-state helpers and process-global initialization state; the accepted public `EngineConfig`/`EngineConfigurationError`, private resolved per-engine configuration and validation introduced by the correction. Process-global SQLite mode remains distinct from per-engine knobs. |
| `embed_dispatch` | Concrete private embedding queue/workers, request-state/deadline protocol, dispatch outcomes, accounting and bounded drain. It receives the provider and typed runtime fields, never an `Engine`, SQLite connection or projection state. Caller owners translate outcomes into their existing public/panic boundaries. No generic executor, reaper thread, callback bridge or reverse dependency on open/search/projection is introduced. |
| `connection_runtime` | `open_managed_connection`, `open_runtime_connection`, `configure_reader_lookaside`, `apply_perf_experiment_reader_pragmas`, `apply_perf_experiment_writer_pragmas`, SQLite extension/connection setup, profile-callback installation/uninstallation and connection-only constants. Preserve ABI-sensitive types and exact pragmas. |
| `runtime_lifecycle` | `Engine::close`, `drain`, `drain_for_non_embedding_mutation`, engine `Drop`, and their lifecycle coordination. Retain exact idempotence, drain/freeze, worker join and pool-before-profile-context destruction order. |
| `wal_runtime` | Engine WAL/checkpoint orchestration, `truncate_wal`, `wal_checkpoint_truncate_once`, `TruncateWalStatus`/`TruncateWalReport`, connection-inventory/actual-checkpoint coordination and associated runtime observation carriers. It consumes Slice 85's `wal_attribution` and typed reader capabilities. It does not take ownership of the collector again. |
| `reader_pool` (retained) | `HoldWalSnapshot`, `HoldWalSnapshotBounded`, `HoldWalSnapshotWithCommitAck`, `LookasideStatus`, `CacheStatus`, `SecureDeleteStatus`, and both `Wal*Inventory` arms remain beside the worker-owned connection and request loop. These operations must run on that connection/thread; retaining them preserves pin/ack/release and request-finish ordering. This is a final disposition, not a deferred extraction choice. |
| `operator` | `verify_embedder`, `check_integrity`, `safe_export`, `dump_schema`, `dump_row_counts`, `dump_profile`, `orphan_provenance`, their report/option carriers, integrity sections and pure formatting helpers. Existing data-plane inspection/recovery entry points and their admission/URI helpers live in a private operator submodule; public and operator-feature availability remain exact. |
| Existing domain owners | `trace_source_ref`, `TraceReport`, `TraceEvent` go to `provenance`; `trace_dependency` and `DependencyTraceMeasurement` to `dependency_trace`; `check_data_plane_integrity` to `data_plane_integrity`; `drain_embedder_events` and `Engine::usable_dense_runtime` to `embedding`. No new public module path is introduced. |
| `index_projector` | `project_canonical_node_row`, `project_canonical_edge_row`, `IndexTargetSet`, `index_targets_for_row_kind`, `reproject_search_index_after_tokenizer_upgrade`, `search_index_tokenizer_reproject_complete`, `CanonicalNodeRow`, `canonical_node_rows`, `row_kind_from_column`, and `restore_registered_derived_projections`. Existing projection/vector helpers are called through their owners, not copied. |
| `open` maintenance | `edge_vector_prune_complete` and `prune_orphaned_edge_vectors` remain together under open maintenance because their current authority is admission-time compatibility work. Their markers and ordering move intact. |
| Runtime constants | `PROJECTION_CURSOR_KEY`, `PROJECTION_COMMIT_BATCH`, `PROJECTION_TEMPORAL_WAKE_POLL`, `DEFAULT_PROJECTION_RETRY_DELAYS_MS`, `PROJECTION_RUNTIME_STARTUP_TIMEOUT` remain projection-runtime-owned. The functional correction replaces fixed `PROJECTION_WORKERS`/`PROJECTION_INFLIGHT_LIMIT`/`PROJECTION_SCAN_FETCH` with resolved worker count and checked row-capacity derivation. `embed_dispatch` owns timeout/queue/drain policy; the successor retires `DEFAULT_EMBED_CIRCUIT_THRESHOLD` rather than moving an unused session-latch control. Configuration consumes defaults from these owners. Reader constants are pool-owned after Slice 85. |
| Root | `Engine` storage, module declarations, exact public re-exports, and the small `path`/`ensure_open` core controls remain rooted. Their role is state identity/composition, not a catch-all helper home. Test modules and explicitly inventoried Slice 140 test-seam exceptions preserve qualified test identities. |

The following named residual dispositions are fixed now, not delegated to an
implementation-time inventory. Existing root public paths remain re-exports;
private imports use the semantic owner. A change requires design review.

| Items | Final owner and reason |
| --- | --- |
| `Engine::write_node_importance` | `write`; preserves validation-before-close and its existing transaction/closure guard. |
| `Engine::node_importance` | `read_api`; preserves the present writer-connection read and eligibility predicate, not a new pool dispatch. |
| `EXPLANATION_OPEN_NONCE_SEQUENCE`, `mint_explanation_open_nonce` | `open`; session identity minting is admission wiring; consumers receive the stored Engine nonce, not another global counter. |
| `RowKind` and its impl | `write_types` (new small private contract owner); shared canonical write classification used by write, projection and index projectors, not owned by an executor. |
| `is_legal_transition_move`, `LIFECYCLE_DRAIN_TIMEOUT_MS` | `record_lifecycle`; transition legality and its drain budget. |
| `embedder_required_for`, `EmbeddingReadinessState`, `EmbeddingOperation`, `EmbedderRequired`, `EmbeddingReadiness` and impls | `embedding`; capability/refusal vocabulary, with no second public definition. |
| `RebuildKind`, `RebuildReport`, `REBUILD_DRAIN_TIMEOUT_MS` | `projection_rebuild`. |
| `ProjectionRole`, `ProjectionFts`, `DenseReadiness`, `ProjectionVector`, `ProjectionSpec`, `ProjectionDelta` and impls | `projection_registry`; declaration and registry contract, consumed by runtime. |
| `ProjectionRuntimeUnavailabilityReason`, `ProjectionStatusDenseReadiness`, `ProjectionRuntimeStatusEntry`, `ProjectionRuntimeStatus` and impls | `projection_runtime`; runtime availability report, distinct from registry readiness. |
| `load_next_cursor`, `reserved_write_cursor`, `max_cursor` | `write_commit`; cursor high-water semantics shared with open, including reserved migration boundary. |
| `projection_status`, `PROJECTION_CURSOR_KEY` readers | `projection_runtime`; retain durable terminal/cursor interpretation and `lifecycle::ProjectionStatus` public type. The constant has the single runtime owner above. |
| `projection_batch_has_no_custom_triggers` | `projection_commit`; commit batching safety predicate. |
| `legacy_revision_id`, `digest_record_identity`, root `hex_encode`/`hex_nibble` | `identity`; stable hash/identity recipes shared by write, actuation, projection and export, unchanged. The separate frozen-token codec is not merged into this work. |
| `map_open_sqlite_error`, `map_migration_error` | `errors`; typed corruption/open conversion, called by open and operator without a reverse open dependency. |
| `sqlite_extended_code_name`, `sqlite_extended_code_name_from_int` | `lifecycle`; diagnostic code naming, retaining `SQLITE_UNKNOWN` behavior (no unrelated error redesign). |
| `detect_slow`, `emit_event`, `emit_sqlite_internal_error`, `emit_open_error_event`, `Engine::subscribe` | `lifecycle`; lifecycle event construction/delivery. Existing `Event`, `Phase`, `EventSource`, `EventCategory`, `SubscriberRegistry`, `Subscription` remain there. |
| `CounterSnapshot`, `Engine::counters`, `set_profiling`, `set_slow_threshold_ms`, `DEFAULT_SLOW_THRESHOLD_MS`, both cfg versions of `process_current_rss_bytes`/`process_peak_rss_bytes` | `telemetry`; observability state and controls; connection callback installation/context/trampoline remain `connection_runtime`, which consumes lifecycle events and shared threshold state. |
| `MeanRecomputeReport` | `mean`; existing recomputation result contract. |
| `table_exists` | `open`; admission-only legacy-shape probe. |
| `read_schema_objects`, `order_canonical_first` | `operator`; schema inspection/export helpers. |
| `EDGE_FACT_KIND` | `embedding`; this is the embedding-readiness kind discriminator in `embedder_required_for`, also read by `projection_worker` when deciding whether an absent provider can skip an edge fact. Keep one literal and let the worker import it from the owner. Its value and branches do not change. |
| `MEAN_VEC_PIN_THRESHOLD` | `mean`; the public constant is the compute-once mean threshold consumed by open recovery, test vector writes and `projection_commit`. Keep its value, re-export the same root path, and import the single owner constant at its consumers. |
| `Engine::execute_for_test` | Retain at root through Slice 90 as a Slice 140 test seam already allocated by Slice 80. It takes arbitrary SQL on the Engine writer handle and reports through the root `detect_slow`/lifecycle path; forcing it into a production domain would give that domain an unrelated test-only writer capability. Its `any(test, debug_assertions, test-hooks)` gate, public doc-hidden path and qualified test identity stay exact. Callers include lifecycle observability, fault-injection, integrity and dependency fixtures. |
| `Engine::run_one_thread_poison_for_test` | Retain at root through Slice 90 as a Slice 140 test seam already allocated by Slice 80. This debug-only composite fixture deliberately invokes public write and search paths, inspects projection status and dispatches a lifecycle stress-failure event. No single production owner owns that cross-domain fixture. Keep its `debug_assertions` gate, public doc-hidden path and qualified test identity exact. |

All root constants already consumed by an extracted domain follow that
specific domain: `DEFAULT_VECTOR_PROFILE`/`DEFAULT_VECTOR_PARTITION` to
`vector_storage`; the `VECTOR_EQUIVALENCE_*` constants to `vector_equivalence`;
`DEFAULT_EMBEDDER_*`/`BGE_SMALL_EMBEDDER_NAME` to `open`;
`DEFAULT_PROVENANCE_ROW_CAP` to `provenance`; `DEPENDENCY_GENERATION_KEY`,
`SOURCE_DEPENDENCY_SCHEMA_VERSION`, `DEPENDENCY_LOOKUP_LIMIT` to
`dependency_trace`; `EDGE_TEMPORAL_EPOCH_SCHEMA_VERSION` to `temporal`;
`SEARCH_INDEX_TOKENIZER_SCHEMA_VERSION`/`SEARCH_INDEX_TOKENIZER_REPROJECT_MARKER_KEY`
to `index_projector`; `EDGE_VECTOR_PRUNE_MARKER_KEY` to open maintenance;
`ERASURE_WAL_TRUNCATE_ATTEMPTS`, `ERASURE_WAL_TRUNCATE_BACKOFF_MS`,
`REDACTED_STABLE_ID`, `ERASURE_AUDIT_COLLECTIONS`,
`ERASURE_PENDING_REDACTION_COLLECTION`, `is_erasure_bookkeeping_collection`
to `erasure`. These are moves, not permission to alter values or policies.

The four fields `search_limit_override`, `recency_reweight_enabled`,
`importance_reweight_enabled`, and `vector_stage_only_for_test` remain on
`ProjectionRuntimeShared` in `projection_runtime`. Their shared allocation
already supplies the engine/search test controls with one lifetime and one
atomic value per setting. Keeping each field avoids duplicate state and a
gratuitous struct-shape change. Their semantic consumer is search; their
storage owner is the shared runtime. Record both in the field map. Preserve
the respective limit/off/off/off defaults, atomic orderings, and exact test
seams. Any future change of this disposition needs a reviewed plan amendment,
not a pending decision left at Slice 90 closeout.

Before the first production edit, derive an exact inventory at the current
release-source SHA of every root item, Engine method/field, relevant runtime constant,
and named handoff item. Confirm each final owner above or already settled
Slice 85 ownership; inventory is not authority to assign a different owner.
Any newly discovered unallocated item blocks movement until a prospective
item-specific design amendment is reviewed. Expand
grouped rows into actual symbol names in that inventory, including cfg-gated
variants, impls, statics and test carriers. For each root retention, record its
callers, invariant, cfg, why moving would be worse, and whether it is a
previously allocated Slice 140 test-seam obligation. No wildcard "remaining
helpers", unclassified production item, or general aesthetic exemption passes.
Independent design review approves this exact map before movement. No new
production deferral to Slice 100/140 is permitted; existing Slice 140 test-gate
work stays there without concealing production logic.

## Configuration correction

### Current substrate and required consuming effects

This table describes candidate `ab8f43be2c9ceaa9ad19b23b23f40e9d2c484513`,
not a claim that the accepted executor topology exists. Python exposes
optional arbitrary-precision `int` fields, TypeScript optional `number`, and
NAPI optional `u32` fields; PyO3 has no equivalent five-knob open input yet.
Rust currently has per-field atomics/constants, not one resolved option API.

| Knob | Accepted/design default | Current consumer and width | Correction/proof obligation |
| --- | --- | --- | --- |
| `scheduler_runtime_threads` | Scheduler ADR: two dedicated Tokio orchestration workers | No engine consumer or Tokio runtime; dispatcher plus two OS projection workers use `usize` constants. NAPI's Tokio `spawn_blocking` is a separate binding handoff, not this executor. | Requires the architecture decision below; never wire this to the reader pool, NAPI pool or merely rename projection workers. Prove selected orchestration capacity and bounded backlog under the accepted final contract. |
| `embedder_pool_size` | Scheduler/embedder ADRs: `num_cpus::get()` dedicated dispatch slots | No configured pool. Projection worker count is two, healthy projection embeds serialize through one mutex, and each watchdog invocation spawns an OS thread. Query/direct embeds and open-time vector-equivalence probes invoke the provider outside that guard. | New engine-owned dispatch capacity for every production inference path, including first-open/reopen equivalence probes; prove selected bound, independent-engine isolation, hung-slot accounting and no calls on host runtime threads. The current serialization guard cannot make a claimed N-slot pool effective. The default-compiled `write_vector_for_test` seam is the accepted temporary Slice 140 exception and supplies no dispatch evidence. |
| `embedder_call_timeout_ms` | Embedder ADR: 30,000 ms for every embed call | `ProjectionRuntimeShared::embed_timeout_ms: AtomicU64`, default30,000; projection batch watchdog multiplies by batch length. `search.rs`, `search_api.rs`, `embedding::embed_text`, vector-equivalence probes, and root test vector writes call directly without it. | Route all production inference through the accepted deadline contract; specify admission, queue/service, batch, open-probe and per-operation outcome semantics before RED tests. Preserve hybrid sparse fallback, degraded-open behavior, direct-call errors and projection retry/terminal precedence unless an approved delta explicitly changes one. `write_vector_for_test` remains outside the proof and may not be used in performance, timeout, or lock-order oracles. |
| `provenance_row_cap` | Existing engine default1,000,000 rows | `Engine::provenance_row_cap: AtomicU64`, used by `write`/`actuation` → `write_commit::enforce_provenance_retention`; zero currently disables retention. It is not consumed by the reader/provenance reporting pool. | Forward open-time value and prove actual retention after real write/actuation commits, including zero and independent engines; do not prove it by echoing config. |
| `slow_threshold_ms` | Existing engine default100 ms | Shared `Arc<AtomicU64>` read by `detect_slow` and SQLite profile callback; public setter works after open (PyO3 `u64`, NAPI `u32`), but open config is ignored. | Set before connection/profile setup; prove both operation and statement slow signals plus subsequent setter behavior, retaining strict elapsed>threshold semantics. |

The exposed `EngineConfig` value is an immutable requested-open snapshot, not
a live effective-value view. The existing slow-threshold setter changes only
the effective atomic and does not rewrite that snapshot. Python retains its
frozen dataclass; TypeScript clones and freezes the input object, exposes
readonly fields, and proves mutations of both the caller object and returned
snapshot cannot falsify the reported open request.

The reader pool is eight dedicated OS threads (`READER_POOL_SIZE`) and is not
an exposed scheduler or embedder control. Projection dispatcher/worker
connections and their WAL inventory counts must track the chosen actual
runtime topology. NAPI `call_engine` uses its host-side Tokio blocking
executor, never evidence that the engine owns the required pools.

Resolve numeric contracts before implementation: NAPI `u32` cannot represent
Rust `u64` timeouts/caps, and JS `number` loses integer precision above
2^53−1; Python type annotations do not validate values. The option/interface
delta must explicitly choose and justify checked ranges for each field,
including `usize` platform limits and checked `4*N`/projection-capacity
multiplication,
and preserve or explicitly amend zero semantics. Neither lossy casts nor
silently narrowing Rust to NAPI's current width satisfies symmetry. Test
fractional/negative/NaN/infinity, booleans where relevant, u32+1, JS safe-integer
boundary and Rust overflow; legitimate language-specific representations
need not have identical syntax. Defaults without a pinned accepted range
above are current behavior to characterize, not an invented HITL contract.

### Ruled architecture direction before AC27-90B

**Ruled `D27-runtime-topology`, Option B (`seq-293`).** Slice 90 must retain the
current synchronous primary-writer, projection-worker, and `commit_gate`
ownership model and adopt a narrow successor specifying two real,
independently configurable engine-owned orchestration/embedding capacities
and universal embed deadlines. The literal historical Tokio/task/writer-channel
topology (Option A) is not selected. The successor must define
`scheduler_runtime_threads` meaning/default, dispatch ownership, bounded
backlog, serialized-provider safety, timeout slot accounting, shutdown, and
binding invariants before production code.

A is substantially more than forwarding: it replaces scheduling and writer
handoff/commit ownership and needs generation-token, transaction order,
backpressure, failure recovery and lifecycle qualification. B avoids that
unrelated writer-topology rewrite while closing the advertised controls. The
HITL ruling selects its architecture, but the reviewed successor ADR must
explicitly supersede the conflicting scheduler/async/embedder clauses before
implementation. Do not infer formal ADR amendment from the ruling or from a
source comment calling the current serialization trade-off accepted. The
selected option preserves no-reentrancy, no host-thread embedding,
finish-and-discard cancellation and public sync Rust/Python APIs; conflicting
language-adapter constraints must be resolved in the same accepted contract,
not silently assigned to Slice 110.
Standalone SDK embedding utilities are not Engine config consumers and retain
their own contract; inventory them to prevent a false universal claim.
The deadline covers the engine's `Embedder::embed`/`embed_batch` invocations,
not cross-encoder reranking, provider construction/warmup or identity. The
scaffold's explicit clause map also retires the obsolete adapter shedding and
public saturation-metric prescription: retain public readiness reporting and
private testable runtime accounting, without changing the locked public
`CounterSnapshot` keys or inventing a telemetry API.

The accepted
[`engine-owned runtime successor ADR`](../../../../adr/ADR-0.8.27-engine-owned-runtime-topology.md)
codifies the reviewed
[`Option B successor scaffold`](option-b-successor-adr-scaffold.md). The ADR and
this design require the following before production work:
open-time vector-equivalence dispatch; nonblocking bounded
admission with absolute queue-plus-service deadlines; one fixed timeout per
provider invocation/batch; operation-specific fallback/error behavior;
recovering timed-out-slot accounting without replacement threads; an absolute
embed-runtime drain budget after safe database quiescence and a truthful
incomplete-embed-shutdown result; exact projection row capacity/observability;
current Python either/or and TypeScript object input;
no hypothetical binding custom-embedder bridge; the configured open integrated
with `EmbedderChoice`; and clause-level supersession of scheduler, writer,
embedder, projection-model and async-binding authorities. Decision `seq-293`
selected the architectural direction; `seq-295` accepted the numeric ceilings,
distinct queue/admission bounds, default-one embed concurrency, new typed
configuration error, public configured-open delta, exact incomplete-close
outcome and supersession map. One accepted consequence is explicit: with
default-one embed concurrency, a single hung
provider call occupies the only slot, so every later dense projection and
foreground embed in that session is admission-unavailable until the call
returns, durable projection work stays pending, and bounded drain reports
`Scheduler`. Under PR-9 today the projection watchdog retries and can
terminalize the affected row while later work continues until the session
live-thread breaker prevents additional projection embedding; it is not a
general foreground-safe pool. The HITL accepted that fixed-slot stall trade;
it is not mislabeled as unchanged behavior.

HITL `seq-299` subsequently authorized choosing the lowest default embed
capacity that passes the frozen D27 comparison and unchanged release gates.
The proposed successor selects five workers while preserving explicit
one-worker configuration and its one-slot stall posture. With five workers,
one hung provider call retains one slot; later work may use the other four.
The remaining topology, queue formula, deadline and outcome rules are unchanged.

The D27 successor-design block is closed. Slice 90 still depends on Slice 85
completion and its own explicit execution ruling; `seq-295` does not commission
implementation, waive any of the five controls, or authorize publication.

Implement-as-accepted does not itself create a Slice 91 necessity: verified
runtime replacement can be a mandatory checkpoint inside Slice 90 before
mechanical extraction. No external prerequisite or independently delivered
artifact currently forces a new slice. If the approved topology exposes one,
review that concrete boundary and update the ladder before commissioning;
size or an incomplete runtime receipt cannot justify declaring 90 complete.

Close ledger gap `TC-b602d87a…` (seq 258) within this slice. Trace all five
existing knobs from Python/TypeScript through PyO3/NAPI to the engine's
resolved configuration: `embedder_pool_size`, `scheduler_runtime_threads`,
`provenance_row_cap`, `embedder_call_timeout_ms`, `slow_threshold_ms` (with
the existing language-specific spellings). For each, document accepted
default/range/units, rejection type and ordering, actual consuming component,
and a test proving effect rather than echoing `Engine.config`. No advertised
engine-owned knob may remain silently ignored. Respect the bindings design's
distinction between engine scheduler controls and binding handoff pools.

Before implementation, reconcile the effective runtime topology with the
accepted contracts and review the specific option/API delta. Implement the
accepted contract; an unaccepted ADR proposal or a documented ignored option
does not satisfy completion. If a successor is necessary, its acceptance,
corresponding implementation, and tests must all finish before Slice 90 exits.
Do not change an accepted default merely to preserve the current bug.

Use genuine failing tests before forwarding/runtime changes. Test default,
nondefault, invalid, boundary and independent-engine configurations through
Rust and actual installed Python/Node artifacts. Prove selected concurrency
on the engine executor, not the host's GIL/libuv pool; prove timeout refusal,
late-result discard, no committed late result, and healthy subsequent work.
Use controlled blocking embedders/rendezvous and bounded waits, never a
sleep-based race or a mocked database. Check omitted options, zero/negative,
overflow, wrong types, and mixed config/keyword precedence where applicable.
Installed TypeScript tests reject zero only for the three strictly positive
runtime controls and accept/prove the existing zero behavior of
`provenance_row_cap` and `slow_threshold_ms`.
Dynamic worker-count changes must update capacity, shutdown joins, WAL
inventory expectations and fault cleanup; replacing one constant is not enough.
Preserve Python's existing rejection of mixed `config=` plus per-knob keywords
and TypeScript's single `engineConfig` object; do not invent override/merge
precedence.

Distinguish projection dispatch admission full, queued expiration,
started-provider timeout, provider error and close cancellation. Admission or
queued expiration retains durable pending work without consuming the existing
provider-failure retry budget or producing terminal residue. A real saturation
test holds capacity unavailable longer than that retry schedule, then releases
it and proves projection succeeds without manual repair. Started-provider
timeouts/errors consume the existing failure budget; existing
invalid-vector/dimension validation keeps its current retry/code
semantics. Preserve spent attempts across interruptible capacity waits for the
same admitted generation, within the row-capacity bound. Panics remain a
separate outcome transported to existing caller panic boundaries; they neither
become retryable embed errors nor silently trigger sparse fallback. Late
panics are discarded and fixed dispatch workers remain reusable.

For direct `Engine::embed_text`, map admission full and queued expiration to
`EngineError::Overloaded` (`OverloadedError` in Python and TypeScript), because
the provider has not started. Map a started provider timeout or provider error
to `EngineError::Embedder` (`EmbedderError` in both bindings). Map close
cancellation to `EngineError::Closing` (`ClosingError` in both bindings).
Successful completion or a provider error that wins the request-state race
before close retains its result; close wins only while the request is pending.
The configured-embedder check and existing validation errors keep their
earlier precedence. Timely panics use the existing panic boundary rather than
any of these error variants.

Shutdown is explicitly two-phase. Database quiescence first stops new
public and embed admission, cancels queued/running-result waiters, wakes
projection retry/capacity waits, safely drains active database work and
joins every SQLite owner in the existing teardown order; it is not covered by
the 30-second inference-runtime drain budget, so Slice 90 does not claim a
bounded total `Engine::close`. Full-reader-queue, synchronized active-query and
active-primary-operation real-database tests prove no stop signal is trapped
behind work, no owner is released early, and close completes after the active
operation is released. Only then does the absolute 30-second embed-runtime
drain begin. A provider exceeding it retains no SQLite/WAL/admission ownership,
remains counted by worker-owned shared state without a replacement or reaper
thread, and produces the accepted existing `EngineError::Scheduler` outcome.
The budget is shared across joins and is never renewed by repeated close or
Drop. Concurrent close has one teardown; later calls report incomplete while
workers remain and success after they exit. This is a per-session retention
bound, not a process-wide limit across arbitrarily many reopenings.

Engine publication, not arbitrary provider termination, is all-or-nothing.
A timed-out equivalence probe leaves a live degraded engine with an occupied
slot and no accepted baseline mutation. Separately test a post-probe startup
failure: cancel waiters, unwind database/profile/admission state, apply bounded
embed drain, and retain only provider/request/accounting state while preserving
the original open error. Do not claim complete thread cleanup for a hung
provider or destroy the executor of a successfully degraded open.

For frozen hybrid search, the inference deadline bounds only extra snapshot
retention caused by dispatch/provider waiting. Sparse fallback continues on
the same authoritative reader transaction and releases it at normal query
completion or error; it must not reacquire a different snapshot. A total-query
deadline is not introduced by this slice.

Before executor changes, resolve the batch-fallback self-deadlock at
`projection_worker.rs:553-575` (line numbers at the reviewed baseline). It is
confirmed by reading, not suspected: `embed_projection_batch` binds
`_embed_permit` on `embed_serialize` at line 557 and the `Err(_) => return
per_job()` arm at line 575 evaluates `per_job()` while that guard is still
alive; `run_projection_job` then locks the same non-reentrant mutex at line
754. The row-count and dimension-mismatch fallbacks after the guard block are
not affected. Reachability: the batch path runs only when
`FATHOMDB_PROJECTION_BATCH` is set (`projection_batch_enabled`, opt-in), so
the defect is latent in default configurations and reachable in any session
that opts in and then receives a provider error, timeout or disconnect.
Add a bounded RED test that reaches this fallback, then ensure every fallback
drops the guard/permit before entering the per-job path. A mutant that restores
the under-guard fallback must fail. If the approved executor removes this
mutex, preserve the same no-reentrant-permit proof rather than treating the
mechanical disappearance as evidence.
The breaker-open route is a separate fast-failure check, not a deadlock RED
witness. Bound the mutant with a subprocess or cancellation-safe harness so
Engine Drop cannot hang the test runner. Review the intentional PR-9 oracle
delta: one hung explicit one-worker slot leaves durable work pending and drain
returns `Scheduler` until capacity recovers, instead of spending fabricated retries to
force terminal failure. Preserve write liveness, no late commit, finite thread
counts and recovery tests; do not label this unchanged historical behavior.

This functional correction can require an additive Rust/native option seam.
Review and update the applicable ADR/interface docs and a named allowed-delta
record before that change. Keep Slice 30's immutable baseline intact; compare
against it and explicitly account for only approved configuration deltas.
All unrelated public rows remain exact. Subsequent mechanical batches compare
against the separately captured post-correction candidate as well. Binding
decomposition still belongs to Slices 100/110/120/130; only config forwarding
and its necessary native signature/stub/declaration updates occur here.

Documentation is part of the configuration correction, not deferred
convergence. In the same reviewed batch, update the accepted successor and ADR
index, `dev/design/engine.md`, `dev/design/scheduler.md`, `dev/design/embedder.md`,
`dev/design/bindings.md`, the Rust/Python/TypeScript interface contracts,
`docs/reference/config.md`, and affected error guidance. Record every setting's
binding spelling, default, unit, accepted range, zero/omission meaning,
mutability, precedence, consuming component, backpressure, and observable
error/fallback behavior. Slice 140 may remove stale prose but may not supply
missing Slice 90 configuration truth.

### Binding handoff to Slices 100 and 110

Slice 90 closes configuration behavior before either binding is decomposed.
Keep changes in the existing binding owners: Python config/open, the PyO3
native open seam and stub, NAPI options/open, and TypeScript options/open.
Do not introduce a second Engine wrapper, alternate native module, generic
adapter hierarchy, or new callback executor merely to forward configuration.
The engine-owned scheduler/embedder knobs do not configure the NAPI host
handoff pool. Review that distinction in the five-knob consuming-component
map and test it directly.

The handoff is a candidate-bound package containing the resolved option
contracts, conversion/validation order, effective-value evidence, exact native
signatures, approved surface deltas, production and test-feature artifacts,
runtime export/registration captures, Python stub and generated Node
declarations, hashes, and installed-consumer results. Slices 100/110 compare
their moves against these corrected entry contracts as well as the immutable
Slice 30 baseline plus reviewed deltas. They do not reimplement forwarding or
repeat a completed configuration correction. A missing or failing handoff
receipt blocks Slice 90 exit rather than becoming binding-decomposition debt.

The five settings' consuming effects are proved at the engine owner, with
caller-supplied providers and an attached Rust lifecycle subscriber where those
are needed for deterministic capacity, timeout, and slow-event witnesses.
Installed Python and Node consumers must independently prove all five requested
values reach their freshly built native configured-open path, reject invalid
values before a database is created, and exercise every effect their existing
public bindings expose (including worker inventory where a test feature
already exposes it, and provenance retention). The Python and Node subscriber
adapters presently accept calls without delivering lifecycle events, and their
public opens do not take a caller provider. Do not add a new binding test-only
provider or event adapter solely to duplicate the engine-owner effect tests;
record the exact cross-layer evidence mapping and these observation limits in
the installed-artifact receipt. This does not relax any engine consuming-effect
test, native forwarding check, public config contract, or binding behavior
already exposed by the current API.

At baseline the native subscriber methods accept arguments but do not deliver
events, and NAPI's executor differs from the older accepted async ADR's named
mechanism. These are separate binding-contract questions, not engine-config
options. The binding designs require explicit pre-move disposition and any
necessary separately tested correction within their own slice; no claim of
callback/executor conformance is inferred from configuration tests.

## Runtime configuration and performance qualification

Slice 85 supplies ownership boundaries, dependency enforcement and the exact
entry baseline. It does not implement, tune or qualify D27 runtime behavior.
Slice 90 owns the complete compatibility and performance proof before any
mechanical runtime movement begins. Slices 114, 115 and 135 may audit,
characterize more broadly and compare the completed release, but none may be
the first proof that an accepted setting works or that the default meets the
project's performance contract.

All accepted values require correctness, bounded resources, cleanup and an
observable consuming effect. Performance promises bind the default
configuration, not every legal override. The stage-2 qualification matrix is:

| Runtime configuration | Required proof |
| --- | --- |
| `scheduler_runtime_threads=2`, `embedder_pool_size=5` | Canonical default release-performance gate, installed Rust/Python/Node parity and before/after comparison. |
| `2/1` | Explicit prior default: bounded one-slot progress, overload, timeout and cleanup behavior. |
| `1/1` | Minimum-bound progress, timeout, backpressure, reopen and shutdown correctness. |
| `2/2` | Actual concurrent provider execution, mixed foreground/projection contention and independent-engine isolation. |
| `4/4` | Representative larger override, advertised capacity effect and bounded resource growth. |
| `64/64` | Ceiling validation, exact resource inventory and cleanup only; this is not a performance target. |

A pure resolved-configuration property test covers every scheduler/embed pair
in `1..=64`, including checked row/queue arithmetic. The runtime matrix also
includes `2/no-provider`, which must allocate no idle embed workers or queued
requests. This distinguishes accepted-value compatibility from the smaller set
of expensive runtime samples.

The exact ceiling inventory is derived from the accepted successor before the
RED test is written. Under the current scaffold it includes the configured
orchestration and embed workers, the existing primary/projection/reader owners,
exact SQLite connections, the embed waiting capacity of
`4 * embedder_pool_size`, and projection admission of
`scheduler_runtime_threads * PROJECTION_COMMIT_BATCH`. The test must fail if
either capacity is mislabeled as the other, if an unconfigured host pool is
counted, or if shutdown leaves an engine-owned worker or connection behind.

At the default, run AC-011a/b through the exact release-mode selectors in
`scripts/run-ac011-write-throughput.sh` on the named tier-1 host; the harness
owns the five-second warm-up, 60-second measurement, 1 KiB/100 KiB payloads
and unchanged 1,000/100 commits-per-second thresholds. Also run AC-017
projection-freshness (p99 at most 5 seconds), AC-018 projection-drain (100
deterministic vectors within 2 seconds), AC-029 projection-stall write-tolerance
(at most 1.5 times baseline), AC-072 vector-retrieval latency, AC-073
real-corpus mixed-retrieval tail, AC-076 text/hybrid-query latency and
AC-081a/b/c sequential/eight-reader progress gates. The exact existing scripts,
datasets, thresholds and feature routes are inventoried and cited before the
first run; Slice 90 does not weaken or reinterpret them to make D27 pass.

The immutable workload and decision rule are
`d27-runtime-qualification-protocol.json`. It freezes the entry engine SHA,
runner, generated corpus identity, seeds, operation ratios, concurrency,
warm-up, duration, repetitions, starvation window, metrics, environment
invalidators and median/MAD comparison formula before runtime behavior changes.
The entry run records the generated corpus and raw-output hashes; checkpoint
and final runs must match the protocol and corpus hashes exactly.
The historical entry has no embed-dispatch queue or fixed embed-worker set.
Only operation throughput and latency measured by both versions enter the
entry-versus-candidate median/MAD comparison. Dispatch queue, deadline,
capacity, resource and cleanup metrics are mandatory candidate-only
correctness evidence; the entry receipt marks them unavailable rather than
fabricating zeros or failing for their historical absence.
The historical engine's strict test-only connection-inventory hook returns
`Err(Storage)` even immediately after open in the standalone D27 runner. Its
`sqlite_connections` value is explicitly unavailable at entry, and the error
is retained in raw output. The observer's SQLite connection is external to the
engine and excluded from candidate engine-owned counts. Candidate qualification
requires an exact engine-owned connection inventory from a successful hook;
neither a configured-count inference nor the observer connection can fill it.

Add a D27-specific mixed workload with concurrent canonical writes, dense
projection, foreground hybrid queries and direct embeds. Run both contention
directions and record commit throughput; projection and foreground-query
p50/p95/p99; queue wait and saturation; durable backlog high-water mark;
observed provider concurrency; thread and SQLite-connection inventories; close
latency and residual workers; and starvation. Controlled rendezvous providers,
not sleeps, create queue-full, queued-expiration, started-timeout, provider
error, invalid-vector, panic, hung/recovery, concurrent-engine, failed-startup,
shutdown and batch-fallback conditions. The receipt also proves that
`provenance_row_cap` and `slow_threshold_ms` have their accepted zero/default/
nondefault consuming effects without changing unrelated runtime capacity.

The historical bundle is complete under the shared protocol and corpus hashes.
Candidate-only measurement fields need a narrow test-hooks observation seam;
they cannot be filled from the requested config echo, provider-local counters,
thread-count guesses or the workload's operation timestamps. The candidate
workload opens the real engine with `EngineConfig` and a caller provider,
selects the engine default `2/5` comparison cell and retains the frozen
corpus, operation mix, timing and repetition order. The verifier rejects a
missing or non-engine observation, any scheduler count other than two, and an
embed count outside the approved `2..=64` sweep range. Candidate-bound engine
tests and the receipt pin the exact selected default; explicit overrides
belong to the separate configuration matrix. A read-only engine collector
records the resolved worker counts, projection admission high-water mark and
dispatch request lifecycle. Each dispatch record has a unique engine request
identity, monotonic admission/start/terminal instants, and an engine-carried
owner: one measured foreground operation sequence or the measured canonical
cursor set for a projection batch or per-job fallback. The test-only owner
context must cross foreground reader-work handoffs and both projection routes.
Start collection after the warm-up drain; continue through the measured drain
so every measured foreground sequence and committed projection cursor is
covered by at least one valid request. Retries and per-job fallback may yield
multiple requests for one owner; no event may be unowned. The workload
serializes typed engine-hook records without inventing missing values. A JSON
`source` label alone is not provenance proof: focused tests and independent
source review must establish that every serialized record came from the hook.
The verifier checks unique request IDs, owner coverage, timestamp order,
interval and capacity bounds, provider concurrency, engine thread
and SQLite inventories, and zero residual workers. The seam is absent from
ordinary builds and cannot add an executor, database owner or replacement
thread. Candidate runner/source hashes may differ from the retained historical
bundle; each validates against its own artifacts, while both require identical
protocol and corpus hashes. A changed measurement protocol or corpus would
require a fresh historical run before comparison. A self-test pins that
candidate-only fields do not alter historical raw recomputation or the
median/MAD formula.

Before 2a or any other semantic runtime edit, land the measurement-only D27
harness, independently review its conformance to the frozen protocol, and run
it against engine candidate `7a2f9bf90783f545603516502bac0016d4b93a14` in
an exact-source checkout. Before Phase 2, retain the harness, protocol,
binary, corpus, raw-output and environment hashes in either a valid entry
receipt or an explicitly invalid attempt record. The qualifying entry receipt
is required before the runtime checkpoint and structural Phase 3. A harness
that cannot execute on that source blocks runtime implementation rather than
falling back to a post-change baseline. Separately inventory and characterize
the recovered Slice 85 source in the current release branch; the historical performance
reference is not its ownership or scanner baseline. Missing recovery
qualification cannot be described as a Slice 90 entry PASS.

The 2026-10-01 HITL direction to use windchill3 as currently installed permits
Phase 2 RED/GREEN runtime implementation after the reviewed harness has built
and executed on the exact historical source, even if the run is environment
invalid. The current reviewed harness did so and its first repetition was
invalidated by host swap-in and a long-lived host pytest process. The two
earlier invalid attempts used an older protocol hash; none provides qualifying
entry center/MAD. This is a sequencing exception only: the
historical source remains fixed, invalid attempts are not promoted to PASS,
and the candidate-bound runtime checkpoint and Phase 3 remain blocked until
valid entry/candidate evidence exists. HITL `seq-297` permits a reviewed
bounded-swap rule applied identically to historical and candidate repetitions.
The amended protocol caps combined host `pswpin`/`pswpout` movement at 128
pages per entire workload child repetition, requires each counter to be present and
monotonic at every sample, and retains both deltas in raw-linked metrics.
The bound starts immediately before the child and ends immediately after it,
including seed/setup, warmup, measurement, drain and close. Compilation and
corpus generation outside the child are excluded. The cap is small relative to
the fresh 91.57-second invalid child attempt, which moved 8 pages in and 16
pages out; a separate 20-second idle sample moved 16 pages in. It accommodates
observed host background movement without accepting an unbounded swap episode.
Other invalidators and
the frozen median/MAD comparison are unchanged. The amended protocol, runner
and verifier require independent review before any fresh entry or candidate
run; old invalid attempts keep their original protocol hash and cannot be
promoted. HITL `seq-298` authorizes the newly installed GPU driver version,
but GPU PASS still requires a successful gate on the running host module.

After the 2026-10-01 reboot, windchill3's kernel returns `EACCES` for the
unprivileged readlink of `/proc/1/ns/pid` under `ptrace_scope=1`. The same
host view still reports the pinned initial PID namespace for the runner,
`systemd` as PID 1 from both `/proc/1/comm` and `ps`, `hidepid=0`, matching
runner PID via `os.getpid()`, `/proc/self` and `ps`, and a complete competitor
scan. Treat only `EACCES` or `EPERM` on that one link as the protocol's blank
unavailable-link value. Read PID 1's name independently so the link error
cannot erase it. Missing PID 1 identity, any other link error, a mismatched
nonblank namespace, or any failed host-view corroboration remains invalid.
Apply this identically to entry and candidate; retain the new protocol and
runner hashes, raw observations and independent review. No swap, competitor,
workload or median/MAD rule changes at this step.

The owner subsequently directed a report-only rule for host-wide swap
movement because unrelated host activity cannot be contained. The reviewed
`d27-runtime-qualification-protocol-v2.json` applies that rule to new candidate
runs. Counters must still be present, nonnegative and monotonic at every
sample, with exact raw-linked deltas. The original v1 protocol, corpus and
six-repetition historical entry stay frozen; the v2 verifier revalidates that
entry and its raw artifacts under the original stricter v1 swap rule before
comparison. All other invalidators and the median/MAD rule remain unchanged.
Two complete, environment-valid v2 campaigns on exact source `2e94aaf4f`
conflict: the first misses projection-heavy canonical-commit p95, while one
retry passes. Both remain evidence; their disposition is required before the
runtime checkpoint can pass.

The durable qualification receipt names the exact stage-2 candidate, optimized
build and features, hardware/software, dataset/workload, warm-up, repetitions,
raw or reproducible outputs, entry candidate and comparison method. A miss at
the default blocks the runtime checkpoint. The already-frozen noise-aware rule
may only substitute the measured entry center/MAD values; it may not change its
formula after the entry run. The D27 workload must show progress in both
contention directions within the
contractual queue and resource bounds. Optimize within the accepted safety
contract and repeat the identical matrix; if the remedy changes a default,
capacity partition or executor shape, stop and formally revise or succeed the
ADR before proceeding. Do not defer the miss to Slice 114, 115 or 135. Create
Slice 91 only if evidence establishes a materially different executor or
performance-remediation boundary that warrants its own reviewed contract. Write
the candidate-bound result, commands and artifact hashes to
`dev/plans/0.8.27/features/slice-90/runtime-performance-qualification.md`.

## Ordered batches and checks

1. Capture the exact entry baseline, inventory and reviewed owner map. Reuse
   Slice 85 receipts only when source/artifact/features genuinely match.
   Freeze focused route counts, public/hidden captures and configuration
   expectations; identify source scrapers before moving their inputs. Before
   2a, land and independently review the measurement-only D27 harness against
   `d27-runtime-qualification-protocol.json`, then execute it against exact
   engine source `7a2f9bf9` and bind the entry binary/protocol/corpus/raw-output
   hashes. This is a prerequisite, not part of 2k.
2. Implement the accepted runtime successor after Slice 85 closes and Slice 90
   receives its explicit execution ruling, as the following ordered RED/GREEN
   sub-batches. Each is separately buildable, separately reviewed
   against the 300–600 non-mechanical threshold, and lands its own tests:
   1. **2a — R27-90J.** Bounded RED for the `embed_projection_batch`
      error/timeout under-guard fallback under `FATHOMDB_PROJECTION_BATCH`;
      GREEN drops the guard before every per-job route; restoration mutant
      fails under a subprocess/cancellation-safe bound.
   2. **2b — validation and configured-open seam.** `EngineConfig` /
      `ResolvedRuntimeConfiguration` / typed configuration error;
      `open_with_choice_and_config` joins the `EmbedderChoice` family; range,
      zero/omission and precedence tests; no executor yet.
   3. **2c — executor core.** Engine-owned orchestration and embed-dispatch
      executors with bounded admission, absolute queue-plus-service deadlines,
      per-engine hung-slot accounting and `Overloaded` saturation; unit and
      mixed-load tests against the executor alone.
   4. **2d — projection path.** Route `run_projection_job` /
      `embed_projection_batch` through the executor; stage-specific retry
      accounting; retire `embed_serialize`, the session latch and the
      detached watchdog threads for this path while preserving the 2a
      lock-order proof.
   5. **2e — query and direct paths.** Route `search_api`, reader-worker
      hybrid search and `Engine::embed_text` through the executor with
      same-snapshot sparse fallback and operation-specific outcomes.
   6. **2f — open-time probe.** Route the vector-equivalence probe; degraded
      open retains its live engine; failed post-probe startup preserves the
      original error.
   7. **2g — close protocol.** Two-phase close: database quiescence with
      cancellation-before-join, then the bounded embed-runtime drain and the
      truthful incomplete result; PR-9 oracle delta applied as reviewed.
   8. **2h — Python/PyO3 forwarding** with either/or input and installed
      artifact tests.
   9. **2i — Node/TypeScript/NAPI forwarding** with object input, Number-safe
      caps and installed artifact tests.
   10. **2j — documentation.** Successor ADR and decision index, internal
       designs, three interface contracts, `docs/reference/config.md`, error
       guidance.
   11. **2k — candidate-bound runtime performance and resource
       qualification.** Run the matrix and named gates above, preserve raw or
       reproducible evidence, compare the default with the entry candidate and
       resolve any miss before continuing.
   12. **2l — installed-artifact parity and checkpoint receipts.** Repeat the
       installed binding checks at the qualified candidate and assemble the
       configuration, fault, resource and performance receipts.
   Do not mix structural movement into these diffs. **Runtime checkpoint:**
   after 2l, obtain independent code review and independent read-only
   verification at the exact candidate and populate the structured
   `runtime_checkpoint` object in the Slice 90 release-state ladder entry. It
   records the candidate and binding SHAs plus each performance/review receipt's
   path, SHA-256, PASS status and candidate SHA. The always-on
   `scripts/check-runtime-checkpoints.py` gate verifies those bindings. Stage 3
   may not start until the checkpoint is PASS and its binding SHA precedes the
   recorded first stage-3 commit. The checkpoint is the
   candidate-bound verification boundary that makes a separate runtime slice
   unnecessary; it is not optional and cannot be replaced by the final Slice
   90 closeout review.
3. Move resolved/process runtime configuration, then connection primitives,
   then open/admission and open maintenance. Split open by an actual admission
   or cleanup phase while retaining one public path and exact sequencing.
4. Move runtime lifecycle and WAL orchestration separately; verify retained
   reader arms and each retained shared search field. Re-run shutdown, fault,
   snapshot and inventory tests after each cluster.
5. Move index projectors, then operator families in separate batches, then
   remaining named domain carriers/helpers and core facade wiring. Every move
   consumes the approved inventory; no domain is reopened for generic cleanup.
6. Reconcile every root residual against its approved disposition, run the
   exit matrix, obtain independent code review and independent read-only
   verification at the exact candidate, and close all requirements below.
   Re-run the default performance gates, the D27 mixed workload and exact
   resource/cleanup inventory at this final candidate. If stages 3–5 changed
   runtime behavior or configuration logic rather than moving it verbatim,
   re-run the full stage-2 configuration matrix before closeout.

Target 300–600 non-mechanical changed lines; split above 800 unless a recorded
cohesion reason receives review. Mechanical batches normally move 300–1,200
lines and one semantic cluster. Larger cohesive bodies require a documented
split analysis and focused review, not an artificial helper boundary. These
are review thresholds, not completion criteria. Keep each batch buildable;
characterization/compile RED→GREEN cycles preserve behavioral assertions.
New logic requires genuine RED tests. Any unrelated semantic defect stops its
move and gets a separately reviewed test-first correction.

Per batch: formatting/lint, affected feature typechecks, focused owning tests,
source-scraping gates, and the Slice 85 boundary report/gate. Update the
discovered classification when a module is added; update only touched
ownership entries, never widen a cycle allowance. Public/re-export/config or
operator changes require the affected official surface comparisons in that
batch. Hidden/test identities are checked for removals or unintended changes.
Run repository-required verification before declaring the slice green.

Inventory scraper inputs from `scripts/tests` and engine tests. At minimum
retain the Windows WAL guard, slice35 audit/manifest, public removal/C1 gates,
and `slice60_fix1_wire` negative scan; a new graph file joins that scan. Parser
retargets need RED fixtures/mutants proving no assertion was weakened.

The exit matrix includes separate default, operator, test-hooks,
slice72-test-hooks, benchmark, debug/release and applicable accelerator
routes; do not substitute incompatible all-features builds. Run the existing
feature-complete gate, installed Python/Node configuration tests and a real
non-Linux compile of the moved graph/runtime arms, with platform and feature
inputs recorded. Fresh processes cover runtime-mode initialization, defaults
and overrides, invalid config, open/close/reopen, lock contention, probe side
effects, error precedence, and faulted startup/shutdown without orphaned
runtime/WAL state. Independent reopen verifies persistent results. Preserve
existing codec/property tests and qualified test identities.

## Requirements and exit acceptance

AC27-90A's entry inventory confirms the item-specific owners above; it cannot
invent final homes during implementation. AC27-90B implements the reviewed and
formally accepted Option B successor; its pass requires real
executors and all five table rows' consuming-effect/width/precedence tests,
including open-time equivalence, query, direct-call and projection deadline
coverage, stage-specific projection outcomes, frozen-snapshot authority,
database quiescence, bounded embed-runtime drain, documentation, and
slow-threshold open initialization. A config echo, worker-count rename,
proposed ADR or forwarder without an executor is a failing result. AC27-90H consumes Slice 85's
bounded explicit-path extraction, transitive root reach and named type-only
admission under the recovered source grammar and relevant configuration table.
It does not consume a frozen edge census or arbitrary receiver inference, and
does not treat newly moved owners as invisible out-of-scope endpoints.

| ID | Requirement | Falsifiable acceptance |
| --- | --- | --- |
| R27-90A | Complete semantic ownership and root closure. | AC27-90A: the source-derived entry and final inventories account for every named/root item, field, method and cfg variant; each reaches its approved final owner or named retained-root disposition, with no unresolved/optional entries or production carryover. Root retains only composition/state/core controls and specifically justified test/contract items. The 300–800-line aspiration cannot override ownership. |
| R27-90B | Runtime configuration is effective, symmetric and documented. | AC27-90B: all five advertised knobs have an authoritative contract and observable consuming effect; Rust plus installed Python/Node default/nondefault/invalid cases pass, including accepted zero values for provenance retention and slow thresholds. Pool sizing, exact projection-row capacity, every production inference path, stage-specific projection retry accounting, operation-specific fallback/errors, queue/service timeout, mixed foreground/projection load, same-snapshot frozen fallback, late completion, recovering hung-slot accounting, concurrent engines, safe database quiescence, absolute embed-drain cleanup, and truthful incomplete embed shutdown match the accepted contract. Saturation longer than the provider-failure retry schedule cannot terminalize durable projection work. The successor/ADR index, internal designs, all language interfaces, public config reference and affected error guidance document every accepted setting and behavior before closeout. Seq-258's gap is closed by implementation evidence or an accepted and implemented successor, never by a proposal alone. |
| R27-90C | Open and connection semantics survive extraction. | AC27-90C: fresh-process admission/probe/runtime-mode/error-order tests and failure injection pass with unchanged SQL, locks, side effects, cfg and cleanup apart from the separately approved configuration behavior. Every named open/connection helper has its inventory disposition. |
| R27-90D | WAL and lifecycle ownership is complete. | AC27-90D: open/close/reopen, drain, concurrent/idempotent close, faulted startup/shutdown, full reader queues, active long queries, active primary operations, busy/checkpoint behavior, native inventories and worker-zero pause/ack tests pass. With an inference timeout longer than the drain budget, cancellation still wakes SQLite-owning waiters before join. Retained reader arms execute on their existing connection/thread; no sender escapes. No SQLite owner, runtime probe, WAL pin, profile callback or admission lock survives database quiescence. Any provider call surviving the later shared embed-drain deadline is counted by worker-owned state, has no database state, receives no replacement/reaper thread and cannot be reported as complete shutdown. Repeated close/Drop never renews the budget. Failed post-probe startup preserves the original error and cleans database ownership; successful degraded open retains its usable engine/executor. |
| R27-90E | Projector and operator work is finished. | AC27-90E: every projector and operator family in the owner map is moved and tested under its exact feature gates; projection/registry/vector state, integrity findings, reports, error mappings and nonmutating diagnostic behavior match the entry evidence. No index-projector or operator item remains pending for Slice 100. |
| R27-90F | Shared runtime field decisions are closed. | AC27-90F: the four named search-control fields remain one value each on the shared runtime allocation; exact defaults, atomics, lifetime and test controls are characterized and unchanged. Storage and consumer ownership are both recorded; there is no remaining relocation decision. |
| R27-90G | Surfaces and platform coverage remain truthful. | AC27-90G: immutable Slice 30 comparison reports only individually reviewed config deltas; mechanical comparisons against the post-correction candidate are equal. Hidden surface is additive only unless an existing accepted contract explicitly requires a reviewed change. Rust root paths, Python stubs, Node declarations, feature gates and qualified tests remain accounted for. All required matrix routes, including non-Linux compilation, have candidate-bound receipts; an unavailable executor blocks closeout. |
| R27-90H | Structural enforcement survives runtime moves. | AC27-90H: the bounded Slice 85 gate and negative fixtures pass; root-item paths and touched new owners are classified, source scrapers retain their oracles, and none of the four forbidden cycles or a new governed return path is introduced. No whole-crate normalization or automatic exception growth occurs. |
| R27-90I | Completion is independently demonstrated before bindings decompose. | AC27-90I: the stage-2 runtime checkpoint is a structured PASS object in release state whose candidate and binding SHAs plus performance, independent-code-review and independent-read-only-verification receipt paths, SHA-256 digests, PASS states and candidate SHAs pass `scripts/check-runtime-checkpoints.py`; its binding commit is an ancestor of the recorded first stage-3 commit. Independent code review and read-only verification also pass at the final candidate, repository-required gates and installed-artifact receipts pass, and the owner/requirement inventory has zero open Slice 90 items. Only then can release state mark Slice 90 complete and unblock Slice 100. Slice 150 still owns exact-final-candidate AC-037; historical security receipts are not reused as current claims. |
| R27-90J | Batch embed fallback cannot reacquire its own serialization guard or executor permit. | AC27-90J: a bounded RED test, run with `FATHOMDB_PROJECTION_BATCH` enabled, reproduces the `embed_projection_batch` returned-error/timeout fallback (the `Err(_) => return per_job()` arm) while the batch guard is held; breaker-open fast failure is tested separately. GREEN proves every per-job fallback occurs only after the guard/permit is dropped; the restoration mutant fails under a subprocess/cancellation-safe bound rather than hanging Drop, and the approved executor transition retains the same lock-order guarantee. |
| R27-90K | Accepted runtime settings are compatible with the codebase and the default meets the project's performance contract. | AC27-90K: before semantic runtime work, the measurement-only D27 harness is reviewed and executed against exact engine candidate `7a2f9bf9` under `d27-runtime-qualification-protocol.json`, retaining protocol/corpus/binary/raw-output hashes and any invalidation. Before the runtime checkpoint and structural Phase 3, a valid six-repetition entry binds those hashes and entry center/MAD values. The exact stage-2 candidate then has a durable qualification receipt for the default `2/5`, explicit `2/1`, `1/1`, `2/2`, `4/4`, `64/64` and `2/no-provider` matrix, plus a property test of checked capacity derivation over every pair in `1..=64`. Every valid case proves consuming effect, bounds, cleanup and fault behavior; the ceiling case proves exact resource inventory rather than throughput. At default `2/5`, the exact AC-011a/b release selectors, AC-017, AC-018, AC-029, AC-072, AC-073, AC-076 and AC-081a/b/c pass unchanged, and the D27 workload passes the frozen median/MAD rule while reporting the required throughput, latency, queue/backlog, concurrency, resource, close and bidirectional-starvation metrics. Rust and installed Python/Node cover omission, explicit zero, minimum, maximum, invalid, overflow, wrong-type and language-safe-number rules. The exact final Slice 90 candidate repeats the default gates, mixed workload and resource/cleanup inventory; any post-checkpoint semantic runtime/configuration edit triggers the full matrix again. A default miss blocks the checkpoint or closeout and cannot be assigned to Slices 114, 115 or 135. |

If any acceptance remains unmet, Slice 90 remains incomplete. A revision of
the ladder requires an explicit reviewed dependency/verification reason; batch
size or the convenience of declaring partial success is not such a reason.
