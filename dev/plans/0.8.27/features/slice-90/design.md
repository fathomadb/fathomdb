---
title: FathomDB 0.8.27 Slice 90 — runtime and root closure design
status: PLANNED
target_release: 0.8.27
---

# Slice 90 runtime and root closure design

This is prospective design, not implementation authority or a verification
receipt. Slice 85 must close first; Slice 90 remains uncommissioned. This
record and the master plan own the complete Slice 90 obligation. Slice 100
cannot start with any requirement below incomplete. No Slice 91 is allocated:
runtime implementation and verification can precede structural moves inside
Slice 90. AC27-90B is currently blocked on the unruled architecture choice
below; this record does not authorize a successor to an accepted ADR.

## Evidence and scope

Reviewed baseline: `459d528f2af008739dfb81656403d0a55e23072b`. The current
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
| `runtime_configuration` | `RuntimeConfiguration`, `RuntimeConfigurationError`, `RuntimeSqliteMode`, `configure_runtime`, its locking/effective-state helpers and process-global initialization state; the engine-owned resolved per-engine configuration and validation introduced by the correction. Process-global SQLite mode remains distinct from per-engine knobs. |
| `connection_runtime` | `open_managed_connection`, `open_runtime_connection`, `configure_reader_lookaside`, `apply_perf_experiment_reader_pragmas`, `apply_perf_experiment_writer_pragmas`, SQLite extension/connection setup, profile-callback installation/uninstallation and connection-only constants. Preserve ABI-sensitive types and exact pragmas. |
| `runtime_lifecycle` | `Engine::close`, `drain`, `drain_for_non_embedding_mutation`, engine `Drop`, and their lifecycle coordination. Retain exact idempotence, drain/freeze, worker join and pool-before-profile-context destruction order. |
| `wal_runtime` | Engine WAL/checkpoint orchestration, `truncate_wal`, `wal_checkpoint_truncate_once`, `TruncateWalStatus`/`TruncateWalReport`, connection-inventory/actual-checkpoint coordination and associated runtime observation carriers. It consumes Slice 85's `wal_attribution` and typed reader capabilities. It does not take ownership of the collector again. |
| `reader_pool` (retained) | `HoldWalSnapshot`, `HoldWalSnapshotBounded`, `HoldWalSnapshotWithCommitAck`, `LookasideStatus`, `CacheStatus`, `SecureDeleteStatus`, and both `Wal*Inventory` arms remain beside the worker-owned connection and request loop. These operations must run on that connection/thread; retaining them preserves pin/ack/release and request-finish ordering. This is a final disposition, not a deferred extraction choice. |
| `operator` | `verify_embedder`, `check_integrity`, `safe_export`, `dump_schema`, `dump_row_counts`, `dump_profile`, `orphan_provenance`, their report/option carriers, integrity sections and pure formatting helpers. Existing data-plane inspection/recovery entry points and their admission/URI helpers live in a private operator submodule; public and operator-feature availability remain exact. |
| Existing domain owners | `trace_source_ref`, `TraceReport`, `TraceEvent` go to `provenance`; `trace_dependency` and `DependencyTraceMeasurement` to `dependency_trace`; `check_data_plane_integrity` to `data_plane_integrity`; `drain_embedder_events` and `Engine::usable_dense_runtime` to `embedding`. No new public module path is introduced. |
| `index_projector` | `project_canonical_node_row`, `project_canonical_edge_row`, `IndexTargetSet`, `index_targets_for_row_kind`, `reproject_search_index_after_tokenizer_upgrade`, `search_index_tokenizer_reproject_complete`, `CanonicalNodeRow`, `canonical_node_rows`, `row_kind_from_column`, and `restore_registered_derived_projections`. Existing projection/vector helpers are called through their owners, not copied. |
| `open` maintenance | `edge_vector_prune_complete` and `prune_orphaned_edge_vectors` remain together under open maintenance because their current authority is admission-time compatibility work. Their markers and ordering move intact. |
| Runtime constants | `PROJECTION_CURSOR_KEY`, `PROJECTION_WORKERS`, `PROJECTION_COMMIT_BATCH`, `PROJECTION_INFLIGHT_LIMIT`, `PROJECTION_SCAN_FETCH`, `PROJECTION_TEMPORAL_WAKE_POLL`, `DEFAULT_PROJECTION_RETRY_DELAYS_MS`, `PROJECTION_RUNTIME_STARTUP_TIMEOUT` go to `projection_runtime`; `DEFAULT_EMBED_TIMEOUT_MS` and `DEFAULT_EMBED_CIRCUIT_THRESHOLD` to `embedding`; configuration consumes defaults from these owners. Reader constants are pool-owned after Slice 85. |
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

Before the first production edit, derive an exact inventory at the Slice 85
exit SHA of every root item, Engine method/field, relevant runtime constant,
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
| `embedder_pool_size` | Scheduler/embedder ADRs: `num_cpus::get()` dedicated dispatch slots | No configured pool. Projection worker count is two, healthy projection embeds serialize through one mutex, and each watchdog invocation spawns an OS thread. Query/direct embeds execute on their caller. | New engine-owned dispatch capacity for all engine embed paths; prove selected bound, independent-engine isolation, hung-slot accounting and no calls on host runtime threads. The current serialization guard cannot make a claimed N-slot pool effective. |
| `embedder_call_timeout_ms` | Embedder ADR: 30,000 ms for every embed call | `ProjectionRuntimeShared::embed_timeout_ms: AtomicU64`, default30,000; projection batch watchdog multiplies by batch length. `search.rs`, `embedding::embed_text`, and root test vector writes call directly without it. | Route engine-owned query, projection and direct embedding through the accepted watchdog contract; specify batch deadline and queue-wait versus service deadline before RED tests. Preserve public error precedence unless an approved delta explicitly changes it. |
| `provenance_row_cap` | Existing engine default1,000,000 rows | `Engine::provenance_row_cap: AtomicU64`, used by `write`/`actuation` → `write_commit::enforce_provenance_retention`; zero currently disables retention. It is not consumed by the reader/provenance reporting pool. | Forward open-time value and prove actual retention after real write/actuation commits, including zero and independent engines; do not prove it by echoing config. |
| `slow_threshold_ms` | Existing engine default100 ms | Shared `Arc<AtomicU64>` read by `detect_slow` and SQLite profile callback; public setter works after open (PyO3 `u64`, NAPI `u32`), but open config is ignored. | Set before connection/profile setup; prove both operation and statement slow signals plus subsequent setter behavior, retaining strict elapsed>threshold semantics. |

The reader pool is eight dedicated OS threads (`READER_POOL_SIZE`) and is not
an exposed scheduler or embedder control. Projection dispatcher/worker
connections and their WAL inventory counts must track the chosen actual
runtime topology. NAPI `call_engine` uses its host-side Tokio blocking
executor, never evidence that the engine owns the required pools.

Resolve numeric contracts before implementation: NAPI `u32` cannot represent
Rust `u64` timeouts/caps, and JS `number` loses integer precision above
2^53−1; Python type annotations do not validate values. The option/interface
delta must explicitly choose and justify checked ranges for each field,
including `usize` platform limits and checked `4*N`/batch multiplication,
and preserve or explicitly amend zero semantics. Neither lossy casts nor
silently narrowing Rust to NAPI's current width satisfies symmetry. Test
fractional/negative/NaN/infinity, booleans where relevant, u32+1, JS safe-integer
boundary and Rust overflow; legitimate language-specific representations
need not have identical syntax. Defaults without a pinned accepted range
above are current behavior to characterize, not an invented HITL contract.

### Architecture decision required before AC27-90B

**Unruled `D27-runtime-topology`.** Should Slice 90 (A) implement the accepted
scheduler ADR literally, including its Tokio/task/writer-channel topology,
or (B, recommended) seek an accepted narrow successor retaining the current
synchronous projection/commit architecture while specifying two real,
independently configurable engine-owned orchestration/embedding capacities
and universal embed deadlines? B is a proposal, not permission to reinterpret
`scheduler_runtime_threads`; its successor must define that option's exact
meaning/default, dispatch ownership, bounded backlog, serialized-provider
safety, timeout slot accounting, shutdown and binding invariants before code.

A is substantially more than forwarding: it replaces scheduling and writer
handoff/commit ownership and needs generation-token, transaction order,
backpressure, failure recovery and lifecycle qualification. B avoids that
unrelated writer-topology rewrite while closing the advertised controls, but
cannot override the accepted scheduler/async/embedder ADRs without acceptance.
Do not infer acceptance from this recommendation or from a source comment
calling the current serialization trade-off accepted. Both options preserve
no-reentrancy, no host-thread embedding, finish-and-discard cancellation and
public sync Rust/Python APIs; conflicting language-adapter constraints must
be resolved in the same accepted contract, not silently assigned to Slice 110.
Standalone SDK embedding utilities are not Engine config consumers and retain
their own contract; inventory them to prevent a false universal claim.

AC27-90B and commissioning of dependent runtime changes are **BLOCKED** until
this choice and the resulting contract are approved. Safe characterization,
ownership design and Slice 85 planning can proceed. There is no automatic
waiver or deferral of the five controls. After a ruling, amend this design
with exact topology and tests before independent design approval.

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
Dynamic worker-count changes must update capacity, shutdown joins, WAL
inventory expectations and fault cleanup; replacing one constant is not enough.

This functional correction can require an additive Rust/native option seam.
Review and update the applicable ADR/interface docs and a named allowed-delta
record before that change. Keep Slice 30's immutable baseline intact; compare
against it and explicitly account for only approved configuration deltas.
All unrelated public rows remain exact. Subsequent mechanical batches compare
against the separately captured post-correction candidate as well. Binding
decomposition still belongs to Slices 100/110/120/130; only config forwarding
and its necessary native signature/stub/declaration updates occur here.

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

At baseline the native subscriber methods accept arguments but do not deliver
events, and NAPI's executor differs from the older accepted async ADR's named
mechanism. These are separate binding-contract questions, not engine-config
options. The binding designs require explicit pre-move disposition and any
necessary separately tested correction within their own slice; no claim of
callback/executor conformance is inferred from configuration tests.

## Ordered batches and checks

1. Capture the exact entry baseline, inventory and reviewed owner map. Reuse
   Slice 85 receipts only when source/artifact/features genuinely match.
   Freeze focused route counts, public/hidden captures and configuration
   expectations; identify source scrapers before moving their inputs.
2. Implement the configuration correction in RED/GREEN sub-batches: engine
   validation/executor/watchdog after the architecture ruling and design
   approval; Python/PyO3 forwarding; Node/TypeScript/NAPI
   forwarding; installed-artifact parity and independent review. Do not mix
   structural movement into these diffs.
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
invent final homes during implementation. AC27-90B is blocked by
`D27-runtime-topology`; its pass requires the approved topology's real
executors and all five table rows' consuming-effect/width/precedence tests,
including query and direct-call timeout coverage and slow-threshold open
initialization. A config echo, worker-count rename, proposed ADR or forwarder
without an executor is a failing result. AC27-90H consumes Slice 85's
whole-crate extraction, transitive root reach and named type-only admission;
it does not treat newly moved owners as invisible out-of-scope endpoints.

| ID | Requirement | Falsifiable acceptance |
| --- | --- | --- |
| R27-90A | Complete semantic ownership and root closure. | AC27-90A: the source-derived entry and final inventories account for every named/root item, field, method and cfg variant; each reaches its approved final owner or named retained-root disposition, with no unresolved/optional entries or production carryover. Root retains only composition/state/core controls and specifically justified test/contract items. The 300–800-line aspiration cannot override ownership. |
| R27-90B | Runtime configuration is effective and symmetric. | AC27-90B: all five advertised knobs have an authoritative contract and observable consuming effect; Rust plus installed Python/Node default/nondefault/invalid cases pass. Pool sizing, timeout, late completion, concurrent engines, and cleanup match the accepted contract. Seq-258's gap is closed by implementation evidence or an accepted and implemented successor, never by a proposal alone. |
| R27-90C | Open and connection semantics survive extraction. | AC27-90C: fresh-process admission/probe/runtime-mode/error-order tests and failure injection pass with unchanged SQL, locks, side effects, cfg and cleanup apart from the separately approved configuration behavior. Every named open/connection helper has its inventory disposition. |
| R27-90D | WAL and lifecycle ownership is complete. | AC27-90D: open/close/reopen, drain, idempotent close, faulted startup/shutdown, busy/checkpoint behavior, native inventories and worker-zero pause/ack tests pass. Retained reader arms execute on their existing connection/thread; no sender escapes. No orphaned workers, runtime probes, WAL pins or profile callbacks survive the defined cleanup point. |
| R27-90E | Projector and operator work is finished. | AC27-90E: every projector and operator family in the owner map is moved and tested under its exact feature gates; projection/registry/vector state, integrity findings, reports, error mappings and nonmutating diagnostic behavior match the entry evidence. No index-projector or operator item remains pending for Slice 100. |
| R27-90F | Shared runtime field decisions are closed. | AC27-90F: the four named search-control fields remain one value each on the shared runtime allocation; exact defaults, atomics, lifetime and test controls are characterized and unchanged. Storage and consumer ownership are both recorded; there is no remaining relocation decision. |
| R27-90G | Surfaces and platform coverage remain truthful. | AC27-90G: immutable Slice 30 comparison reports only individually reviewed config deltas; mechanical comparisons against the post-correction candidate are equal. Hidden surface is additive only unless an existing accepted contract explicitly requires a reviewed change. Rust root paths, Python stubs, Node declarations, feature gates and qualified tests remain accounted for. All required matrix routes, including non-Linux compilation, have candidate-bound receipts; an unavailable executor blocks closeout. |
| R27-90H | Structural enforcement survives runtime moves. | AC27-90H: the bounded Slice 85 gate and negative fixtures pass; root-item paths and touched new owners are classified, source scrapers retain their oracles, and none of the four forbidden cycles or a new governed return path is introduced. No whole-crate normalization or automatic exception growth occurs. |
| R27-90I | Completion is independently demonstrated before bindings decompose. | AC27-90I: independent code review and read-only verification pass at the final candidate, repository-required gates and installed-artifact receipts pass, and the owner/requirement inventory has zero open Slice 90 items. Only then can release state mark Slice 90 complete and unblock Slice 100. Slice 150 still owns exact-final-candidate AC-037; historical security receipts are not reused as current claims. |

If any acceptance remains unmet, Slice 90 remains incomplete. A revision of
the ladder requires an explicit reviewed dependency/verification reason; batch
size or the convenience of declaring partial success is not such a reason.
