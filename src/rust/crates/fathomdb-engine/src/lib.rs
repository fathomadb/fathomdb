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
mod ingest;
pub mod lifecycle;
mod mean;
mod pagination;
mod projection_commit;
mod projection_generation;
#[cfg(feature = "operator")]
mod projection_rebuild;
mod projection_registry;
mod projection_runtime;
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
mod search;
mod search_api;
mod search_types;
mod structural_state;
#[cfg(feature = "tc5-benchmark")]
pub mod tc5_benchmark;
mod telemetry;
mod temporal;
mod test_hooks;
mod vector_equivalence;
mod vector_storage;
mod wal_attribution;
mod write;
mod write_commit;
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
use embed_dispatch::{DispatchError, EmbedDispatcher, EmbedOutput, EmbedReply};
use embedding::map_runtime_embedder_error;
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
    append_edge_eligibility_sql, append_node_eligibility_sql, body_fts_rank_sql,
    build_vector_phase1_sql, edge_fts_hit_passes_filter, edge_fts_rank_sql, property_fts_rank_sql,
    text_hit_passes_filter,
};
pub use filter::{ComparisonOp, Filter, FilterTerm, Predicate, ScalarValue, SearchFilter};
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
pub use ingest::{ExtractDocument, IngestWithExtractorReceipt};
use mean::{
    identity_requires_mean_centering, read_pinned_mean_vec, recover_mean_vec_pin,
    run_pin_and_requantize_pass, run_requantize_pass, subtract_mean, MeanAccumulator,
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
    ProvenancedNodeV1, SourceLocator, WriteProvenanceV1,
};
pub(crate) use provider::{ProviderSession, ProviderTask};
#[cfg(feature = "test-hooks")]
pub(crate) use read::{
    canonical_page_query, OPERATIONAL_STATE_PAGE_SQL, OPERATIONAL_STATE_POINT_SQL,
};
pub use read::{NodeRecord, OpStoreRow, OperationalStateRecordV1};
#[cfg(debug_assertions)]
pub use reader_pool::CacheStatusReply;
pub(crate) use reader_pool::ReaderWorkerPool;
pub use record_lifecycle::{InitialState, LifecycleState};
pub use rerank::rerank_passages;
#[doc(hidden)]
pub use rerank::{rerank_fused, try_rerank_fused};
use runtime_configuration::ResolvedRuntimeConfiguration;
pub use runtime_configuration::{EngineConfig, EngineConfigurationError};
#[cfg(test)]
pub(crate) use search::retain_complete_rank_boundary_candidates;
#[cfg(feature = "test-hooks")]
pub(crate) use search::{
    append_json_witness_for_test, record_slice71_profile_statement_for_test,
    slice71_search_statement_trace,
};
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
    checked_locator_columns, commit_batch, revision_hash_field, CommitBatchError,
    TriggerStateGuard,
};
pub(crate) use write_validation::{
    collect_projection_jobs, prior_edge_cursors_by_logical_id, prior_edge_cursors_by_triple,
    prior_node_cursors_by_logical_id, validate_batch, validate_write, WritePlan,
};

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
use std::sync::Once;
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
    migrate_with_event_sink, MigrationError as SchemaMigrationError, MigrationStepReport,
    LOCK_SUFFIX, MIGRATIONS, SCHEMA_VERSION,
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
use sqlite_vec::sqlite3_vec_init;

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

// EU-5b lock-flip: the engine's default embedder identity is now the
// pinned bge-small variant. Pre-existing 0.7.0 workspaces opened with
// `EmbedderChoice::Default` will fail-closed on identity mismatch per
// ADR-0.6.0-vector-identity-embedder-owned; callers can still hold an
// older noop profile by supplying `EmbedderChoice::Caller(NoopEmbedder)`.
const DEFAULT_EMBEDDER_NAME: &str = "fathomdb-bge-small-en-v1.5";
const DEFAULT_EMBEDDER_REVISION: &str = "5c38ec7c405ec4b44b94cc5a9bb96e735b38267a";
const DEFAULT_EMBEDDER_DIMENSION: u32 = 384;

/// Identity name of the bge-small embedder. `OpenReport.embedder_mean_centering_required`
/// is `true` iff the live embedder identity reports this name. NoopEmbedder
/// is `false`. Lifted out as a constant so the EU-5b lock-flip (when the
/// engine's default identity becomes bge-small) is a single-line change.
///
/// TODO(EU-5b): when `DEFAULT_EMBEDDER_NAME` flips to this constant, the
/// Default path will populate `embedder_mean_centering_required = true`
/// without further engine work. Caller-supplied bge-small (rare today)
/// already does the right thing.
const BGE_SMALL_EMBEDDER_NAME: &str = "fathomdb-bge-small-en-v1.5";

/// REQ-006a / AC-007a default slow-statement threshold. Mutated at runtime
/// via [`Engine::set_slow_threshold_ms`].
const DEFAULT_SLOW_THRESHOLD_MS: u64 = 100;
const DEFAULT_VECTOR_PROFILE: &str = "default";
const DEFAULT_VECTOR_PARTITION: &str = "vector_default";

/// 0.8.18 Slice 5 (#5 vector-equivalence probe) — the committed 45-probe fixture
/// (byte-identical to `fathomdb-embedder/tests/fixtures/candle_onnx_equivalence_probes.txt`;
/// a drift-guard test pins the two copies equal). One probe per non-empty line;
/// lines whose first non-whitespace char is `#` are comments.
const VECTOR_EQUIVALENCE_PROBE_FIXTURE: &str = include_str!("vector_equivalence_probes.txt");

/// 0.8.18 Slice 5 (#5 vector-equivalence probe) — the FROZEN D4 tolerance floor,
/// **P2 component**: the un-centered Phase-2 L2 epsilon. `‖reembed − reference‖₂`
/// (un-centered, `vec_distance_l2` semantics) strictly greater than this ⇒
/// divergence ⇒ dense refused. Named constant so the final ε (HITL look at
/// landing) is trivially tunable. The **P1 component** (Phase-1 mean-centered
/// `embedding_bin` sign-flip count) has an *exact-zero* floor: ANY single flip on
/// the 45 probes ⇒ divergence (see [`VECTOR_EQUIVALENCE_P1_FLIP_FLOOR`]).
const VECTOR_EQUIVALENCE_L2_EPSILON: f32 = 1e-5;

/// 0.8.18 Slice 5 — the FROZEN D4 tolerance floor, **P1 component**: the maximum
/// tolerated Phase-1 mean-centered `embedding_bin` sign-flip count across all 45
/// probes. `0` = exact: any single flip ⇒ divergence ⇒ dense refused.
const VECTOR_EQUIVALENCE_P1_FLIP_FLOOR: u64 = 0;

/// 0.8.20 Slice 22 (TC-68) — `_fathomdb_open_state` key holding the
/// `probe_verification_fingerprint` of the last open at which the
/// vector-equivalence probe actually RAN and PASSED on this workspace. An open
/// whose freshly computed fingerprint equals this value reuses that verdict and
/// performs ZERO probe embeds; anything else re-runs the full probe.
///
/// It lives in `_fathomdb_open_state` — the engine's existing open-time
/// durable-marker KV table (migration step 1) — alongside
/// [`SEARCH_INDEX_TOKENIZER_REPROJECT_MARKER_KEY`] and
/// [`EDGE_VECTOR_PRUNE_MARKER_KEY`], which is exactly this shape of state. So
/// TC-68 adds **no** table, **no** migration step and **no** `SCHEMA_VERSION`
/// bump: an old DB simply has no row here and re-runs the probe once.
const VECTOR_EQUIVALENCE_VERDICT_CACHE_KEY: &str = "vector_equivalence_verified_fingerprint";

/// 0.8.20 Slice 22 (TC-68) — recipe tag mixed into every verdict fingerprint.
/// **Bump it whenever the SET of fingerprint inputs changes.** Every cached
/// verdict in the field then stops matching and the probe re-runs once per
/// workspace — the fail-SAFE direction, and the reason a stale recipe can never
/// silently keep vouching for a narrower check than the current build performs.
const VECTOR_EQUIVALENCE_FINGERPRINT_RECIPE: &str = "fathomdb-veq-verdict-v1";
/// Default drain budget for `rebuild_projections` / `rebuild_vec0`. The
/// rebuild path freezes the scheduler before truncating shadow rows, so
/// the only outstanding work is whatever workers were mid-flight when
/// the call landed; 30 s is generous for normal job sizes and bounded
/// for tests.
#[cfg(feature = "operator")]
const REBUILD_DRAIN_TIMEOUT_MS: u64 = 30_000;
/// OPP-12 Phase-1 (0.8.19 Slice 10) — drain budget the `transition`/`purge`
/// lifecycle verbs use to settle in-flight projection work before mutating.
/// Same 30 s budget as `REBUILD_DRAIN_TIMEOUT_MS`, but not `operator`-gated
/// (the lifecycle verbs are always-on governed surface).
const LIFECYCLE_DRAIN_TIMEOUT_MS: u64 = 30_000;
/// 0.8.0 Slice 5 (G1) — schema version that introduces the global FTS5
/// tokenizer-default upgrade (`SCHEMA_VERSION` 11, migration step 11). A DB
/// migrated to (or past) this version re-tokenizes `search_index` from
/// canonical source rows on open (the drop+recreate leaves the FTS index
/// empty). Repair is keyed off the completion marker below — NOT off crossing
/// the step boundary — so it is crash-retryable (see
/// `SEARCH_INDEX_TOKENIZER_REPROJECT_MARKER_KEY`).
const SEARCH_INDEX_TOKENIZER_SCHEMA_VERSION: u32 = 11;
/// 0.8.0 Slice 5 (G1) fix-1 — `_fathomdb_open_state` key set, in the SAME
/// transaction as the reproject DELETE+INSERT, once the post-tokenizer-upgrade
/// re-tokenization commits durably. Step 11 commits `user_version = 11` with an
/// EMPTY `search_index` in its own transaction; the reproject runs in a later
/// transaction on open. A crash in that window leaves a durable `user_version =
/// 11` + empty index. Gating repair on a boundary crossing (`before < 11`)
/// would skip it on the next open (it sees `before == 11`), stranding the index
/// empty forever. Gating on this marker's ABSENCE instead makes repair
/// idempotent and crash-retryable: written atomically with the reindex, so a
/// crash before commit leaves no marker and the next open re-runs.
const SEARCH_INDEX_TOKENIZER_REPROJECT_MARKER_KEY: &str =
    "search_index_tokenizer_reproject_complete";
/// 0.8.20 Slice 15c (TC-33) fix-6 — schema version at which the
/// `canonical_edges` INTEGER-epoch recreate (migration step 23) runs. A DB
/// migrated to (or past) this version has had every edge row DROPPED with NO
/// DATA MIGRATION, and the migration removed the dropped edges'
/// `_fathomdb_vector_rows` sidecar rows. The vec0 `vector_default` shadow it
/// mirrors is engine-created and dim-parameterized, so the migration cannot
/// touch it; the engine prunes the now-orphaned vec0 rows on open.
const EDGE_TEMPORAL_EPOCH_SCHEMA_VERSION: u32 = 23;

const DEPENDENCY_GENERATION_KEY: &str = "_fathomdb_dependency_generation";
const SOURCE_DEPENDENCY_SCHEMA_VERSION: u32 = 28;
const DEPENDENCY_LOOKUP_LIMIT: usize = 100;
/// 0.8.20 Slice 15c (TC-33) fix-6 — `_fathomdb_open_state` key set once the
/// one-time edge-vector prune commits durably (written in the SAME transaction
/// as the vec0 DELETEs). Gating repair on this marker's ABSENCE — not on
/// crossing the step-23 boundary — makes it crash-retryable: the step-23
/// migration commits `user_version = 23` (edges dropped, sidecar cleared) in its
/// own transaction, and the prune runs in a later transaction on open. A crash
/// in that window leaves a durable `user_version = 23` with orphaned vec0 rows;
/// a boundary-crossing gate (`before < 23`) would skip the prune forever on the
/// next open (it sees `before == 23`). The marker is absent on any DB upgraded
/// before this fix shipped, so the prune runs once and cleans the lingering
/// orphans; thereafter the paired vec0/sidecar insert+delete keeps the invariant
/// so no new orphans arise.
const EDGE_VECTOR_PRUNE_MARKER_KEY: &str = "tc33_edge_vector_prune_complete";
const DEFAULT_PROVENANCE_ROW_CAP: u64 = 1_000_000;
/// 0.8.20 Slice 5b (R-20-E5) — how many times an erasure verb re-tries
/// `PRAGMA wal_checkpoint(TRUNCATE)` before refusing with
/// [`EngineError::ErasureIncomplete`]. Deliberately small: a concurrent reader
/// pinning a WAL snapshot can hold it for an unbounded time, and an erasure verb
/// must fail loudly rather than block a caller indefinitely.
const ERASURE_WAL_TRUNCATE_ATTEMPTS: u32 = 5;
/// 0.8.20 Slice 5b (R-20-E5) — pause between WAL-truncation attempts
/// (~100 ms total budget across [`ERASURE_WAL_TRUNCATE_ATTEMPTS`]).
const ERASURE_WAL_TRUNCATE_BACKOFF_MS: u64 = 25;
/// 0.8.20 Slice 5b (R-20-E6) — the sentinel that replaces an erased
/// `result_stable_ids` element in the telemetry sink. Positional alignment with
/// the parallel `result_ids` array is preserved, so a redacted sink stays
/// parseable by the gold pipeline.
const REDACTED_STABLE_ID: &str = "[erased]";
/// 0.8.20 Slice 5b (design `0.8.20-slice0-erasure-design.md` §2 defect D-A,
/// HITL-ruled 2026-07-19: *"there must be an auditable record of deletion
/// event."*) — op-store collections holding ERASURE-AUDIT records.
///
/// These rows are **exempt from `enforce_provenance_retention`**. Before this
/// slice they were swept like any other op-store row: cap-first, oldest-`id`
/// first, with no collection filter — and because the audit row is written
/// *before* the workload that follows it, it was among the FIRST evicted. The
/// proof of erasure was therefore destructible, and shared a retention pool with
/// the very payloads it must prove erased. Accountability (demonstrating *that*
/// an erasure occurred) is a distinct obligation from erasure itself, and a
/// retention sweep must not silently discharge it.
///
/// **Guarantee:** a row in one of these collections is never removed by the
/// retention sweep, and (0.8.20 Slice 5 fix-3) never by
/// [`Engine::excise_collection_record`] either — see
/// [`is_erasure_bookkeeping_collection`].
const ERASURE_AUDIT_COLLECTIONS: &[&str] = &["excise_source_audit", "excise_record_audit"];
/// 0.8.20 Slice 5 fix-1 (codex §9 P2) — the `operational_mutations` collection
/// holding the DURABLE record of a telemetry redaction that is owed but not yet
/// performed. See [`Engine::discharge_pending_redactions`].
///
/// Like the audit collections it is exempt from the retention sweep: an
/// outstanding erasure obligation must not be discharged by cap pressure.
const ERASURE_PENDING_REDACTION_COLLECTION: &str = "erasure_pending_redaction";
/// 0.8.20 Slice 5 fix-3 (codex §9 round-3 P1) — true for the op-store
/// collections that hold the engine's ERASURE BOOKKEEPING: the durable
/// pending-redaction queue and the erasure-audit trail.
///
/// These are engine-owned invariants that happen to be *stored* as op-store
/// records. They are not caller data, and the generic record-erasure verb
/// [`Engine::excise_collection_record`] must refuse to target them:
///
/// * **The pending queue** ([`ERASURE_PENDING_REDACTION_COLLECTION`]) records a
///   telemetry redaction the engine still OWES. Deleting the entry makes
///   [`Engine::complete_erasure_at_rest`] see no outstanding work, so the next
///   erasure verb reports SUCCESS while the erased `l:`/`h:` ids are still in
///   the telemetry sink. That is the exact R-20-E5 violation — *an erasure verb
///   must never report success on an incomplete erasure* — that the queue was
///   introduced to close, and the verb re-opened it through an
///   operator-reachable path (`--excise-collection erasure_pending_redaction
///   --excise-record-key <verb>`).
/// * **The audit trail** ([`ERASURE_AUDIT_COLLECTIONS`]) is protected by the
///   HITL ruling of 2026-07-19: *"there must be an auditable record of deletion
///   event."* Deleting audit rows one-by-one defeats that ruled-on guarantee as
///   surely as a retention sweep would. Accountability is a distinct obligation
///   from erasure, and no verb may silently discharge it.
///
/// Neither carries erasable payload, so refusing them costs a caller nothing: a
/// pending-queue row holds only stable ids the engine is about to remove from
/// the sink (and deletes itself on discharge), and an audit row holds a
/// `source_id` bound by the non-PII rule or a SHA-256 record digest.
///
/// The refusal is TYPED ([`EngineError::InvalidArgument`]), never a silent
/// no-op — an operator who aimed at the wrong collection must be told. The shape
/// mirrors the slice's existing precedent: [`Engine::erase_source`] refuses the
/// reserved `_`-prefixed provenance namespace while [`Engine::excise_source`]
/// stays permissive.
///
/// Gated on `feature = "operator"` to match its only call site,
/// [`Engine::excise_collection_record`], which is itself operator-only: without
/// the matching `cfg` a non-`operator` build emits a `dead_code` warning for a
/// helper that has nothing to guard, because the verb it guards does not exist
/// in that build.
#[cfg(feature = "operator")]
fn is_erasure_bookkeeping_collection(collection: &str) -> bool {
    collection == ERASURE_PENDING_REDACTION_COLLECTION
        || ERASURE_AUDIT_COLLECTIONS.contains(&collection)
}
const PROJECTION_CURSOR_KEY: &str = "projection_cursor";
#[cfg(test)]
const PROJECTION_WORKERS: usize = 2;
const DEFAULT_EMBED_TIMEOUT_MS: u64 = 30_000;
const PROJECTION_COMMIT_BATCH: usize = 64;
const PROJECTION_TEMPORAL_WAKE_POLL: Duration = Duration::from_secs(1);
const DEFAULT_PROJECTION_RETRY_DELAYS_MS: [u64; 3] = [1_000, 4_000, 16_000];

/// G11 — the fixed projection kind every EDGE body is scheduled under
/// (`resolve_source_type` maps it to `source_type = 'edge_fact'` in
/// `vector_default`). Named so the fix-4 deferral can say WHICH rows it
/// deliberately leaves on the shipped terminal path (see
/// `projection_dispatcher_loop`).
const EDGE_FACT_KIND: &str = "edge_fact";

/// Reader pool size. Per `dev/design/engine.md` § Writer / reader split,
/// reader connections are pooled and never serialize behind one
/// connection. AC-021 exercises 8 concurrent readers.
const READER_POOL_SIZE: usize = 8;

/// Per-reader-connection lookaside slot size, in bytes. Pack 6.G G.1.
/// Picked from G.0 telemetry (`allocator_lookaside` 26.67% conc cycles
/// with 3.89× ratio) + the SQLite docs' typical-workload sizing
/// guidance (https://www.sqlite.org/malloc.html §3): 1200-byte slots
/// cover the small allocations from `sqlite3DbMallocRaw`,
/// `sqlite3Fts5ExprNew`, and `vec0Filter_knn` visible at the top of the
/// concurrent profile.
const READER_LOOKASIDE_SLOT_SIZE: std::os::raw::c_int = 1200;

/// Per-reader-connection lookaside slot count. SQLite default is 128;
/// we use 500 to absorb the per-statement allocation footprint of the
/// hybrid search workload across a sticky worker connection without
/// falling back to the glibc malloc-arena mutex.
const READER_LOOKASIDE_SLOT_COUNT: std::os::raw::c_int = 500;

static EXPLANATION_OPEN_NONCE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn mint_explanation_open_nonce() -> u128 {
    let time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    let sequence = EXPLANATION_OPEN_NONCE_SEQUENCE.fetch_add(1, Ordering::Relaxed) as u128;
    time.rotate_left(17) ^ sequence
}

#[cfg(all(feature = "test-hooks", target_os = "linux"))]
fn process_current_rss_bytes() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|status| {
            status.lines().find_map(|line| {
                line.strip_prefix("VmRSS:")
                    .and_then(|value| value.split_whitespace().next())
                    .and_then(|value| value.parse::<u64>().ok())
            })
        })
        .unwrap_or(0)
        .saturating_mul(1024)
}

#[cfg(all(feature = "test-hooks", not(target_os = "linux")))]
fn process_current_rss_bytes() -> u64 {
    0
}

#[cfg(all(feature = "test-hooks", target_os = "linux"))]
fn process_peak_rss_bytes() -> u64 {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::zeroed();
    // SAFETY: `getrusage` initializes the supplied `rusage` on a zero return.
    if unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) } == 0 {
        // SAFETY: guarded by the successful `getrusage` return above.
        let kilobytes = unsafe { usage.assume_init() }.ru_maxrss;
        u64::try_from(kilobytes).unwrap_or(0).saturating_mul(1024)
    } else {
        0
    }
}

#[cfg(all(feature = "test-hooks", not(target_os = "linux")))]
fn process_peak_rss_bytes() -> u64 {
    0
}

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

const PROJECTION_RUNTIME_STARTUP_TIMEOUT: Duration = Duration::from_secs(30);

struct OpenEmbedDispatchGuard(Option<Arc<EmbedDispatcher>>);

impl OpenEmbedDispatchGuard {
    fn disarm(&mut self) {
        self.0.take();
    }
}

impl Drop for OpenEmbedDispatchGuard {
    fn drop(&mut self) {
        if let Some(dispatch) = self.0.take() {
            dispatch.close();
            let _ = dispatch.join_until(Instant::now() + Duration::from_secs(30));
        }
    }
}

/// Test-only live-connection audit. Each long-lived Engine connection acquires
/// one registration after its actual SQLite handle exists and drops it when the
/// handle leaves service; an incomplete registry is a diagnostic failure, never
/// evidence of an external WAL holder.
#[cfg(any(test, feature = "test-hooks"))]
#[derive(Default)]
struct ManagedConnectionRegistry {
    live: Mutex<BTreeSet<(WalAttributionRole, usize)>>,
    opens: Mutex<BTreeMap<ManagedConnectionCategory, usize>>,
    #[cfg(test)]
    runtime_probe_lifecycle: Mutex<RuntimeProbeLifecycle>,
}

#[cfg(any(test, feature = "test-hooks"))]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum ManagedConnectionCategory {
    Writer,
    ReaderWorker,
    ProjectionDispatcher,
    ProjectionWorker,
    RuntimeProbe,
}

#[cfg(any(test, feature = "test-hooks"))]
struct ManagedConnectionRegistration {
    registry: Arc<ManagedConnectionRegistry>,
    role: WalAttributionRole,
    index: usize,
}

/// Private lifecycle facts for an independent diagnostic probe. A probe is
/// useful only after its native SQLite connection has actually been dropped;
/// an implicit drop is deliberately retained as incomplete rather than being
/// mistaken for an external holder.
#[cfg(test)]
#[derive(Clone, Copy, Debug, Default)]
struct RuntimeProbeLifecycle {
    live: usize,
    actual_drops: usize,
    incomplete_drops: usize,
}

#[cfg(test)]
struct RuntimeProbeRegistration {
    registry: Arc<ManagedConnectionRegistry>,
    acknowledged: bool,
}

#[cfg(test)]
struct RuntimeProbeConnection {
    connection: Option<Connection>,
    registration: RuntimeProbeRegistration,
}

#[cfg(any(test, feature = "test-hooks"))]
impl ManagedConnectionRegistry {
    fn register(
        self: &Arc<Self>,
        role: WalAttributionRole,
        index: usize,
    ) -> ManagedConnectionRegistration {
        let inserted = self.live.lock().expect("managed connection registry").insert((role, index));
        assert!(inserted, "duplicate managed connection registration for {}:{index}", role.name());
        ManagedConnectionRegistration { registry: Arc::clone(self), role, index }
    }

    fn exact_live(&self, worker_count: usize) -> bool {
        let expected = native_state_expected_roles(worker_count);
        self.live.lock().map(|live| *live == expected).unwrap_or(false)
    }

    fn record_open(&self, category: ManagedConnectionCategory) {
        let mut opens = self.opens.lock().expect("managed connection open audit");
        *opens.entry(category).or_default() += 1;
    }

    fn creation_counts(&self) -> Option<(usize, usize, usize, usize, usize)> {
        let opens = self.opens.lock().ok()?;
        Some((
            *opens.get(&ManagedConnectionCategory::Writer).unwrap_or(&0),
            *opens.get(&ManagedConnectionCategory::ReaderWorker).unwrap_or(&0),
            *opens.get(&ManagedConnectionCategory::ProjectionDispatcher).unwrap_or(&0),
            *opens.get(&ManagedConnectionCategory::ProjectionWorker).unwrap_or(&0),
            *opens.get(&ManagedConnectionCategory::RuntimeProbe).unwrap_or(&0),
        ))
    }

    #[cfg(test)]
    fn register_runtime_probe(self: &Arc<Self>) -> RuntimeProbeRegistration {
        let mut lifecycle = self.runtime_probe_lifecycle.lock().expect("runtime probe lifecycle");
        lifecycle.live += 1;
        RuntimeProbeRegistration { registry: Arc::clone(self), acknowledged: false }
    }
}

#[cfg(any(test, feature = "test-hooks"))]
impl Drop for ManagedConnectionRegistration {
    fn drop(&mut self) {
        if let Ok(mut live) = self.registry.live.lock() {
            live.remove(&(self.role, self.index));
        }
    }
}

#[cfg(test)]
impl RuntimeProbeRegistration {
    fn acknowledge_actual_drop(&mut self) -> RuntimeProbeLifecycle {
        let mut lifecycle =
            self.registry.runtime_probe_lifecycle.lock().expect("runtime probe lifecycle");
        assert!(lifecycle.live > 0, "runtime probe acknowledgement without a live probe");
        lifecycle.live -= 1;
        lifecycle.actual_drops += 1;
        self.acknowledged = true;
        *lifecycle
    }
}

#[cfg(test)]
impl Drop for RuntimeProbeRegistration {
    fn drop(&mut self) {
        if !self.acknowledged {
            if let Ok(mut lifecycle) = self.registry.runtime_probe_lifecycle.lock() {
                lifecycle.live = lifecycle.live.saturating_sub(1);
                lifecycle.incomplete_drops += 1;
            }
        }
    }
}

#[cfg(test)]
impl RuntimeProbeConnection {
    fn open(path: &Path, registry: &Arc<ManagedConnectionRegistry>) -> rusqlite::Result<Self> {
        let connection =
            open_managed_connection(path, ManagedConnectionCategory::RuntimeProbe, registry)?;
        Ok(Self { connection: Some(connection), registration: registry.register_runtime_probe() })
    }

    fn connection(&self) -> &Connection {
        self.connection.as_ref().expect("runtime probe connection remains live")
    }

    fn close_and_acknowledge(mut self) -> rusqlite::Result<RuntimeProbeLifecycle> {
        self.connection
            .take()
            .expect("runtime probe connection remains live")
            .close()
            .map_err(|(_, error)| error)?;
        Ok(self.registration.acknowledge_actual_drop())
    }
}

#[cfg(any(test, feature = "test-hooks"))]
struct RuntimeConnectionInventoryRequest {
    pending: BTreeSet<(WalAttributionRole, usize)>,
    respond: SyncSender<(WalAttributionRole, usize, bool)>,
}

#[cfg(any(test, feature = "test-hooks"))]
struct RuntimeNativeStateRequest {
    pending: BTreeSet<(WalAttributionRole, usize)>,
    respond: SyncSender<NativeConnectionStateFact>,
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

/// Per-connection profile-callback context.
///
/// Holds the registry handle the callback dispatches to, plus shared
/// references to the engine's profiling toggle and slow-statement
/// threshold. The `Arc` clones here mirror the same atomics held by
/// `Engine`, so `set_profiling` / `set_slow_threshold_ms` mutations are
/// visible inside the callback without restart (REQ-006a / AC-005a /
/// AC-007b runtime-toggle contract).
#[derive(Debug)]
struct ProfileContext {
    subscribers: Arc<lifecycle::SubscriberRegistry>,
    profiling_enabled: Arc<AtomicBool>,
    slow_threshold_ms: Arc<AtomicU64>,
    #[cfg(test)]
    callback_uninstalled: AtomicBool,
}

#[cfg(not(test))]
type ProfileContexts = Vec<Box<ProfileContext>>;

#[cfg(test)]
struct ProfileContexts {
    // SQLite stores these allocation addresses; moving a context would invalidate userdata.
    #[allow(clippy::vec_box)]
    contexts: Vec<Box<ProfileContext>>,
    observer: Option<Arc<ProfileReleaseObserver>>,
}

#[cfg(test)]
impl From<Vec<Box<ProfileContext>>> for ProfileContexts {
    fn from(contexts: Vec<Box<ProfileContext>>) -> Self {
        Self { contexts, observer: None }
    }
}

#[cfg(test)]
struct ProfileReleaseObserver {
    registry: Arc<ManagedConnectionRegistry>,
    live_workers: Arc<AtomicUsize>,
    releases: Mutex<Vec<ProfileReleaseFact>>,
    // Custody preserves the original SQLite userdata even in an early-release mutant.
    #[allow(clippy::vec_box)]
    custody: Mutex<Vec<Box<ProfileContext>>>,
}

#[cfg(test)]
struct ProfileReleaseFact {
    callback_uninstalled: bool,
    live_connections: BTreeSet<(WalAttributionRole, usize)>,
    live_workers: usize,
}

#[cfg(test)]
impl ProfileContexts {
    fn clear(&mut self) {
        if let Some(observer) = &self.observer {
            for context in self.contexts.drain(..) {
                observer.releases.lock().unwrap().push(ProfileReleaseFact {
                    callback_uninstalled: context.callback_uninstalled.load(Ordering::SeqCst),
                    live_connections: observer.registry.live.lock().unwrap().clone(),
                    live_workers: observer.live_workers.load(Ordering::SeqCst),
                });
                observer.custody.lock().unwrap().push(context);
            }
        } else {
            self.contexts.clear();
        }
    }
}

#[cfg(test)]
impl Drop for ProfileContexts {
    fn drop(&mut self) {
        self.clear();
    }
}

use wal_attribution::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenReport {
    pub schema_version_before: u32,
    pub schema_version_after: u32,
    pub migration_steps: Vec<MigrationStepReport>,
    pub embedder_warmup_ms: u64,
    pub query_backend: &'static str,
    pub default_embedder: EmbedderIdentity,
    /// Total wall time the loader spent materializing default-embedder
    /// weights — covers HF GETs, sha256 verification, atomic rename,
    /// parent-dir fsync (POSIX), and cache directory writes. This is
    /// the "engine open paid by the embedder" envelope, useful for SLA
    /// budgeting; it is intentionally wider than just the bytes-flowing
    /// time so callers see the full first-use cost.
    ///
    /// `Some(ms)` when network bytes flowed (`bytes_downloaded > 0`);
    /// `None` for caller-supplied embedders (loader bypassed) and on
    /// full cache hits (no bytes flowed). For pure per-file network
    /// analysis, use the `DefaultEmbedderDownload` events on
    /// [`embedder_events`](Self::embedder_events) — each event carries
    /// the file's bytes + sha256 + cache path.
    pub embedder_download_ms: Option<u64>,
    /// Structured loader events (`dev/design/embedder.md` §7). Empty for
    /// caller-supplied embedders; populated from `LoadedWeights.events`
    /// for the Default path.
    pub embedder_events: Vec<EmbedderEvent>,
    /// Static identity capability (`dev/design/embedder.md` §0.6). True
    /// iff the live embedder identity is the bge-small default, which is
    /// the only identity that ships with the EU-5a2 mean-centering apply
    /// paths. `false` for `fathomdb-noop` and for any other
    /// caller-supplied identity. EU-5b's identity flip makes the Default
    /// path return `true` here.
    pub embedder_mean_centering_required: bool,
    /// Dynamic workspace state (`dev/design/embedder.md` §0.6). True iff
    /// `_fathomdb_embedder_profiles.mean_vec IS NOT NULL` for the default
    /// profile. EU-5a2 reads from the schema column added in migration
    /// step 10; the value is dimension-validated (§0.2) at open time
    /// and fails closed via `EmbedderIdentityMismatch` on drift.
    pub embedder_mean_vec_pinned: bool,
    /// 0.8.18 Slice 5 (#5 vector-equivalence probe, R-VEQ-6) — degraded-open
    /// observability. `true` iff the open-time #5 self-check re-embedded the 45
    /// committed probes and found a divergence beyond the frozen D4 floor (a
    /// Phase-1 mean-centered `embedding_bin` sign flip OR a Phase-2 un-centered
    /// L2 over `VECTOR_EQUIVALENCE_L2_EPSILON`). When `true`, `Engine::open`
    /// SUCCEEDED but every vector-dependent arm refuses at query time with
    /// `EngineError::VectorEquivalenceMismatch`; the text-only/FTS-only path stays
    /// serviceable. The state is RE-DERIVED at every open (the probe re-runs), so
    /// a reopen with a still-divergent backend stays degraded (never silently
    /// re-enables dense) and a reopen with a matching backend clears it.
    pub dense_disabled: bool,
    /// R-VEQ-6 — human-readable reason for `dense_disabled` (which representation
    /// tripped: P1 flip count or P2 L2). `None` when `dense_disabled == false`.
    pub dense_disabled_reason: Option<String>,
    /// Strict CPU/CUDA policy resolution used to construct the default
    /// embedder. `None` when the caller supplied an embedder or selected none.
    /// A present report is the one selection passed into default-embedder
    /// construction; forced CUDA failures return
    /// [`EngineOpenError::EmbedDevicePolicy`] rather than report CPU.
    pub embedder_device_resolution: Option<DeviceResolution>,
    /// Independent CPU/CUDA selection for the optional cross-encoder. This is
    /// never inferred from embedding-device state and makes no claim about
    /// database candidate retrieval or scoring.
    pub reranker_device_resolution: Option<RerankerDeviceResolution>,
    /// 0.8.23 Slice 80.6 (D-80.6-6, AC80-6, R80-13) — the in-process GPU
    /// allocation witness, when one was measured during this open.
    ///
    /// This carries the *retained record* of `fathomdb-embedder`'s
    /// `fathomdb.tegra-gpu-allocation-witness/v1`: the ordinal Candle actually
    /// retained, the driver-API UUID, and every raw number the verdict used
    /// (`free_before_bytes`, `free_after_bytes`, `total_bytes`, `delta_bytes`,
    /// `delta_floor_bytes`, plus the deliberate control allocation), so a
    /// reader re-derives the verdict rather than trusting it (R80-13). Its
    /// point is that the *installed artifact's own process* holds the
    /// evidence, rather than a sibling Rust process — which is what makes
    /// AC80-6's "in-process" clause as strong on Tegra as on x86_64.
    ///
    /// `None` is the normal case and means **no witness was measured** — never
    /// "a witness measured nothing". A zero, negative, or below-floor delta is
    /// a typed failure inside the witness (R80-12) and fails the open, so a
    /// zero-valued record is not reachable through this field.
    ///
    /// Populated only by an opted-in default-embedder open
    /// ([`ENV_GPU_ALLOCATION_WITNESS`]) on a CUDA-capable artifact whose
    /// device policy actually selected CUDA. It is deliberately opt-in: the
    /// witness holds a multi-gigabyte deliberate control allocation and loads
    /// the model a second time, which is evidence-run behavior and must not be
    /// imposed on ordinary opens (§ 12 non-goals).
    pub embedder_gpu_allocation_witness: Option<GpuAllocationWitness>,
}

#[derive(Debug)]
pub struct OpenedEngine {
    pub engine: Engine,
    pub report: OpenReport,
}

impl OpenedEngine {
    /// Return the process-wide SQLite configuration effective for this open.
    pub fn runtime_configuration(&self) -> RuntimeConfiguration {
        effective_runtime_configuration()
    }
}

/// SQLite runtime mode selected before FathomDB opens its first connection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeSqliteMode {
    /// Disable SQLite's process-global memory statistics and heap-limit
    /// enforcement to remove their shared allocator lock from read traffic.
    Performance,
    /// Enable SQLite's process-global memory statistics and heap-limit
    /// enforcement for diagnostic applications.
    Diagnostics,
}

/// Effective process-wide SQLite runtime configuration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeConfiguration {
    pub sqlite_mode: RuntimeSqliteMode,
}

/// Startup configuration failure. Changing modes requires a process restart.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeConfigurationError {
    /// SQLite was already initialized before FathomDB could configure it.
    TooLate,
    /// The runtime is already configured in a different mode.
    Conflict { requested: RuntimeSqliteMode, effective: RuntimeSqliteMode },
    /// SQLite rejected configuration or initialization with this result code.
    SqliteFailure { code: i32 },
}

impl Display for RuntimeConfigurationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooLate => write!(f, "SQLite runtime configuration is too late; restart required"),
            Self::Conflict { requested, effective } => write!(
                f,
                "SQLite runtime is already configured as {effective:?}, not {requested:?}; restart required"
            ),
            Self::SqliteFailure { code } => {
                write!(f, "SQLite runtime configuration failed with code {code}")
            }
        }
    }
}

impl Error for RuntimeConfigurationError {}

#[derive(Clone, Copy, Debug)]
enum RuntimeState {
    Unconfigured,
    Configured(RuntimeConfiguration),
    Failed(RuntimeConfigurationError),
}

static SQLITE_RUNTIME_STATE: Mutex<RuntimeState> = Mutex::new(RuntimeState::Unconfigured);

/// Configure the SQLite runtime before opening any FathomDB Engine.
///
/// Repeating the same mode is idempotent. Selecting a different mode or
/// configuring after any SQLite initialization fails without shutdown or
/// reconfiguration. The setting applies to the SQLite image linked into this
/// artifact and persists for the process lifetime.
pub fn configure_runtime(
    sqlite_mode: RuntimeSqliteMode,
) -> Result<RuntimeConfiguration, RuntimeConfigurationError> {
    configure_runtime_locked(Some(sqlite_mode))
}

fn configure_runtime_for_open() -> Result<RuntimeConfiguration, RuntimeConfigurationError> {
    configure_runtime_locked(None)
}

#[cfg(test)]
struct AdmissionLockedHookForTest {
    path: PathBuf,
    rendezvous: Arc<Barrier>,
}

#[cfg(test)]
static ADMISSION_LOCKED_HOOK_FOR_TEST: Mutex<Option<AdmissionLockedHookForTest>> = Mutex::new(None);

#[cfg(test)]
fn install_admission_locked_hook_for_test(path: PathBuf, rendezvous: Arc<Barrier>) {
    *ADMISSION_LOCKED_HOOK_FOR_TEST.lock().expect("admission hook lock") =
        Some(AdmissionLockedHookForTest { path, rendezvous });
}

#[cfg(test)]
fn run_admission_locked_hook_for_test(path: &Path) {
    let rendezvous = {
        let mut hook = ADMISSION_LOCKED_HOOK_FOR_TEST.lock().expect("admission hook lock");
        match hook.as_ref() {
            Some(candidate) if candidate.path == path => {
                hook.take().map(|candidate| candidate.rendezvous)
            }
            _ => None,
        }
    };
    if let Some(rendezvous) = rendezvous {
        rendezvous.wait();
        rendezvous.wait();
    }
}

#[cfg(feature = "operator")]
fn data_plane_inspection_error(
    reason: DataPlaneIntegrityErrorReasonV1,
    field_path: &'static str,
) -> EngineError {
    DataPlaneIntegrityErrorV1::new(reason, field_path).into()
}

#[cfg(feature = "operator")]
fn data_plane_sidecar_path(path: &Path, suffix: &str) -> PathBuf {
    let mut sidecar = path.as_os_str().to_os_string();
    sidecar.push(suffix);
    PathBuf::from(sidecar)
}

#[cfg(feature = "operator")]
fn immutable_sqlite_uri(path: &Path) -> String {
    sqlite_uri(path, "immutable=1")
}

/// Recover a current, quiescent database by asking SQLite to truncate its WAL.
///
/// Unlike [`Engine::open`], this operator-only path may proceed when the WAL's
/// fixed header is malformed. The main database is first validated through an
/// immutable, read-only connection, and a non-empty rollback journal is always
/// refused. The canonical product lock is held across validation and the
/// checkpoint, so a live FathomDB process cannot race the recovery. SQLite owns
/// the destructive WAL checkpoint/discard. A healthy-WAL preflight snapshots
/// the transient SHM sidecar and restores it if validation refuses recovery.
///
/// `discarded_corrupt_wal` is true only when the locked pre-probe classified
/// the WAL header as malformed and SQLite subsequently reported a completed
/// truncate checkpoint. A busy checkpoint remains a successful typed report
/// with [`TruncateWalStatus::Busy`] and never claims that corrupt data was
/// discarded.
///
/// # Errors
///
/// Returns [`EngineOpenError`] when the database is missing, empty, locked,
/// corrupt, not at the current schema version, accompanied by a non-empty
/// rollback journal, or inaccessible to SQLite.
#[cfg(feature = "operator")]
pub fn recover_truncate_wal(
    path: impl Into<PathBuf>,
) -> Result<TruncateWalReport, EngineOpenError> {
    let requested_path = path.into();
    let canonical_path = canonical_database_path(&requested_path)?;
    validate_recovery_database_file(&canonical_path)?;

    let pending_lock = acquire_lock_without_metadata_mutation(&canonical_path)?;
    let _lock = pending_lock.initialize()?;

    // Recheck every admission fact after the lock is held. The first check
    // prevents bootstrap; this one closes the rename/truncate race.
    validate_recovery_database_file(&canonical_path)?;
    match std::fs::metadata(data_plane_sidecar_path(&canonical_path, "-journal")) {
        Ok(metadata) if metadata.len() > 0 => {
            return Err(EngineOpenError::Io {
                message: "non-empty rollback journal blocks WAL recovery".to_string(),
            })
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => {
            return Err(EngineOpenError::Io {
                message: "database rollback journal is not accessible".to_string(),
            })
        }
    }

    configure_runtime_for_open().map_err(EngineOpenError::RuntimeConfiguration)?;
    register_sqlite_vec_extension();

    let validation = Connection::open_with_flags(
        immutable_sqlite_uri(&canonical_path),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
            | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX
            | rusqlite::OpenFlags::SQLITE_OPEN_URI,
    )
    .map_err(|error| map_open_sqlite_error(error, OpenStage::HeaderProbe))?;
    validation
        .pragma_update(None, "query_only", "ON")
        .map_err(|error| map_open_sqlite_error(error, OpenStage::SchemaProbe))?;
    probe_database_header(&validation)?;
    probe_open_integrity(&validation)?;
    reject_legacy_shape(&validation)?;
    let main_file_schema_version = validation
        .pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))
        .map_err(|error| map_open_sqlite_error(error, OpenStage::SchemaProbe))?;

    let wal_header = classify_wal_sidecar(&canonical_path)?;
    let malformed_wal = matches!(wal_header, WalSidecarHeader::Malformed { .. });
    // A malformed WAL cannot contribute trustworthy schema state. Require the
    // standalone main file itself to be current before asking SQLite to discard
    // that WAL. A healthy WAL may legitimately carry the current schema cookie
    // while the main file still reports an older value, so its effective version
    // is checked through SQLite below.
    if malformed_wal && main_file_schema_version != SCHEMA_VERSION {
        return Err(EngineOpenError::IncompatibleSchemaVersion {
            seen: main_file_schema_version,
            supported: SCHEMA_VERSION,
        });
    }
    if main_file_schema_version == SCHEMA_VERSION {
        validate_recovery_schema_invariants(&validation, main_file_schema_version)?;
    }
    drop(validation);
    if !malformed_wal {
        validate_effective_recovery_schema(&canonical_path)?;
    }

    // No read/write SQLite connection is opened until every refusal condition
    // has passed. In particular, dropping a read/write connection after a
    // noncurrent effective-schema check could checkpoint a healthy WAL while
    // reporting refusal.
    let connection = Connection::open_with_flags(
        sqlite_uri(&canonical_path, "mode=rw"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
            | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX
            | rusqlite::OpenFlags::SQLITE_OPEN_URI,
    )
    .map_err(|error| map_open_sqlite_error(error, OpenStage::WalReplay))?;
    connection
        .busy_timeout(Duration::ZERO)
        .map_err(|error| map_open_sqlite_error(error, OpenStage::WalReplay))?;
    let (busy, log_frames, checkpointed_frames): (i64, i64, i64) = connection
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .map_err(|error| map_open_sqlite_error(error, OpenStage::WalReplay))?;
    let status = if busy == 0 { TruncateWalStatus::Done } else { TruncateWalStatus::Busy };

    Ok(TruncateWalReport {
        status,
        busy: busy.max(0) as u32,
        log_frames: log_frames.max(0) as u32,
        checkpointed_frames: checkpointed_frames.max(0) as u32,
        discarded_corrupt_wal: malformed_wal && status == TruncateWalStatus::Done,
    })
}

#[cfg(feature = "operator")]
fn validate_recovery_database_file(path: &Path) -> Result<(), EngineOpenError> {
    let metadata = std::fs::metadata(path).map_err(|_| EngineOpenError::Io {
        message: "recovery requires an existing database file".to_string(),
    })?;
    if !metadata.is_file() || metadata.len() == 0 {
        return Err(EngineOpenError::Io {
            message: "recovery requires a non-empty regular database file".to_string(),
        });
    }
    Ok(())
}

#[cfg(feature = "operator")]
fn validate_effective_recovery_schema(path: &Path) -> Result<(), EngineOpenError> {
    let shm = ShmSnapshot::capture(path)?;
    let result = (|| {
        let connection = Connection::open_with_flags(
            read_only_sqlite_uri(path),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
                | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX
                | rusqlite::OpenFlags::SQLITE_OPEN_URI,
        )
        .map_err(|error| map_open_sqlite_error(error, OpenStage::WalReplay))?;
        connection
            .pragma_update(None, "query_only", "ON")
            .map_err(|error| map_open_sqlite_error(error, OpenStage::SchemaProbe))?;
        probe_database_header(&connection)?;
        probe_open_integrity(&connection)?;
        reject_legacy_shape(&connection)?;
        let seen = connection
            .pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))
            .map_err(|error| map_open_sqlite_error(error, OpenStage::SchemaProbe))?;
        if seen != SCHEMA_VERSION {
            return Err(EngineOpenError::IncompatibleSchemaVersion {
                seen,
                supported: SCHEMA_VERSION,
            });
        }
        validate_recovery_schema_invariants(&connection, seen)
    })();
    match result {
        Ok(()) => Ok(()),
        Err(error) => {
            shm.restore()?;
            Err(error)
        }
    }
}

#[cfg(feature = "operator")]
fn validate_recovery_schema_invariants(
    connection: &Connection,
    schema_version: u32,
) -> Result<(), EngineOpenError> {
    validate_dependency_generation_on_open(connection, schema_version)?;
    frozen_read::validate_on_open(connection, schema_version)
        .map_err(|_| recovery_schema_corruption("_fathomdb_read_visibility_state"))?;
    dependency_closure::validate_closure_state_on_open(connection, schema_version)?;
    Ok(())
}

#[cfg(feature = "operator")]
fn recovery_schema_corruption(table: &'static str) -> EngineOpenError {
    EngineOpenError::Corruption(CorruptionDetail {
        kind: CorruptionKind::SchemaInconsistent,
        stage: OpenStage::SchemaProbe,
        locator: CorruptionLocator::TableRow { table, rowid: 0 },
        recovery_hint: RecoveryHint {
            code: "E_CORRUPT_SCHEMA",
            doc_anchor: "design/recovery.md#schema-inconsistent",
        },
    })
}

fn read_only_sqlite_uri(path: &Path) -> String {
    sqlite_uri(path, "mode=ro")
}

fn sqlite_uri(path: &Path, query: &str) -> String {
    let mut uri = String::from("file:");
    for byte in path.as_os_str().as_encoded_bytes() {
        match *byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => {
                uri.push(char::from(*byte))
            }
            byte => {
                const HEX: &[u8; 16] = b"0123456789ABCDEF";
                uri.push('%');
                uri.push(char::from(HEX[usize::from(byte >> 4)]));
                uri.push(char::from(HEX[usize::from(byte & 0x0f)]));
            }
        }
    }
    uri.push('?');
    uri.push_str(query);
    uri
}

/// Inspect a quiescent database through a strictly read-only operator boundary.
///
/// The request is validated before filesystem access. The database and its
/// pre-existing lock file must both exist, the lock must be exclusively
/// acquirable without modifying it, and non-empty WAL or rollback-journal
/// sidecars are refused. SQLite is then opened read-only with `query_only`
/// enabled; migrations, projection reconciliation, worker startup, and lock
/// metadata writes are never performed.
///
/// FathomDB writers are excluded by the product lock. Callers must also stop
/// raw external SQLite writers, which do not participate in that lock protocol,
/// before invoking this function. The connection uses a percent-encoded
/// `file:` URI with SQLite `immutable=1`, `READ_ONLY|URI`, and `query_only`.
///
/// # Errors
///
/// Returns a typed [`DataPlaneIntegrityErrorV1`] through [`EngineError`] for
/// invalid requests, unavailable or non-quiescent inputs, runtime setup,
/// incompatible schema versions, corruption, or bounded inspection failures.
#[cfg(feature = "operator")]
pub fn inspect_data_plane_integrity(
    path: impl Into<PathBuf>,
    request: DataPlaneIntegrityRequestV1,
) -> Result<DataPlaneIntegrityResultV1, EngineError> {
    data_plane_integrity::validate_request(&request)?;

    let requested_path = path.into();
    let unresolved_path = canonical_database_path(&requested_path).map_err(|_| {
        data_plane_inspection_error(
            DataPlaneIntegrityErrorReasonV1::InspectionUnavailable,
            "/dbPath",
        )
    })?;
    let canonical_path = unresolved_path.canonicalize().map_err(|_| {
        data_plane_inspection_error(
            DataPlaneIntegrityErrorReasonV1::InspectionUnavailable,
            "/dbPath",
        )
    })?;
    if !canonical_path.is_file() {
        return Err(data_plane_inspection_error(
            DataPlaneIntegrityErrorReasonV1::InspectionUnavailable,
            "/dbPath",
        ));
    }

    let inspection_lock_path = lock_path(&canonical_path);
    if !inspection_lock_path.is_file() {
        return Err(data_plane_inspection_error(
            DataPlaneIntegrityErrorReasonV1::InspectionLockMissing,
            "/dbPath",
        ));
    }
    let inspection_lock =
        OpenOptions::new().read(true).open(&inspection_lock_path).map_err(|_| {
            data_plane_inspection_error(
                DataPlaneIntegrityErrorReasonV1::InspectionUnavailable,
                "/dbPath",
            )
        })?;
    match inspection_lock.try_lock() {
        Ok(()) => {}
        Err(std::fs::TryLockError::WouldBlock) => {
            return Err(data_plane_inspection_error(
                DataPlaneIntegrityErrorReasonV1::InspectionNotQuiescent,
                "/dbPath",
            ));
        }
        Err(_) => {
            return Err(data_plane_inspection_error(
                DataPlaneIntegrityErrorReasonV1::InspectionUnavailable,
                "/dbPath",
            ));
        }
    }

    for suffix in ["-wal", "-journal"] {
        match std::fs::metadata(data_plane_sidecar_path(&canonical_path, suffix)) {
            Ok(metadata) if metadata.len() > 0 => {
                return Err(data_plane_inspection_error(
                    DataPlaneIntegrityErrorReasonV1::InspectionNotQuiescent,
                    "/dbPath",
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => {
                return Err(data_plane_inspection_error(
                    DataPlaneIntegrityErrorReasonV1::InspectionUnavailable,
                    "/dbPath",
                ));
            }
        }
    }

    configure_runtime_for_open().map_err(|_| {
        data_plane_inspection_error(
            DataPlaneIntegrityErrorReasonV1::RuntimeConfiguration,
            "/runtimeConfiguration",
        )
    })?;
    register_sqlite_vec_extension();
    let mut connection = Connection::open_with_flags(
        immutable_sqlite_uri(&canonical_path),
        OpenFlags::SQLITE_OPEN_READ_ONLY
            | OpenFlags::SQLITE_OPEN_NO_MUTEX
            | OpenFlags::SQLITE_OPEN_URI,
    )
    .map_err(|_| {
        data_plane_inspection_error(
            DataPlaneIntegrityErrorReasonV1::InspectionUnavailable,
            "/dbPath",
        )
    })?;
    connection.pragma_update(None, "query_only", "ON").map_err(|_| {
        data_plane_inspection_error(DataPlaneIntegrityErrorReasonV1::IntegrityCorrupt, "")
    })?;
    let database_schema_version =
        connection.pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0)).map_err(
            |_| data_plane_inspection_error(DataPlaneIntegrityErrorReasonV1::IntegrityCorrupt, ""),
        )?;
    if database_schema_version != i64::from(SCHEMA_VERSION) {
        return Err(data_plane_inspection_error(
            DataPlaneIntegrityErrorReasonV1::DatabaseSchemaMismatch,
            "/databaseSchemaVersion",
        ));
    }

    data_plane_integrity::execute(&mut connection, request).map_err(|error| match error {
        EngineError::DataPlaneIntegrity(_) => error,
        _ => data_plane_inspection_error(DataPlaneIntegrityErrorReasonV1::IntegrityCorrupt, ""),
    })
}

fn configure_runtime_locked(
    requested: Option<RuntimeSqliteMode>,
) -> Result<RuntimeConfiguration, RuntimeConfigurationError> {
    let mut state = SQLITE_RUNTIME_STATE.lock().map_err(|_| {
        RuntimeConfigurationError::SqliteFailure { code: rusqlite::ffi::SQLITE_ERROR }
    })?;
    match *state {
        RuntimeState::Configured(effective) => {
            if requested.is_none_or(|mode| mode == effective.sqlite_mode) {
                return Ok(effective);
            }
            return Err(RuntimeConfigurationError::Conflict {
                requested: requested.expect("checked Some"),
                effective: effective.sqlite_mode,
            });
        }
        RuntimeState::Failed(error) => return Err(error),
        RuntimeState::Unconfigured => {}
    }

    let mode = requested.unwrap_or(RuntimeSqliteMode::Performance);
    let memstatus = match mode {
        RuntimeSqliteMode::Performance => 0_i32,
        RuntimeSqliteMode::Diagnostics => 1_i32,
    };
    let config_rc =
        unsafe { rusqlite::ffi::sqlite3_config(rusqlite::ffi::SQLITE_CONFIG_MEMSTATUS, memstatus) };
    if config_rc != rusqlite::ffi::SQLITE_OK {
        let error = if config_rc == rusqlite::ffi::SQLITE_MISUSE {
            RuntimeConfigurationError::TooLate
        } else {
            RuntimeConfigurationError::SqliteFailure { code: config_rc }
        };
        *state = RuntimeState::Failed(error);
        return Err(error);
    }
    let initialize_rc = unsafe { rusqlite::ffi::sqlite3_initialize() };
    if initialize_rc != rusqlite::ffi::SQLITE_OK {
        let error = RuntimeConfigurationError::SqliteFailure { code: initialize_rc };
        *state = RuntimeState::Failed(error);
        return Err(error);
    }
    let effective = RuntimeConfiguration { sqlite_mode: mode };
    *state = RuntimeState::Configured(effective);
    Ok(effective)
}

fn effective_runtime_configuration() -> RuntimeConfiguration {
    match *SQLITE_RUNTIME_STATE.lock().expect("SQLite runtime state") {
        RuntimeState::Configured(configuration) => configuration,
        RuntimeState::Unconfigured | RuntimeState::Failed(_) => {
            unreachable!("an opened Engine always has a configured SQLite runtime")
        }
    }
}

/// EU-5b — loader-supplied open-time telemetry threaded into
/// `OpenReport.embedder_download_ms` and `OpenReport.embedder_events`.
#[derive(Clone, Debug)]
struct LoaderInfo {
    download_ms: Option<u64>,
    events: Vec<EmbedderEvent>,
    device_resolution: DeviceResolution,
    /// 0.8.23 Slice 80.6 (D-80.6-6) — the opted-in in-process GPU allocation
    /// witness. `None` for every path that measured none.
    gpu_allocation_witness: Option<GpuAllocationWitness>,
}

/// 0.8.23 Slice 80.6 (D-80.6-6) — opt-in switch for the in-process GPU
/// allocation witness carried on [`OpenReport::embedder_gpu_allocation_witness`].
///
/// Opt-in rather than automatic, and deliberately so. Producing the witness
/// costs a second load of the pinned model plus the multi-gigabyte deliberate
/// control allocation D-80.5-3 requires in order to prove the shared iGPU
/// memory counter is live and attributable. That is evidence-run behavior;
/// imposing it on every CUDA open would be exactly the runtime-contract change
/// § 12 of `dev/design/0.8.23-aarch64-tegra.md` rules out.
///
/// `1`/`true` enable it; unset, empty, `0`/`false` disable it. Any other value
/// is **rejected at open time** rather than read as "off", so a typo cannot
/// silently turn the evidence off — the same fail-closed posture R80-12 puts
/// on the witness itself.
pub const ENV_GPU_ALLOCATION_WITNESS: &str = "FATHOMDB_GPU_ALLOCATION_WITNESS";

/// Parse [`ENV_GPU_ALLOCATION_WITNESS`]. Pure, so every arm is testable on a
/// host with no GPU and on a build with no CUDA.
#[cfg(any(feature = "default-embedder", test))]
fn parse_gpu_allocation_witness_opt_in(raw: Option<&str>) -> Result<bool, String> {
    match raw.map(str::trim) {
        None | Some("") => Ok(false),
        Some(value) => match value.to_ascii_lowercase().as_str() {
            "1" | "true" => Ok(true),
            "0" | "false" => Ok(false),
            other => Err(format!(
                "{ENV_GPU_ALLOCATION_WITNESS} must be 1/true or 0/false, got {other:?}"
            )),
        },
    }
}

/// Measure the in-process GPU allocation witness when the operator asked for
/// one, and only then.
///
/// The contract is deliberately binary: opted in means this open carries a
/// witness or it fails, and not opted in means `None`. There is no third
/// outcome where the field is `None` while the operator believes a witness was
/// taken, because that is how a missing measurement becomes indistinguishable
/// from a measurement of zero (R80-12).
#[cfg(feature = "default-embedder")]
fn witness_gpu_allocation_if_requested(
    device_resolution: &DeviceResolution,
) -> Result<Option<GpuAllocationWitness>, EngineOpenError> {
    let raw = std::env::var(ENV_GPU_ALLOCATION_WITNESS).ok();
    let requested = parse_gpu_allocation_witness_opt_in(raw.as_deref())
        .map_err(|message| EngineOpenError::Embedder(RuntimeEmbedderError::Failed { message }))?;
    if !requested {
        return Ok(None);
    }
    run_requested_gpu_allocation_witness(device_resolution).map(Some)
}

/// Name a refusal rather than degrading to `None`, carrying the witness's own
/// stable failure tag so the caller reads the same vocabulary the retained
/// record uses.
#[cfg(feature = "default-embedder")]
fn gpu_allocation_witness_refusal(tag: &str, detail: &str) -> EngineOpenError {
    EngineOpenError::Embedder(RuntimeEmbedderError::Failed {
        message: format!(
            "{ENV_GPU_ALLOCATION_WITNESS} was requested but no GPU allocation witness \
             could be produced ({tag}): {detail}"
        ),
    })
}

#[cfg(all(feature = "default-embedder", feature = "embed-cuda"))]
fn run_requested_gpu_allocation_witness(
    device_resolution: &DeviceResolution,
) -> Result<GpuAllocationWitness, EngineOpenError> {
    use fathomdb_embedder::{AllocationWitnessConfig, EffectiveEmbedDevice};

    let ordinal = match &device_resolution.effective_device {
        EffectiveEmbedDevice::Cuda(info) => info.ordinal,
        EffectiveEmbedDevice::Cpu => {
            return Err(gpu_allocation_witness_refusal(
                "cpu_fallback",
                "the embedder device policy resolved to CPU, so there is no GPU \
                 allocation to witness",
            ));
        }
    };
    fathomdb_embedder::run_default_embedder_allocation_witness(AllocationWitnessConfig {
        ordinal,
        ..AllocationWitnessConfig::default()
    })
    .map_err(|error| gpu_allocation_witness_refusal(error.as_str(), &error.to_string()))
}

#[cfg(all(feature = "default-embedder", not(feature = "embed-cuda")))]
fn run_requested_gpu_allocation_witness(
    _device_resolution: &DeviceResolution,
) -> Result<GpuAllocationWitness, EngineOpenError> {
    Err(gpu_allocation_witness_refusal(
        "cuda_not_compiled",
        "this artifact has no CUDA provider compiled in",
    ))
}

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

/// Test-only Slice 45 attribution for authenticated frozen-page setup.
#[cfg(feature = "test-hooks")]
#[derive(Clone, Copy, Debug)]
#[doc(hidden)]
pub struct Slice45FrozenStageTiming {
    pub cursor_authentication_ns: u128,
    pub token_authentication_ns: u128,
    pub snapshot_binding_ns: u128,
}

/// Test-only bounded trace measurement captured around the trace call itself.
#[cfg(feature = "test-hooks")]
#[derive(Clone, Debug)]
#[doc(hidden)]
pub struct DependencyTraceMeasurement {
    pub vm_steps: u64,
    pub elapsed: Duration,
    pub peak_rss_delta_bytes: u64,
    pub response_bytes: Vec<u8>,
    pub bound_exceeded: bool,
}

/// Test-only Slice 45 attribution for frozen-context minting.
#[cfg(feature = "test-hooks")]
#[derive(Clone, Copy, Debug)]
#[doc(hidden)]
pub struct Slice45MintStageTiming {
    pub context_validation_ns: u128,
    pub snapshot_validation_ns: u128,
    pub binding_ns: u128,
    pub token_codec_ns: u128,
}

/// EXP-S (0.8.14 Slice 5, D1) — structural-role tag for a canonical row.
///
/// A SEPARATE axis from the doc-type `kind` (email/article/paper/meeting/
/// note/todo/doc/edge_fact): `row_kind` describes *what structural role* a row
/// plays in the "one store, many indexes" substrate, not what document type it
/// carries. Stored in `canonical_nodes.row_kind` (schema migration step 16).
///
/// `Leaf` is the default (a normal record; every existing/normal write is a
/// leaf — back-compat preserving). `Coverage` = coverage/summary rows;
/// `Graph` = graph structural rows. Engine-internal in 0.8.14 — there is NO
/// public Py/TS SDK surface for `row_kind` this release (`Leaf` for all normal
/// writes; `Coverage`/`Graph` are set only by internal paths). Cross-binding
/// parity (X1) is a Slice-40 concern.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RowKind {
    Leaf,
    Coverage,
    Graph,
}

impl RowKind {
    /// On-disk `canonical_nodes.row_kind` spelling. Must match the migration
    /// step-16 `DEFAULT 'leaf'` and the schema vocabulary (D1).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            RowKind::Leaf => "leaf",
            RowKind::Coverage => "coverage",
            RowKind::Graph => "graph",
        }
    }
}

/// OPP-12 Phase-1 (0.8.19 Slice 10) — whether `(from, to)` is one of the four
/// legal `transition`-verb moves (design §2 table): `pending→active` (promote),
/// `pending→deleted` (reject), `active→deleted` (soft-delete), `deleted→active`
/// (undelete). Every other pair — self-loops, any move to `Purged` (purge-only)
/// or `Pending` (create-only), or from `Purged` — is illegal via `transition`.
#[must_use]
fn is_legal_transition_move(from: LifecycleState, to: LifecycleState) -> bool {
    matches!(
        (from, to),
        (LifecycleState::Pending, LifecycleState::Active)
            | (LifecycleState::Pending, LifecycleState::Deleted)
            | (LifecycleState::Active, LifecycleState::Deleted)
            | (LifecycleState::Deleted, LifecycleState::Active)
    )
}

/// Snapshot of engine-internal counters returned by [`Engine::counters`].
///
/// Public key set is owned by `dev/design/lifecycle.md` § Public key set
/// and locked by AC-004a. Reading a snapshot is non-perturbing per
/// AC-004c. The 0.6.0 surface exposes exactly these seven fields.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CounterSnapshot {
    pub queries: u64,
    pub writes: u64,
    pub write_rows: u64,
    pub errors_by_code: BTreeMap<String, u64>,
    pub admin_ops: u64,
    pub cache_hit: u64,
    pub cache_miss: u64,
}

pub use lifecycle::Subscription;

/// Caller-facing selector for the embedder used by an opened engine
/// (`dev/design/embedder.md` §0).
#[derive(Clone)]
pub enum EmbedderChoice {
    /// Use the engine's default embedder. With the `default-embedder`
    /// Cargo feature enabled, this materializes a `CandleBgeEmbedder`
    /// via the EU-3 loader at `Engine::open`; on first use the loader
    /// downloads pinned bge-small-en-v1.5 weights from HuggingFace per
    /// `ADR-0.7.1-default-embedder-weight-fetch`. Without the feature,
    /// this returns `EmbedderError::Failed` directing the caller to
    /// rebuild with `--features default-embedder` or supply
    /// `EmbedderChoice::Caller`.
    Default,
    /// Caller supplies the embedder instance. The supplied embedder's
    /// `identity()` becomes the workspace's default-profile identity.
    Caller(Arc<dyn Embedder>),
    /// Caller supplies an embedder plus its already-resolved device outcome.
    ///
    /// The resolution is recorded in [`OpenReport::embedder_device_resolution`]
    /// exactly once. This is for opt-in embedders, such as ONNX Runtime, whose
    /// final CUDA/CPU outcome is known only after their own construction.
    CallerWithDeviceResolution {
        /// The caller-supplied runtime embedder.
        embedder: Arc<dyn Embedder>,
        /// The embedder's final CPU/CUDA resolution.
        device_resolution: DeviceResolution,
    },
    /// No embedder configured. Engine opens; subsequent vector writes
    /// fail with `EngineError::EmbedderNotConfigured`. Useful for
    /// read-only or canonical-only flows.
    None,
}

/// Doctor `check-integrity` invocation flags. `quick` and `round_trip`
/// are accepted in 0.6.0 but treated as default; only `full` activates
/// `PRAGMA integrity_check`. Per `dev/design/recovery.md` § Doctor-only
/// flags.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CheckIntegrityOpts {
    pub quick: bool,
    pub full: bool,
    pub round_trip: bool,
}

/// One section of an [`IntegrityReport`]. Either every check in the
/// section was clean, or one or more typed [`Finding`]s describe the
/// detected issue. Per AC-043b.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Section {
    Clean,
    Findings(Vec<Finding>),
}

/// Single doctor finding record. Stable report-shape per AC-043c. The
/// `code` and `doc_anchor` strings are stable dispatch keys owned by
/// `dev/design/recovery.md` § Code-to-operator-action cross-reference.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Finding {
    pub code: &'static str,
    pub stage: &'static str,
    pub locator: CorruptionLocator,
    pub doc_anchor: &'static str,
    pub detail: String,
}

/// Three-section integrity report. AC-043a pins exactly these three
/// keys.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IntegrityReport {
    pub physical: Section,
    pub logical: Section,
    pub semantic: Section,
}

/// Result of a successful [`Engine::safe_export`] call. The returned
/// `manifest_sha256` equals the SHA-256 of the export file bytes (per
/// AC-039a) and matches the `sha256` field written into the manifest
/// JSON.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SafeExportArtifact {
    pub export_path: PathBuf,
    pub manifest_path: PathBuf,
    pub manifest_sha256: String,
}

/// Phase 9 Pack B trace report (AC-042). One event per canonical row
/// attributable to the requested `source_id`, ordered by `write_cursor`
/// ascending.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceReport {
    pub source_ref: String,
    pub events: Vec<TraceEvent>,
}

/// Single canonical-row tracing record. `table` is one of
/// `"canonical_nodes"` or `"canonical_edges"`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceEvent {
    pub write_cursor: u64,
    pub kind: String,
    pub table: &'static str,
}

/// Which shadow-state surface a [`RebuildReport`] describes.
/// `Projections` covers the full FTS5 + vec0 + projection-terminal
/// rebuild emitted by [`Engine::rebuild_projections`]. `Vec0` covers
/// the vec0-only path emitted by [`Engine::rebuild_vec0`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RebuildKind {
    Projections,
    Vec0,
}

/// Structured result of a rebuild operation. `rows_invalidated` is the
/// total shadow-state rows truncated before re-derivation; `rows_rebuilt`
/// is the count of rows the synchronous rebuild loop re-materialised
/// (asynchronous re-enqueue work performed by the projection scheduler is
/// not counted here). `projection_cursor_after` is the post-rebuild value
/// of the projection cursor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RebuildReport {
    pub kind: RebuildKind,
    pub rows_invalidated: u64,
    pub rows_rebuilt: u64,
    pub projection_cursor_after: u64,
}

/// 0.8.20 Slice 15d (R-20-PR, C-1) — one member of a [`ProjectionSpec`]'s role
/// set. **Exactly three members** (HITL-ratified S8, `api-surface.md:87`):
/// `searchable→FTS` and `searchable→vector` are NOT roles — they are tier labels
/// carried by the `fts`/`vector` sub-objects of the spec, so an attribute is
/// `Searchable` once and the sub-objects select FTS-only / vector-only / both.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ProjectionRole {
    /// Projects into the EAV store + its `(attr_name, attr_value)` composite
    /// index — cheap equality/range, built same-transaction.
    Filterable,
    /// The F9 importance/recency signal. **Graceful-absent (Q6a):** declaring
    /// it is legal and never errors, but the engine DEFERS the build until F9
    /// exists and grafts it on the next idempotent `configure_projections`.
    Rankable,
    /// Full-text / dense recall of the meaning text. The `fts`/`vector`
    /// sub-objects select the sub-target.
    Searchable,
}

impl ProjectionRole {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            ProjectionRole::Filterable => "filterable",
            ProjectionRole::Rankable => "rankable",
            ProjectionRole::Searchable => "searchable",
        }
    }

    #[must_use]
    pub fn from_str_opt(value: &str) -> Option<Self> {
        match value {
            "filterable" => Some(ProjectionRole::Filterable),
            "rankable" => Some(ProjectionRole::Rankable),
            "searchable" => Some(ProjectionRole::Searchable),
            _ => None,
        }
    }
}

/// 0.8.20 Slice 15d (R-20-PR) — the `searchable→FTS` sub-target selector.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProjectionFts {
    /// Optional tokenizer override; `None` ⇒ the engine default FTS5 tokenizer
    /// (`body`-FTS's `porter unicode61 remove_diacritics 2`). A custom
    /// per-attr tokenizer is the ≥0.9.x multi-field FTS work — recorded but
    /// not honoured here (graceful-graft later, same as `rankable`).
    pub tokenizer: Option<String>,
}

/// 0.8.20 Slice 20 (R-20-DR) — the ENGINE-SET readiness of the
/// `searchable→vector` projection, per
/// `dev/design/record-lifecycle-protocol/projection-registry-and-async-embed.md`
/// §3.
///
/// **Exactly three members.** `filterable` and `searchable→FTS` are
/// same-transaction (non-stale on commit) so they need no readiness axis at all;
/// `searchable→vector` is **async, rebuild-durable**, so it carries one.
///
/// **Naming discipline (load-bearing).** The token **`pending` is RESERVED for
/// the admission axis** (quarantine/trust — an app judgment). Index-readiness is
/// a DIFFERENT, orthogonal dimension (a record can be
/// `active ∧ is_latest ∧ admissible` yet `dense_readiness = embedding`), so this
/// enum deliberately does **not** reuse that word: the non-ready member is
/// `Embedding`, never `Pending`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum DenseReadiness {
    /// Engine-selected state for a session with no usable dense runtime (an
    /// absent embedder or refused vector equivalence). Caller input remains
    /// accept-inert; reads select this through the shared runtime predicate.
    Unavailable,
    /// At least one row in the vector projection's row set has not yet reached a
    /// projection terminal — embedding is outstanding. This is the ONLY
    /// tolerable torn state: readiness `embedding` with the vector absent (the
    /// dense arm reads as partial and RRF under-ranks; it does not hide).
    Embedding,
    /// Every row in the vector projection's row set has reached a projection
    /// terminal — the dense arm is caught up. Because the vector INSERT and the
    /// terminal record are written in ONE transaction
    /// ([`commit_projection_outcomes`]), `Ready` can never be observed with the
    /// vector row absent (design §4.1 invariant 1).
    Ready,
}

impl DenseReadiness {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            DenseReadiness::Unavailable => "unavailable",
            DenseReadiness::Embedding => "embedding",
            DenseReadiness::Ready => "ready",
        }
    }

    /// The three accepted spellings. `"pending"` is DELIBERATELY not one of them
    /// (reserved for the admission axis) and so parses to `None`.
    #[must_use]
    pub fn from_str_opt(value: &str) -> Option<Self> {
        match value {
            "unavailable" => Some(DenseReadiness::Unavailable),
            "embedding" => Some(DenseReadiness::Embedding),
            "ready" => Some(DenseReadiness::Ready),
            _ => None,
        }
    }
}

/// The reason [`ProjectionRuntimeStatus::runtime_embedder_available`] is false.
///
/// This facade is deliberately distinct from the internal lifecycle
/// `ProjectionStatus`: it describes this open engine session's ability to run
/// the shared dense pipeline, not the terminal state of a canonical row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectionRuntimeUnavailabilityReason {
    /// A usable dense runtime is attached, so there is no unavailability.
    None,
    /// This engine session was opened without an attached embedder.
    NoRuntime,
    /// The attached embedder failed the existing vector-equivalence guard.
    VectorEquivalenceDisabled,
}

impl ProjectionRuntimeUnavailabilityReason {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::NoRuntime => "no_runtime",
            Self::VectorEquivalenceDisabled => "vector_equivalence_disabled",
        }
    }
}

/// The dense-readiness projection of [`ProjectionRuntimeStatusEntry`].
///
/// `NotDeclared` means that the declaration has no *effective* vector arm.
/// The remaining states reuse the shared runtime/readiness facts, which are
/// corpus-wide until the engine gains per-projection dense work tracking.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectionStatusDenseReadiness {
    /// The declaration has no `searchable` + vector sub-object pair.
    NotDeclared,
    /// An effective vector arm exists but this session has no usable runtime.
    Unavailable,
    /// An effective vector arm has eligible outstanding shared dense work.
    Embedding,
    /// An effective vector arm's shared dense work is quiescent.
    Ready,
}

impl ProjectionStatusDenseReadiness {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotDeclared => "not_declared",
            Self::Unavailable => "unavailable",
            Self::Embedding => "embedding",
            Self::Ready => "ready",
        }
    }
}

/// One declaration's current dense status in [`ProjectionRuntimeStatus`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectionRuntimeStatusEntry {
    /// Declared projection name. Entries are returned in ascending name order.
    pub name: String,
    /// Current dense state for this declaration's effective vector arm.
    pub dense_readiness: ProjectionStatusDenseReadiness,
}

/// A pure, current view of projection-runtime facts for one open engine session.
///
/// `runtime_embedder_available` and its reason describe the dense runtime, not
/// whether any projection is declared. `projections` contains one entry per
/// durable declaration, sorted by name. `vector_unsupported_kinds` is current
/// and declaration-scoped: it is empty unless at least one declaration has an
/// effective (`searchable` + vector) arm.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectionRuntimeStatus {
    /// Whether an attached embedder passed the identity/equivalence safeguards.
    pub runtime_embedder_available: bool,
    /// `None` exactly when `runtime_embedder_available` is true.
    pub runtime_unavailability_reason: ProjectionRuntimeUnavailabilityReason,
    /// One sorted entry for every durable declaration.
    pub projections: Vec<ProjectionRuntimeStatusEntry>,
    /// Sorted, deduplicated permanently non-committable kinds for an effective arm.
    pub vector_unsupported_kinds: Vec<String>,
}

/// The lifecycle state of outstanding embedding work for this open session.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EmbeddingReadinessState {
    /// No eligible embedding work is outstanding.
    Ready,
    /// A usable embedder is processing eligible work.
    Processing,
    /// Work exists but the session is unavailable for a non-configuration reason.
    Deferred,
    /// Work exists and this session has no configured embedder.
    Blocked,
}

impl EmbeddingReadinessState {
    /// Stable lower-case wire spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Processing => "processing",
            Self::Deferred => "deferred",
            Self::Blocked => "blocked",
        }
    }
}

/// The projection operation that needs an embedder.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EmbeddingOperation {
    /// The body of a canonical graph edge projects as `edge_fact`.
    GraphEdgeBodyProjection,
    /// A caller-declared vector projection has outstanding work.
    VectorProjection,
}

impl EmbeddingOperation {
    /// Stable lower-case wire spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::GraphEdgeBodyProjection => "graph_edge_body_projection",
            Self::VectorProjection => "vector_projection",
        }
    }
}

/// A typed configuration outcome shared by drain errors and readiness reports.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmbedderRequired {
    /// Stable error code for all bindings.
    pub code: &'static str,
    /// The blocked embedding operation.
    pub operation: EmbeddingOperation,
    /// Stable state spelling, always `blocked` for this payload.
    pub state: EmbeddingReadinessState,
    /// Ordered, machine-readable corrective actions.
    pub remediations: Vec<&'static str>,
    /// Stable documentation address for this outcome.
    pub documentation_url: &'static str,
}

/// A Rust-owned, pure current report for embedding readiness.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmbeddingReadiness {
    /// `ready`, `processing`, `deferred`, or `blocked`.
    pub state: EmbeddingReadinessState,
    /// Whether this session can use its dense runtime now.
    pub usable_embedder: bool,
    /// Number of eligible rows awaiting embedding.
    pub pending_count: u64,
    /// Sorted projection kinds represented by the pending rows.
    pub affected_kinds: Vec<String>,
    /// Present exactly when `state` is `blocked`.
    pub blocked: Option<EmbedderRequired>,
}

fn embedder_required_for(affected_kinds: &[String]) -> EmbedderRequired {
    let operation = if affected_kinds.iter().any(|kind| kind == EDGE_FACT_KIND) {
        EmbeddingOperation::GraphEdgeBodyProjection
    } else {
        EmbeddingOperation::VectorProjection
    };
    EmbedderRequired {
        code: "FDB_EMBEDDER_REQUIRED",
        operation,
        state: EmbeddingReadinessState::Blocked,
        remediations: vec![
            "configure_default_embedder",
            "configure_caller_embedder",
            "submit_non_embedding_input",
        ],
        documentation_url: "https://fathomdb.dev/errors/FDB_EMBEDDER_REQUIRED",
    }
}

/// 0.8.20 Slice 15d (R-20-PR) — the `searchable→vector` sub-target selector.
///
/// **Slice 20 (R-20-DR) attached `dense_readiness` HERE, additively:** this
/// sub-object is STORED by 15d (so the shape exists and a caller can declare a
/// vector projection); Slice 20 hangs the READ-METADATA readiness flag off it.
/// Nothing in 15d's persisted shape changed (the registry columns
/// `vector_embedder` + `vector_declared` still round-trip the declaration) —
/// **readiness is DERIVED, never stored**, so there is no schema step and no
/// separate flag that could tear (see [`derive_dense_readiness`]).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProjectionVector {
    /// Optional embedder override; `None` ⇒ the engine's shipped default.
    pub embedder: Option<String>,
    /// 0.8.20 Slice 20 (R-20-DR) — **READ METADATA, engine-set.** Populated by
    /// [`Engine::read_projections`]; `None` on every caller-authored spec.
    ///
    /// It is **not part of the declaration**: `configure_projections` neither
    /// stores nor honours it (see [`StoredProjection::from_spec`], which reads
    /// only `embedder`), so a value supplied here is INERT — the engine always
    /// reports the derived truth. This is deliberately accept-inert rather than
    /// hard-reject so `read.projections` output stays feedable straight back
    /// into `configure_projections` (the fix-4 read→configure round-trip, which
    /// both bindings pin with a test).
    ///
    /// **0.8.20 Slice 23 (`R-20-SV`) correction (TC-39 class).** This doc used to
    /// justify accept-inert by analogy with "the already-audited accept-inert
    /// ruling on an `fts`/`vector` sub-object declared without the `searchable`
    /// role". **That ruling is OVERRULED** — the HITL ruled the shape an INVALID
    /// SPEC on 2026-07-24 and `apply_projection_config` now rejects it with
    /// [`EngineError::WriteValidation`]. `dense_readiness` accept-inert is
    /// UNCHANGED and stands on its own footing: it is engine-set READ METADATA,
    /// never part of the declaration, so there is nothing about it to reject.
    ///
    /// The bindings still HARD-REJECT the shapes that could
    /// not round-trip: a readiness supplied with `vector = false`, and any
    /// spelling outside `{unavailable, embedding, ready}`.
    pub dense_readiness: Option<DenseReadiness>,
}

/// 0.8.20 Slice 15d (R-20-PR / C-1) — a single declarative projection
/// declaration. HITL-ratified shape (`api-surface.md:85-89`):
/// `{ name, roles: Set<ProjectionRole>, fts?, vector? }`. `roles` carries SET
/// semantics (dedup + membership; an attribute can be `Filterable` AND
/// `Searchable`) — encoded here as a sorted, de-duplicated `BTreeSet`. Named
/// `roles`, not `kind` (`kind` is the node/edge type discriminator).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectionSpec {
    pub name: String,
    pub roles: BTreeSet<ProjectionRole>,
    pub fts: Option<ProjectionFts>,
    pub vector: Option<ProjectionVector>,
    /// Optional ordered literal object-member path in the canonical node body.
    /// `None` preserves the legacy direct top-level lookup by `name`.
    pub source: Option<Vec<String>>,
}

/// 0.8.20 Slice 15d (R-20-PR) — the diff [`Engine::configure_projections`]
/// applied. Idempotent re-registration yields `unchanged == true` with all
/// vecs empty (the "re-registration is a no-op" acceptance signal). A
/// destructive change without an explicit `drop` is an `Err`, not a delta.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProjectionDelta {
    /// Attribute names whose same-transaction projections (EAV / property-FTS)
    /// were (re)built by this apply.
    pub built: Vec<String>,
    /// Attribute names dropped (explicit `drop` list) — their EAV + property-FTS
    /// rows and registry row removed.
    pub dropped: Vec<String>,
    /// Attribute names whose declared roles were persisted but NOT built:
    /// `rankable` (F9 not yet live) and the `searchable→vector` sub-target
    /// (Slice 20). These graft on a future idempotent apply. No error.
    pub deferred: Vec<String>,
    /// True iff nothing was built, dropped, or newly deferred — the whole apply
    /// diffed to a no-op.
    pub unchanged: bool,
    /// 0.8.20 Slice 22 (R-20-VC / **TC-67**) — **node KINDS, not attribute
    /// names.** The vector-eligible node kinds present in the corpus that the
    /// vector writer can NEVER commit, so no `searchable→vector` declaration
    /// will ever produce an embedding for them.
    ///
    /// # Why this field exists — the silence it replaces
    ///
    /// [`kind_is_vector_committable`] (Slice 20c fix-2) restricted enrolment to
    /// the kinds [`resolve_source_type`] maps, because enrolling any other kind
    /// is a permanent liveness wedge. That fix was correct and is unchanged —
    /// but it made the exclusion **silent**: the declaration persists, its name
    /// is pushed onto [`ProjectionDelta::deferred`], and the caller cannot tell
    /// "waiting on the embedder" (transient) from "this kind will never be
    /// embedded" (permanent). Per the HITL ruling on TC-67 the remedy is
    /// option **(c) REPORT** — the vocabulary is NOT grown and the Pack-1 D3
    /// partition-key lock is NOT touched (`dev/design/0.7.0-vector-quant-pack1.md`).
    ///
    /// # Axis, and why the name is what it is
    ///
    /// `built` / `dropped` / `deferred` are all lists of **projection attribute
    /// names**. This one is a list of **node kinds** — a different axis entirely,
    /// so the name says `kinds` explicitly and is prefixed `vector_` to bind it
    /// to the dense arm (an unsupported kind is still fully FTS/lexically
    /// searchable). Sorted and de-duplicated (`SELECT DISTINCT … ORDER BY kind`).
    ///
    /// # It is a STATE report, not a diff
    ///
    /// Unlike the other three vectors it does not describe what this call
    /// changed; it describes the corpus as it stands. So it is populated on an
    /// idempotent re-apply too (where `unchanged == true` and the other three
    /// are empty), and it deliberately does NOT feed [`ProjectionDelta::unchanged`].
    /// That is what makes the declare-time residual cheap to live with: to
    /// refresh the report after writing new kinds, re-apply the same spec — a
    /// no-op that still returns a current report.
    ///
    /// # Independent of the embedder
    ///
    /// Computed whenever a `searchable→vector` projection is declared, whether
    /// or not this session has a usable dense runtime. The vocabulary is static, so
    /// "this kind can never be embedded" is true in a no-embedder session too —
    /// and must not be conflated with the Q6a graceful-absent deferral, which is
    /// transient and is reported through `deferred`.
    ///
    /// Empty (never absent) when there is nothing to report.
    pub vector_unsupported_kinds: Vec<String>,
}

/// Typed outcome of [`Engine::verify_embedder`]. Mismatches do not raise
/// `EngineError`; the operator workflow needs to see the stored vs.
/// supplied pair to decide on next action.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VerifyEmbedderStatus {
    Match,
    IdentityMismatch,
    DimensionMismatch,
    BothMismatch,
}

/// Result of [`Engine::verify_embedder`]. `stored_identity` is the
/// `name:revision` pair persisted in `_fathomdb_embedder_profiles`;
/// `supplied_identity` echoes the operator's input verbatim.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifyEmbedderReport {
    pub stored_identity: String,
    pub stored_dimension: u32,
    pub supplied_identity: String,
    pub supplied_dimension: u32,
    pub status: VerifyEmbedderStatus,
}

/// Single table or index entry emitted by [`Engine::dump_schema`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchemaObject {
    pub name: String,
    pub sql: String,
}

/// Result of [`Engine::dump_schema`]. `user_version` is the
/// `PRAGMA user_version` sentinel. Canonical tables appear first per
/// [`fathomdb_schema::CANONICAL_TABLES`], then remaining non-`sqlite_*`
/// tables alphabetically. Indexes follow the same alphabetical rule.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DumpSchemaReport {
    pub user_version: u32,
    pub tables: Vec<SchemaObject>,
    pub indexes: Vec<SchemaObject>,
}

/// Single canonical-table row count emitted by [`Engine::dump_row_counts`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TableRowCount {
    pub name: String,
    pub rows: u64,
}

/// Result of [`Engine::dump_row_counts`]. Canonical tables only;
/// projection / FTS / vec0 shadow tables are excluded. Order matches
/// [`fathomdb_schema::CANONICAL_TABLES`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DumpRowCountsReport {
    pub counts: Vec<TableRowCount>,
}

/// 0.8.20 Slice 5d (R-20-E8) — one `source_id` bucket in an
/// [`OrphanProvenanceReport`]. `source_id` is `None` for the NULL-provenance
/// bucket, which after migration step 21 should contain ONLY governed NODES.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrphanProvenanceSource {
    /// `None` = the NULL-`source_id` bucket.
    pub source_id: Option<String>,
    /// Canonical rows (nodes + edges) carrying this provenance.
    pub rows: u64,
    /// How many of `rows` carry a `logical_id`.
    ///
    /// NOT the same thing as "purge-addressable": only a NODE's `logical_id`
    /// confers purge-addressability. An EDGE's `logical_id` is a supersession
    /// identity and reaches no erasure verb (see
    /// [`Engine::orphan_provenance`]), so governed edges are counted here but
    /// are NOT subtracted from
    /// [`OrphanProvenanceReport::unerasable_rows`].
    pub governed_rows: u64,
    /// True for the engine's reserved `_`-prefixed namespace (`_engine:*`,
    /// `_legacy:pre-0.8.20`). Reserved buckets are reachable only through the
    /// operator seam `excise_source`, never through the governed
    /// [`Engine::erase_source`].
    pub reserved: bool,
}

/// Result of [`Engine::orphan_provenance`] — the per-`source_id` census behind
/// `fathomdb doctor orphan-provenance` (design §4 item 11).
///
/// `unerasable_rows` is the load-bearing field: canonical rows carrying
/// NEITHER a `source_id` NOR a `logical_id`. Such a row is reachable by no
/// erasure verb at all — `purge` keys on `logical_id`, `erase_source` keys on
/// `source_id` — so it can never be deleted on request. Slice 5c made that
/// state unwritable and migration step 21 back-filled the historical cases, so
/// a non-zero count means the invariant has been violated and the verb exits
/// `DOCTOR_FOUND_ISSUES`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrphanProvenanceReport {
    /// Per-`source_id` buckets, ordered by descending `rows` then `source_id`
    /// so the output is deterministic (a diagnostic that reorders between runs
    /// cannot be diffed).
    pub sources: Vec<OrphanProvenanceSource>,
    /// Total canonical rows surveyed.
    pub total_rows: u64,
    /// Rows with NO `source_id` AND NO `logical_id` — un-erasable by any verb.
    pub unerasable_rows: u64,
}

/// Result of [`Engine::dump_profile`]. Mirrors the open-time embedder
/// posture + the per-kind vector configuration registered in
/// `_fathomdb_vector_kinds`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DumpProfileReport {
    pub embedder_identity: String,
    pub embedder_dimension: u32,
    pub vectorized_kinds: Vec<String>,
}

/// 0.7.2 PR-2b — result of [`Engine::recompute_mean`] (the manual
/// `doctor recompute-mean` path) and of the shared in-transaction
/// recompute core. `drift_cos_before` is the cosine between the freshly
/// derived corpus mean and the previously-pinned mean (1.0 when nothing
/// was pinned yet, i.e. a first pin). `mean_was_pinned` distinguishes a
/// refresh of an existing mean from an initial pin. See
/// `dev/design/embedder.md` §0.3.
#[derive(Clone, Debug, PartialEq)]
pub struct MeanRecomputeReport {
    pub dim: u32,
    pub old_doc_count: u64,
    pub doc_count_requantized: u64,
    pub drift_cos_before: f32,
    pub mean_was_pinned: bool,
    pub elapsed_ms: u64,
}

/// Typed outcome of [`Engine::truncate_wal`]. `Done` matches SQLite's
/// `busy = 0` return from `PRAGMA wal_checkpoint(TRUNCATE)`; any other
/// value surfaces as `Busy`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TruncateWalStatus {
    Done,
    Busy,
}

/// Result of [`Engine::truncate_wal`] or `recover_truncate_wal`. Carries the
/// three counters returned by `PRAGMA wal_checkpoint(TRUNCATE)`: `busy`,
/// `log_frames`, `checkpointed_frames`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TruncateWalReport {
    pub status: TruncateWalStatus,
    pub busy: u32,
    pub log_frames: u32,
    pub checkpointed_frames: u32,
    /// True only when the locked pre-probe found a malformed WAL and SQLite
    /// subsequently completed the truncate checkpoint.
    pub discarded_corrupt_wal: bool,
}

impl Drop for Engine {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

impl Engine {
    /// Return the immutable settings requested at open. Omitted fields remain
    /// `None`; this does not reflect later effective-value setter calls.
    pub fn config(&self) -> &EngineConfig {
        &self.requested_config
    }

    #[cfg(feature = "test-hooks")]
    #[doc(hidden)]
    pub fn explain_graph_evidence_preflights_for_test(&self) -> Result<Vec<String>, EngineError> {
        self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        evidence::explain_intrinsic_preflights_for_test(connection)
    }

    fn usable_dense_runtime(&self) -> bool {
        usable_dense_runtime(
            self.runtime_embedder.as_deref(),
            self.dense_disabled.load(Ordering::Acquire),
        )
    }

    pub fn open(path: impl Into<PathBuf>) -> Result<OpenedEngine, EngineOpenError> {
        Self::open_with_embedder_and_subscriber(
            path,
            default_embedder_identity(),
            None,
            None,
            None,
            &mut |_| {},
        )
    }

    /// Open an engine with an explicit [`EmbedderChoice`].
    ///
    /// Per `dev/design/embedder.md` §0 + the 0.7.1 EU-5 campaign, this is
    /// the canonical entry point for selecting how the workspace's
    /// default embedder is supplied. See [`EmbedderChoice`] for the
    /// semantics of each variant; in particular `Default` materializes
    /// the pinned BGE embedder via the loader when the `default-embedder`
    /// feature is enabled.
    pub fn open_with_choice(
        path: impl Into<PathBuf>,
        choice: EmbedderChoice,
    ) -> Result<OpenedEngine, EngineOpenError> {
        Self::open_with_choice_and_config(path, choice, EngineConfig::default())
    }

    /// Open with an embedder choice and per-engine settings. Invalid settings
    /// fail before path, lock, provider, or SQLite side effects.
    pub fn open_with_choice_and_config(
        path: impl Into<PathBuf>,
        choice: EmbedderChoice,
        config: EngineConfig,
    ) -> Result<OpenedEngine, EngineOpenError> {
        ResolvedRuntimeConfiguration::resolve(&config)
            .map_err(EngineOpenError::EngineConfiguration)?;
        match choice {
            EmbedderChoice::Default => Self::open_default_embedder(path, config),
            EmbedderChoice::Caller(embedder) => {
                let identity = embedder.identity();
                Self::open_with_embedder_and_subscriber_config(
                    path,
                    identity,
                    Some(embedder),
                    None,
                    None,
                    &mut |_| {},
                    config,
                )
            }
            EmbedderChoice::CallerWithDeviceResolution { embedder, device_resolution } => {
                let identity = embedder.identity();
                Self::open_with_embedder_and_subscriber_config(
                    path,
                    identity,
                    Some(embedder),
                    Some(LoaderInfo {
                        download_ms: None,
                        events: Vec::new(),
                        device_resolution,
                        // A caller-supplied embedder was not constructed here,
                        // so nothing in this process measured an allocation.
                        gpu_allocation_witness: None,
                    }),
                    None,
                    &mut |_| {},
                    config,
                )
            }
            EmbedderChoice::None => Self::open_with_embedder_and_subscriber_config(
                path,
                default_embedder_identity(),
                None,
                None,
                None,
                &mut |_| {},
                config,
            ),
        }
    }

    /// EU-5b: materialize the engine's pinned default embedder
    /// (`CandleBgeEmbedder` backed by the EU-3 loader) and open the
    /// workspace with it. Without the `default-embedder` feature, fails
    /// with a typed `Embedder` error rather than touching the network.
    #[cfg(feature = "default-embedder")]
    fn open_default_embedder(
        path: impl Into<PathBuf>,
        config: EngineConfig,
    ) -> Result<OpenedEngine, EngineOpenError> {
        use std::time::Instant as DownloadInstant;
        let device_resolution = fathomdb_embedder::resolve_default_embedder_device_from_env()
            .map_err(EngineOpenError::EmbedDevicePolicy)?;
        // 0.8.23 Slice 80.6 (D-80.6-6) — the witness runs BEFORE this open's
        // own model reaches the device, so its `free_before`/`free_after`
        // bracket surrounds nothing but the load it is measuring. Opted in
        // only; see `ENV_GPU_ALLOCATION_WITNESS`.
        let gpu_allocation_witness = witness_gpu_allocation_if_requested(&device_resolution)?;
        let download_start = DownloadInstant::now();
        let weights = fathomdb_embedder::loader::load_pinned_default_embedder().map_err(|err| {
            EngineOpenError::Embedder(RuntimeEmbedderError::Failed {
                message: format!("default embedder loader: {err}"),
            })
        })?;
        let events = weights.events.clone();
        let download_ms = if weights.bytes_downloaded > 0 {
            Some(u64::try_from(download_start.elapsed().as_millis()).unwrap_or(u64::MAX))
        } else {
            None
        };
        let embedder =
            fathomdb_embedder::CandleBgeEmbedder::new_from_weights_with_device_resolution(
                weights,
                &device_resolution,
            )
            .map_err(|err| {
                EngineOpenError::Embedder(RuntimeEmbedderError::Failed {
                    message: format!("default embedder construct: {err}"),
                })
            })?;
        let embedder: Arc<dyn Embedder> = Arc::new(embedder);
        let identity = embedder.identity();
        let loader_info =
            LoaderInfo { download_ms, events, device_resolution, gpu_allocation_witness };
        Self::open_with_embedder_and_subscriber_config(
            path,
            identity,
            Some(embedder),
            Some(loader_info),
            None,
            &mut |_| {},
            config,
        )
    }

    #[cfg(not(feature = "default-embedder"))]
    fn open_default_embedder(
        _path: impl Into<PathBuf>,
        _config: EngineConfig,
    ) -> Result<OpenedEngine, EngineOpenError> {
        Err(EngineOpenError::Embedder(RuntimeEmbedderError::Failed {
            message: "EmbedderChoice::Default requires the `default-embedder` Cargo feature"
                .to_string(),
        }))
    }

    pub fn open_with_migration_event_sink(
        path: impl Into<PathBuf>,
        mut emit_migration_event: impl FnMut(&MigrationStepReport),
    ) -> Result<OpenedEngine, EngineOpenError> {
        Self::open_with_embedder_and_subscriber(
            path,
            default_embedder_identity(),
            None,
            None,
            None,
            &mut emit_migration_event,
        )
    }

    #[cfg(feature = "migration-test-hooks")]
    #[doc(hidden)]
    pub fn open_with_migrations_for_test(
        path: impl Into<PathBuf>,
        migrations: &'static [fathomdb_schema::Migration],
        mut emit_migration_event: impl FnMut(&MigrationStepReport),
    ) -> Result<OpenedEngine, EngineOpenError> {
        Self::open_with_migrations(
            path,
            DatabaseOpenPlan {
                migrations,
                admission: DatabaseAdmission::TestMigrations,
                config: EngineConfig::default(),
            },
            default_embedder_identity(),
            None,
            None,
            &mut emit_migration_event,
            None,
        )
    }

    #[doc(hidden)]
    pub fn open_with_subscriber_for_test(
        path: impl Into<PathBuf>,
        subscriber: Arc<dyn lifecycle::Subscriber>,
    ) -> Result<OpenedEngine, EngineOpenError> {
        Self::open_with_embedder_and_subscriber(
            path,
            default_embedder_identity(),
            None,
            None,
            Some(subscriber),
            &mut |_| {},
        )
    }

    #[doc(hidden)]
    pub fn open_without_embedder_for_test(
        path: impl Into<PathBuf>,
    ) -> Result<OpenedEngine, EngineOpenError> {
        Self::open_with_embedder_and_subscriber(
            path,
            default_embedder_identity(),
            None,
            None,
            None,
            &mut |_| {},
        )
    }

    #[doc(hidden)]
    pub fn open_with_embedder_for_test(
        path: impl Into<PathBuf>,
        embedder: Arc<dyn Embedder>,
    ) -> Result<OpenedEngine, EngineOpenError> {
        let identity = embedder.identity();
        Self::open_with_embedder_and_subscriber(
            path,
            identity,
            Some(embedder),
            None,
            None,
            &mut |_| {},
        )
    }

    fn open_with_embedder_and_subscriber(
        path: impl Into<PathBuf>,
        embedder_identity: EmbedderIdentity,
        runtime_embedder: Option<Arc<dyn Embedder>>,
        loader_info: Option<LoaderInfo>,
        initial_subscriber: Option<Arc<dyn lifecycle::Subscriber>>,
        emit_migration_event: &mut impl FnMut(&MigrationStepReport),
    ) -> Result<OpenedEngine, EngineOpenError> {
        Self::open_with_embedder_and_subscriber_config(
            path,
            embedder_identity,
            runtime_embedder,
            loader_info,
            initial_subscriber,
            emit_migration_event,
            EngineConfig::default(),
        )
    }

    fn open_with_embedder_and_subscriber_config(
        path: impl Into<PathBuf>,
        embedder_identity: EmbedderIdentity,
        runtime_embedder: Option<Arc<dyn Embedder>>,
        loader_info: Option<LoaderInfo>,
        initial_subscriber: Option<Arc<dyn lifecycle::Subscriber>>,
        emit_migration_event: &mut impl FnMut(&MigrationStepReport),
        config: EngineConfig,
    ) -> Result<OpenedEngine, EngineOpenError> {
        Self::open_with_migrations(
            path,
            DatabaseOpenPlan {
                migrations: MIGRATIONS,
                admission: DatabaseAdmission::CurrentOnly,
                config,
            },
            embedder_identity,
            runtime_embedder,
            loader_info,
            emit_migration_event,
            initial_subscriber,
        )
    }

    fn open_with_migrations(
        path: impl Into<PathBuf>,
        plan: DatabaseOpenPlan,
        embedder_identity: EmbedderIdentity,
        runtime_embedder: Option<Arc<dyn Embedder>>,
        loader_info: Option<LoaderInfo>,
        emit_migration_event: &mut impl FnMut(&MigrationStepReport),
        initial_subscriber: Option<Arc<dyn lifecycle::Subscriber>>,
    ) -> Result<OpenedEngine, EngineOpenError> {
        let resolved_config = ResolvedRuntimeConfiguration::resolve(&plan.config)
            .map_err(EngineOpenError::EngineConfiguration)?;
        let config = plan.config.clone();
        // Resolve at open rather than piggybacking on embedding selection. This
        // probes no model/cache/database and makes an invalid or forced CUDA
        // policy visible to every SDK before a query could silently fall back.
        #[cfg(feature = "default-reranker")]
        let reranker_device_resolution = Some(
            fathomdb_embedder::resolve_default_reranker_device_from_env()
                .map_err(EngineOpenError::RerankerDevicePolicy)?,
        );
        #[cfg(not(feature = "default-reranker"))]
        let reranker_device_resolution = None;
        let canonical_path = canonical_database_path(&path.into())?;
        let report_preopen_error = |error| {
            if let Some(subscriber) = initial_subscriber.as_ref() {
                emit_open_error_event(subscriber, &error);
            }
            error
        };
        let embed_dispatch = Arc::new(
            EmbedDispatcher::new(
                runtime_embedder.clone(),
                resolved_config.embedder_pool_size,
                Duration::from_millis(resolved_config.embedder_call_timeout_ms),
            )
            .map_err(|error| EngineOpenError::Io { message: error.to_string() })
            .map_err(&report_preopen_error)?,
        );
        let mut embed_dispatch_guard = OpenEmbedDispatchGuard(Some(Arc::clone(&embed_dispatch)));
        let pending_lock = acquire_lock_without_metadata_mutation(&canonical_path)
            .map_err(&report_preopen_error)?;
        configure_runtime_for_open()
            .map_err(EngineOpenError::RuntimeConfiguration)
            .map_err(&report_preopen_error)?;
        #[cfg(test)]
        run_admission_locked_hook_for_test(&canonical_path);
        if plan.admission == DatabaseAdmission::CurrentOnly {
            admit_current_database(&canonical_path).map_err(&report_preopen_error)?;
        }
        let lock = pending_lock.initialize().map_err(&report_preopen_error)?;
        #[cfg(any(test, feature = "test-hooks"))]
        let managed_connections = Arc::new(ManagedConnectionRegistry::default());
        #[cfg(feature = "migration-test-hooks")]
        let allow_populated_legacy_projection_bootstrap =
            plan.admission == DatabaseAdmission::TestMigrations;
        #[cfg(not(feature = "migration-test-hooks"))]
        let allow_populated_legacy_projection_bootstrap = false;
        let open_result = Self::open_locked(
            canonical_path.clone(),
            plan.migrations,
            allow_populated_legacy_projection_bootstrap,
            &embedder_identity,
            emit_migration_event,
            #[cfg(any(test, feature = "test-hooks"))]
            Arc::clone(&managed_connections),
        );

        match open_result {
            Ok((connection, readers, mut report, reader_lookaside_rcs)) => {
                // EU-5b — splice the loader's measurements + structured
                // events into the report. The loader path is the only
                // surface that produces these today; caller-supplied
                // embedders and EmbedderChoice::None leave them as the
                // open_locked defaults (None / empty).
                if let Some(info) = loader_info {
                    if info.download_ms.is_some() {
                        report.embedder_download_ms = info.download_ms;
                    }
                    if !info.events.is_empty() {
                        report.embedder_events = info.events;
                    }
                    report.embedder_device_resolution = Some(info.device_resolution);
                    // D-80.6-6 — assigned, not merged: `None` here means this
                    // open measured no witness, and there is no earlier value
                    // that a `None` could be hiding.
                    report.embedder_gpu_allocation_witness = info.gpu_allocation_witness;
                }
                report.reranker_device_resolution = reranker_device_resolution;

                // 0.8.18 Slice 5 (#5 vector-equivalence probe KEYSTONE) — run the
                // open-time self-check on the FINAL post-recovery connection (the
                // mean is already pinned/recovered inside open_locked, U1-b). First
                // registration persists the 45 UN-centered f32 references; a
                // subsequent open re-embeds + asserts P1 (mean-centered flip count,
                // floor 0) and P2 (un-centered L2 ε). Divergence ⇒ degraded-open
                // (`dense_disabled=true`), surfaced on the OpenReport (R-VEQ-6); the
                // query-time refusal fires later at `search_inner_with_stats`.
                // A durable declaration is a prospective dense arm even before
                // it has enrolled a kind. Check it before the boot graft below:
                // a refused backend may leave the declaration at rest, but must
                // not enrol, requeue, dispatch, or write any dense work.
                let prospective_dense_arm =
                    vector_projection_declared(&connection).map_err(|_| EngineOpenError::Io {
                        message: "could not inspect declared vector projection on open".to_string(),
                    })?;
                let veq = run_vector_equivalence_probe(
                    &connection,
                    runtime_embedder.as_ref().map(|_| embed_dispatch.as_ref()),
                    &embedder_identity,
                    report.embedder_mean_vec_pinned,
                    prospective_dense_arm,
                );
                report.dense_disabled = veq.dense_disabled;
                report.dense_disabled_reason = veq.reason.clone();

                let dense_runtime_usable =
                    usable_dense_runtime(runtime_embedder.as_deref(), veq.dense_disabled);
                let boot_graft_enqueued = if dense_runtime_usable {
                    boot_graft_declared_vector_backfill(&connection).map_err(|_| {
                        EngineOpenError::Io {
                            message: "could not graft declared vector projection on boot"
                                .to_string(),
                        }
                    })?
                } else {
                    false
                };

                let next_cursor = load_next_cursor(&connection);
                let read_visibility_generation =
                    frozen_read::load_visibility_generation(&connection).map_err(|_| {
                        EngineOpenError::Io {
                            message: "could not load frozen-read visibility generation".to_string(),
                        }
                    })?;
                let subscribers = Arc::new(lifecycle::SubscriberRegistry::new());
                let profiling_enabled = Arc::new(AtomicBool::new(false));
                let slow_threshold_ms = Arc::new(AtomicU64::new(resolved_config.slow_threshold_ms));
                let wal_attribution = Arc::new(WalAttributionCollector::new());
                wal_attribution.register(WalAttributionRole::Writer, 0);
                #[cfg(any(test, feature = "test-hooks"))]
                let writer_connection_registration =
                    managed_connections.register(WalAttributionRole::Writer, 0);
                let mut profile_contexts: Vec<Box<ProfileContext>> = Vec::new();
                let scheduler_embedder =
                    if dense_runtime_usable { runtime_embedder.clone() } else { None };
                let projection_runtime = ProjectionRuntime::new(
                    canonical_path.clone(),
                    scheduler_embedder,
                    Arc::clone(&embed_dispatch),
                    embedder_identity.clone(),
                    report.embedder_mean_vec_pinned,
                    Arc::clone(&subscribers),
                    Arc::clone(&wal_attribution),
                    resolved_config,
                    #[cfg(any(test, feature = "test-hooks"))]
                    Arc::clone(&managed_connections),
                )?;

                install_profile_callback(
                    &connection,
                    &subscribers,
                    &profiling_enabled,
                    &slow_threshold_ms,
                    &mut profile_contexts,
                );
                for reader in &readers {
                    install_profile_callback(
                        reader,
                        &subscribers,
                        &profiling_enabled,
                        &slow_threshold_ms,
                        &mut profile_contexts,
                    );
                }

                #[cfg(test)]
                let profile_contexts = ProfileContexts::from(profile_contexts);
                let opened = OpenedEngine {
                    engine: Self {
                        path: canonical_path.clone(),
                        requested_config: config,
                        resolved_config,
                        next_cursor: AtomicU64::new(next_cursor),
                        read_visibility_generation: Arc::new(AtomicU64::new(
                            read_visibility_generation,
                        )),
                        projection_generation_status_cache: Mutex::new(None),
                        mutation_projection_status_cache: Mutex::new(None),
                        #[cfg(feature = "test-hooks")]
                        projection_generation_status_full_owner_scan_count: AtomicU64::new(0),
                        #[cfg(feature = "test-hooks")]
                        graph_expand_rss_baseline_bytes: AtomicU64::new(0),
                        #[cfg(feature = "test-hooks")]
                        graph_expand_rss_delta_bytes: AtomicU64::new(0),
                        #[cfg(feature = "test-hooks")]
                        graph_evidence_before_resolve_return_hook: Mutex::new(None),
                        #[cfg(feature = "test-hooks")]
                        erasure_before_primary_lock_hook: Mutex::new(None),
                        closed: AtomicBool::new(false),
                        lock: Mutex::new(Some(lock)),
                        connection: Mutex::new(Some(connection)),
                        reader_pool: ReaderWorkerPool::new(
                            readers,
                            Arc::clone(&wal_attribution),
                            #[cfg(any(test, feature = "test-hooks"))]
                            Arc::clone(&managed_connections),
                        ),
                        counters: lifecycle::Counters::new(),
                        subscribers,
                        profiling_enabled,
                        slow_threshold_ms,
                        runtime_embedder,
                        embed_dispatch,
                        runtime_embedder_identity: embedder_identity,
                        projection_runtime,
                        wal_attribution,
                        #[cfg(any(test, feature = "test-hooks"))]
                        managed_connections,
                        #[cfg(any(test, feature = "test-hooks"))]
                        writer_connection_registration: Mutex::new(Some(
                            writer_connection_registration,
                        )),
                        #[cfg(any(test, feature = "test-hooks"))]
                        actual_checkpoint_observations: Mutex::new(None),
                        #[cfg(any(test, feature = "test-hooks"))]
                        binding_native_state_observations: Mutex::new(None),
                        provenance_row_cap: AtomicU64::new(resolved_config.provenance_row_cap),
                        profile_contexts: Mutex::new(profile_contexts),
                        reader_lookaside_rcs,
                        telemetry: Mutex::new(None),
                        telemetry_enabled: AtomicBool::new(false),
                        explanation_open_nonce: mint_explanation_open_nonce(),
                        explanation_sequence: AtomicU64::new(0),
                        dense_disabled: AtomicBool::new(veq.dense_disabled),
                        dense_disabled_reason: Mutex::new(veq.reason),
                        vector_equivalence_refusals: AtomicU64::new(0),
                        #[cfg(debug_assertions)]
                        force_next_commit_failure: AtomicBool::new(false),
                        #[cfg(debug_assertions)]
                        actuation_after_initial_lookup_delay_ms: AtomicU64::new(0),
                        #[cfg(debug_assertions)]
                        actuation_failure_after_operation: AtomicUsize::new(usize::MAX),
                    },
                    report,
                };
                if let Some(subscriber) = initial_subscriber {
                    opened.engine.subscribers.attach_persistent(subscriber);
                }
                if dense_runtime_usable
                    && (boot_graft_enqueued
                        || database_has_pending_projection_work(
                            &canonical_path,
                            #[cfg(any(test, feature = "test-hooks"))]
                            &opened.engine.managed_connections,
                        )
                        .unwrap_or(false))
                {
                    opened.engine.projection_runtime.notify_new_work();
                }
                embed_dispatch_guard.disarm();
                Ok(opened)
            }
            Err(err) => {
                if let Some(subscriber) = initial_subscriber {
                    emit_open_error_event(&subscriber, &err);
                }
                drop(lock);
                Err(err)
            }
        }
    }

    fn open_locked(
        path: PathBuf,
        migrations: &'static [fathomdb_schema::Migration],
        allow_populated_legacy_projection_bootstrap: bool,
        embedder_identity: &EmbedderIdentity,
        emit_migration_event: &mut impl FnMut(&MigrationStepReport),
        #[cfg(any(test, feature = "test-hooks"))] managed_connections: Arc<
            ManagedConnectionRegistry,
        >,
    ) -> Result<(Connection, Vec<Connection>, OpenReport, Vec<i32>), EngineOpenError> {
        register_sqlite_vec_extension();
        let mut connection = open_managed_connection(
            &path,
            #[cfg(any(test, feature = "test-hooks"))]
            ManagedConnectionCategory::Writer,
            #[cfg(any(test, feature = "test-hooks"))]
            &managed_connections,
        )
        .map_err(|err| map_open_sqlite_error(err, OpenStage::HeaderProbe))?;
        // Order pinned by `dev/design/errors.md` § OpenStage matrix: each
        // step routes its own SQLite-level error to a distinct
        // `CorruptionKind` (Header → WalReplay → Schema → EmbedderIdentity).
        // The schema and WAL probes both happen BEFORE `pragma WAL`
        // because that pragma also reads page 1 — letting it run first
        // would reclassify schema-side corruption as a WAL replay
        // failure, breaking the AC-035b stable-code contract.
        probe_database_header(&connection)?;
        probe_open_integrity(&connection)?;
        probe_wal_sidecar(&path)?;
        // 0.7.0 perf-experiments: apply writer-side experiment PRAGMAs
        // (page_size, etc.) BEFORE journal_mode + migrations. page_size
        // is silently ignored once any table exists; this is the only
        // legal window to set it on a fresh DB. Gated on
        // FATHOMDB_PERF_EXPERIMENTS=1; no-op in production.
        apply_perf_experiment_writer_pragmas(&connection);
        // OPP-12 Phase-1 (0.8.19 Slice 10, design §3 gap-4) — standing
        // `secure_delete=ON` on the writer, applied at EVERY open (fresh + migrated).
        // It zeroes every page freed by a future DELETE, so the Slice-10 `purge`
        // hard-erase is complete WITHOUT a per-purge `VACUUM`. It is a connection
        // PRAGMA (not schema DDL), so it belongs here, not in the 19→20 migration.
        // RESIDUAL (documented, not forced): pages freed on a pre-20 DB BEFORE this
        // was enabled are not retroactively scrubbed; there is no migration-time
        // full `VACUUM` (O(db-size)). NOTE: this is a standing pragma set at EVERY
        // connection open (writer here, plus the reader-pool and
        // `open_runtime_connection`), NOT the writer alone — non-writer connections
        // also free pages (projection / vector-rewrite DELETEs), so a writer-only
        // `secure_delete` would leak freed content on disk. See the matching
        // reader/runtime open comment (~lines 3335-3336).
        connection
            .pragma_update(None, "secure_delete", "ON")
            .map_err(|err| map_open_sqlite_error(err, OpenStage::WalReplay))?;
        connection
            .pragma_update(None, "journal_mode", "WAL")
            .map_err(|err| map_open_sqlite_error(err, OpenStage::WalReplay))?;
        connection
            .pragma_update(None, "synchronous", "NORMAL")
            .map_err(|err| map_open_sqlite_error(err, OpenStage::WalReplay))?;
        #[cfg(feature = "test-hooks")]
        record_writer_pragma_witness_for_test(&connection);

        reject_legacy_shape(&connection)?;
        let migration = migrate_with_event_sink(&connection, migrations, emit_migration_event)
            .map_err(map_migration_error)?;
        validate_dependency_generation_on_open(&connection, migration.schema_version_after)?;
        frozen_read::validate_on_open(&connection, migration.schema_version_after)
            .map_err(|message| EngineOpenError::Io { message })?;
        dependency_closure::validate_closure_state_on_open(
            &connection,
            migration.schema_version_after,
        )?;
        // 0.8.0 Slice 5 (G1) — global FTS5 tokenizer-default upgrade. Step 11
        // drops + recreates `search_index` with the new tokenizer, leaving it
        // EMPTY on a migrated DB. The projection scheduler will NOT
        // repopulate it (`database_has_pending_projection_work` keys "pending"
        // off `_fathomdb_projection_terminal`, which the migration does not
        // clear). Re-tokenize from the canonical source rows here, on the
        // writer connection, single-threaded, before readers spawn —
        // projection-only, no source-record migration.
        //
        // Crash-retryable (fix-1): step 11 commits `user_version = 11` with an
        // empty index in its OWN transaction; this reproject commits in a
        // LATER transaction. A crash in that window leaves a durable v11 + empty
        // index, on which a boundary-crossing guard (`before < 11`) is FALSE,
        // skipping repair forever. So gate on the completion marker's ABSENCE
        // (written atomically with the reindex) instead: idempotent, and a
        // crash before the reindex commit simply re-runs on the next open.
        if migration.schema_version_after >= SEARCH_INDEX_TOKENIZER_SCHEMA_VERSION
            && !search_index_tokenizer_reproject_complete(&connection).map_err(|_| {
                EngineOpenError::Io {
                    message: "could not read search_index tokenizer reproject marker".to_string(),
                }
            })?
        {
            reproject_search_index_after_tokenizer_upgrade(&connection).map_err(|_| {
                EngineOpenError::Io {
                    message: "could not re-tokenize search_index after tokenizer upgrade"
                        .to_string(),
                }
            })?;
        }
        let mut embedder_mean_vec_pinned = check_embedder_profile(&connection, embedder_identity)?;
        ensure_vector_partition(&mut connection, embedder_identity.dimension).map_err(|_| {
            EngineOpenError::Io { message: "could not initialize vector partition".to_string() }
        })?;
        projection_generation::bootstrap(
            &mut connection,
            migration.schema_version_after,
            allow_populated_legacy_projection_bootstrap,
        )
        .map_err(|error| match error {
            EngineError::ProjectionGeneration(_) => EngineOpenError::Corruption(CorruptionDetail {
                kind: CorruptionKind::ProjectionGenerationDrift,
                stage: OpenStage::ProjectionGeneration,
                locator: CorruptionLocator::TableRow {
                    table: "_fathomdb_projection_generation_current",
                    rowid: 1,
                },
                recovery_hint: RecoveryHint {
                    code: "E_CORRUPT_PROJECTION_GENERATION",
                    doc_anchor: "design/recovery-0.8.25.md#projection-generation",
                },
            }),
            _ => EngineOpenError::Io {
                message: "could not initialize projection generation".to_string(),
            },
        })?;

        // 0.8.20 Slice 15c (TC-33) fix-6 [codex §9 P1] — the step-23
        // `canonical_edges` recreate drops every edge row (NO DATA MIGRATION) and
        // removes their `_fathomdb_vector_rows` sidecar rows, but the vec0
        // `vector_default` shadow those mirror is engine-created + dim-aware, so
        // the migration cannot delete its rows. Left behind, an orphaned edge vec0
        // row (whose `canonical_edges` row is gone) still occupies a top-K KNN
        // candidate slot — `build_vector_phase1_sql` reads candidates DIRECTLY
        // from `vector_default` before hydrating them through the canonical tables
        // — and is then discarded at hydration, so an upgraded DB silently returns
        // too few / no vector results. Prune the orphans now that
        // `ensure_vector_partition` guarantees `vector_default` exists, BEFORE the
        // mean-vec row-count recovery below (so the count excludes them). One-time
        // and crash-retryable via the durable completion marker; a no-op on any
        // healthy corpus (every vec0 row has a sidecar entry), so recall / eu7
        // fidelity are unchanged on a DB that never dropped edges.
        if migration.schema_version_after >= EDGE_TEMPORAL_EPOCH_SCHEMA_VERSION
            && !edge_vector_prune_complete(&connection).map_err(|_| EngineOpenError::Io {
                message: "could not read edge-vector prune marker".to_string(),
            })?
        {
            prune_orphaned_edge_vectors(&connection).map_err(|_| EngineOpenError::Io {
                message: "could not prune orphaned edge vector rows".to_string(),
            })?;
        }

        // 0.8.20 Slice 15d (R-20-PR, Q5) — boot re-derive the projection registry
        // (the engine `ProjectionSpec` is a derived cache). For every persisted
        // declaration, clear + backfill its EAV / property-FTS rows from the
        // canonical nodes so a crash window (registry row survives, projection
        // rows partial) self-heals idempotently. A no-op single empty-table read
        // on every DB that has not declared a projection. On the writer
        // connection, single-threaded, before readers spawn — like the tokenizer
        // reproject above. Runs after the fix-6 edge-vector prune above; the two
        // are independent boot reconciliations.
        rederive_projections_on_boot(&connection).map_err(|_| EngineOpenError::Io {
            message: "could not re-derive projection registry on boot".to_string(),
        })?;

        // 0.8.20 Slice 21 fix-1 (codex §9 round 1 [P2], ledger `TC-71`) — bring an
        // ALREADY-ENROLLED inert vector kind into agreement with the role-aware
        // decision. Slice 21c closed the three forward doors, but a database that
        // already ran the old code under `{roles:[filterable], vector:{}}` keeps
        // its `_fathomdb_vector_kinds` rows — `vector_kind_needs_enrolment`
        // short-circuits on `kind_is_vector_indexed` and never reaches the new
        // predicate, and `project_canonical_node_row` reads only the registry
        // membership — so upgrading did not actually stop the unwanted embeddings.
        // Narrowly authorised (registry EXISTS, declares a `vector` sub-object,
        // and declares no `searchable→vector` projection) so a LEGACY workspace
        // with a working dense arm is never touched; see
        // [`registry_governs_an_inert_dense_arm`]. Deletes no embedding. Runs
        // BEFORE `run_vector_equivalence_probe` (which fires after `open_locked`
        // returns), so a database whose only enrolment was the inert one pays no
        // probe embeds on the healing open. Another boot reconciliation on the
        // writer connection, single-threaded, before readers spawn.
        reconcile_inert_vector_enrolments_on_boot(&connection).map_err(|_| {
            EngineOpenError::Io {
                message: "could not reconcile inert vector kind enrolments on boot".to_string(),
            }
        })?;

        // 0.8.20 Slice 15e — reconcile the live `vector_default` attribute columns
        // with the registry's `filterable` set. On a DB whose vec0 shape already
        // matches the registry (the common case, incl. every reopen of a DB that
        // declared filterable projections in a prior session) this is a pure
        // no-op: the diff is empty, so boot never re-inserts and NEVER silently
        // wipes the corpus. It converges only a shape that drifted from the
        // registry (e.g. a restored registry row). A no-op when the table is
        // absent (no embedder). Runs on the writer connection, single-threaded,
        // before readers spawn — like the boot re-derive above.
        {
            let tx = connection.transaction().map_err(|_| EngineOpenError::Io {
                message: "could not begin vector-attr reconcile on boot".to_string(),
            })?;
            reconcile_vector_attr_columns(&tx, embedder_identity.dimension).map_err(|_| {
                EngineOpenError::Io {
                    message: "could not reconcile vector attribute columns on boot".to_string(),
                }
            })?;
            tx.commit().map_err(|_| EngineOpenError::Io {
                message: "could not commit vector-attr reconcile on boot".to_string(),
            })?;
        }

        // EU-5f — recovery pin (`dev/design/embedder.md` §0.3, Hazard 4). If
        // the identity is MC-required, no mean is pinned, yet the workspace
        // already holds >= MEAN_VEC_PIN_THRESHOLD vector rows (e.g. a crash
        // between the threshold-crossing write and its pin commit), derive
        // the mean from the existing un-centered rows and pin+re-quantize
        // now, single-threaded, before the projection workers spawn. The
        // NULL guard makes this idempotent on subsequent opens.
        if identity_requires_mean_centering(embedder_identity) && !embedder_mean_vec_pinned {
            let row_count: u64 = connection
                .query_row("SELECT COUNT(*) FROM vector_default", [], |row| row.get(0))
                .unwrap_or(0);
            if row_count >= MEAN_VEC_PIN_THRESHOLD {
                recover_mean_vec_pin(&mut connection, embedder_identity).map_err(|_| {
                    EngineOpenError::Io {
                        message: "could not recover mean-centering pin".to_string(),
                    }
                })?;
                embedder_mean_vec_pinned = true;
            }
        }

        let warmup_started = Instant::now();
        // Static identity capability — see `dev/design/embedder.md`
        // §0.6. Today only the bge-small identity reports `true`; the
        // noop scaffolding identity is `false`. EU-5b's identity flip
        // makes the Default path return `true` here automatically.
        let embedder_mean_centering_required = embedder_identity.name == BGE_SMALL_EMBEDDER_NAME;
        // EU-5a2 — populated from `_fathomdb_embedder_profiles.mean_vec`
        // by `check_embedder_profile` above (was hard-coded `false` in
        // EU-5a1). Dimension invariant (§0.2) enforced by that check.
        let report = OpenReport {
            schema_version_before: migration.schema_version_before,
            schema_version_after: migration.schema_version_after,
            migration_steps: migration.migration_steps,
            embedder_warmup_ms: u64::try_from(warmup_started.elapsed().as_millis())
                .unwrap_or(u64::MAX),
            query_backend: "fathomdb-query + sqlite-vec",
            default_embedder: embedder_identity.clone(),
            // TODO(EU-5b): surface `LoadedWeights.download_ms` from the
            // loader once the Default path materializes through it.
            embedder_download_ms: None,
            // TODO(EU-5b): surface `LoadedWeights.events` from the loader.
            embedder_events: Vec::new(),
            embedder_mean_centering_required,
            embedder_mean_vec_pinned,
            // 0.8.18 Slice 5 — set by the #5 self-check in `open_with_migrations`
            // (which has the runtime embedder in scope). `open_locked` returns the
            // non-degraded default; the probe runs after this returns.
            dense_disabled: false,
            dense_disabled_reason: None,
            embedder_device_resolution: None,
            reranker_device_resolution: None,
            // 0.8.23 Slice 80.6 (D-80.6-6) — set by `open_with_migrations`
            // only when an opted-in CUDA default-embedder open measured one.
            embedder_gpu_allocation_witness: None,
        };

        let mut readers = Vec::with_capacity(READER_POOL_SIZE);
        let mut lookaside_rcs: Vec<i32> = Vec::with_capacity(READER_POOL_SIZE);
        for _ in 0..READER_POOL_SIZE {
            let reader = open_managed_connection(
                &path,
                #[cfg(any(test, feature = "test-hooks"))]
                ManagedConnectionCategory::ReaderWorker,
                #[cfg(any(test, feature = "test-hooks"))]
                &managed_connections,
            )
            .map_err(|err| map_open_sqlite_error(err, OpenStage::HeaderProbe))?;
            // Pack 6.G G.1: configure per-connection lookaside BEFORE
            // any PRAGMA / prepare runs on this reader. Reordering this
            // after the journal-mode / query_only PRAGMAs would let
            // SQLite silently ignore the lookaside setting.
            let rc: i32 = configure_reader_lookaside(&reader);
            debug_assert_eq!(
                rc,
                rusqlite::ffi::SQLITE_OK,
                "sqlite3_db_config(LOOKASIDE) must return SQLITE_OK on a freshly opened reader",
            );
            lookaside_rcs.push(rc);
            reader
                .pragma_update(None, "journal_mode", "WAL")
                .map_err(|err| map_open_sqlite_error(err, OpenStage::WalReplay))?;
            // OPP-12 Phase-1 (0.8.19 Slice 10, design §3 gap-4) — `secure_delete=ON`
            // at EVERY connection open, not just the writer. `secure_delete` is a
            // per-connection pager flag, so a reader-pool connection that frees a
            // page (vector-rewrite / projection DELETEs run off non-writer
            // connections) would otherwise leave that freed content on disk,
            // defeating GDPR erasure. Set BEFORE `query_only=ON` so the ordering is
            // unambiguous (the flag is a pager setting, not a DB write).
            reader
                .pragma_update(None, "secure_delete", "ON")
                .map_err(|err| map_open_sqlite_error(err, OpenStage::WalReplay))?;
            reader
                .pragma_update(None, "query_only", "ON")
                .map_err(|err| map_open_sqlite_error(err, OpenStage::SchemaProbe))?;
            apply_perf_experiment_reader_pragmas(&reader);
            readers.push(reader);
        }

        Ok((connection, readers, report, lookaside_rcs))
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    #[allow(dead_code)]
    fn wal_attribution_snapshot(&self) -> WalAttributionSnapshot {
        self.wal_attribution.snapshot()
    }

    #[allow(dead_code)]
    fn wal_attribution_checkpoints_for_test(&self) -> Vec<WalCheckpointRecord> {
        self.wal_attribution.checkpoints()
    }

    // `test` added alongside `debug_assertions`/`test-hooks` so the crate's own
    // `--release --tests` lib-test build (cfg(test) true, debug_assertions
    // false) can still see this seam; it stays absent from any shipped build.
    #[cfg(any(test, debug_assertions, feature = "test-hooks"))]
    #[allow(dead_code)]
    #[doc(hidden)]
    pub fn pause_reader_after_wal_snapshot_for_test(&self) -> (Arc<Barrier>, Arc<Barrier>) {
        let snapshot_ready = Arc::new(Barrier::new(2));
        let release = Arc::new(Barrier::new(2));
        self.reader_pool
            .hold_worker_zero_wal_snapshot(Arc::clone(&snapshot_ready), Arc::clone(&release));
        (snapshot_ready, release)
    }

    /// Pause worker zero on a real snapshot with cancellation-safe bounded
    /// release. Dropping the returned sender releases the worker immediately.
    #[cfg(debug_assertions)]
    #[doc(hidden)]
    pub fn pause_reader_with_timeout_for_test(&self) -> (Receiver<usize>, SyncSender<()>) {
        let (snapshot_ready_tx, snapshot_ready_rx) = mpsc::sync_channel(1);
        let (release_tx, release_rx) = mpsc::sync_channel(1);
        self.reader_pool.hold_worker_zero_wal_snapshot_bounded(snapshot_ready_tx, release_rx);
        (snapshot_ready_rx, release_tx)
    }

    #[cfg(test)]
    fn post_commit_ack_for_test(&self) -> (Arc<Barrier>, Arc<Barrier>, Arc<Barrier>) {
        let snapshot_ready = Arc::new(Barrier::new(2));
        let release = Arc::new(Barrier::new(2));
        let committed = Arc::new(Barrier::new(2));
        self.reader_pool.hold_worker_zero_wal_snapshot_with_commit_ack(
            Arc::clone(&snapshot_ready),
            Arc::clone(&release),
            Arc::clone(&committed),
        );
        (snapshot_ready, release, committed)
    }

    #[cfg(any(test, feature = "test-hooks"))]
    #[doc(hidden)]
    pub fn arm_next_reader_snapshot_pause_for_test(
        &self,
    ) -> (Arc<Barrier>, Arc<Barrier>, Arc<Mutex<Option<String>>>) {
        self.wal_attribution.arm_reader_snapshot_pause()
    }

    /// Arm a private completion rendezvous for the next public reader request.
    ///
    /// The ready barrier fires only after the helper-local SQLite transaction
    /// and statements have dropped, the attribution collector is idle, and
    /// before the materialized response is delivered. Available only to tests
    /// and disposable `test-hooks` artifacts.
    #[cfg(any(test, feature = "test-hooks"))]
    #[doc(hidden)]
    pub fn arm_next_reader_completion_pause_for_test(
        &self,
    ) -> (Arc<Barrier>, Arc<Barrier>, Arc<AtomicBool>) {
        self.wal_attribution.arm_reader_completion_pause()
    }

    /// Private Slice 65 test rendezvous. Unlike the snapshot hook this is
    /// deliberately unavailable to test-hook artifacts: it verifies only the
    /// collector's internal response-handoff boundary.
    #[cfg(test)]
    fn pause_next_reader_handoff_for_test(&self) -> (Arc<Barrier>, Arc<Barrier>) {
        self.wal_attribution.arm_reader_handoff_pause()
    }

    #[cfg(feature = "test-hooks")]
    #[doc(hidden)]
    pub fn wal_attribution_checkpoint_records_for_test(
        &self,
    ) -> Vec<(usize, bool, String, Vec<String>)> {
        self.wal_attribution_checkpoints_for_test()
            .into_iter()
            .map(|record| {
                (
                    record.attempt,
                    record.busy,
                    record.classification.to_string(),
                    record
                        .active_roles
                        .iter()
                        .map(|(role, index)| format!("{}:{index}", role.name()))
                        .collect(),
                )
            })
            .collect()
    }

    #[cfg(feature = "test-hooks")]
    #[doc(hidden)]
    pub fn wal_attribution_idle_for_test(&self) -> bool {
        self.wal_attribution_snapshot().no_owned_snapshot
    }

    /// Arm private observation for the next erasure checkpoint sequence. The
    /// observer only reads connection state around the already-required
    /// checkpoint call; it never issues a diagnostic SQLite statement.
    #[cfg(any(test, feature = "test-hooks"))]
    fn arm_actual_checkpoint_observation_for_test(&self, control: &'static str) {
        *self.actual_checkpoint_observations.lock().expect("actual checkpoint observation") =
            Some(ActualCheckpointObserver { control, records: Vec::new() });
    }

    #[cfg(any(test, feature = "test-hooks"))]
    fn actual_checkpoint_observation_for_test(
        &self,
        phase: &'static str,
        ordinal: usize,
        checkpoint_begin_overlap: bool,
        elapsed: Option<Duration>,
        report: Option<TruncateWalReport>,
    ) {
        let mut observations =
            self.actual_checkpoint_observations.lock().expect("actual checkpoint observation");
        let Some(observer) = observations.as_mut() else {
            return;
        };
        let writer_autocommit = self
            .connection
            .lock()
            .ok()
            .and_then(|connection| connection.as_ref().map(Connection::is_autocommit))
            .unwrap_or(false);
        let expected_runtime_probes = match observer.control {
            "direct_rust" => 0,
            "python_serial" => 0,
            _ => 2,
        };
        let direct_inventory =
            self.actual_checkpoint_direct_inventory_for_test(expected_runtime_probes);
        let collector_roles = self
            .wal_attribution_snapshot()
            .active_roles
            .into_iter()
            .map(|(role, index)| format!("{}:{index}", role.name()))
            .collect();
        observer.records.push(ActualCheckpointObservation {
            control: observer.control,
            phase,
            ordinal,
            writer_autocommit,
            direct_inventory,
            collector_roles,
            checkpoint_begin_overlap,
            elapsed,
            report,
        });
    }

    #[cfg(any(test, feature = "test-hooks"))]
    fn actual_checkpoint_direct_inventory_for_test(
        &self,
        expected_runtime_probes: usize,
    ) -> String {
        let worker_count = self.projection_runtime.shared.worker_count;
        let registry_complete = self.managed_connections.exact_live(worker_count);
        let creation = self.managed_connections.creation_counts();
        let writer_autocommit = self
            .connection
            .lock()
            .ok()
            .and_then(|connection| connection.as_ref().map(Connection::is_autocommit))
            .unwrap_or(false);
        let readers = self.reader_pool.wal_connection_inventory_for_test();
        let runtime = self.projection_runtime.report_runtime_connection_inventory_for_test();
        let reader_autocommit =
            readers.len() == READER_POOL_SIZE && readers.iter().all(|value| *value);
        let runtime_autocommit = runtime.as_ref().is_ok_and(|entries| {
            let expected = projection_runtime::projection_wal_roles(worker_count);
            entries.iter().map(|(role, index, _)| (*role, *index)).collect::<BTreeSet<_>>()
                == expected
                && entries.iter().all(|(_, _, autocommit)| *autocommit)
        });
        let dispatcher_autocommit = runtime.as_ref().is_ok_and(|entries| {
            entries
                .iter()
                .find(|(role, index, _)| {
                    *role == WalAttributionRole::ProjectionDispatcher && *index == 0
                })
                .is_some_and(|(_, _, autocommit)| *autocommit)
        });
        let workers_autocommit = runtime.as_ref().is_ok_and(|entries| {
            entries
                .iter()
                .filter(|(role, _, _)| *role == WalAttributionRole::ProjectionWorker)
                .count()
                == worker_count
                && entries
                    .iter()
                    .filter(|(role, _, _)| *role == WalAttributionRole::ProjectionWorker)
                    .all(|(_, _, autocommit)| *autocommit)
        });
        let creation_text = creation.map_or_else(
            || "unknown".to_string(),
            |(writer, readers, dispatcher, workers, probes)| {
                format!("writer:{writer},readers:{readers},dispatcher:{dispatcher},workers:{workers},probes:{probes}")
            },
        );
        let complete = registry_complete
            && creation == Some((1, READER_POOL_SIZE, 1, worker_count, expected_runtime_probes))
            && writer_autocommit
            && reader_autocommit
            && runtime_autocommit;
        format!(
            "roles=writer:0,readers:0-7,dispatcher:0,workers:0-{};writer={};readers={};dispatcher={};workers={};registry={};creation={};complete={}",
            worker_count - 1,
            if writer_autocommit { "autocommit" } else { "not_autocommit" },
            if reader_autocommit { "autocommit" } else { "not_autocommit" },
            if dispatcher_autocommit { "autocommit" } else { "not_autocommit" },
            if workers_autocommit { format!("{worker_count}-autocommit") } else { "not_autocommit".to_string() },
            if registry_complete { "complete" } else { "incomplete" },
            creation_text,
            u8::from(complete),
        )
    }

    #[cfg(any(test, feature = "test-hooks"))]
    fn take_actual_checkpoint_observations_for_test(&self) -> Vec<String> {
        let records = self
            .actual_checkpoint_observations
            .lock()
            .expect("actual checkpoint observation")
            .take()
            .map_or_else(Vec::new, |observer| observer.records);
        records
            .into_iter()
            .map(|record| {
                let collector_roles = if record.collector_roles.is_empty() {
                    "idle".to_string()
                } else {
                    record.collector_roles.join(",")
                };
                let timing = record.elapsed.map_or_else(String::new, |elapsed| {
                    format!(" elapsed_ms={}", elapsed.as_millis())
                });
                let outcome = record.report.map_or_else(String::new, |report| {
                    format!(
                        " busy={} log_frames={} checkpointed_frames={}",
                        u8::from(report.busy != 0), report.log_frames, report.checkpointed_frames,
                    )
                });
                format!(
                    "control={} phase={} ordinal={} writer_autocommit={} direct_inventory={} collector_roles={} checkpoint_begin_overlap={}{}{}",
                    record.control,
                    record.phase,
                    record.ordinal,
                    u8::from(record.writer_autocommit),
                    record.direct_inventory,
                    collector_roles,
                    u8::from(record.checkpoint_begin_overlap),
                    timing,
                    outcome,
                )
            })
            .collect()
    }

    #[cfg(feature = "test-hooks")]
    #[doc(hidden)]
    pub fn arm_python_serial_actual_checkpoint_observation_for_test(&self) {
        self.arm_actual_checkpoint_observation_for_test("python_serial");
    }

    #[cfg(feature = "test-hooks")]
    #[doc(hidden)]
    pub fn drain_actual_checkpoint_observations_for_test(&self) -> Vec<String> {
        self.take_actual_checkpoint_observations_for_test()
    }

    /// Arm direct native-state observation around the existing Slice 65
    /// test-hook sampler. It is deliberately separate from the real-erasure
    /// observer so the normal serial path gains no connection inspection.
    #[cfg(any(test, feature = "test-hooks"))]
    #[doc(hidden)]
    pub fn arm_binding_native_state_observation_for_test(&self) {
        *self.binding_native_state_observations.lock().expect("binding native state observation") =
            Some(BindingNativeStateObserver { records: Vec::new() });
    }

    #[cfg(any(test, feature = "test-hooks"))]
    fn binding_native_state_observation_for_test(&self, phase: &'static str, ordinal: usize) {
        let mut observations = self
            .binding_native_state_observations
            .lock()
            .expect("binding native state observation");
        let Some(observer) = observations.as_mut() else {
            return;
        };
        observer.records.push(BindingNativeStateObservation {
            phase,
            ordinal,
            inventory: self.native_state_inventory_for_test(),
        });
    }

    #[cfg(any(test, feature = "test-hooks"))]
    #[doc(hidden)]
    pub fn drain_binding_native_state_observations_for_test(&self) -> Vec<String> {
        self.binding_native_state_observations
            .lock()
            .expect("binding native state observation")
            .take()
            .map_or_else(Vec::new, |observer| observer.records)
            .into_iter()
            .map(|record| {
                format!(
                    "control=binding_sampler phase={} ordinal={} {}",
                    record.phase,
                    record.ordinal,
                    native_state_inventory_text(&record.inventory),
                )
            })
            .collect()
    }

    #[cfg(any(test, feature = "test-hooks"))]
    fn native_state_inventory_for_test(&self) -> NativeStateInventory {
        let worker_count = self.projection_runtime.shared.worker_count;
        let mut facts = Vec::with_capacity(1 + READER_POOL_SIZE + 1 + worker_count);
        let writer = match self.connection.lock() {
            Ok(connection) => connection.as_ref().map_or_else(
                || {
                    unavailable_native_connection_state_for_test(
                        WalAttributionRole::Writer,
                        0,
                        NativeStateReply::Error("writer_unavailable"),
                    )
                },
                |connection| {
                    native_connection_state_for_test(connection, WalAttributionRole::Writer, 0)
                },
            ),
            Err(_) => unavailable_native_connection_state_for_test(
                WalAttributionRole::Writer,
                0,
                NativeStateReply::Error("writer_lock"),
            ),
        };
        facts.push(writer);
        facts.extend(self.reader_pool.wal_native_state_inventory_for_test());
        match self.projection_runtime.report_runtime_native_state_inventory_for_test() {
            Ok(runtime) => facts.extend(runtime),
            Err(reason) => {
                facts.push(unavailable_native_connection_state_for_test(
                    WalAttributionRole::ProjectionDispatcher,
                    0,
                    NativeStateReply::Error(reason),
                ));
                facts.extend((0..worker_count).map(|index| {
                    unavailable_native_connection_state_for_test(
                        WalAttributionRole::ProjectionWorker,
                        index,
                        NativeStateReply::Error(reason),
                    )
                }));
            }
        }
        facts.sort_by_key(|fact| (fact.role, fact.index));
        let expected = native_state_expected_roles(worker_count);
        let actual = facts.iter().map(|fact| (fact.role, fact.index)).collect::<BTreeSet<_>>();
        let unique = facts.len() == actual.len();
        let managed = self.managed_connections.exact_live(worker_count);
        let received_and_idle = facts.iter().all(|fact| {
            matches!(fact.reply, NativeStateReply::Received)
                && fact.autocommit == Some(true)
                && fact.transaction == NativeTransactionState::None
                && fact.busy_statement == Some(false)
        });
        let (complete, reason) = if !managed {
            (false, "registry_mismatch")
        } else if actual != expected || !unique {
            (false, "role_mismatch")
        } else if !received_and_idle {
            (false, "native_state_not_idle")
        } else {
            (true, "complete")
        };
        NativeStateInventory { facts, complete, reason }
    }

    /// Return exact direct native state for the private installed-binding
    /// diagnostic. Any missing, duplicate, timed-out, errored, or non-idle
    /// role fails closed; callers cannot upgrade a checkpoint classification.
    #[cfg(feature = "test-hooks")]
    #[doc(hidden)]
    pub fn binding_native_state_inventory_for_test(&self) -> Result<String, EngineError> {
        self.ensure_open()?;
        let inventory = self.native_state_inventory_for_test();
        if !inventory.complete {
            return Err(EngineError::Storage);
        }
        Ok(native_state_inventory_text(&inventory))
    }

    /// Return direct, complete managed-connection facts for the private Slice
    /// 65 installed-binding diagnostic. This is intentionally unavailable from
    /// ordinary builds and never enters an SDK error or diagnostic surface.
    #[cfg(any(test, feature = "test-hooks"))]
    #[doc(hidden)]
    pub fn binding_connection_inventory_for_test(&self) -> Result<String, EngineError> {
        self.ensure_open()?;
        let deadline = Instant::now() + Duration::from_secs(2);
        let worker_count = self.projection_runtime.shared.worker_count;
        while !self.managed_connections.exact_live(worker_count) && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(5));
        }
        if !self.managed_connections.exact_live(worker_count) {
            return Err(EngineError::Storage);
        }
        let creation = self.managed_connections.creation_counts().ok_or(EngineError::Storage)?;
        if creation != (1, READER_POOL_SIZE, 1, worker_count, 0) {
            return Err(EngineError::Storage);
        }
        let writer_autocommit = self
            .connection
            .lock()
            .map_err(|_| EngineError::Storage)?
            .as_ref()
            .is_some_and(Connection::is_autocommit);
        if !writer_autocommit {
            return Err(EngineError::Storage);
        }
        let readers = self.reader_pool.wal_connection_inventory_for_test();
        if readers.len() != READER_POOL_SIZE || readers.into_iter().any(|autocommit| !autocommit) {
            return Err(EngineError::Storage);
        }
        let runtime = self
            .projection_runtime
            .report_runtime_connection_inventory_for_test()
            .map_err(|_| EngineError::Storage)?;
        let expected = projection_runtime::projection_wal_roles(worker_count);
        let actual =
            runtime.iter().map(|(role, index, _)| (*role, *index)).collect::<BTreeSet<_>>();
        if actual != expected || runtime.iter().any(|(_, _, autocommit)| !autocommit) {
            return Err(EngineError::Storage);
        }
        let snapshot = self.wal_attribution_snapshot();
        if !snapshot.no_owned_snapshot
            || snapshot.roles.iter().any(|role| role.active || role.phase != "idle")
        {
            return Err(EngineError::Storage);
        }
        Ok(format!(
            "roles=writer:0,readers:0-7,dispatcher:0,workers:0-{};writer=autocommit;readers=8-autocommit;dispatcher=autocommit;workers={worker_count}-autocommit;creation=writer:{},readers:{},dispatcher:{},workers:{},probes:{}",
            worker_count - 1,
            creation.0, creation.1, creation.2, creation.3, creation.4,
        ))
    }

    /// Run one bounded, test-only checkpoint sampler without performing any
    /// erasure work. The returned records are private diagnostic observations,
    /// not a retry or reclassification of an erasure outcome.
    #[cfg(any(test, feature = "test-hooks"))]
    #[doc(hidden)]
    pub fn checkpoint_at_rest_for_test(&self) -> Result<Vec<(bool, u32, u32)>, EngineError> {
        self.ensure_open()?;
        let mut reports = Vec::new();
        for attempt in 0..ERASURE_WAL_TRUNCATE_ATTEMPTS {
            self.wal_attribution.set(WalAttributionRole::Writer, 0, true, "checkpoint_start");
            let overlap = self.wal_attribution.checkpoint_begin();
            self.binding_native_state_observation_for_test("before", (attempt + 1) as usize);
            let started = Instant::now();
            let report = self.wal_checkpoint_truncate_once(false)?;
            self.binding_native_state_observation_for_test("after", (attempt + 1) as usize);
            self.wal_attribution.checkpoint_end();
            self.wal_attribution.set(WalAttributionRole::Writer, 0, false, "idle");
            let snapshot = self.wal_attribution_snapshot();
            let classification = self.wal_attribution.classification(&snapshot, overlap);
            self.wal_attribution.checkpoint_event(
                (attempt + 1) as usize,
                started.elapsed(),
                &report,
                classification,
                snapshot.active_roles,
            );
            reports.push((report.busy != 0, report.log_frames, report.checkpointed_frames));
            if report.busy == 0 {
                break;
            }
            if attempt + 1 < ERASURE_WAL_TRUNCATE_ATTEMPTS {
                thread::sleep(Duration::from_millis(ERASURE_WAL_TRUNCATE_BACKOFF_MS));
            }
        }
        Ok(reports)
    }

    /// Take one native Rusqlite checkpoint sample for the disposable Slice 65
    /// child probe. This opens and drops exactly one independent connection;
    /// it neither opens an Engine nor retries an erasure result.
    #[cfg(feature = "test-hooks")]
    #[doc(hidden)]
    pub fn native_raw_wal_checkpoint_for_test(path: &str) -> Result<(bool, u32, u32), EngineError> {
        let registry = Arc::new(ManagedConnectionRegistry::default());
        let connection = open_managed_connection(
            Path::new(path),
            ManagedConnectionCategory::RuntimeProbe,
            &registry,
        )
        .map_err(|_| EngineError::Storage)?;
        connection.execute_batch("PRAGMA busy_timeout = 0").map_err(|_| EngineError::Storage)?;
        connection
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
                Ok((row.get::<_, i32>(0)? != 0, row.get(1)?, row.get(2)?))
            })
            .map_err(|_| EngineError::Storage)
    }

    #[cfg(debug_assertions)]
    #[allow(dead_code)]
    #[doc(hidden)]
    pub fn pause_projection_worker_after_wal_transaction_for_test(
        &self,
    ) -> ProjectionWorkerTransactionPauseForTest {
        self.projection_runtime.pause_projection_worker_after_wal_transaction_for_test()
    }

    /// Pause one captured-generation job while it remains in the worker queue.
    #[cfg(feature = "test-hooks")]
    #[doc(hidden)]
    pub fn pause_projection_worker_while_queued_for_test(&self) -> (Arc<Barrier>, Arc<Barrier>) {
        self.projection_runtime.pause_projection_worker_while_queued_for_test()
    }

    /// Pause one computed job immediately before SQLite write-lock acquisition.
    #[cfg(feature = "test-hooks")]
    #[doc(hidden)]
    pub fn pause_projection_worker_before_write_lock_for_test(
        &self,
    ) -> (Arc<Barrier>, Arc<Barrier>) {
        self.projection_runtime.pause_projection_worker_before_write_lock_for_test()
    }

    fn detect_slow(&self, started: Instant, category: lifecycle::EventCategory) {
        let elapsed = started.elapsed();
        let threshold = self.slow_threshold_ms.load(Ordering::Relaxed);
        let threshold_duration = std::time::Duration::from_millis(threshold);
        if elapsed > threshold_duration {
            // `dev/design/lifecycle.md` § Slow and heartbeat policy: a slow
            // operation produces TWO correlated facts. The
            // statement-level slow-statement signal is dispatched by the
            // sqlite3_profile callback (`profile_callback_trampoline`).
            // This site emits the lifecycle `Phase::Slow` event for the
            // outer operation envelope (AC-008).
            self.emit_event(lifecycle::Phase::Slow, category, None);
        }
    }

    fn emit_event(
        &self,
        phase: lifecycle::Phase,
        category: lifecycle::EventCategory,
        code: Option<&'static str>,
    ) {
        let event =
            lifecycle::Event { phase, source: lifecycle::EventSource::Engine, category, code };
        self.subscribers.dispatch(&event);
    }

    /// Emit a `(SqliteInternal, Error, code: <SQLITE_*>)` lifecycle
    /// event for a rusqlite error. Per `dev/design/lifecycle.md`
    /// § Diagnostic source and category, SQLite-originated diagnostics
    /// route through the same host subscriber as engine-originated
    /// events with `source` preserved. AC-021 dispatches on
    /// `code == "SQLITE_SCHEMA"`.
    fn emit_sqlite_internal_error(&self, err: &rusqlite::Error) {
        if let Some(code) = sqlite_extended_code_name(err) {
            let event = lifecycle::Event {
                phase: lifecycle::Phase::Failed,
                source: lifecycle::EventSource::SqliteInternal,
                category: lifecycle::EventCategory::Error,
                code: Some(code),
            };
            self.subscribers.dispatch(&event);
        }
    }

    /// G0 Phase-2 (BLOCK-1) test seam — runs the graph-arm retrieval path and
    /// returns the frontier meter (`GraphFrontierStats`) for `query`. Mirrors the
    /// sanctioned `set_vector_stage_only_for_test` / `_configure_vector_kind_for_test`
    /// pattern: kept OFF the governed surface (test/eval-only), so the meter never
    /// appears on `SearchResult`. Used by the recall harness to prove the
    /// doc-seeded frontier is empty (`resolved_seed_rate == 0.0`) and, post-C1, the
    /// 0→>0 flip.
    pub fn _graph_frontier_stats_for_test(
        &self,
        query: &str,
    ) -> Result<GraphFrontierStats, EngineError> {
        self.search_inner_with_stats(
            query,
            None,
            0,
            true,
            0.3,
            0,
            false,
            ReadView::default(),
            DEFAULT_SEARCH_RESULT_LIMIT,
        )
        .map(|(_result, stats)| stats)
    }

    /// Test-only stage attribution for the Slice 45 performance receipt.
    #[cfg(feature = "test-hooks")]
    #[doc(hidden)]
    pub fn measure_slice45_frozen_stages_for_test(
        &self,
        context: &FrozenReadContextV1,
        page: &PageRequestV1,
    ) -> Result<Slice45FrozenStageTiming, EngineError> {
        self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        let started = Instant::now();
        pagination::authenticate_cursor(connection, page)?;
        let cursor_authentication_ns = started.elapsed().as_nanos();
        let started = Instant::now();
        let binding = frozen_read::authenticate(connection, context)?;
        let token_authentication_ns = started.elapsed().as_nanos();
        let started = Instant::now();
        frozen_read::validate_snapshot(connection, &binding)?;
        let snapshot_binding_ns = started.elapsed().as_nanos();
        Ok(Slice45FrozenStageTiming {
            cursor_authentication_ns,
            token_authentication_ns,
            snapshot_binding_ns,
        })
    }

    /// Test-only stage attribution for one Slice 45 frozen-context mint.
    #[cfg(feature = "test-hooks")]
    #[doc(hidden)]
    pub fn measure_slice45_mint_stages_for_test(
        &self,
        context: &ReadContextV1,
    ) -> Result<Slice45MintStageTiming, EngineError> {
        self.ensure_open()?;
        let mut connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_mut().ok_or(EngineError::Closing)?;
        let (_, generation, timing) = frozen_read::mint_measured(connection, context)?;
        self.read_visibility_generation.fetch_max(generation, Ordering::AcqRel);
        Ok(Slice45MintStageTiming {
            context_validation_ns: timing.context_validation_ns,
            snapshot_validation_ns: timing.snapshot_validation_ns,
            binding_ns: timing.binding_ns,
            token_codec_ns: timing.token_codec_ns,
        })
    }

    /// Test-only `EXPLAIN QUERY PLAN` output for the three Slice 45 read shapes.
    #[cfg(feature = "test-hooks")]
    #[doc(hidden)]
    pub fn slice45_page_query_plans_for_test(
        &self,
        kind: &str,
    ) -> Result<Vec<(String, Vec<String>)>, EngineError> {
        self.ensure_open()?;
        let connection = open_managed_connection(
            &self.path,
            ManagedConnectionCategory::RuntimeProbe,
            &self.managed_connections,
        )
        .map_err(|_| EngineError::Storage)?;
        let (canonical_sql, canonical_binds) =
            canonical_page_query(kind, &ReadView::default(), &SearchFilter::default(), 0, 100)
                .map_err(|_| EngineError::Storage)?;
        let plans = [
            ("canonical_page", canonical_sql, canonical_binds),
            (
                "operational_point",
                OPERATIONAL_STATE_POINT_SQL.to_string(),
                vec![
                    rusqlite::types::Value::Text("state".to_string()),
                    rusqlite::types::Value::Text("key".to_string()),
                ],
            ),
            (
                "operational_page",
                OPERATIONAL_STATE_PAGE_SQL.to_string(),
                vec![
                    rusqlite::types::Value::Text("state".to_string()),
                    rusqlite::types::Value::Integer(0),
                    rusqlite::types::Value::Integer(101),
                ],
            ),
        ];
        plans
            .into_iter()
            .map(|(name, sql, binds)| {
                let mut statement = connection
                    .prepare(&format!("EXPLAIN QUERY PLAN {sql}"))
                    .map_err(|_| EngineError::Storage)?;
                let details = statement
                    .query_map(rusqlite::params_from_iter(binds.iter()), |row| row.get(3))
                    .map_err(|_| EngineError::Storage)?
                    .collect::<Result<Vec<String>, _>>()
                    .map_err(|_| EngineError::Storage)?;
                Ok((name.to_string(), details))
            })
            .collect()
    }

    pub fn close(&self) -> Result<(), EngineError> {
        self.closed.store(true, Ordering::SeqCst);
        self.projection_runtime.stop();
        // Uninstall profile callbacks before dropping the connections so
        // SQLite cannot fire one last callback against a profile context
        // whose Box is about to free. Per `dev/design/engine.md` § Close
        // path step 6, readers drain before the writer connection so
        // SQLite's last-handle checkpointer runs on the writer. Each
        // reader worker uninstalls its own callback inside
        // `reader_worker_loop` before dropping its connection, then
        // exits — `shutdown` joins those threads here.
        self.reader_pool.shutdown();
        if let Ok(mut connection) = self.connection.lock() {
            if let Some(conn) = connection.as_ref() {
                uninstall_profile_callback(conn);
            }
            connection.take();
        }
        #[cfg(any(test, feature = "test-hooks"))]
        if let Ok(mut registration) = self.writer_connection_registration.lock() {
            registration.take();
        }
        if let Ok(mut contexts) = self.profile_contexts.lock() {
            contexts.clear();
        }
        if let Ok(mut lock) = self.lock.lock() {
            lock.take();
        }
        Ok(())
    }

    /// Block until in-flight writes drain or `timeout_ms` elapses.
    ///
    /// Surface owned by `dev/interfaces/rust.md` § Engine-attached
    /// instrumentation; semantics are owned by `dev/design/lifecycle.md`.
    pub fn drain(&self, timeout_ms: u64) -> Result<(), EngineError> {
        self.ensure_open()?;
        // Only a session with no configured embedder can produce the typed
        // missing-configuration result. A configured (including refused)
        // runtime goes straight to the scheduler's authoritative idle check,
        // avoiding an otherwise duplicate full pending-work scan.
        if self.runtime_embedder.is_none() {
            let readiness = self.read_embedding_readiness()?;
            if let Some(blocked) = readiness.blocked {
                return Err(EngineError::EmbedderRequired(blocked));
            }
            // The readiness read already uses the same durable pending-work
            // predicate as `wait_for_idle`. When it finds no pending row, wait
            // only for any worker finishing its post-commit bookkeeping;
            // opening a second connection for an identical database scan cannot
            // add evidence.
            if readiness.pending_count == 0 {
                return if self.projection_runtime.wait_for_workers_idle(timeout_ms) {
                    Ok(())
                } else {
                    Err(EngineError::Scheduler)
                };
            }
        }
        if self.projection_runtime.wait_for_idle(timeout_ms, || match self.connection.try_lock() {
            Ok(connection) => Some(
                connection
                    .as_ref()
                    .and_then(|connection| connection_has_pending_projection_work(connection).ok())
                    .unwrap_or(true),
            ),
            Err(std::sync::TryLockError::WouldBlock) => None,
            Err(std::sync::TryLockError::Poisoned(_)) => Some(true),
        }) {
            Ok(())
        } else {
            Err(EngineError::Scheduler)
        }
    }

    /// Wait before a metadata-only mutation that must not turn an absent
    /// embedder into an unrelated operation failure. Direct [`Self::drain`]
    /// still reports the typed Slice-30 feedback; with no configured runtime
    /// no embedding worker can be concurrently committing the durable pending
    /// rows, so registry and lifecycle metadata may be updated safely.
    fn drain_for_non_embedding_mutation(&self) -> Result<(), EngineError> {
        match self.drain(LIFECYCLE_DRAIN_TIMEOUT_MS) {
            Ok(()) | Err(EngineError::EmbedderRequired(_)) => Ok(()),
            Err(error) => Err(error),
        }
    }

    /// Snapshot of engine-internal counters.
    ///
    /// Field set owned by `dev/design/lifecycle.md`.
    #[must_use]
    pub fn counters(&self) -> CounterSnapshot {
        self.counters.snapshot()
    }

    /// Toggle response-cycle profiling.
    ///
    /// Per `dev/design/lifecycle.md` § Per-statement profiling, profiling
    /// is an opt-in surface that is independently toggleable on a running
    /// engine without restart. AC-005a locks runtime toggleability.
    pub fn set_profiling(&self, enabled: bool) -> Result<(), EngineError> {
        self.profiling_enabled.store(enabled, Ordering::Relaxed);
        Ok(())
    }

    /// Set the threshold above which an operation is reported as slow.
    ///
    /// Per `dev/design/lifecycle.md` § Slow and heartbeat policy, the
    /// threshold is runtime-configurable; mutating it changes detection
    /// behavior on subsequent statements without restart (AC-007b).
    pub fn set_slow_threshold_ms(&self, value: u64) -> Result<(), EngineError> {
        self.slow_threshold_ms.store(value, Ordering::Relaxed);
        Ok(())
    }

    /// Attach a host subscriber to engine events.
    ///
    /// Dropping the returned [`Subscription`] detaches the subscriber.
    /// Payload shape owned by `dev/design/lifecycle.md` and
    /// `dev/design/migrations.md`.
    #[must_use]
    pub fn subscribe(&self, subscriber: Arc<dyn lifecycle::Subscriber>) -> Subscription {
        self.subscribers.attach(subscriber)
    }

    #[cfg(debug_assertions)]
    #[doc(hidden)]
    pub fn reader_worker_count_for_test(&self) -> usize {
        self.reader_pool.worker_count()
    }

    #[cfg(debug_assertions)]
    #[doc(hidden)]
    pub fn live_reader_worker_count_for_test(&self) -> usize {
        self.reader_pool.live_count()
    }

    /// Return the worker index that the next round-robin read dispatch will use.
    /// This test-only witness does not mutate scheduling state.
    #[cfg(debug_assertions)]
    #[doc(hidden)]
    pub fn next_reader_worker_index_for_test(&self) -> usize {
        self.reader_pool.next_worker_index()
    }

    /// Pack 6.G G.1 — return the `sqlite3_db_config(LOOKASIDE)` rc
    /// captured for each reader worker at open time, in worker index
    /// order. SQLITE_OK (= 0) means the lookaside was configured
    /// before any allocation happened on the connection.
    #[cfg(debug_assertions)]
    #[doc(hidden)]
    pub fn reader_lookaside_config_rcs_for_test(&self) -> Vec<i32> {
        self.reader_lookaside_rcs.clone()
    }

    /// Pack 6.G G.1 — query each reader worker's
    /// `SQLITE_DBSTATUS_LOOKASIDE_USED` counter. A value > 0 means at
    /// least one allocation was satisfied from the per-connection
    /// lookaside arena (proof the configuration was honored before the
    /// first prepare).
    #[cfg(debug_assertions)]
    #[doc(hidden)]
    pub fn reader_lookaside_used_per_worker_for_test(&self) -> Vec<i32> {
        self.reader_pool.lookaside_used_per_worker()
    }

    /// Pack 6.G G.3.5 — broadcast a debug-only `CacheStatus` request to
    /// every reader worker and collect per-worker
    /// `SQLITE_DBSTATUS_CACHE_HIT` / `_CACHE_MISS` / `_CACHE_USED`
    /// values. Counters are monotonic (reset flag = 0); callers compute
    /// pre/post deltas explicitly.
    #[cfg(debug_assertions)]
    #[doc(hidden)]
    pub fn cache_status_per_worker_for_test(&self, label: &str) -> Vec<CacheStatusReply> {
        self.reader_pool.cache_status_per_worker(label)
    }

    #[cfg(debug_assertions)]
    #[doc(hidden)]
    pub fn force_next_commit_failure_for_test(&self) {
        self.force_next_commit_failure.store(true, Ordering::SeqCst);
    }

    #[cfg(debug_assertions)]
    #[doc(hidden)]
    pub fn set_actuation_after_initial_lookup_delay_ms_for_test(&self, value: u64) {
        self.actuation_after_initial_lookup_delay_ms.store(value, Ordering::SeqCst);
    }

    #[cfg(debug_assertions)]
    #[doc(hidden)]
    pub fn force_actuation_failure_after_operation_for_test(&self, index: usize) {
        self.actuation_failure_after_operation.store(index, Ordering::SeqCst);
    }

    /// Force the next background projection terminal commit to fail with a
    /// synthetic SQLite busy error. Test-only seam for TC-91 rollback and
    /// redispatch coverage; it does not affect the caller's write transaction.
    #[cfg(debug_assertions)]
    #[doc(hidden)]
    pub fn force_next_projection_commit_failure_for_test(&self) {
        self.projection_runtime.force_next_projection_commit_failure_for_test();
    }

    /// Force the next background projection terminal commit to fail with a
    /// rusqlite-layer storage error. Test-only TC-91 diagnostic classifier seam.
    #[cfg(debug_assertions)]
    #[doc(hidden)]
    pub fn force_next_projection_storage_failure_for_test(&self) {
        self.projection_runtime.force_next_projection_storage_failure_for_test();
    }

    /// Pause a worker after a forced projection-commit error was reported and
    /// before its state cleanup. TC-91 test-only shutdown/reopen rendezvous.
    #[cfg(debug_assertions)]
    #[doc(hidden)]
    pub fn pause_projection_commit_failure_cleanup_for_test(
        &self,
        reported: Arc<Barrier>,
        release: Arc<Barrier>,
    ) {
        self.projection_runtime.pause_projection_commit_failure_cleanup_for_test(reported, release);
    }

    /// Acknowledge after `Engine::close` marks the projection runtime stopping
    /// and before it joins workers. TC-91 test-only shutdown rendezvous.
    #[cfg(debug_assertions)]
    #[doc(hidden)]
    pub fn acknowledge_projection_stop_for_test(&self, acknowledged: Arc<Barrier>) {
        self.projection_runtime.acknowledge_projection_stop_for_test(acknowledged);
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

    #[doc(hidden)]
    pub fn set_projection_scheduler_frozen_for_test(&self, frozen: bool) {
        self.projection_runtime.set_frozen(frozen);
    }

    /// Mint a configuration-origin generation without changing declarations.
    ///
    /// This test hook isolates the worker's captured-generation publication
    /// fence. Production transitions remain owned by configuration/rebuild.
    #[cfg(feature = "test-hooks")]
    #[doc(hidden)]
    pub fn transition_projection_generation_for_test(
        &self,
    ) -> Result<ProjectionGenerationId, EngineError> {
        self.ensure_open()?;
        let mut guard = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = guard.as_mut().ok_or(EngineError::Closing)?;
        let tx = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| EngineError::Storage)?;
        let generation =
            projection_generation::transition(&tx, ProjectionGenerationOriginV1::Configuration)?;
        tx.commit().map_err(|_| EngineError::Storage)?;
        Ok(generation)
    }

    /// Return the number of uncached generation-status full-owner scans.
    ///
    /// The counter increments at the sole call site immediately before
    /// `status_in_snapshot`, whose completion summary aggregates every eligible
    /// node and edge owner. Cache hits do not increment it.
    #[cfg(feature = "test-hooks")]
    #[doc(hidden)]
    pub fn projection_generation_status_full_owner_scan_count_for_test(&self) -> u64 {
        self.projection_generation_status_full_owner_scan_count.load(Ordering::Relaxed)
    }

    /// Return `EXPLAIN QUERY PLAN` details for the production status queries.
    #[cfg(feature = "test-hooks")]
    #[doc(hidden)]
    pub fn projection_generation_status_query_plans_for_test(
        &self,
    ) -> Result<Vec<String>, EngineError> {
        self.ensure_open()?;
        let guard = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = guard.as_ref().ok_or(EngineError::Closing)?;
        projection_generation::status_query_plans_for_test(connection)
    }

    /// Attempt worker-success publication with an explicitly captured epoch.
    #[cfg(feature = "test-hooks")]
    #[doc(hidden)]
    pub fn publish_projection_success_for_test(
        &self,
        cursor: u64,
        kind: &str,
        generation_id: ProjectionGenerationId,
    ) -> Result<(), EngineError> {
        self.ensure_open()?;
        let vector =
            vec![0.25_f32; self.projection_runtime.shared.embedder_identity.dimension as usize];
        let blob = encode_vector_blob(&vector);
        let outcome = ProjectionOutcome::Success {
            cursor,
            kind: kind.to_string(),
            blob: blob.clone(),
            bin_blob: blob,
            generation_id,
        };
        let mut connection = open_runtime_connection(
            &self.projection_runtime.shared.path,
            ManagedConnectionCategory::ProjectionWorker,
            &self.projection_runtime.shared.managed_connections,
        )
        .map_err(|_| EngineError::Storage)?;
        commit_projection_outcomes(&mut connection, &[outcome], &self.projection_runtime.shared, 0)
            .map_err(|_| EngineError::Storage)
    }

    /// Test-only snapshot of whether the dispatcher has a scan wake pending.
    ///
    /// This exists to prove pure observers do not notify the scheduler. It is
    /// deliberately narrower than a scheduler control or diagnostic surface.
    #[doc(hidden)]
    pub fn projection_scheduler_pending_scan_for_test(&self) -> bool {
        self.projection_runtime.pending_scan_for_test()
    }

    #[doc(hidden)]
    pub fn set_projection_retry_delays_for_test(&self, delays_ms: &[u64]) {
        self.projection_runtime.set_retry_delays_for_test(delays_ms);
    }

    /// Set the provider dispatch deadline for projection tests; production
    /// requests use the validated engine-open configuration.
    #[doc(hidden)]
    pub fn set_embed_timeout_ms_for_test(&self, timeout_ms: u64) {
        self.projection_runtime.set_embed_timeout_ms_for_test(timeout_ms);
    }

    #[doc(hidden)]
    pub fn projection_status_for_test(
        &self,
        kind: &str,
    ) -> Result<lifecycle::ProjectionStatus, EngineError> {
        self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        projection_status(connection, kind)
    }

    #[doc(hidden)]
    pub fn has_vector_for_cursor_for_test(&self, cursor: u64) -> Result<bool, EngineError> {
        self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        terminal_state_for_cursor(connection, cursor)
            .map(|state| matches!(state.as_deref(), Some("up_to_date")))
            .map_err(|_| EngineError::Storage)
    }

    #[doc(hidden)]
    pub fn projection_failure_count_for_test(&self, cursor: u64) -> Result<u64, EngineError> {
        self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        connection
            .query_row(
                "SELECT COUNT(*) FROM operational_mutations
                 WHERE collection_name = 'projection_failures'
                   AND record_key = ?1",
                [cursor.to_string()],
                |row| row.get::<_, u64>(0),
            )
            .map_err(|_| EngineError::Storage)
    }

    #[doc(hidden)]
    pub fn set_provenance_row_cap_for_test(&self, cap: Option<u64>) {
        self.provenance_row_cap.store(cap.unwrap_or(0), Ordering::Relaxed);
    }

    #[doc(hidden)]
    pub fn provenance_row_count_for_test(&self) -> Result<u64, EngineError> {
        self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        connection
            .query_row("SELECT COUNT(*) FROM operational_mutations", [], |row| row.get::<_, u64>(0))
            .map_err(|_| EngineError::Storage)
    }

    #[doc(hidden)]
    pub fn oldest_provenance_record_key_for_test(
        &self,
        collection: &str,
    ) -> Result<Option<String>, EngineError> {
        self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        connection
            .query_row(
                "SELECT record_key FROM operational_mutations
                 WHERE collection_name = ?1
                 ORDER BY id
                 LIMIT 1",
                [collection],
                |row| row.get::<_, String>(0),
            )
            .map(Some)
            .or_else(|err| match err {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                _ => Err(EngineError::Storage),
            })
    }

    #[doc(hidden)]
    pub fn configure_vector_kind_for_test(&self, kind: &str) -> Result<(), EngineError> {
        self.ensure_open()?;
        let mut connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_mut().ok_or(EngineError::Closing)?;
        connection
            .execute(
                "INSERT OR REPLACE INTO _fathomdb_vector_kinds(kind, profile, created_at)
                 VALUES(?1, ?2, 0)",
                params![kind, DEFAULT_VECTOR_PROFILE],
            )
            .map_err(|_| EngineError::Storage)?;
        Ok(())
    }

    /// Install the pre-Slice-23 inert vector-subobject shape for compatibility tests.
    ///
    /// The mutation and a matching projection-generation transition are atomic,
    /// so tests can exercise legacy reconciliation without manufacturing the
    /// declaration-digest corruption that Slice 40 must reject.
    #[cfg(debug_assertions)]
    #[doc(hidden)]
    pub fn set_legacy_projection_vector_declared_for_test(
        &self,
        name: &str,
    ) -> Result<(), EngineError> {
        self.ensure_open()?;
        let mut connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_mut().ok_or(EngineError::Closing)?;
        let tx = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| EngineError::Storage)?;
        let changed = tx
            .execute(
                "UPDATE _fathomdb_projection_registry SET vector_declared = 1 WHERE name = ?1",
                [name],
            )
            .map_err(|_| EngineError::Storage)?;
        if changed != 1 {
            return Err(EngineError::Storage);
        }
        projection_generation::transition(&tx, ProjectionGenerationOriginV1::Configuration)?;
        tx.commit().map_err(|_| EngineError::Storage)
    }

    /// Install the inert pre-Slice-23 FTS/vector subobject shape for tests.
    ///
    /// The named row must already be a plain filterable declaration. The
    /// registry mutation and matching generation transition are atomic, so the
    /// fixture exercises inert-shape handling without bypassing current
    /// projection-generation authority.
    #[cfg(any(debug_assertions, feature = "test-hooks"))]
    #[doc(hidden)]
    pub fn set_legacy_projection_search_subobjects_for_test(
        &self,
        name: &str,
    ) -> Result<(), EngineError> {
        self.ensure_open()?;
        let mut connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_mut().ok_or(EngineError::Closing)?;
        let tx = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| EngineError::Storage)?;
        let changed = tx
            .execute(
                "UPDATE _fathomdb_projection_registry \
                 SET fts_tokenizer='', vector_declared=1 \
                 WHERE name=?1 AND roles='filterable' \
                   AND fts_tokenizer IS NULL AND vector_declared=0",
                [name],
            )
            .map_err(|_| EngineError::Storage)?;
        if changed != 1 {
            return Err(EngineError::Storage);
        }
        projection_generation::transition(&tx, ProjectionGenerationOriginV1::Configuration)?;
        tx.commit().map_err(|_| EngineError::Storage)
    }

    /// OPP-12 Phase-1 (0.8.19 Slice 10) — read the writer connection's
    /// `PRAGMA secure_delete` (design §3 gap-4). `true` iff the standing
    /// connection-open PRAGMA is in effect, so `purge` freelist erasure is
    /// complete without a per-purge `VACUUM`.
    #[doc(hidden)]
    pub fn secure_delete_enabled_for_test(&self) -> Result<bool, EngineError> {
        self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        let value: i64 = connection
            .query_row("PRAGMA secure_delete", [], |r| r.get(0))
            .map_err(|_| EngineError::Storage)?;
        Ok(value != 0)
    }

    /// OPP-12 Phase-1 (0.8.19 Slice 10, design §3 gap-4) — `true` iff EVERY
    /// reader-pool connection reports `PRAGMA secure_delete = ON`. Broadcasts a
    /// per-worker probe; proves the standing flag is set on the non-writer
    /// connections (which perform projection/vector-rewrite DELETEs), closing
    /// the GDPR-erasure leak codex flagged.
    #[cfg(debug_assertions)]
    #[doc(hidden)]
    pub fn reader_secure_delete_enabled_for_test(&self) -> Result<bool, EngineError> {
        self.ensure_open()?;
        let per_worker = self.reader_pool.secure_delete_per_worker();
        if per_worker.is_empty() {
            return Err(EngineError::Storage);
        }
        Ok(per_worker.iter().all(|&v| v == 1))
    }

    /// OPP-12 Phase-1 (0.8.19 Slice 10, design §3 gap-4) — `true` iff a freshly
    /// opened projection/runtime connection (`open_runtime_connection`) reports
    /// `PRAGMA secure_delete = ON`. The runtime connection performs the
    /// vector-rewrite/projection DELETEs, so its freed pages must be scrubbed too.
    #[doc(hidden)]
    pub fn runtime_secure_delete_enabled_for_test(&self) -> Result<bool, EngineError> {
        self.ensure_open()?;
        let connection = open_runtime_connection(
            &self.path,
            #[cfg(any(test, feature = "test-hooks"))]
            ManagedConnectionCategory::RuntimeProbe,
            #[cfg(any(test, feature = "test-hooks"))]
            &self.managed_connections,
        )
        .map_err(|_| EngineError::Storage)?;
        let value: i64 = connection
            .query_row("PRAGMA secure_delete", [], |r| r.get(0))
            .map_err(|_| EngineError::Storage)?;
        Ok(value != 0)
    }

    /// EXP-S (0.8.14 Slice 5, D1) — write one canonical node row carrying an
    /// explicit structural `row_kind` (leaf/coverage/graph), routing the index
    /// projection through the SAME `row_kind -> index-target` dispatch seam
    /// (`project_canonical_node_row`) as the production `leaf` write path.
    ///
    /// This is the internal-only writer for `coverage`/`graph` rows (there is no
    /// public SDK surface for `row_kind` in 0.8.14). Cursor assignment preserves
    /// the `rowid == write_cursor == cursor` determinism identity. When the row
    /// projects into an async vector index, the worker pool is notified so the
    /// embed is scheduled exactly as for a normal write.
    #[doc(hidden)]
    pub fn write_canonical_row_with_kind_for_test(
        &self,
        kind: &str,
        body: &str,
        row_kind: RowKind,
    ) -> Result<WriteReceipt, EngineError> {
        self.ensure_open()?;
        let mut connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_mut().ok_or(EngineError::Closing)?;

        // R-20-E3 / design §4 item 6 — this writer BYPASSES `PreparedWrite`, so
        // the `SourceId` newtype cannot reach it; before 0.8.20 it inserted a
        // literal NULL `source_id` and produced a row that no `excise_source`
        // call could reach. Engine-derived rows instead take a reserved
        // `_engine:*` provenance, keyed by the structural role that produced
        // them, so they are both erasable and distinguishable from caller data.
        let engine_provenance = SourceId::engine_derived(row_kind.as_str());

        // 0.8.20 Slice 20c — same late enrolment the governed write path takes
        // (`Engine::batch_vector_kinds_needing_enrolment`), so this internal writer does not
        // silently diverge into the false-ready barrier for `coverage` rows. The
        // live-embedder precondition is checked here, as that caller does; the
        // `row_kind` gate keeps `graph` rows out of the vector registry.
        //
        // fix-2 (codex §9 [P2]) — including the un-stranding half, so this door
        // cannot diverge from the other one either. fix-5 (codex §9 round 4 [P2])
        // — and both halves commit as ONE transaction, via the same shared
        // `enrol_and_unstrand`.
        let unstranded = if self.usable_dense_runtime()
            && self.vector_kind_needs_enrolment(connection, kind, row_kind)?
        {
            self.enrol_and_unstrand(connection, &[kind])?
        } else {
            false
        };

        let cursor = self.next_cursor.load(Ordering::SeqCst).saturating_add(1);
        let enqueued = {
            let tx = connection.transaction().map_err(|_| EngineError::Storage)?;
            // 0.8.20 Slice 15b (TC-34) — this writer takes NO validity window, and
            // that is deliberate rather than an oversight. It is a `#[doc(hidden)]`
            // test-only writer for the internal `coverage`/`graph` row kinds, which
            // have no public SDK surface at all (see the doc comment above); the
            // caller-facing authoring path is `PreparedWrite::Node`, handled in
            // `commit_batch`. Omitting the columns binds NULL — the migration
            // step-22 default and the UNBOUNDED reading — so engine-derived rows
            // stay valid at every instant, which is the only correct answer for a
            // structural row that no caller can address a window to.
            tx.execute(
                "INSERT INTO canonical_nodes(write_cursor, kind, body, source_id, logical_id, row_kind)
                 VALUES(?1, ?2, ?3, ?4, NULL, ?5)",
                params![cursor, kind, body, engine_provenance.as_str(), row_kind.as_str()],
            )
            .map_err(|_| EngineError::Storage)?;
            let enqueued = project_canonical_node_row(
                &tx,
                cursor,
                kind,
                body,
                row_kind,
                ProjectionPass::Write,
                // This #[doc(hidden)] writer inserts with the column DEFAULT
                // `state = 'active'` (no state column in its INSERT), so the row
                // is always active and its attributes project.
                true,
            )
            .map_err(|_| EngineError::Storage)?;
            advance_projection_cursor(&tx).map_err(|_| EngineError::Storage)?;
            tx.commit().map_err(|_| EngineError::Storage)?;
            enqueued
        };
        self.next_cursor.store(cursor, Ordering::SeqCst);
        if enqueued || unstranded {
            self.projection_runtime.notify_new_work();
        }
        Ok(WriteReceipt { cursor, row_cursors: vec![cursor], dangling_edge_endpoints: 0 })
    }

    /// EXP-S (0.8.14 Slice 5, D1) — select the active canonical rows carrying a
    /// given `row_kind`, returning their `write_cursor`s in cursor order. Proves
    /// the engine can query/select rows by the structural `row_kind` axis.
    #[doc(hidden)]
    pub fn canonical_rows_with_row_kind_for_test(
        &self,
        row_kind: RowKind,
    ) -> Result<Vec<u64>, EngineError> {
        self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        let mut stmt = connection
            .prepare(
                "SELECT write_cursor FROM canonical_nodes
                 WHERE row_kind = ?1 AND superseded_at IS NULL
                 ORDER BY write_cursor",
            )
            .map_err(|_| EngineError::Storage)?;
        let cursors = stmt
            .query_map(params![row_kind.as_str()], |row| row.get::<_, u64>(0))
            .map_err(|_| EngineError::Storage)?
            .collect::<rusqlite::Result<Vec<u64>>>()
            .map_err(|_| EngineError::Storage)?;
        Ok(cursors)
    }

    #[doc(hidden)]
    pub fn write_vector_for_test(
        &self,
        kind: &str,
        text: &str,
    ) -> Result<WriteReceipt, EngineError> {
        self.ensure_open()?;
        let embedder =
            self.runtime_embedder.as_ref().cloned().ok_or(EngineError::EmbedderNotConfigured)?;

        let mut connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_mut().ok_or(EngineError::Closing)?;
        if !kind_is_vector_indexed(connection, kind)? {
            return Err(EngineError::KindNotVectorIndexed);
        }

        let expected = default_profile_dimension(connection)?;
        ensure_vector_partition(connection, expected).map_err(|_| EngineError::Storage)?;
        let vector = embedder.embed(text).map_err(map_runtime_embedder_error)?;
        let actual = u32::try_from(vector.len()).unwrap_or(u32::MAX);
        if actual != expected {
            return Err(EngineError::EmbedderDimensionMismatch { expected, actual });
        }

        let cursor = self.next_cursor.load(Ordering::SeqCst).saturating_add(1);
        // EU-5a2 mean-centering apply path (write side). f32 BLOB stored
        // is ALWAYS un-centered; the sign-quant input is the centered
        // vector iff the identity is MC-required AND a `mean_vec` is
        // pinned. NoopEmbedder identity (the only EU-5a2 live one) is
        // NOT MC-required, so this is a no-op until EU-5b's flip.
        let blob = encode_vector_blob(&vector);
        let bin_blob = if identity_requires_mean_centering(&self.runtime_embedder_identity) {
            match read_pinned_mean_vec(connection, self.runtime_embedder_identity.dimension)? {
                Some(mean) => encode_vector_blob(&subtract_mean(&vector, &mean)),
                None => blob.clone(),
            }
        } else {
            blob.clone()
        };
        let source_type = resolve_source_type(kind)?;
        let now_unix =
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64;

        // EU-5b — feed the streaming mean accumulator (if live) and detect
        // a threshold-crossing pin. The mean materialization, pre-pin
        // re-quantize, and `MeanVecPinned` event emission all happen in
        // the SAME SQLite transaction as the row INSERT.
        let pin_event = {
            let runtime = &self.projection_runtime.shared;
            let mut accumulator =
                runtime.mean_accumulator.lock().map_err(|_| EngineError::Storage)?;
            if let Some(acc) = accumulator.as_mut() {
                acc.add(&vector);
                if acc.count() >= MEAN_VEC_PIN_THRESHOLD {
                    let mean = acc.materialize();
                    *accumulator = None;
                    Some(mean)
                } else {
                    None
                }
            } else {
                None
            }
        };

        let tx = connection.transaction().map_err(|_| EngineError::Storage)?;
        tx.execute(
            "INSERT INTO _fathomdb_vector_rows(rowid, kind, write_cursor) VALUES(?1, ?2, ?3)",
            params![cursor, kind, cursor],
        )
        .map_err(|_| EngineError::Storage)?;
        // Slice 10 / G10 — `status` ships an empty-string sentinel only: vec0 TEXT
        // metadata columns are NOT NULL-able ("Expected text for TEXT metadata
        // column"), so the "no real population yet" state is `''`, not NULL.
        //
        // 0.8.20 Slice 15e — this test helper carries no JSON body, so every live
        // `filterable` `attr_<hex>` column binds the `''` sentinel (an empty body
        // extracts nothing). When the table has no attr columns the statement is
        // byte-identical to the shipped form.
        let (cols_sql, ph_sql, attr_vals) =
            vector_attr_insert_fragments(&tx, "", 7).map_err(|_| EngineError::Storage)?;
        let sql = format!(
            "INSERT INTO vector_default(
                rowid, embedding, embedding_bin, source_type, kind, created_at, status{cols_sql}
             ) VALUES(?1, ?2, vec_quantize_binary(?3), ?4, ?5, ?6, ''{ph_sql})"
        );
        let mut pv: Vec<rusqlite::types::Value> = vec![
            rusqlite::types::Value::Integer(cursor as i64),
            rusqlite::types::Value::Blob(blob.clone()),
            rusqlite::types::Value::Blob(bin_blob.clone()),
            rusqlite::types::Value::Text(source_type.to_string()),
            rusqlite::types::Value::Text(kind.to_string()),
            rusqlite::types::Value::Integer(now_unix),
        ];
        pv.extend(attr_vals);
        tx.execute(&sql, rusqlite::params_from_iter(pv.iter()))
            .map_err(|_| EngineError::Storage)?;

        let mut emitted_event: Option<EmbedderEvent> = None;
        if let Some(mean_vec) = pin_event {
            let mean_bytes = encode_vector_blob(&mean_vec);
            tx.execute(
                "UPDATE _fathomdb_embedder_profiles SET mean_vec = ?1 WHERE profile = 'default'",
                params![mean_bytes],
            )
            .map_err(|_| EngineError::Storage)?;
            // Read all pre-pin (rowid, embedding) and re-quantize within
            // the same tx. The just-inserted row above is also covered.
            let rows: Vec<(i64, Vec<u8>)> = {
                let mut statement = tx
                    .prepare("SELECT rowid, embedding FROM vector_default ORDER BY rowid")
                    .map_err(|_| EngineError::Storage)?;
                let mapped = statement
                    .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Vec<u8>>(1)?)))
                    .map_err(|_| EngineError::Storage)?;
                let mut out = Vec::new();
                for r in mapped {
                    out.push(r.map_err(|_| EngineError::Storage)?);
                }
                out
            };
            let (doc_count, _) = run_pin_and_requantize_pass(&tx, &rows, &mean_vec)?;
            emitted_event = Some(EmbedderEvent::MeanVecPinned {
                dim: u32::try_from(mean_vec.len()).unwrap_or(u32::MAX),
                doc_count,
            });
        }

        tx.commit().map_err(|_| EngineError::Storage)?;

        if let Some(ev) = emitted_event {
            if let Ok(mut events) = self.projection_runtime.shared.pending_events.lock() {
                events.push(ev);
            }
        }

        self.next_cursor.store(cursor, Ordering::SeqCst);
        // G8 — this path (embedder-profile pin) commits no canonical edges, so
        // no endpoint can dangle.
        Ok(WriteReceipt { cursor, row_cursors: vec![cursor], dangling_edge_endpoints: 0 })
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

    /// 0.7.2 PR-2b — NON-test observation seam. Drains and returns every
    /// `EmbedderEvent` queued since the last drain (mean pin, manual mean
    /// recompute). Production callers use
    /// this to observe the synchronous recompute work; events are queued
    /// only AFTER the recompute transaction is durable, so a rolled-back
    /// recompute never surfaces. Mirrors the at-open
    /// `OpenReport.embedder_events` channel for the steady-state path.
    pub fn drain_embedder_events(&self) -> Result<Vec<EmbedderEvent>, EngineError> {
        self.ensure_open()?;
        let mut events = self
            .projection_runtime
            .shared
            .pending_events
            .lock()
            .map_err(|_| EngineError::Storage)?;
        Ok(std::mem::take(&mut *events))
    }

    /// Test seam that raises vector-candidate fanout for recall tests.
    ///
    /// This does not alter the caller-requested final result limit or the
    /// caller-visible result cardinality. Production uses the default and
    /// never consults an environment variable.
    #[doc(hidden)]
    pub fn set_search_limit_for_test(&self, limit: usize) {
        self.projection_runtime.shared.search_limit_override.store(limit, Ordering::SeqCst);
    }

    /// Slice 10 / G12-recency test seam — flip the dedicated recency-reweight
    /// flag (off by default). The reweight runs AFTER bit-KNN on the fused hits;
    /// it is never a vec0 predicate and is NOT `fusion_mode`.
    #[doc(hidden)]
    pub fn set_recency_reweight_enabled_for_test(&self, enabled: bool) {
        self.projection_runtime.shared.recency_reweight_enabled.store(enabled, Ordering::SeqCst);
    }

    /// 0.8.16 Slice 5 / F9 test seam — flip the dedicated importance/confidence
    /// reweight flag (off by default). The reweight runs AFTER bit-KNN + RRF on
    /// the fused hits (multiplicative-on-fused, `NULL ⇒ neutral`); it is never a
    /// vec0 predicate and is NOT `fusion_mode`. Mirrors
    /// `set_recency_reweight_enabled_for_test`.
    #[doc(hidden)]
    pub fn set_importance_reweight_enabled_for_test(&self, enabled: bool) {
        self.projection_runtime.shared.importance_reweight_enabled.store(enabled, Ordering::SeqCst);
    }

    /// 0.8.16 Slice 5 / F9 (R-F9-1) — set the caller-supplied `importance` ranking
    /// scalar on the `canonical_nodes` row identified by `write_cursor` (the
    /// interim id `SearchHit.id` carries). Validates `importance ∈ [0.0, 1.0]`,
    /// mirroring the existing `canonical_edges.confidence` write-path check —
    /// an out-of-range value is a deterministic [`EngineError::WriteValidation`].
    ///
    /// The 3-way sentinel: NOT calling this leaves the column `NULL` (never
    /// assigned = graceful-absent, ranks NEUTRAL); `0.0` is the explicit floor;
    /// `(0.0, 1.0]` is an explicit importance. Importance is a caller-supplied
    /// scalar — the engine does NOT compute graph-centrality importance (ADR §4
    /// non-goal). Engine-internal minimal surface for this keystone; SDK (Py/TS)
    /// exposure is a Slice-40 concern.
    pub fn write_node_importance(
        &self,
        write_cursor: u64,
        importance: f64,
    ) -> Result<(), EngineError> {
        if !importance.is_finite() || !(0.0..=1.0).contains(&importance) {
            return Err(EngineError::WriteValidation);
        }
        self.ensure_open()?;
        let mut connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_mut().ok_or(EngineError::Closing)?;
        dependency_closure::maintain_before_writer(connection)?;
        let tx = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| EngineError::Storage)?;
        dependency_closure::guard_no_pending_physical(&tx)?;
        tx.execute(
            "UPDATE canonical_nodes SET importance = ?1 WHERE write_cursor = ?2",
            params![importance, write_cursor],
        )
        .map_err(|_| EngineError::Storage)?;
        tx.commit().map_err(|_| EngineError::Storage)?;
        Ok(())
    }

    /// 0.8.16 Slice 5 / F9 (R-F9-1) — read back the `importance` scalar for the
    /// `canonical_nodes` row identified by `write_cursor`. `None` = SQL `NULL` =
    /// never assigned (graceful-absent). The reciprocal read for
    /// [`Engine::write_node_importance`].
    pub fn node_importance(&self, write_cursor: u64) -> Result<Option<f64>, EngineError> {
        self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        let eligibility =
            dependency_closure::read_eligibility_sql("canonical_nodes", false, false, false, 2);
        connection
            .query_row(
                &format!(
                    "SELECT importance FROM canonical_nodes \
                     WHERE write_cursor = ?1{eligibility} LIMIT 1"
                ),
                params![write_cursor, current_epoch_seconds()],
                |r| r.get::<_, Option<f64>>(0),
            )
            .optional()
            .map(Option::flatten)
            .map_err(|_| EngineError::Storage)
    }

    /// GA-2 / Slice-40 (◆ B-1) measurement seam — make `search()` return the
    /// pre-fusion VECTOR-branch ranking (the ANN+ bit-KNN K=192 + f32 rerank
    /// signal) instead of the unconditional RRF-fused result, so the eu7 recall
    /// gate (AC-075) can measure ANN-quantization FIDELITY — vector top-10 vs
    /// the exact-f32 VECTOR top-10 ground truth — in isolation. Off by default;
    /// never set on any production path. This is NOT a `fusion_mode` knob:
    /// production RRF fusion stays unconditional and `fuse_rrf`/`rerank_fused`/
    /// recency are unchanged. Mirrors `set_recency_reweight_enabled_for_test`
    /// (release-available, since eu7 runs in `--release`).
    #[doc(hidden)]
    pub fn set_vector_stage_only_for_test(&self, enabled: bool) {
        self.projection_runtime.shared.vector_stage_only_for_test.store(enabled, Ordering::SeqCst);
    }

    /// 0.7.2 PR-2b test seam — arm a one-shot fault inside the NEXT
    /// `recompute_mean` so it errors after the `mean_vec` UPDATE but before
    /// the re-quantize completes. Proves the recompute tx rolls back whole.
    #[doc(hidden)]
    #[cfg(debug_assertions)]
    pub fn force_next_recompute_failure_for_test(&self) {
        self.projection_runtime.shared.force_recompute_failure.store(true, Ordering::SeqCst);
    }

    #[doc(hidden)]
    pub fn vector_row_count_for_test(&self) -> Result<u64, EngineError> {
        self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        connection
            .query_row("SELECT COUNT(*) FROM vector_default", [], |row| row.get::<_, u64>(0))
            .map_err(|_| EngineError::Storage)
    }

    /// Test-only distinction between a physical vec0 row and its terminal
    /// readiness record. Closure may terminalize an ineligible projection
    /// without publishing vector bytes, so readiness alone is not a row oracle.
    #[doc(hidden)]
    pub fn has_vector_row_for_cursor_for_test(&self, cursor: u64) -> Result<bool, EngineError> {
        self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM vector_default WHERE rowid = ?1)",
                [i64::try_from(cursor).map_err(|_| EngineError::Storage)?],
                |row| row.get::<_, bool>(0),
            )
            .map_err(|_| EngineError::Storage)
    }

    #[doc(hidden)]
    pub fn read_vector_blob_for_test(&self, rowid: i64) -> Result<Vec<u8>, EngineError> {
        self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        connection
            .query_row("SELECT embedding FROM vector_default WHERE rowid = ?1", [rowid], |row| {
                row.get::<_, Vec<u8>>(0)
            })
            .map_err(|_| EngineError::Storage)
    }

    /// 0.8.20 Slice 15e — read a row's raw `embedding_bin` blob bytes (the
    /// sign-quantized vector). Used to prove the non-destructive reshape copies the
    /// bits VERBATIM (condition #4): the pre-reshape and post-reshape bytes must be
    /// byte-identical.
    #[doc(hidden)]
    pub fn read_vector_bin_for_test(&self, rowid: i64) -> Result<Vec<u8>, EngineError> {
        self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        connection
            .query_row(
                "SELECT embedding_bin FROM vector_default WHERE rowid = ?1",
                [rowid],
                |row| row.get::<_, Vec<u8>>(0),
            )
            .map_err(|_| EngineError::Storage)
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

    /// Doctor read-only integrity report. Three-section output per
    /// AC-043a/b. `opts.full` adds `PRAGMA integrity_check`. `quick` and
    /// `round_trip` are accepted but treated as default for 0.6.0.
    #[cfg(feature = "operator")]
    pub fn check_integrity(
        &self,
        opts: CheckIntegrityOpts,
    ) -> Result<IntegrityReport, EngineError> {
        self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        Ok(IntegrityReport {
            physical: physical_section(connection, opts.full),
            logical: logical_section(connection),
            semantic: semantic_section(connection),
        })
    }

    /// Doctor bit-preserving export. Runs `VACUUM INTO` to produce a
    /// self-contained SQLite file at `out`, computes SHA-256 of the
    /// resulting bytes, and writes a JSON manifest at `manifest`. Per
    /// AC-039a/b.
    #[cfg(feature = "operator")]
    pub fn safe_export(
        &self,
        out: &Path,
        manifest: &Path,
    ) -> Result<SafeExportArtifact, EngineError> {
        self.ensure_open()?;
        {
            let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
            let connection = connection.as_ref().ok_or(EngineError::Closing)?;
            let target = out.to_string_lossy().to_string();
            connection
                .execute("VACUUM INTO ?1", params![target])
                .map_err(|_| EngineError::Storage)?;
        }
        let bytes = std::fs::read(out).map_err(|_| EngineError::Storage)?;
        let digest = sha2::Sha256::digest(&bytes);
        let sha256_hex = hex_encode(digest.as_slice());
        let export_abs = out.canonicalize().unwrap_or_else(|_| out.to_path_buf());
        let manifest_json = serde_json::json!({
            "export_path": export_abs.to_string_lossy(),
            "sha256": sha256_hex,
            "byte_count": bytes.len() as u64,
        });
        let manifest_bytes =
            serde_json::to_vec_pretty(&manifest_json).map_err(|_| EngineError::Storage)?;
        std::fs::write(manifest, &manifest_bytes).map_err(|_| EngineError::Storage)?;
        Ok(SafeExportArtifact {
            export_path: out.to_path_buf(),
            manifest_path: manifest.to_path_buf(),
            manifest_sha256: sha256_hex,
        })
    }

    /// Phase 9 Pack B / AC-042 source trace. Returns the canonical-row
    /// id set produced by `source_id`, ordered by `write_cursor`. Empty
    /// string is not a valid `source_id`; rows with NULL `source_id`
    /// are excluded from every result.
    #[cfg(feature = "operator")]
    pub fn trace_source_ref(&self, source_id: &str) -> Result<TraceReport, EngineError> {
        self.ensure_open()?;
        if source_id.is_empty() {
            return Err(EngineError::WriteValidation);
        }
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;

        let mut events: Vec<TraceEvent> = Vec::new();
        let mut nodes = connection
            .prepare(
                "SELECT write_cursor, kind FROM canonical_nodes WHERE source_id = ?1
                 ORDER BY write_cursor",
            )
            .map_err(|_| EngineError::Storage)?;
        let node_rows = nodes
            .query_map([source_id], |row| {
                Ok(TraceEvent {
                    write_cursor: row.get::<_, i64>(0)? as u64,
                    kind: row.get::<_, String>(1)?,
                    table: "canonical_nodes",
                })
            })
            .map_err(|_| EngineError::Storage)?;
        for row in node_rows {
            events.push(row.map_err(|_| EngineError::Storage)?);
        }

        let mut edges = connection
            .prepare(
                "SELECT write_cursor, kind FROM canonical_edges WHERE source_id = ?1
                 ORDER BY write_cursor",
            )
            .map_err(|_| EngineError::Storage)?;
        let edge_rows = edges
            .query_map([source_id], |row| {
                Ok(TraceEvent {
                    write_cursor: row.get::<_, i64>(0)? as u64,
                    kind: row.get::<_, String>(1)?,
                    table: "canonical_edges",
                })
            })
            .map_err(|_| EngineError::Storage)?;
        for row in edge_rows {
            events.push(row.map_err(|_| EngineError::Storage)?);
        }

        events.sort_by_key(|e| e.write_cursor);
        Ok(TraceReport { source_ref: source_id.to_string(), events })
    }

    /// Trace one reciprocal source-to-derived dependency page under an authenticated frozen view.
    ///
    /// The read is one SQLite snapshot, never mutates durable state, and returns no partial page.
    pub fn trace_dependency(
        &self,
        request: DependencyTraceRequestV1,
    ) -> Result<DependencyTraceResultV1, EngineError> {
        self.ensure_open()?;
        let mut connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_mut().ok_or(EngineError::Closing)?;
        dependency_trace::execute(connection, request)
    }

    /// Query-plan details for both indexed dependency-trace directions.
    #[cfg(feature = "test-hooks")]
    pub fn dependency_trace_query_plans_for_test(&self) -> Result<Vec<String>, EngineError> {
        self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        let candidate_queries = dependency_trace::candidate_queries_for_test();
        let mut plans = Vec::new();
        {
            let sql = format!("EXPLAIN QUERY PLAN {}", candidate_queries[0]);
            let mut statement = connection.prepare(&sql).map_err(|_| EngineError::Storage)?;
            plans.extend(
                statement
                    .query_map(
                        rusqlite::params![
                            "derived-r1",
                            2_i64,
                            false,
                            false,
                            false,
                            1_i64,
                            Option::<&str>::None,
                            Option::<&str>::None,
                            Option::<i64>::None,
                            Option::<&str>::None,
                            "[]",
                        ],
                        |row| row.get::<_, String>(3),
                    )
                    .map_err(|_| EngineError::Storage)?
                    .collect::<rusqlite::Result<Vec<_>>>()
                    .map_err(|_| EngineError::Storage)?,
            );
        }
        {
            let sql = format!("EXPLAIN QUERY PLAN {}", candidate_queries[1]);
            let mut statement = connection.prepare(&sql).map_err(|_| EngineError::Storage)?;
            plans.extend(
                statement
                    .query_map(
                        rusqlite::params![
                            "source-r1",
                            "",
                            2_i64,
                            false,
                            false,
                            false,
                            1_i64,
                            Option::<&str>::None,
                            Option::<&str>::None,
                            Option::<i64>::None,
                            Option::<&str>::None,
                            "[]",
                        ],
                        |row| row.get::<_, String>(3),
                    )
                    .map_err(|_| EngineError::Storage)?
                    .collect::<rusqlite::Result<Vec<_>>>()
                    .map_err(|_| EngineError::Storage)?,
            );
        }
        Ok(plans)
    }

    /// Candidate SQL used by both bounded dependency-trace directions.
    #[cfg(feature = "test-hooks")]
    #[doc(hidden)]
    pub fn dependency_trace_candidate_queries_for_test(&self) -> [&'static str; 2] {
        dependency_trace::candidate_queries_for_test()
    }

    /// Query-plan details for bounded Slice 55 integrity owner and physical scans.
    #[cfg(all(feature = "operator", feature = "test-hooks"))]
    pub fn data_plane_integrity_query_plans_for_test(&self) -> Result<Vec<String>, EngineError> {
        self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        let statements = data_plane_integrity::candidate_queries_for_test();
        let mut plans = Vec::new();
        for sql in statements {
            let mut statement = connection
                .prepare(&format!("EXPLAIN QUERY PLAN {sql}"))
                .map_err(|_| EngineError::Storage)?;
            plans.extend(
                statement
                    .query_map(rusqlite::params![0_i64, 101_i64], |row| row.get::<_, String>(3))
                    .map_err(|_| EngineError::Storage)?
                    .collect::<rusqlite::Result<Vec<_>>>()
                    .map_err(|_| EngineError::Storage)?,
            );
        }
        Ok(plans)
    }

    /// Candidate SQL executed by the bounded Slice 55 integrity scans.
    #[cfg(all(feature = "operator", feature = "test-hooks"))]
    #[doc(hidden)]
    pub fn data_plane_integrity_candidate_queries_for_test(&self) -> [&'static str; 12] {
        data_plane_integrity::candidate_queries_for_test()
    }

    /// Measure SQLite VM-step quanta, wall time, and process peak-RSS delta.
    #[cfg(feature = "test-hooks")]
    pub fn measure_dependency_trace_for_test(
        &self,
    ) -> Result<DependencyTraceMeasurement, EngineError> {
        let context = self.freeze_read_context(&ReadContextV1::new(
            ReadView { valid_as_of: Some(1), ..ReadView::default() },
            SearchFilter::default(),
        )?)?;
        let request = DependencyTraceRequestV1::new(
            "source-r1",
            DependencyTraceDirectionV1::ToDependents,
            context,
        )?
        .with_bounds(1, 2)?;
        let callbacks = Arc::new(AtomicU64::new(0));
        let callback_counter = Arc::clone(&callbacks);
        let mut connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_mut().ok_or(EngineError::Closing)?;
        connection
            .progress_handler(
                1_000,
                Some(move || {
                    callback_counter.fetch_add(1, Ordering::Relaxed).saturating_add(1) > 10_000
                }),
            )
            .map_err(|_| EngineError::Storage)?;
        let rss_before = process_peak_rss_bytes();
        let started = Instant::now();
        let outcome = dependency_trace::execute(connection, request);
        let elapsed = started.elapsed();
        let rss_after = process_peak_rss_bytes();
        connection
            .progress_handler(0, Option::<fn() -> bool>::None)
            .map_err(|_| EngineError::Storage)?;
        let (response_bytes, bound_exceeded) = match outcome {
            Ok(result) => (encode_dependency_trace_result_v1(&result)?, false),
            Err(EngineError::DependencyTrace(error))
                if error.reason == DependencyTraceErrorReasonV1::TraceBoundExceeded =>
            {
                (Vec::new(), true)
            }
            Err(error) => return Err(error),
        };
        Ok(DependencyTraceMeasurement {
            vm_steps: callbacks.load(Ordering::Relaxed).saturating_mul(1_000),
            elapsed,
            peak_rss_delta_bytes: rss_after.saturating_sub(rss_before),
            response_bytes,
            bound_exceeded,
        })
    }

    /// Seed a real hidden-dependent performance fixture outside measurement.
    #[cfg(feature = "test-hooks")]
    #[doc(hidden)]
    pub fn seed_hidden_dependency_trace_fixture_for_test(
        &self,
        count: u32,
    ) -> Result<(), EngineError> {
        self.ensure_open()?;
        let mut connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_mut().ok_or(EngineError::Closing)?;
        let tx = connection.transaction().map_err(|_| EngineError::Storage)?;
        {
            let mut nodes = tx
                .prepare_cached(
                    "INSERT INTO canonical_nodes(write_cursor,kind,body,source_id,state) \
                     VALUES(?1,'hidden','hidden','slice55-source','deleted')",
                )
                .map_err(|_| EngineError::Storage)?;
            let mut owners = tx
                .prepare_cached(
                    "INSERT INTO _fathomdb_artifact_revisions(\
                         schema_version,revision_id,artifact_class,write_cursor,artifact_role,completeness) \
                     VALUES(1,?1,'node',?2,'derived_semantic','complete')",
                )
                .map_err(|_| EngineError::Storage)?;
            let mut links = tx
                .prepare_cached(
                    "INSERT INTO _fathomdb_source_links(\
                         schema_version,artifact_revision_id,source_id,source_version_id,\
                         source_revision_id,locator_kind,start_byte,end_byte,hash_algorithm,hash_digest) \
                     VALUES(1,?1,'slice55-source','slice55-v1','source-r1','whole_body',\
                            NULL,NULL,'sha256',?2)",
                )
                .map_err(|_| EngineError::Storage)?;
            let mut dependencies = tx
                .prepare_cached(
                    "INSERT INTO _fathomdb_source_dependencies(\
                         schema_version,dependency_id,derived_revision_id,registered_dependency_generation) \
                     VALUES(1,?1,?2,1)",
                )
                .map_err(|_| EngineError::Storage)?;
            for index in 0..count {
                // Negative corrupt-fixture cursors remain below the source's
                // real high-water mark, preserving the observable boundary.
                let cursor = -i64::from(index) - 1;
                let revision = format!("hidden-r{index:05}");
                let dependency = format!("hidden-dep-{index:05}");
                nodes.execute([cursor]).map_err(|_| EngineError::Storage)?;
                owners
                    .execute(rusqlite::params![revision, cursor])
                    .map_err(|_| EngineError::Storage)?;
                links
                    .execute(rusqlite::params![revision, "0".repeat(64)])
                    .map_err(|_| EngineError::Storage)?;
                dependencies
                    .execute(rusqlite::params![dependency, revision])
                    .map_err(|_| EngineError::Storage)?;
            }
        }
        tx.commit().map_err(|_| EngineError::Storage)
    }

    /// Run bounded, read-only operator integrity checks in one SQLite snapshot.
    #[cfg(feature = "operator")]
    pub fn check_data_plane_integrity(
        &self,
        request: DataPlaneIntegrityRequestV1,
    ) -> Result<DataPlaneIntegrityResultV1, EngineError> {
        self.ensure_open()?;
        let mut connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_mut().ok_or(EngineError::Closing)?;
        data_plane_integrity::execute(connection, request)
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

    /// Doctor `verify-embedder` seam (AC-040a). Compares the
    /// `_fathomdb_embedder_profiles` row to the operator-supplied
    /// `name:revision` identity + dimension; never raises on mismatch.
    #[cfg(feature = "operator")]
    pub fn verify_embedder(
        &self,
        supplied_identity: &str,
        supplied_dimension: u32,
    ) -> Result<VerifyEmbedderReport, EngineError> {
        self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        let stored = load_default_profile(connection).map_err(|_| EngineError::Storage)?;
        let stored_identity = format!("{}:{}", stored.name, stored.revision);
        let identity_match = stored_identity == supplied_identity;
        let dimension_match = stored.dimension == supplied_dimension;
        let status = match (identity_match, dimension_match) {
            (true, true) => VerifyEmbedderStatus::Match,
            (false, true) => VerifyEmbedderStatus::IdentityMismatch,
            (true, false) => VerifyEmbedderStatus::DimensionMismatch,
            (false, false) => VerifyEmbedderStatus::BothMismatch,
        };
        Ok(VerifyEmbedderReport {
            stored_identity,
            stored_dimension: stored.dimension,
            supplied_identity: supplied_identity.to_string(),
            supplied_dimension,
            status,
        })
    }

    /// Doctor `dump-schema` seam (AC-040a). Returns the
    /// `PRAGMA user_version` sentinel plus the table + index inventory
    /// from `sqlite_schema`, excluding `sqlite_*` internal rows.
    /// Canonical tables appear first per [`CANONICAL_TABLES`].
    #[cfg(feature = "operator")]
    pub fn dump_schema(&self) -> Result<DumpSchemaReport, EngineError> {
        self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        let user_version: u32 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(|_| EngineError::Storage)?;
        let tables = read_schema_objects(connection, "table")?;
        let indexes = read_schema_objects(connection, "index")?;
        Ok(DumpSchemaReport { user_version, tables: order_canonical_first(tables), indexes })
    }

    /// Doctor `dump-row-counts` seam (AC-040a). Emits canonical-table
    /// counts only; projection / FTS / vec0 shadow tables are excluded.
    #[cfg(feature = "operator")]
    pub fn dump_row_counts(&self) -> Result<DumpRowCountsReport, EngineError> {
        self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        let mut counts = Vec::with_capacity(CANONICAL_TABLES.len());
        for name in CANONICAL_TABLES {
            let rows: u64 = connection
                .query_row(&format!("SELECT COUNT(*) FROM {name}"), [], |row| row.get(0))
                .map_err(|_| EngineError::Storage)?;
            counts.push(TableRowCount { name: (*name).to_string(), rows });
        }
        Ok(DumpRowCountsReport { counts })
    }

    /// 0.8.20 Slice 5d (R-20-E8, design §4 item 11) — doctor
    /// `orphan-provenance` seam: a **read-only** per-`source_id` census over
    /// `canonical_nodes` + `canonical_edges`.
    ///
    /// Answers the operator question the erasure work made askable: *"for this
    /// database, is every row actually reachable by some erasure verb?"* A row
    /// is reachable by `erase_source` / `excise_source` via `source_id`, or —
    /// **if it is a NODE** — by `purge` via `logical_id`. A row with neither is
    /// un-erasable, and is counted into
    /// [`OrphanProvenanceReport::unerasable_rows`].
    ///
    /// The node/edge asymmetry is load-bearing and mirrors migration step 21:
    /// an EDGE's `logical_id` is a supersession identity only and confers no
    /// purge-addressability, so a NULL-`source_id` edge is un-erasable however
    /// governed it looks. See the query comment below.
    ///
    /// CLI-only (no SDK parity), matching the `dump-*` diagnostic family.
    ///
    /// Read-only by construction: this method issues SELECTs exclusively and
    /// opens no transaction.
    #[cfg(feature = "operator")]
    pub fn orphan_provenance(&self) -> Result<OrphanProvenanceReport, EngineError> {
        self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;

        // One UNION ALL over both canonical tables so a source that spans nodes
        // AND edges reports as a single bucket.
        //
        // TWO DIFFERENT SUMS, and the difference is the whole point:
        //
        // * `governed` counts `logical_id` carriers — a reporting figure;
        // * `purge_addressable` counts rows that `purge` can actually reach,
        //   and it is NODE-ONLY (the edge arm contributes a literal 0).
        //
        // This is the same node/edge asymmetry migration step 21 carries, for
        // the same reason, and the two must stay in step: `purge_inner`
        // resolves its target exclusively through `canonical_nodes` (`SELECT
        // state FROM canonical_nodes WHERE logical_id = ?1`) and then erases
        // edges by ENDPOINT (`from_id`/`to_id`). It NEVER resolves an edge by
        // edge `logical_id` — an edge `logical_id` is only a SUPERSESSION
        // identity and confers no purge-addressability whatsoever.
        //
        // Crediting an edge's `logical_id` here made the diagnostic subtract
        // exactly the rows it exists to find: a NULL-`source_id` edge is
        // reachable by no erasure verb at all, yet `orphan-provenance` would
        // exit CLEAN on precisely the legacy/corrupt shape step 21 closes.
        // False assurance from a governance verb is worse than no verb.
        // (codex §9 [P2]; `null_source_governed_edge_counts_as_unerasable`.)
        let mut stmt = connection
            .prepare(
                "SELECT source_id,
                        COUNT(*) AS rows_total,
                        SUM(CASE WHEN logical_id IS NOT NULL THEN 1 ELSE 0 END) AS governed,
                        SUM(purge_addressable) AS purge_addressable
                   FROM (SELECT source_id,
                                logical_id,
                                CASE WHEN logical_id IS NOT NULL THEN 1 ELSE 0 END
                                    AS purge_addressable
                           FROM canonical_nodes
                         UNION ALL
                         SELECT source_id, logical_id, 0 AS purge_addressable
                           FROM canonical_edges)
                  GROUP BY source_id
                  ORDER BY rows_total DESC, source_id",
            )
            .map_err(|_| EngineError::Storage)?;

        let rows = stmt
            .query_map([], |row| {
                let source_id: Option<String> = row.get(0)?;
                let rows: i64 = row.get(1)?;
                let governed: i64 = row.get(2)?;
                let purge_addressable: i64 = row.get(3)?;
                Ok((source_id, rows, governed, purge_addressable))
            })
            .map_err(|_| EngineError::Storage)?;

        let mut sources = Vec::new();
        let mut total_rows: u64 = 0;
        let mut unerasable_rows: u64 = 0;
        for row in rows {
            let (source_id, rows, governed, purge_addressable) =
                row.map_err(|_| EngineError::Storage)?;
            let rows = u64::try_from(rows).unwrap_or(0);
            let governed_rows = u64::try_from(governed).unwrap_or(0);
            let purge_addressable = u64::try_from(purge_addressable).unwrap_or(0);
            total_rows = total_rows.saturating_add(rows);
            if source_id.is_none() {
                // No provenance: only the PURGE-ADDRESSABLE subset (governed
                // NODES) is reachable. The remainder — including every governed
                // EDGE, whose `logical_id` reaches nothing — is reachable by no
                // erasure verb at all.
                unerasable_rows =
                    unerasable_rows.saturating_add(rows - purge_addressable.min(rows));
            }
            let reserved = source_id.as_deref().is_some_and(|s| s.starts_with('_'));
            sources.push(OrphanProvenanceSource { source_id, rows, governed_rows, reserved });
        }

        Ok(OrphanProvenanceReport { sources, total_rows, unerasable_rows })
    }

    /// Doctor `dump-profile` seam (AC-040a). Returns the stored
    /// embedder identity + dimension plus the registered vectorized
    /// kinds from `_fathomdb_vector_kinds`.
    #[cfg(feature = "operator")]
    pub fn dump_profile(&self) -> Result<DumpProfileReport, EngineError> {
        self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        let stored = load_default_profile(connection).map_err(|_| EngineError::Storage)?;
        let mut stmt = connection
            .prepare("SELECT kind FROM _fathomdb_vector_kinds ORDER BY kind")
            .map_err(|_| EngineError::Storage)?;
        let rows =
            stmt.query_map([], |row| row.get::<_, String>(0)).map_err(|_| EngineError::Storage)?;
        let mut vectorized_kinds = Vec::new();
        for row in rows {
            vectorized_kinds.push(row.map_err(|_| EngineError::Storage)?);
        }
        Ok(DumpProfileReport {
            embedder_identity: format!("{}:{}", stored.name, stored.revision),
            embedder_dimension: stored.dimension,
            vectorized_kinds,
        })
    }

    /// Recover `--truncate-wal` seam. Runs
    /// `PRAGMA wal_checkpoint(TRUNCATE)` and returns the three counters
    /// SQLite reports. `status = Busy` when SQLite signalled a blocked
    /// checkpoint (`busy != 0`); the WAL may still be partially
    /// checkpointed in that case.
    #[cfg(feature = "operator")]
    pub fn truncate_wal(&self) -> Result<TruncateWalReport, EngineError> {
        self.ensure_open()?;
        // The operator verb keeps SQLite's own busy handler: `recover
        // --truncate-wal` is an explicit, foreground operator act, so waiting out
        // a transient reader is the helpful behaviour.
        self.wal_checkpoint_truncate_once(true)
    }

    /// One `PRAGMA wal_checkpoint(TRUNCATE)` on the writer connection.
    ///
    /// NOT operator-gated: the erasure verbs (`purge` is a default-feature verb)
    /// need it too, and a `#[cfg(feature = "operator")]` helper would break the
    /// default build. Acquires the connection mutex, so callers must NOT already
    /// hold it — every erasure verb calls this AFTER its transaction has
    /// committed and the guard has been dropped.
    ///
    /// `honor_busy_timeout = false` suppresses SQLite's busy handler for the
    /// duration of the checkpoint. rusqlite installs a **5 s** default
    /// `busy_timeout`, so a blocked checkpoint sits for 5 s before reporting
    /// `busy` — under the erasure verbs' bounded retry that compounds to a ~25 s
    /// stall on a verb that is supposed to fail fast. The erasure path therefore
    /// takes the immediate `busy` answer and runs its OWN short backoff; the
    /// prior value is restored before returning, on every path.
    fn wal_checkpoint_truncate_once(
        &self,
        honor_busy_timeout: bool,
    ) -> Result<TruncateWalReport, EngineError> {
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;

        let restore_timeout_ms: Option<i64> = if honor_busy_timeout {
            None
        } else {
            let previous: i64 = connection
                .query_row("PRAGMA busy_timeout", [], |row| row.get(0))
                .map_err(|_| EngineError::Storage)?;
            connection.busy_timeout(Duration::ZERO).map_err(|_| EngineError::Storage)?;
            Some(previous)
        };

        let checkpoint: rusqlite::Result<(i64, i64, i64)> =
            connection.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            });

        if let Some(previous) = restore_timeout_ms {
            let previous = u64::try_from(previous.max(0)).unwrap_or(0);
            connection
                .busy_timeout(Duration::from_millis(previous))
                .map_err(|_| EngineError::Storage)?;
        }

        let (busy, log_frames, checkpointed_frames) =
            checkpoint.map_err(|_| EngineError::Storage)?;
        let status = if busy == 0 { TruncateWalStatus::Done } else { TruncateWalStatus::Busy };
        Ok(TruncateWalReport {
            status,
            busy: busy.max(0) as u32,
            log_frames: log_frames.max(0) as u32,
            checkpointed_frames: checkpointed_frames.max(0) as u32,
            discarded_corrupt_wal: false,
        })
    }

    fn ensure_open(&self) -> Result<(), EngineError> {
        if self.closed.load(Ordering::SeqCst) {
            return Err(EngineError::Closing);
        }

        Ok(())
    }
}

fn restore_registered_derived_projections(
    tx: &Connection,
    cursor: i64,
    kind: &str,
    body: &str,
    row_kind: &str,
) -> Result<bool, EngineError> {
    let registered: bool = tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM _fathomdb_artifact_revisions r \
               JOIN _fathomdb_source_dependencies d ON d.derived_revision_id=r.revision_id \
               WHERE r.artifact_class='node' AND r.write_cursor=?1)",
            [cursor],
            |row| row.get(0),
        )
        .map_err(|_| EngineError::Storage)?;
    if !registered {
        return Ok(false);
    }
    let legacy_fts: Vec<(String, String)> = tx
        .prepare("SELECT body,kind FROM search_index WHERE write_cursor=?1")
        .and_then(|mut statement| {
            statement.query_map([cursor], |row| Ok((row.get(0)?, row.get(1)?)))?.collect()
        })
        .map_err(|_| EngineError::Storage)?;
    let fielded_fts: Vec<(String, String, String)> = tx
        .prepare("SELECT kind,body,status FROM search_index_v2 WHERE write_cursor=?1")
        .and_then(|mut statement| {
            statement
                .query_map([cursor], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
                .collect()
        })
        .map_err(|_| EngineError::Storage)?;
    let expected_status: String = tx
        .query_row(
            "SELECT CASE WHEN json_valid(?1) \
               THEN COALESCE(json_extract(?1,'$.status'),'') ELSE '' END",
            [body],
            |row| row.get(0),
        )
        .map_err(|_| EngineError::Storage)?;
    match (legacy_fts.as_slice(), fielded_fts.as_slice()) {
        ([(stored_body, stored_kind)], [(stored_kind_v2, stored_body_v2, stored_status)])
            if stored_body == body
                && stored_kind == kind
                && stored_kind_v2 == kind
                && stored_body_v2 == body
                && stored_status == &expected_status => {}
        ([], []) => {
            let row_kind = match row_kind {
                "leaf" => RowKind::Leaf,
                "coverage" => RowKind::Coverage,
                "graph" => RowKind::Graph,
                _ => return Err(EngineError::Storage),
            };
            project_canonical_node_row(
                tx,
                u64::try_from(cursor).map_err(|_| EngineError::Storage)?,
                kind,
                body,
                row_kind,
                ProjectionPass::FtsOnly,
                true,
            )
            .map_err(|_| EngineError::Storage)?;
        }
        _ => return Err(projection_generation::corruption_error()),
    }
    if !vector_projection_declared(tx).map_err(|_| EngineError::Storage)?
        || !matches!(row_kind, "leaf" | "coverage")
        || !kind_is_vector_committable(kind)
    {
        return Ok(false);
    }
    let terminal: Option<String> = tx
        .query_row(
            "SELECT state FROM _fathomdb_projection_terminal WHERE write_cursor=?1",
            [cursor],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let sidecar: Option<String> = tx
        .query_row(
            "SELECT kind FROM _fathomdb_vector_rows WHERE write_cursor=?1",
            [cursor],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let physical: Option<(String, String)> = tx
        .query_row("SELECT source_type,kind FROM vector_default WHERE rowid=?1", [cursor], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let expected_source_type = resolve_source_type(kind)?;
    if terminal.as_deref() == Some("up_to_date")
        && sidecar.as_deref() == Some(kind)
        && physical.as_ref().is_some_and(|(source_type, physical_kind)| {
            source_type == expected_source_type && physical_kind == kind
        })
    {
        return Ok(false);
    }
    if !matches!(terminal.as_deref(), None | Some("up_to_date"))
        || sidecar.is_some()
        || physical.is_some()
    {
        return Err(projection_generation::corruption_error());
    }
    tx.execute(
        "INSERT OR IGNORE INTO _fathomdb_vector_kinds(kind,profile,created_at) \
         VALUES(?1,'default',0)",
        [kind],
    )
    .map_err(|_| EngineError::Storage)?;
    tx.execute("DELETE FROM _fathomdb_projection_terminal WHERE write_cursor=?1", [cursor])
        .map_err(|_| EngineError::Storage)?;
    tx.execute(
        "INSERT INTO _fathomdb_projection_state(kind,last_enqueued_cursor,updated_at) \
         VALUES(?1,?2,0) ON CONFLICT(kind) DO UPDATE SET \
         last_enqueued_cursor=MAX(last_enqueued_cursor,excluded.last_enqueued_cursor)",
        params![kind, cursor],
    )
    .map_err(|_| EngineError::Storage)?;
    let cursor = u64::try_from(cursor).map_err(|_| EngineError::Storage)?;
    if load_projection_cursor(tx).map_err(|_| EngineError::Storage)? >= cursor {
        store_projection_cursor(tx, cursor.saturating_sub(1)).map_err(|_| EngineError::Storage)?;
    }
    Ok(true)
}

/// EU-5a2 — number of documents required before the workspace's
/// `_fathomdb_embedder_profiles.mean_vec` is pinned for the default
/// profile. Per `dev/design/embedder.md` §0.3 (compute-once-on-first-
/// ingest lifecycle). Public-visible so the EU-5a2 machinery test can
/// assert the value.
pub const MEAN_VEC_PIN_THRESHOLD: u64 = 256;

/// EU-5a2 — test-visible re-exports of the mean-centering internals.
/// Per the handoff RED tests; the production accumulator and re-quantize
/// pass are otherwise crate-private.
#[doc(hidden)]
pub mod mean_centering_internals_for_test {
    use super::{EmbedderEvent, MeanAccumulator};

    pub struct AccumulatorHandle(MeanAccumulator);

    #[must_use]
    pub fn new_mean_accumulator(dim: usize) -> AccumulatorHandle {
        AccumulatorHandle(MeanAccumulator::new(dim))
    }

    pub fn accumulator_add(handle: &mut AccumulatorHandle, v: &[f32]) {
        handle.0.add(v);
    }

    #[must_use]
    pub fn accumulator_materialize(handle: &AccumulatorHandle) -> Vec<f32> {
        handle.0.materialize()
    }

    #[must_use]
    pub fn accumulator_count(handle: &AccumulatorHandle) -> u64 {
        handle.0.count()
    }

    #[must_use]
    pub fn run_requantize_pass(rows: &[(i64, Vec<u8>)], mean: &[f32]) -> (u64, Vec<EmbedderEvent>) {
        super::run_requantize_pass(rows, mean)
    }
}

/// Test seam — exposes [`build_vector_phase1_sql`] at the production
/// `SEARCH_RERANK_LIMIT` so `pr_g10_filtered_knn.rs` can pin the `filter=None`
/// byte-identity and the appended predicates.
#[doc(hidden)]
#[must_use]
pub fn vector_phase1_sql_for_test(filter: Option<&SearchFilter>) -> String {
    build_vector_phase1_sql(filter, SEARCH_RERANK_LIMIT)
}

/// Test seam pinning production ranked SQL around the shared eligibility
/// compiler. Eligibility must precede ranking and any SQL candidate limit.
#[doc(hidden)]
#[must_use]
pub fn slice35_ranked_eligibility_sql_for_test(
    filter: &SearchFilter,
) -> [(&'static str, String); 3] {
    let mut body_params = vec![rusqlite::types::Value::Text("fixture".to_string())];
    let body_filter = append_node_eligibility_sql(Some(filter), "cn", &mut body_params);
    let mut edge_params = vec![
        rusqlite::types::Value::Text("fixture".to_string()),
        rusqlite::types::Value::Integer(0),
    ];
    let edge_filter = append_edge_eligibility_sql(Some(filter), "ce", &mut edge_params);
    let mut property_params = vec![
        rusqlite::types::Value::Text("owner".to_string()),
        rusqlite::types::Value::Text("fixture".to_string()),
    ];
    let property_filter = append_node_eligibility_sql(Some(filter), "n", &mut property_params);
    [
        ("body_fts", body_fts_rank_sql("", "", &body_filter, " LIMIT 10")),
        ("edge_fts", edge_fts_rank_sql("", &edge_filter)),
        ("property_fts", property_fts_rank_sql("", &property_filter)),
    ]
}

/// Return and clear the normalized Slice 71 search-statement trace.
#[cfg(feature = "test-hooks")]
#[doc(hidden)]
pub fn take_slice71_search_statement_trace_for_test() -> Vec<String> {
    slice71_search_statement_trace()
        .lock()
        .map(|mut trace| std::mem::take(&mut *trace))
        .unwrap_or_default()
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

#[cfg(any(test, feature = "test-hooks"))]
fn report_runtime_connection_inventory_for_test(
    shared: &ProjectionRuntimeShared,
    connection: &Connection,
    role: WalAttributionRole,
    index: usize,
) {
    let respond = {
        let Ok(mut request_slot) = shared.runtime_inventory_request.lock() else {
            return;
        };
        let Some(request) = request_slot.as_mut() else {
            return;
        };
        if !request.pending.remove(&(role, index)) {
            return;
        }
        let respond = request.respond.clone();
        if request.pending.is_empty() {
            *request_slot = None;
        }
        respond
    };
    let _ = respond.send((role, index, connection.is_autocommit()));
}

#[cfg(any(test, feature = "test-hooks"))]
fn report_runtime_native_state_inventory_for_test(
    shared: &ProjectionRuntimeShared,
    connection: &Connection,
    role: WalAttributionRole,
    index: usize,
) {
    let respond = {
        let Ok(mut request_slot) = shared.runtime_native_state_request.lock() else {
            return;
        };
        let Some(request) = request_slot.as_mut() else {
            return;
        };
        if !request.pending.remove(&(role, index)) {
            return;
        }
        let respond = request.respond.clone();
        if request.pending.is_empty() {
            *request_slot = None;
        }
        respond
    };
    let _ = respond.send(native_connection_state_for_test(connection, role, index));
}

struct CanonicalNodeRow {
    cursor: u64,
    kind: String,
    body: String,
    row_kind: RowKind,
    /// fix-2 [P2] — whether this row is in the attribute projection's row set
    /// (`state = 'active' AND superseded_at IS NULL`, the exact `backfill_attribute`
    /// predicate). A projector-replay rebuild uses this to gate the attribute
    /// projection so it does not re-surface a pending / superseded node's values.
    /// Node-FTS / vector shadows are rebuilt for every row (their stale versions
    /// are excluded by the read-side lifecycle join, unchanged from before).
    attr_projected: bool,
}

/// 0.8.0 Slice 5 (G1) — re-tokenize `search_index` from the canonical source
/// rows after the step-11 tokenizer-default upgrade drops + recreates the FTS5
/// virtual table. Projection-only: it reads `canonical_nodes` (the source of
/// truth, untouched) and rewrites the FTS shadow; it performs **no**
/// source-record migration. Every canonical node already carries an FTS row at
/// write time (the projection-time INSERT is unconditional), so reinserting
/// every node exactly reproduces the prior index content under the new
/// tokenizer. Runs in a single transaction on the writer connection before
/// readers spawn.
///
/// Crash-retryable (fix-1): the reindex and its durable completion marker
/// (`SEARCH_INDEX_TOKENIZER_REPROJECT_MARKER_KEY` in `_fathomdb_open_state`)
/// commit together in ONE `BEGIN IMMEDIATE…COMMIT`. A crash before the commit
/// rolls both back, leaving no marker; the next open re-runs. A crash after
/// the commit finds the marker present and skips. Idempotent.
fn reproject_search_index_after_tokenizer_upgrade(connection: &Connection) -> rusqlite::Result<()> {
    let rows = canonical_node_rows(connection)?;
    connection.execute_batch("BEGIN IMMEDIATE")?;
    let result = (|| {
        // 0.8.20 Slice 5a (R-20-E1) — registry-driven: re-tokenize EVERY
        // node-FTS projection, not just `search_index`. `search_index_v2` uses
        // the SAME tokenizer (`porter unicode61 remove_diacritics 2`), so it is
        // equally invalidated by a tokenizer-default upgrade; before this slice
        // it was neither cleared nor re-tokenized here. Edge FTS is out of scope
        // for this open-path repair (it postdates the step-11 upgrade and is
        // rebuilt by `rebuild_projections`).
        truncate_row_projections_in(connection, &[ProjectionClass::NodeFts])?;
        for row in &rows {
            project_canonical_node_row(
                connection,
                row.cursor,
                &row.kind,
                &row.body,
                row.row_kind,
                ProjectionPass::FtsOnly,
                // FtsOnly never touches the attribute store (predates step 24), so
                // `node_active` is inert here; forward the row's flag anyway (it is
                // the backfill's active-and-non-superseded predicate) so the field
                // has a reader in every build configuration.
                row.attr_projected,
            )?;
        }
        connection.execute(
            "INSERT INTO _fathomdb_open_state(key, value) VALUES(?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![SEARCH_INDEX_TOKENIZER_REPROJECT_MARKER_KEY, "1"],
        )?;
        Ok(())
    })();
    match result {
        Ok(()) => connection.execute_batch("COMMIT"),
        Err(err) => {
            let _ = connection.execute_batch("ROLLBACK");
            Err(err)
        }
    }
}

/// 0.8.0 Slice 5 (G1) fix-1 — has the post-tokenizer-upgrade re-tokenization
/// committed durably on this DB? Keys off the
/// `SEARCH_INDEX_TOKENIZER_REPROJECT_MARKER_KEY` row written inside the reindex
/// transaction; its absence on a v11 DB means the reindex never committed
/// (fresh-after-step-11 or crash-in-window) and must (re-)run.
///
/// A MISSING `_fathomdb_open_state` table is reported as "complete" (skip the
/// reproject): that table is created by migration step 1, so its absence means
/// the DB never ran our migrations (e.g. a synthetic DB whose `user_version`
/// was stamped to 11 by hand, or a legacy/foreign shape). Such DBs are
/// rejected by the downstream embedder-identity/integrity probes; the reproject
/// must not run — and must not mask those errors — on them. On a genuinely
/// migrated DB the table always exists, so the crash-repair path is unaffected.
fn search_index_tokenizer_reproject_complete(connection: &Connection) -> rusqlite::Result<bool> {
    match connection.query_row(
        "SELECT value FROM _fathomdb_open_state WHERE key = ?1",
        [SEARCH_INDEX_TOKENIZER_REPROJECT_MARKER_KEY],
        |row| row.get::<_, String>(0),
    ) {
        Ok(value) => Ok(value == "1"),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
        Err(rusqlite::Error::SqliteFailure(_, Some(ref message)))
            if message.contains("no such table") =>
        {
            Ok(true)
        }
        Err(err) => Err(err),
    }
}

/// 0.8.20 Slice 15c (TC-33) fix-6 — has the one-time edge-vector prune committed
/// durably on this DB? Keys off the [`EDGE_VECTOR_PRUNE_MARKER_KEY`] row written
/// inside the prune transaction; its absence means the prune never ran (a DB
/// upgraded before this fix shipped, or a crash between the step-23 commit and
/// the prune commit) and must (re-)run.
///
/// A MISSING `_fathomdb_open_state` table is reported as "complete" (skip the
/// prune) — that table is created by migration step 1, so its absence means the
/// DB never ran our migrations (a synthetic/foreign shape rejected downstream);
/// the prune must not run, and must not mask those errors, on it. Mirrors
/// [`search_index_tokenizer_reproject_complete`].
fn edge_vector_prune_complete(connection: &Connection) -> rusqlite::Result<bool> {
    match connection.query_row(
        "SELECT value FROM _fathomdb_open_state WHERE key = ?1",
        [EDGE_VECTOR_PRUNE_MARKER_KEY],
        |row| row.get::<_, String>(0),
    ) {
        Ok(value) => Ok(value == "1"),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
        Err(rusqlite::Error::SqliteFailure(_, Some(ref message)))
            if message.contains("no such table") =>
        {
            Ok(true)
        }
        Err(err) => Err(err),
    }
}

/// 0.8.20 Slice 15c (TC-33) fix-6 — delete every `vector_default` (vec0) row that
/// has NO `_fathomdb_vector_rows` sidecar entry, then record the durable
/// completion marker, all in one `BEGIN IMMEDIATE` transaction (crash-retryable:
/// a crash before COMMIT leaves no marker and the next open re-runs).
///
/// A vec0 row and its sidecar row are written and deleted TOGETHER (same
/// transaction) on every steady-state path, so a sidecar-less vec0 row is ONLY
/// ever produced by the step-23 recreate, which drops the edge rows and their
/// sidecar entries but cannot reach the engine-created vec0 table. So this
/// targets exactly the dropped edges' orphans and touches NOTHING on a healthy
/// corpus. Node vec0 rows keep their sidecar entry, so they are never pruned —
/// node recall is unaffected.
///
/// The orphans are gathered with plain scans (both proven vec0 forms — a full
/// `SELECT rowid FROM vector_default` and per-`rowid` `DELETE`) and diffed in
/// Rust, rather than relying on a compound `DELETE ... WHERE rowid NOT IN (...)`
/// over the virtual table.
fn prune_orphaned_edge_vectors(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute_batch("BEGIN IMMEDIATE")?;
    let result = (|| {
        let sidecar: std::collections::HashSet<i64> = {
            let mut statement =
                connection.prepare("SELECT write_cursor FROM _fathomdb_vector_rows")?;
            let rows = statement.query_map([], |row| row.get::<_, i64>(0))?;
            let mut set = std::collections::HashSet::new();
            for r in rows {
                set.insert(r?);
            }
            set
        };
        let vec_rowids: Vec<i64> = {
            let mut statement = connection.prepare("SELECT rowid FROM vector_default")?;
            let rows = statement.query_map([], |row| row.get::<_, i64>(0))?;
            let mut out = Vec::new();
            for r in rows {
                out.push(r?);
            }
            out
        };
        for rowid in vec_rowids {
            if !sidecar.contains(&rowid) {
                // vec0 rowid IS the canonical write_cursor; delete by rowid (the
                // proven vec0 delete form, as `prune_edge_projection_shadows`),
                // through the one TC-76-safe vec0-delete primitive.
                delete_vector_partition_row(connection, rowid)?;
            }
        }
        connection.execute(
            "INSERT INTO _fathomdb_open_state(key, value) VALUES(?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![EDGE_VECTOR_PRUNE_MARKER_KEY, "1"],
        )?;
        Ok(())
    })();
    match result {
        Ok(()) => connection.execute_batch("COMMIT"),
        Err(err) => {
            let _ = connection.execute_batch("ROLLBACK");
            Err(err)
        }
    }
}

fn canonical_node_rows(connection: &Connection) -> rusqlite::Result<Vec<CanonicalNodeRow>> {
    // fix-2 [P2] — also read `state` + `superseded_at` so a replay rebuild can gate
    // the attribute projection to the backfill's row set. `attr_projected` mirrors
    // the exact `backfill_attribute` predicate (`state = 'active' AND
    // superseded_at IS NULL`): a NULL/foreign state is NOT 'active' and so is
    // excluded, identical to the SQL equality.
    let mut statement = connection.prepare(
        "SELECT write_cursor, kind, body, row_kind, state, superseded_at \
         FROM canonical_nodes ORDER BY write_cursor",
    )?;
    let rows = statement.query_map([], |row| {
        let state: Option<String> = row.get::<_, Option<String>>(4)?;
        let superseded_at: Option<i64> = row.get::<_, Option<i64>>(5)?;
        Ok(CanonicalNodeRow {
            cursor: row.get::<_, u64>(0)?,
            kind: row.get::<_, String>(1)?,
            body: row.get::<_, String>(2)?,
            row_kind: row_kind_from_column(&row.get::<_, String>(3)?),
            attr_projected: state.as_deref() == Some("active") && superseded_at.is_none(),
        })
    })?;
    rows.collect()
}

/// 0.8.20 Slice 5a — inverse of [`RowKind::as_str`] for the stored
/// `canonical_nodes.row_kind` column. An unrecognized spelling degrades to
/// `Leaf`, the column DEFAULT and the shape every pre-EXP-S row carries; that
/// keeps a projector replay behavior-identical to the pre-registry rebuild,
/// which ignored `row_kind` entirely.
fn row_kind_from_column(value: &str) -> RowKind {
    match value {
        "coverage" => RowKind::Coverage,
        "graph" => RowKind::Graph,
        _ => RowKind::Leaf,
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(hex_nibble(byte >> 4));
        out.push(hex_nibble(byte & 0x0f));
    }
    out
}

fn hex_nibble(value: u8) -> char {
    match value {
        0..=9 => (b'0' + value) as char,
        10..=15 => (b'a' + value - 10) as char,
        _ => unreachable!(),
    }
}

#[cfg(feature = "operator")]
fn physical_section(connection: &Connection, full: bool) -> Section {
    let mut findings = Vec::new();
    if let Err(err) = connection.query_row("PRAGMA page_count", [], |row| row.get::<_, i64>(0)) {
        findings.push(Finding {
            code: "E_CORRUPT_HEADER",
            stage: "PhysicalProbe",
            locator: locator_from_rusqlite_error(&err),
            doc_anchor: "design/recovery.md#header-malformed",
            detail: format!("page_count probe failed: {err}"),
        });
    }
    if full {
        match collect_integrity_check_findings(connection) {
            Ok(rows) => findings.extend(rows),
            Err(err) => findings.push(Finding {
                code: "E_CORRUPT_INTEGRITY_CHECK",
                stage: "IntegrityCheck",
                locator: locator_from_rusqlite_error(&err),
                doc_anchor: "design/recovery.md#integrity-check-full-findings",
                detail: format!("PRAGMA integrity_check failed: {err}"),
            }),
        }
    }
    if findings.is_empty() {
        Section::Clean
    } else {
        Section::Findings(findings)
    }
}

#[cfg(feature = "operator")]
fn logical_section(connection: &Connection) -> Section {
    let mut findings = Vec::new();
    if let Err(err) = connection.query_row("PRAGMA schema_version", [], |row| row.get::<_, i64>(0))
    {
        findings.push(Finding {
            code: "E_CORRUPT_SCHEMA",
            stage: "SchemaProbe",
            locator: locator_from_rusqlite_error(&err),
            doc_anchor: "design/recovery.md#schema-inconsistent",
            detail: format!("schema_version probe failed: {err}"),
        });
    }
    match connection.query_row("PRAGMA user_version", [], |row| row.get::<_, u32>(0)) {
        Ok(0) => findings.push(Finding {
            code: "E_CORRUPT_SCHEMA",
            stage: "SchemaProbe",
            locator: CorruptionLocator::MigrationStep { from: 0, to: 0 },
            doc_anchor: "design/recovery.md#schema-inconsistent",
            detail: "user_version is zero".to_string(),
        }),
        Ok(_) => {}
        Err(err) => findings.push(Finding {
            code: "E_CORRUPT_SCHEMA",
            stage: "SchemaProbe",
            locator: locator_from_rusqlite_error(&err),
            doc_anchor: "design/recovery.md#schema-inconsistent",
            detail: format!("user_version probe failed: {err}"),
        }),
    }
    if findings.is_empty() {
        Section::Clean
    } else {
        Section::Findings(findings)
    }
}

#[cfg(feature = "operator")]
fn semantic_section(connection: &Connection) -> Section {
    match load_default_profile(connection) {
        Ok(_) => Section::Clean,
        Err(rusqlite::Error::QueryReturnedNoRows) => Section::Findings(vec![Finding {
            code: "E_CORRUPT_EMBEDDER_IDENTITY",
            stage: "EmbedderIdentity",
            locator: CorruptionLocator::OpaqueSqliteError { sqlite_extended_code: 0 },
            doc_anchor: "design/recovery.md#embedder-identity-drift",
            detail: "default embedder profile row is missing".to_string(),
        }]),
        Err(err) => Section::Findings(vec![Finding {
            code: "E_CORRUPT_EMBEDDER_IDENTITY",
            stage: "EmbedderIdentity",
            locator: locator_from_rusqlite_error(&err),
            doc_anchor: "design/recovery.md#embedder-identity-drift",
            detail: format!("default embedder profile probe failed: {err}"),
        }]),
    }
}

#[cfg(feature = "operator")]
fn collect_integrity_check_findings(connection: &Connection) -> rusqlite::Result<Vec<Finding>> {
    let mut statement = connection.prepare("PRAGMA integrity_check")?;
    let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
    let mut findings = Vec::new();
    for row in rows {
        let message = row?;
        if message == "ok" {
            continue;
        }
        findings.push(Finding {
            code: "E_CORRUPT_INTEGRITY_CHECK",
            stage: "IntegrityCheck",
            locator: CorruptionLocator::OpaqueSqliteError {
                sqlite_extended_code: rusqlite::ffi::SQLITE_CORRUPT,
            },
            doc_anchor: "design/recovery.md#integrity-check-full-findings",
            detail: message,
        });
    }
    Ok(findings)
}

#[cfg(feature = "operator")]
fn locator_from_rusqlite_error(err: &rusqlite::Error) -> CorruptionLocator {
    let extended = err.sqlite_error().map(|inner| inner.extended_code).unwrap_or(0);
    CorruptionLocator::OpaqueSqliteError { sqlite_extended_code: extended }
}

fn open_managed_connection(
    path: &Path,
    #[cfg(any(test, feature = "test-hooks"))] category: ManagedConnectionCategory,
    #[cfg(any(test, feature = "test-hooks"))] managed_connections: &Arc<ManagedConnectionRegistry>,
) -> rusqlite::Result<Connection> {
    #[cfg(any(test, feature = "test-hooks"))]
    managed_connections.record_open(category);
    Connection::open(path)
}

fn open_runtime_connection(
    path: &Path,
    #[cfg(any(test, feature = "test-hooks"))] category: ManagedConnectionCategory,
    #[cfg(any(test, feature = "test-hooks"))] managed_connections: &Arc<ManagedConnectionRegistry>,
) -> rusqlite::Result<Connection> {
    let connection = open_managed_connection(
        path,
        #[cfg(any(test, feature = "test-hooks"))]
        category,
        #[cfg(any(test, feature = "test-hooks"))]
        managed_connections,
    )?;
    connection.pragma_update(None, "journal_mode", "WAL")?;
    // OPP-12 Phase-1 (0.8.19 Slice 10, design §3 gap-4) — `secure_delete=ON` at
    // EVERY open. The projection/vector-rewrite runtime connection performs
    // DELETEs (shadow-table rewrites), so its freed pages must be scrubbed too;
    // setting the pragma only on the writer left a GDPR-erasure leak here.
    connection.pragma_update(None, "secure_delete", "ON")?;
    Ok(connection)
}

/// 0.8.20 Slice 5b (R-20-E7) — the audit handle for an erased op-store record:
/// `SHA-256(collection + 0x1F + record_key)`, lowercase hex.
///
/// A record key is arbitrary caller-supplied text and may itself be the
/// identifier being erased, so a durable audit row must not echo it. `0x1F`
/// (ASCII unit separator) is the delimiter because it cannot appear in a
/// well-formed collection name, keeping the pairing unambiguous.
#[cfg(feature = "operator")]
fn digest_record_identity(collection: &str, record_key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(collection.as_bytes());
    hasher.update([0x1f_u8]);
    hasher.update(record_key.as_bytes());
    hasher.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

fn projection_status(
    connection: &Connection,
    kind: &str,
) -> Result<lifecycle::ProjectionStatus, EngineError> {
    let latest = connection
        .query_row(
            "SELECT COALESCE(MAX(write_cursor), 0) FROM canonical_nodes WHERE kind = ?1",
            [kind],
            |row| row.get::<_, u64>(0),
        )
        .map_err(|_| EngineError::Storage)?;
    if latest == 0 {
        return Ok(lifecycle::ProjectionStatus::UpToDate);
    }
    let pending: u64 = connection
        .query_row(
            "SELECT COUNT(*)
             FROM canonical_nodes
             LEFT JOIN _fathomdb_projection_terminal
               ON _fathomdb_projection_terminal.write_cursor = canonical_nodes.write_cursor
             WHERE canonical_nodes.kind = ?1
               AND _fathomdb_projection_terminal.write_cursor IS NULL",
            [kind],
            |row| row.get(0),
        )
        .map_err(|_| EngineError::Storage)?;
    if pending > 0 {
        return Ok(lifecycle::ProjectionStatus::Pending);
    }
    match terminal_state_for_cursor(connection, latest).map_err(|_| EngineError::Storage)? {
        Some(state) if state == "failed" => Ok(lifecycle::ProjectionStatus::Failed),
        _ => Ok(lifecycle::ProjectionStatus::UpToDate),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DatabaseAdmission {
    CurrentOnly,
    #[cfg(feature = "migration-test-hooks")]
    TestMigrations,
}

struct DatabaseOpenPlan {
    migrations: &'static [fathomdb_schema::Migration],
    admission: DatabaseAdmission,
    config: EngineConfig,
}

struct ShmSnapshot {
    path: PathBuf,
    bytes: Option<Vec<u8>>,
}

impl ShmSnapshot {
    fn capture(database_path: &Path) -> Result<Self, EngineOpenError> {
        let mut shm_path = database_path.as_os_str().to_os_string();
        shm_path.push("-shm");
        let path = PathBuf::from(shm_path);
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(_) => {
                return Err(EngineOpenError::Io {
                    message: "database shared-memory sidecar is not accessible".to_string(),
                })
            }
        };
        Ok(Self { path, bytes })
    }

    fn restore(self) -> Result<(), EngineOpenError> {
        match self.bytes {
            Some(bytes) => std::fs::write(self.path, bytes).map_err(|_| EngineOpenError::Io {
                message: "database shared-memory sidecar could not be restored".to_string(),
            }),
            None => match std::fs::remove_file(self.path) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(_) => Err(EngineOpenError::Io {
                    message: "temporary database shared-memory sidecar could not be removed"
                        .to_string(),
                }),
            },
        }
    }
}

fn read_effective_schema_version(path: &Path) -> Result<u32, EngineOpenError> {
    probe_wal_sidecar(path)?;
    let connection = Connection::open_with_flags(
        read_only_sqlite_uri(path),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
            | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX
            | rusqlite::OpenFlags::SQLITE_OPEN_URI,
    )
    .map_err(|error| map_open_sqlite_error(error, OpenStage::HeaderProbe))?;
    connection
        .pragma_update(None, "query_only", "ON")
        .map_err(|error| map_open_sqlite_error(error, OpenStage::SchemaProbe))?;
    probe_database_header(&connection)?;
    probe_open_integrity(&connection)?;
    connection
        .pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))
        .map_err(|error| map_open_sqlite_error(error, OpenStage::SchemaProbe))
}

fn admit_current_database(path: &Path) -> Result<(), EngineOpenError> {
    let metadata = match std::fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => {
            return Err(EngineOpenError::Io {
                message: "database candidate metadata is not accessible".to_string(),
            })
        }
    };
    if metadata.len() == 0 {
        return Ok(());
    }
    let shm = ShmSnapshot::capture(path)?;
    match read_effective_schema_version(path) {
        Ok(seen) if seen == SCHEMA_VERSION => Ok(()),
        Ok(seen) => {
            shm.restore()?;
            Err(EngineOpenError::IncompatibleSchemaVersion { seen, supported: SCHEMA_VERSION })
        }
        Err(error) => {
            shm.restore()?;
            Err(error)
        }
    }
}

fn canonical_database_path(path: &Path) -> Result<PathBuf, EngineOpenError> {
    match path.canonicalize() {
        Ok(canonical) => return Ok(canonical),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => {
            return Err(EngineOpenError::Io {
                message: "database path is not accessible".to_string(),
            })
        }
    }
    // `canonicalize` reports NotFound for both a genuinely absent file and a
    // dangling final-component symlink. Only the former is a legal fresh-path
    // bootstrap; treating the latter as a filename would create a split lock
    // namespace beside the alias.
    match std::fs::symlink_metadata(path) {
        Ok(_) => {
            return Err(EngineOpenError::Io {
                message: "database path does not resolve to an accessible file".to_string(),
            })
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => {
            return Err(EngineOpenError::Io {
                message: "database path is not accessible".to_string(),
            })
        }
    }
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let canonical_parent = parent.canonicalize().map_err(|_| EngineOpenError::Io {
        message: "database parent directory is not accessible".to_string(),
    })?;
    let file_name = path.file_name().ok_or_else(|| EngineOpenError::Io {
        message: "database path has no file name".to_string(),
    })?;

    Ok(canonical_parent.join(file_name))
}

struct PendingDatabaseLock {
    file: Option<File>,
}

impl PendingDatabaseLock {
    fn initialize(mut self) -> Result<File, EngineOpenError> {
        let file = self.file.as_mut().expect("pending database lock retains its file");
        let pid = std::process::id().to_string();
        file.set_len(0).map_err(|_| EngineOpenError::Io {
            message: "could not initialize database lock file".to_string(),
        })?;
        file.seek(SeekFrom::Start(0)).map_err(|_| EngineOpenError::Io {
            message: "could not initialize database lock file".to_string(),
        })?;
        file.write_all(pid.as_bytes()).map_err(|_| EngineOpenError::Io {
            message: "could not initialize database lock file".to_string(),
        })?;
        Ok(self.file.take().expect("initialized database lock retains its file"))
    }
}

fn acquire_lock_without_metadata_mutation(
    path: &Path,
) -> Result<PendingDatabaseLock, EngineOpenError> {
    let lock_path = lock_path(path);
    let mut create_options = OpenOptions::new();
    create_options.read(true).write(true).create_new(true);
    #[cfg(unix)]
    create_options.mode(0o600);
    let file = match create_options.open(&lock_path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let mut existing_options = OpenOptions::new();
            existing_options.read(true).write(true);
            existing_options.open(&lock_path).map_err(|_| EngineOpenError::Io {
                message: "could not open database lock file".to_string(),
            })?
        }
        Err(_) => {
            return Err(EngineOpenError::Io {
                message: "could not open database lock file".to_string(),
            })
        }
    };
    let pending = PendingDatabaseLock { file: Some(file) };

    match pending.file.as_ref().expect("pending database lock retains its file").try_lock() {
        Ok(()) => Ok(pending),
        Err(std::fs::TryLockError::WouldBlock) => {
            Err(EngineOpenError::DatabaseLocked { holder_pid: read_holder_pid(&lock_path) })
        }
        Err(_) => {
            Err(EngineOpenError::Io { message: "could not acquire database lock".to_string() })
        }
    }
}

fn lock_path(path: &Path) -> PathBuf {
    let mut lock_path = path.as_os_str().to_os_string();
    lock_path.push(LOCK_SUFFIX);
    PathBuf::from(lock_path)
}

fn read_holder_pid(path: &Path) -> Option<u32> {
    std::fs::read_to_string(path).ok()?.trim().parse().ok()
}

fn map_migration_error(err: SchemaMigrationError) -> EngineOpenError {
    match err {
        SchemaMigrationError::IncompatibleSchemaVersion { seen, supported } => {
            EngineOpenError::IncompatibleSchemaVersion { seen, supported }
        }
        SchemaMigrationError::MigrationError(report) => EngineOpenError::MigrationError {
            schema_version_before: report.schema_version_before,
            schema_version_current: report.schema_version_current,
            step_id: report.migration_steps.last().map_or(0, |step| step.step_id),
        },
        SchemaMigrationError::Storage { message } => {
            EngineOpenError::Io { message: message.to_string() }
        }
    }
}

fn register_sqlite_vec_extension() {
    static REGISTER: Once = Once::new();
    REGISTER.call_once(|| unsafe {
        let entrypoint: unsafe extern "C" fn(
            *mut rusqlite::ffi::sqlite3,
            *mut *mut std::os::raw::c_char,
            *const rusqlite::ffi::sqlite3_api_routines,
        ) -> std::os::raw::c_int = std::mem::transmute(sqlite3_vec_init as *const ());
        rusqlite::ffi::sqlite3_auto_extension(Some(entrypoint));
    });
}

fn probe_open_integrity(connection: &Connection) -> Result<(), EngineOpenError> {
    // `SELECT COUNT(*) FROM sqlite_schema` forces a full traversal of the
    // sqlite_schema b-tree; this surfaces page-1 b-tree corruption that a
    // bare `PRAGMA schema_version` (which only reads the schema cookie
    // out of the file header) would miss.
    connection
        .query_row("SELECT COUNT(*) FROM sqlite_schema", [], |row| row.get::<_, i64>(0))
        .map(|_| ())
        .map_err(|err| map_open_sqlite_error(err, OpenStage::SchemaProbe))
}

fn probe_database_header(connection: &Connection) -> Result<(), EngineOpenError> {
    connection
        .query_row("PRAGMA application_id", [], |row| row.get::<_, i64>(0))
        .map(|_| ())
        .map_err(|err| map_open_sqlite_error(err, OpenStage::HeaderProbe))
}

/// Pre-`pragma WAL` sidecar validation. SQLite silently discards a WAL
/// file whose header magic is wrong or whose advertised page size is
/// outside `[512, SQLITE_MAX_PAGE_SIZE]`, which would cause us to lose
/// committed frames at open time. AC-035a requires that we instead
/// refuse to open with `Corruption(WalReplayFailure)` rather than
/// silently rebuild from a truncated WAL.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WalSidecarHeader {
    AbsentOrShort,
    Valid,
    Malformed { offset: u64 },
}

fn classify_wal_sidecar(db_path: &Path) -> Result<WalSidecarHeader, EngineOpenError> {
    let mut wal_path = db_path.as_os_str().to_owned();
    wal_path.push("-wal");
    let wal_path = PathBuf::from(wal_path);
    // Bounded read: the WAL header is fixed-layout in the first 32
    // bytes (magic + format + page-size + checkpoint-seq + salts +
    // checksums); frame data starts at offset 32 and is irrelevant to
    // the magic + page-size pre-check. A `std::fs::read` of the whole
    // sidecar would force an unclean-shutdown open path to allocate
    // and copy the entire WAL into memory before SQLite touches
    // recovery — a real latency + RSS regression on AC-035.
    use std::io::Read;
    let mut file = match std::fs::File::open(&wal_path) {
        Ok(file) => file,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(WalSidecarHeader::AbsentOrShort)
        }
        Err(_) => {
            return Err(EngineOpenError::Io {
                message: "database WAL sidecar is not accessible".to_string(),
            })
        }
    };
    let mut bytes = [0u8; 32];
    if let Err(error) = file.read_exact(&mut bytes) {
        if error.kind() == std::io::ErrorKind::UnexpectedEof {
            // A short (< 32-byte) sidecar carries no committed frames;
            // SQLite treats it as empty and re-initializes WAL state.
            return Ok(WalSidecarHeader::AbsentOrShort);
        }
        return Err(EngineOpenError::Io {
            message: "database WAL sidecar could not be read".to_string(),
        });
    }
    let magic = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    let page_size = u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
    // WAL_MAGIC mask per SQLite `walIndexRecover`: low bit distinguishes
    // big-endian vs little-endian checksum encoding; the rest of the
    // magic is fixed.
    const WAL_MAGIC_MASK: u32 = 0xFFFF_FFFE;
    const WAL_MAGIC: u32 = 0x377F_0682;
    const SQLITE_MAX_PAGE_SIZE: u32 = 65536;
    let magic_ok = (magic & WAL_MAGIC_MASK) == WAL_MAGIC;
    let page_size_ok =
        page_size.is_power_of_two() && (512..=SQLITE_MAX_PAGE_SIZE).contains(&page_size);
    if magic_ok && page_size_ok {
        return Ok(WalSidecarHeader::Valid);
    }
    Ok(WalSidecarHeader::Malformed { offset: if !magic_ok { 0 } else { 8 } })
}

fn probe_wal_sidecar(db_path: &Path) -> Result<(), EngineOpenError> {
    let WalSidecarHeader::Malformed { offset } = classify_wal_sidecar(db_path)? else {
        return Ok(());
    };
    Err(EngineOpenError::Corruption(CorruptionDetail {
        kind: CorruptionKind::WalReplayFailure,
        stage: OpenStage::WalReplay,
        locator: CorruptionLocator::FileOffset { offset },
        recovery_hint: RecoveryHint {
            code: "E_CORRUPT_WAL_REPLAY",
            doc_anchor: "design/recovery.md#wal-replay-failures",
        },
    }))
}

fn reject_legacy_shape(connection: &Connection) -> Result<(), EngineOpenError> {
    let has_legacy_table = table_exists(connection, "fathom_nodes")
        || table_exists(connection, "fathom_edges")
        || table_exists(connection, "fathom_chunks");
    if !has_legacy_table {
        return Ok(());
    }

    let seen =
        connection.query_row("PRAGMA user_version", [], |row| row.get::<_, u32>(0)).unwrap_or(0);
    Err(EngineOpenError::IncompatibleSchemaVersion { seen, supported: SCHEMA_VERSION })
}

fn validate_dependency_generation_on_open(
    connection: &Connection,
    schema_version: u32,
) -> Result<(), EngineOpenError> {
    if schema_version < SOURCE_DEPENDENCY_SCHEMA_VERSION {
        return Ok(());
    }
    let valid = (|| -> Result<bool, rusqlite::Error> {
        let value: String = connection.query_row(
            "SELECT value FROM _fathomdb_open_state WHERE key=?1",
            [DEPENDENCY_GENERATION_KEY],
            |row| row.get(0),
        )?;
        let Some(generation) = canonical_dependency_generation(&value) else {
            return Ok(false);
        };
        let max_generation: i64 = connection.query_row(
            "SELECT COALESCE(MAX(registered_dependency_generation), 0) \
             FROM _fathomdb_source_dependencies",
            [],
            |row| row.get(0),
        )?;
        Ok(max_generation >= 0 && generation >= max_generation as u64)
    })()
    .unwrap_or(false);
    if valid {
        return Ok(());
    }
    Err(EngineOpenError::Corruption(CorruptionDetail {
        kind: CorruptionKind::SchemaInconsistent,
        stage: OpenStage::SchemaProbe,
        locator: CorruptionLocator::TableRow { table: "_fathomdb_open_state", rowid: 0 },
        recovery_hint: RecoveryHint {
            code: "E_CORRUPT_SCHEMA",
            doc_anchor: "design/recovery.md#schema-inconsistent",
        },
    }))
}

fn table_exists(connection: &Connection, table: &str) -> bool {
    connection
        .query_row(
            "SELECT 1 FROM sqlite_schema WHERE type = 'table' AND name = ?1",
            [table],
            |_row| Ok(()),
        )
        .is_ok()
}

#[cfg(feature = "operator")]
fn read_schema_objects(
    connection: &Connection,
    obj_type: &str,
) -> Result<Vec<SchemaObject>, EngineError> {
    let mut stmt = connection
        .prepare(
            "SELECT name, sql FROM sqlite_schema
             WHERE type = ?1 AND name NOT LIKE 'sqlite_%' AND sql IS NOT NULL
             ORDER BY name",
        )
        .map_err(|_| EngineError::Storage)?;
    let rows = stmt
        .query_map([obj_type], |row| {
            Ok(SchemaObject { name: row.get::<_, String>(0)?, sql: row.get::<_, String>(1)? })
        })
        .map_err(|_| EngineError::Storage)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|_| EngineError::Storage)?);
    }
    Ok(out)
}

#[cfg(feature = "operator")]
fn order_canonical_first(mut objects: Vec<SchemaObject>) -> Vec<SchemaObject> {
    let mut canonical: Vec<SchemaObject> = Vec::new();
    for name in CANONICAL_TABLES {
        if let Some(pos) = objects.iter().position(|o| o.name == *name) {
            canonical.push(objects.remove(pos));
        }
    }
    canonical.extend(objects);
    canonical
}

fn default_embedder_identity() -> EmbedderIdentity {
    EmbedderIdentity::new(
        DEFAULT_EMBEDDER_NAME,
        DEFAULT_EMBEDDER_REVISION,
        DEFAULT_EMBEDDER_DIMENSION,
    )
}

fn check_embedder_profile(
    connection: &Connection,
    supplied: &EmbedderIdentity,
) -> Result<bool, EngineOpenError> {
    // Returns `true` iff `_fathomdb_embedder_profiles.mean_vec IS NOT NULL`
    // for the default profile (and its byte length matches `4 * dimension`
    // per `dev/design/embedder.md` §0.2). EU-5a2: column lands in step 10.
    let mut statement = match connection.prepare(
        "SELECT name, revision, dimension, mean_vec FROM _fathomdb_embedder_profiles WHERE profile = 'default'",
    ) {
        Ok(statement) => statement,
        Err(_) => return Ok(false),
    };
    let mut rows = statement.query([]).map_err(|_| {
        EngineOpenError::Corruption(CorruptionDetail {
            kind: CorruptionKind::EmbedderIdentityDrift,
            stage: OpenStage::EmbedderIdentity,
            locator: CorruptionLocator::OpaqueSqliteError { sqlite_extended_code: 0 },
            recovery_hint: RecoveryHint {
                code: "E_CORRUPT_EMBEDDER_IDENTITY",
                doc_anchor: "design/recovery.md#embedder-identity-drift",
            },
        })
    })?;

    let Some(row) = rows.next().map_err(|_| {
        EngineOpenError::Corruption(CorruptionDetail {
            kind: CorruptionKind::EmbedderIdentityDrift,
            stage: OpenStage::EmbedderIdentity,
            locator: CorruptionLocator::OpaqueSqliteError { sqlite_extended_code: 0 },
            recovery_hint: RecoveryHint {
                code: "E_CORRUPT_EMBEDDER_IDENTITY",
                doc_anchor: "design/recovery.md#embedder-identity-drift",
            },
        })
    })?
    else {
        connection
            .execute(
                "INSERT INTO _fathomdb_embedder_profiles(profile, name, revision, dimension)
                 VALUES(?1, ?2, ?3, ?4)",
                params![
                    DEFAULT_VECTOR_PROFILE,
                    supplied.name,
                    supplied.revision,
                    supplied.dimension
                ],
            )
            .map_err(|_| EngineOpenError::Io {
                message: "could not persist embedder profile".to_string(),
            })?;
        return Ok(false);
    };

    let stored_name = row.get::<_, String>(0).map_err(|_| {
        EngineOpenError::Corruption(CorruptionDetail {
            kind: CorruptionKind::EmbedderIdentityDrift,
            stage: OpenStage::EmbedderIdentity,
            locator: CorruptionLocator::TableRow { table: "_fathomdb_embedder_profiles", rowid: 0 },
            recovery_hint: RecoveryHint {
                code: "E_CORRUPT_EMBEDDER_IDENTITY",
                doc_anchor: "design/recovery.md#embedder-identity-drift",
            },
        })
    })?;
    let stored_revision = row.get::<_, String>(1).map_err(|_| {
        EngineOpenError::Corruption(CorruptionDetail {
            kind: CorruptionKind::EmbedderIdentityDrift,
            stage: OpenStage::EmbedderIdentity,
            locator: CorruptionLocator::TableRow { table: "_fathomdb_embedder_profiles", rowid: 0 },
            recovery_hint: RecoveryHint {
                code: "E_CORRUPT_EMBEDDER_IDENTITY",
                doc_anchor: "design/recovery.md#embedder-identity-drift",
            },
        })
    })?;
    let dimension = row.get::<_, u32>(2).map_err(|_| {
        EngineOpenError::Corruption(CorruptionDetail {
            kind: CorruptionKind::EmbedderIdentityDrift,
            stage: OpenStage::EmbedderIdentity,
            locator: CorruptionLocator::TableRow { table: "_fathomdb_embedder_profiles", rowid: 0 },
            recovery_hint: RecoveryHint {
                code: "E_CORRUPT_EMBEDDER_IDENTITY",
                doc_anchor: "design/recovery.md#embedder-identity-drift",
            },
        })
    })?;

    let stored = EmbedderIdentity::new(stored_name, stored_revision, dimension);

    if stored.name != supplied.name || stored.revision != supplied.revision {
        return Err(EngineOpenError::EmbedderIdentityMismatch {
            stored,
            supplied: supplied.clone(),
        });
    }
    if dimension != supplied.dimension {
        return Err(EngineOpenError::EmbedderDimensionMismatch {
            stored: dimension,
            supplied: supplied.dimension,
        });
    }

    // EU-5a2 / `dev/design/embedder.md` §0.2 invariant: if `mean_vec` is
    // populated, byte length MUST equal `4 * dimension`. Debug builds
    // assert; release builds fail closed via EmbedderIdentityMismatch
    // (the same fail-closed channel the rest of profile drift takes).
    let mean_vec: Option<Vec<u8>> = row.get::<_, Option<Vec<u8>>>(3).map_err(|_| {
        EngineOpenError::Corruption(CorruptionDetail {
            kind: CorruptionKind::EmbedderIdentityDrift,
            stage: OpenStage::EmbedderIdentity,
            locator: CorruptionLocator::TableRow { table: "_fathomdb_embedder_profiles", rowid: 0 },
            recovery_hint: RecoveryHint {
                code: "E_CORRUPT_EMBEDDER_IDENTITY",
                doc_anchor: "design/recovery.md#embedder-identity-drift",
            },
        })
    })?;
    let pinned = match mean_vec {
        Some(bytes) => {
            let expected_len = (dimension as usize).saturating_mul(4);
            // `dev/design/embedder.md` §0.2 invariant: when populated,
            // `mean_vec` byte length MUST equal `4 * dimension`. Fail
            // closed via the existing identity-drift channel in both
            // debug and release builds — tests deliberately poke
            // malformed values to exercise this branch.
            if bytes.len() != expected_len {
                return Err(EngineOpenError::EmbedderIdentityMismatch {
                    stored,
                    supplied: supplied.clone(),
                });
            }
            true
        }
        None => false,
    };

    Ok(pinned)
}

/// EXP-S (0.8.14 Slice 5, D2) — the set of coexisting indexes a `row_kind`
/// projects into. `fts` = the FTS index (`search_index`), written SYNCHRONOUSLY
/// in the write transaction; `vector` = the vec0 vector index, written
/// ASYNCHRONOUSLY by the projection worker pool (and additionally gated per
/// doc-type `kind` by [`kind_is_vector_indexed`]).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct IndexTargetSet {
    fts: bool,
    vector: bool,
}

/// EXP-S (0.8.14 Slice 5) — the `row_kind -> index-target set` dispatch
/// (ADR-0.8.14 §D2), and the OPP-12 forward-compat seam (ADR-0.8.14 §D5(a) /
/// ledger `TC-1`).
///
/// This is deliberately a per-kind LOOKUP rather than branching inlined at each
/// write call-site: it is the single seam a later declarative OPP-12 projection
/// registry (`dev/design/projection-registry-and-async-embed.md`) would wrap to
/// populate `row_kind -> {filterable, searchable->FTS (same-txn), searchable->
/// vector (async)}` without reshaping the substrate. Per D5, EXP-S implements
/// NO OPP-12 surface here (OPP-12 lands >=0.9.x; re-check at its scheduling) —
/// this function only records the index-target intent so the async-vs-sync split
/// (D5(b)) and the per-kind-extensible terminal-cursor readiness (D5(c)) stay
/// wrappable.
///
/// `Leaf` MUST preserve today's behavior exactly: FTS (sync) + vector (async,
/// gated by `kind_is_vector_indexed`).
fn index_targets_for_row_kind(row_kind: RowKind) -> IndexTargetSet {
    match row_kind {
        // Normal record — identical to pre-EXP-S behavior.
        RowKind::Leaf => IndexTargetSet { fts: true, vector: true },
        // Coverage/summary rows — searchable and embeddable.
        RowKind::Coverage => IndexTargetSet { fts: true, vector: true },
        // Graph structural rows — lexically searchable, not embedded.
        RowKind::Graph => IndexTargetSet { fts: true, vector: false },
    }
}

/// EXP-S (0.8.14 Slice 5, D2/D5) — apply the per-`row_kind` index-target
/// dispatch for one just-inserted canonical node row (write_cursor `cursor`).
///
/// Preserves the OPP-12-shaped split (D5(b)): FTS is written in THIS
/// transaction (same-txn `searchable->FTS`); vector work is only *enqueued*
/// here into `_fathomdb_projection_state` and embedded later, asynchronously,
/// by the projection worker pool (`searchable->vector`). When the row projects
/// into no async vector index, its readiness is terminated up-front (D5(c),
/// per-kind-extensible) so `advance_projection_cursor` can walk past it.
///
/// Returns `true` iff async vector work was enqueued (the caller must then
/// `notify_new_work`). For `RowKind::Leaf` this is behavior-identical to the
/// pre-EXP-S inline node path.
fn project_canonical_node_row(
    tx: &Connection,
    cursor: u64,
    kind: &str,
    body: &str,
    row_kind: RowKind,
    pass: ProjectionPass,
    node_active: bool,
) -> rusqlite::Result<bool> {
    let targets = index_targets_for_row_kind(row_kind);
    if targets.fts && pass.writes_fts() {
        tx.prepare_cached("INSERT INTO search_index(body, kind, write_cursor) VALUES(?1, ?2, ?3)")?
            .execute(params![body, kind, cursor])?;
        // F5 (0.8.14 Slice 10) — same coexisting `searchable->FTS` target also
        // populates the multi-column `search_index_v2` (kind/body/status) so a
        // BM25F query can field-weight the lexical arm. Written SYNCHRONOUSLY in
        // THIS transaction, exactly like `search_index` (rowid==write_cursor
        // identity preserved). The `status` field mirrors the migration-17
        // O(N) re-index: `$.status` from a JSON body, guarded by `json_valid` so
        // non-JSON bodies index an empty status. NOTE (codex fix-1 finding 2):
        // this is F5's OWN `$.status`-derived field for the BM25F `status`
        // column — it is NOT (yet) the value the shipped G10 SearchFilter reads.
        // G10 filtering reads the vec0 `status` column, which is still hardwired
        // to the empty-string sentinel; wiring G10 onto this field is out of
        // scope for F5. Determinism (R-SUB-2) is preserved: the derivation is
        // a pure function of `body`, evaluated in-SQL identically on every run.
        tx.prepare_cached(
            "INSERT INTO search_index_v2(kind, body, status, write_cursor)
             VALUES(
                 ?1,
                 ?2,
                 CASE WHEN json_valid(?2)
                      THEN COALESCE(json_extract(?2, '$.status'), '')
                      ELSE '' END,
                 ?3
             )",
        )?
        .execute(params![kind, body, cursor])?;
    }
    // 0.8.20 Slice 15d (R-20-EAV) — same-transaction attribute projection. Only
    // the full `Write` pass re-derives attributes (see `writes_attributes`): the
    // FtsOnly tokenizer reproject predates step 24 and must not touch the
    // registry/attribute tables; VectorOnly rebuilds only vector shadows. A full
    // operator FTS rebuild uses `Write`, so it re-derives attributes after the
    // truncate.
    //
    // fix-2 [P2]: gated on `node_active`. The at-rest attribute projection tracks
    // EXACTLY the backfill's row set — `state = 'active' AND superseded_at IS NULL`
    // (see `backfill_attribute`). Unlike node-FTS / vector shadows (whose stale
    // versions are excluded by the canonical read path's `superseded_at IS NULL`
    // / `state = 'active'` join), the property tables carry NO read-side lifecycle
    // filter (`property_search_index` is an FTS5 table that cannot), so a pending
    // or superseded node's attribute values would otherwise LEAK into a
    // same-session property filter / property-FTS. The write path passes
    // `state == Active`; a projector-replay rebuild passes `active ∧ non-superseded`
    // per row. Lifecycle transitions maintain the store directly (see
    // `Engine::transition`). Passes where `writes_attributes()` is false ignore the
    // flag entirely.
    if pass.writes_attributes() && node_active {
        project_node_attributes(tx, cursor as i64, body)?;
    }
    // 0.8.20 Slice 20c (R-20-DR remainder) — UNCHANGED, deliberately. Late
    // enrolment of a kind first written AFTER a `searchable→vector` declaration
    // happens in [`Engine::enrol_vector_kind_if_declared`], upstream of this
    // transaction, NOT here: the decision needs the engine's usable-runtime
    // predicate, which a free function holding only a `Connection` cannot see.
    // Enrolling without one would queue embeds that cannot safely run.
    let enqueue_vector = targets.vector && kind_is_vector_indexed(tx, kind).unwrap_or(false);
    if pass.writes_vector_state() {
        if enqueue_vector {
            tx.prepare_cached(
                "INSERT INTO _fathomdb_projection_state(kind, last_enqueued_cursor, updated_at)
                 VALUES(?1, ?2, 0)
                 ON CONFLICT(kind) DO UPDATE SET last_enqueued_cursor = excluded.last_enqueued_cursor",
            )?
            .execute(params![kind, cursor])?;
        } else {
            // Never-vector-projected rows terminate the cursor up-front so
            // `advance_projection_cursor` can advance the readiness watermark.
            record_projection_terminal(tx, cursor, "up_to_date")?;
        }
    }
    Ok(enqueue_vector)
}

/// 0.8.20 Slice 5a (R-20-E1, work item 1) — the EDGE half of the total
/// projector, extracted verbatim from the inlined `commit_batch` edge arm.
///
/// Before this extraction there was NO edge projector function: `commit_batch`
/// inlined the edge FTS insert + the edge vector enqueue, and
/// `rebuild_shadow_state` re-implemented a SUBSET of it (edge FTS only, and only
/// for body-carrying edges), so a projector-replay rebuild silently dropped the
/// rest — notably the `up_to_date` readiness terminal that the write path
/// records for a body-less structural edge. With both sites now calling this one
/// function, the write path and the rebuild path produce identical edge
/// projections by construction.
///
/// Mirrors [`project_canonical_node_row`]'s split (ADR-0.8.14 §D5(b)): FTS in
/// THIS transaction; vector work only ENQUEUED, embedded later by the worker
/// pool. Edge bodies enqueue under the fixed kind `"edge_fact"` so
/// `resolve_source_type` maps them to `source_type = "edge_fact"` in
/// `vector_default` (partition correctness); that kind is auto-registered in
/// `_fathomdb_vector_kinds` (idempotent).
///
/// Returns `true` iff async vector work was enqueued.
fn project_canonical_edge_row(
    tx: &Connection,
    cursor: u64,
    kind: &str,
    body: Option<&str>,
    pass: ProjectionPass,
) -> rusqlite::Result<bool> {
    // G11 — edge FTS projection into `search_index_edges` (separate table from
    // node-body `search_index` — Option B partition). Body-less structural
    // edges carry no lexical content and project no FTS row.
    if pass.writes_fts() {
        if let Some(edge_body) = body {
            tx.execute(
                "INSERT INTO search_index_edges(body, kind, write_cursor)
                 VALUES(?1, ?2, ?3)",
                params![edge_body, kind, cursor],
            )?;
        }
    }
    let enqueue_vector = body.is_some();
    if pass.writes_vector_state() {
        if enqueue_vector {
            let now_unix =
                SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64;
            tx.execute(
                "INSERT OR IGNORE INTO _fathomdb_vector_kinds(kind, profile, created_at)
                 VALUES('edge_fact', 'default', ?1)",
                params![now_unix],
            )?;
            tx.execute(
                "INSERT INTO _fathomdb_projection_state(
                     kind, last_enqueued_cursor, updated_at
                 ) VALUES('edge_fact', ?1, 0)
                 ON CONFLICT(kind) DO UPDATE
                     SET last_enqueued_cursor = excluded.last_enqueued_cursor",
                params![cursor],
            )?;
            // Do NOT call record_projection_terminal — let the scheduler embed
            // the body and mark it terminal after projection.
        } else {
            record_projection_terminal(tx, cursor, "up_to_date")?;
        }
    }
    Ok(enqueue_vector)
}

// Slice 15 stores no owner row for pre-step-27 content. Keep its deterministic
// identity derivation internal until the opt-in Slice 50 evidence resolver
// exposes it; default records and search hits remain unchanged.
#[allow(dead_code)]
fn legacy_revision_id(
    artifact_class: &str,
    cursor: u64,
    source_id: Option<&str>,
    body: Option<&str>,
) -> String {
    let mut hasher = Sha256::new();
    revision_hash_field(&mut hasher, b"fathomdb:artifact-revision:migrated:v1");
    revision_hash_field(&mut hasher, artifact_class.as_bytes());
    revision_hash_field(&mut hasher, cursor.to_string().as_bytes());
    match source_id {
        Some(source_id) => {
            revision_hash_field(&mut hasher, b"source-id:some");
            revision_hash_field(&mut hasher, source_id.as_bytes());
        }
        None => revision_hash_field(&mut hasher, b"source-id:none"),
    }
    match body {
        Some(body) => {
            revision_hash_field(&mut hasher, b"body:some");
            revision_hash_field(&mut hasher, body.as_bytes());
        }
        None => revision_hash_field(&mut hasher, b"body:none"),
    }
    format!("_fdb:m:{}", hex_encode(&hasher.finalize()))
}

fn projection_batch_has_no_custom_triggers(connection: &Connection) -> rusqlite::Result<bool> {
    let unexpected: bool = connection
        .prepare_cached(
            "SELECT EXISTS(
             SELECT 1 FROM sqlite_master
             WHERE type='trigger'
               AND tbl_name IN (
                   '_fathomdb_vector_rows','_fathomdb_projection_terminal',
                   '_fathomdb_embedder_profiles','operational_mutations',
                   '_fathomdb_open_state','_fathomdb_read_visibility_state',
                   'vector_default'
               )
               AND name NOT LIKE '_fathomdb_read_visibility_%'
             UNION ALL
             SELECT 1 FROM sqlite_temp_master
             WHERE type='trigger'
               AND tbl_name IN (
                   '_fathomdb_vector_rows','_fathomdb_projection_terminal',
                   '_fathomdb_embedder_profiles','operational_mutations',
                   '_fathomdb_open_state','_fathomdb_read_visibility_state',
                   'vector_default'
               )
               AND name NOT LIKE '_fathomdb_read_visibility_%'
         )",
        )?
        .query_row([], |row| row.get(0))?;
    Ok(!unexpected)
}

fn load_next_cursor(connection: &Connection) -> u64 {
    let nodes = max_cursor(connection, "canonical_nodes").unwrap_or(0);
    let edges = max_cursor(connection, "canonical_edges").unwrap_or(0);
    let mutations = max_cursor(connection, "operational_mutations").unwrap_or(0);
    let state = max_cursor(connection, "operational_state").unwrap_or(0);
    // TC-33: schema step 23 RECREATES `canonical_edges` (no data migration), so
    // the edge rows that used to hold the high-water mark are gone. Without this
    // term the allocator can hand out a cursor a PREVIOUS edge already used —
    // and stale `_fathomdb_projection_terminal` / `_fathomdb_vector_rows` / vec0
    // rows still key on it, so a brand-new row would be treated as
    // already-projected and never get indexed. Step 23 stashes the pre-drop
    // maximum here; folding it in keeps cursors monotonic across the migration.
    let reserved = reserved_write_cursor(connection);
    let closure_boundary = connection
        .query_row(
            "SELECT COALESCE(MAX(admitted_write_boundary),0) \
             FROM _fathomdb_dependency_closures",
            [],
            |row| row.get::<_, u64>(0),
        )
        .unwrap_or(0);
    nodes.max(edges).max(mutations).max(state).max(reserved).max(closure_boundary)
}

/// The write-cursor high-water mark reserved by schema step 23, or 0 when the
/// key is absent (fresh DB, or a DB that never had edges). Never fails the
/// caller: a missing/unparseable value degrades to 0, which is the pre-TC-33
/// behaviour.
fn reserved_write_cursor(connection: &Connection) -> u64 {
    connection
        .query_row(
            "SELECT value FROM _fathomdb_open_state WHERE key = ?1",
            params![fathomdb_schema::RESERVED_WRITE_CURSOR_KEY],
            |row| row.get::<_, String>(0),
        )
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .unwrap_or(0)
}

fn max_cursor(connection: &Connection, table: &str) -> rusqlite::Result<u64> {
    let sql = format!("SELECT COALESCE(MAX(write_cursor), 0) FROM {table}");
    connection.query_row(&sql, [], |row| row.get::<_, u64>(0))
}

/// Map a rusqlite error to its stable SQLite extended-code name.
///
/// Returns `None` for non-`SqliteFailure` variants (e.g. JSON conversion
/// failures, type mismatches at the rusqlite layer) — those are not
/// SQLite-internal events and should not be surfaced under
/// `EventSource::SqliteInternal`. The names returned here are the
/// canonical `SQLITE_*` symbol names from `sqlite3.h` and are stable
/// dispatch keys for AC-021 / AC-006 binding adapters.
///
/// Only the subset of codes the engine can reach in 0.6.0 is enumerated
/// — bare-extended-code matching covers the rest with a stable
/// `"SQLITE_UNKNOWN"` fallback so subscribers always see a typed code.
///
/// Diagnostic completeness for unmapped codes — **corrected 0.8.20 Slice 21a-2
/// (TC-57)**. This comment used to claim that when the helper returns
/// `"SQLITE_UNKNOWN"` the numeric extended code "is not lost — it remains on the
/// underlying `rusqlite::Error::SqliteFailure` carried in the engine error chain
/// that subscribers can inspect via `EngineError`'s `source()`". **That is
/// false.** There is no such chain: `EngineError::Storage` is a UNIT variant with
/// no payload and no `source()`, and `write_inner` drops the `rusqlite::Error`
/// immediately after emitting the lifecycle event. So for an unmapped code the
/// numeric value IS lost, and the only signal a host receives is the string
/// `"SQLITE_UNKNOWN"`.
///
/// Concretely: `SQLITE_BUSY_SNAPSHOT` (517) matches none of the PRIMARY constants
/// below — the match is on the EXTENDED value — so it reaches subscribers as
/// `"SQLITE_UNKNOWN"` and is unrecoverable from the public API. Restructuring the
/// error path so busy codes are distinguishable (and surfacing the numeric code as
/// a typed payload field) is candidate R2 of
/// `dev/design/0.8.20-tc57-write-race-characterization.md` §7, explicitly OUT of
/// scope for the 21a-2 fix and recorded here rather than silently carried.
fn sqlite_extended_code_name(err: &rusqlite::Error) -> Option<&'static str> {
    let sqlite_error = err.sqlite_error()?;
    let extended = sqlite_error.extended_code;
    Some(match extended {
        rusqlite::ffi::SQLITE_SCHEMA => "SQLITE_SCHEMA",
        rusqlite::ffi::SQLITE_BUSY => "SQLITE_BUSY",
        rusqlite::ffi::SQLITE_LOCKED => "SQLITE_LOCKED",
        rusqlite::ffi::SQLITE_CORRUPT => "SQLITE_CORRUPT",
        rusqlite::ffi::SQLITE_NOTADB => "SQLITE_NOTADB",
        rusqlite::ffi::SQLITE_IOERR => "SQLITE_IOERR",
        rusqlite::ffi::SQLITE_FULL => "SQLITE_FULL",
        rusqlite::ffi::SQLITE_READONLY => "SQLITE_READONLY",
        rusqlite::ffi::SQLITE_CONSTRAINT => "SQLITE_CONSTRAINT",
        rusqlite::ffi::SQLITE_MISUSE => "SQLITE_MISUSE",
        rusqlite::ffi::SQLITE_INTERRUPT => "SQLITE_INTERRUPT",
        rusqlite::ffi::SQLITE_NOMEM => "SQLITE_NOMEM",
        rusqlite::ffi::SQLITE_PERM => "SQLITE_PERM",
        rusqlite::ffi::SQLITE_ABORT => "SQLITE_ABORT",
        rusqlite::ffi::SQLITE_PROTOCOL => "SQLITE_PROTOCOL",
        rusqlite::ffi::SQLITE_RANGE => "SQLITE_RANGE",
        rusqlite::ffi::SQLITE_TOOBIG => "SQLITE_TOOBIG",
        rusqlite::ffi::SQLITE_MISMATCH => "SQLITE_MISMATCH",
        rusqlite::ffi::SQLITE_AUTH => "SQLITE_AUTH",
        rusqlite::ffi::SQLITE_NOTFOUND => "SQLITE_NOTFOUND",
        rusqlite::ffi::SQLITE_CANTOPEN => "SQLITE_CANTOPEN",
        _ => "SQLITE_UNKNOWN",
    })
}

fn sqlite_extended_code_name_from_int(extended: i32) -> &'static str {
    match extended {
        rusqlite::ffi::SQLITE_SCHEMA => "SQLITE_SCHEMA",
        rusqlite::ffi::SQLITE_BUSY => "SQLITE_BUSY",
        rusqlite::ffi::SQLITE_LOCKED => "SQLITE_LOCKED",
        rusqlite::ffi::SQLITE_CORRUPT => "SQLITE_CORRUPT",
        rusqlite::ffi::SQLITE_NOTADB => "SQLITE_NOTADB",
        rusqlite::ffi::SQLITE_IOERR => "SQLITE_IOERR",
        rusqlite::ffi::SQLITE_FULL => "SQLITE_FULL",
        rusqlite::ffi::SQLITE_READONLY => "SQLITE_READONLY",
        rusqlite::ffi::SQLITE_CONSTRAINT => "SQLITE_CONSTRAINT",
        rusqlite::ffi::SQLITE_MISUSE => "SQLITE_MISUSE",
        rusqlite::ffi::SQLITE_INTERRUPT => "SQLITE_INTERRUPT",
        rusqlite::ffi::SQLITE_NOMEM => "SQLITE_NOMEM",
        rusqlite::ffi::SQLITE_PERM => "SQLITE_PERM",
        rusqlite::ffi::SQLITE_ABORT => "SQLITE_ABORT",
        rusqlite::ffi::SQLITE_PROTOCOL => "SQLITE_PROTOCOL",
        rusqlite::ffi::SQLITE_RANGE => "SQLITE_RANGE",
        rusqlite::ffi::SQLITE_TOOBIG => "SQLITE_TOOBIG",
        rusqlite::ffi::SQLITE_MISMATCH => "SQLITE_MISMATCH",
        rusqlite::ffi::SQLITE_AUTH => "SQLITE_AUTH",
        rusqlite::ffi::SQLITE_NOTFOUND => "SQLITE_NOTFOUND",
        rusqlite::ffi::SQLITE_CANTOPEN => "SQLITE_CANTOPEN",
        _ => "SQLITE_UNKNOWN",
    }
}

fn map_open_sqlite_error(err: rusqlite::Error, stage: OpenStage) -> EngineOpenError {
    let Some(sqlite_error) = err.sqlite_error() else {
        return EngineOpenError::Io { message: "could not open database".to_string() };
    };
    match sqlite_error.extended_code {
        rusqlite::ffi::SQLITE_CORRUPT | rusqlite::ffi::SQLITE_NOTADB => {
            EngineOpenError::Corruption(CorruptionDetail {
                kind: match stage {
                    OpenStage::WalReplay => CorruptionKind::WalReplayFailure,
                    OpenStage::HeaderProbe => CorruptionKind::HeaderMalformed,
                    OpenStage::SchemaProbe => CorruptionKind::SchemaInconsistent,
                    OpenStage::EmbedderIdentity => CorruptionKind::EmbedderIdentityDrift,
                    OpenStage::ProjectionGeneration => CorruptionKind::ProjectionGenerationDrift,
                },
                stage,
                locator: CorruptionLocator::OpaqueSqliteError {
                    sqlite_extended_code: sqlite_error.extended_code,
                },
                recovery_hint: RecoveryHint {
                    code: match stage {
                        OpenStage::WalReplay => "E_CORRUPT_WAL_REPLAY",
                        OpenStage::HeaderProbe => "E_CORRUPT_HEADER",
                        OpenStage::SchemaProbe => "E_CORRUPT_SCHEMA",
                        OpenStage::EmbedderIdentity => "E_CORRUPT_EMBEDDER_IDENTITY",
                        OpenStage::ProjectionGeneration => "E_CORRUPT_PROJECTION_GENERATION",
                    },
                    doc_anchor: match stage {
                        OpenStage::WalReplay => "design/recovery.md#wal-replay-failures",
                        OpenStage::HeaderProbe => "design/recovery.md#header-malformed",
                        OpenStage::SchemaProbe => "design/recovery.md#schema-inconsistent",
                        OpenStage::EmbedderIdentity => "design/recovery.md#embedder-identity-drift",
                        OpenStage::ProjectionGeneration => {
                            "design/recovery-0.8.25.md#projection-generation"
                        }
                    },
                },
            })
        }
        _ => EngineOpenError::Io { message: "could not open database".to_string() },
    }
}

fn emit_open_error_event(subscriber: &Arc<dyn lifecycle::Subscriber>, err: &EngineOpenError) {
    if let EngineOpenError::Corruption(detail) = err {
        let code = match detail.locator {
            CorruptionLocator::OpaqueSqliteError { sqlite_extended_code } => {
                Some(sqlite_extended_code_name_from_int(sqlite_extended_code))
            }
            _ => None,
        };
        let event = lifecycle::Event {
            phase: lifecycle::Phase::Failed,
            source: lifecycle::EventSource::SqliteInternal,
            category: lifecycle::EventCategory::Corruption,
            code,
        };
        subscriber.on_event(&event);
    }
}

/// Install a `sqlite3_profile` callback on `connection` that dispatches
/// per-statement profile records and slow-statement signals to the
/// engine's subscriber registry.
///
/// Why FFI rather than `rusqlite::Connection::profile`: the safe API
/// (rusqlite 0.31) accepts only a `fn(&str, Duration)` with no
/// environment, so it cannot carry a per-engine subscriber-registry
/// pointer. We use `sqlite3_profile` directly with a leaked-into-`Box`
/// context whose pointer is tied to the engine's lifetime via
/// `Engine::profile_contexts`.
///
/// `sqlite3_profile` is documented as deprecated in favor of
/// `sqlite3_trace_v2`, but it remains supported and is sufficient for
/// the wall-clock + SQL-text payload required by AC-005a/b.
#[allow(clippy::vec_box)]
fn install_profile_callback(
    connection: &Connection,
    subscribers: &Arc<lifecycle::SubscriberRegistry>,
    profiling_enabled: &Arc<AtomicBool>,
    slow_threshold_ms: &Arc<AtomicU64>,
    contexts: &mut Vec<Box<ProfileContext>>,
) {
    let mut ctx = Box::new(ProfileContext {
        subscribers: Arc::clone(subscribers),
        profiling_enabled: Arc::clone(profiling_enabled),
        slow_threshold_ms: Arc::clone(slow_threshold_ms),
        #[cfg(test)]
        callback_uninstalled: AtomicBool::new(false),
    });
    let ctx_ptr: *mut ProfileContext = &mut *ctx;

    // SAFETY: the Box outlives the connection. Rust drops struct fields
    // in declaration order. `connection` and `reader_pool` are declared
    // before `profile_contexts`. `ReaderWorkerPool::Drop` joins every
    // reader worker, and each worker uninstalls and drops its owned
    // connection inside `reader_worker_loop` before the worker thread
    // returns. Therefore all connections — and SQLite's internal
    // profile-callback state with them — are torn down before the
    // `Box<ProfileContext>` allocations are freed. `Engine::close`
    // additionally clears the callback via
    // `sqlite3_profile(handle, None, NULL)` before connection close to
    // drain any in-flight callback dispatch.
    unsafe {
        rusqlite::ffi::sqlite3_profile(
            connection.handle(),
            Some(profile_callback_trampoline),
            ctx_ptr.cast::<std::ffi::c_void>(),
        );
    }
    contexts.push(ctx);
}

/// Uninstall the profile callback so SQLite stops calling into our
/// freed `Box<ProfileContext>` pointer once a connection is being torn
/// down. Call before dropping `profile_contexts`.
fn uninstall_profile_callback(connection: &Connection) {
    // SAFETY: passing `None` as the callback unregisters the previous
    // callback; SQLite documents this as legal and idempotent.
    unsafe {
        let previous =
            rusqlite::ffi::sqlite3_profile(connection.handle(), None, std::ptr::null_mut());
        #[cfg(test)]
        if !previous.is_null() {
            // The connection holds our stable context, retained until teardown completes.
            (*previous.cast::<ProfileContext>()).callback_uninstalled.store(true, Ordering::SeqCst);
        }
        #[cfg(not(test))]
        let _ = previous;
    }
}

/// Pack 6.G G.1 — configure SQLite per-connection lookaside on a reader
/// worker connection. Must be called BEFORE any statement is prepared
/// or any PRAGMA is run on `connection`; per the SQLite docs
/// (https://www.sqlite.org/malloc.html §3) lookaside is silently
/// ignored if reconfigured after the first allocation on the
/// connection. Passing `NULL` for the buffer pointer lets SQLite
/// allocate the lookaside backing memory itself.
///
/// rusqlite 0.31's `set_db_config` only handles the boolean
/// `DbConfig::*` variants; `SQLITE_DBCONFIG_LOOKASIDE` is not surfaced
/// (it is commented out in `rusqlite/src/config.rs`), so we call the
/// raw FFI directly.
///
/// Returns the rc of `sqlite3_db_config` so callers can debug-assert
/// `SQLITE_OK` and surface configuration failure under
/// `debug_assertions` test builds without expanding the public surface.
/// 0.7.0 perf-experiments hook: apply caller-supplied reader PRAGMAs
/// from the `FATHOMDB_PERF_READER_PRAGMAS` env var. Format:
/// comma-separated `name=value` pairs (e.g.
/// `cache_size=-262144,mmap_size=268435456,temp_store=MEMORY`).
///
/// **Gated on `FATHOMDB_PERF_EXPERIMENTS=1`.** No-op if the gate env
/// var is unset, so production paths are never affected. Failures to
/// apply individual PRAGMAs are logged to stderr (via `eprintln!`) but
/// do not error the connection open — experiments are best-effort,
/// not contract.
///
/// Scope: 0.7.0 perf-experiment campaign per
/// `dev/plans/0.7.0-perf-experiments.md`. Once Wave 5 picks the
/// landing combination, the chosen PRAGMAs are hardcoded as the new
/// reader-open default and this hook is removed.
/// 0.7.0 perf-experiments hook: apply writer-side PRAGMAs from
/// `FATHOMDB_PERF_WRITER_PRAGMAS` (same format as reader hook).
/// **Runs BEFORE migrations** so PRAGMAs like `page_size` that must
/// precede any table creation take effect on a fresh DB.
///
/// Gated on `FATHOMDB_PERF_EXPERIMENTS=1`. No-op otherwise.
fn apply_perf_experiment_writer_pragmas(connection: &Connection) {
    if std::env::var_os("FATHOMDB_PERF_EXPERIMENTS").is_none() {
        return;
    }
    let raw = match std::env::var("FATHOMDB_PERF_WRITER_PRAGMAS") {
        Ok(s) if !s.is_empty() => s,
        _ => return,
    };
    for entry in raw.split(',') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }
        let (name, value) = match entry.split_once('=') {
            Some((n, v)) => (n.trim(), v.trim()),
            None => {
                eprintln!("perf-experiment: bad writer pragma entry (expect name=value): {entry}");
                continue;
            }
        };
        if name.is_empty() {
            eprintln!("perf-experiment: empty pragma name in writer entry: {entry}");
            continue;
        }
        match connection.pragma_update(None, name, value) {
            Ok(()) => {
                eprintln!(
                    "perf-experiment: applied PRAGMA {name}={value} on writer (pre-migration)"
                );
            }
            Err(err) => {
                eprintln!("perf-experiment: writer PRAGMA {name}={value} failed: {err}");
            }
        }
    }
}

fn apply_perf_experiment_reader_pragmas(connection: &Connection) {
    if std::env::var_os("FATHOMDB_PERF_EXPERIMENTS").is_none() {
        return;
    }
    let raw = match std::env::var("FATHOMDB_PERF_READER_PRAGMAS") {
        Ok(s) if !s.is_empty() => s,
        _ => return,
    };
    for entry in raw.split(',') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }
        let (name, value) = match entry.split_once('=') {
            Some((n, v)) => (n.trim(), v.trim()),
            None => {
                eprintln!("perf-experiment: bad pragma entry (expect name=value): {entry}");
                continue;
            }
        };
        if name.is_empty() {
            eprintln!("perf-experiment: empty pragma name in entry: {entry}");
            continue;
        }
        match connection.pragma_update(None, name, value) {
            Ok(()) => {
                eprintln!("perf-experiment: applied PRAGMA {name}={value} on reader");
            }
            Err(err) => {
                eprintln!("perf-experiment: PRAGMA {name}={value} failed: {err}");
            }
        }
    }
}

fn configure_reader_lookaside(connection: &Connection) -> std::os::raw::c_int {
    // SAFETY: `connection.handle()` returns a valid `*mut sqlite3` for
    // the lifetime of `connection`. The variadic
    // `sqlite3_db_config(LOOKASIDE)` call expects three trailing
    // arguments of types `void*`, `int`, `int` — the prototype shape
    // documented in `sqlite3.h`. We pass a null buffer so SQLite owns
    // the lookaside backing allocation, and the slot size / count from
    // the G.1 constants. No allocations happen on the connection
    // before this call (reader open path is `Connection::open` ->
    // `configure_reader_lookaside` -> first PRAGMA).
    unsafe {
        rusqlite::ffi::sqlite3_db_config(
            connection.handle(),
            rusqlite::ffi::SQLITE_DBCONFIG_LOOKASIDE,
            std::ptr::null_mut::<std::ffi::c_void>(),
            READER_LOOKASIDE_SLOT_SIZE,
            READER_LOOKASIDE_SLOT_COUNT,
        )
    }
}

/// FFI trampoline for `sqlite3_profile`.
///
/// Invoked by SQLite at statement-finish with the SQL text and the
/// statement's wall-clock cost in nanoseconds. We dispatch a
/// `ProfileRecord` (when profiling is enabled) and a `SlowStatement`
/// signal (when `wall_clock_ms` exceeds the configured slow threshold).
///
/// Per `dev/design/lifecycle.md` § Public record shape, the public
/// payload exposes `wall_clock_ms`, `step_count`, and `cache_delta`.
/// `sqlite3_profile` does not surface per-statement step counts or
/// cache-hit deltas in its callback; we emit `0` for those fields and
/// document the hazard. AC-005b requires the fields be typed numeric,
/// not that they carry non-zero values for every backend.
unsafe extern "C" fn profile_callback_trampoline(
    user_data: *mut std::ffi::c_void,
    sql: *const std::os::raw::c_char,
    nanoseconds: u64,
) {
    if user_data.is_null() || sql.is_null() {
        return;
    }
    let ctx = unsafe { &*(user_data.cast::<ProfileContext>()) };
    let sql_text = match unsafe { std::ffi::CStr::from_ptr(sql) }.to_str() {
        Ok(s) => s,
        Err(_) => return,
    };

    #[cfg(feature = "test-hooks")]
    record_slice71_profile_statement_for_test(sql_text);

    let wall_clock_ms = nanoseconds / 1_000_000;

    if ctx.profiling_enabled.load(Ordering::Relaxed) {
        let record = lifecycle::ProfileRecord {
            wall_clock_ms,
            // step_count / cache_delta are not surfaced by
            // sqlite3_profile; placeholder 0 satisfies AC-005b's
            // "typed numeric" contract. A future profiling refactor
            // around sqlite3_stmt_status + sqlite3_db_status would
            // populate them with non-zero deltas.
            step_count: 0,
            cache_delta: 0,
        };
        ctx.subscribers.dispatch_profile(&record);
    }

    let threshold = ctx.slow_threshold_ms.load(Ordering::Relaxed);
    if wall_clock_ms > threshold {
        let signal = lifecycle::SlowStatement { statement: sql_text.to_string(), wall_clock_ms };
        ctx.subscribers.dispatch_slow_statement(&signal);
    }
}

#[cfg(test)]
mod slice20_fix1_tests;

#[cfg(test)]
mod slice90_close_tests;

#[cfg(test)]
mod tests {
    use super::reader_pool::ReaderRequest;
    use super::vector_storage::KIND_TO_SOURCE_TYPE_CASE_SQL;
    use super::{
        acquire_lock_without_metadata_mutation, derive_stable_id,
        install_admission_locked_hook_for_test, legacy_revision_id,
        native_connection_state_for_test, prepare_search_statement, resolve_source_type,
        retain_complete_rank_boundary_candidates, DeviceResolution, EmbedderChoice, Engine,
        EngineConfig, EngineError, EngineOpenError, IdSpace, IdSpaceKind, InitialState, LoaderInfo,
        ManagedConnectionRegistry, NativeTransactionState, PreparedWrite, ProjectionRuntime,
        ProjectionRuntimeStartupFaultForTest, ProjectionRuntimeStartupRole, RuntimeProbeConnection,
        SearchHit, SoftFallbackBranch, SourceId, WalAttributionCollector, WalAttributionRole,
        ERASURE_WAL_TRUNCATE_ATTEMPTS, PROJECTION_WORKERS, READER_POOL_SIZE, ROW_OWNED_PROJECTIONS,
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
