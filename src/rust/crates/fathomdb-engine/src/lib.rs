//! **FathomDB engine** — the runtime core: storage, projections, ingest and
//! query.
//!
//! ⚠ **Most consumers should depend on the [`fathomdb`] facade crate instead.**
//! `fathomdb` re-exports exactly the governed application surface and gates the
//! operator/recovery seam behind a cargo feature; this crate is the
//! implementation and exposes internals the facade deliberately withholds.
//! Depend on it directly only if you are building FathomDB tooling.
//!
//! [`fathomdb`]: https://docs.rs/fathomdb
//!
//! # What it does
//!
//! One `Engine` owns an embedded SQLite database (FTS5 + `sqlite-vec`), the
//! single writer thread, a thread-affine reader pool serving DEFERRED-tx
//! snapshots, the background projection scheduler, and — optionally — an
//! in-process embedder. There is no server and no sidecar.
//!
//! - **Hybrid retrieval.** A vector branch and an FTS5 branch, fused by
//!   Reciprocal Rank Fusion on ordinal *rank* (never on raw, non-comparable
//!   scores), with an optional cross-encoder rerank and an optional
//!   graph-BFS third arm over temporal fact edges.
//! - **Canonical rows + projections.** Writes land as durable canonical rows;
//!   FTS, vector and attribute indexes are engine-maintained projections
//!   rebuildable from them.
//! - **Record lifecycle.** Transaction-time supersession keyed on `logical_id`,
//!   an existence axis (`transition` / `purge`), and world-time validity
//!   windows on nodes plus `t_valid` / `t_invalid` on edges.
//! - **Deletion on request.** `Engine::erase_source` erases every row carrying a
//!   provenance id — including anonymous rows `Engine::purge` cannot reach —
//!   and finishes the erasure at rest.
//!
//! # Provenance is mandatory
//!
//! `PreparedWrite::Node` and `PreparedWrite::Edge` carry `source_id: SourceId`,
//! a newtype rather than an `Option<String>`. `Engine::erase_source` addresses
//! rows **by** `source_id`, so a row written without one could never be erased;
//! `SourceId::new` is the only public constructor and makes that state
//! inexpressible.
//!
//! # Stability
//!
//! Pre-1.0, so **beta**. `SCHEMA_VERSION` is the on-disk contract; migrations
//! run at open and only there. `PreparedWrite`, `SearchFilter` and
//! `EngineError` are `#[non_exhaustive]` or documented as additive.

mod actuation;
mod connection_runtime;
mod consolidation;
#[cfg(feature = "operator")]
mod data_plane_integrity;
mod dependency;
mod dependency_closure;
mod dependency_trace;
// Foreground routing and bounded close join consume the remaining core seams
// in the next integration batches.
#[allow(dead_code)]
mod embed_dispatch;
mod embedding;
mod erasure;
mod errors;
mod evidence;
mod filter;
mod frozen_read;
mod fusion;
mod graph_api;
mod graph_expand;
mod identity;
mod index_projector;
mod ingest;
pub mod lifecycle;
mod mean;
mod open;
mod operator;
mod pagination;
mod projection_commit;
mod projection_generation;
mod projection_rebuild;
pub use projection_rebuild::{RebuildKind, RebuildReport};
mod projection_registry;
pub use projection_registry::{
    DenseReadiness, ProjectionDelta, ProjectionFts, ProjectionRole, ProjectionSpec,
    ProjectionVector,
};
mod projection_runtime;
pub use projection_runtime::{
    ProjectionRuntimeStatus, ProjectionRuntimeStatusEntry, ProjectionRuntimeUnavailabilityReason,
    ProjectionStatusDenseReadiness,
};
mod projection_worker;
mod provenance;
mod provider;
mod read;
mod read_api;
mod reader_pool;
mod reader_transaction;
mod record_lifecycle;
mod rerank;
mod runtime_configuration;
mod runtime_lifecycle;
mod search;
mod search_api;
#[doc(hidden)]
pub use search_api::slice35_ranked_eligibility_sql_for_test;
#[cfg(feature = "test-hooks")]
#[doc(hidden)]
pub use search_api::take_slice71_search_statement_trace_for_test;
#[doc(hidden)]
pub use search_api::vector_phase1_sql_for_test;
mod search_types;
mod structural_state;
#[cfg(feature = "tc5-benchmark")]
pub mod tc5_benchmark;
mod telemetry;
pub use telemetry::CounterSnapshot;
mod temporal;
mod test_hooks;
mod vector_equivalence;
mod vector_storage;
mod wal_attribution;
mod wal_runtime;
mod write;
mod write_commit;
mod write_types;
mod write_validation;

pub use actuation::{
    ActuationBatchV1, ActuationError, ActuationErrorReason, ActuationOperationV1,
    ActuationOutcomeV1, ActuationReceiptV1, ActuationRefusalReasonV1, LifecycleActuationV1,
};
pub use consolidation::{ConsolidateAxis, ConsolidateCandidateEdge, ConsolidateReceipt};
#[cfg(feature = "operator")]
pub use data_plane_integrity::{
    DataPlaneIntegrityBoundaryV1, DataPlaneIntegrityCheckCountV1, DataPlaneIntegrityCheckV1,
    DataPlaneIntegrityErrorReasonV1, DataPlaneIntegrityErrorV1, DataPlaneIntegrityFindingCodeV1,
    DataPlaneIntegrityFindingV1, DataPlaneIntegrityRequestV1, DataPlaneIntegrityResultV1,
    DataPlaneIntegritySeverityV1,
};
pub(crate) use dependency::{
    apply_validated_source_dependency, canonical_dependency_generation, load_dependency_generation,
    load_persisted_canonical_source, reserve_dependency_generation, store_dependency_generation,
    stored_artifact_revision_id_is_valid, stored_source_id_is_valid, validate_dependency_chain,
    validate_persisted_dependency_row, validate_source_dependency_registration,
    DependencyProspectiveState, DependencyValidationMode, ValidatedSourceDependencyRegistration,
};
pub use dependency::{
    DependencyDerivedLookupV1, DependencyError, DependencyErrorReason, DependencyListV1,
    DependencySourceLookupV1, SourceDependencyRegistrationV1, SourceDependencyV1,
};
pub use dependency_closure::{
    ClosureCauseV1, ClosureLookupV1, ClosureOperationId, ClosurePhaseV1, ClosureProofV1,
    ClosureRootV1, ClosureStatusV1, DependencyClosureError, DependencyClosureErrorReason,
};
#[cfg(feature = "test-hooks")]
#[doc(hidden)]
pub use dependency_trace::DependencyTraceMeasurement;
pub use dependency_trace::{
    decode_dependency_trace_result_v1, encode_dependency_trace_result_v1,
    DependencyTraceDirectionV1, DependencyTraceEdgeV1, DependencyTraceErrorReasonV1,
    DependencyTraceErrorV1, DependencyTraceNodeV1, DependencyTraceRequestV1,
    DependencyTraceResultV1, TraceArtifactClassV1, TraceArtifactRoleV1, TraceNodeLifecycleV1,
    TraceReadBoundaryV1,
};
#[cfg(feature = "test-hooks")]
pub use dependency_trace::{
    decode_dependency_trace_root_for_test, encode_dependency_trace_root_for_test,
};
#[cfg(feature = "test-hooks")]
pub use embed_dispatch::d27_observation::D27Observation;
use embed_dispatch::{DispatchError, EmbedDispatcher, EmbedOutput, EmbedReply};
use embedding::map_runtime_embedder_error;
pub use embedding::{
    EmbedderRequired, EmbeddingOperation, EmbeddingReadiness, EmbeddingReadinessState,
};
pub use erasure::{ExciseRecordReport, ExciseReport};
pub use errors::{
    CorruptionDetail, CorruptionKind, CorruptionLocator, EngineError, EngineOpenError, OpenStage,
    RecoveryHint,
};
pub use evidence::{
    encode_resolved_graph_evidence_v1, EvidenceArmV1, EvidenceArtifactClassV1,
    EvidenceArtifactLifecycleV1, EvidenceContributionV1, EvidenceErrorReasonV1, EvidenceErrorV1,
    EvidenceGraphOriginV1, EvidenceProjectionOriginV1, EvidenceRefV1, EvidenceResolveRequestV1,
    EvidenceSearchRequestV1, EvidenceSearchResultV1, EvidenceSidecarEntryV1,
    GraphEvidenceArtifactV1, GraphEvidenceRefV1, GraphEvidenceResolveRequestV1,
    GraphEvidenceSidecarEntryV1, GraphEvidenceSidecarV1, ResolvedEvidenceV1,
    ResolvedGraphEvidenceV1,
};
pub(crate) use filter::{
    append_node_eligibility_sql, edge_fts_hit_passes_filter, text_hit_passes_filter,
};
pub use filter::{ComparisonOp, Filter, FilterTerm, Predicate, ScalarValue, SearchFilter};
#[cfg(feature = "test-hooks")]
#[doc(hidden)]
pub use frozen_read::Slice45FrozenStageTiming;
#[cfg(feature = "test-hooks")]
#[doc(hidden)]
pub use frozen_read::Slice45MintStageTiming;
pub use frozen_read::{FrozenReadContextV1, FrozenReadError, FrozenReadErrorReason, ReadContextV1};
pub use fusion::{
    apply_importance_reweight, apply_recency_reweight, fuse_rrf, fuse_three_arms, RECENCY_WEIGHT,
    RRF_K, RRF_WEIGHT_GRAPH, RRF_WEIGHT_TEXT, RRF_WEIGHT_VECTOR,
};
pub use graph_expand::{
    decode_graph_expand_request_v1, decode_graph_expand_result_v1, encode_graph_expand_request_v1,
    encode_graph_expand_result_v1, GraphExpandRequestV1, GraphExpandResultV1,
    GraphExpansionDegradationCodeV1, GraphExpansionErrorReasonV1, GraphExpansionErrorV1,
    GraphExpansionExplanationV1, GraphOriginV1, GraphProjectionOriginV1,
    GraphProjectionReadinessV1, GraphReadContextV1, GraphReadModeV1, GraphSeedSourceV1,
    GraphSeedV1, GraphTargetExplanationV1, GraphTargetV1, ResolvedGraphSeedV1, SearchExpandResult,
    TraversalDirection,
};
#[cfg(feature = "test-hooks")]
pub use graph_expand::{
    graph_expansion_degradation_codes_for_test, GraphExpandMeasurementForTest,
    GraphExpandProjectionStateForTest, GraphExpandRendezvousForTest,
};
pub(crate) use identity::{derive_logical_id, derive_stable_id, valid_caller_identity};
pub use identity::{
    ArtifactRevisionId, CanonicalHash, DependencyId, IdSpace, IdSpaceKind, SourceId,
    SourceRevisionId, SourceVersionId,
};
#[cfg(feature = "operator")]
use index_projector::canonical_node_rows;
use index_projector::{
    index_targets_for_row_kind, project_canonical_edge_row, project_canonical_node_row,
    reproject_search_index_after_tokenizer_upgrade, restore_registered_derived_projections,
    search_index_tokenizer_reproject_complete, SEARCH_INDEX_TOKENIZER_SCHEMA_VERSION,
};
pub use ingest::{ExtractDocument, IngestWithExtractorReceipt};
#[doc(hidden)]
pub use mean::mean_centering_internals_for_test;
use mean::{
    identity_requires_mean_centering, read_pinned_mean_vec, recover_mean_vec_pin,
    run_pin_and_requantize_pass, subtract_mean, MeanAccumulator,
};
pub use mean::{MeanRecomputeReport, MEAN_VEC_PIN_THRESHOLD};
#[cfg(feature = "operator")]
pub use operator::{inspect_data_plane_integrity, recover_truncate_wal};
pub use operator::{
    CheckIntegrityOpts, DumpProfileReport, DumpRowCountsReport, DumpSchemaReport, Finding,
    IntegrityReport, OrphanProvenanceReport, OrphanProvenanceSource, SafeExportArtifact,
    SchemaObject, Section, TableRowCount, VerifyEmbedderReport, VerifyEmbedderStatus,
};
pub use pagination::{PageCursor, PageError, PageErrorReason, PageRequestV1, PageV1};
use projection_commit::{
    advance_projection_cursor, commit_projection_outcomes, load_projection_cursor,
    record_projection_terminal, store_projection_cursor, terminal_state_for_cursor,
};
use projection_generation::derive_dense_readiness;
pub use projection_generation::{
    MutationProjectionStatusRequestV1, MutationProjectionStatusV1, ProjectionGenerationError,
    ProjectionGenerationErrorReason, ProjectionGenerationId, ProjectionGenerationOriginV1,
    ProjectionGenerationStatusV1, ProjectionReadinessV1, ProjectionRuntimeStateV1,
};
use projection_registry::{
    boot_graft_declared_vector_backfill, erase_row_projections, load_projection_registry,
    project_node_attributes, purge_row_projections_for_cursor_in,
    reconcile_inert_vector_enrolments_on_boot, rederive_projections_on_boot,
    reenqueue_stranded_vector_rows, register_vector_kind, truncate_row_projections_in,
    unsupported_vector_kinds, validate_nested_projection_sources_for_body,
    validate_nested_projection_sources_for_write, vector_attr_insert_fragments,
    vector_projection_declared, ProjectionClass, ProjectionPass, StoredProjection,
    ROW_OWNED_PROJECTIONS,
};
#[cfg(feature = "operator")]
use projection_registry::{
    extract_scalar_attribute, load_projection_registry_row, truncate_all_row_projections,
};
#[cfg(test)]
use projection_runtime::ProjectionRuntimeStartupFaultForTest;
use projection_runtime::{
    ProjectionJob, ProjectionRuntime, ProjectionRuntimeShared, ProjectionRuntimeStartupMessage,
    ProjectionRuntimeStartupReport, ProjectionRuntimeStartupRole,
};
use projection_worker::{
    connection_has_pending_projection_work, database_has_pending_projection_work,
    pending_embedding_work, projection_dispatcher_loop, projection_worker_loop, ProjectionOutcome,
};
pub(crate) use provenance::ProvenanceRole;
pub use provenance::{
    ProvenanceCompleteness, ProvenanceError, ProvenanceErrorReason, ProvenancedEdgeV1,
    ProvenancedNodeV1, SourceLocator, TraceEvent, TraceReport, WriteProvenanceV1,
};
pub(crate) use provider::{ProviderSession, ProviderTask};
pub use read::{NodeRecord, OpStoreRow, OperationalStateRecordV1};
#[cfg(debug_assertions)]
pub use reader_pool::CacheStatusReply;
pub(crate) use reader_pool::ReaderWorkerPool;
pub use record_lifecycle::{InitialState, LifecycleState};
pub use rerank::rerank_passages;
#[doc(hidden)]
pub use rerank::{rerank_fused, try_rerank_fused};
pub use runtime_configuration::{
    configure_runtime, EngineConfig, EngineConfigurationError, RuntimeConfiguration,
    RuntimeConfigurationError, RuntimeSqliteMode,
};
use runtime_configuration::{
    configure_runtime_for_open, effective_runtime_configuration, ResolvedRuntimeConfiguration,
};
#[cfg(test)]
pub(crate) use search::retain_complete_rank_boundary_candidates;
#[cfg(feature = "test-hooks")]
pub(crate) use search::{append_json_witness_for_test, record_slice71_profile_statement_for_test};
pub(crate) use search::{prepare_search_statement, CapturedGraphOrigin};
pub use search_types::{
    Bm25fFieldWeights, Bm25fQueryPlan, Explanation, GraphFrontierStats, PerHitExplain, QueryTrace,
    SearchHit, SearchResult, SoftFallback, SoftFallbackBranch, StructuralDegradationCodeV1,
    StructuralDependencyStateV1, StructuralInclusionStateV1, StructuralInclusionV1,
    StructuralLifecycleStateV1, StructuralProjectionOriginV1, DEFAULT_SEARCH_RESULT_LIMIT,
    MAX_SEARCH_RESULT_LIMIT, SEARCH_RERANK_LIMIT, TOP_K_BIT_CANDIDATES,
};
#[doc(hidden)]
pub use temporal::clock_reads_for_test;
pub(crate) use temporal::{
    current_epoch_seconds, edge_validity_sql, epoch_seconds_to_iso8601,
    normalize_extractor_timestamp, reject_unrenderable_edge_epoch,
};
pub use temporal::{BoundaryCrossing, ReadView};
#[cfg(feature = "test-hooks")]
pub(crate) use test_hooks::slice15_erasure_lock_hook;
#[cfg(feature = "slice72-test-hooks")]
#[doc(hidden)]
pub use test_hooks::slice72_test_hooks;
#[cfg(debug_assertions)]
pub(crate) use test_hooks::PROJECTION_TRANSACTION_TEST_PAUSE_RELEASE_TIMEOUT;
#[cfg(feature = "test-hooks")]
pub use test_hooks::{
    arm_erasure_before_primary_lock_hook_for_test,
    arm_explanation_after_telemetry_lock_hook_for_test,
    arm_explanation_before_telemetry_lock_hook_for_test,
};
pub use test_hooks::{
    arm_evidence_before_resolve_return_hook_for_test, arm_evidence_before_sidecar_hook_for_test,
    arm_frozen_after_validation_hook_for_test, arm_page_after_validation_hook_for_test,
    arm_reader_search_hook_for_test, ReaderSearchPauseForTest,
};
#[cfg(debug_assertions)]
pub use test_hooks::{ProjectionWorkerPauseReadyError, ProjectionWorkerTransactionPauseForTest};
use vector_equivalence::{run_vector_equivalence_probe, usable_dense_runtime};
use vector_storage::{
    actual_vector_attr_columns, decode_attr_vec0_column, decode_vector_blob,
    default_profile_dimension, delete_vector_partition_row, encode_vector_blob,
    ensure_vector_partition, hamming_bytes, kind_is_vector_committable, kind_is_vector_indexed,
    load_default_profile, quantize_binary_via_sql, reconcile_vector_attr_columns,
    refresh_vector_attr_values, refresh_vector_attr_values_for_row, resolve_source_type,
    VECTOR_COMMITTABLE_NODE_KIND_SOURCE_TYPES,
};
pub(crate) use write::storage_write_shape;
pub use write::{PreparedWrite, WriteReceipt};
pub(crate) use write_commit::{
    advance_read_visibility, apply_batch_in_transaction, canonical_body_hash,
    checked_locator_columns, commit_batch, CommitBatchError, TriggerStateGuard,
};
pub use write_types::RowKind;
pub(crate) use write_validation::{
    collect_projection_jobs, prior_edge_cursors_by_logical_id, prior_edge_cursors_by_triple,
    prior_node_cursors_by_logical_id, validate_batch, validate_write, WritePlan,
};

#[cfg(test)]
use connection_runtime::ProfileReleaseObserver;
use connection_runtime::{
    apply_perf_experiment_reader_pragmas, apply_perf_experiment_writer_pragmas,
    configure_reader_lookaside, install_profile_callback, open_managed_connection,
    open_runtime_connection, register_sqlite_vec_extension, uninstall_profile_callback,
    ProfileContext, ProfileContexts,
};

#[cfg(any(test, feature = "operator"))]
use open::acquire_lock_without_metadata_mutation;
#[cfg(feature = "operator")]
use open::{
    canonical_database_path, classify_wal_sidecar, lock_path, probe_database_header,
    probe_open_integrity, reject_legacy_shape, validate_dependency_generation_on_open, ShmSnapshot,
    WalSidecarHeader,
};
#[cfg(test)]
use open::{
    install_admission_locked_hook_for_test, install_post_probe_startup_fault_for_test,
    install_post_probe_visibility_fault_for_test,
};
#[cfg(test)]
use open::{parse_gpu_allocation_witness_opt_in, LoaderInfo};

use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs::{File, OpenOptions};
use std::hash::BuildHasher;
use std::io::{BufRead, BufReader, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
#[cfg(any(test, debug_assertions, feature = "test-hooks"))]
use std::sync::mpsc::SyncSender;
use std::sync::mpsc::{self, Receiver};
#[cfg(any(test, debug_assertions, feature = "test-hooks"))]
use std::sync::Barrier;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use fathomdb_embedder::{
    DeviceResolution, EmbedDevicePolicyError, EmbedderEvent, GpuAllocationWitness,
    RerankerDevicePolicyError, RerankerDeviceResolution,
};
// `MeanRecomputeTrigger` is used only by the operator-gated `recompute_mean`.
#[cfg(feature = "operator")]
use fathomdb_embedder::MeanRecomputeTrigger;
use fathomdb_embedder_api::{Embedder, EmbedderError as RuntimeEmbedderError, EmbedderIdentity};
use fathomdb_schema::{
    migrate_with_event_sink, MigrationStepReport, LOCK_SUFFIX, MIGRATIONS, SCHEMA_VERSION,
};
// `CANONICAL_TABLES` is used only by the operator-gated `dump_row_counts`.
#[cfg(feature = "operator")]
use fathomdb_schema::CANONICAL_TABLES;
use jsonschema::JSONSchema;
#[cfg(feature = "operator")]
use rusqlite::OpenFlags;
use rusqlite::{config::DbConfig, params, Connection, OptionalExtension};
use serde_json::Value;
// `sha2::Digest` + `sha2::Sha256` — used by `safe_export` (operator-gated)
// and unconditionally by `ingest_with_extractor` (G11 logical_id derivation).
#[cfg(feature = "operator")]
use sha2::Digest;
#[cfg(not(feature = "operator"))]
use sha2::Digest as _;
use sha2::Sha256;

#[cfg(test)]
const PROJECTION_WORKERS: usize = 2;

pub struct Engine {
    path: PathBuf,
    requested_config: EngineConfig,
    #[allow(dead_code)]
    resolved_config: ResolvedRuntimeConfiguration,
    next_cursor: AtomicU64,
    read_visibility_generation: Arc<AtomicU64>,
    projection_generation_status_cache:
        Mutex<Option<projection_generation::CachedProjectionGenerationStatus>>,
    mutation_projection_status_cache:
        Mutex<Option<projection_generation::CachedMutationProjectionStatus>>,
    #[cfg(feature = "test-hooks")]
    projection_generation_status_full_owner_scan_count: AtomicU64,
    #[cfg(feature = "test-hooks")]
    graph_expand_rss_baseline_bytes: AtomicU64,
    #[cfg(feature = "test-hooks")]
    graph_expand_rss_delta_bytes: AtomicU64,
    #[cfg(feature = "test-hooks")]
    graph_evidence_before_resolve_return_hook: Mutex<Option<Box<dyn Fn() + Send>>>,
    #[cfg(feature = "test-hooks")]
    erasure_before_primary_lock_hook: Mutex<Option<Box<dyn Fn() + Send>>>,
    closed: AtomicBool,
    close_lock: Mutex<()>,
    lock: Mutex<Option<File>>,
    connection: Mutex<Option<Connection>>,
    reader_pool: ReaderWorkerPool,
    counters: lifecycle::Counters,
    subscribers: Arc<lifecycle::SubscriberRegistry>,
    profiling_enabled: Arc<AtomicBool>,
    slow_threshold_ms: Arc<AtomicU64>,
    runtime_embedder: Option<Arc<dyn Embedder>>,
    embed_dispatch: Arc<EmbedDispatcher>,
    runtime_embedder_identity: EmbedderIdentity,
    projection_runtime: ProjectionRuntime,
    /// Slice 65 — private, opt-in owner attribution for WAL checkpoint
    /// investigations. It never enters the SDK surface or `EngineError`.
    wal_attribution: Arc<WalAttributionCollector>,
    /// Slice 65 Fix-N: test-only audited inventory of every live Engine-owned
    /// SQLite connection. It is absent from production and binding builds.
    #[cfg(any(test, feature = "test-hooks"))]
    managed_connections: Arc<ManagedConnectionRegistry>,
    #[cfg(any(test, feature = "test-hooks"))]
    writer_connection_registration: Mutex<Option<ManagedConnectionRegistration>>,
    /// Slice 65 follow-on: private observations attached to actual erasure
    /// checkpoint attempts. This is absent from shipping builds and never
    /// changes an erasure outcome or retry policy.
    #[cfg(any(test, feature = "test-hooks"))]
    actual_checkpoint_observations: Mutex<Option<ActualCheckpointObserver>>,
    /// Slice 65 N23-WAL-BINDING-NATIVE-STATE: private observations around the
    /// existing test-hook sampler only. This never arms the normal erasure
    /// observer and never changes a checkpoint result.
    #[cfg(any(test, feature = "test-hooks"))]
    binding_native_state_observations: Mutex<Option<BindingNativeStateObserver>>,
    provenance_row_cap: AtomicU64,
    /// Per-connection profile-callback contexts. Each box's pointer is
    /// installed into the connection's `sqlite3_profile` userdata; the
    /// box must outlive the connection so the callback never reads
    /// freed memory. Connections are dropped before this vec on
    /// `close`/`Drop`, so the lifetime ordering holds.
    ///
    /// Why `Box<ProfileContext>` and not `ProfileContext` directly: the
    /// FFI pointer captured during `install_profile_callback` MUST
    /// remain stable for the connection's lifetime; pushing onto a
    /// `Vec<ProfileContext>` could reallocate and invalidate that
    /// pointer.
    #[allow(clippy::vec_box)]
    profile_contexts: Mutex<ProfileContexts>,
    /// Pack 6.G G.1 — `sqlite3_db_config(LOOKASIDE)` rc per reader
    /// worker, captured at open time before any PRAGMA / prepare ran
    /// on the connection. Read only by the debug-only test accessor
    /// `reader_lookaside_config_rcs_for_test`; held in release builds
    /// too because the field is set unconditionally at open and a cfg
    /// gate would force two open-locked return shapes.
    #[allow(dead_code)]
    reader_lookaside_rcs: Vec<i32>,
    /// 0.8.8 Slice 15 (OPP-9) — opt-in telemetry sink. `None` (default) = OFF.
    /// Local JSONL append; no network/egress. The OFF path never takes this lock —
    /// it is gated by `telemetry_enabled` (below).
    telemetry: Mutex<Option<telemetry::TelemetrySink>>,
    /// 0.8.8 Slice 15 — fast OFF-path guard. `false` (default) → search does ZERO
    /// telemetry work: a single `Relaxed` atomic load, NO mutex acquisition (the
    /// §B.1 footprint / zero-cost gate, codex §9 P2). Set `true` by
    /// `enable_telemetry` after the sink is installed; the `telemetry` mutex is only
    /// ever taken when this flag is set.
    telemetry_enabled: AtomicBool,
    explanation_open_nonce: u128,
    explanation_sequence: AtomicU64,
    /// 0.8.18 Slice 5 (#5 vector-equivalence probe, R-VEQ-4/6) — degraded-open
    /// latch, re-derived at every open by the #5 self-check. `true` ⇒ every
    /// vector-dependent arm refuses at the `search_inner_with_stats` choke point
    /// with `EngineError::VectorEquivalenceMismatch`. Read lock-free on the query
    /// hot path (a single `Relaxed`/`Acquire` load); the text-only/FTS-only path
    /// never reads it.
    dense_disabled: AtomicBool,
    /// R-VEQ-6 — the human-readable reason attached to the query-time refusal (and
    /// surfaced on `OpenReport.dense_disabled_reason`). Set once at open; read only
    /// when `dense_disabled` is `true`.
    dense_disabled_reason: Mutex<Option<String>>,
    /// R-VEQ-6 — telemetry counter: number of query-time vector-dependent-arm
    /// refusals raised because the engine opened in the `dense_disabled` state.
    /// Observable pre/post-query via `vector_equivalence_refusal_count`.
    vector_equivalence_refusals: AtomicU64,
    #[cfg(debug_assertions)]
    force_next_commit_failure: AtomicBool,
    #[cfg(debug_assertions)]
    actuation_after_initial_lookup_delay_ms: AtomicU64,
    #[cfg(debug_assertions)]
    actuation_failure_after_operation: AtomicUsize,
}

impl std::fmt::Debug for Engine {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Engine")
            .field("path", &self.path)
            .field("closed", &self.closed.load(Ordering::SeqCst))
            .field("runtime_embedder_identity", &self.runtime_embedder_identity)
            .finish_non_exhaustive()
    }
}

use wal_attribution::*;
#[cfg(any(test, feature = "test-hooks"))]
use wal_runtime::*;
pub use wal_runtime::{TruncateWalReport, TruncateWalStatus};

#[cfg(test)]
mod gpu_allocation_witness_opt_in_tests {
    use super::{parse_gpu_allocation_witness_opt_in, ENV_GPU_ALLOCATION_WITNESS};

    #[test]
    fn absent_and_empty_are_off() {
        assert_eq!(parse_gpu_allocation_witness_opt_in(None), Ok(false));
        assert_eq!(parse_gpu_allocation_witness_opt_in(Some("")), Ok(false));
        assert_eq!(parse_gpu_allocation_witness_opt_in(Some("   ")), Ok(false));
    }

    #[test]
    fn explicit_on_and_off_are_accepted_case_insensitively() {
        for on in ["1", "true", "TRUE", " True "] {
            assert_eq!(parse_gpu_allocation_witness_opt_in(Some(on)), Ok(true), "{on:?}");
        }
        for off in ["0", "false", "FALSE", " False "] {
            assert_eq!(parse_gpu_allocation_witness_opt_in(Some(off)), Ok(false), "{off:?}");
        }
    }

    #[test]
    fn an_unrecognized_value_is_rejected_rather_than_read_as_off() {
        // The fail-closed arm: `ture` must not quietly disable the evidence.
        let error = parse_gpu_allocation_witness_opt_in(Some("ture"))
            .expect_err("an unrecognized value is not a silent off");
        assert!(error.contains(ENV_GPU_ALLOCATION_WITNESS), "{error}");
        assert!(error.contains("ture"), "{error}");
    }
}

pub use lifecycle::Subscription;
pub use open::{EmbedderChoice, OpenReport, OpenedEngine, ENV_GPU_ALLOCATION_WITNESS};

impl Engine {
    #[cfg(feature = "test-hooks")]
    #[doc(hidden)]
    pub fn explain_graph_evidence_preflights_for_test(&self) -> Result<Vec<String>, EngineError> {
        self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        evidence::explain_intrinsic_preflights_for_test(connection)
    }

    /// Start the private D27 collector after the warm-up drain.
    #[cfg(feature = "test-hooks")]
    pub fn begin_d27_observation_for_test(&self, origin: Instant) {
        self.embed_dispatch.begin_d27_observation(origin);
    }

    /// Run one measured foreground operation under its engine dispatch owner.
    #[cfg(feature = "test-hooks")]
    pub fn with_d27_foreground_owner_for_test<R>(
        &self,
        sequence: usize,
        work: impl FnOnce() -> R,
    ) -> R {
        embed_dispatch::d27_observation::with_owner(
            Some(embed_dispatch::d27_observation::Owner::Foreground {
                operation_sequence: sequence,
            }),
            work,
        )
    }

    /// Snapshot the engine-owned D27 records after measured projection drain.
    #[cfg(feature = "test-hooks")]
    pub fn d27_observation_for_test(&self) -> Option<D27Observation> {
        self.embed_dispatch.d27_observation(
            self.resolved_config.scheduler_runtime_threads,
            self.resolved_config.embedder_pool_size,
        )
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Execute an arbitrary SQL statement on the writer connection through
    /// the same wall-clock + slow-detect path as `write` / `search`.
    ///
    /// Test-only helper for the deterministic-slow-cte fixture used by
    /// AC-007a / AC-007b. Not part of the public 0.6.0 surface; gated on
    /// `debug_assertions` so release builds do not expose it.
    // `test` added alongside `debug_assertions`/`test-hooks` so the crate's own
    // `--release --tests` lib-test build (cfg(test) true, debug_assertions
    // false) can still see this seam; it stays absent from any shipped build.
    #[cfg(any(test, debug_assertions, feature = "test-hooks"))]
    #[doc(hidden)]
    pub fn execute_for_test(&self, sql: &str) -> Result<(), EngineError> {
        self.ensure_open()?;
        let started = Instant::now();
        {
            let mut connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
            let connection = connection.as_mut().ok_or(EngineError::Closing)?;
            connection.execute_batch(sql).map_err(|_| EngineError::Storage)?;
        }
        self.detect_slow(started, lifecycle::EventCategory::Search);
        Ok(())
    }

    /// One-thread-poison robustness fixture (AC-009).
    ///
    /// Spawns four reader threads + one writer thread that all make
    /// forward progress (single canonical write + repeated searches),
    /// plus one designated poison thread that runs an empty-batch write
    /// — a deterministic `EngineError::WriteValidation`. The captured
    /// poison failure is dispatched as a `StressFailureContext` whose
    /// `last_error_chain` is `[EngineError::stable_code(),
    /// engine_error.to_string()]` per the lifecycle § Stress-failure
    /// context payload contract.
    #[doc(hidden)]
    #[cfg(debug_assertions)]
    pub fn run_one_thread_poison_for_test(&self) -> Result<(), EngineError> {
        self.ensure_open()?;

        // Forward-progress writer seeds a row so readers + the poison
        // thread share a non-trivial canonical state.
        self.write(&[PreparedWrite::Node {
            kind: "doc".to_string(),
            body: "poison-fixture-seed".to_string(),
            source_id: SourceId::engine_derived("poison-fixture"),
            logical_id: None,
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        }])?;

        let poison_outcome: Mutex<Option<EngineError>> = Mutex::new(None);
        let poison_thread_id: AtomicU64 = AtomicU64::new(0);

        thread::scope(|scope| {
            // N=4 reader threads make forward progress.
            for _ in 0..4 {
                scope.spawn(|| {
                    for _ in 0..4 {
                        let _ = self.search("poison-fixture-seed");
                    }
                });
            }
            // One forward-progress writer thread.
            scope.spawn(|| {
                let _ = self.write(&[PreparedWrite::Node {
                    kind: "doc".to_string(),
                    body: "writer-progress".to_string(),
                    source_id: SourceId::engine_derived("poison-fixture"),
                    logical_id: None,
                    state: InitialState::Active,
                    reason: None,
                    valid_from: None,
                    valid_until: None,
                }]);
            });
            // One poison thread — empty batch is a deterministic
            // WriteValidation failure.
            scope.spawn(|| {
                // Use a non-zero, deterministic group id so subscribers
                // see a stable identifier across runs of the fixture.
                poison_thread_id.store(1, Ordering::SeqCst);
                if let Err(err) = self.write(&[]) {
                    *poison_outcome.lock().expect("poison_outcome lock") = Some(err);
                }
            });
        });

        let err = poison_outcome
            .into_inner()
            .expect("poison_outcome lock")
            .expect("poison thread must produce a deterministic error");

        let projection_state = match self.projection_status_for_test("doc") {
            Ok(lifecycle::ProjectionStatus::Pending) => "Pending",
            Ok(lifecycle::ProjectionStatus::Failed) => "Failed",
            Ok(lifecycle::ProjectionStatus::UpToDate) => "UpToDate",
            // Default to UpToDate when projection status is unobservable
            // (e.g. embedder not configured for the seed kind). The
            // value is still one of the documented enum stringifications
            // per AC-010.
            Err(_) => "UpToDate",
        };

        let context = lifecycle::StressFailureContext {
            thread_group_id: poison_thread_id.load(Ordering::SeqCst),
            op_kind: "write".to_string(),
            last_error_chain: vec![err.stable_code().to_string(), err.to_string()],
            projection_state: projection_state.to_string(),
        };
        self.subscribers.dispatch_stress_failure(&context);
        Ok(())
    }

    /// EU-5b test seam — drain MeanVecPinned events queued by the
    /// projection-commit pin transaction since the last drain. Production
    /// callers consume these via `OpenReport.embedder_events`; this seam
    /// exists so the EU-5b RED test can observe the live emission.
    #[doc(hidden)]
    pub fn drain_mean_centering_events_for_test(&self) -> Result<Vec<EmbedderEvent>, EngineError> {
        self.ensure_open()?;
        let mut events = self
            .projection_runtime
            .shared
            .pending_events
            .lock()
            .map_err(|_| EngineError::Storage)?;
        let out = std::mem::take(&mut *events);
        Ok(out)
    }

    /// 0.8.20 Slice 15e — run an arbitrary read-only SELECT on the ENGINE
    /// connection (which has the vec0 extension loaded, unlike a bare
    /// `Connection::open`) and collect column 0 as `i64`. Lets a test run a
    /// phase-1-style KNN `MATCH ... {attr clause}` and observe which `rowid`s
    /// survive the pre-KNN filter.
    #[doc(hidden)]
    pub fn query_i64_col_for_test(&self, sql: &str) -> Result<Vec<i64>, EngineError> {
        self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        let mut stmt = connection.prepare(sql).map_err(|_| EngineError::Storage)?;
        let rows =
            stmt.query_map([], |row| row.get::<_, i64>(0)).map_err(|_| EngineError::Storage)?;
        rows.collect::<rusqlite::Result<Vec<i64>>>().map_err(|_| EngineError::Storage)
    }

    /// 0.8.20 Slice 15e — as [`query_i64_col_for_test`] but collects column 0 as
    /// `String` (e.g. an `attr_<hex>` metadata column's stored value).
    #[doc(hidden)]
    pub fn query_text_col_for_test(&self, sql: &str) -> Result<Vec<String>, EngineError> {
        self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        let mut stmt = connection.prepare(sql).map_err(|_| EngineError::Storage)?;
        let rows =
            stmt.query_map([], |row| row.get::<_, String>(0)).map_err(|_| EngineError::Storage)?;
        rows.collect::<rusqlite::Result<Vec<String>>>().map_err(|_| EngineError::Storage)
    }

    #[doc(hidden)]
    pub fn default_embedder_profile_for_test(&self) -> Result<EmbedderIdentity, EngineError> {
        self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        load_default_profile(connection).map_err(|_| EngineError::Storage)
    }

    /// Enumerate schema objects for the no-reverse-table contract test.
    #[cfg(feature = "test-hooks")]
    pub fn schema_objects_for_test(&self) -> Result<Vec<String>, EngineError> {
        self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        let mut statement = connection
            .prepare("SELECT name FROM sqlite_master ORDER BY name")
            .map_err(|_| EngineError::Storage)?;
        let objects = statement
            .query_map([], |row| row.get(0))
            .map_err(|_| EngineError::Storage)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|_| EngineError::Storage)?;
        // Slice 25's accepted actuation receipt lookup index predates the
        // Slice 55 no-new-reverse-state rule and is outside dependency trace.
        Ok(objects
            .into_iter()
            .filter(|name| name != "_fathomdb_actuation_receipt_refs_reverse")
            .collect())
    }

    fn ensure_open(&self) -> Result<(), EngineError> {
        if self.closed.load(Ordering::SeqCst) {
            return Err(EngineError::Closing);
        }

        Ok(())
    }
}

#[cfg(feature = "test-hooks")]
fn record_writer_pragma_witness_for_test(connection: &Connection) {
    let observation = (|| -> rusqlite::Result<serde_json::Value> {
        Ok(serde_json::json!({
            "role": "writer",
            "journal_mode": connection.pragma_query_value(
                None,
                "journal_mode",
                |row| row.get::<_, String>(0),
            )?,
            "synchronous": connection.pragma_query_value(
                None,
                "synchronous",
                |row| row.get::<_, i64>(0),
            )?,
        }))
    })();
    if let Ok(observation) = observation {
        append_json_witness_for_test("FATHOMDB_WRITER_PRAGMA_WITNESS_FOR_TEST", &observation);
    }
}

#[cfg(test)]
mod slice20_fix1_tests;

#[cfg(test)]
mod slice90_close_tests;

#[cfg(test)]
mod slice90_concurrent_close_tests;

#[cfg(test)]
mod slice90_close_review_tests;

#[cfg(test)]
mod slice90_post_probe_real_error_tests;

#[cfg(test)]
mod tests {
    use super::erasure::ERASURE_WAL_TRUNCATE_ATTEMPTS;
    use super::identity::migrated_revision_id_for_test as legacy_revision_id;
    use super::reader_pool::{ReaderRequest, READER_POOL_SIZE};
    use super::vector_storage::KIND_TO_SOURCE_TYPE_CASE_SQL;
    use super::{
        acquire_lock_without_metadata_mutation, derive_stable_id,
        install_admission_locked_hook_for_test, native_connection_state_for_test,
        prepare_search_statement, resolve_source_type, retain_complete_rank_boundary_candidates,
        DeviceResolution, EmbedderChoice, Engine, EngineConfig, EngineError, EngineOpenError,
        IdSpace, IdSpaceKind, InitialState, LoaderInfo, ManagedConnectionRegistry,
        NativeTransactionState, PreparedWrite, ProjectionRuntime,
        ProjectionRuntimeStartupFaultForTest, ProjectionRuntimeStartupRole, RuntimeProbeConnection,
        SearchHit, SoftFallbackBranch, SourceId, WalAttributionCollector, WalAttributionRole,
        PROJECTION_WORKERS, ROW_OWNED_PROJECTIONS,
    };
    use fathomdb_embedder::{
        DeviceResolutionReason, EffectiveEmbedDevice, EmbedDevicePolicy, NoopEmbedder,
    };
    use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
    use proptest::prelude::*;
    use rusqlite::hooks::{AuthAction, AuthContext, Authorization};
    use rusqlite::Connection;
    use std::collections::BTreeSet;
    use std::path::Path;
    use std::process::Command;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::{mpsc, Arc, Barrier, Condvar, Mutex};
    use std::thread;
    use std::time::{Duration, Instant};
    use tempfile::TempDir;

    #[test]
    fn current_opener_holds_lock_before_admission_classification() {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("current-wins.sqlite");
        let rendezvous = Arc::new(Barrier::new(2));
        install_admission_locked_hook_for_test(path.clone(), Arc::clone(&rendezvous));

        let opener_path = path.clone();
        let current = thread::spawn(move || Engine::open(opener_path));
        rendezvous.wait();

        let older_installed_schema = match acquire_lock_without_metadata_mutation(&path) {
            Ok(pending) => {
                let lock = pending.initialize().expect("initialize older lock");
                let connection = Connection::open(&path).expect("open older database");
                connection.pragma_update(None, "user_version", 33).expect("install schema 33");
                connection.close().expect("close older database");
                drop(lock);
                true
            }
            Err(EngineOpenError::DatabaseLocked { holder_pid }) => {
                assert_eq!(holder_pid, None, "metadata remains untouched during admission");
                false
            }
            Err(other) => panic!("unexpected competing opener result: {other:?}"),
        };

        rendezvous.wait();
        assert!(!older_installed_schema, "the losing older opener must not install schema 33");
        let opened = current.join().expect("current opener thread").expect("current open");
        assert_eq!(opened.report.schema_version_before, 0);
        assert_eq!(opened.report.schema_version_after, 34);
        assert_eq!(opened.report.migration_steps.last().map(|step| step.step_id), Some(34));
        opened.engine.close().expect("close current engine");
    }

    #[test]
    fn statement_reuse_refreshes_alternating_bindings() {
        let connection = Connection::open_in_memory().expect("open");
        connection
            .execute_batch(
                "CREATE TABLE values_by_id(id INTEGER PRIMARY KEY, value TEXT);\
                 INSERT INTO values_by_id VALUES(1, 'one'), (2, 'two');",
            )
            .expect("seed");
        let mut reuse = Vec::new();
        for (id, expected) in [(1_i64, "one"), (2, "two"), (1, "one")] {
            let mut statement =
                prepare_search_statement(&connection, "SELECT value FROM values_by_id WHERE id=?1")
                    .expect("prepare");
            reuse.push(statement.was_reused());
            let actual: String = statement.query_row([id], |row| row.get(0)).expect("query");
            assert_eq!(actual, expected);
        }
        assert_eq!(reuse, [false, true, true]);
    }

    #[test]
    fn statement_reuse_releases_rows_and_recovers_after_error() {
        let mut connection = Connection::open_in_memory().expect("open");
        connection
            .execute_batch("CREATE TABLE item(value INTEGER); INSERT INTO item VALUES(7)")
            .expect("seed");
        let transaction = connection.transaction().expect("transaction");
        {
            let mut statement =
                prepare_search_statement(&transaction, "SELECT value FROM item WHERE value=?1")
                    .expect("prepare");
            assert!(statement.query_row([9_i64], |row| row.get::<_, i64>(0)).is_err());
        }
        {
            let mut statement =
                prepare_search_statement(&transaction, "SELECT value FROM item WHERE value=?1")
                    .expect("reprepare");
            assert!(statement.was_reused());
            assert_eq!(statement.query_row([7_i64], |row| row.get::<_, i64>(0)).unwrap(), 7);
        }
        transaction.commit().expect("commit after statement release");
    }

    #[test]
    fn statement_reuse_reprepares_after_schema_change() {
        let connection = Connection::open_in_memory().expect("open");
        connection
            .execute_batch(
                "CREATE TABLE item(id INTEGER PRIMARY KEY, value TEXT);\
                 INSERT INTO item VALUES(1, 'before');",
            )
            .expect("seed");
        {
            let mut statement =
                prepare_search_statement(&connection, "SELECT value FROM item WHERE id=?1")
                    .expect("prepare");
            assert_eq!(
                statement.query_row([1_i64], |row| row.get::<_, String>(0)).unwrap(),
                "before"
            );
        }
        connection.execute_batch("ALTER TABLE item ADD COLUMN extra TEXT").expect("alter");
        let mut statement =
            prepare_search_statement(&connection, "SELECT value FROM item WHERE id=?1")
                .expect("cached prepare");
        assert!(statement.was_reused());
        assert_eq!(statement.query_row([1_i64], |row| row.get::<_, String>(0)).unwrap(), "before");
        assert!(statement.reprepare_count() >= 1, "SQLite must reprepare after schema change");
    }

    #[test]
    fn statement_reuse_recovers_after_concurrent_schema_change() {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("statement-cache.sqlite");
        let reader = Connection::open(&path).expect("reader");
        reader.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE item(id INTEGER PRIMARY KEY, value TEXT); INSERT INTO item VALUES(1, 'before');").expect("seed");
        {
            let mut statement =
                prepare_search_statement(&reader, "SELECT value FROM item WHERE id=?1")
                    .expect("prepare");
            assert_eq!(
                statement.query_row([1_i64], |row| row.get::<_, String>(0)).unwrap(),
                "before"
            );
        }
        let writer_path = path.clone();
        thread::spawn(move || {
            Connection::open(writer_path)
                .expect("writer")
                .execute_batch("ALTER TABLE item ADD COLUMN extra TEXT")
                .expect("alter");
        })
        .join()
        .expect("ddl thread");
        let mut statement = prepare_search_statement(&reader, "SELECT value FROM item WHERE id=?1")
            .expect("cached prepare");
        assert!(statement.was_reused());
        assert_eq!(statement.query_row([1_i64], |row| row.get::<_, String>(0)).unwrap(), "before");
        assert!(statement.reprepare_count() >= 1);
    }

    #[test]
    fn reader_search_pause_is_engine_owned() {
        use super::arm_reader_search_hook_for_test;
        use std::time::Duration;
        let directory = tempfile::TempDir::new().unwrap();
        let intended = Engine::open(directory.path().join("intended.sqlite")).unwrap().engine;
        let unrelated = Engine::open(directory.path().join("unrelated.sqlite")).unwrap().engine;
        let pause = arm_reader_search_hook_for_test(&intended);
        unrelated.search("needle").unwrap();
        assert!(
            pause.wait_ready(Duration::ZERO).is_err(),
            "unrelated engine consumed the intended pause"
        );
        // A readiness timeout cancels even while its guard remains alive.
        intended.search("needle").unwrap();
        assert!(pause.wait_ready(Duration::ZERO).is_err(), "cancelled pause was consumed");
        let timed_out = pause;
        let pause = arm_reader_search_hook_for_test(&intended);
        drop(timed_out); // stale custody must not disarm the replacement pause.
        std::thread::scope(|scope| {
            let search = scope.spawn(|| intended.search("needle"));
            pause.wait_ready(Duration::from_secs(15)).unwrap();
            pause.release();
            search.join().unwrap().unwrap();
        });
        drop(pause);
        // Cancellation before arrival must disarm the seam without parking a worker.
        drop(arm_reader_search_hook_for_test(&intended));
        intended.search("needle").unwrap();
        // Cancellation after arrival must release before a scoped join, including unwind.
        let unwind = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            std::thread::scope(|scope| {
                let pause = arm_reader_search_hook_for_test(&intended);
                let search = scope.spawn(|| intended.search("needle"));
                pause.wait_ready(Duration::from_secs(15)).unwrap();
                let _search = search;
                panic!("controlled pause-owner unwind");
            });
        }));
        assert!(unwind.is_err());
        intended.search("needle").unwrap();
    }

    #[test]
    fn profile_context_release_follows_callback_and_connection_teardown() {
        use super::{Mutex, ProfileReleaseObserver};
        use fathomdb_schema::SQLITE_SUFFIX;
        for explicit_close in [true, false] {
            let directory = tempfile::TempDir::new().unwrap();
            let opened =
                Engine::open(directory.path().join(format!("profile-lifetime{SQLITE_SUFFIX}")))
                    .unwrap();
            let engine = opened.engine;
            for _ in 0..READER_POOL_SIZE {
                engine.search("profile-lifetime").unwrap();
            }
            let observer = Arc::new(ProfileReleaseObserver {
                registry: Arc::clone(&engine.managed_connections),
                live_workers: engine.reader_pool.live_workers_for_test(),
                releases: Mutex::new(Vec::new()),
                custody: Mutex::new(Vec::new()),
            });
            {
                let mut contexts = engine.profile_contexts.lock().unwrap();
                assert_eq!(contexts.contexts.len(), 9, "primary and eight readers");
                assert!(contexts
                    .contexts
                    .iter()
                    .all(|context| !context.callback_uninstalled.load(Ordering::SeqCst)));
                contexts.observer = Some(Arc::clone(&observer));
            }
            assert!(observer.registry.exact_live(PROJECTION_WORKERS));
            if explicit_close {
                engine.close().unwrap();
            }
            drop(engine);
            let releases = observer.releases.lock().unwrap();
            assert_eq!(releases.len(), 9);
            for fact in releases.iter() {
                assert!(
                    fact.callback_uninstalled,
                    "profile callback still installed at context release"
                );
                assert!(
                    fact.live_connections.is_empty(),
                    "managed connections still live: {:?}",
                    fact.live_connections
                );
                assert_eq!(fact.live_workers, 0, "reader workers still live at context release");
            }
            assert!(observer.registry.live.lock().unwrap().is_empty());
            assert_eq!(observer.live_workers.load(Ordering::SeqCst), 0);
        }
    }

    #[test]
    fn reader_request_envelope_stays_bounded_as_search_capabilities_grow() {
        assert!(
            std::mem::size_of::<ReaderRequest>() <= 128,
            "reader requests cross a bounded channel on every query; box capability payloads instead of inflating the envelope (actual={} bytes)",
            std::mem::size_of::<ReaderRequest>()
        );
    }

    #[test]
    fn legacy_revision_derivation_is_stable_and_tuple_sensitive_without_persisting_an_owner() {
        let revision = legacy_revision_id("node", 42, Some("legacy-source"), Some("AéB"));
        assert_eq!(revision, legacy_revision_id("node", 42, Some("legacy-source"), Some("AéB")));
        assert!(revision.starts_with("_fdb:m:"));
        assert_eq!(revision.len(), "_fdb:m:".len() + 64);
        assert!(revision["_fdb:m:".len()..].bytes().all(|byte| byte.is_ascii_hexdigit()));

        for changed in [
            legacy_revision_id("edge", 42, Some("legacy-source"), Some("AéB")),
            legacy_revision_id("node", 43, Some("legacy-source"), Some("AéB")),
            legacy_revision_id("node", 42, None, Some("AéB")),
            legacy_revision_id("node", 42, Some("legacy-source"), Some("different")),
            legacy_revision_id("node", 42, Some("legacy-source"), None),
        ] {
            assert_ne!(changed, revision);
        }
    }

    #[test]
    fn loader_device_resolution_reaches_open_report_once() {
        let dir = TempDir::new().expect("temp dir");
        let embedder: Arc<dyn Embedder> = Arc::new(NoopEmbedder::default());
        let identity = embedder.identity();
        let resolution = DeviceResolution {
            requested_policy: EmbedDevicePolicy::Auto,
            cuda_compiled: false,
            effective_device: EffectiveEmbedDevice::Cpu,
            visible_cuda_devices: Vec::new(),
            selected_cuda_uuid: None,
            reason: Some(DeviceResolutionReason::CudaNotCompiled),
        };
        let opened = Engine::open_with_embedder_and_subscriber(
            dir.path().join("device-resolution.sqlite"),
            identity,
            Some(embedder),
            Some(LoaderInfo {
                download_ms: None,
                events: Vec::new(),
                device_resolution: resolution.clone(),
                gpu_allocation_witness: None,
            }),
            None,
            &mut |_| {},
        )
        .expect("open with the already-resolved default device");

        assert_eq!(opened.report.embedder_device_resolution, Some(resolution));
        // D-80.6-6 — the loader path carries a witness only when it measured
        // one; a resolution alone never synthesizes a record.
        assert_eq!(opened.report.embedder_gpu_allocation_witness, None);
    }

    #[test]
    fn caller_device_resolution_reaches_open_report_once() {
        let dir = TempDir::new().expect("temp dir");
        let resolution = DeviceResolution {
            requested_policy: EmbedDevicePolicy::Auto,
            cuda_compiled: true,
            effective_device: EffectiveEmbedDevice::Cpu,
            visible_cuda_devices: Vec::new(),
            selected_cuda_uuid: None,
            reason: Some(DeviceResolutionReason::CudaProbeFailed),
        };
        let opened = Engine::open_with_choice(
            dir.path().join("caller-device-resolution.sqlite"),
            EmbedderChoice::CallerWithDeviceResolution {
                embedder: Arc::new(NoopEmbedder::default()),
                device_resolution: resolution.clone(),
            },
        )
        .expect("caller-supplied resolution opens");

        assert_eq!(opened.report.embedder_device_resolution, Some(resolution));
    }

    /// Slice 65: the attribution collector starts with every Engine-owned
    /// connection role registered and no active owned snapshot.  This is the
    /// negative control that prevents a later busy checkpoint from being
    /// labelled "external" merely because an idle runtime role was omitted.
    #[test]
    fn wal_attribution_registers_all_owned_roles_idle_at_open() {
        let dir = TempDir::new().expect("temp dir");
        let opened = Engine::open(dir.path().join("wal-attribution.sqlite")).expect("open");

        let snapshot = opened.engine.wal_attribution_snapshot();
        assert_eq!(
            snapshot.roles.iter().filter(|role| role.role == "reader_worker").count(),
            READER_POOL_SIZE,
            "every reader worker must be registered before Engine::open returns"
        );

        assert!(snapshot.no_owned_snapshot, "fresh engine must be idle: {snapshot:?}");
        assert!(!snapshot.local_checkpoint_overlap, "open must not report a checkpoint overlap");
        assert!(snapshot.roles.iter().any(|role| role.role == "writer" && role.index == 0));
        assert_eq!(
            snapshot.roles.iter().filter(|role| role.role == "reader_worker").count(),
            READER_POOL_SIZE,
            "every reader worker must be explicitly registered"
        );
        assert!(snapshot.roles.iter().any(|role| role.role == "projection_dispatcher"));
        assert!(snapshot.roles.iter().all(|role| role.phase == "idle"));
        assert_eq!(
            snapshot.roles.iter().filter(|role| role.role == "projection_worker").count(),
            PROJECTION_WORKERS,
            "every projection worker must be explicitly registered"
        );

        let inventory = opened.engine.native_state_inventory_for_test();
        assert!(inventory.complete, "runtime roles must be immediately queryable: {inventory:?}");
    }

    #[test]
    fn configured_projection_workers_own_exact_connections_and_close_cleanly() {
        let dir = TempDir::new().expect("temp dir");
        for count in [1_u64, 2, 4, 64] {
            let path = dir.path().join(format!("projection-{count}.sqlite"));
            let opened = Engine::open_with_choice_and_config(
                path,
                EmbedderChoice::None,
                EngineConfig { scheduler_runtime_threads: Some(count), ..EngineConfig::default() },
            )
            .expect("configured open");
            let runtime = opened
                .engine
                .projection_runtime
                .report_runtime_connection_inventory_for_test()
                .expect("runtime inventory");
            assert_eq!(runtime.len(), count as usize + 1);
            assert_eq!(
                runtime.iter().filter(|(role, _, auto)| *role == WalAttributionRole::ProjectionWorker && *auto).count(),
                count as usize,
            );
            let snapshot = opened.engine.wal_attribution_snapshot();
            assert_eq!(
                snapshot.roles.iter().filter(|role| role.role == "projection_worker").count(),
                count as usize
            );
            assert_eq!(
                opened.engine.managed_connections.creation_counts(),
                Some((1, READER_POOL_SIZE, 1, count as usize, 0)),
            );
            let native = opened.engine.native_state_inventory_for_test();
            assert!(native.complete, "{count} workers: {native:?}");
            assert_eq!(native.facts.len(), 1 + READER_POOL_SIZE + 1 + count as usize);
            let binding =
                opened.engine.binding_connection_inventory_for_test().expect("binding inventory");
            assert!(binding.contains(&format!("workers:{count}")));
            assert_eq!(
                opened.engine.projection_runtime.shared.admission_capacity,
                count as usize * 64
            );
            opened.engine.close().expect("close all projection owners");
            assert!(opened.engine.managed_connections.live.lock().expect("registry").is_empty());
        }
    }

    #[derive(Debug)]
    struct ProjectionAdmissionGate {
        open: Mutex<bool>,
        entered: AtomicBool,
        cvar: Condvar,
    }

    #[derive(Debug)]
    struct ProjectionAdmissionEmbedder(Arc<ProjectionAdmissionGate>);

    impl Embedder for ProjectionAdmissionEmbedder {
        fn identity(&self) -> EmbedderIdentity {
            EmbedderIdentity::new("projection-admission", "r1", 8)
        }

        fn embed(&self, _text: &str) -> Result<Vector, EmbedderError> {
            let mut open = self.0.open.lock().expect("admission gate");
            self.0.entered.store(true, Ordering::SeqCst);
            self.0.cvar.notify_all();
            while !*open {
                open = self.0.cvar.wait(open).expect("admission gate");
            }
            Ok(vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0])
        }
    }

    struct ReleaseProjectionAdmissionGate(Arc<ProjectionAdmissionGate>);

    impl Drop for ReleaseProjectionAdmissionGate {
        fn drop(&mut self) {
            let mut open = self.0.open.lock().expect("admission gate");
            *open = true;
            self.0.cvar.notify_all();
        }
    }

    #[test]
    fn configured_projection_admission_stops_at_exact_row_capacity() {
        let dir = TempDir::new().expect("temp dir");
        let gate = Arc::new(ProjectionAdmissionGate {
            open: Mutex::new(true),
            entered: AtomicBool::new(false),
            cvar: Condvar::new(),
        });
        let opened = Engine::open_with_choice_and_config(
            dir.path().join("projection-admission.sqlite"),
            EmbedderChoice::Caller(Arc::new(ProjectionAdmissionEmbedder(Arc::clone(&gate)))),
            EngineConfig { scheduler_runtime_threads: Some(1), ..EngineConfig::default() },
        )
        .expect("configured open");
        opened.engine.configure_vector_kind_for_test("doc").expect("vector kind");
        *gate.open.lock().expect("admission gate") = false;
        gate.entered.store(false, Ordering::SeqCst);
        let release = ReleaseProjectionAdmissionGate(Arc::clone(&gate));
        let writes = (0..65)
            .map(|index| PreparedWrite::Node {
                kind: "doc".to_string(),
                body: format!("projection admission {index}"),
                source_id: SourceId::new("projection-admission-source").expect("source"),
                logical_id: Some(format!("admission-{index}")),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            })
            .collect::<Vec<_>>();
        opened.engine.write(&writes).expect("write pending rows");
        let started = Instant::now();
        let mut entered = gate.open.lock().expect("admission gate");
        while !gate.entered.load(Ordering::SeqCst) {
            let (next, timed) = gate.cvar.wait_timeout(entered, Duration::from_secs(5)).unwrap();
            entered = next;
            assert!(!timed.timed_out(), "provider was never called");
        }
        drop(entered);
        let mut state = opened.engine.projection_runtime.shared.state.lock().unwrap();
        while state.active_jobs + state.queued_jobs < 64 {
            let (next, timed) = opened
                .engine
                .projection_runtime
                .shared
                .state_cvar
                .wait_timeout(state, Duration::from_secs(5))
                .unwrap();
            state = next;
            assert!(!timed.timed_out(), "dispatcher did not fill admission");
        }
        assert_eq!(state.active_jobs + state.queued_jobs, 64);
        assert_eq!(state.in_flight.len(), 64);
        drop(state);
        assert!(started.elapsed() < Duration::from_secs(10));
        drop(release);
        opened.engine.drain(30_000).expect("all 65 rows project after release");
        opened.engine.close().expect("close");
    }

    fn initialized_projection_runtime_path(dir: &TempDir, name: &str) -> std::path::PathBuf {
        let path = dir.path().join(name);
        let opened = Engine::open(&path).expect("initialize runtime database");
        opened.engine.close().expect("close initialization engine");
        path
    }

    fn start_projection_runtime_for_test(
        path: std::path::PathBuf,
        timeout: Duration,
        fault: Option<ProjectionRuntimeStartupFaultForTest>,
    ) -> (
        Result<ProjectionRuntime, EngineOpenError>,
        Arc<ManagedConnectionRegistry>,
        Arc<WalAttributionCollector>,
    ) {
        start_projection_runtime_with_count_for_test(
            path,
            PROJECTION_WORKERS as u64,
            timeout,
            fault,
        )
    }

    fn start_projection_runtime_with_count_for_test(
        path: std::path::PathBuf,
        worker_count: u64,
        timeout: Duration,
        fault: Option<ProjectionRuntimeStartupFaultForTest>,
    ) -> (
        Result<ProjectionRuntime, EngineOpenError>,
        Arc<ManagedConnectionRegistry>,
        Arc<WalAttributionCollector>,
    ) {
        let managed_connections = Arc::new(ManagedConnectionRegistry::default());
        let wal_attribution = Arc::new(WalAttributionCollector::new());
        let result = ProjectionRuntime::new_for_test(
            path,
            None,
            NoopEmbedder::default().identity(),
            false,
            Arc::new(super::lifecycle::SubscriberRegistry::new()),
            Arc::clone(&wal_attribution),
            super::ResolvedRuntimeConfiguration::resolve(&EngineConfig {
                scheduler_runtime_threads: Some(worker_count),
                ..EngineConfig::default()
            })
            .unwrap(),
            Arc::clone(&managed_connections),
            timeout,
            fault,
        );
        (result, managed_connections, wal_attribution)
    }

    #[test]
    fn configured_projection_startup_faults_join_every_partial_owner() {
        let faults = [
            ProjectionRuntimeStartupFaultForTest::SetupFailure(
                ProjectionRuntimeStartupRole::Dispatcher(0),
            ),
            ProjectionRuntimeStartupFaultForTest::SetupFailure(
                ProjectionRuntimeStartupRole::Worker(0),
            ),
            ProjectionRuntimeStartupFaultForTest::MissingReport(
                ProjectionRuntimeStartupRole::Worker(1),
            ),
            ProjectionRuntimeStartupFaultForTest::SetupFailure(
                ProjectionRuntimeStartupRole::Worker(2),
            ),
            ProjectionRuntimeStartupFaultForTest::ExitAfterReport(
                ProjectionRuntimeStartupRole::Worker(3),
            ),
        ];
        for (index, fault) in faults.into_iter().enumerate() {
            let dir = TempDir::new().expect("temp dir");
            let path = initialized_projection_runtime_path(
                &dir,
                &format!("configured-runtime-fault-{index}.sqlite"),
            );
            let (result, registry, _) = start_projection_runtime_with_count_for_test(
                path,
                4,
                Duration::from_millis(250),
                Some(fault),
            );
            assert!(matches!(result, Err(EngineOpenError::Io { .. })), "{fault:?}");
            assert!(registry.live.lock().expect("managed registry").is_empty(), "{fault:?}");
        }
    }

    #[test]
    fn configured_projection_workers_are_isolated_between_open_engines() {
        let dir = TempDir::new().expect("temp dir");
        let open = |name, count| {
            Engine::open_with_choice_and_config(
                dir.path().join(name),
                EmbedderChoice::None,
                EngineConfig { scheduler_runtime_threads: Some(count), ..EngineConfig::default() },
            )
            .expect("configured engine")
        };
        let one = open("one.sqlite", 1);
        let four = open("four.sqlite", 4);
        assert_eq!(one.engine.projection_runtime.shared.worker_count, 1);
        assert_eq!(four.engine.projection_runtime.shared.worker_count, 4);
        assert_eq!(one.engine.managed_connections.creation_counts().unwrap().3, 1);
        assert_eq!(four.engine.managed_connections.creation_counts().unwrap().3, 4);
        one.engine.close().expect("close one");
        assert_eq!(four.engine.managed_connections.creation_counts().unwrap().3, 4);
        four.engine.close().expect("close four");
    }

    #[test]
    fn projection_runtime_startup_invalid_path_is_fallible_and_cleans_up() {
        let dir = TempDir::new().expect("temp dir");
        let invalid_path = dir.path().join("missing-parent").join("runtime.sqlite");
        let (result, managed_connections, _) =
            start_projection_runtime_for_test(invalid_path, Duration::from_secs(30), None);

        assert!(matches!(result, Err(EngineOpenError::Io { .. })));
        assert!(
            managed_connections.live.lock().expect("managed registry").is_empty(),
            "failed startup must join every partial runtime connection"
        );
    }

    #[test]
    fn projection_runtime_startup_returns_exact_live_roles_immediately() {
        let dir = TempDir::new().expect("temp dir");
        let path = initialized_projection_runtime_path(&dir, "runtime-startup-success.sqlite");
        let (result, managed_connections, wal_attribution) =
            start_projection_runtime_for_test(path, Duration::from_secs(30), None);
        let runtime = result.expect("runtime startup");
        let expected = BTreeSet::from([
            (WalAttributionRole::ProjectionDispatcher, 0),
            (WalAttributionRole::ProjectionWorker, 0),
            (WalAttributionRole::ProjectionWorker, 1),
        ]);

        assert_eq!(*managed_connections.live.lock().expect("managed registry"), expected);
        let snapshot = wal_attribution.snapshot();
        let observed =
            snapshot.roles.iter().map(|role| (role.role, role.index)).collect::<BTreeSet<_>>();
        assert_eq!(
            observed,
            BTreeSet::from([
                ("projection_dispatcher", 0),
                ("projection_worker", 0),
                ("projection_worker", 1)
            ])
        );
        let facts = runtime
            .report_runtime_native_state_inventory_for_test()
            .expect("runtime roles are immediately queryable");
        assert_eq!(facts.len(), 3);
        assert_eq!(
            facts.iter().map(|fact| (fact.role, fact.index)).collect::<BTreeSet<_>>(),
            expected,
            "successful startup must return the exact service-ready role set"
        );
        assert!(facts.iter().all(|fact| {
            expected.contains(&(fact.role, fact.index))
                && fact.autocommit == Some(true)
                && fact.transaction == NativeTransactionState::None
                && fact.busy_statement == Some(false)
        }));
        runtime.stop();
        assert!(managed_connections.live.lock().expect("managed registry").is_empty());
    }

    #[test]
    fn projection_runtime_idle_probe_retry_honors_deadline() {
        let dir = TempDir::new().expect("temp dir");
        let path = initialized_projection_runtime_path(&dir, "runtime-idle-probe-timeout.sqlite");
        let (result, _, _) = start_projection_runtime_for_test(path, Duration::from_secs(30), None);
        let runtime = result.expect("runtime startup");
        let timeout = Duration::from_millis(25);
        let started = Instant::now();

        assert!(
            !runtime.wait_for_idle(timeout.as_millis() as u64, || None),
            "a persistently busy durable-work probe must time out"
        );
        assert!(
            started.elapsed() <= timeout * 3,
            "the retryable probe must remain bounded by the drain deadline"
        );

        runtime.stop();
    }

    #[test]
    fn projection_runtime_startup_exit_after_report_is_rejected_and_cleans_up() {
        let dir = TempDir::new().expect("temp dir");
        let path = initialized_projection_runtime_path(&dir, "runtime-startup-exit.sqlite");
        let started = Instant::now();
        let (result, managed_connections, _) = start_projection_runtime_for_test(
            path,
            Duration::from_millis(250),
            Some(ProjectionRuntimeStartupFaultForTest::ExitAfterReport(
                ProjectionRuntimeStartupRole::Worker(1),
            )),
        );
        let incorrectly_accepted = result.is_ok();
        if let Ok(runtime) = result {
            runtime.stop();
        }

        assert!(
            !incorrectly_accepted,
            "phase-one setup success must not accept worker:1 after it exits before service readiness"
        );
        assert!(started.elapsed() < Duration::from_secs(2), "failed startup cleanup wedged");
        assert!(
            managed_connections.live.lock().expect("managed registry").is_empty(),
            "exit-after-report left a live runtime connection"
        );
    }

    #[test]
    fn projection_runtime_startup_protocol_faults_stop_and_join_partial_runtime() {
        let cases = [
            ProjectionRuntimeStartupFaultForTest::SetupFailure(
                ProjectionRuntimeStartupRole::Worker(0),
            ),
            ProjectionRuntimeStartupFaultForTest::DuplicateReport {
                role: ProjectionRuntimeStartupRole::Worker(1),
                reported_as: ProjectionRuntimeStartupRole::Worker(0),
            },
            ProjectionRuntimeStartupFaultForTest::MissingReport(
                ProjectionRuntimeStartupRole::Worker(1),
            ),
            ProjectionRuntimeStartupFaultForTest::StallUntilStop(
                ProjectionRuntimeStartupRole::Worker(1),
            ),
        ];

        for (index, fault) in cases.into_iter().enumerate() {
            let dir = TempDir::new().expect("temp dir");
            let path = initialized_projection_runtime_path(
                &dir,
                &format!("runtime-startup-fault-{index}.sqlite"),
            );
            let started = Instant::now();
            let (result, managed_connections, _) =
                start_projection_runtime_for_test(path, Duration::from_millis(50), Some(fault));

            assert!(matches!(result, Err(EngineOpenError::Io { .. })), "fault {fault:?}");
            assert!(started.elapsed() < Duration::from_secs(2), "fault {fault:?} cleanup wedged");
            assert!(
                managed_connections.live.lock().expect("managed registry").is_empty(),
                "fault {fault:?} left a live runtime connection"
            );
        }
    }

    /// Slice 65 managed-reader witness: preserve the original typed refusal,
    /// then observe post-finish connection state without retrying that erase.
    #[test]
    fn wal_attribution_owned_reader_typed_refusal_then_post_release_sampler_is_recorded() {
        let dir = TempDir::new().expect("temp dir");
        let opened = Engine::open(dir.path().join("wal-attribution-owned.sqlite")).expect("open");
        opened
            .engine
            .write(&[PreparedWrite::Node {
                kind: "doc".to_string(),
                body: "owned reader erasable body".to_string(),
                source_id: SourceId::new("slice65-owned-reader").expect("source"),
                logical_id: Some("slice65-owned-reader".to_string()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            }])
            .expect("write");
        let (completion_ready, completion_release, reader_autocommit) =
            opened.engine.arm_next_reader_completion_pause_for_test();
        let (snapshot_ready, release) = opened.engine.pause_reader_after_wal_snapshot_for_test();
        snapshot_ready.wait();
        eprintln!("slice65_wal managed_reader_snapshot_ready");
        let blocked = opened.engine.erase_source("slice65-owned-reader");
        let busy = opened.engine.wal_attribution_checkpoints_for_test();
        assert_eq!(busy.len(), ERASURE_WAL_TRUNCATE_ATTEMPTS as usize);
        assert!(busy.iter().enumerate().all(|(offset, record)| {
            record.attempt == offset + 1
                && record.busy
                && record.classification == "owned_reader_snapshot"
                && record.active_roles == vec![(WalAttributionRole::ReaderWorker, 0)]
        }));
        assert!(matches!(blocked, Err(EngineError::ErasureIncomplete { .. })));
        eprintln!(
            "slice65_wal managed_reader_original_erase=typed_erasure_incomplete owned_busy_attempts={}",
            busy.len()
        );

        release.wait();
        completion_ready.wait();
        assert!(
            reader_autocommit.load(std::sync::atomic::Ordering::Acquire),
            "post-finish reader connection must be autocommit"
        );
        let completion = opened.engine.wal_attribution_snapshot();
        assert!(completion.no_owned_snapshot, "collector must be idle after reader finish");
        eprintln!(
            "slice65_wal managed_reader_completion_ack reader_autocommit=1 collector_roles=idle"
        );
        completion_release.wait();

        let inventory = opened.engine.native_state_inventory_for_test();
        let inventory_text = super::native_state_inventory_text(&inventory);
        eprintln!("slice65_wal managed_reader_native_state_inventory={inventory_text}");
        if !inventory.complete {
            eprintln!(
                "slice65_wal managed_reader_native_state_inventory=state_inventory=incomplete"
            );
        }
        assert!(inventory.complete, "post-finish native inventory: {inventory:?}");

        opened.engine.arm_binding_native_state_observation_for_test();
        let samples = opened
            .engine
            .checkpoint_at_rest_for_test()
            .expect("run bounded post-finish checkpoint sampler");
        assert!(!samples.is_empty() && samples.len() <= ERASURE_WAL_TRUNCATE_ATTEMPTS as usize);
        let state_records = opened.engine.drain_binding_native_state_observations_for_test();
        assert_eq!(state_records.len(), samples.len() * 2);
        for state in &state_records {
            eprintln!("slice65_wal managed_reader_sampler_native_state {state}");
            if !state.contains("state_inventory=complete reason=complete") {
                eprintln!(
                    "slice65_wal managed_reader_sampler_native_state state_inventory=incomplete"
                );
            }
            assert!(state.contains("state_inventory=complete reason=complete"));
        }
        let (busy, log_frames, checkpointed_frames) =
            samples.last().copied().expect("sampler result");
        eprintln!(
            "slice65_wal managed_reader_sampler_terminal outcome={} attempts={} busy={} log_frames={} checkpointed_frames={}",
            if busy { "busy" } else { "clean" },
            samples.len(),
            u8::from(busy),
            log_frames,
            checkpointed_frames,
        );
    }

    /// Slice 65 N23-WAL-BINDING-NATIVE-STATE RED: an autocommit connection
    /// may still own a stepped SQLite statement.  The diagnostic must retain
    /// the direct owning-connection state, then observe it reset after the
    /// rows/statement values are dropped.
    #[test]
    fn wal_attribution_native_state_observes_busy_statement_then_reset() {
        let connection = Connection::open_in_memory().expect("open");
        connection
            .execute_batch(
                "CREATE TABLE state_probe(value INTEGER); INSERT INTO state_probe VALUES (1);",
            )
            .expect("seed");
        let mut statement = connection.prepare("SELECT value FROM state_probe").expect("prepare");
        let mut rows = statement.query([]).expect("query");
        assert!(rows.next().expect("step").is_some(), "step one real row");

        let stepped =
            native_connection_state_for_test(&connection, WalAttributionRole::ReaderWorker, 0);
        assert_eq!(stepped.autocommit, Some(true), "implicit stepped reads remain autocommit");
        assert_eq!(stepped.transaction, NativeTransactionState::Read);
        assert_eq!(stepped.busy_statement, Some(true), "stepped rows keep their statement busy");

        drop(rows);
        let reset =
            native_connection_state_for_test(&connection, WalAttributionRole::ReaderWorker, 0);
        assert_eq!(reset.busy_statement, Some(false), "dropping Rows resets the statement");
        drop(statement);
        let dropped =
            native_connection_state_for_test(&connection, WalAttributionRole::ReaderWorker, 0);
        assert_eq!(dropped.transaction, NativeTransactionState::None);
        assert_eq!(
            dropped.busy_statement,
            Some(false),
            "dropping Statement leaves no busy statement"
        );
    }

    #[test]
    fn wal_attribution_native_state_inventory_requires_all_managed_roles_idle() {
        let dir = TempDir::new().expect("temp dir");
        let opened =
            Engine::open(dir.path().join("wal-attribution-native-state.sqlite")).expect("open");
        let inventory = opened.engine.native_state_inventory_for_test();
        assert!(inventory.complete, "idle engine inventory: {inventory:?}");
        assert_eq!(inventory.facts.len(), 1 + READER_POOL_SIZE + 1 + PROJECTION_WORKERS);
        assert!(inventory.facts.iter().all(|fact| {
            fact.autocommit == Some(true)
                && fact.transaction == NativeTransactionState::None
                && fact.busy_statement == Some(false)
                && matches!(fact.reply, super::NativeStateReply::Received)
        }));

        *opened
            .engine
            .binding_native_state_observations
            .lock()
            .expect("binding native state observation") =
            Some(super::BindingNativeStateObserver { records: Vec::new() });
        opened.engine.binding_native_state_observation_for_test("before", 1);
        let records = opened
            .engine
            .binding_native_state_observations
            .lock()
            .expect("binding native state observation")
            .take()
            .expect("armed observer")
            .records;
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].phase, "before");
        assert_eq!(records[0].ordinal, 1);
        assert!(records[0].inventory.complete);
        assert!(super::native_state_inventory_text(&records[0].inventory)
            .contains("state_inventory=complete"));
    }

    /// Slice 65 follow-on: establish whether the observed post-release busy
    /// checkpoint occurs before or after SQLite has actually committed the held
    /// reader. This is a diagnostic only: it deliberately runs exactly one
    /// existing fail-closed erase, then records independent raw checkpoints.
    /// It never retries, relabels, or completes that original erasure.
    #[test]
    fn wal_attribution_post_commit_acknowledges_and_records_raw_checkpoint_diagnostic() {
        const CHILD_PATH: &str = "FATHOMDB_SLICE65_POST_COMMIT_CHILD_PATH";

        if let Some(path) = std::env::var_os(CHILD_PATH) {
            let report = raw_post_commit_checkpoint(Path::new(&path), 0, None, "after_close");
            eprintln!(
                "slice65_wal post_commit_child_raw case=after_close outcome=recorded raw_busy={} raw_log_frames={} raw_checkpointed_frames={}",
                report.busy, report.log_frames, report.checkpointed_frames,
            );
            return;
        }

        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("wal-attribution-post-commit.sqlite");
        let opened = Engine::open(&path).expect("open");
        opened
            .engine
            .write(&[PreparedWrite::Node {
                kind: "doc".to_string(),
                body: "post commit diagnostic".to_string(),
                source_id: SourceId::new("slice65-post-commit-source").expect("source"),
                logical_id: Some("slice65-post-commit".to_string()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            }])
            .expect("write");

        let (snapshot_ready, release, committed) = opened.engine.post_commit_ack_for_test();
        snapshot_ready.wait();
        let blocked = opened.engine.erase_source("slice65-post-commit-source");
        assert!(
            matches!(blocked, Err(EngineError::ErasureIncomplete { .. })),
            "the diagnostic must retain the single existing fail-closed erase"
        );
        release.wait();
        committed.wait();

        let inventory = match post_commit_connection_inventory(&opened) {
            Ok(inventory) => inventory,
            Err(reason) => {
                eprintln!("slice65_wal post_commit_inventory=incomplete reason={reason}");
                panic!("post-COMMIT inventory is incomplete: {reason}");
            }
        };
        eprintln!("slice65_wal post_commit_ack direct_inventory={inventory} collector_roles=idle");
        let reports = [
            raw_post_commit_checkpoint(&path, 1, Some(&opened), "pre_close"),
            raw_post_commit_checkpoint(&path, 2, Some(&opened), "pre_close"),
        ];
        let pre_close_busy = reports.iter().any(|report| report.busy != 0);
        if pre_close_busy {
            opened.engine.close().expect("close Engine before child probe");
            let output = Command::new(std::env::current_exe().expect("current test executable"))
                .arg("--exact")
                .arg("tests::wal_attribution_post_commit_acknowledges_and_records_raw_checkpoint_diagnostic")
                .arg("--nocapture")
                .env(CHILD_PATH, &path)
                .output()
                .expect("run fresh-child raw checkpoint probe");
            assert!(
                output.status.success(),
                "fresh-child raw checkpoint probe failed with status {:?}",
                output.status
            );
            eprint!("{}", String::from_utf8_lossy(&output.stderr));
        } else {
            eprintln!("slice65_wal post_commit_child_raw case=after_close outcome=not_required");
        }
        eprintln!("slice65_wal post_commit_diagnostic=recorded");
    }

    fn post_commit_connection_inventory(
        opened: &super::OpenedEngine,
    ) -> Result<String, &'static str> {
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut snapshot = opened.engine.wal_attribution_snapshot();
        while snapshot.roles.len() < 1 + READER_POOL_SIZE + 1 + PROJECTION_WORKERS
            && Instant::now() < deadline
        {
            thread::sleep(Duration::from_millis(5));
            snapshot = opened.engine.wal_attribution_snapshot();
        }
        if snapshot.roles.len() != 1 + READER_POOL_SIZE + 1 + PROJECTION_WORKERS
            || !opened.engine.managed_connections.exact_live(PROJECTION_WORKERS)
        {
            return Err("registry_mismatch");
        }
        let creation =
            opened.engine.managed_connections.creation_counts().ok_or("creation_audit_lock")?;
        eprintln!(
            "slice65_wal post_commit_creation expected=writer:1,readers:8,dispatcher:1,workers:2,probes:0 actual=writer:{},readers:{},dispatcher:{},workers:{},probes:{}",
            creation.0, creation.1, creation.2, creation.3, creation.4,
        );
        if creation != (1, READER_POOL_SIZE, 1, PROJECTION_WORKERS, 0) {
            return Err("creation_counts_mismatch");
        }
        if !snapshot.no_owned_snapshot
            || snapshot.roles.iter().any(|role| role.active || role.phase != "idle")
        {
            return Err("collector_not_idle");
        }
        let writer_autocommit = opened
            .engine
            .connection
            .lock()
            .map_err(|_| "writer_lock")?
            .as_ref()
            .is_some_and(Connection::is_autocommit);
        if !writer_autocommit {
            return Err("writer_not_autocommit");
        }
        let readers = opened.engine.reader_pool.wal_connection_inventory_for_test();
        if readers.len() != READER_POOL_SIZE || readers.into_iter().any(|autocommit| !autocommit) {
            return Err("reader_not_autocommit");
        }
        let runtime =
            opened.engine.projection_runtime.report_runtime_connection_inventory_for_test()?;
        let expected = BTreeSet::from([
            (WalAttributionRole::ProjectionDispatcher, 0),
            (WalAttributionRole::ProjectionWorker, 0),
            (WalAttributionRole::ProjectionWorker, 1),
        ]);
        let actual =
            runtime.iter().map(|(role, index, _)| (*role, *index)).collect::<BTreeSet<_>>();
        if actual != expected || runtime.iter().any(|(_, _, autocommit)| !autocommit) {
            return Err("runtime_not_autocommit");
        }
        Ok(format!(
            "writer:autocommit;readers:8-autocommit;dispatcher:autocommit;workers:2-autocommit;expected_creation=writer:1,readers:8,dispatcher:1,workers:2,probes:0;actual_creation=writer:{},readers:{},dispatcher:{},workers:{},probes:{}",
            creation.0, creation.1, creation.2, creation.3, creation.4,
        ))
    }

    fn raw_post_commit_checkpoint(
        path: &Path,
        sample: usize,
        opened: Option<&super::OpenedEngine>,
        case: &str,
    ) -> super::TruncateWalReport {
        let started = Instant::now();
        if let Some(opened) = opened {
            assert!(
                !opened.engine.closed.load(std::sync::atomic::Ordering::SeqCst),
                "Engine must remain open for pre-close raw diagnostic samples"
            );
            assert!(
                opened.engine.wal_attribution_snapshot().no_owned_snapshot,
                "collector must be idle for pre-close raw diagnostic samples"
            );
        }
        let connection = Connection::open(path).expect("independent raw sqlite open");
        connection.busy_timeout(Duration::ZERO).expect("raw checkpoint no wait");
        let (busy, log_frames, checkpointed_frames): (i64, i64, i64) = connection
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .expect("independent raw checkpoint");
        connection.close().expect("independent raw sqlite close");
        let report = super::TruncateWalReport {
            status: if busy == 0 {
                super::TruncateWalStatus::Done
            } else {
                super::TruncateWalStatus::Busy
            },
            busy: busy.max(0) as u32,
            log_frames: log_frames.max(0) as u32,
            checkpointed_frames: checkpointed_frames.max(0) as u32,
            discarded_corrupt_wal: false,
        };
        let inventory = opened
            .map(post_commit_connection_inventory)
            .transpose()
            .unwrap_or_else(|reason| panic!("post-COMMIT inventory is incomplete: {reason}"))
            .unwrap_or_else(|| "engine:closed".to_string());
        eprintln!(
            "slice65_wal post_commit_raw case={case} sample={sample} elapsed_ms={} raw_busy={} raw_log_frames={} raw_checkpointed_frames={} inventory={inventory}",
            started.elapsed().as_millis(), report.busy, report.log_frames, report.checkpointed_frames,
        );
        report
    }

    #[derive(Clone, Debug)]
    struct Slice65ProjectionEmbedder;

    impl Embedder for Slice65ProjectionEmbedder {
        fn identity(&self) -> EmbedderIdentity {
            EmbedderIdentity::new("slice65", "wal-attribution", 8)
        }

        fn embed(&self, _text: &str) -> Result<Vector, EmbedderError> {
            Ok(vec![1.0; 8])
        }
    }

    /// Slice 65: retaining a materialized result must not leave a managed
    /// reader snapshot alive when the erasure checkpoint begins. Windows can
    /// still report an unattributed BUSY checkpoint after every managed role
    /// is idle, so that typed refusal is a valid outcome of this ownership
    /// control rather than evidence that the retained result owns a snapshot.
    #[test]
    fn wal_attribution_retained_materialized_result_is_idle_at_checkpoint() {
        let dir = TempDir::new().expect("temp dir");
        let opened =
            Engine::open(dir.path().join("wal-attribution-retained.sqlite")).expect("open");
        opened
            .engine
            .write(&[PreparedWrite::Node {
                kind: "doc".to_string(),
                body: "retained materialized result".to_string(),
                source_id: SourceId::new("slice65-retained-source").expect("source"),
                logical_id: Some("slice65-retained".to_string()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            }])
            .expect("write");
        let retained = opened
            .engine
            .read_get("slice65-retained", &Default::default())
            .expect("read")
            .expect("materialized node");
        assert_eq!(retained.logical_id, "slice65-retained");
        let idle = opened.engine.wal_attribution_snapshot();
        assert!(
            idle.no_owned_snapshot,
            "retained result must not retain a SQLite snapshot: {idle:?}"
        );
        let before = opened.engine.wal_attribution_checkpoints_for_test().len();
        let checkpoint = opened.engine.erase_source("slice65-retained-source");
        let records = opened.engine.wal_attribution_checkpoints_for_test();
        let records = &records[before..];
        assert!(!records.is_empty(), "erase must record at least one checkpoint attempt");
        assert!(records.iter().all(|record| record.active_roles.is_empty()));
        assert!(records.iter().all(|record| {
            (!record.busy && record.classification == "no_owned_snapshot")
                || (record.busy && record.classification == "unclassified_external")
        }));
        let record = records.last().expect("checkpoint record");
        let outcome = match checkpoint {
            Ok(_) => {
                assert!(!record.busy && record.classification == "no_owned_snapshot");
                "clean"
            }
            Err(EngineError::ErasureIncomplete { stage, .. }) if stage == "wal_checkpoint" => {
                assert!(record.busy && record.classification == "unclassified_external");
                "unclassified_external"
            }
            Err(error) => panic!("unexpected retained-result checkpoint outcome: {error}"),
        };
        eprintln!("slice65_wal retained_materialized_idle=passed outcome={outcome}");
    }

    /// Slice 65 reader-handoff RED: a materialized `read_get` result must be
    /// handed back only after its collector state is already idle. This parks
    /// the worker after the helper has dropped its SQLite transaction and
    /// before its response send, then proves no managed reader owns the
    /// checkpoint while the caller still cannot receive that materialized
    /// record. A typed busy result remains possible when Windows has an
    /// unrelated WAL holder.
    #[test]
    fn wal_attribution_reader_handoff_is_idle_before_materialized_reply() {
        let dir = TempDir::new().expect("temp dir");
        let opened = Arc::new(
            Engine::open(dir.path().join("wal-attribution-reader-handoff.sqlite")).expect("open"),
        );
        opened
            .engine
            .write(&[PreparedWrite::Node {
                kind: "doc".to_string(),
                body: "reader handoff materialized body".to_string(),
                source_id: SourceId::new("slice65-reader-handoff-source").expect("source"),
                logical_id: Some("slice65-reader-handoff".to_string()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            }])
            .expect("write");

        let (handoff_ready, release) = opened.engine.pause_next_reader_handoff_for_test();
        let (caller_result_tx, caller_result_rx) = mpsc::sync_channel(1);
        let caller = {
            let engine = Arc::clone(&opened);
            thread::spawn(move || {
                let result = engine.engine.read_get("slice65-reader-handoff", &Default::default());
                caller_result_tx.send(result).expect("caller result receiver remains live");
            })
        };
        handoff_ready.wait();

        assert!(
            matches!(caller_result_rx.try_recv(), Err(mpsc::TryRecvError::Empty)),
            "public read_get must not return while the pre-send handoff barrier is held"
        );

        let idle = opened.engine.wal_attribution_snapshot();
        assert!(
            idle.no_owned_snapshot,
            "the collector must be idle before the materialized response send: {idle:?}"
        );
        let checkpoint = opened.engine.erase_source("slice65-reader-handoff-source");
        let record = opened
            .engine
            .wal_attribution_checkpoints_for_test()
            .last()
            .cloned()
            .expect("checkpoint record");
        assert!(record.active_roles.is_empty());
        let outcome = match checkpoint {
            Ok(_) => {
                assert!(!record.busy && record.classification == "no_owned_snapshot");
                "clean"
            }
            Err(EngineError::ErasureIncomplete { stage, .. }) if stage == "wal_checkpoint" => {
                assert!(record.busy && record.classification == "unclassified_external");
                "unclassified_external"
            }
            Err(error) => {
                panic!("unexpected checkpoint result while materialized reply is parked: {error}")
            }
        };

        release.wait();
        let materialized = caller_result_rx
            .recv()
            .expect("caller result after handoff release")
            .expect("read result")
            .expect("materialized record");
        caller.join().expect("reader thread");
        assert_eq!(materialized.logical_id, "slice65-reader-handoff");
        eprintln!("slice65_wal reader_handoff_idle_before_reply=passed outcome={outcome}");
    }

    #[test]
    fn wal_attribution_reader_handoff_pause_is_scoped_to_its_engine() {
        let first_dir = TempDir::new().expect("first temp dir");
        let second_dir = TempDir::new().expect("second temp dir");
        let first = Arc::new(
            Engine::open(first_dir.path().join("wal-attribution-first.sqlite"))
                .expect("first open"),
        );
        let second = Arc::new(
            Engine::open(second_dir.path().join("wal-attribution-second.sqlite"))
                .expect("second open"),
        );

        let (first_ready, first_release) = first.engine.pause_next_reader_handoff_for_test();
        let (second_result_tx, second_result_rx) = mpsc::sync_channel(1);
        let second_reader = {
            let engine = Arc::clone(&second);
            thread::spawn(move || {
                second_result_tx
                    .send(engine.engine.read_get("missing", &Default::default()))
                    .expect("second result receiver remains live");
            })
        };

        if second_result_rx.recv_timeout(Duration::from_secs(1)).is_err() {
            first_ready.wait();
            first_release.wait();
            second_reader.join().expect("second reader join");
            panic!("a first-engine handoff pause blocked a second-engine reader");
        }
        second_reader.join().expect("second reader join");

        let (first_result_tx, first_result_rx) = mpsc::sync_channel(1);
        let first_reader = {
            let engine = Arc::clone(&first);
            thread::spawn(move || {
                first_result_tx
                    .send(engine.engine.read_get("missing", &Default::default()))
                    .expect("first result receiver remains live");
            })
        };
        first_ready.wait();
        assert!(matches!(first_result_rx.try_recv(), Err(mpsc::TryRecvError::Empty)));
        first_release.wait();
        first_result_rx.recv().expect("first result after release").expect("first read");
        first_reader.join().expect("first reader join");
    }

    #[test]
    fn wal_attribution_reader_snapshot_pause_is_scoped_to_its_engine() {
        let first_dir = TempDir::new().expect("first temp dir");
        let second_dir = TempDir::new().expect("second temp dir");
        let first = Arc::new(
            Engine::open(first_dir.path().join("wal-attribution-first.sqlite"))
                .expect("first open"),
        );
        let second = Arc::new(
            Engine::open(second_dir.path().join("wal-attribution-second.sqlite"))
                .expect("second open"),
        );

        let (first_ready, first_release, native_state) =
            first.engine.arm_next_reader_snapshot_pause_for_test();
        let (second_result_tx, second_result_rx) = mpsc::sync_channel(1);
        let second_reader = {
            let engine = Arc::clone(&second);
            thread::spawn(move || {
                second_result_tx
                    .send(engine.engine.read_get("missing", &Default::default()))
                    .expect("second result receiver remains live");
            })
        };

        if second_result_rx.recv_timeout(Duration::from_secs(1)).is_err() {
            first_ready.wait();
            first_release.wait();
            second_reader.join().expect("second reader join");
            panic!("a first-engine snapshot pause blocked a second-engine reader");
        }
        second_reader.join().expect("second reader join");

        let (first_result_tx, first_result_rx) = mpsc::sync_channel(1);
        let first_reader = {
            let engine = Arc::clone(&first);
            thread::spawn(move || {
                first_result_tx
                    .send(engine.engine.read_get("missing", &Default::default()))
                    .expect("first result receiver remains live");
            })
        };
        first_ready.wait();
        assert!(matches!(first_result_rx.try_recv(), Err(mpsc::TryRecvError::Empty)));
        assert!(native_state.lock().expect("native state").is_some());
        first_release.wait();
        first_result_rx.recv().expect("first result after release").expect("first read");
        first_reader.join().expect("first reader join");
    }

    #[test]
    fn wal_attribution_reader_completion_pause_is_scoped_to_its_engine() {
        let first_dir = TempDir::new().expect("first temp dir");
        let second_dir = TempDir::new().expect("second temp dir");
        let first = Arc::new(
            Engine::open(first_dir.path().join("wal-attribution-first.sqlite"))
                .expect("first open"),
        );
        let second = Arc::new(
            Engine::open(second_dir.path().join("wal-attribution-second.sqlite"))
                .expect("second open"),
        );

        let (first_ready, first_release, reader_autocommit) =
            first.engine.arm_next_reader_completion_pause_for_test();
        let (second_result_tx, second_result_rx) = mpsc::sync_channel(1);
        let second_reader = {
            let engine = Arc::clone(&second);
            thread::spawn(move || {
                second_result_tx
                    .send(engine.engine.read_get("missing", &Default::default()))
                    .expect("second result receiver remains live");
            })
        };

        if second_result_rx.recv_timeout(Duration::from_secs(1)).is_err() {
            first_ready.wait();
            first_release.wait();
            second_reader.join().expect("second reader join");
            panic!("a first-engine completion pause blocked a second-engine reader");
        }
        second_reader.join().expect("second reader join");

        let (first_result_tx, first_result_rx) = mpsc::sync_channel(1);
        let first_reader = {
            let engine = Arc::clone(&first);
            thread::spawn(move || {
                first_result_tx
                    .send(engine.engine.read_get("missing", &Default::default()))
                    .expect("first result receiver remains live");
            })
        };
        first_ready.wait();
        assert!(matches!(first_result_rx.try_recv(), Err(mpsc::TryRecvError::Empty)));
        assert!(reader_autocommit.load(std::sync::atomic::Ordering::Acquire));
        first_release.wait();
        first_result_rx.recv().expect("first result after release").expect("first read");
        first_reader.join().expect("first reader join");
    }

    /// Slice 65 RED: a fully closed Engine is not itself a reader-holder. The
    /// raw checkpoint is deliberately independent so this distinguishes the
    /// close boundary from the incident-shaped recovery reads below.
    #[test]
    fn wal_attribution_close_boundary_raw_checkpoint_is_clean() {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("wal-attribution-close-boundary.sqlite");
        let old = seed_close_boundary_database(&path);
        old.engine.close().expect("old close");
        assert_closed_engine_is_idle(&old);

        let report = raw_close_boundary_checkpoint(&path, "direct_close", None);
        assert_eq!(report.busy, 0, "a closed Engine must not hold the raw checkpoint");
    }

    /// Slice 65 RED: a fresh open/close adds no holder beyond the close
    /// boundary control.
    #[test]
    fn wal_attribution_close_boundary_fresh_open_is_clean() {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("wal-attribution-close-fresh.sqlite");
        let old = seed_close_boundary_database(&path);
        old.engine.close().expect("old close");
        assert_closed_engine_is_idle(&old);

        let fresh = Engine::open(&path).expect("fresh open");
        assert!(fresh.engine.wal_attribution_snapshot().no_owned_snapshot);
        let report = raw_close_boundary_checkpoint(&path, "fresh_open", Some(&fresh));
        assert_eq!(report.busy, 0, "an idle fresh Engine must not hold the raw checkpoint");
        fresh.engine.close().expect("fresh close");
        assert_closed_engine_is_idle(&fresh);
    }

    /// Slice 65 RED: a completed read-get followed by explicit close leaves no
    /// checkpoint holder. This is intentionally separate from the incident
    /// recovery sequence.
    #[test]
    fn wal_attribution_close_boundary_read_get_is_clean() {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("wal-attribution-close-read-get.sqlite");
        let old = seed_close_boundary_database(&path);
        old.engine.close().expect("old close");
        assert_closed_engine_is_idle(&old);

        let fresh = Engine::open(&path).expect("fresh open");
        assert!(fresh
            .engine
            .read_get("slice65-close-root", &Default::default())
            .expect("read get")
            .is_some());
        assert!(fresh.engine.wal_attribution_snapshot().no_owned_snapshot);
        let report = raw_close_boundary_checkpoint(&path, "read_get", Some(&fresh));
        assert_eq!(report.busy, 0, "an idle read-get Engine must not hold the raw checkpoint");
        fresh.engine.close().expect("fresh close");
        assert_closed_engine_is_idle(&fresh);
    }

    /// Slice 65 RED: completed graph-neighbor traversal followed by explicit
    /// close leaves no checkpoint holder. This is the final staged sibling,
    /// not a replacement for the primary incident-shaped control.
    #[test]
    fn wal_attribution_close_boundary_neighbors_is_clean() {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("wal-attribution-close-neighbors.sqlite");
        let old = seed_close_boundary_database(&path);
        old.engine.close().expect("old close");
        assert_closed_engine_is_idle(&old);

        let fresh = Engine::open(&path).expect("fresh open");
        assert_eq!(
            fresh
                .engine
                .graph_neighbors(
                    "slice65-close-root",
                    1,
                    super::TraversalDirection::Outgoing,
                    &Default::default(),
                )
                .expect("neighbors")
                .len(),
            1
        );
        assert!(fresh.engine.wal_attribution_snapshot().no_owned_snapshot);
        let report = raw_close_boundary_checkpoint(&path, "graph_neighbors", Some(&fresh));
        assert_eq!(report.busy, 0, "an idle neighbors Engine must not hold the raw checkpoint");
        fresh.engine.close().expect("fresh close");
        assert_closed_engine_is_idle(&fresh);
    }

    fn seed_close_boundary_database(path: &Path) -> super::OpenedEngine {
        let old = Engine::open(path).expect("old open");
        old.engine
            .write(&[
                PreparedWrite::Node {
                    kind: "doc".to_string(),
                    body: "close boundary root".to_string(),
                    source_id: SourceId::new("slice65-close-root-source").expect("source"),
                    logical_id: Some("slice65-close-root".to_string()),
                    state: InitialState::Active,
                    reason: None,
                    valid_from: None,
                    valid_until: None,
                },
                PreparedWrite::Node {
                    kind: "doc".to_string(),
                    body: "close boundary nested".to_string(),
                    source_id: SourceId::new("slice65-close-nested-source").expect("source"),
                    logical_id: Some("slice65-close-nested".to_string()),
                    state: InitialState::Active,
                    reason: None,
                    valid_from: None,
                    valid_until: None,
                },
                PreparedWrite::Edge {
                    kind: "relates_to".to_string(),
                    from: "slice65-close-root".to_string(),
                    to: "slice65-close-nested".to_string(),
                    source_id: SourceId::new("slice65-close-nested-source").expect("source"),
                    logical_id: Some("slice65-close-edge".to_string()),
                    body: None,
                    t_valid: None,
                    t_invalid: None,
                    confidence: None,
                    extractor_model_id: None,
                    temporal_fallback: None,
                },
            ])
            .expect("old write");
        old
    }

    fn assert_closed_engine_is_idle(opened: &super::OpenedEngine) {
        assert!(opened.engine.closed.load(std::sync::atomic::Ordering::SeqCst));
        assert!(matches!(
            opened.engine.read_get("slice65-close-root", &Default::default()),
            Err(EngineError::Closing)
        ));
        assert!(opened.engine.wal_attribution_snapshot().no_owned_snapshot);
    }

    /// Issue one intentionally independent SQLite checkpoint. The optional
    /// fresh Engine stays live only for the staged siblings, where this helper
    /// proves its managed writer connection exists while its collector is idle.
    /// The emitted fields are limited to fixed state labels and SQLite's
    /// busy/frame counters; it deliberately emits no database or request data.
    fn raw_close_boundary_checkpoint(
        path: &Path,
        case: &str,
        fresh: Option<&super::OpenedEngine>,
    ) -> super::TruncateWalReport {
        let (fresh_engine_open, fresh_writer_connection_open) = if let Some(opened) = fresh {
            assert!(
                !opened.engine.closed.load(std::sync::atomic::Ordering::SeqCst),
                "fresh Engine must remain open during the raw checkpoint"
            );
            assert!(
                opened.engine.wal_attribution_snapshot().no_owned_snapshot,
                "fresh Engine must be collector-idle during the raw checkpoint"
            );
            let writer_open =
                opened.engine.connection.lock().expect("fresh writer connection mutex").is_some();
            assert!(writer_open, "fresh Engine writer connection must remain open");
            (true, true)
        } else {
            (false, false)
        };
        let connection = Connection::open(path).expect("independent raw sqlite open");
        connection.busy_timeout(Duration::ZERO).expect("raw checkpoint no wait");
        let (busy, log_frames, checkpointed_frames): (i64, i64, i64) = connection
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .expect("independent raw checkpoint");
        connection.close().expect("independent raw sqlite close");

        let report = super::TruncateWalReport {
            status: if busy == 0 {
                super::TruncateWalStatus::Done
            } else {
                super::TruncateWalStatus::Busy
            },
            busy: busy.max(0) as u32,
            log_frames: log_frames.max(0) as u32,
            checkpointed_frames: checkpointed_frames.max(0) as u32,
            discarded_corrupt_wal: false,
        };
        eprintln!(
            "slice65_wal close_boundary case={case} old_engine_closed=1 fresh_engine_open={fresh_engine_open} fresh_writer_connection_open={fresh_writer_connection_open} raw_busy={} raw_log_frames={} raw_checkpointed_frames={}",
            report.busy, report.log_frames, report.checkpointed_frames,
        );
        report
    }

    /// Slice 65 follow-on: retain the direct-Rust incident as one typed
    /// fail-closed observation. Ordered native raw samples distinguish the old
    /// close, completed fresh recovery reads, and pre-erasure probe lifecycle;
    /// none retries or changes the original nested-source erase.
    #[test]
    fn wal_attribution_incident_checkpoint_ladder_retains_typed_erase_observation() {
        const CHILD_PATH: &str = "FATHOMDB_SLICE65_INCIDENT_LADDER_CHILD_PATH";

        if let Some(path) = std::env::var_os(CHILD_PATH) {
            let registry = Arc::new(ManagedConnectionRegistry::default());
            let report = incident_ladder_raw_checkpoint(
                Path::new(&path),
                "fresh_child",
                &registry,
                true,
                false,
            );
            eprintln!(
                "slice65_wal incident_ladder child_raw stage=fresh_child raw_busy={} raw_log_frames={} raw_checkpointed_frames={}",
                report.busy, report.log_frames, report.checkpointed_frames,
            );
            return;
        }

        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("wal-attribution-incident-ladder.sqlite");
        let old = seed_close_boundary_database(&path);
        old.engine.close().expect("old close");
        assert_closed_engine_is_idle(&old);
        let old_report = incident_ladder_raw_checkpoint(
            &path,
            "old_close",
            &old.engine.managed_connections,
            true,
            false,
        );

        let fresh = Engine::open(&path).expect("fresh open");
        assert!(fresh
            .engine
            .read_get("slice65-close-root", &Default::default())
            .expect("fresh recovery read")
            .is_some());
        assert_eq!(
            fresh
                .engine
                .graph_neighbors(
                    "slice65-close-root",
                    1,
                    super::TraversalDirection::Outgoing,
                    &Default::default(),
                )
                .expect("fresh recovery neighbors")
                .len(),
            1
        );
        let inventory =
            incident_ladder_long_lived_inventory(&fresh).expect("complete fresh inventory");
        eprintln!(
            "slice65_wal incident_ladder stage=after_fresh_reads direct_inventory={inventory} collector_roles=idle"
        );
        let fresh_reads_report = incident_ladder_raw_checkpoint(
            &path,
            "after_fresh_reads",
            &fresh.engine.managed_connections,
            false,
            true,
        );
        incident_ladder_runtime_probe_preflight(&path, &fresh.engine.managed_connections);
        let preflight_report = incident_ladder_raw_checkpoint(
            &path,
            "after_erasure_preflight",
            &fresh.engine.managed_connections,
            false,
            true,
        );

        let observed = fresh.engine.erase_source("slice65-close-nested-source");
        match observed {
            Err(EngineError::ErasureIncomplete { stage, .. }) if stage == "wal_checkpoint" => {
                eprintln!("slice65_wal incident_ladder typed_erase_observation=wal_checkpoint");
                eprintln!("slice65_wal incident_ladder erase_observation=typed_erasure_incomplete");
            }
            Ok(report) => {
                assert_eq!(
                    report.nodes_excised, 1,
                    "clean incident observation must excise nested node"
                );
                eprintln!("slice65_wal incident_ladder erase_observation=clean_completion");
            }
            other => panic!("incident erase produced an unrecognized outcome: {other:?}"),
        }

        let any_busy = [old_report, fresh_reads_report, preflight_report]
            .into_iter()
            .any(|report| report.busy != 0);
        if any_busy {
            fresh.engine.close().expect("close and join fresh Engine before follow-up samples");
            assert_closed_engine_is_idle(&fresh);
            let same_process = incident_ladder_raw_checkpoint(
                &path,
                "after_fresh_close",
                &fresh.engine.managed_connections,
                true,
                false,
            );
            eprintln!(
                "slice65_wal incident_ladder same_process_raw stage=after_fresh_close raw_busy={} raw_log_frames={} raw_checkpointed_frames={}",
                same_process.busy, same_process.log_frames, same_process.checkpointed_frames,
            );
            let output = Command::new(std::env::current_exe().expect("current test executable"))
                .arg("--exact")
                .arg("tests::wal_attribution_incident_checkpoint_ladder_retains_typed_erase_observation")
                .arg("--nocapture")
                .env(CHILD_PATH, &path)
                .output()
                .expect("run native fresh-child incident probe");
            assert!(
                output.status.success(),
                "native fresh-child incident probe failed with status {:?}",
                output.status
            );
            eprint!("{}", String::from_utf8_lossy(&output.stderr));
            eprintln!("slice65_wal incident_ladder child_raw stage=fresh_child outcome=recorded");
        } else {
            eprintln!(
                "slice65_wal incident_ladder child_raw stage=fresh_child outcome=not_required"
            );
        }
        eprintln!("slice65_wal incident_ladder=recorded");
    }

    /// Slice 65 follow-on: observe the real checkpoint calls in the exact
    /// close/reopen/recovery-read incident path. No raw checkpoint or runtime
    /// probe is permitted between the recovery reads and the one original
    /// erasure.
    #[test]
    fn wal_attribution_actual_checkpoint_observation_retains_real_attempt_facts() {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("wal-attribution-actual-checkpoint.sqlite");
        let old = seed_close_boundary_database(&path);
        old.engine.close().expect("old close");
        assert_closed_engine_is_idle(&old);

        let fresh = Engine::open(&path).expect("fresh open");
        assert!(fresh
            .engine
            .read_get("slice65-close-root", &Default::default())
            .expect("fresh recovery read")
            .is_some());
        assert_eq!(
            fresh
                .engine
                .graph_neighbors(
                    "slice65-close-root",
                    1,
                    super::TraversalDirection::Outgoing,
                    &Default::default(),
                )
                .expect("fresh recovery neighbors")
                .len(),
            1
        );

        fresh.engine.arm_actual_checkpoint_observation_for_test("direct_rust");
        let observed = fresh.engine.erase_source("slice65-close-nested-source");
        let records = fresh.engine.take_actual_checkpoint_observations_for_test();
        assert!(!records.is_empty(), "real erase must retain checkpoint observations");
        assert_eq!(records.len() % 2, 0, "every real attempt has before/after facts");
        for pair in records.chunks_exact(2) {
            assert!(pair[0].contains("control=direct_rust phase=before"));
            assert!(pair[1].contains("control=direct_rust phase=after"));
            assert!(pair[0].contains("writer_autocommit=1"));
            assert!(pair[1].contains("writer_autocommit=1"));
            assert!(
                pair[0].contains("direct_inventory=roles=writer:0,readers:0-7,dispatcher:0,workers:0-1;writer=autocommit;readers=autocommit;dispatcher=autocommit;workers=2-autocommit;registry=complete;creation=writer:1,readers:8,dispatcher:1,workers:2,probes:0;complete=1"),
                "unexpected direct inventory: {}",
                pair[0]
            );
            assert!(pair[1].contains("collector_roles=idle"));
            assert!(pair[1].contains("elapsed_ms="));
            assert!(pair[1].contains("busy="));
            eprintln!("slice65_wal actual_checkpoint {}", pair[0]);
            eprintln!("slice65_wal actual_checkpoint {}", pair[1]);
        }
        match observed {
            Err(EngineError::ErasureIncomplete { stage, .. }) if stage == "wal_checkpoint" => {
                eprintln!("slice65_wal actual_checkpoint control=direct_rust erase_observation=typed_erasure_incomplete");
            }
            Ok(report) => {
                assert_eq!(report.nodes_excised, 1, "clean observation must excise nested node");
                eprintln!("slice65_wal actual_checkpoint control=direct_rust erase_observation=clean_completion");
            }
            other => panic!("actual checkpoint erase produced an unrecognized outcome: {other:?}"),
        }
        eprintln!("slice65_wal actual_checkpoint control=direct_rust recorded");
    }

    fn incident_ladder_long_lived_inventory(
        opened: &super::OpenedEngine,
    ) -> Result<String, &'static str> {
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut snapshot = opened.engine.wal_attribution_snapshot();
        while snapshot.roles.len() < 1 + READER_POOL_SIZE + 1 + PROJECTION_WORKERS
            && Instant::now() < deadline
        {
            thread::sleep(Duration::from_millis(5));
            snapshot = opened.engine.wal_attribution_snapshot();
        }
        if snapshot.roles.len() != 1 + READER_POOL_SIZE + 1 + PROJECTION_WORKERS
            || !opened.engine.managed_connections.exact_live(PROJECTION_WORKERS)
        {
            return Err("registry_mismatch");
        }
        if !snapshot.no_owned_snapshot
            || snapshot.roles.iter().any(|role| role.active || role.phase != "idle")
        {
            return Err("collector_not_idle");
        }
        let writer_autocommit = opened
            .engine
            .connection
            .lock()
            .map_err(|_| "writer_lock")?
            .as_ref()
            .is_some_and(Connection::is_autocommit);
        if !writer_autocommit {
            return Err("writer_not_autocommit");
        }
        let readers = opened.engine.reader_pool.wal_connection_inventory_for_test();
        if readers.len() != READER_POOL_SIZE || readers.into_iter().any(|autocommit| !autocommit) {
            return Err("reader_not_autocommit");
        }
        let runtime =
            opened.engine.projection_runtime.report_runtime_connection_inventory_for_test()?;
        let expected = BTreeSet::from([
            (WalAttributionRole::ProjectionDispatcher, 0),
            (WalAttributionRole::ProjectionWorker, 0),
            (WalAttributionRole::ProjectionWorker, 1),
        ]);
        let actual =
            runtime.iter().map(|(role, index, _)| (*role, *index)).collect::<BTreeSet<_>>();
        if actual != expected || runtime.iter().any(|(_, _, autocommit)| !autocommit) {
            return Err("runtime_not_autocommit");
        }
        let creation =
            opened.engine.managed_connections.creation_counts().ok_or("creation_audit_lock")?;
        Ok(format!(
            "writer:autocommit;readers:8-autocommit;dispatcher:autocommit;workers:2-autocommit;creation=writer:{},readers:{},dispatcher:{},workers:{},probes:{}",
            creation.0, creation.1, creation.2, creation.3, creation.4,
        ))
    }

    fn incident_ladder_runtime_probe_preflight(
        path: &Path,
        registry: &Arc<ManagedConnectionRegistry>,
    ) {
        let probe =
            RuntimeProbeConnection::open(path, registry).expect("open runtime probe preflight");
        let _: i64 = probe
            .connection()
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .expect("query runtime probe preflight");
        let lifecycle = probe.close_and_acknowledge().expect("close runtime probe preflight");
        assert_eq!(lifecycle.live, 0, "runtime probe must not remain live at preflight");
        assert_eq!(
            lifecycle.incomplete_drops, 0,
            "runtime probe preflight must retain an actual close acknowledgement"
        );
        eprintln!(
            "slice65_wal incident_ladder runtime_probe_drop_ack stage=erasure_preflight live={} actual_drops={} incomplete_drops={}",
            lifecycle.live, lifecycle.actual_drops, lifecycle.incomplete_drops,
        );
    }

    fn incident_ladder_raw_checkpoint(
        path: &Path,
        stage: &str,
        registry: &Arc<ManagedConnectionRegistry>,
        old_engine_closed: bool,
        fresh_engine_open: bool,
    ) -> super::TruncateWalReport {
        let started = Instant::now();
        let probe =
            RuntimeProbeConnection::open(path, registry).expect("open native runtime probe");
        probe.connection().busy_timeout(Duration::ZERO).expect("raw probe no wait");
        let (busy, log_frames, checkpointed_frames): (i64, i64, i64) = probe
            .connection()
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .expect("native runtime probe checkpoint");
        let lifecycle = probe.close_and_acknowledge().expect("close native runtime probe");
        assert_eq!(lifecycle.live, 0, "runtime probe must be dropped before recording {stage}");
        assert_eq!(
            lifecycle.incomplete_drops, 0,
            "runtime probe lifecycle is incomplete at {stage}"
        );
        let report = super::TruncateWalReport {
            status: if busy == 0 {
                super::TruncateWalStatus::Done
            } else {
                super::TruncateWalStatus::Busy
            },
            busy: busy.max(0) as u32,
            log_frames: log_frames.max(0) as u32,
            checkpointed_frames: checkpointed_frames.max(0) as u32,
            discarded_corrupt_wal: false,
        };
        eprintln!(
            "slice65_wal incident_ladder stage={stage} old_engine_closed={} fresh_engine_open={} elapsed_ms={} raw_busy={} raw_log_frames={} raw_checkpointed_frames={} runtime_probe_live={} runtime_probe_actual_drop={} runtime_probe_incomplete_drops={}",
            u8::from(old_engine_closed),
            u8::from(fresh_engine_open),
            started.elapsed().as_millis(),
            report.busy,
            report.log_frames,
            report.checkpointed_frames,
            lifecycle.live,
            lifecycle.actual_drops,
            lifecycle.incomplete_drops,
        );
        report
    }

    fn projection_worker_attribution_matches(
        active_roles: &[(WalAttributionRole, usize)],
        paused_worker: (WalAttributionRole, usize),
    ) -> bool {
        active_roles.iter().filter(|role| **role == paused_worker).count() == 1
            && active_roles
                .iter()
                .filter(|(role, index)| {
                    *role == WalAttributionRole::ProjectionDispatcher && *index == 0
                })
                .count()
                <= 1
            && active_roles.iter().all(|role| {
                *role == paused_worker || *role == (WalAttributionRole::ProjectionDispatcher, 0)
            })
    }

    #[test]
    fn projection_worker_attribution_accepts_concurrent_dispatcher_snapshot() {
        let paused_worker = (WalAttributionRole::ProjectionWorker, 1);
        assert!(projection_worker_attribution_matches(&[paused_worker], paused_worker));
        assert!(projection_worker_attribution_matches(
            &[(WalAttributionRole::ProjectionDispatcher, 0), paused_worker],
            paused_worker,
        ));
        assert!(!projection_worker_attribution_matches(
            &[(WalAttributionRole::ProjectionWorker, 0), paused_worker],
            paused_worker,
        ));
        assert!(!projection_worker_attribution_matches(
            &[(WalAttributionRole::Writer, 0), paused_worker],
            paused_worker,
        ));
    }

    /// Slice 65 projection-worker witness: preserve the original typed
    /// refusal, then observe post-finish connection state without retrying
    /// that erasure.
    // Exercises `pause_projection_worker_after_wal_transaction_for_test`, a
    // debug-build-only hook; gated so the release-profile lib-test build
    // (cfg(test) true, debug_assertions false) does not need it widened.
    #[cfg(debug_assertions)]
    #[test]
    fn wal_attribution_projection_worker_typed_refusal_then_post_release_sampler_is_recorded() {
        let dir = TempDir::new().expect("temp dir");
        let opened = Engine::open_with_embedder_for_test(
            dir.path().join("wal-attribution-projection.sqlite"),
            Arc::new(Slice65ProjectionEmbedder),
        )
        .expect("open");
        opened.engine.configure_vector_kind_for_test("doc").expect("vector kind");
        let mut transaction_pause =
            opened.engine.pause_projection_worker_after_wal_transaction_for_test();
        opened
            .engine
            .write(&[PreparedWrite::Node {
                kind: "doc".to_string(),
                body: "projection transaction".to_string(),
                source_id: SourceId::new("slice65-projection-source").expect("source"),
                logical_id: Some("slice65-projection".to_string()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            }])
            .expect("write");
        transaction_pause
            .wait_ready(Duration::from_secs(30))
            .expect("projection worker reaches its WAL transaction under loaded CI");
        let active = opened.engine.wal_attribution_snapshot();
        assert_eq!(
            opened.engine.wal_attribution.classification(&active, false),
            "owned_runtime_transaction"
        );
        let paused_workers = active
            .active_roles
            .iter()
            .copied()
            .filter(|(role, _)| *role == WalAttributionRole::ProjectionWorker)
            .collect::<Vec<_>>();
        assert_eq!(paused_workers.len(), 1);
        let paused_worker = paused_workers[0];
        assert!(projection_worker_attribution_matches(&active.active_roles, paused_worker));
        eprintln!("slice65_wal projection_worker_transaction_ready");
        let blocked = opened.engine.complete_erasure_at_rest("slice65-projection-checkpoint");
        let busy = opened.engine.wal_attribution_checkpoints_for_test();
        assert!(matches!(blocked, Err(EngineError::ErasureIncomplete { .. })));
        assert_eq!(busy.len(), ERASURE_WAL_TRUNCATE_ATTEMPTS as usize);
        assert!(busy.iter().all(|record| {
            record.busy
                && record.classification == "owned_runtime_transaction"
                && projection_worker_attribution_matches(&record.active_roles, paused_worker)
        }));
        eprintln!(
            "slice65_wal projection_worker_original_erase=typed_erasure_incomplete owned_busy_attempts={}",
            busy.len()
        );

        transaction_pause.release();
        opened.engine.drain(5_000).expect("projection settles");
        let runtime = opened
            .engine
            .projection_runtime
            .report_runtime_connection_inventory_for_test()
            .expect("post-finish runtime inventory");
        assert_eq!(
            runtime
                .iter()
                .find(|(role, index, _)| {
                    *role == WalAttributionRole::ProjectionWorker && *index == 0
                })
                .map(|(_, _, autocommit)| *autocommit),
            Some(true),
            "projection worker owner thread must be autocommit after finishing"
        );
        let completion = opened.engine.wal_attribution_snapshot();
        assert!(completion.no_owned_snapshot, "collector must be idle after projection finish");
        eprintln!(
            "slice65_wal projection_worker_completion_ack worker_autocommit=1 collector_roles=idle"
        );

        let inventory = opened.engine.native_state_inventory_for_test();
        let inventory_text = super::native_state_inventory_text(&inventory);
        eprintln!("slice65_wal projection_worker_native_state_inventory={inventory_text}");
        if !inventory.complete {
            eprintln!(
                "slice65_wal projection_worker_native_state_inventory=state_inventory=incomplete"
            );
        }
        assert!(inventory.complete, "post-finish native inventory: {inventory:?}");

        opened.engine.arm_binding_native_state_observation_for_test();
        let samples = opened
            .engine
            .checkpoint_at_rest_for_test()
            .expect("run bounded post-finish checkpoint sampler");
        assert!(!samples.is_empty() && samples.len() <= ERASURE_WAL_TRUNCATE_ATTEMPTS as usize);
        let state_records = opened.engine.drain_binding_native_state_observations_for_test();
        assert_eq!(state_records.len(), samples.len() * 2);
        for state in &state_records {
            eprintln!("slice65_wal projection_worker_sampler_native_state {state}");
            if !state.contains("state_inventory=complete reason=complete") {
                eprintln!(
                    "slice65_wal projection_worker_sampler_native_state state_inventory=incomplete"
                );
            }
            assert!(state.contains("state_inventory=complete reason=complete"));
        }
        let (busy, log_frames, checkpointed_frames) =
            samples.last().copied().expect("sampler result");
        eprintln!(
            "slice65_wal projection_worker_sampler_terminal outcome={} attempts={} busy={} log_frames={} checkpointed_frames={}",
            if busy { "busy" } else { "clean" },
            samples.len(),
            u8::from(busy),
            log_frames,
            checkpointed_frames,
        );
    }

    // Exercises `pause_projection_worker_after_wal_transaction_for_test`, a
    // debug-build-only hook; gated so the release-profile lib-test build
    // (cfg(test) true, debug_assertions false) does not need it widened.
    #[cfg(debug_assertions)]
    #[test]
    fn projection_transaction_pause_ready_timeout_cancels_before_worker_arrival() {
        let dir = TempDir::new().expect("temp dir");
        let opened = Engine::open_with_embedder_for_test(
            dir.path().join("projection-pause-ready-timeout.sqlite"),
            Arc::new(Slice65ProjectionEmbedder),
        )
        .expect("open");
        opened.engine.configure_vector_kind_for_test("doc").expect("vector kind");
        opened.engine.set_projection_scheduler_frozen_for_test(true);
        let mut transaction_pause =
            opened.engine.pause_projection_worker_after_wal_transaction_for_test();
        opened
            .engine
            .write(&[PreparedWrite::Node {
                kind: "doc".to_string(),
                body: "projection pause cancelled before arrival".to_string(),
                source_id: SourceId::new("projection-pause-timeout-source").expect("source"),
                logical_id: Some("projection-pause-timeout".to_string()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            }])
            .expect("write");

        let timeout = Duration::from_millis(25);
        let error =
            transaction_pause.wait_ready(timeout).expect_err("frozen scheduler cannot arrive");
        assert_eq!(
            error.to_string(),
            "projection worker did not reach transaction pause within 25ms"
        );
        drop(transaction_pause);

        opened.engine.set_projection_scheduler_frozen_for_test(false);
        opened.engine.drain(5_000).expect("cancelled pause cannot strand projection drain");
        opened.engine.close().expect("cancelled pause cannot strand Engine close");
    }

    // Exercises `pause_projection_worker_after_wal_transaction_for_test`, a
    // debug-build-only hook; gated so the release-profile lib-test build
    // (cfg(test) true, debug_assertions false) does not need it widened.
    #[cfg(debug_assertions)]
    #[test]
    fn projection_transaction_pause_drop_releases_worker_after_ready() {
        let dir = TempDir::new().expect("temp dir");
        let opened = Engine::open_with_embedder_for_test(
            dir.path().join("projection-pause-drop-release.sqlite"),
            Arc::new(Slice65ProjectionEmbedder),
        )
        .expect("open");
        opened.engine.configure_vector_kind_for_test("doc").expect("vector kind");
        let mut transaction_pause =
            opened.engine.pause_projection_worker_after_wal_transaction_for_test();
        opened
            .engine
            .write(&[PreparedWrite::Node {
                kind: "doc".to_string(),
                body: "projection pause released by drop".to_string(),
                source_id: SourceId::new("projection-pause-drop-source").expect("source"),
                logical_id: Some("projection-pause-drop".to_string()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            }])
            .expect("write");
        transaction_pause
            .wait_ready(Duration::from_secs(30))
            .expect("projection worker reaches its WAL transaction under loaded CI");
        drop(transaction_pause);

        opened.engine.drain(5_000).expect("Drop release cannot strand projection drain");
        opened.engine.close().expect("Drop release cannot strand Engine close");
    }

    /// 0.8.20 Slice 5a (R-20-E1, work item 2) — the registry GUARD.
    ///
    /// Introspects `sqlite_master` on a freshly migrated database and asserts
    /// that EVERY `write_cursor`-keyed table is accounted for: either it is a
    /// registered row-owned projection, or it is one of the explicitly named
    /// canonical / operational tables that are sources of truth, not shadows.
    /// A future projection table therefore cannot be added without either
    /// registering it in [`ROW_OWNED_PROJECTIONS`] (making it erasable at every
    /// maintenance site at once) or consciously failing this test.
    ///
    /// **`_fathomdb_projection_state` is allowlisted as KIND-owned** (design v5
    /// §1.1): it is keyed by `kind`, not by `write_cursor`, and holds a per-kind
    /// enqueue watermark. Erasing one row must not rewind a whole kind's
    /// watermark, so it must NEVER be deleted per-cursor. The test asserts both
    /// halves of that claim — that it carries no `write_cursor` column, and that
    /// it is absent from the row-owned registry.
    #[test]
    fn guard_row_owned_registry() {
        /// Canonical + operational tables: `write_cursor`-carrying SOURCES OF
        /// TRUTH, never row-owned projections of another row.
        const NON_PROJECTION_CURSOR_TABLES: &[&str] = &[
            "canonical_nodes",
            "canonical_edges",
            "operational_mutations",
            "operational_state",
            "_fathomdb_artifact_revisions",
        ];

        let dir = TempDir::new().unwrap();
        let path = dir.path().join("registry_guard.fathomdb");
        Engine::open(&path).expect("open").engine.close().expect("close");
        let conn = Connection::open(&path).expect("open sqlite");

        let table_names: Vec<String> = conn
            .prepare(
                "SELECT name FROM sqlite_master
                 WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
            )
            .expect("prepare")
            .query_map([], |row| row.get::<_, String>(0))
            .expect("query")
            .collect::<rusqlite::Result<Vec<_>>>()
            .expect("collect");
        assert!(table_names.len() > 5, "sqlite_master introspection returned nothing useful");

        let has_write_cursor = |table: &str| -> bool {
            conn.prepare(&format!("PRAGMA table_info({table})"))
                .and_then(|mut stmt| {
                    let names = stmt
                        .query_map([], |row| row.get::<_, String>(1))?
                        .collect::<rusqlite::Result<Vec<_>>>()?;
                    Ok(names.iter().any(|n| n == "write_cursor"))
                })
                .unwrap_or(false)
        };

        let registered: Vec<&str> = ROW_OWNED_PROJECTIONS.iter().map(|p| p.table).collect();

        // (1) Every write_cursor-keyed table is registered or explicitly excused.
        for table in &table_names {
            if !has_write_cursor(table) {
                continue;
            }
            assert!(
                registered.contains(&table.as_str())
                    || NON_PROJECTION_CURSOR_TABLES.contains(&table.as_str()),
                "table `{table}` is keyed by write_cursor but is neither registered in \
                 ROW_OWNED_PROJECTIONS nor listed as a non-projection source of truth. \
                 If it is a projection, register it — otherwise erasure will leave its \
                 rows on disk (the `search_index_v2` defect)."
            );
        }

        // (2) Every registered projection actually exists and is erasable by its
        //     declared cursor column (vec0's `rowid` included).
        for projection in ROW_OWNED_PROJECTIONS {
            assert!(
                table_names.iter().any(|t| t == projection.table),
                "registered projection `{}` does not exist in the schema",
                projection.table
            );
            conn.query_row(
                &format!(
                    "SELECT COUNT(*) FROM {} WHERE {} = 0",
                    projection.table, projection.cursor_column
                ),
                [],
                |row| row.get::<_, u64>(0),
            )
            .unwrap_or_else(|err| {
                panic!(
                    "registered projection `{}` is not erasable by `{}`: {err}",
                    projection.table, projection.cursor_column
                )
            });
        }

        // (3) `_fathomdb_projection_state` is KIND-owned, not row-owned.
        assert!(
            !has_write_cursor("_fathomdb_projection_state"),
            "_fathomdb_projection_state gained a write_cursor column — re-decide its ownership \
             class before treating it as kind-owned"
        );
        assert!(
            !registered.contains(&"_fathomdb_projection_state"),
            "_fathomdb_projection_state is KIND-owned (per-kind enqueue watermark) and must \
             never be deleted per-cursor: erasing one row would rewind a whole kind's watermark"
        );
    }

    /// 0.8.20 Slice 15d (R-20-EAV) — PROVE THE GUARD BITES. The two net-new
    /// content-storing projection tables (`canonical_attributes`,
    /// `property_search_index`) are `write_cursor`-keyed and hold attribute
    /// values at rest. This test asserts (1) they ARE registered in
    /// `ROW_OWNED_PROJECTIONS` (so `erase_row_projections` reaches them), and (2)
    /// the guard's core predicate — "registered OR a named source of truth" —
    /// FAILS for either table if it is (hypothetically) removed from the
    /// registry. This is what makes forgetting to register a future
    /// content-storing projection a red test, not a silent erasure leak.
    #[test]
    fn slice15d_attribute_projections_registered_and_guard_bites() {
        const NON_PROJECTION_CURSOR_TABLES: &[&str] =
            &["canonical_nodes", "canonical_edges", "operational_mutations", "operational_state"];

        let registered: Vec<&str> = ROW_OWNED_PROJECTIONS.iter().map(|p| p.table).collect();

        // (1) Both new content-storing projections are registered as row-owned.
        for table in ["canonical_attributes", "property_search_index"] {
            assert!(
                registered.contains(&table),
                "{table} holds attribute values at rest and MUST be in ROW_OWNED_PROJECTIONS \
                 so purge/excise_source reach it"
            );
        }

        // (2) The guard predicate BITES: pretend one of them was never
        //     registered — the guard's "registered OR source-of-truth" check must
        //     reject it (the exact assertion `guard_row_owned_registry` runs).
        for hidden in ["canonical_attributes", "property_search_index"] {
            let as_if_unregistered: Vec<&str> =
                registered.iter().copied().filter(|t| *t != hidden).collect();
            let accepted = as_if_unregistered.contains(&hidden)
                || NON_PROJECTION_CURSOR_TABLES.contains(&hidden);
            assert!(
                !accepted,
                "if {hidden} were unregistered the guard would still (incorrectly) accept it — \
                 the guard does not actually bite"
            );
        }
    }

    /// Cause-A (0.8.11.2) / C-2 (0.8.19) — `derive_stable_id` id-space contract:
    /// a present `logical_id` yields a `Logical` (`"l:"`) [`IdSpace`]; a NULL or
    /// empty `logical_id` falls back to a deterministic `Content` (`"h:"`) sha256
    /// content-hash of the body. The typed spaces are prefix-distinguishable and
    /// the value is behaviour-neutral (never used in ranking). Post-C-2 the helper
    /// returns a typed [`IdSpace`] whose `to_prefixed()` reproduces the pre-swap
    /// string byte-for-byte (eu7 no-op basis).
    #[test]
    fn derive_stable_id_id_space_contract() {
        // logical_id present → Logical space, body-independent.
        assert_eq!(derive_stable_id(Some("alice-1"), "any body"), IdSpace::logical("alice-1"));
        assert_eq!(
            derive_stable_id(Some("alice-1"), "a different body"),
            IdSpace::logical("alice-1")
        );
        // Byte-identical prefixed form to the pre-C-2 `stable_id` string.
        assert_eq!(derive_stable_id(Some("alice-1"), "any body").to_prefixed(), "l:alice-1");

        // NULL logical_id → Content space, deterministic on body.
        let h1 = derive_stable_id(None, "stable body text");
        let h2 = derive_stable_id(None, "stable body text");
        assert_eq!(h1, h2, "content-hash is deterministic");
        assert_eq!(h1.space, IdSpaceKind::Content);
        let h1s = h1.to_prefixed();
        assert!(h1s.starts_with("h:"));
        assert_eq!(h1s.len(), 2 + 64, "h: + sha256 hex");
        assert!(h1s["h:".len()..].chars().all(|c| c.is_ascii_hexdigit()));

        // Empty logical_id is treated as absent (falls back to content-hash).
        assert_eq!(derive_stable_id(Some(""), "stable body text"), h1);

        // Distinct bodies → distinct content-hashes (no collision).
        assert_ne!(derive_stable_id(None, "body A"), derive_stable_id(None, "body B"));
    }

    /// C-2 (0.8.19 / TC-8) — [`IdSpace`] parse/format round-trip is stable across
    /// all three spaces, including a value that itself contains `":"`.
    #[test]
    fn id_space_parse_format_round_trip() {
        let cases = [
            IdSpace::logical("alice-1"),
            IdSpace::content("a".repeat(64)),
            IdSpace::passage("7"),
            IdSpace::logical("l:weird:value"), // value contains the delimiter
        ];
        for id in cases {
            assert_eq!(IdSpace::parse(&id.to_prefixed()), Some(id.clone()), "round-trip {id:?}");
        }
        assert_eq!(IdSpace::logical("x").to_prefixed(), "l:x");
        assert_eq!(IdSpace::content("y").to_prefixed(), "h:y");
        assert_eq!(IdSpace::passage("3").to_prefixed(), "p:3");
        assert_eq!(IdSpace::parse("untagged"), None);
    }

    // Pack 1 drift-detection: the Rust helper used by the two writer
    // sites must agree with the CASE WHEN used by the Pack 1 reshape
    // migration in `migrate_vector_partition_to_pack1`. The CASE SQL
    // is exported as `KIND_TO_SOURCE_TYPE_CASE_SQL`; this test
    // exercises it against an in-memory SQLite (no sqlite-vec extension
    // required — only the CASE) and asserts byte-equal output with the
    // Rust helper for every kind in the locked Pack 1 vocabulary
    // (incl. the synthetic `doc` -> `article` coercion). See
    // `dev/design/0.7.0-vector-quant-pack1.md` D3 / D4.
    #[test]
    fn resolve_source_type_drift_check() {
        let kinds = ["email", "article", "paper", "meeting", "note", "todo", "doc"];

        // 1. Rust helper return values (table is the contract: changes
        //    here must be reflected in the SQL CASE or this test fails).
        let want: &[(&str, &str)] = &[
            ("email", "email"),
            ("article", "article"),
            ("paper", "paper"),
            ("meeting", "meeting"),
            ("note", "note"),
            ("todo", "todo"),
            ("doc", "article"),
        ];
        for (kind, expected) in want {
            let got = resolve_source_type(kind).unwrap_or_else(|_| {
                panic!("resolve_source_type({kind}) returned Err; want Ok({expected})")
            });
            assert_eq!(got, *expected, "Rust helper drift for kind={kind}");
        }
        assert!(
            resolve_source_type("banana").is_err(),
            "unknown kind must surface as writer error"
        );

        // 2. SQL CASE evaluated against the same kinds. Build a
        //    one-row staging row per kind and SELECT through
        //    KIND_TO_SOURCE_TYPE_CASE_SQL; assert each row equals the
        //    Rust helper's output. Drift in either direction fails.
        let conn = Connection::open_in_memory().expect("in-memory sqlite");
        conn.execute_batch("CREATE TABLE s(kind TEXT NOT NULL)").expect("create s");
        for kind in &kinds {
            conn.execute("INSERT INTO s(kind) VALUES (?1)", [kind]).expect("insert kind");
        }
        let sql = format!("SELECT s.kind, {KIND_TO_SOURCE_TYPE_CASE_SQL} FROM s");
        let mut stmt = conn.prepare(&sql).expect("prepare CASE");
        let rows: Vec<(String, String)> = stmt
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
            .expect("query")
            .map(|r| r.expect("row"))
            .collect();
        assert_eq!(rows.len(), kinds.len(), "row count drift");
        for (kind, sql_result) in &rows {
            let rust_result = resolve_source_type(kind).expect("known kind");
            assert_eq!(
                sql_result, rust_result,
                "SQL CASE vs Rust helper drift for kind={kind}: SQL={sql_result}, Rust={rust_result}"
            );
        }
    }

    #[test]
    fn write_advances_cursor() {
        let dir = TempDir::new().unwrap();
        let opened = Engine::open(dir.path().join("rewrite.sqlite")).expect("engine should open");
        let receipt = opened
            .engine
            .write(&[PreparedWrite::Node {
                kind: "doc".to_string(),
                body: "hello".to_string(),
                source_id: crate::SourceId::new("test:fixture").expect("test source id"),
                logical_id: None,
                state: crate::InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            }])
            .expect("write should succeed");

        assert_eq!(receipt.cursor, 1);
    }

    #[test]
    fn write_batch_prepares_visibility_triggered_statements_once() {
        let dir = TempDir::new().unwrap();
        let opened =
            Engine::open(dir.path().join("statement-cache.sqlite")).expect("engine should open");
        let canonical_prepares = Arc::new(AtomicUsize::new(0));
        let supersession_prepares = Arc::new(AtomicUsize::new(0));
        let search_index_prepares = Arc::new(AtomicUsize::new(0));
        let fielded_index_prepares = Arc::new(AtomicUsize::new(0));
        let revision_probe_prepares = Arc::new(AtomicUsize::new(0));
        let artifact_prepares = Arc::new(AtomicUsize::new(0));
        let terminal_prepares = Arc::new(AtomicUsize::new(0));
        {
            let mut guard = opened.engine.connection.lock().expect("writer lock");
            let connection = guard.as_mut().expect("open writer");
            connection.flush_prepared_statement_cache();
            let canonical_prepares = Arc::clone(&canonical_prepares);
            let supersession_prepares = Arc::clone(&supersession_prepares);
            let search_index_prepares = Arc::clone(&search_index_prepares);
            let fielded_index_prepares = Arc::clone(&fielded_index_prepares);
            let revision_probe_prepares = Arc::clone(&revision_probe_prepares);
            let artifact_prepares = Arc::clone(&artifact_prepares);
            let terminal_prepares = Arc::clone(&terminal_prepares);
            connection
                .authorizer(Some(move |context: AuthContext<'_>| {
                    match context.action {
                        AuthAction::Insert { table_name } if context.accessor.is_none() => {
                            match table_name {
                                "canonical_nodes" => {
                                    canonical_prepares.fetch_add(1, Ordering::Relaxed);
                                }
                                "search_index" => {
                                    search_index_prepares.fetch_add(1, Ordering::Relaxed);
                                }
                                "search_index_v2" => {
                                    fielded_index_prepares.fetch_add(1, Ordering::Relaxed);
                                }
                                "_fathomdb_artifact_revisions" => {
                                    artifact_prepares.fetch_add(1, Ordering::Relaxed);
                                }
                                "_fathomdb_projection_terminal" => {
                                    terminal_prepares.fetch_add(1, Ordering::Relaxed);
                                }
                                _ => {}
                            }
                        }
                        AuthAction::Update {
                            table_name: "canonical_nodes",
                            column_name: "superseded_at",
                        } if context.accessor.is_none() => {
                            supersession_prepares.fetch_add(1, Ordering::Relaxed);
                        }
                        AuthAction::Read {
                            table_name: "_fathomdb_artifact_revisions",
                            column_name: "revision_id",
                        } if context.accessor.is_none() => {
                            revision_probe_prepares.fetch_add(1, Ordering::Relaxed);
                        }
                        _ => {}
                    }
                    Authorization::Allow
                }))
                .expect("install preparation counter");
        }

        let source_id = SourceId::new("test:statement-cache").unwrap();
        let batch = (0..4)
            .map(|index| PreparedWrite::Node {
                kind: "doc".to_string(),
                body: format!("body {index}"),
                source_id: source_id.clone(),
                logical_id: Some(format!("logical-{index}")),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            })
            .collect::<Vec<_>>();
        opened.engine.write(&batch).expect("batch write");

        assert_eq!(canonical_prepares.load(Ordering::Relaxed), 1);
        assert_eq!(supersession_prepares.load(Ordering::Relaxed), 1);
        assert_eq!(search_index_prepares.load(Ordering::Relaxed), 1);
        assert_eq!(fielded_index_prepares.load(Ordering::Relaxed), 1);
        assert_eq!(revision_probe_prepares.load(Ordering::Relaxed), 1);
        assert_eq!(artifact_prepares.load(Ordering::Relaxed), 1);
        assert_eq!(terminal_prepares.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn governed_node_batch_advances_visibility_once() {
        let dir = TempDir::new().unwrap();
        let opened = Engine::open(dir.path().join("visibility-batch.sqlite")).unwrap();
        let before = crate::frozen_read::load_visibility_generation(
            opened.engine.connection.lock().unwrap().as_ref().unwrap(),
        )
        .unwrap();
        let source_id = SourceId::new("test:visibility-batch").unwrap();
        let batch = (0..4)
            .map(|index| PreparedWrite::Node {
                kind: "doc".to_string(),
                body: format!("body {index}"),
                source_id: source_id.clone(),
                logical_id: Some(format!("logical-{index}")),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            })
            .collect::<Vec<_>>();

        opened.engine.write(&batch).unwrap();

        let after = crate::frozen_read::load_visibility_generation(
            opened.engine.connection.lock().unwrap().as_ref().unwrap(),
        )
        .unwrap();
        assert_eq!(after, before + 1);
    }

    #[test]
    fn projection_batches_coalesce_visibility_invalidation() {
        let dir = TempDir::new().unwrap();
        let opened = Engine::open_with_embedder_for_test(
            dir.path().join("projection-visibility-batch.sqlite"),
            Arc::new(Slice65ProjectionEmbedder),
        )
        .unwrap();
        opened.engine.configure_vector_kind_for_test("doc").unwrap();
        let before = crate::frozen_read::load_visibility_generation(
            opened.engine.connection.lock().unwrap().as_ref().unwrap(),
        )
        .unwrap();
        let source_id = SourceId::new("test:projection-visibility-batch").unwrap();
        let batch = (0..32)
            .map(|index| PreparedWrite::Node {
                kind: "doc".to_string(),
                body: format!("body {index}"),
                source_id: source_id.clone(),
                logical_id: Some(format!("logical-{index}")),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            })
            .collect::<Vec<_>>();

        opened.engine.write(&batch).unwrap();
        opened.engine.drain(5_000).unwrap();

        let guard = opened.engine.connection.lock().unwrap();
        let connection = guard.as_ref().unwrap();
        let after = crate::frozen_read::load_visibility_generation(connection).unwrap();
        assert!(
            after <= before + 1 + batch.len() as u64,
            "projection visibility advanced once per row mutation: before={before}, after={after}"
        );
        assert_eq!(
            connection
                .query_row("SELECT COUNT(*) FROM _fathomdb_vector_rows", [], |row| {
                    row.get::<_, i64>(0)
                })
                .unwrap(),
            batch.len() as i64
        );
    }

    #[test]
    fn custom_projection_trigger_forces_row_trigger_fallback() {
        let dir = TempDir::new().unwrap();
        let opened = Engine::open_with_embedder_for_test(
            dir.path().join("projection-custom-trigger.sqlite"),
            Arc::new(Slice65ProjectionEmbedder),
        )
        .unwrap();
        opened.engine.configure_vector_kind_for_test("doc").unwrap();
        {
            let guard = opened.engine.connection.lock().unwrap();
            guard
                .as_ref()
                .unwrap()
                .execute_batch(
                    "CREATE TABLE custom_projection_fires(id INTEGER PRIMARY KEY);
                     CREATE TRIGGER custom_projection_insert
                     AFTER INSERT ON _fathomdb_vector_rows
                     BEGIN INSERT INTO custom_projection_fires VALUES(NULL); END;",
                )
                .unwrap();
        }
        let source_id = SourceId::new("test:projection-custom-trigger").unwrap();
        let batch = (0..4)
            .map(|index| PreparedWrite::Node {
                kind: "doc".to_string(),
                body: format!("body {index}"),
                source_id: source_id.clone(),
                logical_id: Some(format!("logical-{index}")),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            })
            .collect::<Vec<_>>();

        opened.engine.write(&batch).unwrap();
        opened.engine.drain(5_000).unwrap();

        let guard = opened.engine.connection.lock().unwrap();
        assert_eq!(
            guard
                .as_ref()
                .unwrap()
                .query_row("SELECT COUNT(*) FROM custom_projection_fires", [], |row| {
                    row.get::<_, i64>(0)
                })
                .unwrap(),
            batch.len() as i64
        );
    }

    #[test]
    fn visibility_exhaustion_rolls_back_and_restores_triggers() {
        let dir = TempDir::new().unwrap();
        let opened = Engine::open(dir.path().join("visibility-exhaustion.sqlite")).unwrap();
        {
            let guard = opened.engine.connection.lock().unwrap();
            let connection = guard.as_ref().unwrap();
            connection
                .execute(
                    "UPDATE _fathomdb_read_visibility_state
                     SET generation=9223372036854775807 WHERE singleton=1",
                    [],
                )
                .unwrap();
        }
        let node = PreparedWrite::Node {
            kind: "doc".to_string(),
            body: "body".to_string(),
            source_id: SourceId::new("test:visibility-exhaustion").unwrap(),
            logical_id: Some("logical".to_string()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        };

        assert!(matches!(
            opened.engine.write(std::slice::from_ref(&node)),
            Err(EngineError::Storage)
        ));

        {
            let guard = opened.engine.connection.lock().unwrap();
            let connection = guard.as_ref().unwrap();
            assert!(connection
                .db_config(rusqlite::config::DbConfig::SQLITE_DBCONFIG_ENABLE_TRIGGER)
                .unwrap());
            assert_eq!(
                connection
                    .query_row("SELECT COUNT(*) FROM canonical_nodes", [], |row| {
                        row.get::<_, i64>(0)
                    })
                    .unwrap(),
                0
            );
            assert_eq!(
                crate::frozen_read::load_visibility_generation(connection).unwrap(),
                9_223_372_036_854_775_807
            );
            connection
                .execute(
                    "UPDATE _fathomdb_read_visibility_state SET generation=0 WHERE singleton=1",
                    [],
                )
                .unwrap();
        }
        opened.engine.write(&[node]).unwrap();
        let guard = opened.engine.connection.lock().unwrap();
        assert_eq!(
            crate::frozen_read::load_visibility_generation(guard.as_ref().unwrap()).unwrap(),
            1
        );
    }

    #[test]
    fn custom_canonical_trigger_forces_row_trigger_fallback() {
        let dir = TempDir::new().unwrap();
        let opened = Engine::open(dir.path().join("visibility-custom-trigger.sqlite")).unwrap();
        let before;
        {
            let guard = opened.engine.connection.lock().unwrap();
            let connection = guard.as_ref().unwrap();
            connection
                .execute_batch(
                    "CREATE TABLE custom_trigger_fires(id INTEGER PRIMARY KEY);
                     CREATE TRIGGER custom_canonical_insert
                     AFTER INSERT ON canonical_nodes
                     BEGIN INSERT INTO custom_trigger_fires VALUES(NULL); END;",
                )
                .unwrap();
            before = crate::frozen_read::load_visibility_generation(connection).unwrap();
        }
        let source_id = SourceId::new("test:visibility-custom-trigger").unwrap();
        let batch = (0..4)
            .map(|index| PreparedWrite::Node {
                kind: "doc".to_string(),
                body: format!("body {index}"),
                source_id: source_id.clone(),
                logical_id: Some(format!("logical-{index}")),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            })
            .collect::<Vec<_>>();

        opened.engine.write(&batch).unwrap();

        let guard = opened.engine.connection.lock().unwrap();
        let connection = guard.as_ref().unwrap();
        assert_eq!(
            connection
                .query_row("SELECT COUNT(*) FROM custom_trigger_fires", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            4
        );
        assert_eq!(
            crate::frozen_read::load_visibility_generation(connection).unwrap(),
            before + 12
        );
    }

    #[test]
    fn custom_internal_trigger_targets_force_row_trigger_fallback() {
        for (case, table) in [
            ("open-state", "_fathomdb_open_state"),
            ("visibility-state", "_fathomdb_read_visibility_state"),
        ] {
            let dir = TempDir::new().unwrap();
            let opened = Engine::open(dir.path().join(format!("{case}.sqlite"))).unwrap();
            {
                let guard = opened.engine.connection.lock().unwrap();
                let connection = guard.as_ref().unwrap();
                let insert_trigger = if table == "_fathomdb_open_state" {
                    format!(
                        "CREATE TRIGGER custom_internal_insert
                         AFTER INSERT ON {table}
                         BEGIN INSERT INTO custom_trigger_fires VALUES(NULL); END;"
                    )
                } else {
                    String::new()
                };
                connection
                    .execute_batch(&format!(
                        "CREATE TABLE custom_trigger_fires(id INTEGER PRIMARY KEY);
                         CREATE TRIGGER custom_internal_update
                         AFTER UPDATE ON {table}
                         BEGIN INSERT INTO custom_trigger_fires VALUES(NULL); END;
                         {insert_trigger}"
                    ))
                    .unwrap();
            }

            opened
                .engine
                .write(&[PreparedWrite::Node {
                    kind: "doc".to_string(),
                    body: "body".to_string(),
                    source_id: SourceId::new(format!("test:{case}")).unwrap(),
                    logical_id: Some(format!("logical-{case}")),
                    state: InitialState::Active,
                    reason: None,
                    valid_from: None,
                    valid_until: None,
                }])
                .unwrap();

            let guard = opened.engine.connection.lock().unwrap();
            assert!(
                guard
                    .as_ref()
                    .unwrap()
                    .query_row("SELECT COUNT(*) FROM custom_trigger_fires", [], |row| {
                        row.get::<_, i64>(0)
                    })
                    .unwrap()
                    > 0,
                "custom trigger on {table} was suppressed"
            );
        }
    }

    proptest! {
        #[test]
        fn completed_rank_group_matches_the_full_stable_prefix(
            scores in proptest::collection::vec(-20_i16..=0_i16, 1..200),
            requested_limit in 1_usize..100,
        ) {
            let hit = |cursor: u64, score: f64| SearchHit {
                id: IdSpace::logical(format!("rank-stream-{cursor}")),
                write_cursor: cursor,
                kind: "doc".to_string(),
                body: format!("body {cursor}"),
                score,
                branch: SoftFallbackBranch::Text,
                source_id: Some("slice20:test".to_string()),
                ce_score: None,
            };
            let mut native = scores
                .iter()
                .enumerate()
                .map(|(index, score)| hit(index as u64 + 1, f64::from(*score)))
                .collect::<Vec<_>>();
            native.sort_by(|left, right| {
                left.score
                    .total_cmp(&right.score)
                    .then_with(|| right.write_cursor.cmp(&left.write_cursor))
            });
            let limit = requested_limit.min(native.len());
            let boundary_score = native[limit - 1].score;
            let completed = native
                .iter()
                .take_while(|candidate| {
                    candidate.score.total_cmp(&boundary_score) != std::cmp::Ordering::Greater
                })
                .cloned()
                .collect::<Vec<_>>();
            let selected = retain_complete_rank_boundary_candidates(completed, limit);
            let mut expected = native;
            expected.sort_by(|left, right| {
                left.score
                    .total_cmp(&right.score)
                    .then_with(|| left.write_cursor.cmp(&right.write_cursor))
            });
            expected.truncate(limit);
            prop_assert_eq!(selected, expected);
        }
    }
}
