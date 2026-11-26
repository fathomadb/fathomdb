# Rust API

Crate: `fathomdb` (documentation target `0.8.32`, pre-publish; newest
published crates.io release: `0.8.31`). The facade re-exports the supported application
surface from `fathomdb-engine`; generated item-level documentation is published
at [`docs.rs/fathomdb`](https://docs.rs/fathomdb/). The locked
[Rust interface](https://github.com/fathomadb/fathomdb/blob/main/dev/interfaces/rust.md)
owns the contract and feature-gating details.

## Surface model

This facade is not namespace-identical to the Python and TypeScript SDKs;
for that surface in Rust, use [`fathomdb-sdk`](rust-sdk.md) (`0.8.32` target;
`0.8.31` published). Application
operations are inherent `Engine` methods and public carriers are facade
re-exports. The default feature set contains no recovery-named or raw-SQL
method. The `operator` feature enables the diagnostic and recovery seam used by
`fathomdb-cli`.

The established core remains `Engine::open`, `Engine::write`,
`Engine::search`, `Engine::close`, and schema configuration. `PreparedWrite`
is non-exhaustive; downstream matches require a wildcard arm.

## Process-start runtime configuration

Call `fathomdb::admin::configure_runtime(RuntimeSqliteMode)` before the first
Engine opens. `RuntimeSqliteMode` is `Performance` or `Diagnostics`; success
returns `RuntimeConfiguration`. Identical calls are idempotent, while late or
conflicting calls return `RuntimeConfigurationError`. With no explicit call,
the first open selects performance.

## 0.8.25 data-plane additions

| Capability | Engine method or facade entry point | Principal result |
| ---------- | ----------------------------------- | ---------------- |
| Atomic caller-decided batch | `Engine::actuate` | `ActuationReceiptV1` |
| Source-dependency registration | `Engine::register_source_dependency` | `SourceDependencyV1` |
| Dependencies for a source | `Engine::dependencies_for_source` | `DependencyListV1` |
| Dependency for a derived revision | `Engine::dependency_for_derived` | `Option<SourceDependencyV1>` |
| Keyed closure status | `Engine::read_dependency_closure` | `Option<ClosureStatusV1>` |
| Mint frozen authority | `Engine::freeze_read_context` | `FrozenReadContextV1` |
| Frozen retrieval | `Engine::search_frozen` | `SearchResult` |
| Frozen search plus expansion | `Engine::search_expand_frozen` | `SearchExpandResult` |
| Evidence-bearing retrieval | `Engine::search_with_evidence` | `EvidenceSearchResultV1` |
| Exact evidence resolution | `Engine::resolve_evidence` | `ResolvedEvidenceV1` |
| Exact graph artifact evidence | `Engine::resolve_graph_evidence` | `ResolvedGraphEvidenceV1` |
| Dependency trace | `Engine::trace_dependency` | `DependencyTraceResultV1` |
| Canonical pagination | `Engine::read_canonical_page` | `PageV1<NodeRecord>` |
| Operational-state point read | `Engine::read_operational_state` | `Option<OperationalStateRecordV1>` |
| Operational-state pagination | `Engine::read_operational_state_page` | `PageV1<OperationalStateRecordV1>` |
| Projection-generation status | `Engine::read_projection_generation_status` | `ProjectionGenerationStatusV1` |
| Receipt-keyed mutation readiness | `Engine::read_mutation_projection_status` | `MutationProjectionStatusV1` |
| Constrained graph expansion | `Engine::graph_expand` | `GraphExpandResultV1` |

Versioned provenance is carried by `PreparedWrite::ProvenancedNode` and
`PreparedWrite::ProvenancedEdge`, using `WriteProvenanceV1`, immutable revision
IDs, `CanonicalHash`, and exact `SourceLocator` values. The legacy write
variants remain available.

The frozen-read, evidence, trace, page, projection-generation, graph-expansion,
dependency, closure, and actuation families are closed version-1 contracts.
Rust uses `u64` for native counters and boundaries; Python and TypeScript
serialize the corresponding values as canonical decimal strings.

Explained `search_frozen` and `search_with_evidence` results carry finalized,
non-empty correlation identities. See
[Frozen evidence](../guides/frozen-evidence.md) for the canonical evidence flow
and retry rules.

## 0.8.28 retrieval and maintenance

The core facade exposes `search_text_only_filtered_with_view` with an optional
`SearchFilter`; existing unfiltered entries delegate to it. Eligibility is
applied before direct node/edge body-FTS caps, deduplication, ranking and limit.
See [filter semantics](../guides/hybrid-search-filtering.md) for metadata and
historical-view limits. Application SDK users pass `TextSearchOptions.filter`.

`0.8.28–0.8.30` used format 37, `0.8.31` used format 38, and `0.8.32` uses
format 39 (`S39-COMPOSITE`, with explicit offline `fathomdb migrate` for
committed Schema 38 or chained Schema 37 stores). `PhysicalErasurePolicy` is
persisted; the typed queue/status/withdraw APIs and owned `ErasureMaintenance`
handle implement the
[explicit-maintenance contract](../operations/erasure.md#schedule-an-explicit-maintenance-window).
Open mismatch/verification failures return no Engine. Production provider choice
is built-in default or none, with no caller-provider injection.

## 0.8.31 properties, operational discovery, endpoint integrity, traversal predicates, and fused seeding

- `PreparedWrite` (`Node`, `Edge`, `ProvenancedNode`, `ProvenancedEdge`)
  accepts optional inline JSON `properties`, materialized alongside `body`
  properties into `canonical_property_values` (returned on `NodeRecord` and
  `SearchHit`).
- `SearchFilter.property_predicates` adds typed inline property predicates
  (`eq`, `ne`, `lt`, `lte`, `gt`, `gte`) alongside `attributes`. In frozen
  evidence flows (`search_with_evidence`, `resolve_evidence`,
  `resolve_evidence_many`, `graph_expand(include_evidence = true)`,
  `resolve_graph_evidence`), `property_predicates` is artifact-scoped (cleared
  on canonical source authorization alongside `kind` and `source_type`) while
  access-bearing `attributes`, lifecycle, dependency closures, and `ScopePolicy`
  remain source-enforced; `trace_dependency` retains symmetric endpoint
  enforcement across the full `ReadContextV1.eligibility`. On `GraphArm` hits,
  `SearchHit.source_id` preserves the traversed edge's `source_id` while
  `ResolvedEvidenceV1.source_id` reports the returned node artifact's canonical
  provenance source.
- `ProjectionSpec` adds `entity_class` (`Node` or `Edge` projection targeting)
  and `vector_kinds` (opt-in custom node kinds enrolled in `_fathomdb_vector_kinds`
  alongside the 7 built-in defaults; reserved `"edge_fact"` is rejected).
- `Engine::read_operational_collections` and
  `Engine::read_operational_collection_info` enumerate and inspect declared
  operational collections (conflicting `kind` re-registration is refused with
  `EngineError::WriteValidation`).
- `Engine::write` validates every edge in the batch (including in-batch
  superseded edges and mixed-`source_id` batches) against active,
  non-superseded endpoint nodes (`state = 'active'`, `superseded_at IS NULL`),
  failing atomically with `EngineError::WriteValidation` on violation and
  reporting `WriteReceipt.dangling_edge_endpoints == 0` on every commit.
  Same-batch forward references and active self-loops commit normally;
  `Engine::actuate` retains `ActuationOutcomeV1::Refused`
  (`ReferenceUnavailable`).
- `GraphExpandRequestV1` is `#[non_exhaustive]` with
  `GraphExpandRequestV1::new(...)` and fluent builders for closed inline
  `edge_predicates` and `target_predicates` (`<= 32` each, implicit `AND`;
  malformed predicates return `GraphExpansionErrorReasonV1::GraphEdgePredicatesInvalid`
  or `GraphTargetPredicatesInvalid`) plus optional mutually exclusive
  knowledge-time bounds `as_of_write_cursor: Option<u64>`
  (`.with_as_of_write_cursor(u64)`) and `as_of_epoch_ms: Option<i64>`
  (`.with_as_of_epoch_ms(i64)`). Setting both bounds or passing JSON `null` on
  wire refuses with `GraphExpansionErrorReasonV1::GraphContextInvalid`
  (`/asOfEpochMs` or `/asOfWriteCursor`). Default current-time expansion pushes
  `superseded_at IS NULL`, `kind`, and bitemporal `[t_valid, t_invalid)` gating
  into indexed SQL (`UNION ALL` for `TraversalDirection::Both`); historical
  `as_of` mode composes orthogonally with `ReadView.valid_as_of`, resolves
  `as_of_epoch_ms` via `_fathomdb_commit_timestamps_ms_idx` (cursor `0` yields
  empty query seeds and `GraphSeedUnavailable` for explicit seeds), evaluates
  declared `Filterable` keys inline on superseded revisions, honors
  current-state `Engine::transition` and erasure/closure-barrier subordination,
  and supports `include_evidence: true` with `Frozen` context
  (`artifact_lifecycle.superseded == true` when later superseded).
- `GraphSeedMode`, `GraphArmSeedSource`, and `GraphArmOptions` configure
  opt-in graph-arm seeding (`GraphFts` default, `FusedTopK`, `Both`),
  `graph_seed_k` (`1..=100`), `graph_edge_kinds`, and `graph_edge_predicates`
  on `Engine::search_reranked_view_with_graph_options`,
  `Engine::search_frozen_with_graph_options`, and `#[non_exhaustive]`
  `EvidenceSearchRequestV1::new(...)`. `PerHitExplain` reports
  `graph_seed_source`, `graph_hop_depth`, and `vector_cosine: Option<f64>`
  (pre-RRF cosine similarity in `[-1.0, 1.0]` on the opt-in `explain = true`
  path whenever `vector_rank.is_some()`, `None` otherwise or for zero-norm
  vectors), and `EvidenceGraphOriginV1`
  reports `seed_source`.
- `Engine::resolve_evidence_many`, `EVIDENCE_RESOLVE_MANY_MAX` (`50`),
  `EvidenceResolveManyRequestV1`, and `EvidenceResolveManyItemV1` resolve
  bounded batches of evidence references in one read snapshot with duplicate
  preservation and per-slot nondisclosing refusals (`"/evidenceRefs/{i}"`).
- `Engine::search_reranked_view_with_hints`, `Engine::search_frozen_with_hints`,
  `EvidenceSearchRequestV1::with_hints(bool)`, and `SearchResult.hints` expose
  opt-in agent search hints (`Hint`, `HintAction`, `HintReason`, `HintMode`,
  `HintArm`, `HintCause`) with zero overhead when `false`.
- `OpenReport` exposes `reranker_compiled`, `cuda_compiled`, and
  `rerank_available` without triggering network I/O or GPU allocation.

## 0.8.32 caller scope isolation and Schema 39

- **Single-scope default:** By default a database is single-scope /
  single-agent (`EngineConfig.scope_policy == ScopePolicy::Unscoped`,
  storing `scope = ""`). Multi-agent or shared stores opt into isolation via
  `ScopePolicy` (`single`, `read_only`, `read_write`) and
  `EngineConfig::with_scope_policy` (`EngineConfig` remains non-`#[non_exhaustive]`).
- **Declared WAL durability sync mode (`SyncMode`, 0.8.32):** `fathomdb`
  re-exports `SyncMode::{Normal, Full}` (`as_pragma()` / `as_str()`), configured
  via `EngineConfig.sync_mode: Option<SyncMode>` or
  `EngineConfig::with_sync_mode(SyncMode)`. `None` resolves to
  `SyncMode::Normal` (`PRAGMA synchronous = NORMAL` — zero corruption and
  checkpoint-bounded atomic prefix recovery); opt-in `SyncMode::Full` configures
  `PRAGMA synchronous = FULL` (per-commit WAL `fsync`, strict zero loss of
  client-acknowledged commits on an `fsync`/`FLUSH`/`FUA`-honoring local POSIX
  storage stack) on both writer and projection/runtime connections.
- **Facade exports & accessor:** `fathomdb` re-exports `ScopePolicy`,
  `ScopePolicyError`, `MAX_READ_SCOPES` (`32`), and `MAX_SCOPE_TOKEN_BYTES`
  (`128`); `Engine::scope_policy(&self) -> &ScopePolicy` inspects the bound
  policy, and `FrozenReadContextV1` binds its SHA-256 digest.
- **Scoped `search_expand`:** `Engine::search_expand` and
  `Engine::search_expand_with_limit` take trailing `view: Option<ReadView>`.
- **Schema 39 & `EngineError::ScopeViolation`:** `SCHEMA_VERSION = 39` adds
  composite `(scope, ...)` keys/indexes, 8-column `vec0` (`scope TEXT`), scoped
  triggers, partitioned frozen-read visibility watermarks
  (`_fathomdb_scope_visibility_watermarks` + `global_struct_gen`/`global_struct_nonce`),
  and declarative `fathomdb_schema::TABLE_MANIFEST`. Disjoint-`kind` additive
  writes/actuations, `AppendOnlyLog` appends, `LatestState` upserts (against
  non-`OperationalState` surfaces), disjoint-scope writes/lifecycle/erasure, and
  disjoint-`(scope, kind)` background vector commits produce zero unintended
  drift on `FrozenReadContextV1` (raw FTS5 `bm25()` scores remain corpus-wide
  and are not frozen across disjoint additive writes; in
  `Engine::resolve_evidence_many`, drifted `Edge`/`GraphArm` slots refuse
  per-slot while unaffected node-only slots resolve). Unauthorized writes on
  read-only handles return `EngineError::ScopeViolation`, while cross-scope
  endpoint/evidence lookups preserve D-01 nondisclosure.
- **Opt-in bulk write mode (`EndpointValidationMode`, `WriteOptions`, `Engine::write_with_options`):**
  Default `Engine::write` and `Engine::actuate` remain strictly endpoint-validated
  (`WriteReceipt.dangling_edge_endpoints == 0` on every commit). Re-exported
  `WriteOptions::bulk_non_strict()` (`EndpointValidationMode::BulkNonStrict`)
  with `Engine::write_with_options(&self, batch, options)` admits out-of-order
  same-scope edges and returns exact `WriteReceipt.dangling_edge_endpoints`
  counts (`0`/`1`/`2` per surviving active edge; `0` for node-only batches)
  while still rejecting cross-scope active endpoints and read-only handles
  atomically with `EngineError::ScopeViolation`. Under the `operator` feature,
  `DataPlaneIntegrityCheckV1::EdgeEndpointIntegrity` (`"edge_endpoint_integrity"`)
  and `check_integrity` detect dangling (`dangling_edge_endpoint`) and
  cross-scope (`cross_scope_edge_endpoint`) active edges read-only.
- **Visibility state, `create_if_missing`, and atomic `PutLatestState` (Pin C):**
  `Engine::read_visibility_state` returns `VisibilityStateV1` on one deferred
  reader transaction without acquiring the writer lock (`EP-01`).
  `Engine::open_with_choice_config_and_create` supports
  `create_if_missing: false` (`EP-07`), failing with
  `EngineOpenError::DatabaseNotFound` under the sidecar lock before
  CUDA/embedder setup and leaving no newly created `.lock` or SQLite files
  behind. `ActuationOperationV1::PutLatestState(LatestStateActuationV1)`
  atomically upserts a schema-validated `latest_state` operational checkpoint
  inside `Engine::actuate` under the opener-bound caller `scope`.
- **Shared-bucket `Engine::purge` receipt preservation (`WP-41A`):**
  Single-node `Engine::purge(logical_id)` narrows actuation receipt redaction to
  purged entity/revision references rather than redacting the entire `source_id`
  bucket (preserving idempotent `operation_id` replay for surviving siblings
  sharing `source_id`, while `Engine::erase_source` continues redacting the
  whole `source_id` bucket), and maintains monotonic cursor floors across
  post-purge writes.
- **Query scaling (`WP-41B`):** `Engine::search_with_evidence` with
  `include_explanation = false` captures lightweight `EvidenceContributionV1`
  without triggering `explain = true` cosine/status/`PerHitExplain` overhead,
  and `Engine::read_get` / `Engine::read_get_many` with
  `ReadView { include_superseded: true, .. }` execute an indexed `SEARCH`-only
  SQL lookup without full-table scans.

## Errors

Methods return `EngineError` or `EngineOpenError` and preserve the typed
sub-errors re-exported by the facade, including `ProvenanceError`,
`DependencyError`, `DependencyClosureError`, `ActuationError`,
`FrozenReadError`, `ProjectionGenerationError`, `PageError`, `EvidenceErrorV1`,
`DependencyTraceErrorV1`, and `GraphExpansionErrorV1`. Match non-exhaustive
engine errors with a wildcard arm.

## Operator boundary

Enable the `operator` feature only for diagnostics, migration, or recovery. It
exposes report types and functions used by `fathomdb doctor`,
`fathomdb recover`, and offline `fathomdb migrate` (`migrate_37_to_38`,
`migrate_38_to_39`, `MigrateReport`), including the current
data-plane-integrity report. Python and TypeScript do not expose an SDK
equivalent of the doctor, migrate, or recovery surface.

## See also

- [Install — Rust](../install/rust.md)
- [Python API](python-api.md)
- [TypeScript API](typescript-api.md)
- [CLI](cli.md)
- [Errors](errors.md)
