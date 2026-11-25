---
title: Rust Public Interface
date: 2026-11-25
target_release: 0.8.32
desc: Public Rust surface (traits, functions, types, errors) for 0.8.32
blast_radius: src/rust/crates/fathomdb; design/engine.md; design/bindings.md; design/errors.md; design/lifecycle.md
status: locked
---

# Rust Interface

This file owns Rust-visible symbol spelling and result shape. Cross-binding
parity rules remain owned by `design/bindings.md`.

This document covers the lower-level `fathomdb` facade. Rust application code
uses the dedicated `fathomdb-sdk` crate, whose Python/TypeScript-parity
contract is [`rust-sdk.md`](rust-sdk.md).

## Fresh-database open boundary

All public `Engine::open*` routes bootstrap a missing or zero-length path at
schema 39. A non-empty database is admitted only when its effective committed
`PRAGMA user_version` is exactly 39. Any other version returns
`EngineOpenError::IncompatibleSchemaVersion { seen, supported: 39 }` before
product mutation or migration events. `Engine::open*` performs no public
automatic-upgrade migration; offline upgrades from committed Schema 37 or
Schema 38 run explicitly via `fathomdb migrate` (using the `#[cfg(feature =
"operator")]` `migrate_37_to_38` and `migrate_38_to_39` entry points, not
exposed on the default `fathomdb` facade).

On Windows, database paths with a component ending in a dot or space are
refused before database creation because SQLite can resolve them to a
different filename than the engine lock. Local verbatim drive paths that
cannot be safely converted to a short drive path require SQLite 3.53.3 or the
bundled FathomDB Windows VFS correction; otherwise `Engine::open*` returns
`EngineOpenError::Io` before database creation. This applies to published Rust
crate consumers whose dependency resolver selects an uncorrected SQLite
3.53.2. The bundled FathomDB build supports long local drive paths. SQLite
WAL on a network filesystem remains unsupported.

## SQLite runtime configuration (0.8.25 Slice 79)

`fathomdb::admin::configure_runtime(RuntimeSqliteMode)` is a synchronous,
startup-only runtime control. `RuntimeSqliteMode` has exactly `Performance` and
`Diagnostics`; success returns `RuntimeConfiguration { sqlite_mode }`.
`RuntimeConfigurationError` distinguishes `TooLate`, `Conflict { requested,
effective }`, and `SqliteFailure { code }`. A first `Engine::open` without a
prior call selects `Performance`; a configuration failure is surfaced as
`EngineOpenError::RuntimeConfiguration`.

The choice applies to the loaded SQLite runtime for the process lifetime.
Performance disables SQLite global memory statistics and heap-limit enforcement;
Diagnostics retains them. A same-mode repeat succeeds even after Engine open.
A conflicting choice fails, as does a first request after SQLite was
initialized elsewhere; changing mode requires restart. In a Rust process that
also uses `rusqlite`, the first SQLite connection of any kind initializes the
runtime, so `configure_runtime` or the first `Engine::open` must come before
it; a process that has latched `TooLate` must restart. No path calls
`sqlite3_shutdown()` to force the choice. The API is not a database connection
permission or file-access boundary.

SQLite page cache (0.8.28 Slices 40 and 42). Every build bundles SQLite
3.54.1 from the crates.io `libsqlite3-sys` 0.39.1 (through `rusqlite` 0.41),
which is built without `SQLITE_ENABLE_MEMORY_MANAGEMENT`, so each SQLite
connection keeps its own page cache of SQLite's default 2,000 KiB. That holds
for the Python wheel, the npm addon, `cargo install fathomdb-cli` and every
consumer of the published Rust crates; no build flag is needed. A consumer that
links its own SQLite instead of the bundled one (for example through
`LIBSQLITE3_SYS_USE_PKG_CONFIG`) gets that library's page-cache mode.

## Per-engine configuration

`Engine::open_with_choice_and_config(path, choice, config)` accepts a public
`EngineConfig` whose fields are optional requested values: the five `u64`
fields below and, since 0.8.28 Slice 31, the three GPU memory pool settings
(§ CUDA private memory pool). Omission selects the default. `Engine::config()` returns the unchanged requested-open
snapshot; `set_slow_threshold_ms` changes the effective threshold without
rewriting that snapshot. The older open methods select default settings.

| Field | Accepted inclusive range | Default |
| --- | --- | --- |
| `scheduler_runtime_threads` | `1..=64` | `2` |
| `embedder_pool_size` | `1..=64` | `5` |
| `embedder_call_timeout_ms` | `1..=u32::MAX` | `30_000` ms |
| `provenance_row_cap` | `0..=2^53-1` | `1_000_000` rows |
| `slow_threshold_ms` | `0..=2^53-1` | `100` ms |

Zero disables provenance retention and remains accepted for the slow threshold.
Validation precedes filesystem, lock, provider and SQLite work. Invalid values
and native-width capacity overflow return
`EngineOpenError::EngineConfiguration(EngineConfigurationError)`; process-wide
SQLite mode failures retain `EngineOpenError::RuntimeConfiguration`.

Since 0.8.28 Slice 41 phase 2, `EngineConfig` also has
`query_stop_words: Option<QueryStopWords>` (§ Query stop words), and since
0.8.32 Slice 20 it carries `scope_policy: ScopePolicy` (plus builder
`EngineConfig::with_scope_policy(self, ScopePolicy) -> Self`, defaulting to
`ScopePolicy::Unscoped` for single-scope/single-agent databases). `EngineConfig`
is not `#[non_exhaustive]`, so a downstream struct literal that does not end
in `..EngineConfig::default()` stops compiling when a field is added; pre-1.0
this is accepted. `EngineConfigurationError` is `#[non_exhaustive]` since
0.8.28, gained `QueryStopWords { field, index, reason }` in 0.8.28 and
`ScopePolicy { reason: ScopePolicyError }` in 0.8.32 (`MAX_READ_SCOPES = 32`,
`MAX_SCOPE_TOKEN_BYTES = 128`); it stays `Copy`. Match it with a wildcard arm.

`scheduler_runtime_threads` starts exactly that many projection workers and
worker-owned SQLite connections. Projection admission and each dispatcher scan
are bounded by `scheduler_runtime_threads * PROJECTION_COMMIT_BATCH`, where the
commit batch is 64 rows.

With a provider, `embedder_pool_size` starts that many fixed dispatch workers
and permits `4 * workers` waiting requests; without a provider it starts none.
Projection, ordinary and frozen search, direct `embed_text`, and open-time
vector-equivalence probes use the engine dispatcher. Each invocation (including
a batch) has one absolute queue-plus-service deadline. Full admission and
queued expiry map to `EngineError::Overloaded` for direct `embed_text`; a
started failure/timeout maps to `EngineError::Embedder`, except a provider
failure of a CUDA pool kind, which keeps its typed variant (§ Errors). Hybrid search retains
same-snapshot sparse fallback, except for a query-embedding failure of a CUDA pool
kind, which search raises typed. Projection capacity waits leave durable work
pending without spending or resetting a provider-failure retry; started
failures/timeouts use the fixed 1/4/16-second ladder and may terminalize.
Close cancels pending replies; if a provider worker remains after the shared
30-second post-quiescence drain budget, explicit close returns
`EngineError::Scheduler` until that worker exits. A timely provider panic keeps
the operation's existing panic boundary; a late result/panic is discarded.

## TC-5 benchmark-only boundary (Slice 70)

`fathomdb-tc5-benchmark` is a non-published workspace executable, available
only with its `tc5-benchmark` Cargo feature. It is not re-exported by the
`fathomdb` facade and adds no Python, TypeScript, or ordinary CLI option. With
that feature, `fathomdb-engine::tc5_benchmark` exposes the documented narrow
internal ABI `VectorStageScope`, `VectorStageRequest`, `VectorStageResult`,
`VectorStageError`, and `RouteAttestation`, plus `Engine::tc5_vector_stage`.
The ABI accepts an already-created query vector and immutable manifest-derived
scope, including one manifest-pinned query-source `logical_id` exclusion applied
to candidate selection, exact rerank, and full-corpus truth. It cannot carry
compiled text, FTS, fusion, graph, CE, or ambient device settings. Its row keys
are internal executable input and must be digested before an external result
artifact is written; only aggregate recall may accompany those digests.

## Support posture

The Rust facade is the stable public Rust contract and is the
ground-truth source for engine-side type names. The Python/TypeScript governed
SDK surface parity set is tested by AC-074 (which supersedes AC-057a's
five-verb cap). Under the signed Q5 = BIND-RUST
(`ADR-0.8.0-supersede-five-verb-surface-cap`) the Rust facade is **also** bound
by AC-074; its positive-allowlist/parity pin **landed at reserved-gap Slice 27**
(see § Governed-surface contract below). Rust keeps the facade shape below
unless a successor ADR expands it.

### Temporary default-compiled test seam

`Engine::write_vector_for_test` is a `#[doc(hidden)]` public method that remains
default-compiled through Slice 90 and is scheduled for removal from default
builds by Slice 140. It is not a production vector-write contract and is the
sole explicit exception to the Slice 90 embed-dispatch rule: it may call the
provider directly and must not supply deadline, queue-bound, lock-order, or
performance evidence. Production open, projection, search, frozen-search, and
direct-embed paths have no such exception.

### Explicit engine test fixtures

The engine's doc-hidden test fixtures require `fathomdb-engine/test-hooks`
for external consumers, in both dev and release profiles. Crate unit tests
also compile them under `cfg(test)`. `debug_assertions` grants no test access.
This includes arbitrary writer SQL (`execute_for_test`, `query_i64_col_for_test`,
`query_text_col_for_test`), injected failures, reader/projection pauses,
fixture constructors and vector writes, search tuning, conversion witnesses,
and mean/clock inspection. The [complete removal inventory](../plans/0.8.27/features/slice-140/test-seam-inventory.json)
records public paths, current owners, former gates, and integration callers.
Integration targets declare `required-features`; binding unit tests use a dev
dependency with the feature. Production binding preflight
`validate_frozen_read_context_for_binding`, BM25F, fusion, reranking, and
operator diagnostics retain their production routes. This feature is absent
from shipped SDK, addon, wheel, and CLI feature lists.

The test-only `arm_evidence_before_sidecar_hook_for_test(&Engine, hook)` and
`arm_evidence_before_resolve_return_hook_for_test(&Engine, hook)` each bind their
one-shot rendezvous to that exact engine. Other engines cannot consume or
overwrite it. The sidecar hook fires post-ranking/pre-sidecar in the reader
transaction; the resolver hook fires post-resolution/pre-return in its existing
transaction. Neither holds its hook mutex across the callback. Re-arming an
already armed hook on the same engine panics; callbacks must provide their own
bounded release/cleanup. An unfired callback is dropped with its engine. Ordinary
search and shipped artifacts carry no hook state or new contract fields.

### Private CUDA pool observation

Slice130 D binding support follows the independently accepted
[ADR addendum](../adr/ADR-0.8.29-sdk-qualification-observation.md#slice130-d-addendum)
and exact passive feature boundary. The dedicated nondefault `cuda-pool-observer` feature
forwards embedder → engine → SDK/Py/NAPI without enabling CUDA, model features,
`test-hooks` or `operator`. No ordinary/shipping feature enables it.

The exact hidden Rust SDK static method is
`Engine::cuda_pool_observation_for_qualification() -> Result<String>`, forwarding
only `Ok(fathomdb_engine::Engine::cuda_pool_observation_for_qualification()?)`
under direct feature and doc-hidden attributes. It borrows no SDK engine and
works before open/after close. Core returns one JSON object with
`schema="fathomdb.cuda-pool-observation/v1"`. Fixed JSON fields serialize without
numeric precision loss; existing binding error/panic translation is retained,
while unavailable/read-failed observations remain named data.
The same JSON string is returned by native-only Python
`fathomdb._fathomdb._cuda_pool_observation_for_qualification()` and native-addon
Node `cudaPoolObservationForQualification()`. No Python/TypeScript application
wrapper or facade re-export is added; no engine/model reference is acquired.

Fields are `status`, numeric u32 `pid`/optional `device_ordinal`, optional
canonical device `uuid`, `context_id`, `pool_id`, `live_models_before`,
`live_models_after`, `used_current_bytes`, `reserved_current_bytes`,
`configured_max_size_bytes`, `release_threshold` and `reason`/`error`.
All u64/usize fields use decimal strings, except opaque process-local `pool_id`
uses hexadecimal pointer text without dereference; threshold is `"0"` or
`"max"`. Unavailable current counters are null, never zero-filled; ordinal,
configuration/threshold and live count may truthfully be zero in an unavailable
observation. Raw error/first-loss
details preserve domain, optional code and optional observed context ID;
synthetic projections retain their label and never claim a measured driver code.

Status is `private` only after successful native-token admission and checked
current-counter reads from an already-published private decision. UUID and
context identity are cached during construction/probing; opaque pool identity
comes from the existing pool owner. Loss before/during reads discards partial
counters and retains the native or explicitly synthetic first cause. Other
statuses are `not_built`, `not_decided`, `no_private_pool`, `untracked`, `lost`,
`read_error` and `ownership_changed`; these have null current counters.
`ownership_changed` means differing before/after live counts. Equal counts do
not exclude close/open ABA or other intervening activity; no epoch is added.
`no_private_pool` retains the fallback reason without discovering a default
context or inventing private identity. Counter reads do not synchronize/trim,
initialize CUDA, construct/retain ownership, allocate GPU memory or reset counters.
The [D design](../plans/0.8.29/features/slice-130/design.md#d-observational-seam-proposal)
owns checked-read order and the actual110 integration boundary. Instrumented
bytes/results remain separate from shipping lifecycle/witness evidence.

### Private Slice30 qualification observations

The nondefault `fathomdb-engine/slice30-observer` feature exposes doc-hidden
`Engine::begin_slice30_observation(max_records)` and the returned
`Slice30ObservationGuard`/batch/record types. The SDK's same-named feature forwards
only that engine capability; its method returns the guard with normal SDK error
translation and exposes no core engine. Shipped bindings/facade/CLI/MCP feature
lists do not enable it. Ordinary builds carry no observation state/clocks or
new result/configuration/counter fields.

One exact engine accepts one active session, bounded to1..=32768 records; bad
bounds/duplicate begin return `InvalidArgument`, a closed engine returns `Closing`,
and poisoned begin bookkeeping returns `Storage`. The guard is neither Clone nor
Send, owns no engine/SQL/provider lifetime, and Drop disarms only its own session.
`with_operation(id, work)` restores nested context on return/unwind; explicit weak
tokens carry the operation into actual reader work, and other engines cannot
consume it. Records identify the engine, session, operation and parent span with
monotonic nanoseconds. `take()` drains outside product clocks and reports cumulative
loss/issues. Hot append uses bounded storage/try_lock; capacity/contention drops
are explicit, never zero-cost or complete attribution. No hot I/O/subscriber call
or global collector is introduced. Diagnostic intervals are inclusive and cannot
be added as exclusive shares or subtracted across independent percentiles.

Actual G-route intervals name `api_text`, `api_hybrid` and `api_embed`, with
`query_compile`, `query_embedding_dispatch_wall`, `query_mean_serialization`,
`reader_wait` and `final_response` where entered. Dispatch wall includes queue
and service; reader wait overlaps reader work and is not an additive share.
The reader records `vector_retrieval` with nested `vector_phase1_f32_sql` and
`vector_hydration`, then `node_fts_candidates`, `edge_fts_candidates`,
`fusion_final_ranking` and optional `deferred_identity_hydration`. Binary KNN
and f32 reranking occur in one existing SQL operation; no exclusive split is
inferred. Candidate counts describe the respective existing output population.
On the qualified hybrid route with no edge FTS rows, node candidates retain
complete lightweight ranks with snapshot body/kind consistency checks. Shared
record-key fusion selects the final winners before `deferred_identity_hydration`
performs their actual body/identity reads as a separate reader-transaction child.
SQLite may still read bodies for consistency; Rust validates borrowed TEXT/UTF-8
for every match without allocating owned losing bodies. Mixed edge corpora retain
full candidate materialization and the legacy logical-ID/source-only scope of
`deferred_identity_hydration`. Skipped vector/identity sections carry untimed
`not_run`.
Snapshot authority reads are feature+active gated, explicitly observer-added on
the actual reader transaction; read failure records unavailable without authority
values or changing the product outcome.

The additional private `slice50-measurement` feature (also forwarded by the SDK)
selects only the retained eager-body/deferred-identity comparison when
`FATHOMDB_SLICE50_EAGER_TEXT_FOR_MEASUREMENT=1`; unset retains winner hydration.
It enables no general test hooks, observer or extra SQL. With a separately active
`slice30-observer` operation, one successful-reader `slice50_work` record reports
route, actual lightweight rows, vector/FTS owned-body conversions and their UTF-8
byte sums. Repeated conversions count repeatedly; borrowed text validation is
excluded. Failure, missing records or collector loss makes these counts unavailable,
not zero. These are Rust body materializations, not SQLite I/O or total allocation.
Default/shipped builds do not enable this feature or selector.

### Private Slice30 projection test observations

Only the conjunction of engine features `slice30-observer` and `test-hooks`
exposes doc-hidden
`Engine::begin_slice30_projection_observation_for_test(max_records, origin)`
and `Slice30ProjectionObservationGuard`. No SDK, facade, binding or shipped
CLI forwards this test hook. The caller supplies its whole-cell monotonic
`Instant`; capacity/errors and the single G-or-D engine slot follow the existing
bounded observer contract. Begin/drop performs no SQL, wake or provider call.
The D guard is Send+Sync, not Clone, and retains no engine/SQL/provider lifetime.
`with_operation` preserves return/unwind behavior; `take` retains bounded loss
accounting. D producers use a bounded nonblocking channel: full/disconnected
sends count once as loss, retired sessions suppress appends, and each take
receives at most the configured capacity. Only consumers share the receiver
mutex; the G collector remains unchanged.
`owner_identity()` reads the stored `(engine_id, session_id)` pair so
pre-seed pollers can tag their own observations without SQL, wake or admission;
a retired guard continues to name its original session.
D worker admission is engine/session-owned, carried weakly from actual scans
through bounded queued jobs. Untimed dequeue and actual processing/submit-wait
intervals retain actual cursors; stale/mixed admission cannot relabel work.
Further numeric/outcome/overlap contracts follow ordered runtime RED/GREEN in
[the D contract](../plans/0.8.29/features/slice-30/continuation-gd.md#d-source-fit-and-exact-private-observer-contract).
The same conjunction alone exposes doc-hidden
`Engine::slice30_projection_snapshot_for_test()` and
`Slice30ProjectionSnapshotForTest`. Field names match the resolved
`scheduler_runtime_threads`, `embedder_pool_size` and `admission_capacity`,
plus actual `active_jobs`, `active_scans`, `queued_jobs`, `in_flight_count`,
`frozen`, `pending_scan` and `stopping`. Active/queued counts name rows, not
worker threads. This is one runtime-state-lock instant; it never composes a
SQLite readiness/generation snapshot. Closed/unavailable state is Closing/
Storage, never zero. The state lock is released before serialization or any
separate readiness/SQL work. Numeric fields expose current rows and flags;
they never infer physical closure, provider engagement or atomic SQL authority.
D sessions additionally name actual `projection_commit_gate_wait`,
`projection_commit_begin`, `projection_commit_prepare`,
`projection_commit_publication`, `projection_commit_sqlite` and
`projection_commit_completion` intervals under their processing operation.
These are inclusive actual boundaries: gate acquisition, IMMEDIATE begin,
existing preparation reads, publication SQL, SQLite commit and post-commit
accumulator/event work. Outcome accounting is distinct from
these intervals; a successful earlier phase does not prove durable publication.
The untimed `projection_commit_outcome` success fact is emitted only after
SQLite commit and post-commit work. It distinguishes actual typed outcome
inputs, generation/member/existing-terminal skips, staged terminal writes and
committed success/failure counts, copying the already-hoisted generation ID.
Failed attempts retain actual staged work and copied generation after transaction/
lock cleanup. A pre-COMMIT error reports no attempted/durable publication;
a failed COMMIT leaves durable counts null/unavailable, not zero. The original
error is returned unchanged. Panic unwinding leaves the receipt unavailable,
never a fabricated zero-cost or successful outcome.
No additional SQL/descriptor reads, row ordering or mean-publication changes
are introduced. A foreground G session never activates these D phase clocks.
The foreground G guard and signed/default SDK surface remain unchanged.

### Explicit MCP admission observations

`fathomdb-mcp/test-seams` alone exposes `Seams::observe_pre_enqueue` and its
owned observation/snapshot types. One exact store/unique token observes Ready
selection after the registry mutex is released and before permit/queue accounting.
Duplicate outstanding arms panic. The async hold is bounded by the original wait
or a10s safety bound; it holds no registry mutex or Tokio worker thread. Handle
Drop/release_all releases its hold. Attempt/guard Drop, abort and unwind remove
only their matching live token; old handles cannot remove a newer arm. Bounded
snapshot history retains reached epoch/counters, actual stale rejections and
bridge-entry epochs for that attempt only. Production builds expose no observer,
state/result fields, queue policy or interruption/cancellation promise.

### Unstable binding-support items in `fathomdb-embedder`

`fathomdb-embedder` exports five `#[doc(hidden)]` items for the Node addon's
early CUDA initialisation on Jetson: `initialize_cuda_driver`,
`last_cuda_driver_init`, `cuda_driver_init_seen_by`, `CudaDriverInit` and
`CudaInitCaller`. They exist only in builds for
`cfg(all(target_os = "linux", target_arch = "aarch64"))` with `embed-cuda` or
`rerank-cuda`, so every such build compiles them (the Node addon, the Tegra
Python wheel, the CLI), but only the Node addon calls them. The probes in
those builds record their own `cuInit` outcomes as well. These items are
binding support, not an SDK surface. They are unstable, are not re-exported
by the `fathomdb` facade, are outside the AC-074 governed set, and may change
or disappear in any release without a successor ADR.

0.8.28 Slice 30 adds two more for `doctor cuda-allocator`:
`cuda_context_state(ordinal)` and its `CudaPrimaryContextState { active,
flags }`. The function reads `cuDevicePrimaryCtxGetState`, which never
retains or creates a context. It returns `None` unless the build has the
Tegra private-pool driver part (`tegra-pool` with `embed-cuda` or
`rerank-cuda` on aarch64 Linux) and the driver can be read. The same terms
apply: binding support, unstable, not re-exported by the facade.

## Governed-surface contract (AC-074, Q5 = BIND-RUST — landed Slice 27; method-level + feature-gated by Slice 27 fix-1)

This file **owns** the governed Rust-facade surface. The `fathomdb` facade is a
**different consumer contract** than the Python/TypeScript 5-verb + `read.*`
SDK: the Rust application verbs are methods on `Engine` (`open`/`write`/
`search`/`close`), and the facade's public surface is a set of re-exported
*types*, not free verbs. So the Rust allowlist is **not** the Py/TS verb set; it
is the **typed governed application surface** this file owns. Three load-bearing
properties hold (asserted by `src/rust/crates/fathomdb/tests/governed_surface.rs`,
which binds AC-074 — not a new AC id):

- **P1 — positive allowlist (`GOVERNED_SURFACE_ALLOWLIST`, 45 types):** the
  facade re-exports exactly the curated governed application surface — the
  original 17: `Engine`, `OpenedEngine`, `OpenReport`, `WriteReceipt`,
  `SearchResult`, `PreparedWrite`, `EngineError`, `EngineOpenError`, the open-path
  diagnostics (`CorruptionDetail`, `CorruptionKind`, `CorruptionLocator`,
  `OpenStage`, `RecoveryHint`), the retrieval soft-fallback shapes (`SoftFallback`,
  `SoftFallbackBranch`), and the instrumentation handles (`CounterSnapshot`,
  `Subscription`) — plus the additive groups: Slice 20 (G5/G6) graph-traversal
  types (`TraversalDirection`, `NodeRecord`, `SearchExpandResult`, `SearchFilter`),
  Slice 35 (G4) filter-grammar types (`Predicate`, `ScalarValue`, `ComparisonOp`),
  Slice 15 (G11) BYO-LLM ingest types (`ExtractDocument`,
  `IngestWithExtractorReceipt`), and 0.8.8 Slice 5 (EXP-OBS) explain-sidecar types
  (`Explanation`, `QueryTrace`, `PerHitExplain`). 0.8.20 Slice 10b
  (R-20-RV / R-20-NV) adds the read-view / node-validity types (`ReadView`,
  `BoundaryCrossing`) — **HITL-SIGNED 2026-07-29 (steward `seq-157`)**, recorded
  in `src/conformance/governed-surface-allowlist.json`. `ReadView` is threaded as a
  PARAMETER on the five existing read verbs (`read_get`, `read_get_many`,
  `read_list`, `read_list_filter`, `graph_neighbors`) rather than shipped as five
  `*_with_view` sibling verbs, which is what keeps the delta at two TYPES and zero
  new verb names; `ReadView::default()` is the strict view and reproduces the
  pre-slice read semantics exactly. 0.8.20 Slices 5c/5d (R-20-E3 / R-20-E4) add
  the erasure types `SourceId` and `ExciseReport` (the latter moved out of the
  operator-gated block — it is `erase_source`'s return type) — **HITL-SIGNED
  2026-07-29 (steward `seq-157`)**. Agent-access Block 1 S3 (EP-03) adds the
  search degradation report types `ArmDegradation` and `DegradationReason`
  — **HITL-SIGNED 2026-10-19**. 0.8.29 Slice70 adds exactly the six retrieval
  carriers `SearchHit`, `IdSpace`, `IdSpaceKind`, `SnippetOptions`, `SnippetAnchor`
  and `SnippetV1`. Each listed type resolves through the facade
  at compile time (`type_name::<…>()`). The facade also `pub use`s additional
  explicitly documented additive groups that are **not members of this
  historical const**; do not infer the full re-export surface from its count.
  The Slice 15d (R-20-PR)
  projection-registry types `ProjectionSpec`, `ProjectionRole` and
  `ProjectionDelta` are **HITL-SIGNED 2026-07-29 (steward `seq-157`)** — the
  signed delta names exactly those three. Their sub-object types
  `ProjectionFts` and `ProjectionVector` are not members of that `seq-157`
  governed-command signature. `DenseReadiness`'s closed vocabulary is instead
  **HITL-SIGNED 2026-08-07 (steward `seq-246`)** by the F5/C1 record and its
  governed-surface signature. Caller input is accept-inert; Slice 21 runtime
  selection makes `read_projections` select `Unavailable` for a no-usable-
  runtime session.
  See § "Projection registry" below. The recovery /
  integrity / dump operator-seam report types in § "Recovery / operator seam
  re-exports" are deliberately **excluded** from this allowlist — they are
  CLI-only ergonomic symbols (the Rust analogue of "recovery is CLI-only, not an
  SDK verb"), not governed application surface.

- **P2 — parity-in-intent (NOT membership-identity):** the Rust governed surface
  is posture-consistent with the Py/TS governed surface (a governed allowlist,
  recovery-denylist-absent, typed / no-raw-SQL) but is a different consumer
  contract — a type set, not a verb set — so it is **not** asserted
  membership-equal to the Py/TS verb allowlist. The one genuinely shared element
  is the recovery denylist, declared once in
  `src/conformance/governed-surface-allowlist.json` (`recovery_denylist`); the
  Rust test pins the same five names.

- **P3 — recovery-denylist absence:** no governed-surface symbol *is* a recovery
  verb in `{recover, restore, repair, fix, rebuild}` (exact, case-insensitive —
  not substring, so the typed `RecoveryHint` hint is correctly not flagged). The
  **canonical** denylist enforcement remains the **byte-frozen**
  `tests/no_recovery_surface.rs`; `governed_surface.rs` adds the *positive*
  allowlist half + an allowlist-scope denylist check.

Rust has no runtime symbol-table introspection (no `dir(module)`), so — exactly
like `no_recovery_surface.rs` / `reexports.rs` — the type-level pin is a
compile-time resolves-check plus this source-inspection-documented contract. See
`dev/design/slice-27-rust-allowlist-design.md`.

### Method-level boundary: default surface vs the `operator` feature (Slice 27 fix-1)

The Slice 27 type-only audit missed that the facade re-exports the engine's
`Engine` **wholesale**, so its inherent **methods** — including
`rebuild_projections`/`rebuild_vec0` (recovery-denylist names) and the
then-debug-only raw-SQL `execute_for_test` — were reachable. Per the signed Option B
(codex [P1], HITL 2026-06-05) the **operator/recovery seam is feature-gated**:

- **Default `fathomdb` facade (operator OFF)** — the governed runtime surface:
  the 29 governed types + the application methods `Engine::open`/`write`/`search`/
  `search_explained`/`close` (+ the engine-attached instrumentation/control methods). It exposes
  **no method whose name is in `{recover, restore, repair, fix, rebuild}`** and
  **no raw-SQL method**. This is enforced at the **method** level by
  `compile_fail` doctests in `fathomdb/src/lib.rs`
  (`governed_surface_method_absence_proof`, default build;
  `release_surface_raw_sql_absence_proof`, release build) — the only mechanism
  that can assert a method does *not* resolve.
- **`operator` feature (ON — `fathomdb-cli` enables it)** — un-gates the
  operator/recovery methods (`rebuild_*`, `excise_source`, `dump_*`,
  `trace_source_ref`, `truncate_wal`, `verify_embedder`, `check_integrity`,
  `pending_erasures`, `safe_export`, `recompute_mean`) and the operator-seam
  re-exports below. The
  CLI (`fathomdb recover`/`doctor`) is the operator substrate. **Gating, not
  deletion**: engine behavior is byte-identical with the feature on.

So the recovery-denylist + no-raw-SQL guarantees hold at the **method** level on
the default governed surface, while the CLI retains the seam via the feature.
See `dev/design/slice-27-fix1-operator-gate-design.md`.

## Public surface

Rust exposes:

- `Engine::open(...) -> Result<OpenedEngine, EngineOpenError>`
- `Engine::write(...) -> Result<WriteReceipt, EngineError>`
- `Engine::write_with_options(&self, batch: &[PreparedWrite], options: &WriteOptions) -> Result<WriteReceipt, EngineError>`
- `Engine::search(...) -> Result<SearchResult, EngineError>` — defaults to 10 ranked hits.
- `Engine::search_with_limit(query, limit) -> Result<SearchResult, EngineError>` — `limit` is
  `1..=100`; out-of-range values return `EngineError::InvalidArgument`.
- `Engine::search_explained(...) -> Result<SearchResult, EngineError>` — 0.8.8
  EXP-OBS: same retrieval as `search_reranked`, additionally returning the opt-in
  `Explanation` sidecar (`SearchResult.explanation`); default paths are unchanged.
- `Engine::search_text_only(&self, query: &str) -> Result<SearchResult, EngineError>` — default 10.
- `Engine::search_text_only_with_limit(query, limit) -> Result<SearchResult, EngineError>`
- `Engine::close(...) -> Result<(), EngineError>`

`close` stops admission, waits for active database work and releases every
SQLite owner before beginning a shared 30-second embed-worker drain. It returns
`EngineError::Scheduler` if a provider call survives that deadline; worker
accounting remains live, and a later `close` succeeds after the provider exits.
Concurrent or repeated calls and Drop do not renew the drain deadline. Active
database work can extend total close time beyond 30 seconds.
After a successful drain, `close` releases the engine's embedder reference even
if the `Engine` object remains alive. A caller-held embedder reference remains
caller-owned. A timed-out drain retains the provider until its worker exits and
a later `close` completes.

### Doc-hidden engine fusion helpers (0.8.29)

`fathomdb_engine::FusionHit { hit: SearchHit, ownership: FusionOwnership }`
retains source-derived snapshot-local ownership through ranking. Ownership is
`CanonicalNode`, `CanonicalEdge` or `OwnerlessNodeFts { rowid: i64 }`; canonical
keys use the enclosed hit's cursor. These are internal ranking carriers, absent
from SDK/wire search results and unrelated to stable IdSpace.

`fuse_rrf(Vec<FusionHit>, Vec<FusionHit>)` and `fuse_three_arms` with three such
arms return `Result<Vec<FusionHit>, EngineError>`. Callers supply ownership
explicitly. Conflicting bodies for one record return `InvalidArgument`; kind and
source remain first-representative fields and may legitimately differ across
arms. The helpers access no database and infer no ownership from body or branch.
There is no bare-SearchHit compatibility overload. Public reweight/rerank helpers
retain their existing SearchHit signatures and share the same arithmetic with
internal keyed carriers.

### Direct text-only result-prefix contract (0.8.22 Slice 23)

`search_text_only` and its explicit-limit/read-view forms do not embed, use
vector retrieval, CE reranking, or graph expansion. Matching node- and
edge-body FTS candidates enter one text arm, whose stored records are deterministically
deduplicated and ranked before the returned limit is applied. The node candidate
collection is fixed at 100 for every accepted caller limit.

For one immutable selection, query, and effective validity time, the complete
ordered result at `small` is the prefix of the result at `large` for
`1 <= small <= large <= 100`. Calls compared through `ReadView` must use the
same explicit `valid_as_of`; `None` resolves separately at each call. This
direct-FTS contract does not apply to hybrid `search*` APIs.

### Read verbs

The canonical-row reads take a trailing `&ReadView` (`ReadView::default()` is
the strict view); the op-store reads do not — they page an append-only log that
has no existence or validity axis.

- `Engine::read_get`, `Engine::read_get_many` — `&ReadView`
- `Engine::read_list`, `Engine::read_list_filter` — `&ReadView`
- `Engine::read_collection`, `Engine::read_mutations` — `(collection, after_id,
  limit)`, no `ReadView`
- `Engine::crossed_boundary_since(&self, since: i64, view: &ReadView) ->
  Result<Vec<BoundaryCrossing>, EngineError>` — 0.8.20 Slice 10b (R-20-NV):
  which nodes entered or left validity since instant `since` (INTEGER epoch
  seconds). The one net-new command in that delta, because no existing read
  verb can express the question.

### Graph verbs

- `Engine::graph_neighbors(&self, ..., view: &ReadView) -> Result<Vec<NodeRecord>, EngineError>`
- `Engine::search_expand(&self, query: &str, filter: Option<SearchFilter>, depth: u32, view: Option<ReadView>) ->
  Result<SearchExpandResult, EngineError>`
- `Engine::search_expand_with_limit(query, filter, depth, limit, view: Option<ReadView>) -> Result<SearchExpandResult,
  EngineError>` — limits only initial ranked `search_hits`; expansion remains capped at 50 per root.

### Lifecycle + erasure verbs (0.8.19 Slice 10 / 0.8.20 Slice 5d)

**Governed, allowlisted, and present in all three bindings.** They are not
recovery verbs — none carries a REQ-054 denylist name — and they are on the
DEFAULT facade, not behind the `operator` feature.

- `Engine::transition(&self, logical_id: &str, to_state: LifecycleState, reason:
  Option<String>) -> Result<(), EngineError>` — move a GOVERNED node between
  existence states per the engine-enforced legal-transition table
  (`pending→active` promote, `pending→deleted` REJECTED, `active→deleted`
  soft-delete, `deleted→active` undelete). Promote/undelete CLEAR `reason`;
  reject/soft-delete SET it. `reason` is advisory and never engine-interpreted.
  Keys on the bare `logical_id` — the `l:` id space ONLY; any other space is
  `EngineError::NotLifecycleAddressable`. An illegal move is
  `EngineError::IllegalTransition { from_state, to_state, legal }`.
- `Engine::purge(&self, logical_id: &str) -> Result<(), EngineError>` —
  irreversible hard-erase of a governed node across every row-owned target (all
  versions, FTS/vector shadows, touching edges cascade-removed). DELETED-FIRST
  (legal only from `deleted`) and IDEMPOTENT. A SEPARATE verb from `transition`.
  **There is no restore counterpart on any surface.** A logical id remains
  purgeable after a correction supersedes one of its revisions: completed
  direct-dependent closure rows do not block receipt redaction.
- `Engine::erase_source(&self, source_id: &str) -> Result<ExciseReport, EngineError>`
  — 0.8.20 Slice 5d (R-20-E4): erase every canonical row carrying `source_id`
  plus its row-owned projections, then finish the erasure AT REST (telemetry
  redaction + WAL truncating checkpoint). The COMPANION to `purge`: `purge`
  addresses a governed node by `logical_id`, `erase_source` addresses
  ANONYMOUS content (rows with no `logical_id`) by provenance, which `purge`
  cannot reach at all. Idempotent. Refuses an empty / whitespace-only /
  reserved (`_`-prefixed) id with `EngineError::WriteValidation`; the reserved
  namespace stays reachable only through the CLI `--excise-source` seam.
  Reports `EngineError::ErasureIncomplete` rather than success when the
  at-rest step could not complete. A source remains erasable after a
  correction supersedes it: completed direct-dependent closure rows do not
  block receipt redaction, and the original and replacement source buckets
  may be erased in either order. Report counts remain exact for only the
  requested source bucket; no cross-bucket expansion or new report field is
  implied.

### Projection-registry verbs

- `Engine::configure_projections(...) -> Result<ProjectionDelta, EngineError>`
- `Engine::read_projections() -> Result<Vec<ProjectionSpec>, EngineError>`
- `Engine::read_projection_status() -> Result<ProjectionRuntimeStatus, EngineError>`
- `Engine::read_embedding_readiness() -> Result<EmbeddingReadiness, EngineError>`
- `Engine::read_projection_generation_status() ->
  Result<ProjectionGenerationStatusV1, EngineError>`
- `Engine::read_mutation_projection_status(MutationProjectionStatusRequestV1)
  -> Result<MutationProjectionStatusV1, EngineError>`

See § "Projection registry" below.

`OpenedEngine` contains:

- `engine`
- `report`

`report` is the `OpenReport` owned by `design/engine.md`.

### `OpenReport::embedder_gpu_allocation_witness` (0.8.23 Slice 80.6)

`OpenReport::embedder_gpu_allocation_witness` is
`Option<fathomdb_embedder::GpuAllocationWitness>`. Slice 95 updates current
records to `fathomdb.tegra-gpu-allocation-witness/v2`, measured in the installed
artifact's own process. The contract is
`ADR-0.8.28-attributable-cuda-allocation-witness.md`; historical v1 records remain
historical and are refused by current verifiers.

`None` means no witness was taken. `FATHOMDB_GPU_ALLOCATION_WITNESS=1`
(or `true`) requests it for a CUDA-selected default embedder. An unavailable or
failed requested default-embedder witness fails `Engine::open` with `EngineOpenError::Embedder`
and a named witness tag; it never degrades to `None`. Unrecognized opt-in values
are errors on that default-embedder path. Normal opens do not perform this measurement. Witnessing
loads a second model and performs the selected method's deliberate control.

Common fields are `process_id`, `device_integrated`, requested/actual ordinals,
UUID/name/compute capability, `free_before_bytes`, `free_after_bytes`,
`total_bytes`, signed `delta_bytes`, `delta_floor_bytes`, `embedded_vector_dim`
and `measurement`. The PID is the native process's own PID in its namespace.
The common delta is always truthful global before-minus-after; it can be zero
or negative on a successful private witness. Identity, process binding, a
64 MiB minimum model floor and one actual 384-dimension forward remain required.

`GpuAllocationMeasurement` is a finite enum, also named by `fathomdb-sdk`:

- `GlobalFree` is selected only for an actual discrete/non-private context.
  Its nested canonical record has `kind="global_free"`, the bounded raw
  `control_allocation_request_bytes`, `control_block_count`,
  `control_free_before_bytes`, `control_free_after_bytes`,
  `control_delta_bytes` and `sole_gpu_consumer_precondition`. Model acceptance
  uses the common global decrement. The selected device must have no competing
  GPU consumer during the observation; external PID/UUID evidence remains required.
- `PrivatePoolUsed` is selected only for an actual integrated/private context.
  Its nested record has `kind="private_pool_used"`, lossless decimal-string
  `context_id`, opaque-string `pool_identity`, `used_before_bytes`,
  `used_after_bytes`, signed `delta_bytes`, `control_request_bytes`,
  `control_before_bytes`, `control_live_bytes`, `control_released_bytes` and
  `pool_exclusivity_precondition`. Acceptance uses USED after-minus-before.
  Checked completion precedes the initial baseline and post-load sample. The
  model remains live through the forward and one touched 64 MiB same-pool
  control; release must restore the exact post-forward USED baseline. No competing
  allocation/free may occur in that pool during model/control observation.

Selection precedes samples and never depends on their results. Integrated
non-private and unexpected discrete/private contexts return the named
`unsupported_allocation_method` refusal. Neither allocator policy nor ordinary
model loading changes. Variants have exact keys, without irrelevant null/fake
fields; RESERVED/high-water/global diagnostics cannot replace private USED proof.

The private `FATHOMDB_GPU_POOL_DIAGNOSTIC=1` opt-in adds a separate
`fathomdb.tegra-gpu-pool-diagnostic/v2` stderr record during an already requested
witness, preserving its original result including errors. Other values leave
this developer diagnostic off. Its current partial-observation scope and pinned
historical reproduction are documented in
[`dev/experiments/tegra-pool-diagnostic/README.md`](../experiments/tegra-pool-diagnostic/README.md).

### `OpenReport::embedder_device_resolution` (0.8.23 Slice 70)

`OpenReport::embedder_device_resolution` is the immutable strict CPU/CUDA
selection consumed by the embedder constructor. It is present for the built-in
default embedder and `None` when no provider is configured. It preserves the exact requested policy
(`auto`, `cpu`, or `cuda:N`), artifact CUDA capability, effective CPU/CUDA
backend, the ordered process-visible CUDA inventory (`visible_ordinal`, UUID,
name, compute capability), optional selected UUID, and an automatic-fallback
reason. Ordinals are `CUDA_VISIBLE_DEVICES`-relative; physical host ordinals
are never inferred. CPU-effective automatic outcomes retain the observed
inventory. The additive fields are
`visible_cuda_devices: Vec<CudaVisibleDevice>` (each with `visible_ordinal`,
`uuid`, `name`, and `compute_capability`) and
`selected_cuda_uuid: Option<String>`; a present selected UUID names exactly one
inventory member. A forced-CUDA failure remains `EngineOpenError::EmbedDevicePolicy`,
not a fabricated CPU report.

`DeviceResolution` is the normal open-time result. `DoctorGpuDiagnosticResult`
is a CLI-only result and is deliberately distinct: a
`CudaProbeError::ProbeFailed` can be represented as automatic CPU open evidence,
but `doctor gpu` maps it to `probe_failed` and exit `70`. Rust SDK consumers do
not receive a doctor API or configuration setter.

### Built-in provider admission and cached verification

Production engine admission supports only the pinned built-in BGE/Mean provider
or explicit no-provider access. User-specified providers are unsupported before
1.x and are not promised for 1.0. Device selection is separate from provider
selection. Low-level embedder traits, standalone/test/evaluation constructors and
ONNX evaluation support do not grant production engine admission; feature
unification does not restore caller-provider injection. Standalone CLS embedding
remains a separate supported helper and does not select the engine's Mean provider.
Loader construction, supported device policy, ambient precision behavior and
mean pin/recompute/recovery ordering are unchanged.

New projection declarations accept only omitted/null embedder selectors. Every
string is rejected before mutation, including empty strings, `default` and model
names. Historical stored strings remain verbatim on reads with no provider;
re-applying such a declaration is refused rather than silently selecting BGE.
Vector configuration carries no provider identity string.

The existing checker gate remains: an effective registered vector kind or durable
prospective dense declaration engages verification; a retained baseline alone
introduces no new probe gate. No-provider and ordinary vector-less opens use zero
probes. Preserve the 45-probe fixture, structural validation, identity/dimension
rules, P1 zero binary flips and P2 maximum uncentered L2 `1e-5`. Ordinary clean
first population performs 90 embeddings (45 references plus 45 confirmation), a
fresh check of an existing baseline performs 45, and a valid cached open performs
zero. Rejected first population does not publish its new references.

The persistent TC-68 marker remains authoritative for cached acceptance. The
fingerprint recipe and inputs remain unchanged: stored identity, pinned mean,
probe fixture, divergence floors and stored reference baseline. Missing, stale or
malformed markers retain existing validation/recheck behavior; a marker-write
failure remains a successful verified open with possible rechecking later. A
process restart, MCP idle close, or A/B/A reconstruction does not force a fresh
check. Cached acceptance is **not fresh verification in this process**.

A valid marker can hide same-identity backend, pooling, device or library drift
because those runtime facts are not fingerprint inputs. Closing provider injection
does not remove that residual. Finite probes establish neither universal numerical
equivalence nor historical vector provenance; stored references and the marker
are not authenticated. This change adds no process cache, new fingerprint inputs,
per-call numerical check, runtime descriptor or cache revocation policy. The
existing cache rationale and threat model remain in
[TC-68](../design/0.8.20-tc68-equivalence-probe-fingerprint-cache.md).

An actual checker refusal fails open and returns no engine or successful report.
There is no automatic provider-free retry. A caller deliberately selects the
existing no-provider option for canonical/text/lifecycle/erasure access, subject
to normal integrity checks and deferred dense-work rules. Existing historical
profiles are validated as metadata without pretending they use BGE. No-provider
mean recovery and explicit recompute keep their existing behavior and make no
numerical compatibility claim. No-provider access is ordinary mutable access,
not forensic mode or a repaired dense backend. Existing operator rebuild behavior
can invalidate dense state without a usable provider; this change neither fixes
nor certifies that path safe and adds no diagnostic CLI.

## Engine-attached instrumentation / control methods

These are public instance methods, not extra top-level SDK verbs:

- `Engine::drain(timeout_ms: u64) -> Result<(), EngineError>`
- `Engine::counters() -> CounterSnapshot`
- `Engine::set_profiling(enabled: bool) -> Result<(), EngineError>`
- `Engine::set_slow_threshold_ms(value: u64) -> Result<(), EngineError>`
- `Engine::subscribe(&self, subscriber: Arc<dyn lifecycle::Subscriber>) -> Subscription`

`drain` is a bounded completion surface for post-commit projection work. It
returns `Ok(())` when the engine-owned background projection queue reaches a
quiescent state before `timeout_ms`, and returns a typed runtime error when the
timeout elapses first.

`subscribe` owns host-subscriber attachment. The current method takes a
subscriber only; it has no heartbeat-cadence option and does not itself
produce operation heartbeats. The payload semantics remain owned by
`design/lifecycle.md` and `design/migrations.md`.

Cleanup-scheduler debt-presence probes are excluded from subscriber profile and
slow-statement callbacks. Foreground statements and actual cleanup work retain
their configured instrumentation; see the profiling contract in `design/lifecycle.md`.

Each Engine owns one joined physical-cleanup thread in addition to its eight
reader workers, projection dispatcher and configured projection workers. Closing
the Engine joins that cleanup thread; it adds no SQLite connection. Installed
binding topology checks therefore expect ten fixed threads plus the configured
projection-worker count, and exact baseline restoration after close.

## Companion embedder contract

The Rust workspace also exposes the semver-stable companion crate
`fathomdb-embedder-api` for engine-owned embedder dispatch:

- `Embedder`
- `EmbedderIdentity { name, revision, dimension }`
- `EmbedderError`

## Caller-visible data shapes

- `WriteReceipt` exposes `cursor` (the batch high-water `write_cursor`),
  `row_cursors` (per-row, 1:1 with the batch) and `dangling_edge_endpoints`
  (`0` on every commit under `EndpointValidationMode::Strict` / default
  `Engine::write`; truthful `0`/`1`/`2` per surviving active edge under opt-in
  `EndpointValidationMode::BulkNonStrict` via `Engine::write_with_options`)
- `SearchResult` exposes `projection_cursor`, which names the terminal
  projection-visible point for the search snapshot
- hybrid fallback, when present, exposes a typed branch enum whose values are
  owned by `design/retrieval.md`
- `SearchResult.degradations: Vec<ArmDegradation>` lists requested arms that
  did not run for a reported bounded cause. An empty list means no such cause
  was reported; it does not prove every requested arm ran. `ArmDegradation
  { arm: SoftFallbackBranch, reason: DegradationReason }` and
  `DegradationReason` are `#[non_exhaustive]`; `ArmDegradation::new` builds
  one and `DegradationReason::as_str` gives the wire word. `EmbedderAbsent`
  (`embedder_absent`) means no numerical runtime was configured for this query.
  `EmbedderTimeout` (`embedder_timeout`) means the query started in its provider
  but exceeded the dispatcher deadline; its worker retains its slot until return.
  A provider-returned Timeout/Failed or invalid vector keeps the unlabeled sparse fallback.
  `EmbedderOverloaded` (`embedder_overloaded`) means the query embedding
  was refused because the embed queue was full (`Saturated`) or expired in
  that queue (`QueuedExpired`). The search then runs on its other arms and
  reports `[{Vector, EmbedderOverloaded}]`. Every hybrid read that embeds a
  query reports it: `search*`, `search_frozen`, `search_with_evidence` (on
  `EvidenceSearchResultV1.search_result`), and `search_expand` /
  `search_expand_frozen` (on `SearchExpandResult.degradations`). Results,
  ranking, `soft_fallback` and `explanation` are unchanged. Other embed
  failures still leave the list empty. Actual engine shutdown returns typed
  `EngineError::Closing` from the query helper before reader dispatch; unrelated
  dispatch cancellation retains its prior sparse fallback. `embed_text` keeps returning
  `EngineError::Overloaded`. Both types are re-exported by `fathomdb` and
  `fathomdb-sdk` (EP-03)
- counter/profile/stress payload shapes are owned by `design/lifecycle.md`

### `SearchHit.id` is `IdSpace` (C-2, 0.8.19 / TC-8)

`SearchHit::id` is **`IdSpace`**, not the pre-0.8.19 `write_cursor: u64`. This
is the single largest consumer-visible break in the 0.8.9 → 0.8.20 span, and it
is part of the HITL-SIGNED 0.8.19 Slice-10 delta (the allowlist `_comment`
records the `SearchHit.id` `u64 → IdSpace` retype together with the
`IdSpace`/`IdSpaceKind` types).

- `IdSpace { space: IdSpaceKind, value: String }` — `value` is the BARE
  (prefix-stripped) id; `IdSpace::to_prefixed()` (`{prefix}{value}`) reproduces
  the pre-swap `stable_id` byte-for-byte, which is what made the swap a
  real-gold-keying no-op.
- Governed hits are `l:`, doc-seeded hits `h:`, synthetic passages `p:`.
- **This is the PERMANENT caller-facing identity, not an interim carrier.** The
  older "interim identity carrier, swaps to `logical_id` at G0" framing is
  superseded and must not be restated.
- `SearchHit::write_cursor: u64` survives as ENGINE-INTERNAL positional
  book-keeping (vector rowid mapping, `state='active'` lookups, RRF reweight
  keys, telemetry keying, `search_expand` re-resolution). It is reassigned on
  re-projection, is not cross-session stable, and **the Python/TypeScript
  bindings do NOT surface it**.
- Only the `logical` space is lifecycle-addressable: `transition` / `purge`
  refuse any other space with `EngineError::NotLifecycleAddressable`.

The `fathomdb` facade and `fathomdb-sdk` both re-export `SearchHit`, `IdSpace`,
`IdSpaceKind`, `SnippetOptions`, `SnippetAnchor` and `SnippetV1`. A facade consumer
can name actual hit and snippet field types, match id spaces and anchors, and
pass snippet options through existing search methods. These are six type exports;
no helper function or new operation is added to the facade.

`SearchHit::source_id: Option<String>` carries the hit's source-document
provenance — the identifier `Engine::erase_source` consumes — and since TC-31
(0.8.20) it is populated on EVERY hit path, not just the graph arm.

## Caller-supplied write shapes

`PreparedWrite` is the caller-supplied input to `Engine::write` and is itself
governed surface (§ P1), so adding a variant field changes what every binding
must accept. It is `#[non_exhaustive]`.

### Option and result structs are `#[non_exhaustive]` (agent-access Block 1)

Five structs are `#[non_exhaustive]`: `fathomdb_sdk::SearchOptions`,
`fathomdb_sdk::TextSearchOptions`, `fathomdb_sdk::FrozenSearchOptions`
([`rust-sdk.md`](rust-sdk.md)), and `SnippetOptions` and
`SearchExpandResult` from `fathomdb-engine`. `SearchExpandResult` is
re-exported by this facade and by `fathomdb-sdk`; `SnippetOptions` by
`fathomdb-sdk`.

- Another crate cannot build any of them with a struct expression. A full
  literal and `..Default::default()` both fail with `E0639`. A new field is
  then not a source break.
- Option structs follow the `SearchFilter` form: start from `default()` and
  assign the public fields.

  ```rust
  let mut snippet = fathomdb_sdk::SnippetOptions::default();
  snippet.max_bytes = 300;
  let mut options = fathomdb_sdk::SearchOptions::default();
  options.limit = 20;
  options.snippet = Some(snippet);
  ```

- `SearchExpandResult` is a result. Callers read its fields; only the engine
  builds it.
- Adding the attribute is itself a breaking change (TC-103, todos ledger
  `seq-178`). It lands with Block 1, which already breaks literal
  construction of the SDK search options by adding `snippet`, so a caller
  upgrading past Block 1 meets no further break.
- Python and TypeScript build these structs in Rust; their surface does not
  change.

### Versioned identity and provenance (0.8.25 Slice 15)

`PreparedWrite::ProvenancedNode(ProvenancedNodeV1)` and
`PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1)` add complete provenance
without changing the fields of the legacy `Node` and `Edge` variants. The
versioned shapes carry `WriteProvenanceV1`; canonical provenance is valid only
for nodes, while derived provenance is valid for nodes and edges. Caller IDs
use `[A-Za-z0-9][A-Za-z0-9._:-]{0,127}` and may not use the `_fdb:` namespace.

The public closed types are `ArtifactRevisionId`, `SourceRevisionId`,
`SourceVersionId`, `CanonicalHash`, `SourceLocator`,
`ProvenanceCompleteness`, `WriteProvenanceV1`, `ProvenancedNodeV1`, and
`ProvenancedEdgeV1`. Provenance refusals are
`EngineError::Provenance(ProvenanceError)`, whose stable code is
`"provenance"` and whose closed reason is a `ProvenanceErrorReason` plus a
JSON-pointer `field_path`. Existing `WriteReceipt`, `NodeRecord`, provider,
and search-hit shapes are unchanged.

### Source dependencies (0.8.25 Slice 20)

`DependencyId` uses the same caller-ID grammar as Slice 15. The closed request
types are `SourceDependencyRegistrationV1`, `DependencySourceLookupV1`, and
`DependencyDerivedLookupV1`; constructors accept raw strings and validate them
before any database access. `Engine::register_source_dependency` creates or
exactly replays one immutable relation,
`Engine::dependencies_for_source` returns at most 100 rows ordered by derived
revision then dependency ID, and `Engine::dependency_for_derived` returns zero
or one row. Results are `SourceDependencyV1` and `DependencyListV1`.

The registration generation is a `u64` distinct from the canonical write
cursor and projection terminals. One transaction that changes dependency
membership advances it once; replay, reads, refusals, and no-op erasure do not.
`EngineError::Dependency(DependencyError)` has stable code `"DependencyError"`, a
closed `DependencyErrorReason`, and an RFC 6901 `field_path`. Persisted chain or
generation corruption fails closed as `EngineError::Storage`.

### Atomic actuation (0.8.25 Slice 25)

`Engine::actuate(ActuationBatchV1)` applies 1–128 ordered, caller-decided
operations in one transaction. The closed operation set is canonical-node put,
derived-node put, derived-edge put, source-dependency registration, and
revision-pinned lifecycle transition. A derived edge is admitted only when both
endpoints are active in the complete prospective batch state, independent of
operation position. FathomDB validates and actuates the decision; it does not
extract, merge, infer contradiction, or choose truth.

An admitted request returns `ActuationReceiptV1` with a terminal `committed` or
`refused` outcome. Exact `operation_id` and canonical-digest replay returns the
stored receipt; a different digest or an erased receipt raises
`EngineError::Actuation(ActuationError)`. Boundaries, dependency generations,
and projection cursors are `u64`. Source erasure or purge redacts every matching
receipt to an opaque, permanently reserved operation-ID tombstone.

`SourceId` retains its existing closed grammar, including embedded NUL; Python
and TypeScript preserve every Engine-valid source identity exactly.

### Dependency lifecycle closure (0.8.25 Slice 30)

Source supersession, soft deletion, purge, and source erasure atomically close
every direct Slice 20 dependent. A nonterminal closure is an unconditional
eligibility and projection-publication barrier under every `ReadView`.
Physical closure shares the existing telemetry/WAL completion boundary and is
serviced by exact retry or autonomous idle/reopen maintenance. During sanitation,
only certified unrelated plain legacy node/edge writes are admitted; other
mutations and projection publication remain fenced. Finalization persists a
global mutation fence until physical completion. See the
[successor decision](../adr/ADR-0.8.28-resumable-physical-cleanup.md).

`Engine::read_dependency_closure(ClosureLookupV1)` is the sole public closure
read. It returns `Option<ClosureStatusV1>` for one opaque Engine-minted
`ClosureOperationId`; there is no public list, resume, or repair API. The
closed status includes `ClosureRootV1`, `ClosureCauseV1`, `ClosurePhaseV1`,
admission boundaries, affected count, optional blocker, and optional
`ClosureProofV1`. Counts and boundaries are `u64`; valid absent IDs return
`None` without disclosure. Invalid request shape or identity returns
`EngineError::DependencyClosure(DependencyClosureError)` with a closed reason
and RFC 6901 `field_path`.

The old actuation refusal spelling `dependency_closure_required` is reserved
for wire compatibility but is no longer emitted after this slice.

### `source_id` is STRUCTURALLY MANDATORY (0.8.20 Slice 5c, R-20-E3)

`PreparedWrite::Node` and `PreparedWrite::Edge` both carry
**`source_id: SourceId`** — a newtype, not `Option<String>`. The change is a
TYPE change rather than a validation check on purpose: `Engine::erase_source`
addresses rows BY `source_id`, so a row written without one is reachable by no
erasure call, and Rust can make that state **inexpressible**. An
un-provenanced write is a COMPILE error for facade consumers (proven by
`fathomdb/tests/ui/`), not a runtime rejection.

- `SourceId::new(id: impl Into<String>) -> Result<SourceId, EngineError>` is
  the ONLY public constructor. It refuses an empty / whitespace-only id and any
  id in the engine's reserved `_`-prefixed namespace
  (`SourceId::ENGINE_PREFIX` = `"_engine:"`, `SourceId::LEGACY_PRE_0_8_20` =
  `"_legacy:pre-0.8.20"`), both with `EngineError::WriteValidation`.
- `SourceId` is re-exported on the facade and is part of the HITL-SIGNED
  Slice-5d allowlist delta. It MUST stay re-exported: without the constructor a
  facade consumer could not perform a canonical write at all.
- The bindings have no such type system at the boundary, so Python raises and
  TypeScript throws `WriteValidation` for a missing/empty/reserved id — the
  same rule, enforced at the binding.
- Policy note carried to the publish-facing docs: **`source_id` must not
  contain personal data.** It is echoed on every `SearchHit` and recorded in
  the retention-EXEMPT erasure-audit row, so it outlives the rows it names.

### `PreparedWrite::Node` — world-time validity window (0.8.20 Slice 15b, TC-34)

`PreparedWrite::Node` carries two optional validity bounds:

- `valid_from: Option<i64>` — INCLUSIVE lower bound, INTEGER epoch **seconds**
  UTC. `None` lands SQL NULL = unbounded below.
- `valid_until: Option<i64>` — EXCLUSIVE upper bound, same units. `None` lands
  SQL NULL = unbounded above.

The window is **half-open** — `[valid_from, valid_until)` — matching the read
predicate `ReadView::validity_sql` exactly: an instant equal to `valid_from` is
IN the window, an instant equal to `valid_until` is OUT.

These are **fields, not a new verb**. The governed *command* surface is
unchanged and allowlist membership in
`src/conformance/governed-surface-allowlist.json` is byte-identical; the
precedent is `PreparedWrite::Edge`, which has carried `t_valid`/`t_invalid` the
same way since Slice 30. The fields-only delta is **HITL-SIGNED 2026-07-29
(steward `seq-157`)**.

Slice 10b (R-20-NV) shipped the `canonical_nodes.valid_from`/`valid_until`
columns, the `ReadView` validity predicate and `Engine::crossed_boundary_since`
as a READ-ONLY axis with no writer; these two fields are that writer.

**Refusal rule (engine-owned).** Validation lives in the engine's
`validate_write`, so Rust, Python and TypeScript share one rule and cannot
drift:

- Both bounds present with `valid_from >= valid_until` describes an
  UNSATISFIABLE half-open window that no instant can ever match. It is refused
  with **`EngineError::WriteValidation`**. Validation runs **before any INSERT**,
  so the WHOLE batch is rejected. It surfaces as `WriteValidationError` in both
  bindings.

  > **BREAKING (0.8.20 Slice 22, decision #18).** This was
  > `EngineError::InvalidArgument { msg }` **naming both bounds**, which made
  > `validate_write` — one function — reject across two error families. It is now
  > the ONE family the taxonomy of record assigns to that boundary
  > (`dev/design/errors.md`, 2026-07-28 amendment). **`WriteValidation` is a unit
  > variant, so the offending bounds are NO LONGER carried in the error.** A
  > caller that parsed them out must instead validate the pair before calling.
- A **one-sided** window (exactly one bound present) can never be empty and is
  **never** refused, however extreme its single bound.

**No-regression guarantee.** Omitting both fields binds NULL/NULL — identical to
what schema step 22 left on every pre-existing row — so a write that does not
mention validity keeps exactly its pre-slice default-view visibility.

## Read-side validity on `search` (0.8.20 Slice 15b fix-2)

**Status: PROPOSED / NOT SIGNED.**

Slice 10b applied `ReadView` to the five read verbs only. Because Slice 15b made
validity windows AUTHORABLE from the SDK, the default `search` path now also
applies the validity predicate — otherwise a node hidden by `read_get` /
`read_list` would still be returned by `search`.

**Default behaviour change (deliberate, and the only one in this fix).** Every
search entry point (`search`, `search_filtered`, `search_filter`,
`search_reranked`, `search_explained`, `search_text_only`, and the opt-in graph
arm) now hides nodes that are out of window AT QUERY TIME. This is a **no-op on
any corpus that never authored a window**: schema step 22 back-filled
`valid_from` / `valid_until` as NULL with no DEFAULT, and NULL is unbounded on
that side, so the predicate matches every pre-existing row and leaves the
row-set, the `bm25()` ordering and the scores byte-unchanged.

**New methods** (additive; the six shipped search signatures are UNCHANGED):

- `Engine::search_view(query, &ReadView) -> Result<SearchResult, EngineError>`
- `Engine::search_reranked_view(query, filter, rerank_depth, use_graph_arm,
  alpha, pool_n, explain, &ReadView) -> Result<SearchResult, EngineError>` — the
  full-arity form the Python/TS `view=` bindings call, so a caller can combine a
  content filter, the CE knobs and a validity view in one query.
- `Engine::search_text_only_view(query, &ReadView) -> Result<SearchResult, EngineError>`
- `Engine::search_view_with_limit(query, &ReadView, limit) -> Result<SearchResult, EngineError>`
- `Engine::search_reranked_view_with_limit(query, filter, rerank_depth, use_graph_arm, alpha,
  pool_n, explain, &ReadView, limit) -> Result<SearchResult, EngineError>`
- `Engine::search_text_only_view_with_limit(query, &ReadView, limit) -> Result<SearchResult,
  EngineError>`
- `Engine::search_reranked_view_with_snippets(.., limit, snippet: Option<&SnippetOptions>)`,
  `Engine::search_text_only_view_with_snippets(query, &ReadView, limit, snippet)` and
  `Engine::search_frozen_with_snippets(.., limit, snippet)` — the `*_with_limit` and
  `search_frozen` forms with a trailing snippet option (EP-17, § "Match-centred snippets
  on search" below). `None` is exactly the plain call.

`search_reranked(q, f, d, g, a, p)` is exactly
`search_reranked_view(q, f, d, g, a, p, false, &ReadView::default())`.

**Independent view axes.** `valid_as_of`, `include_out_of_window`,
`include_superseded` and `include_inactive` are honored. Default search excludes
superseded/inactive rows. Relaxed hybrid search validates historical physical
vector completeness and reports a whole-vector-arm soft fallback when history
is unavailable, including absent runtime; partial tuples remain corruption.
Direct text does not report vector fallback. History remains ranked retrieval,
not an enumeration of every revision. Graph entities select the newest eligible
revision; canonical historical attributes support relaxed filters.

Property FTS with either existence flag builds a private per-call in-memory
corpus from the persistent property baseline plus authorized canonical history.
Validity-only/default property queries retain the indexed path. All extraction,
ranking and hydration share one reader snapshot; the private connection drops
on return/error. The opt-in corpus copies baseline and added scalar bytes, scans canonical nodes
per FTS declaration,
can prolong WAL snapshots, queueing and close, and has no added deadline/cap.
Concurrent reads may return coherent pre-erasure/supersession data; erasure does
not revoke returned memory. See the [mechanics and invocation table](../plans/0.8.28/features/slice-80/design.md#historical-property-fts-lifecycle-and-ranking).

Evidence existence flags relax artifact lifecycle only. Pinned sources remain
strictly active/nonsuperseded and access-authorized, including graph-origin
sources. Historical self-sourced records may consequently remain unavailable.
Retained/repaired edge FTS can change BM25 corpus statistics and strict scores;
ranking algorithms and default eligibility are unchanged.

The instant is INTEGER epoch SECONDS, read in Rust and BOUND as a positional
parameter (never `datetime('now')`), once per query — the same `:now` seam as
the read verbs, so search validity is deterministically testable.

## Projection registry (0.8.20 Slice 15d, R-20-PR / C-1)

**Status: HITL-SIGNED 2026-07-29 (steward `seq-157`)** for the two net-new
commands (`configure_projections`, `read.projections`), the types
`ProjectionSpec` / `ProjectionRole` / `ProjectionDelta`, and the typed
`ProjectionDestructiveError` — all recorded in
`src/conformance/governed-surface-allowlist.json`. **AC-079 is minted** (0.8.20
Slice 40; `dev/acceptance.md`); the signature is pinned to that file's content, so any
diff re-opens the gate (T1e pin).

⚠ **`ProjectionFts` and `ProjectionVector` are NOT part of that `seq-157`
signature** — neither appears in the allowlist. `DenseReadiness` is separately
**HITL-SIGNED 2026-08-07 (steward `seq-246`)** by Slice 21 F5/C1; this signs its
closed vocabulary and no new command. Slice 21's runtime implementation now
selects `Unavailable` when no usable dense runtime exists.

The registry pair declares and inspects projections over interpretive
attributes. The Slice-22 C5 read separately reports current dense runtime
status. The facade re-exports the five supporting `Projection*` types — plus
`DenseReadiness` since 0.8.20 Slice 20 (R-20-DR) — all part of the public Rust
surface:

- `Engine::configure_projections(specs: &[ProjectionSpec], drop: &[String]) ->
  Result<ProjectionDelta, EngineError>` — declarative, idempotent apply: the
  engine is the SOLE projection authority (C-1) and diffs `specs` against the
  durable registry, backfilling the difference in one write transaction. `drop`
  is EXPLICIT — omitting a live projection from `specs` does NOT drop it; removal
  requires naming it in `drop`. A destructive change (a role removal or a
  tokenizer/embedder change) without a drop is refused with
  `EngineError::ProjectionDestructive { name, delta }` — the delta names what the
  caller must drop. Re-applying an unchanged spec returns
  `ProjectionDelta { unchanged: true, .. }` with the vecs empty.
- `Engine::read_projections() -> Result<Vec<ProjectionSpec>, EngineError>` — the
  registry introspection (the Rust analogue of `read.projections`), sorted by
  name. Pure read; never mutates. Since 0.8.20 Slice 20 (R-20-DR) it is also the
  surface that populates the engine-set `ProjectionVector::dense_readiness`
  READ METADATA (derived on the way out; see below).
- `Engine::read_projection_status() -> Result<ProjectionRuntimeStatus,
  EngineError>` — **HITL-SIGNED 2026-08-07 (steward `seq-247`)** C5 status
  read. It is a separate, pure facade over the durable declaration registry and
  this open session's dense-runtime facts; it neither configures projections nor
  schedules, wakes, or drains projection work. Its result is not a decorated
  `ProjectionSpec` or the internal lifecycle `ProjectionStatus`.

`ProjectionRuntimeStatus` has `runtime_embedder_available`, its closed
`ProjectionRuntimeUnavailabilityReason` (`"none" | "no_runtime"`), sorted `ProjectionRuntimeStatusEntry` values,
sorted/deduplicated `vector_unsupported_kinds`, and required
`vector_coverage: StoredHeadCoverageV1`. Each entry has its name and
closed `ProjectionStatusDenseReadiness` (`"not_declared" | "unavailable" |
"embedding" | "ready"`). `not_declared` means the stored declaration has no
effective vector arm — exactly `StoredProjection::wants_vector`, so a legacy
non-searchable vector sub-object remains `not_declared`. The other readiness
states are corpus-wide shared-pipeline facts and therefore can repeat across
effective vector declarations; they do not assert per-projection progress.
`vector_unsupported_kinds` is `[]` unless an effective vector arm exists.

The C5 facade types are also public Rust surface:
`ProjectionRuntimeStatus`, `ProjectionRuntimeStatusEntry`,
`ProjectionRuntimeUnavailabilityReason`, and
`ProjectionStatusDenseReadiness`. They are deliberately distinct from the
internal lifecycle `ProjectionStatus` enum.

### Projection generation and mutation readiness (0.8.25 Slice 40)

`Engine::read_projection_generation_status()` is a pure, one-snapshot read of
the database-local in-place serving-projection epoch. Its versioned result
contains the `pgen1:<32-lower-hex>` generation ID, declaration digest, origin,
transition and observed boundaries, `ready_through`, closed readiness/runtime
states, and pending/failed counts. It does not schedule, repair, or expose a
work manifest.

`Engine::read_mutation_projection_status(request)` addresses one cursor already
named by a Slice-25 actuation receipt. The request supplies schema version 1,
the receipt operation ID, canonical decimal write cursor, and the receipt's
expected generation ID. The read verifies receipt ownership and generation,
then reports that member's completion with the same snapshot boundaries.
Unknown, erased, retired-generation, or no-longer-eligible members fail with
typed `ProjectionGenerationError`; they are never rebound to the current
generation.

`ActuationReceiptV1::projection_generation_id` is additive and nullable. It is
present exactly when the committed receipt contains pending projection cursors
under schema step 32 or later. Public counters and cursors remain `u64`; SDK
wire forms use canonical decimal strings.

### Embedding readiness (0.8.23 Slice 30)

`Engine::read_embedding_readiness() -> Result<EmbeddingReadiness, EngineError>`
is an additive, pure current read. `EmbeddingReadiness` exposes
`state: EmbeddingReadinessState`, `usable_embedder`, `pending_count`, sorted
`affected_kinds`, and `blocked: Option<EmbedderRequired>`; it exposes no pending
body text. `EmbeddingReadinessState` has the closed values `Ready`,
`Processing` and `Blocked` (`as_str()` supplies the lower-case
wire forms).

`blocked` is present only for pending work with no configured runtime. Its
`EmbedderRequired` payload has stable code `FDB_EMBEDDER_REQUIRED`, an
`EmbeddingOperation`, state `Blocked`, ordered remediations, and a documentation
URL. In the same condition `Engine::drain` returns
`EngineError::EmbedderRequired` immediately. Actual open-time identity/equivalence
refusal returns no engine. Later worker failures retain operational readiness
and ordinary bounded drain behavior.

The blocked payload's remediations are exactly `configure_default_embedder`,
then `submit_non_embedding_input`. The former selects the supported built-in
runtime on a subsequent open; it is not live reconfiguration. The latter avoids
new embedding work and does not clear an already pending backlog. Production caller-supplied
numerical engine providers are unsupported; semantic extractor/consolidation
subprocess providers remain a separate contract.

Types:

- `ProjectionSpec { name: String, roles: BTreeSet<ProjectionRole>, fts:
  Option<ProjectionFts>, vector: Option<ProjectionVector> }`. `roles` carries SET
  semantics (an attribute can be `Filterable` AND `Searchable`).
- `ProjectionRole` — exactly three variants: `Filterable`, `Rankable`,
  `Searchable` (`searchable→FTS` and `searchable→vector` are tier labels carried
  by the `fts`/`vector` sub-objects, not roles). `as_str` / `from_str_opt` give
  the `"filterable" | "rankable" | "searchable"` wire spellings.
- `ProjectionFts { tokenizer: Option<String> }` and `ProjectionVector { embedder:
  Option<String>, dense_readiness: Option<DenseReadiness> }` — the
  `searchable→FTS` / `searchable→vector` sub-target selectors (`None` embedder ⇒
  engine default; every new string selector is rejected before mutation). `dense_readiness` was added additively by 0.8.20 Slice 20
  (R-20-DR); see below.
- `DenseReadiness` — a three-variant enum, `Unavailable`, `Embedding`, and
  `Ready`, with `as_str` / `from_str_opt` giving the
  `"unavailable" | "embedding" | "ready"` wire spellings. Slice 21's F5/C1
  ruling (`steward-ledger` seq-246) signs the vocabulary without adding a
  governed command. Caller input accepts the spelling inertly; reads select
  `Unavailable` when the session has no usable dense runtime.
- `ProjectionDelta { built, dropped, deferred, unchanged, vector_unsupported_kinds }`.
  Cheap roles (`filterable`, `searchable→FTS`) build same-transaction; `rankable`
  and the `searchable→vector` sub-target are persisted-but-deferred (reported in
  `deferred`, never an error).
  `vector_unsupported_kinds: Vec<String>` was added additively by 0.8.20 Slice 22
  (R-20-VC, `TC-67`); see below.
- **`ProjectionDelta::vector_unsupported_kinds` — node KINDS, not attribute
  names** (0.8.20 Slice 22, `TC-67`). The first three vectors list projection
  attribute names; this one lists the vector-eligible node **kinds** present in
  the corpus that the vector writer can **never** commit, so no
  `searchable→vector` declaration will ever produce an embedding for them. Those
  rows stay fully FTS/lexically searchable. It exists because `deferred` alone
  could not distinguish *transient* ("no embedder attached this session, or an
  embed still in flight") from *permanent* ("outside the locked `source_type`
  vocabulary — never, in any session"): both produced the same `deferred` entry.
  Sorted, de-duplicated, and **empty rather than absent**.
  - It is a **STATE report, not a diff**: it does not feed `unchanged`, so an
    idempotent re-apply returns `unchanged: true` with the three diff vectors
    empty **and the report populated**.
  - It is **embedder-independent** — reported identically with and without a live
    embedder, because the vocabulary is static. Do not read it as the deferral.
  - It is **OUTPUT-ONLY**: `configure_projections` takes `&[ProjectionSpec]`, so
    the field has no inbound direction and cannot affect the `read_projections` →
    `configure_projections` round-trip.
  - **Residual — it is computed at DECLARE time.** A non-committable kind written
    *after* the call is not in a delta the caller already holds. To refresh it,
    re-apply the same spec: an idempotent no-op that returns a current report.
  - It is **not an error and not a readiness change**: with a usable dense
    runtime, readiness still reaches `Ready` (see the enrolment bullet below),
    nothing is rejected, and no `projection_failures` row is written. Without
    a usable runtime, runtime selection remains `Unavailable`.

**Spec-validation reject — an `fts`/`vector` sub-object REQUIRES the `searchable`
role (0.8.20 Slice 23, `R-20-SV`).** ⚠ **BREAKING.**
`Engine::configure_projections` REFUSES a `ProjectionSpec` that carries
`fts: Some(_)` or `vector: Some(_)` while `roles` does not contain
`ProjectionRole::Searchable`. The error is **`EngineError::WriteValidation`** —
the decision-#18 write-SHAPE family — surfaced as `WriteValidationError` in
Python and as code `FDB_WRITE_VALIDATION` (message `"write validation error"`,
`data: null`) in TypeScript.

- **Why.** `searchable→FTS` and `searchable→vector` are TIER LABELS, not roles:
  the sub-objects SELECT a sub-target of `searchable` and do not CONFER one. Both
  engine build predicates are conjunctions with the role, so without it the
  declaration builds no property-FTS, enrols no kind and embeds nothing — a
  meaningless config. HITL ruling 2026-07-24 (`dev/plans/plan-0.8.20.md` §11
  item 4, option (b) REJECT).
- **This SUPERSEDES the 0.8.20 Slice 15d fix-4 accept-and-round-trip position**
  and the "accept-inert ruling" this document previously cited as precedent
  (below). Nothing else about that fix changed.
- **Keyed on the ABSENCE of `searchable` alone.** `Filterable` / `Rankable` are
  orthogonal axes; neither supplies nor substitutes for the role, so every
  non-`searchable` role set is refused identically.
- **A rejected request is a TOTAL no-op.** Validation runs before any write, so a
  single invalid spec anywhere in `specs` aborts the whole call: valid siblings
  are not registered and `drop` entries do not apply.
- **`read_projections` is UNAFFECTED** — it is a pure read and rejects nothing.
- **Migration for the LEGACY population.** Databases that declared the shape
  while the engine accepted it still hold it at rest. Those rows still READ back
  verbatim, but the `read_projections` → `configure_projections` round-trip no
  longer closes for them: re-applying raises, and re-declaring only the valid
  half is refused as `ProjectionDestructive` (it removes the stored sub-object).
  The two remedies are **add the `searchable` role** (non-destructive, accepted)
  or **name the projection in `drop`**.
- **Known diagnostic cost (TC-95/TC-98, HITL-deferred).** `WriteValidation` is a
  UNIT variant, so with a LIST of specs the refusal cannot name WHICH spec was
  invalid. See `dev/design/errors.md`.

**Projection-name contract (0.8.20 Slice 15d fix-4).** A projection `name` is an
app-declared identifier that becomes a SQLite JSON-path key (`$."<name>"`) at
write time, so `configure_projections` REJECTS — with
`EngineError::InvalidArgument` naming the offending value — any spec or `drop`
name that cannot round-trip through that quoted-key form: an empty name, a name
containing a double-quote `"`, a name containing a BACKSLASH `\`, or a name
containing any ASCII control char. This upholds the invariant "a name the engine
ACCEPTS is populatable" (accept ⟹ works); previously a backslash name was
accepted yet silently never populated `canonical_attributes`.

**Dense readiness on `ProjectionVector` (0.8.20 Slice 20, R-20-DR).**
`ProjectionVector::dense_readiness: Option<DenseReadiness>` is **engine-set READ
METADATA**, not a declaration. `Engine::read_projections` populates it — and only
for a spec that declares the `searchable→vector` sub-object; `filterable` and
`searchable→FTS` are same-transaction (non-stale on commit) and have no readiness
axis. It is `None` on every caller-authored spec.

- **`DenseReadiness` has exactly three variants**, `Unavailable`, `Embedding`,
  and `Ready`, wire spellings `"unavailable"` / `"embedding"` / `"ready"`.
  The signed target meaning is: `Unavailable` for no usable dense runtime
  (absent), `Embedding` for a usable runtime with
  eligible outstanding work, and `Ready` for usable, quiescent work.
  **`pending` is RESERVED for the orthogonal ADMISSION axis** (quarantine/trust
  — an app judgment) and is deliberately never an index-readiness value.
  `from_str_opt("pending")` is `None`.
- **Runtime selection.** `read_projections` first applies one
  usable-dense-runtime predicate: no runtime yields
  `Unavailable`. With a usable runtime it derives `Embedding` / `Ready` from
  the shared outstanding-work predicate. This adds no schema step or stored
  readiness field.
- **ACCEPT-INERT on the way in.** `Engine::configure_projections` neither stores
  nor honours a caller-supplied `dense_readiness` (`StoredProjection::from_spec`
  reads only `embedder`), so `read_projections` output re-applies as a no-op —
  the shipped read→configure round-trip.
  **⚠ 0.8.20 Slice 23 (`R-20-SV`) correction (TC-39 class):** this bullet used to
  cite "the accept-inert ruling on an `fts`/`vector` sub-object declared without
  the `searchable` role" as its precedent. **That ruling is OVERRULED** — the
  HITL ruled the shape an INVALID SPEC on 2026-07-24 and Slice 23 rejects it (see
  the spec-validation reject above). `dense_readiness` accept-inert stands on its
  OWN footing and is unchanged: it is engine-set READ METADATA, not part of the
  declaration, and the round-trip it protects is still live.
- **The BINDINGS hard-reject** the two shapes that could never round-trip: a
  readiness supplied with `vector = false`, and any spelling outside
  `{unavailable, embedding, ready}` (notably `pending`, and the empty string).
  Both reuse the
  EXISTING `EngineError::InvalidArgument` / `InvalidArgumentError` /
  `FDB_INVALID_ARGUMENT` — **no new error type is minted.** A declared readiness
  never changes what the engine reports.
- **Additive.** Callers who never look at readiness see identical behaviour.
  The vocabulary is **HITL-SIGNED 2026-08-07 (steward `seq-246`)**; runtime
  selection is derived rather than durable.

**`drain` is the flush-to-readiness barrier (0.8.20 Slice 20c, R-20-DR /
`api-surface.md` C4).** There is **no `flush_embeddings()` verb** — the shipped
`Engine::drain(timeout_ms)` carries those semantics, so the surface gains ZERO
net-new governed commands (TC-55 = INSTRUMENTATION). The pinned invariant, tested
in Rust, Python and TypeScript:

> With a usable dense runtime, `drain(timeout)` returning `Ok(())` ⟹
> `dense_readiness == Ready`,
> **and every vector-eligible row has its vector row at rest.**

- **`drain` is a BARRIER, not a trigger.** It waits for the projection runtime to
  go quiescent; it never schedules or wakes anything. Deferred/backfill work is
  therefore enqueued on the **enqueue side**, on the same runtime `drain` waits
  on: `Engine::configure_projections` enrols the vector kinds, re-opens the
  stranded rows' readiness terminals and calls the runtime notify **after its
  commit**. Without that, declaring `searchable→vector` over an existing corpus
  reported `Ready` with no vectors and nothing that would ever create them.
- **Ordering does not matter.** Write-then-declare and declare-then-write behave
  identically: a kind first written after the declaration is enrolled on the write
  path, before the decision to wake the dispatcher is taken. That write-path
  enrolment performs the **same** backfill the declaration does (fix-2), so rows
  of that kind written by an earlier session — for instance one opened without an
  embedder, where the declaration persisted but deferred — are picked up too,
  rather than being left behind a `Ready` that is not true of them.
- **The dense arm covers only the engine's locked `kind` vocabulary** (fix-2).
  A `searchable→vector` declaration turns the dense arm on for node kinds in
  `{email, article, paper, meeting, note, todo, doc}` (plus the engine-internal
  `edge_fact` for edge bodies). Rows of ANY other `kind` are accepted and stay
  lexically searchable, but get **no vector** and are not counted as outstanding
  work, so readiness reaches `Ready` only with a usable dense runtime. An
  absent runtime instead selects `Unavailable`. This is
  **not** an error condition: the write is not rejected and no typed error is
  raised.
  **Since 0.8.20 Slice 22 (`TC-67`) it is no longer silent, either:** the excluded
  kinds are named in `ProjectionDelta::vector_unsupported_kinds`. Only the
  reporting changed — the exclusion, the readiness semantics and the absence of an
  error are exactly as Slice 20c left them.
- **Idempotent.** Re-applying an already-satisfied declaration re-opens nothing,
  rewinds no watermark, and re-embeds nothing (`ProjectionDelta::unchanged`).
- **Dropping the last `searchable→vector` declaration turns the dense arm back
  off** (fix-1). The `drop` un-enrols the node kinds the declaration enrolled, so
  subsequent writes of those kinds enqueue no embed and `drain` no longer waits on
  them. It **deletes no embedding**: vectors already at rest survive, exactly as
  they always have across a `drop`. Re-declaring re-enrols and backfills, so a
  row written while the arm was off is picked up, not stranded. Edge-body vectors
  are unaffected — the `edge_fact` kind is registered off the presence of an edge
  body, not off the projection registry.
- **The dense arm requires the `searchable` ROLE, not merely the `vector`
  sub-object** (0.8.20 Slice 21c, `TC-71`). A spec such as
  `{ roles: {Filterable}, vector: Some(_) }` is **REJECTED since 0.8.20 Slice 23**
  (`R-20-SV`, above); until then it was accepted and round-tripped verbatim while
  being **INERT**: it enrolled no kind, backfilled nothing, and made no later
  write enqueue an embedding. The inertness still governs the LEGACY population
  that holds the shape at rest. Previously the engine keyed
  the dense arm off the stored `vector` sub-object alone, so declaring that
  combination in a session with a live embedder silently embedded the whole
  corpus. **The inverse moves with it:** demoting the last `searchable→vector`
  projection to `{filterable} + vector`, or dropping it while an inert
  `{filterable} + vector` sibling survives, now un-enrols exactly as a literal
  drop does. `ProjectionDelta.deferred` still reports the stored-but-unbuilt
  `vector` sub-object however it was declared — the change is to what the engine
  DOES, not to what it reports.
- **Graceful-absent without a usable dense runtime.** The declaration persists
  and DEFERS rather than queueing unsafe work. A later safe open atomically
  grafts eligible durable work after identity and equivalence acceptance;
  idempotent re-apply remains a repair door.
- **…but graceful-absent stops at the enrolment boundary** (fix-4). Once a kind
  IS enrolled — i.e. some earlier session DID have an embedder — a write of that
  kind is dense work the workspace has committed to, and an absent runtime cannot
  make it go away. Such a write is **accepted** and stays lexically searchable,
  but it remains **outstanding**: `dense_readiness` reads `Unavailable` and
  `drain` returns immediate `EngineError::EmbedderRequired`
  (`FDB_EMBEDDER_REQUIRED`) rather than spending its scheduler timeout. It is
  **not** lost — no failure is recorded and no terminal is written, so the next
  session opened WITH an approved runtime embeds it through the ordinary
  scheduler, with no re-apply and no operator `rebuild`. Actual verification
  refusal fails open before an engine is returned. (Reporting `Ready` here instead would be a torn
  `ready`-without-vector — the silent miss this slice exists to eliminate.)
- **`drain` remains bounded**, returning the typed timeout error rather than
  blocking; a caller sizes `timeout_ms` for the backfill it just asked for.

**Attribute filters on `SearchFilter` (0.8.20 Slice 15e, R-20-PR / ADR-0.8.11 D3).**
`SearchFilter` gains a public field `attributes: Vec<(String, String)>` — each
`(attribute_name, value)` is an equality predicate over a declared-`filterable`
projection, lowered pre-KNN into the indexed vec0 metadata column `attr_<hex>`
(never a post-KNN `json_extract`). Empty ⇒ the byte-identical unfiltered path is
preserved. The struct is now `#[non_exhaustive]`, so EXTERNAL crates cannot use
a struct expression, `..Default::default()` included; they start from
`SearchFilter::default()` and assign fields (a further additive field is then
not a source break). Allowlist MEMBERSHIP in
`src/conformance/governed-surface-allowlist.json` is byte-unchanged (`SearchFilter`
was already a re-exported type; this is a fields-only + attribute delta, the same
pattern as `PreparedWrite::Node`'s validity fields). Semantics are **node-scoped**:
an attribute filter EXCLUDES every edge hit on both retrieval arms (edges are never
attribute-projected), which is HITL ruling (A) — `(A)` is `(D)` endpoint-node
filtering with an empty endpoint rule; (B)/(C)/(D) are reserved widenings, none
implemented in 0.8.20. **This whole delta is PROPOSED, NOT SIGNED.** `attributes`
was engine-internal in 0.8.20; Slice 60 exposes it through both SDK bindings.

## Nested-source projections (0.8.21 Slice 60)

`ProjectionSpec`
adds `source: Option<Vec<String>>`. Omitted preserves direct top-level lookup;
present paths are literal, safe JSON object-member segments stored durably in
the registry. Missing/null terminals create no derived row. Object/array
terminals reject both configuration backfill and normal writes atomically with
`EngineError::WriteValidation`.

`SearchFilter.attributes: Vec<(String, String)>` is portable public API: an AND
of equality predicates over declared `Filterable` projections. Values use the
existing canonical text representation, intentionally so string `"1"` and number
`1` compare equal. `Filter::try_from(&SearchFilter)` rejects a non-empty
attribute list with `EngineError::InvalidFilter`; reverse lowering has no
attributes.

On `search_explained` with attribute predicates, `Explanation.trace.dropped_edge_hits`
reports edge-FTS candidates rejected solely by the
node-scoped attribute rule. Default non-explained searches do not collect the
counter or incur its extra comparison. If the count query or its row decoding
fails, search returns `EngineError::Storage` rather than a plausible zero.
Stored importance/confidence lookup failures likewise return
`EngineError::Storage` instead of a neutral ranking or explanation value.

- `Engine::search_projected_text(query, name, filter, &ReadView) ->
  Result<SearchResult, EngineError>` searches exactly one declared
  `Searchable` property-FTS projection. It applies metadata, validity, and
  attribute filters; orders by ascending FTS5 `bm25` then write cursor; returns
  `branch=Text`, `soft_fallback=None`, and no explanation. It never body-scans,
  embeds, or fuses with body/vector retrieval.
- `Engine::search_projected_text_with_limit(query, name, filter, &ReadView, limit) ->
  Result<SearchResult, EngineError>` has the same semantics with an explicit `1..=100` limit.

## Ranked result limits (0.8.22 Slice 18)

Every ranked search family defaults to 10 returned hits. The explicit `*_with_limit`
forms accept only `1..=100`; an out-of-range value returns `EngineError::InvalidArgument`
and is never silently clamped. The hybrid family also exposes
`search_filtered_with_limit`, `search_filter_with_limit`, `search_reranked_with_limit`, and
`search_explained_with_limit`. Vector candidate fanout is at least the requested limit; the
private test seam can only raise candidate fanout, never public result cardinality.
`graph_neighbors` remains separately capped at 50.

## Hints engine subset (ADR-0.8.29-agent-hints-sidecar)

`fathomdb_engine::search_hints(facts: &SearchFacts, out: &mut Vec<Hint>)` is a
pure function (no SQL, I/O, clock or locks) that appends the suggested next
actions for one search page. `fathomdb-sdk` re-exports it with its types. No
engine search path calls it in this stage; the MCP adapter calls it with its
own page facts, and `SearchResult` is unchanged. The agent vocabulary table
is `dev/interfaces/mcp.md` § 3; this section documents only the engine subset.

- `SearchFacts::new(returned, limit, mode)`, with `with_degradations(&[(HintArm,
  HintCause)])` (canonical order; only the first entry is named) and
  `with_hybrid_available(bool)` (usable embedder, hybrid permitted by
  configuration, dense arm not disabled). Defaults: no degradation, no hybrid.
- Emission, at most one hint per reason, in this order:
  - `Degraded` (`HintAction::None`, `arm`, `cause`) when a degradation entry is
    present.
  - `NoResults` (`HintAction::Search`) when `returned` is zero. `mode` is
    `Some(HintMode::Hybrid)` only for a text-mode search with
    `hybrid_available`; otherwise `None`.
  - `MoreResults` (`HintAction::Search`, `limit`) when `returned >= limit`
    and `limit < HINT_LIMIT_CAP` (100); `limit` is `min(2 × limit, 100)`.
- `HintCause` carries every cause word a degradation can have:
  `soft_fallback`, `dense_disabled`, `embedder_unavailable`,
  `embedder_overloaded`, `embedder_absent`, `embedder_timeout`, and the graph-expansion codes
  `query_seed_text_fallback`, `projection_legacy_unverified`,
  `projection_processing`, `projection_blocked`, `projection_deferred`,
  `projection_degraded`. `HintArm` is `vector`, `text`, `text_edge`,
  `graph_arm` or `graph`. Every type's `as_str()` returns its wire word.
- `Hint`, `HintAction`, `HintReason`, `HintMode`, `HintArm`, `HintCause` and
  `SearchFacts` are `#[non_exhaustive]`; new reasons, words and fields are
  additive.

## Match-centred snippet selector (EP-17)

The engine-only `test-hooks` feature exposes doc-hidden
`snippet_for_full_reference_for_test` with the same arguments and result as
`snippet_for`. It runs the retained full selector for exact differential tests;
shipped feature lists do not expose this seam.

For over-budget bodies with empty raw spans, `snippet_for` specializes only
ASCII letter/digit/space prefixes through the content budget plus one lookahead
byte (`max_bytes − len(" …") + 1`). It examines that bounded prefix and allocates by the output budget; unsupported
prefix bytes retain the full Unicode selector. Exact text, anchors, windows,
spans, highlights, clamping and UTF-8 byte coordinates are unchanged.

`fathomdb_engine::snippet_for(body: &str, spans: &[(u32, u32)], options:
&SnippetOptions) -> SnippetV1` is a pure function (no SQL, I/O, clock,
floating point or locks). It picks the part of a record body to show for a
search hit from the hit's match spans. `fathomdb-sdk` re-exports it with its
types. The search entry points below call the same function for their
snippet sidecar. The specification is `dev/design/agent-access/match-centred-snippets.md` §§ 6
and 7.2.

- `SnippetOptions` is `#[non_exhaustive]`, with public fields and a
  `Default`. Other crates start from `SnippetOptions::default()` and assign
  fields:
  - `max_bytes: u32`, `40..=4000`, default 300. The output budget in UTF-8
    bytes, ellipses included.
  - `max_windows: u8`, `1..=3`, default 1.
  - `head_bytes: u32`, `0..=max_bytes / 2`, default 0.
  - `highlights: bool`, default false.
  - `max_point_queries: u8`, default 20. The number of hits per search that
    get a combined span query; the rule-3 per-term pass draws on a second
    pool of the same size. `snippet_for` does not read it.
- `SnippetOptions::validate()` returns `EngineError::InvalidArgument` for a
  value out of range. `snippet_for` itself never fails: it clamps such
  values into range.
- Spans are body byte ranges `[start, end)`. `snippet_for` ignores spans
  that are empty, run past the body, or do not fall on `char` boundaries. It
  sorts the rest, widens each to grapheme boundaries (an NFD accent belongs to
  its letter), and merges spans that overlap.
- `SnippetV1` is `#[non_exhaustive]`:
  - `text`: at most `max_bytes` bytes.
  - `anchor`: `SnippetAnchor::Whole`, `Match` or `Start`; `as_str()` gives
    `whole`, `match` or `start`.
  - `windows`: the body ranges shown, sorted, disjoint and on grapheme
    boundaries.
  - `spans`: the normalised spans.
  - `highlights`: ranges into `text` of the spans shown in full; empty unless
    `highlights` is set.
  - `cut`: true when any of the body is left out.
  - `scored_bytes: Option<u32>`: exact original stored head endpoint of this
    selected record when it actually contributed to the original vector arm.
    `Some(0)` is known zero; `None` makes no extent claim. The pure selector
    always returns `None`. See the stored mapped-head coverage contract below.
- `text` is the windows joined in body order. `"… "` comes before a first
  window that starts after byte 0, `" … "` goes between windows, and `" …"`
  follows a last window that ends before the body. `windows` says where an
  ellipsis is structure; a record can contain `…` itself.
- Selection:
  - A body that fits is returned whole (`Whole`).
  - Without usable spans, the prefix is cut back to a word boundary (`Start`).
  - Otherwise the window is chosen by distinct span identities (lowercased
    span text), then span count, then a sentence start within 64 bytes before
    its first span, then the earlier start.
  - The window is centred on its spans. Its edges snap to UAX #29 word
    boundaries without cutting a counted span; past 32 bytes they fall back
    to a grapheme boundary. The start moves to the bonus sentence start when
    the budget still reaches the last span.
  - `head_bytes > 0` adds the record start as its own window. It is cut at
    the first line break or at the last word boundary at or before
    `head_bytes`.
  - `max_windows > 1` adds windows for identities that no window shows. The
    first window then takes `1 / max_windows` of the budget, but only when
    a whole span fits that narrower window and at least 80 bytes remain
    beside it.
  - Windows 16 bytes apart or less merge. If the merged text does not fit,
    its end snaps back, never before a counted span or the head's end.
- Output is deterministic per build. Word, sentence and grapheme boundaries
  come from `unicode-segmentation`, so a version bump of that crate may
  change snippet bytes.

## Match-centred snippets on search (EP-17)

`search_reranked_view_with_snippets`, `search_text_only_view_with_snippets`,
`search_text_only_filtered_with_snippets` and `search_frozen_with_snippets` take `snippet: Option<&SnippetOptions>`.
The plain entry points are these with `None`.
The filtered text route takes `(query, filter, &ReadView, limit, snippet)`;
metadata eligibility is applied before candidate bounds, deduplication and ranking,
while snippets are extracted after selection from that same reader snapshot.

- `Some(options)` is validated first (`SnippetOptions::validate`;
  `EngineError::InvalidArgument`). It then fills
  `SearchResult::snippets: Option<Vec<SnippetV1>>`, one snippet per hit, in
  `results` order. `None` leaves `snippets` as `None`, and every other field
  is the same with or without snippets.
- Work runs after ranking, truncation to the limit, and the identity pass, on
  the search's reader transaction, with read-only statements only.
- A text hit's FTS rowid is kept in a search-local map from `write_cursor`
  to `search_index` or `search_index_edges`, only when snippets are
  requested. For a mapped hit that needs spans, the engine runs one
  `highlight()` constrained to that rowid:
  - Spans are needed when the body is over `max_bytes` or `highlights` is
    set. A body that fits with highlights off gets the whole body and no
    query.
  - Markers are the first pair of U+E000/U+E001, U+F8FE/U+F8FF and
    U+FDD0/U+FDD1 that the body does not contain. If the body contains all
    three pairs, or the marked text does not strip back to the body, the hit
    gets no spans.
- Exact term identity (rule 3). It applies when the body is over budget, the
  query has more than one OR term, and some span lies outside every window of
  the approximate snippet. That is the as-built reading of "the first window
  misses a span": with a record head, the first window is the head. The
  engine then runs `highlight()` once per term in `CompiledQuery.terms`, and
  spans are grouped by term rather than by lowercased text.
- `max_point_queries` caps how many hits get a combined span query, in rank
  order. Later hits get the record start (`SnippetAnchor::Start`).
- The per-term queries draw on a second per-search pool of
  `max_point_queries` queries, in rank order. A hit's per-term pass runs only
  if the pool still holds `terms.len()` queries, and then costs that many.
  Otherwise the hit keeps the snippet built from the combined spans. A search
  therefore issues at most 2 × `max_point_queries` point queries.
- Anchor: `Match` only when the hit is in the map and highlighting found a
  span. `fusion` may keep a vector hit's struct (`branch == Vector`) for a
  body the text arm also matched, so `branch` is not consulted. Vector-only
  and graph-arm hits get `Start`.
- Frozen reads: snippet options are presentation. They are not eligibility
  and do not enter the frozen context or its token.
- `search_projected_text` and `bm25f_search` take no snippet option.

`fathomdb_query::CompiledQuery` has `match_expression` and `terms:
Vec<String>`. `terms` holds the OR operands as compiled, each a quoted,
escaped FTS5 string, and `match_expression == terms.join(" OR ")`.

## Errors

### Cross-encoder device policy (0.8.23 Slice 71)

The optional Candle cross-encoder uses the independent FATHOMDB_RERANK_DEVICE
transport. Its grammar is exactly auto, cpu, or cuda:N; unset means auto.
RerankerDevicePolicyError rejects malformed or retired values and forced CUDA
failure. CPU never initializes CUDA; auto may select CPU only with a classified
reason; forced cuda:N never retries on CPU. RerankerDeviceResolution is
intentionally distinct from OpenReport.embedder_device_resolution: CUDA
embedding is not evidence of CUDA reranking.

Both independent policies may resolve to the same CUDA UUID in one process.
That selects two Candle model instances; it is not a device reservation,
resource manager, allocator cap, or evidence that CPU-only retrieval stages
used CUDA.

Rust exposes typed open/runtime errors without message parsing:

- `EngineOpenError`
- `EngineError`

### CUDA private memory pool (0.8.28 Slice 30)

`EngineError` and `EngineOpenError` each carry three CUDA pool kinds, with
stable codes `CudaPoolExhaustedError`, `CudaContextLostError` and
`CudaPrivateBuildRefusedError`:

- `CudaPoolExhausted { ordinal, max_size_bytes, message }` — the private pool
  reached its cap; the device stays CUDA and the next request runs normally.
- `CudaContextLost { recorded_context_id, current_context_id: Option<u64>,
  driver_error, operation }` — the recorded context is gone (`None`: no
  current context) or was replaced.
- `CudaPrivateBuildRefused { ordinal, message }` — another private-pool
  context could not be built; no context on another allocator replaces it.

They are raised by `embed_text`, search query embedding (other
query-embedding failures keep the sparse fallback), search reranking (under
every reranker policy, never as neutral scores), and `Engine::open`
(default-embedder load, not `Embedder(Failed)`, and the embedder and
reranker device resolutions). In a process that allocates from the private
pool, a pool failure inside the resolution probe is
`CudaProbeError::Pool(CudaPoolFailure)` and resolution returns
`DeviceResolutionError::CudaPool` / `RerankerDeviceResolutionError::CudaPool`
under `auto` as under forced CUDA: it never moves to CPU.
`resolve_reranker_device_policy_from_env` returns it as the flat
`RerankerDevicePolicyError` pool variant. `From<RerankerDevicePolicyError> for EngineError` and
`From<EmbedderError> for EngineOpenError` map the corresponding kinds to these
variants and leave the rest unchanged. `rerank_passages` returns
`RerankPassagesError` — `WriteValidation { message }` for a non-finite score,
`Reranker(RerankerDevicePolicyError)` for a reranker refusal — and
`From<RerankPassagesError> for EngineError` maps it. The reranker singleton
memoizes a loaded model, a device-policy refusal and a genuinely unavailable
model; a load failure of a CUDA pool kind is returned and retried next call.

#### Allocator decision, settings and report

The decision rule is
[`ADR-0.8.28-tegra-private-cuda-pool`](../adr/ADR-0.8.28-tegra-private-cuda-pool.md).

- **Feature.** `tegra-pool` is implied by `embed-cuda`/`rerank-cuda`;
  naming it is accepted and redundant (0.8.28 Slice 31, owner ruling Q37).
  It lives in `fathomdb-embedder`, and in `fathomdb-engine`, `fathomdb`,
  `fathomdb-sdk`, `fathomdb-napi`, `fathomdb-py` and `fathomdb-cli` each
  crate's own CUDA features enable its own `tegra-pool`. It has an effect
  only on aarch64 Linux with `embed-cuda` or `rerank-cuda`; on x86_64 it
  compiles nothing. A Rust build therefore cannot compile the pool out on
  aarch64 Linux; `cuda_pool = off` is the run-time opt-out. It adds no dependency:
  the pool primitive comes from `cudarc-fathomdb` 0.21.0 (FathomDB's cudarc
  fork, on crates.io), which the embedder reaches only through the Candle
  fork crates 0.13.0, including the published renamed kernel with static CUDART
  linkage. A downstream build of the published crates with
  `tegra-pool` resolves and builds from crates.io with no `[patch]` table
  (`scripts/check-tegra-pool-registry-build.sh`).
- **Settings** (0.8.28 Slice 31, owner rulings Q33 and Q35). `EngineConfig`
  carries three open settings; each environment variable applies only when
  no open sets its setting:
  - `cuda_pool: Option<CudaPoolMode>` (`Auto` default, `On` lifts only the
    Tegra identity and measured-class gates, `Off`); fallback
    `FATHOMDB_POOL_MODE`;
  - `cuda_pool_max_size: Option<u64>`: bytes, at least
    `CUDA_POOL_MAX_SIZE_MIN` (32 MiB; smaller is
    `EngineConfigurationError::OutOfRange`), up to device total; replaces
    the derived size (3 GiB on the AGX Orin 64 GB) only after every gate
    passes; fallback `FATHOMDB_POOL_MAXSIZE` (bytes, `<n>G` or `<n>M`);
  - `cuda_pool_release_threshold: Option<ReleaseThreshold>` (`Zero`
    default, `Max`); fallback `FATHOMDB_POOL_RELEASE_THRESHOLD`. With `0`,
    freed GPU memory returns to the system automatically; FathomDB never
    holds freed memory. With `max`, freed GPU memory stays reserved for
    reuse while any FathomDB model is open in the process, and is returned
    when the last one closes (a synchronize of the decision's context, then
    `trim_to(0)`, skipped if the context is lost). "Closes" means the model
    is dropped, which can be shortly after `close` returns.

  `CudaPoolMode` (the embedder's `PoolMode`, with `as_str` and `FromStr` on
  `auto`/`on`/`off`), `ReleaseThreshold`, `PoolSettingSource`,
  `CudaPoolSettingConflict`, `CudaPoolSettingValue` and
  `CUDA_POOL_MAX_SIZE_MIN` are re-exported by
  `fathomdb-engine`, `fathomdb` and `fathomdb-sdk`. A malformed environment
  value turns the pool off with reason `invalid_setting`; it never aborts.
- **One pool configuration per process.** The GPU pool settings
  (`cuda_pool`, `cuda_pool_max_size`, `cuda_pool_release_threshold`) are fixed
  once per process: by the first open that passes any of them, or, if none
  does, by the `FATHOMDB_POOL_*` environment variables (or their defaults) at
  the first GPU use. A later open that sets one of them to a different value
  fails with
  `EngineOpenError::EngineConfiguration(EngineConfigurationError::CudaPoolSettingConflict(..))`
  naming the setting, the value in force and its source; the open creates no
  database file before the check. Closing every engine does not reset them.
  Opens that omit them never fail this way. An open that sets any of them
  fixes the configuration on every platform, including machines where the pool
  never engages (x86, discrete GPUs, CPU-only engines). Whether an open that
  omits them fixes the configuration depends on whether it builds a GPU
  device; set the same values on every open, or on none. Pick one set of
  values per process, or leave them unset; a test suite in one process must
  not open engines with different pool settings. A `cuda_pool_max_size` larger
  than the GPU's memory cannot be checked until the pool is created; the pool
  is then turned off, and the allocator report's reason is `invalid_setting`.
  Direct `CandleBgeEmbedder` and `CandleTinyBertReranker` constructors take no
  setting; they use the configuration in force, else the environment.
- **Report.** `CudaDeviceInfo` is `#[non_exhaustive]` (build it with
  `CudaDeviceInfo::new(..).with_cuda_allocator(..)`) and carries
  `cuda_allocator: Option<CudaAllocatorReport>`. `CudaAllocatorReport` is
  `#[non_exhaustive]` with `path: Option<CudaAllocatorPath>` (`Private`,
  `DefaultPool`, `Synchronous`; `None` when unknown), `reason:
  CudaAllocatorReason`, `pool_max_size_bytes: Option<u64>`,
  `release_threshold: Option<ReleaseThreshold>` (`Zero`, `Max`) and
  `module_load_init: ModuleLoadInit` (`Ran`, `OptedOut`, `SkippedCpuOnly`,
  `DriverAbsent`, `Failed(u32)`, `NotAtLoad`), `pool_mode:
  Option<CudaPoolMode>` (`None` for a malformed `FATHOMDB_POOL_MODE`), and
  `pool_mode_source`, `pool_max_size_source`, `pool_release_threshold_source:
  Option<PoolSettingSource>` (`Sdk`, `Env`, `Default`). Each enum has a stable
  `as_str()` name. It is `Some` with `reason: NotBuilt` and `path: None` on
  aarch64 Linux CUDA builds without `tegra-pool`, the built context's real
  allocator with it, and `None` on every other target. These types are
  exported by `fathomdb-embedder` and re-exported by `fathomdb-sdk`; the
  Python (`cuda_allocator`) and TypeScript (`cudaAllocator`) open reports
  carry the same report with each enum as its `as_str()` name.
- **Rust uses the probe's `cuInit`** (0.8.28 Slice 31, owner ruling Q36).
  Rust has no module-load hook. The decision admits `NotAtLoad` when the
  process's most recent `cuInit`, made by FathomDB's own device probe
  before the first device build, succeeded; FathomDB adds no call. Such a
  process reports `module_load_init: NotAtLoad`, `path: Private`, `reason:
  PrivatePool`. `NotAtLoad` with reason `cuinit_not_at_load` remains for a
  process whose driver no FathomDB probe initialised. `FATHOMDB_CUDA_EARLY_INIT`
  has no effect in Rust (it governs the Node and Python load hooks);
  `cuda_pool = off` turns the pool off. A recorded load outcome (Node,
  Python, `doctor cuda-allocator`) is never overridden by the probe, so a
  binding whose hook recorded `Failed` or `SkippedCpuOnly` still falls
  back where a Rust process would reach the pool. `ran` and `not_at_load`
  with `private_pool` mean the same allocator path.
  `run_module_load_early_init` stays binding support (§ Unstable
  binding-support items in `fathomdb-embedder`), not re-exported by the
  facade or the SDK.
- **Module-level model release.** The CLS embedder and reranker caches can be
  released with `release_models()`; under `max` the pool keeps its reservation
  while an owner remains; an engine's own embedder returns its
  memory to the pool, and at threshold 0 to the driver, when the engine
  closes.
- **Context reset remains unsupported (C7).** The 0.8.29 implementation shares
  one terminal native lifetime per CUDA device across private/default/synchronous
  contexts. Detected loss refuses subsequent construction, embed and cached
  rerank with existing typed context-loss errors until process restart, with one
  `fathomdb-cuda-context-lost` line per process. Rust owners, workers and locks
  are released normally; lost native handles/allocations can remain as the
  frozen live set until restart. Opaque native host bytes have no established
  hard bound, and successful close does not assert native residue reclamation.
  CPU seam proof alone does not qualify live-reset survival. Selected
  completed-construction post-loss teardown and explicit release evidence is
  recorded in [0.8.30 Slice50](../plans/0.8.30/features/slice-50/status.md);
  reset during inference and same-process CUDA recovery remain unsupported.

### CUDA library check and GPU floor (0.8.28 Slice 31)

Before a CUDA device is built, `fathomdb-embedder` checks the device against
the build and the CUDA libraries Candle's device construction loads
(`dev/adr/ADR-0.8.28-cuda-version-support-policy.md`, amendment of
2026-10-13):

- **GPU floor.** `CudaProvider` has a provided method
  `kernel_compute_floor(&self) -> Option<(u32, u32)>` returning the build's
  `CUDA_COMPUTE_CAP` (read at compile time; 75 for the x86_64 artifacts, 87
  for Jetson; `None` without it), parsed by the public
  `parse_compute_capability_floor(&str) -> Option<(u32, u32)>` (`"75"` is
  7.5, `"120"` is 12.0). Resolution, `diagnose_gpu` and the reranker refuse an
  enumerated device whose compute capability is below the floor before
  calling `probe_cuda`: forced `cuda:N` is `ForcedCudaUnavailable { reason:
  CudaIncompatible }`, `auto` (which probes only the lowest visible ordinal)
  runs on CPU with `cuda_incompatible`, and `doctor gpu` reports
  `cuda_incompatible`. An unknown capability or no floor refuses nothing.
- **Library check.** The concrete probes call the check after the floor and
  before `new_cuda_device`. It loads cuBLAS and cuRAND for the built CUDA
  major through `cudarc-fathomdb` (reached as `candle_core::cuda::cudarc`)
  and refuses a library it cannot load (`missing`), a cuBLAS that reports
  another major or a candidate rejected for one (`wrong_major`), or a
  library that lacks one of the cuBLAS or cuRAND functions cudarc's result
  layers call (`missing_symbol`). The refusal is
  `CudaProbeError::LibraryUnavailable(CudaLibraryRefusal)`; resolution maps
  it to `DeviceResolutionReason::CudaLibraryUnavailable` /
  `RerankerDeviceResolutionReason::CudaLibraryUnavailable`
  (`"cuda_library_unavailable"`) under `auto` (CPU) and to
  `DeviceResolutionError::CudaLibraryUnavailable { ordinal, refusal }` /
  `RerankerDeviceResolutionError::CudaLibraryUnavailable { ordinal, refusal }`
  under forced CUDA; `EmbedDevicePolicyError::kind()` and
  `RerankerDevicePolicyError::kind()` return `cuda_library_unavailable`.
  `doctor gpu` reports `probe_failed` with that reason. A refusal writes one
  `fathomdb-cuda-libraries {json}` line to stderr per process; a successful
  load writes nothing.
- **`CudaLibraryRefusal`** (`#[non_exhaustive]`, `CudaLibraryRefusal::new`):
  `library: &'static str` (`cublas` or `curand`), `reason:
  CudaLibraryRefusalReason` (`Missing`, `WrongMajor`, `MissingSymbol`, with
  `as_str()` names `missing`, `wrong_major`, `missing_symbol`), `path` and
  `version` (`Option<Box<str>>`), `required_major: u32` (the library's own
  major: 12 for cuBLAS, 10 for cuRAND in a CUDA 12 build), `tried: Vec<(String,
  String)>` and `missing_symbols: Vec<String>`. Its `Display` names the
  library, file, version and major and gives the remedy: for `missing` and
  `missing_symbol`, `pip install nvidia-cublas-cu12 nvidia-curand-cu12` with
  both library directories on `LD_LIBRARY_PATH`; for `wrong_major`, the file
  to remove from the search path.
- **Report.** `CudaDeviceInfo` carries `cuda_libraries:
  Vec<CudaLibraryReport>` (`with_cuda_libraries`): the driver, cuBLAS and
  cuRAND, and cuBLASLt or NVRTC when loaded. `CudaLibraryReport`
  (`#[non_exhaustive]`, `CudaLibraryReport::new`) has `library`,
  `opened_as`, `path` (canonical, comparable with `/proc/self/maps`),
  `version` and `built_major`. These types are re-exported by `fathomdb-sdk`;
  Python (`cuda_libraries`) and TypeScript (`cudaLibraries`) carry the report.
- **Breaks.** `CudaProbeError`, `DeviceResolutionError`,
  `RerankerDeviceResolutionError` and both reason enums are not
  `#[non_exhaustive]`, so the new variants break exhaustive matches (pre-1.0,
  recorded in the changelog). `CudaLibrariesDiagnostic` and
  `cuda_libraries_diagnostic()` (CUDA builds) serve `doctor cuda-libraries`
  and are binding support, hidden from the docs.

Canonical leaf mapping lives in `design/errors.md`. This file adopts those
types without renaming them.

## Recovery / operator seam re-exports

With the `operator` feature, the facade exports path-scoped free functions:

- `inspect_data_plane_integrity(path, DataPlaneIntegrityRequestV1) ->
  Result<DataPlaneIntegrityResultV1, EngineError>` validates first, acquires the
  existing product lock without rewriting it, refuses non-quiescent sidecars,
  and opens exact-schema SQLite via an immutable read-only/query-only
  connection. It does not migrate, repair, reconcile projections, or construct
  a serving runtime.
- `recover_truncate_wal(path: impl Into<PathBuf>) ->
  Result<TruncateWalReport, EngineOpenError>` acquires the canonical product
  lock without constructing an Engine, independently validates the main file
  and current Fathom schema invariants, refuses a nonempty rollback journal,
  and lets SQLite own the destructive checkpoint/discard. Healthy-WAL preflight
  validates the effective view and restores its SHM snapshot when refusing;
  the final connection is `mode=rw` without create permission. Public
  `Engine::open` remains fail-closed on a malformed WAL.
  `TruncateWalReport::discarded_corrupt_wal` is true only when the locked header
  classification was malformed and SQLite returned `Done`; it is false for
  absent/healthy WAL and `Busy`.
- `recover_complete_erasures(path: impl Into<PathBuf>) ->
  Result<CompleteErasuresReport, EngineOpenError>` shares the offline
  admission and lock path, validates all owed physical zero proofs, refuses
  unavailable original telemetry obligations, and uses one read-write
  recovery connection for the truncate checkpoint before marking closures
  complete. A busy checkpoint leaves them owed.

The operator-gated `Engine::pending_erasures()` returns content-free
`PendingErasure` records for doctor diagnosis; it does not expose source
identity or alter closure state.

Slice 82 extends that diagnosis to root-only erasures and retained FTS terms.
Operator identities `root-erasure` and `retained-terms` use sequence zero and are
not governed dependency-closure lookup IDs. `retained_terms` is a maintenance
cause, with `dirty`, `cleaning` or `finalizing` phase. Missing current-format
sanitation metadata is legacy dirty debt. The offline completion path processes
it even without dependent closures; only a clean certificate with no physical
or telemetry debt produces the unchanged no-op. Supported FTS mutations invalidate
the certificate without performing sanitation. Every admitted root erase/purge
persists its obligation atomically with deletion. Slice 83 permits certified
unrelated plain legacy writes during sanitation, then restores the global
mutation fence for identity-free finalization. Publication remains fenced.
One joined Engine-owned worker resumes debt on idle/reopen; finite owned-reader
drain and external checkpoint refusals retain debt without claiming success.

Erasure covers registered original telemetry sink paths prospectively: enabling
a sink registers its canonical path fingerprint durably before capture. Changing
the attached sink does not discharge other registered sinks; reattach each owed
original path and exact-retry. Unbound legacy queue entries refuse; prior
unregistered/rotated copies cannot be reconstructed. See the
[production design](../plans/0.8.28/features/slice-82/design.md) for scope and
crash/finalization invariants.

These functions and their operator-specific reports are absent when `operator`
is disabled.

The `fathomdb` facade re-exports the following recovery and reporting types
from `fathomdb-engine` so that `fathomdb-cli` (the only public consumer of
these types) compiles against the public Rust surface, not engine internals.
These are CLI-only ergonomic types; they are NOT exposed as runtime SDK
verbs (recovery remains CLI-only — see Non-presence below). **Since Slice 27
fix-1 these re-exports — and the `Engine` methods that produce them — are
gated behind the `operator` cargo feature** (which `fathomdb-cli` enables), so
they are absent from the default facade surface (see § Method-level boundary).

Re-exported types (canonical spellings, locked 2026-05-12; extended
2026-05-15):

- `CheckIntegrityOpts`
- `IntegrityReport`
- `SafeExportArtifact`
- `TraceReport`
- `TraceEvent`
- `RebuildReport`
- `RebuildKind`
- `ExciseReport`
- `VerifyEmbedderReport`
- `VerifyEmbedderStatus`
- `DumpSchemaReport`
- `SchemaObject`
- `DumpRowCountsReport`
- `TableRowCount`
- `DumpProfileReport`
- `TruncateWalReport`
- `TruncateWalStatus`
- `CompleteErasuresReport`
- `PendingErasure`

Engine methods backing these types are owned by `design/recovery.md` and
listed in `dev/plans/0.6.0-implementation.md` (Phase 10a + Phase 10b-A).

⚠ **`PurgeLogicalIdReport` / `RestoreLogicalIdReport` — still not shipped, and
NOT the same thing as `Engine::purge`.** These two report types were
forward-referenced for a CLI `purge-logical-id` / `restore-logical-id` pair
(Phase 10b-B, deferred through 0.7.x then 0.8.0). Neither the CLI verbs nor
the report types exist as of 0.8.20, and neither is planned:

- Logical-id lifecycle landed instead as the **governed SDK** verbs
  `Engine::transition` and `Engine::purge` (0.8.19 Slice 10, HITL-SIGNED). They
  are on the DEFAULT facade, return `Result<(), EngineError>`, and produce no
  report type — so they consume neither of the names above.
- **There is no restore counterpart on any surface.** `purge` is irreversible
  by construction; `restore` is also one of the five REQ-054 recovery-denylist
  names, so it can never be an SDK verb.

Do not read the forward reference above as a scheduled deliverable.

## Non-presence

The Rust runtime surface does not expose recovery verbs. Recovery remains CLI
only per `design/recovery.md` and `design/bindings.md`. The re-exported
recovery types above are present as compile-time symbols for `fathomdb-cli`;
the runtime `Engine` does NOT gain corresponding SDK methods.

## Frozen read context (0.8.25 Slice 35)

`ReadContextV1` combines one `ReadView` with one `SearchFilter`.
`Engine::freeze_read_context` resolves its validity instant and returns a
`FrozenReadContextV1` authenticated to the current database read state.
`Engine::search_frozen` and `Engine::search_expand_frozen` accept that complete
object and reject tampering, another database, unsupported versions, or state
drift with `EngineError::FrozenRead`. Frozen combined search and expansion run
on one reader transaction. Eligibility is applied before candidate limits in
the lexical, dense, and graph paths. `search_frozen(..., explain=true)` returns
the same ranked result with a finalized non-empty explanation correlation
identity; explanation-disabled frozen search does not capture telemetry.

## Frozen pagination and operational state (0.8.25 Slice 45)

`Engine::read_canonical_page` returns `PageV1<NodeRecord>` in ascending
`write_cursor` order under one `FrozenReadContextV1`. `PageRequestV1` accepts a
limit from 1 through 250 and an optional opaque `PageCursor`; a continuation is
bound to the database, operation, kind, context, limit, and ordering version.
`Engine::read_operational_state` reads one registered `latest_state` key with
an optional frozen context. `Engine::read_operational_state_page` pages that
same current-state table under a mandatory frozen context. Refusals use
`EngineError::Page(PageError)`; stale authority remains
`EngineError::FrozenRead`.

## Compact source evidence (0.8.25 Slice 50)

`Engine::search_with_evidence(&EvidenceSearchRequestV1)` runs the ordinary
hybrid search under a `FrozenReadContextV1` and returns
`EvidenceSearchResultV1`: the unchanged `SearchResult` plus one positional
`EvidenceSidecarEntryV1` per hit. `Engine::resolve_evidence` accepts the opaque
reference and an equivalent frozen context and returns exact one-source
`ResolvedEvidenceV1` only after current authorization is rechecked.

When requested, the nested explanation has the same finalized correlation
contract as ordinary explained search. Explanation enablement does not change
ranked hits or positional evidence artifact identities, and opaque evidence
reference bytes are not stable across independent calls.

The reference contains keyed commitments, not caller or stored identities.
Resolution is stateless and uses one reader transaction. Ordinary search,
schema 33, and existing result types are unchanged. Failures use
`EngineError::Evidence(EvidenceErrorV1)` with the closed reason vocabulary in
`ADR-0.8.25-compact-source-evidence.md`.

## Dependency trace and structural explanation (0.8.25 Slice 55)

`Engine::trace_dependency(DependencyTraceRequestV1)` is a governed, read-only,
one-hop trace under an authenticated `FrozenReadContextV1`. It accepts only
`to_source` or `to_dependents`, returns the eligible root plus at most 100
registered relations, and fails without a partial result above 101 work units.
Each source link authenticates the SHA-256 of the entire canonical source
body. A `utf8_bytes` locator must also select a nonempty in-bounds range on
UTF-8 character boundaries for trace. The writer and registration accept
empty ranges at valid boundaries.
All `DependencyTrace*V1`, `Trace*V1`, and `DependencyTraceError*` types are
default-facade exports.
`DependencyTraceErrorV1` and the operator-only `DataPlaneIntegrityErrorV1`
carry `schema_version == 1`, a closed reason enum, and an RFC 6901 field path.

Explained search appends `Explanation.correlation_id` and
`PerHitExplain.structural: StructuralInclusionV1`. Ordinary search remains
unchanged. The operator feature separately exposes
`check_data_plane_integrity(DataPlaneIntegrityRequestV1)` and its report types;
it is not a governed SDK method.

## Bounded graph expansion (0.8.25 Slice 60)

`Engine::graph_expand(&GraphExpandRequestV1)` is the governed graph read. It
accepts query or caller-ordered logical seeds, current or authenticated frozen
context, direction, edge and return-only target-kind filters, depth 0 through 3,
result limit 1 through 50, a `u64` work bound, and optional compact explanation.
It returns only a complete deterministic result; exhausting the work bound
raises `EngineError::GraphExpansion` without a partial result.

All `Graph*V1` request, result, explanation, degradation, and error types are
default-facade exports. The operation uses one reader transaction, applies node
validity and eligibility before admission, always enforces shipped edge
recency, and globally selects top-N only after traversal completes. Query
seeding is FTS-first and records its projection status and text-fallback
degradation.

## Exact graph artifact evidence

`GraphExpandRequestV1.include_evidence` is opt-in and defaults to false. It is
accepted only with a frozen graph context. When true,
`GraphExpandResultV1.evidence` has exactly one positional entry per target and
identifies both the target revision and its winning terminal-edge revision with
distinct `GraphEvidenceRefV1` values. The member is absent when not requested.

`Engine::resolve_graph_evidence(&GraphEvidenceResolveRequestV1)` resolves only
one of those opaque references under its equivalent `FrozenReadContextV1`. It
returns `ResolvedGraphEvidenceV1`: the exact node-or-edge artifact, canonical
source bytes and locator, hash, lifecycle, source identities, and optional
direct dependency. It returns no ranking or projection claims. Current context,
raw logical/revision IDs, batching, and schema changes are outside this contract.
`GraphTargetV1.write_cursor` remains part of the graph result; the evidence
sidecar and resolver add no cursor field. Evidence rejects an authenticated
frozen view with `include_out_of_window=true` at
`/context/context/view/includeOutOfWindow` and otherwise applies
start-inclusive, end-exclusive target, terminal-edge, and source validity.
Mutation after context mint is a frozen snapshot drift error before evidence
hydration. For authenticated state, missing provenance is distinguished from an
unavailable linked source only after global target-and-edge authorization.

### Reader-search pause seam (internal tests)

`fathomdb_engine::arm_reader_search_hook_for_test(&Engine)` returns a
`ReaderSearchPauseForTest` guard for that Engine's next reader search before
its snapshot is pinned. This replaces the process-global closure and clear
functions. The arm function, guard type, and guard methods are compiled only
for crate unit tests (`cfg(test)`) or the explicit `fathomdb-engine/test-hooks`
integration-fixture feature. Default dev and release dependency builds omit
them, including when `operator` is enabled. It remains a hidden test seam and
is not exported by the SDK facade.
`wait_ready(Duration)` cancels on timeout/disconnection and clamps readiness waits
to fifteen seconds; the worker release wait has the same upper bound. `release()`
is idempotent, and guard destruction releases a parked reader and disarms an
unconsumed pause on normal return or unwind. Keep the guard local to any scoped
thread closure so cleanup runs before scoped joins on failure. Another Engine
cannot consume or disarm this pause. Arming twice before consumption panics.

## Search behaviour in 0.8.28 (Slice 40)

The signatures are unchanged. The following search behaviour changed or is clarified.

- **Query compilation** (`Engine::search_text_only`, and the text branch of `Engine::search`):
  the query is lowercased, split on non-alphanumeric characters, de-duplicated
  in first-seen order and OR-joined. Tokens of every length are kept, so "C",
  "D3", "II" and the "1" of "IGF-1" are query terms. A fixed English stop list
  is dropped. It now also holds the fragments an apostrophe leaves (`s`, `t`,
  `ll`, `re`, `ve`, `m`) and `my`, `me`, `us`, `am`, `so`, `up`. A query made
  only of stop words falls back to its raw whitespace tokens.
- **Fusion** (`Engine::search_text_only`, `Engine::search`): each branch credits a stored record once, at
  its first original positional rank; repeated occurrences do not renumber later
  records. Canonical node/edge cursors occupy separate namespaces; explicitly
  ownerless node FTS rows use their actual rowid. Distinct versions, sources or
  records remain distinct even with equal body bytes or public IDs. The same
  record found by several arms accumulates one term per arm:
  `score = Σ_branch w_branch / (30 + rank_branch)`, with `w` 3 for the text
  branch and 1 for the vector and graph branches.
  Public IdSpace remains unchanged and can repeat. The SDK context-item helper
  preserves that public ID, and context packing still rejects duplicate item IDs;
  callers can supply unique packing labels with their own selected-record map.
  See [the accepted identity decision](../adr/ADR-0.8.29-retrieval-record-identity.md).
- **`Engine::search_projected_text`**: the projection's FTS5 index is read in rank order and
  reading stops once `limit` rows and any rows tied with the last one are
  read, instead of sorting every match. Results and their order are unchanged:
  bm25 ascending, then write order.
  With an attribute filter whose smallest attribute cohort holds at most 256
  rows, the rank stream skips rows outside that cohort before reading them;
  results, scores and order are the same.
- **Dense retrieval in `Engine::search`**: the vector branch needs a declared
  `SEARCHABLE` projection with `vector: Some(..)` (`Engine::configure_projections`) before the rows are
  written; that declaration enrols the node kind for embedding. Opening with
  an embedder alone embeds nothing, and `Engine::search` then returns text hits
  only. The engine embeds each node's full body, not the projected attribute.
- **Vector pool in `Engine::search`**: the vector branch fuses at least its ten nearest
  surviving rows, whatever the limit. This changes only calls with a limit
  below 10; limits of 10 and above rank as before. For one database state, with
  rerank depth 0 and no graph arm, a search with limit `k` returns the first `k` results of the same
  search with limit `k2` for every `k < k2 <= 10`, with bit-identical scores. Above 10 this is not
  guaranteed.

## Query stop words (0.8.28 Slice 41 phase 2)

`EngineConfig::query_stop_words: Option<QueryStopWords>` sets the query stop
words for one opened engine. `QueryStopWords { base: StopWordBase, extra:
Vec<String>, keep: Vec<String> }` (`StopWordBase::{Fathomdb, None}`, default
`Fathomdb`) gives the effective set `(base ∪ extra) \ keep`; `None` and
`QueryStopWords::default()` are the built-in list. The types, the validation
and the effective-set computation live in `fathomdb-query` (`QueryStopWords::validate`,
`StopWordSet`, `compile_text_query_with`, `DEFAULT_STOP_WORDS_SHA256`) and are
re-exported by `fathomdb-engine`, the `fathomdb` facade and `fathomdb-sdk`.
The rules and the exact-token semantics are in [`python.md`](python.md)
§ Query stop words. A refused setting is
`EngineOpenError::EngineConfiguration(EngineConfigurationError::QueryStopWords {
field, index, reason })`, where `field` is `"query_stop_words.extra"` or
`"query_stop_words.keep"`, `index` is the word's position (`None` for a list
over 256 words) and `reason` is a `StopWordRejection`. The setting applies to
every text-query compile of the engine, is not stored in the database, and is
recorded as `OpenReport::stop_words_sha256` and
`QueryTrace::stop_words_sha256`.

## Context pack (0.8.28 Slice 41)

`fathomdb_engine` exposes the pure context-pack module: `pack_context`,
`encode_round_body`, `decode_round_body`, their JSON-wire forms
(`pack_context_json`, `encode_round_body_json`, `decode_round_body_json`,
used by the Python and TypeScript bindings) and the `Context*` types,
`ROUND_BODY_MARK`, `ROUND_BODY_ROLES` and the `DEFAULT_*` policy constants.
Errors are `ContextPackError`, whose `message()` names the field.
`ContextPolicy::full_ids` and `ContextPackEntry::forced` are the phase 2
full-render override.
Applications use the `fathomdb-sdk` spellings in
[`rust-sdk.md`](rust-sdk.md); the behaviour is in [`python.md`](python.md)
§ Context pack.

## Vector verification open errors

`EngineOpenError::VectorEquivalenceMismatch { reason }` reports actual numeric
refusal. `EngineOpenError::VectorVerification { probe_ordinal, cause }` reports
an inability to verify, preserving the reexported `VectorVerificationFailure`.
Its stable `code()` values are `not_configured`, `saturated`, `queued_expired`,
`started_timeout`, `provider`, `invalid_output`, `panic`, `invalid_state`,
`closing`, `cancelled`. `Provider(RuntimeEmbedderError)` retains the original
provider payload, including CUDA details; `Panic` and `InvalidState` retain their
original message. `probe_ordinal` is zero-based for the 45 stored probes; `0`
is also the convention for whole-baseline/required-mean preflight failure and
does not always identify the first probe. The source chain is open error →
verification cause → original provider error where present. Invalid profile
metadata retains the existing `Corruption` classification; SQL I/O retains `Io`.
Existing bounded cleanup releases database ownership before returning refusal;
later runtime failures retain their operation-specific behavior.

No refused-open `dense_disabled`/`dense_disabled_reason` report fields, engine
accessors or `vector_equivalence_refusal_count` remain. Successful projection
runtime state is `Absent`/`Usable`, with unavailable reason `None`/`NoRuntime`;
obsolete `Refused`/`VectorEquivalenceDisabled` modes are removed. The existing
operation `EngineError::VectorEquivalenceMismatch` and its binding class remain.

These removals do not change `ProjectionDelta.deferred`, durable pending work,
or retained graph/structural codec vocabulary (`deferred`/`projection_deferred`).
Only mappings derived from the removed refused-open runtime state disappear.

## Embedder head coverage (format 35)

The independent `fathomdb-embedder-api` 0.8.0 trait adds
`truncation_offset(&self, input: &str) -> Result<Option<usize>, EmbedderError>`.
The default returns unknown (`None`). A known exclusive UTF-8 byte end must
be within the input and on a character boundary; the input byte length means
full coverage. The provider uses the same tokenizer/truncation policy as
embedding. Projection dispatch captures coverage inside its normal deadline
and failure boundary, and the engine stores it atomically with each vector.
Candle BGE reports exact head coverage; providers that cannot report it
remain unknown. This is metadata, not a new provider admission surface.

## Native vector eligibility (format 35)

Dense search applies direct lifecycle and validity eligibility inside vec0
before its binary candidate cap. Superseded, inactive and out-of-window
vectors do not disable dense retrieval for unrelated eligible records.
Source dependency and unfinished closure barriers retain their conservative
fallback. Canonical hydration checks remain in place.

The internal projection stores lifecycle bits and inclusive validity bounds;
unknown canonical endpoints use min/max sentinels. Updates and erasure resolve
all vector IDs through record identity. An unmapped physical head refuses
erasure transactionally rather than reporting success with residue. The vec0
layout supports at most ten
filterable attributes; an over-capacity configuration fails atomically.
`include_out_of_window` keeps its node-window behavior and the existing
unconditional edge-expiry check. Public `include_superseded` and
`include_inactive` search admission is supported with independent lifecycle gates.

### Maintenance transaction seam (internal tests)

`Engine::maintenance_transaction_for_test` exists only under `cfg(test)` or the
explicit `test-hooks` feature. It runs a trusted callback in an immediate
transaction under the real writer mutex, commits on success and rolls back on
error before releasing that mutex. Callbacks must not issue transaction-control
SQL or reenter the Engine. The result contains the callback value, mutex wait and
successful lock occupancy through transaction finalization; failed calls return
no timing tuple. Closing and storage errors retain their existing Engine meanings.
A callback panic rolls back while unwinding and poisons the mutex. This helper
changes no default admission, public erasure outcome or shipped schema.

## Explicit physical-erasure maintenance (0.8.28 Slice 85)

The [approved contract](../design/explicit-maintenance-erasure/contract.md) and
[production design](../design/explicit-maintenance-erasure/production-design.md)
own format-38 (originally format-37 in `0.8.28`–`0.8.30`) policy, immutable
scope, admission and physical proof.

The lower-level `fathomdb` facade re-exports all twelve maintenance types from
`fathomdb-engine`: `PhysicalErasurePolicy`, `MaintenanceError`, `ErasureServiceState`,
`ErasureMaintenance`, `PhysicalErasureSelector`, `ErasureLogicalState`, `ErasurePhase`,
`ErasureBlocker`, `ErasureStatus`, `QueueResult`, `WithdrawResult`, and `CleanupResult`.

Persisted `PhysicalErasurePolicy::{Autonomous, ExplicitMaintenance}` is selected for fresh
stores with `EngineConfig.physical_erasure_policy`; absent configuration honors
persisted policy on reopen. Missing/unknown policy is corruption, and mismatching
selection refuses before mutation. Both profiles use current-only format 38 on
`0.8.31` (committed Schema 37 stores upgrade offline via `fathomdb migrate`).

`Engine::physical_erasure_policy` observes the effective policy.
`Engine::queue_physical_erasure(selector, key)` returns `QueueResult`;
`physical_erasure_status(ticket)` returns `Option<ErasureStatus>`;
`withdraw_queued_erasure(ticket)` returns `WithdrawResult`.
`PhysicalErasureSelector` is `SourceAllAtCut { source_id }` or
`DeletedLogicalAllAtCut { logical_id }`. The normal new-queue path returns
`NeedsMaintenanceAdmission` when cheap authoritative resource proof is unavailable.
`Accepted { ticket, logical_state: UnchangedByThisRequest }` promises intent only.

`Engine::begin_erasure_maintenance(self: &Arc<Self>, acquisition_wait_ms)` returns
an owned non-cloneable `ErasureMaintenance`, bound to this open Engine. Methods
are `queue_physical_erasure`, `complete_physical_erasure`,
`reattach_original_erasure_sink(ticket, path)` and `close(&mut self)`.
Fallible methods return `Result<_, EngineError>`; close reports service state.
Acquisition wait limits only cooperative drain; it is not an active SQL deadline.
`CleanupResult::{Complete { receipt }, Incomplete { status }}` preserves actual
physical-success meaning. A storage error after commit can still leave durable
owed work; query the ticket afterward. Status is identity-free and contains phase,
logical state, blocker, service state and safe target count.

Newcomers under maintenance refuse `EngineError::Maintenance(MaintenanceBusy)`;
ordinary direct explicit erase/purge refuses `MaintenanceRequired`. Queued plain
writes must prove source/stable-ID/old-new-endpoint disjointness. Unprovable
provenance/dependency/lifecycle/actuation/global admin effects refuse
`QueuedScopeFenced`. Exact identity-bearing pending retries return the same
ticket. Keys are retired at the identity-free Finalizing boundary and remain
retired for terminal receipts; admission returns `KeyRetired`, while retries of
actual finalization use the original ticket. Wrong-store/unknown well-formed
status is None.
Handle drop releases only ephemeral gates; post-delete writes remain fenced
through reopen until actual cleanup. Allowed coherent reads suppress telemetry
capture while fenced. Explicit reopen executes no physical cleanup; original
sink reattachment and offline activated-ticket completion share physical authority.

## Filtered direct body FTS (0.8.28 Slice 90)

The core full-arity entry is
`Engine::search_text_only_filtered(query, Option<SearchFilter>, &ReadView, limit)`.
Existing query/view/limit methods delegate with no filter. Rust SDK
`TextSearchOptions` adds `filter: Option<SearchFilter>` (default `None`).

The existing closed filter applies to node and edge body candidates before
candidate truncation, per-record credit suppression, ranking and returned limit. Attribute
predicates are ANDed, require declared `filterable` projections, and are
node-scoped (edge hits do not match them). Validation occurs in the same reader
snapshot, with typed `InvalidFilter` even when the query has no matches.
Source type uses the established node kind mapping and `edge_fact` for edges;
kind matches the node kind or edge relation kind.

`status` and `created_after` preserve G10 projected-metadata semantics:
status is the empty-string sentinel; creation time is projection time, with an
inclusive lower bound. Missing vector metadata fails to match. These fields do
not read JSON body values or populate metadata. Reading existing metadata does
not execute vector retrieval. Source type, kind and declared attributes work
on provider-free stores; status/time may return no matches there.

ReadView existence and validity axes compose with the filter. No embedding,
dense search, CE rerank, graph expansion or property FTS is invoked. At fixed
corpus, effective validity time and filter, smaller result limits are ordered
prefixes of larger ones. None and empty filters retain unfiltered behavior.

## Stored mapped-head coverage (0.8.29)

`ProjectionRuntimeStatus.vector_coverage: StoredHeadCoverageV1` is required.
Both variants name `population="stored_mapped_spans"` and a
`StoredHeadCoverageProfile { key="default", name, revision, dimension: u32 }`
from the stored profile, independent of current runtime availability.
`Available { counts: StoredHeadCoverageCounts, .. }` contains `records`,
`known_records`, `unknown_records`, `truncated_known_records`,
`unattributed_heads` (u64, at most i64::MAX), `known_body_bytes`,
`unrepresented_known_bytes` (u128), and two nullable
`StoredHeadCoverageFraction { numerator: u128, denominator: u128 }` values:
`truncated_known_share=T/K`, `unrepresented_known_byte_share=V/B`.
Fractions are exact and unreduced, absent iff the denominator is zero.
`Unavailable` instead carries `StoredHeadCoverageUnavailableReason::CacheCold`
or `StateChanged`, with no numeric facts. Metadata, SQL, lock, token and commit
failures remain typed errors.

The population includes attributable committed mapped heads, including retained
history and inactive/out-of-window records, regardless of current declarations.
It excludes canonical records without a mapped head. Passing ownerless,
NULL-body or dual-owner mappings increment only `unattributed_heads`;
original bad-span/orphan predicates and caller-specific completion refusals
remain authoritative. This reports stored representation, not physical-vector
health or current-view coverage. NULL endpoint is unknown even for empty input;
known zero on an empty body is known-empty. R=K+U; T counts known end<length,
B sums known body lengths and V sums their length-end.

Capture adds checked streaming arithmetic to an already-required full writer
validation, with no second corpus pass, physical-vector probe, migration,
reprojection or persisted counter. Only committed captures seed the Engine-local
cache. A pinned writer token compares DB/profile/schema/visibility generation
and nonce plus that same connection's data_version/total_changes. Warm and cold
status reads add fixed token work, with no aggregate owner/span traversal.
Local rollback may conservatively invalidate; an open that initializes existing
provider/repair metadata may stay cold. Availability can remain unavailable
until an existing full status refresh or a stable reopen. The whole MCP orient
operation is not one shared snapshot.

Search retains original vector `RecordKey`s before consuming fusion only when
snippets are requested. After final ranking/snippet selection, each final actual
canonical participant gets at most one indexed tuple read on the same reader
transaction. It validates exact mapping/head/owner/body/physical-row identity,
UTF-8 boundary and checked u32 endpoint; missing, misplaced, extra, ambiguous or
malformed tuples refuse. NULL becomes unknown only after full validation.
Equal bodies, public refs, display arms, explanations and nested graph fusion
cannot lend participation or extents. Text-only/ownerless/standalone paths make
no extent read; snippets-off builds no participation set or coverage sidecar.
Original body bytes include role markers; snippet windows/highlights retain
existing coordinates. SearchHit, ranking, frozen refusals and selectors are unchanged.

## Explicit module model release (0.8.30)

`fathomdb_sdk::release_models() -> Result<()>` releases the cached module-level CLS embedder and cross-encoder
before successful return. Empty/repeated release succeeds without loading
weights or initializing CUDA. It leaves engine-owned providers open. Healthy
later module calls may reload; explicit release clears unavailable/policy
memoization but preserves terminal CUDA generation loss until process restart.

Loading, cache waits, inference or another release cause immediate
`OverloadedError` refusal (Rust `ErrorKind::Overloaded`; TypeScript native
`FDB_OVERLOADED`). Neither cache changes on refusal, and already admitted work
finishes normally. New model work is refused during native teardown. Python
releases the GIL around all native load, wait, inference and release work.

The public Rust `RerankerDevicePolicyError` adds `ModelBusy` (`kind()` is
`"overloaded"`, `ordinal()` is `None`); exhaustive matches must handle it.
Fallible reranker APIs map it to the shared overload class. Infallible
`rerank_fused` keeps its unchanged-input fallback. See
[the ownership decision](../adr/ADR-0.8.30-explicit-module-model-release.md).

## Typed inline properties, operational-collection introspection, and explicit vector enrollment (0.8.31 Slice 20)

`PropertyValue` (`Str(String)`, `Int(i64)`, `Float(f64)`, `Bool(bool)`),
`PropertyMap = BTreeMap<String, PropertyValue>`, `PropertyEntityClass` (`Node`,
`Edge`), `PropertyComparisonOp` (`Eq`, `Ne`, `Lt`, `Lte`, `Gt`, `Gte`),
`PropertyPredicate { key, op, value }`, and `OperationalCollectionInfo`
(`name`, `kind`, `schema_json`, `retention_json`, `format_version`,
`created_at`) are exported from `fathomdb_engine` and re-exported by
`fathomdb`.

- **Writes & validation:** `PreparedWrite::Node`, `PreparedWrite::Edge`, and
  bounded actuation node/edge records carry `properties: Option<PropertyMap>`.
  Empty maps normalize to `None` (`NULL` in SQLite). Non-finite floats, empty
  keys, and keys or string values containing NUL (`\0`) or RS (`\x1e`) are
  refused with `EngineError::WriteValidation`. Negative zero (`-0.0`) is
  canonicalized to `0.0`. `AdminSchema` refuses changing `kind` for an existing
  operational collection name with `EngineError::WriteValidation`.
- **O(1) read hydration:** `NodeRecord`, `SearchHit`, `GraphTargetV1`,
  `ResolvedEvidenceV1`, and `GraphEvidenceArtifactV1` (node/edge variants of
  `ResolvedGraphEvidenceV1`) carry `properties: Option<PropertyMap>`;
  `GraphOriginV1` carries `terminal_edge_properties: Option<PropertyMap>`.
  Values are hydrated directly from canonical row columns with no extra query
  per row.
- **Typed property predicates:** `SearchFilter.property_predicates: Vec<PropertyPredicate>`
  (and `FilterTerm::Property(PropertyPredicate)`) evaluates ANDed comparisons
  against declared node `Filterable` projections using typed SQLite indexes.
  Undeclared keys, non-filterable keys, edge-only keys, and range comparisons
  (`Lt`, `Lte`, `Gt`, `Gte`) against `PropertyValue::Bool` are refused with
  `EngineError::InvalidFilter`.
- **Projection specs:** `ProjectionSpec` adds `entity_class: PropertyEntityClass`
  (default `PropertyEntityClass::Node`) and `vector_kinds: Option<Vec<String>>`
  (explicit opt-in node-kind vector enrollment). Enrolling `"edge_fact"` is
  refused with `EngineError::InvalidArgument`; unenrolled node kinds are
  reported in `vector_unsupported_kinds` without embedding.
- **Operational-collection reads:** `Engine::read_operational_collections(&self) -> Result<Vec<OperationalCollectionInfo>, EngineError>`
  and `Engine::read_operational_collection_info(&self, name: &str) -> Result<Option<OperationalCollectionInfo>, EngineError>`
  inspect registered collections (fresh stores include `projection_failures`).
- **Erasure reporting:** `ExciseReport` adds `completed_pending_erasure: bool`
  (`false` on ordinary erasure; `true` when completing pending post-commit
  erasure work).

## Strict endpoint integrity, typed traversal predicates, and opt-in fused seeding (0.8.31 Slice 30)

- **Strict write endpoint integrity:** Every edge in `Engine::write` — including
  edges superseded later in the same batch under `logical_id` or
  `(from_id, to_id, kind)` rules — must reference active, non-superseded
  endpoint nodes (`superseded_at IS NULL AND state = 'active'`). Missing,
  superseded-only, `Pending`, `Deleted`, or in-batch-deactivated endpoints fail
  the entire batch atomically with `EngineError::WriteValidation` and zero
  committed rows across canonical, revision, property-index, FTS, and projection
  tables. Same-batch forward references (`[Edge(A -> B), Node(A), Node(B)]`) and
  active self-loops commit with `WriteReceipt.dangling_edge_endpoints == 0`.
  Node-only batches execute zero `canonical_nodes` endpoint reads.
  `Engine::actuate` retains `ActuationOutcomeV1::Refused` with reason
  `ReferenceUnavailable` for dangling prospective edges.
- **Mixed-`source_id` batch atomicity:** `Engine::write` executes all items in
  one `BEGIN IMMEDIATE` transaction even when items carry distinct `source_id`
  values; any validation or endpoint refusal rolls back the entire batch
  atomically.
- **`GraphExpandRequestV1` `#[non_exhaustive]` and closed traversal predicates:**
  `GraphExpandRequestV1` is `#[non_exhaustive]` with constructor
  `GraphExpandRequestV1::new(seed, direction, context, max_depth, result_limit, max_work_units)`
  and fluent builders (`with_edge_kinds`, `with_target_kinds`,
  `with_edge_predicates`, `with_target_predicates`, `with_explanation`,
  `with_evidence`). Closed typed `edge_predicates: Vec<PropertyPredicate>` and
  `target_predicates: Vec<PropertyPredicate>` (implicit `AND`, at most `32`
  predicates each) evaluate strictly against inline `properties` (missing keys
  and cross-type comparisons evaluate `false` for all 6 operators; exact 64-bit
  integer precision is preserved; ordering operators on `Bool` and malformed
  predicates return `GraphExpansionErrorReasonV1::GraphEdgePredicatesInvalid` or
  `GraphTargetPredicatesInvalid`). Current-time `graph_expand` pushes
  `superseded_at IS NULL`, `kind`, and bitemporal `[t_valid, t_invalid)` gating
  into indexed SQL (`UNION ALL` for `Both`), so superseded, wrong-kind, expired,
  and future-valid edges consume zero `LIMIT` or `work_units`.
- **Opt-in `use_graph_arm` controls, `SEED_FTS_N` fix, and provenance:**
  `GraphSeedMode` (`GraphFts` default, `FusedTopK`, `Both`),
  `GraphArmSeedSource` (`FusedTopK`, `EdgeFts`, `EntityFts`), and
  `GraphArmOptions` (`graph_seed`, `graph_seed_k` in `1..=100` defaulting to
  `10`, `graph_edge_kinds`, `graph_edge_predicates`) are exported from
  `fathomdb_engine` and re-exported by `fathomdb`. `Engine` exposes
  `search_reranked_view_with_graph_options` and
  `search_frozen_with_graph_options`, and `EvidenceSearchRequestV1` is
  `#[non_exhaustive]` with `EvidenceSearchRequestV1::new(query, context)` plus
  graph-arm builders. Mixed-edge FTS matches never allocate a `SEED_FTS_N = 10`
  slot for an ineligible endpoint; Phase 2 traversal uses `UNION ALL` with edge
  kind/predicate filters; `FusedTopK` and `Both` seed body-less property-graph
  edges from fused/reranked canonical-node hits (`logical_id IS NOT NULL`) with
  single-pass cross-encoder reuse and deterministic `(FusedTopK, EdgeFts, EntityFts)`
  min-hop merging. `PerHitExplain` exposes
  `graph_seed_source: Option<GraphArmSeedSource>` and
  `graph_hop_depth: Option<u32>`, and both `EvidenceGraphOriginV1` variants
  carry `seed_source: GraphArmSeedSource`. `use_graph_arm = false` remains
  byte-identical with zero added SQL or joins.

## Historical bitemporal graph reconstruction (0.8.31 Slice 40)

`GraphExpandRequestV1` adds two optional, mutually exclusive knowledge-time
bounds (`as_of_write_cursor: Option<u64>` and `as_of_epoch_ms: Option<i64>`,
defaulting to `None` in `GraphExpandRequestV1::new`) plus fluent builders
`with_as_of_write_cursor(u64)` and `with_as_of_epoch_ms(i64)`. Both fields
compose orthogonally with world/validity time (`ReadView.valid_as_of` /
`FrozenReadContextV1.effective_valid_at` and `[t_valid, t_invalid)` /
`[valid_from, valid_until)`). `ReadView` remains strictly world-time-only, and
`include_superseded` / `include_inactive` remain refused on `graph_expand`.

- **Closed validation & mutual exclusion:** Setting both `as_of_write_cursor`
  and `as_of_epoch_ms` refuses with `GraphExpansionErrorReasonV1::GraphContextInvalid`
  at `/asOfEpochMs`. On wire, `asOfWriteCursor` is a canonical unsigned decimal
  string and `asOfEpochMs` is a signed 64-bit JSON integer; explicit JSON
  `null` refuses at `/asOfWriteCursor` or `/asOfEpochMs`.
- **Epoch resolution, cursor-`0`, and future cursors:** `as_of_epoch_ms`
  resolves once per reader transaction via an O(log N) single reverse seek on
  `_fathomdb_commit_timestamps_ms_idx`
  (`WHERE committed_at_ms <= ?1 ORDER BY committed_at_ms DESC, write_cursor DESC LIMIT 1`,
  defaulting to cursor `0`). Cursor `0` (including timestamps prior to the
  first Schema-38 commit on fresh or upgraded Schema 37-to-38 stores) yields
  empty `Query` seeds and `GraphSeedUnavailable` for `Explicit` seeds, while
  pre- and post-upgrade revisions remain directly addressable via
  `as_of_write_cursor`. Future cursors (including `> current_write_cursor`,
  `> canonical_write_boundary` on `Frozen`, and `u64::MAX`) saturate safely to
  `i64::MAX` for SQLite binding and act as a monotonic upper bound bounded by
  the pinned snapshot.
- **Zero-penalty default & dual-index `AsOf` traversal:** When both fields are
  `None`, `graph_expand` runs the unchanged current-time fast path
  (`superseded_at IS NULL` via `canonical_edges_{from,to}_active_idx`). When
  historical `AsOf` is active, edge traversal emits direct compound `UNION ALL`
  queries over `canonical_edges_{from,to}_active_idx` and
  `canonical_edges_{from,to}_sup_kind_idx` (with `AND from_id != ?1` on `Both`
  incoming arms to prevent self-loop duplication and `(temporal_fallback IS NULL OR temporal_fallback = 0)`),
  pushes `[t_valid, t_invalid)` and `edge_kinds` into SQL, hydrates node `kind`,
  `body`, `properties`, `write_cursor`, and `origin.terminal_edge_properties`
  from the revisions alive at `as_of`, disables active-only
  `canonical_property_values` pushdown so declared `Filterable` keys evaluate
  inline against historical row JSON (`properties > body`) for both
  `eligibility.attributes` and `eligibility.property_predicates`, and reports
  historical `StructuralDependencyStateV1::Registered` accurately when
  `include_explanation` is enabled.
- **Lifecycle, erasure & historical frozen evidence:** Because
  `Engine::transition` updates `canonical_nodes.state` in place without
  allocating a new `write_cursor`, current-state lifecycle (`state = 'active'`
  on both the historical revision and the logical identity's active head)
  applies to historical reads. Physical erasure (`purge`, `erase_source`),
  active dependency-closure barriers (`phase != 'complete'`), and soft-deleted
  canonical sources strictly subordinate historical reads and prevent nodes,
  derived nodes, and derived edges from reconstructing at any `as_of` cursor or
  timestamp. With `Frozen` context and `include_evidence: true`, `graph_expand`
  mints 704-character `fdbgev1.` references (`payload[7] = 1`, `source_cursor <= c`,
  prefix-free `request_commitment` tags `2u8`/`3u8`), reports
  `artifact_lifecycle.superseded == true` when later superseded in the frozen
  snapshot, and resolves to `EvidenceErrorReasonV1::EvidenceUnavailable` once
  erased, transitioned, or barrier-blocked.

## Batch evidence resolution, search hints, OpenReport capabilities, and operator migrate (0.8.31 Slice 50)

- **Batch evidence resolution:** `fathomdb` re-exports
  `EVIDENCE_RESOLVE_MANY_MAX = 50`, `EvidenceResolveManyRequestV1`, and
  `EvidenceResolveManyItemV1` (`Resolved(Box<ResolvedEvidenceV1>)` |
  `Refused(EvidenceErrorV1)`). `Engine::resolve_evidence_many` checks
  `schema_version == 1`, checks raw length `1..=50` (`EngineError::InvalidArgument`),
  authenticates and validates `FrozenReadContextV1` before and after resolution
  inside one reader transaction (collapsing context errors to whole-call
  `EvidenceUnavailable`), deduplicates repeated tokens while preserving exact
  input order/arity, and returns per-slot nondisclosing `Refused` outcomes at
  `"/evidenceRefs/{i}"`.
- **Opt-in search hints:** `fathomdb` re-exports `Hint`, `HintAction`, `HintArm`,
  `HintCause`, `HintMode`, and `HintReason`. `SearchResult` gains
  `pub hints: Option<Vec<Hint>>` (`None` by default). Additive methods
  `Engine::search_reranked_view_with_hints`, `Engine::search_frozen_with_hints`,
  and `EvidenceSearchRequestV1::with_hints(bool)` compute hints via
  `search_hints` only when `hints == true`.
- **`OpenReport` capability booleans:** `OpenReport` gains
  `reranker_compiled: bool` (`cfg!(feature = "default-reranker")`),
  `cuda_compiled: bool` (`cfg!(any(feature = "embed-cuda", feature = "rerank-cuda"))`),
  and `rerank_available: bool` (local singleton + SHA-256 cache check with zero
  network I/O and zero GPU allocation, preserving fail-closed open on forced
  CUDA policy failure).
- **Operator-only `37 -> 38` migration:** Behind `#[cfg(feature = "operator")]`
  only (verified absent on the default facade by `compile_fail` doctest),
  `fathomdb` re-exports `migrate_37_to_38`, `MigrateReport`,
  `MigratePreflightReport`, `MigrateError`, and `MigrateErrorCode` for
  `fathomdb-cli`. `Engine::open` remains strictly current-schema-only.

## Caller scope isolation, Schema 39, and TABLE_MANIFEST (0.8.32 Slice 20)

- **Default single-scope / single-agent posture:** By default, a FathomDB
  database operates as a single-scope store (`EngineConfig.scope_policy ==
  ScopePolicy::Unscoped`, storing `scope = ""`). Multi-agent or shared-store
  deployments opt into engine-enforced caller scope isolation by configuring
  `ScopePolicy::Scoped { read_scopes, write_scope }` at `Engine::open` time.
- **`ScopePolicy` & validation (`fathomdb` root re-exports):** `fathomdb`
  re-exports `ScopePolicy`, `ScopePolicyError`, `MAX_READ_SCOPES` (`32`), and
  `MAX_SCOPE_TOKEN_BYTES` (`128`):
  - `ScopePolicy::Unscoped` (`Default`) — single-scope default (`scope = ""`).
  - `ScopePolicy::Scoped { read_scopes: Vec<String>, write_scope: Option<String> }`
    with smart constructors `ScopePolicy::single(scope)`,
    `ScopePolicy::read_only(read_scopes)`, and
    `ScopePolicy::read_write(read_scopes, write_scope)`, plus
    `is_unscoped(&self) -> bool`, `read_scopes(&self) -> &[String]`, and
    `write_scope(&self) -> Option<&str>`.
  - Validation (`EngineOpenError::EngineConfiguration(EngineConfigurationError::ScopePolicy { reason })`
    before filesystem/SQLite open): `1..=32` read scopes, non-empty UTF-8 tokens
    `<= 128` bytes, no ASCII control/`NUL`/`\x1e`, no leading/trailing
    whitespace, no wildcard `"*"`, and any `write_scope` must appear in
    `read_scopes`. Canonicalized read-scope sets sort/deduplicate deterministically.
- **Engine accessor & `ScopeViolation`:** `Engine::scope_policy(&self) -> &ScopePolicy`
  returns the validated handle policy. `EngineError::ScopeViolation { detail: String }`
  is returned when a read-only scoped handle attempts a write verb (including
  `write`, `actuate`, `transition`, `purge`, `erase_source`,
  `register_source_dependency`, `begin_erasure_maintenance`,
  `queue_physical_erasure`, `withdraw_queued_erasure`,
  `complete_physical_erasure`, `configure_projections`, `ingest_with_extractor`,
  and `consolidate_with_provider`). D-01 nondisclosure is preserved on cross-scope
  probes: edge endpoints outside `write_scope` fail identically to missing nodes
  (`WriteValidation` / `ActuationRefusalReasonV1::ReferenceUnavailable`), and
  out-of-scope evidence tokens fail with `EvidenceUnavailable` rather than
  `ScopeViolation`.
- **Schema 39 (`S39-COMPOSITE`) & 8-column `vec0`:** `SCHEMA_VERSION = 39`
  upgrades keys/indexes on canonical, provenance, dependency, actuation,
  erasure, and projection tables to scope-leading `(scope, ...)` composite
  identities, expands `vec0` tables to 8 metadata/auxiliary columns including
  `scope TEXT` (preserving `mean.rs` vector storage and parity invariants), and
  scopes visibility/actuation triggers (`_fathomdb_actuation_ref_owner_before_insert`).
- **Frozen contexts & cursors:** `FrozenReadContextV1` authenticates and binds
  the canonicalized `ScopePolicy` SHA-256 digest
  (`fathomdb-scope-policy-v1\x00`), rejecting cross-policy replay with
  `EngineError::FrozenRead`. `PageCursor` carries policy-isolated continuations
  under the same handle binding.
- **Single-source `TABLE_MANIFEST`:** `fathomdb_schema` exposes
  `TABLE_MANIFEST` (`TableManifestEntry`, `TableExportClass`),
  `FTS5_SHADOW_SUFFIXES`, `VEC0_NAMED_SHADOW_SUFFIXES`,
  `VEC0_NUMBERED_SHADOW_PREFIXES`, and `BUILTIN_NON_VISIBILITY_TRIGGERS` as the
  single declarative owner for inspection/export table classification.
- **Operator-only `38 -> 39` migration:** Behind `#[cfg(feature = "operator")]`
  only (verified absent on default `fathomdb` by `compile_fail` doctest),
  `fathomdb` re-exports `migrate_38_to_39` alongside `migrate_37_to_38` (and test
  fault-injection hook helpers stay internal/test-only). `Engine::open` remains
  strictly Schema-39-only.

## Opt-in bulk write mode (`WriteOptions`, `Engine::write_with_options`, 0.8.32 Slice 30)

- **Strict defaults preserved:** `Engine::write` delegates to
  `Engine::write_with_options(batch, &WriteOptions::default())`
  (`EndpointValidationMode::Strict`), rejecting missing, inactive, superseded,
  or cross-scope edge endpoints atomically with `EngineError::WriteValidation`
  and guaranteeing `WriteReceipt.dangling_edge_endpoints == 0` on every commit.
  `Engine::actuate` stays strict-only and never accepts bulk endpoint flags.
- **Re-exported types (`fathomdb` root):**
  - `EndpointValidationMode::{Strict, BulkNonStrict}` (`Default::default() == Strict`).
  - `#[non_exhaustive] WriteOptions { pub endpoint_validation: EndpointValidationMode }`
    with `WriteOptions::strict()`, `WriteOptions::bulk_non_strict()`, and
    `WriteOptions::with_endpoint_validation(self, EndpointValidationMode) -> Self`.
- **Opt-in `BulkNonStrict` semantics:** `Engine::write_with_options(batch, &WriteOptions::bulk_non_strict())`
  admits edges whose same-scope `from`/`to` endpoints do not yet exist as active
  canonical nodes (`superseded_at IS NULL AND state = 'active'`), reporting the
  exact unresolved endpoint count in `WriteReceipt.dangling_edge_endpoints`
  (`0`, `1`, or `2` per surviving active edge written in the batch; node-only /
  non-edge batches take the zero-SQL fast path and return `0`). Subsequent
  same-scope writes supplying the missing nodes resolve graph traversal without
  rewriting edges.
- **Non-bypassable invariants:** Under `BulkNonStrict`, active endpoints
  belonging to a distinct caller `scope` are refused atomically with
  `EngineError::ScopeViolation` (never admitted as dangling); read-only
  `ScopePolicy` handles are refused with `EngineError::ScopeViolation`; and
  anti-spoof `scope` stamping, validity windows, typed scalar `properties`,
  mandatory `SourceId`, `InitialState`, provenance, triggers, and erasure
  invariants remain strictly enforced.
- **Operator verification (`operator` feature):**
  `DataPlaneIntegrityCheckV1::EdgeEndpointIntegrity` (`"edge_endpoint_integrity"`)
  is the 5th closed check in `DataPlaneIntegrityRequestV1::all`, emitting
  content-free `DataPlaneIntegrityFindingCodeV1::DanglingEdgeEndpoint`
  (`Error`) and `CrossScopeEdgeEndpoint` (`Critical`); `check_integrity`
  emits matching `"dangling_edge_endpoint"` / `"cross_scope_edge_endpoint"`
  semantic findings anchored to `design/recovery.md`.
