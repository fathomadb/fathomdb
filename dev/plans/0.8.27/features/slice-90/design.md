---
title: FathomDB 0.8.27 Slice 90 — runtime and root closure design
status: PLANNED
target_release: 0.8.27
---

# Slice 90 runtime and root closure design

This is prospective design, not implementation authority or a verification
receipt. Slice 85 must close first; Slice 90 remains uncommissioned. This
record and the master plan own the complete Slice 90 obligation. Slice 100
cannot start with any requirement below incomplete. No Slice 91 is needed:
the configuration correction and structural work have distinct batches and
review checkpoints within one coherent runtime-closure slice.

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
[`embedder protocol ADR`](../../../../adr/ADR-0.6.0-embedder-protocol.md),
[`bindings design`](../../../../design/bindings.md), and the applicable
[`Rust`](../../../../interfaces/rust.md),
[`Python`](../../../../interfaces/python.md), and
[`TypeScript`](../../../../interfaces/typescript.md) interfaces. The ADR
requires an engine-owned embedder pool with a configurable size (CPU-count
default) and a configurable per-call timeout (30-second default), with
late results discarded rather than forcibly cancelling a running thread.
Projection-worker count is not automatically synonymous with embedder-pool
size: the implementation design must trace query and projection calls to the
actual executor and watchdog before choosing the wiring.

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
| Existing domain owners | `trace_source_ref` and trace carriers go to `provenance`; `trace_dependency` to `dependency_trace`; `check_data_plane_integrity` to `data_plane_integrity`; `drain_embedder_events` and `Engine::usable_dense_runtime` to `embedding`; `detect_slow`, event emission, counters/profiling/subscription controls and `CounterSnapshot` to `telemetry`/existing `lifecycle` according to their actual single owner in the item map. No new public module path is introduced. |
| `index_projector` | `project_canonical_node_row`, `project_canonical_edge_row`, `IndexTargetSet`, `index_targets_for_row_kind`, `reproject_search_index_after_tokenizer_upgrade`, `search_index_tokenizer_reproject_complete`, `CanonicalNodeRow`, `canonical_node_rows`, `row_kind_from_column`, and `restore_registered_derived_projections`. Existing projection/vector helpers are called through their owners, not copied. |
| `open` maintenance | `edge_vector_prune_complete` and `prune_orphaned_edge_vectors` remain together under open maintenance because their current authority is admission-time compatibility work. Their markers and ordering move intact. |
| Existing runtime/domain owners | Projection scheduling constants belong to `projection_runtime`; embedder/watchdog defaults to resolved configuration/embedding runtime; reader constants are already pool-owned after Slice 85. Remaining root value/report types and constants move to their existing semantic owner, or receive an item-specific root-retention entry under the rule below. |
| Root | `Engine` storage, module declarations, exact public re-exports, and the small `path`/`ensure_open` core controls remain rooted. Their role is state identity/composition, not a catch-all helper home. Test modules and explicitly inventoried Slice 140 test-seam exceptions preserve qualified test identities. |

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
and named handoff item. Classify each into one final owner above, already
settled Slice 85 ownership, or an item-specific retained-root entry. Expand
grouped rows into actual symbol names in that inventory, including cfg-gated
variants, impls, statics and test carriers. For each root retention, record its
callers, invariant, cfg, why moving would be worse, and whether it is a
previously allocated Slice 140 test-seam obligation. No wildcard "remaining
helpers", unclassified production item, or general aesthetic exemption passes.
Independent design review approves this exact map before movement. No new
production deferral to Slice 100/140 is permitted; existing Slice 140 test-gate
work stays there without concealing production logic.

## Configuration correction

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
   validation/executor/watchdog; Python/PyO3 forwarding; Node/TypeScript/NAPI
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
