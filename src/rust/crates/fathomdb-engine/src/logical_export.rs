//! Internal Schema-39 logical archive export/restore (`fathomdb.logical-export/v1`).
//!
//! Gated strictly behind `#[cfg(any(test, feature = "test-hooks", feature = "operator"))]`
//! and never exposed on the default `fathomdb` or `fathomdb-sdk` facades.
//!
//! - Export runs all 4 pending physical-erasure gates atomically inside the same
//!   snapshot transaction used to read all `12` `TableExportClass::DurableAuthority`
//!   tables (plus optional `3` `OptInAudit` tables and `OptInVector` uncentered
//!   f32 vectors + `EmbedderIdentity`).
//! - Serialized archives strictly omit `embedding_bin`, `mean_vec`, and
//!   `_fathomdb_embed_probe`.
//! - Restore executes inside a single atomic `BEGIN IMMEDIATE` transaction on a
//!   fresh Schema-39 database, preserving exact original `write_cursor` values,
//!   rebuilding all `6` `DerivedProjection` tables via the real projection
//!   pipeline, re-deriving `mean_vec` iff `identity_requires_mean_centering` and
//!   vector count `>= MEAN_VEC_PIN_THRESHOLD` (`256`), quantizing `embedding_bin`
//!   via `vec_quantize_binary`, and synchronizing Schema-39 internal runtime state.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Display, Formatter};

use fathomdb_embedder_api::EmbedderIdentity;
use fathomdb_schema::{TableExportClass, SCHEMA_VERSION, TABLE_MANIFEST};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};

use crate::dependency::load_dependency_generation;
use crate::dependency_closure;
use crate::erasure::ERASURE_PENDING_REDACTION_COLLECTION;
use crate::erasure_maintenance;
use crate::export_manifest::audit_schema_39_sqlite_master;
use crate::index_projector::{
    canonical_node_rows, project_canonical_edge_row, project_canonical_node_row,
};
use crate::mean::{
    identity_requires_mean_centering, subtract_mean, MeanAccumulator, MEAN_VEC_PIN_THRESHOLD,
};
use crate::open::default_embedder_identity;
use crate::physical_erasure;
use crate::projection_commit::{
    advance_projection_cursor, record_projection_terminal, store_projection_cursor,
    ProjectionTerminalOwnerKind,
};
use crate::projection_generation;
use crate::projection_registry::{
    load_projection_registry, vector_attr_insert_fragments, ProjectionPass,
};
use crate::properties::decode_properties_row_col;
use crate::temporal::current_epoch_seconds;
use crate::vector_storage::{
    decode_vector_blob, encode_vector_blob, ensure_vector_partition, reconcile_vector_attr_columns,
    resolve_source_type_on, vector_eligibility_and_scope_for_cursor, vector_partition_dimension,
    DEFAULT_VECTOR_PARTITION, DEFAULT_VECTOR_PROFILE,
};
use crate::{register_sqlite_vec_extension, PropertyMap};

/// Canonical format discriminator for `fathomdb.logical-export/v1` archives.
pub const LOGICAL_EXPORT_FORMAT_V1: &str = "fathomdb.logical-export/v1";

/// Maximum serialized archive payload size accepted by [`decode_logical_export_v1`].
pub const LOGICAL_EXPORT_MAX_BYTES: usize = 256 * 1024 * 1024;

/// Maximum vector dimension accepted by [`decode_logical_export_v1`] and restore.
pub const LOGICAL_EXPORT_MAX_VECTOR_DIM: u32 = 4096;

/// Machine-readable error classification for logical export/restore operations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LogicalExportErrorCode {
    /// Archive `format_version` is not [`LOGICAL_EXPORT_FORMAT_V1`].
    UnsupportedFormatVersion,
    /// Source DB, target DB, or archive `schema_version` is not Schema 39.
    UnsupportedSchemaVersion,
    /// `sqlite_master` contains a table outside `TABLE_MANIFEST` / shadow patterns.
    UnclassifiedSchemaTable,
    /// At least one of the 4 pending physical-erasure conditions is active.
    PendingPhysicalErasure,
    /// Restore target database already contains authority, audit, or vector rows.
    TargetDatabaseNotEmpty,
    /// Serialized archive is truncated, invalid JSON/UTF-8, or violates invariants.
    MalformedArchive,
    /// Underlying SQLite or storage error.
    Storage,
}

impl LogicalExportErrorCode {
    /// Stable uppercase diagnostic code string.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UnsupportedFormatVersion => "E_LOGICAL_EXPORT_UNSUPPORTED_FORMAT",
            Self::UnsupportedSchemaVersion => "E_LOGICAL_EXPORT_UNSUPPORTED_SCHEMA",
            Self::UnclassifiedSchemaTable => "E_LOGICAL_EXPORT_UNCLASSIFIED_TABLE",
            Self::PendingPhysicalErasure => "E_LOGICAL_EXPORT_PENDING_ERASURE",
            Self::TargetDatabaseNotEmpty => "E_LOGICAL_EXPORT_TARGET_NOT_EMPTY",
            Self::MalformedArchive => "E_LOGICAL_EXPORT_MALFORMED_ARCHIVE",
            Self::Storage => "E_LOGICAL_EXPORT_STORAGE",
        }
    }
}

/// Typed refusal returned by `fathomdb.logical-export/v1` export, decode, and restore APIs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LogicalExportError {
    pub code: LogicalExportErrorCode,
    pub message: String,
}

impl LogicalExportError {
    fn new(code: LogicalExportErrorCode, message: impl Into<String>) -> Self {
        Self { code, message: message.into() }
    }

    fn storage(context: &str, err: impl Display) -> Self {
        Self::new(LogicalExportErrorCode::Storage, format!("{context}: {err}"))
    }
}

impl Display for LogicalExportError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for LogicalExportError {}

/// Opt-in class controls for [`export_logical_archive_v1`].
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LogicalExportOptionsV1 {
    /// Include the 3 `TableExportClass::OptInAudit` tables.
    pub include_audit: bool,
    /// Include `TableExportClass::OptInVector` uncentered f32 vectors + `EmbedderIdentity`.
    pub include_vectors: bool,
}

/// Summary returned by [`restore_logical_archive_v1`] after atomic commit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LogicalExportRestoreReportV1 {
    pub authority_rows_restored: u64,
    pub audit_rows_restored: u64,
    pub vectors_restored: u64,
    pub mean_vec_pinned: bool,
    pub serving_generation_id: String,
    pub projection_cursor_after: u64,
}

/// Versioned Schema-39 logical export archive (`fathomdb.logical-export/v1`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LogicalExportV1 {
    pub format_version: String,
    pub schema_version: u32,
    pub include_audit: bool,
    pub include_vectors: bool,
    pub physical_erasure_policy: String,
    pub dependency_generation: u64,
    pub authority: LogicalExportAuthorityTablesV1,
    pub audit: Option<LogicalExportAuditTablesV1>,
    pub vectors: Option<LogicalExportVectorsV1>,
}

/// All 12 `TableExportClass::DurableAuthority` tables.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LogicalExportAuthorityTablesV1 {
    pub canonical_nodes: Vec<CanonicalNodeExportRowV1>,
    pub canonical_edges: Vec<CanonicalEdgeExportRowV1>,
    pub operational_collections: Vec<OperationalCollectionExportRowV1>,
    pub operational_mutations: Vec<OperationalMutationExportRowV1>,
    pub operational_state: Vec<OperationalStateExportRowV1>,
    pub artifact_revisions: Vec<ArtifactRevisionExportRowV1>,
    pub source_versions: Vec<SourceVersionExportRowV1>,
    pub source_links: Vec<SourceLinkExportRowV1>,
    pub source_dependencies: Vec<SourceDependencyExportRowV1>,
    pub projection_registry: Vec<ProjectionRegistryExportRowV1>,
    pub vector_kinds: Vec<VectorKindExportRowV1>,
    pub commit_timestamps: Vec<CommitTimestampExportRowV1>,
    #[serde(default)]
    pub non_canonical_terminal_cursors: Vec<i64>,
}

/// All 3 `TableExportClass::OptInAudit` tables.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LogicalExportAuditTablesV1 {
    pub actuation_receipts: Vec<ActuationReceiptExportRowV1>,
    pub actuation_receipt_source_refs: Vec<ActuationReceiptSourceRefExportRowV1>,
    pub dependency_closures: Vec<DependencyClosureExportRowV1>,
}

/// `EmbedderIdentity` exported for `OptInVector` (strictly omits `mean_vec`).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LogicalExportEmbedderIdentityV1 {
    pub profile: String,
    pub name: String,
    pub revision: String,
    pub dimension: u32,
}

/// One exported uncentered f32 vector row joined with its mapping and span tuple
/// (strictly omits `embedding_bin` and `_fathomdb_embed_probe`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LogicalExportVectorRowV1 {
    pub rowid: i64,
    pub write_cursor: u64,
    pub kind: String,
    pub created_at: i64,
    pub window_number: i64,
    pub byte_start: i64,
    pub byte_end: Option<i64>,
    pub body_length: i64,
    pub embedding: Vec<f32>,
}

/// `TableExportClass::OptInVector` archive payload.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LogicalExportVectorsV1 {
    pub embedder: LogicalExportEmbedderIdentityV1,
    pub rows: Vec<LogicalExportVectorRowV1>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalNodeExportRowV1 {
    pub write_cursor: i64,
    pub kind: String,
    pub body: String,
    pub source_id: Option<String>,
    pub logical_id: Option<String>,
    pub superseded_at: Option<i64>,
    pub row_kind: String,
    pub importance: Option<f64>,
    pub state: String,
    pub reason: Option<String>,
    pub valid_from: Option<i64>,
    pub valid_until: Option<i64>,
    pub properties: Option<String>,
    pub scope: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalEdgeExportRowV1 {
    pub write_cursor: i64,
    pub kind: String,
    pub from_id: String,
    pub to_id: String,
    pub source_id: Option<String>,
    pub logical_id: Option<String>,
    pub superseded_at: Option<i64>,
    pub body: Option<String>,
    pub t_valid: Option<i64>,
    pub t_invalid: Option<i64>,
    pub confidence: Option<f64>,
    pub extractor_model_id: Option<String>,
    pub temporal_fallback: Option<i64>,
    pub properties: Option<String>,
    pub scope: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationalCollectionExportRowV1 {
    pub name: String,
    pub kind: String,
    pub schema_json: String,
    pub retention_json: String,
    pub format_version: i64,
    pub created_at: i64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationalMutationExportRowV1 {
    pub id: i64,
    pub collection_name: String,
    pub record_key: String,
    pub op_kind: String,
    pub payload_json: String,
    pub schema_id: Option<String>,
    pub write_cursor: i64,
    pub scope: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationalStateExportRowV1 {
    pub collection_name: String,
    pub record_key: String,
    pub payload_json: String,
    pub schema_id: Option<String>,
    pub write_cursor: i64,
    pub scope: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactRevisionExportRowV1 {
    pub schema_version: i64,
    pub revision_id: String,
    pub artifact_class: String,
    pub write_cursor: i64,
    pub artifact_role: String,
    pub completeness: String,
    pub scope: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceVersionExportRowV1 {
    pub schema_version: i64,
    pub source_id: String,
    pub source_version_id: String,
    pub source_revision_id: String,
    pub scope: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceLinkExportRowV1 {
    pub schema_version: i64,
    pub artifact_revision_id: String,
    pub source_id: String,
    pub source_version_id: String,
    pub source_revision_id: String,
    pub locator_kind: String,
    pub start_byte: Option<i64>,
    pub end_byte: Option<i64>,
    pub hash_algorithm: String,
    pub hash_digest: String,
    pub scope: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceDependencyExportRowV1 {
    pub schema_version: i64,
    pub dependency_id: String,
    pub derived_revision_id: String,
    pub registered_dependency_generation: i64,
    pub scope: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectionRegistryExportRowV1 {
    pub name: String,
    pub roles: String,
    pub fts_tokenizer: Option<String>,
    pub vector_embedder: Option<String>,
    pub vector_declared: i64,
    pub source: Option<String>,
    pub entity_class: String,
    pub vector_kinds: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VectorKindExportRowV1 {
    pub kind: String,
    pub profile: String,
    pub created_at: i64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommitTimestampExportRowV1 {
    pub write_cursor: i64,
    pub committed_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActuationReceiptExportRowV1 {
    pub operation_id: String,
    pub schema_version: i64,
    pub request_sha256: Option<String>,
    pub operations_count: Option<i64>,
    pub outcome: String,
    pub refused_operation_index: Option<i64>,
    pub refused_field_path: Option<String>,
    pub reason_codes_json: String,
    pub affected_revision_ids_json: String,
    pub resulting_write_boundary: Option<i64>,
    pub resulting_dependency_generation: Option<i64>,
    pub pending_projection_write_cursors_json: String,
    pub closure_operation_ids_json: String,
    pub projection_generation_id: Option<String>,
    pub scope: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActuationReceiptSourceRefExportRowV1 {
    pub operation_id: String,
    pub schema_version: i64,
    pub ref_kind: String,
    pub ref_value: String,
    pub scope: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DependencyClosureExportRowV1 {
    pub schema_version: i64,
    pub closure_operation_id: String,
    pub root_kind: String,
    pub root_value: String,
    pub cause: String,
    pub effective_at_epoch_s: i64,
    pub admitted_write_boundary: i64,
    pub admitted_dependency_generation: i64,
    pub closure_sequence: i64,
    pub retry_fingerprint: String,
    pub phase: String,
    pub affected_count: i64,
    pub blocker_code: Option<String>,
    pub structural_proof_write_boundary: Option<i64>,
    pub proof_json: Option<String>,
    pub scope: String,
}

/// Encode a validated [`LogicalExportV1`] archive deterministically to UTF-8 JSON bytes.
pub fn encode_logical_export_v1(archive: &LogicalExportV1) -> Result<Vec<u8>, LogicalExportError> {
    validate_logical_export_v1(archive)?;
    serde_json::to_vec(archive).map_err(|err| {
        LogicalExportError::new(
            LogicalExportErrorCode::MalformedArchive,
            format!("failed to serialize logical archive: {err}"),
        )
    })
}

/// Decode and strictly validate serialized `fathomdb.logical-export/v1` archive bytes.
pub fn decode_logical_export_v1(bytes: &[u8]) -> Result<LogicalExportV1, LogicalExportError> {
    if bytes.is_empty() || bytes.len() > LOGICAL_EXPORT_MAX_BYTES {
        return Err(LogicalExportError::new(
            LogicalExportErrorCode::MalformedArchive,
            format!("archive byte length {} is outside valid bounds", bytes.len()),
        ));
    }
    let text = std::str::from_utf8(bytes).map_err(|err| {
        LogicalExportError::new(
            LogicalExportErrorCode::MalformedArchive,
            format!("archive payload is not valid UTF-8: {err}"),
        )
    })?;
    let archive: LogicalExportV1 = serde_json::from_str(text).map_err(|err| {
        LogicalExportError::new(
            LogicalExportErrorCode::MalformedArchive,
            format!("archive JSON decode failed: {err}"),
        )
    })?;
    validate_logical_export_v1(&archive)?;
    Ok(archive)
}

/// Validate structural and semantic invariants of a [`LogicalExportV1`] archive.
pub fn validate_logical_export_v1(archive: &LogicalExportV1) -> Result<(), LogicalExportError> {
    if archive.format_version != LOGICAL_EXPORT_FORMAT_V1 {
        return Err(LogicalExportError::new(
            LogicalExportErrorCode::UnsupportedFormatVersion,
            format!(
                "expected format_version '{}', found '{}'",
                LOGICAL_EXPORT_FORMAT_V1, archive.format_version
            ),
        ));
    }
    if archive.schema_version != SCHEMA_VERSION {
        return Err(LogicalExportError::new(
            LogicalExportErrorCode::UnsupportedSchemaVersion,
            format!("expected schema_version {}, found {}", SCHEMA_VERSION, archive.schema_version),
        ));
    }
    if archive.include_audit != archive.audit.is_some() {
        return Err(LogicalExportError::new(
            LogicalExportErrorCode::MalformedArchive,
            "include_audit flag does not match audit payload presence",
        ));
    }
    if archive.include_vectors != archive.vectors.is_some() {
        return Err(LogicalExportError::new(
            LogicalExportErrorCode::MalformedArchive,
            "include_vectors flag does not match vectors payload presence",
        ));
    }
    if !matches!(archive.physical_erasure_policy.as_str(), "autonomous" | "explicit_maintenance") {
        return Err(LogicalExportError::new(
            LogicalExportErrorCode::MalformedArchive,
            format!("invalid physical_erasure_policy '{}'", archive.physical_erasure_policy),
        ));
    }
    if archive.dependency_generation > i64::MAX as u64 {
        return Err(LogicalExportError::new(
            LogicalExportErrorCode::MalformedArchive,
            "dependency_generation exceeds i64::MAX",
        ));
    }
    for row in &archive.authority.canonical_nodes {
        if row.write_cursor <= 0 {
            return Err(LogicalExportError::new(
                LogicalExportErrorCode::MalformedArchive,
                "canonical_nodes.write_cursor must be positive",
            ));
        }
        if let Some(imp) = row.importance {
            if !imp.is_finite() {
                return Err(LogicalExportError::new(
                    LogicalExportErrorCode::MalformedArchive,
                    "canonical_nodes.importance must be finite",
                ));
            }
        }
    }
    for row in &archive.authority.canonical_edges {
        if row.write_cursor <= 0 {
            return Err(LogicalExportError::new(
                LogicalExportErrorCode::MalformedArchive,
                "canonical_edges.write_cursor must be positive",
            ));
        }
        if let Some(conf) = row.confidence {
            if !conf.is_finite() {
                return Err(LogicalExportError::new(
                    LogicalExportErrorCode::MalformedArchive,
                    "canonical_edges.confidence must be finite",
                ));
            }
        }
    }
    for row in &archive.authority.operational_mutations {
        if row.collection_name == ERASURE_PENDING_REDACTION_COLLECTION {
            return Err(LogicalExportError::new(
                LogicalExportErrorCode::PendingPhysicalErasure,
                "archive contains pending telemetry redaction mutations",
            ));
        }
        if row.write_cursor <= 0 {
            return Err(LogicalExportError::new(
                LogicalExportErrorCode::MalformedArchive,
                "operational_mutations.write_cursor must be positive",
            ));
        }
    }
    for row in &archive.authority.operational_state {
        if row.write_cursor <= 0 {
            return Err(LogicalExportError::new(
                LogicalExportErrorCode::MalformedArchive,
                "operational_state.write_cursor must be positive",
            ));
        }
    }
    for row in &archive.authority.commit_timestamps {
        if row.write_cursor <= 0 {
            return Err(LogicalExportError::new(
                LogicalExportErrorCode::MalformedArchive,
                "_fathomdb_commit_timestamps.write_cursor must be positive",
            ));
        }
    }
    for &cursor in &archive.authority.non_canonical_terminal_cursors {
        if cursor <= 0 {
            return Err(LogicalExportError::new(
                LogicalExportErrorCode::MalformedArchive,
                "non_canonical_terminal_cursors entries must be positive",
            ));
        }
    }
    for row in &archive.authority.source_dependencies {
        if row.registered_dependency_generation <= 0
            || (row.registered_dependency_generation as u64) > archive.dependency_generation
        {
            return Err(LogicalExportError::new(
                LogicalExportErrorCode::MalformedArchive,
                "source_dependencies.registered_dependency_generation exceeds archive dependency_generation",
            ));
        }
    }
    if let Some(audit) = &archive.audit {
        for closure in &audit.dependency_closures {
            if matches!(closure.cause.as_str(), "purged" | "source_erased")
                && closure.phase != "complete"
            {
                return Err(LogicalExportError::new(
                    LogicalExportErrorCode::PendingPhysicalErasure,
                    "archive contains non-complete physical dependency closure",
                ));
            }
        }
    }
    if let Some(vectors) = &archive.vectors {
        if vectors.embedder.profile != DEFAULT_VECTOR_PROFILE
            || vectors.embedder.name.is_empty()
            || vectors.embedder.revision.is_empty()
            || vectors.embedder.dimension == 0
            || vectors.embedder.dimension > LOGICAL_EXPORT_MAX_VECTOR_DIM
        {
            return Err(LogicalExportError::new(
                LogicalExportErrorCode::MalformedArchive,
                "invalid OptInVector embedder identity or dimension",
            ));
        }
        let dim = vectors.embedder.dimension as usize;
        let mut seen_rowids = BTreeSet::new();
        for row in &vectors.rows {
            if row.rowid <= 0 || row.write_cursor == 0 || row.write_cursor > i64::MAX as u64 {
                return Err(LogicalExportError::new(
                    LogicalExportErrorCode::MalformedArchive,
                    "invalid vector rowid or write_cursor",
                ));
            }
            if !seen_rowids.insert(row.rowid) {
                return Err(LogicalExportError::new(
                    LogicalExportErrorCode::MalformedArchive,
                    format!("duplicate vector rowid {}", row.rowid),
                ));
            }
            if row.window_number < 0
                || row.byte_start < 0
                || row.body_length < 0
                || row.byte_start > row.body_length
                || row.byte_end.is_some_and(|end| end < row.byte_start || end > row.body_length)
            {
                return Err(LogicalExportError::new(
                    LogicalExportErrorCode::MalformedArchive,
                    "invalid vector span bounds",
                ));
            }
            if row.embedding.len() != dim || row.embedding.iter().any(|v| !v.is_finite()) {
                return Err(LogicalExportError::new(
                    LogicalExportErrorCode::MalformedArchive,
                    "vector embedding dimension mismatch or non-finite value",
                ));
            }
        }
    }
    Ok(())
}

/// Verify all 4 pending physical-erasure conditions atomically on an active snapshot transaction.
fn check_no_pending_physical_erasures_in_snapshot(
    tx: &Connection,
) -> Result<(), LogicalExportError> {
    let pending_closures = dependency_closure::pending_physical_closures(tx)
        .map_err(|err| LogicalExportError::storage("inspect pending physical closures", err))?;
    let active_physical = physical_erasure::active(tx)
        .map_err(|err| LogicalExportError::storage("inspect physical erasure active state", err))?;
    let physical_diagnostics = physical_erasure::has_diagnostics(tx)
        .map_err(|err| LogicalExportError::storage("inspect physical erasure diagnostics", err))?;
    let non_terminal_requests: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM _fathomdb_erasure_requests \
             WHERE phase NOT IN ('complete', 'withdrawn_before_removal')",
            [],
            |row| row.get(0),
        )
        .map_err(|err| LogicalExportError::storage("inspect _fathomdb_erasure_requests", err))?;
    let pending_redactions: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM operational_mutations WHERE collection_name = ?1",
            [ERASURE_PENDING_REDACTION_COLLECTION],
            |row| row.get(0),
        )
        .map_err(|err| LogicalExportError::storage("inspect pending redaction queue", err))?;

    if !pending_closures.is_empty()
        || active_physical
        || physical_diagnostics
        || non_terminal_requests > 0
        || pending_redactions > 0
    {
        return Err(LogicalExportError::new(
            LogicalExportErrorCode::PendingPhysicalErasure,
            format!(
                "refusing logical export with pending physical erasure obligations \
                 (pending_closures={}, active_physical={}, physical_diagnostics={}, \
                  non_terminal_requests={}, pending_redactions={})",
                pending_closures.len(),
                active_physical,
                physical_diagnostics,
                non_terminal_requests,
                pending_redactions,
            ),
        ));
    }
    Ok(())
}

/// Export a Schema-39 database into a deterministic `fathomdb.logical-export/v1`
/// archive inside a single consistent SQLite snapshot transaction.
pub fn export_logical_archive_v1(
    conn: &mut Connection,
    options: LogicalExportOptionsV1,
) -> Result<LogicalExportV1, LogicalExportError> {
    register_sqlite_vec_extension();
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Deferred)
        .map_err(|err| LogicalExportError::storage("begin export snapshot", err))?;

    // First read pins the MVCC snapshot for the entire export (all 4 erasure gates + table reads).
    let user_version: u32 = tx
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|err| LogicalExportError::storage("read user_version", err))?;
    if user_version != SCHEMA_VERSION {
        return Err(LogicalExportError::new(
            LogicalExportErrorCode::UnsupportedSchemaVersion,
            format!("logical export requires Schema {SCHEMA_VERSION}, found {user_version}"),
        ));
    }

    // Verify complete TableExportClass coverage across sqlite_master.
    if let Err(unclassified) = audit_schema_39_sqlite_master(&tx) {
        return Err(LogicalExportError::new(
            LogicalExportErrorCode::UnclassifiedSchemaTable,
            format!("unclassified tables in sqlite_master: {}", unclassified.join(", ")),
        ));
    }

    // Atomically enforce all 4 pending physical-erasure checks on the same snapshot.
    check_no_pending_physical_erasures_in_snapshot(&tx)?;

    let policy = erasure_maintenance::read_policy(&tx)
        .map_err(|err| LogicalExportError::storage("read physical erasure policy", err))?;
    let dependency_generation = load_dependency_generation(&tx)
        .map_err(|err| LogicalExportError::storage("read dependency generation", err))?;

    // Export all 12 DurableAuthority tables in deterministic order.
    let authority = LogicalExportAuthorityTablesV1 {
        canonical_nodes: export_canonical_nodes(&tx)?,
        canonical_edges: export_canonical_edges(&tx)?,
        operational_collections: export_operational_collections(&tx)?,
        operational_mutations: export_operational_mutations(&tx)?,
        operational_state: export_operational_state(&tx)?,
        artifact_revisions: export_artifact_revisions(&tx)?,
        source_versions: export_source_versions(&tx)?,
        source_links: export_source_links(&tx)?,
        source_dependencies: export_source_dependencies(&tx)?,
        projection_registry: export_projection_registry(&tx)?,
        vector_kinds: export_vector_kinds(&tx)?,
        commit_timestamps: export_commit_timestamps(&tx)?,
        non_canonical_terminal_cursors: export_non_canonical_terminal_cursors(&tx)?,
    };

    let audit = if options.include_audit {
        Some(LogicalExportAuditTablesV1 {
            actuation_receipts: export_actuation_receipts(&tx)?,
            actuation_receipt_source_refs: export_actuation_receipt_source_refs(&tx)?,
            dependency_closures: export_dependency_closures(&tx)?,
        })
    } else {
        None
    };

    let vectors = if options.include_vectors { Some(export_vectors(&tx)?) } else { None };

    let archive = LogicalExportV1 {
        format_version: LOGICAL_EXPORT_FORMAT_V1.to_string(),
        schema_version: SCHEMA_VERSION,
        include_audit: options.include_audit,
        include_vectors: options.include_vectors,
        physical_erasure_policy: policy.wire().to_string(),
        dependency_generation,
        authority,
        audit,
        vectors,
    };
    validate_logical_export_v1(&archive)?;
    Ok(archive)
}

/// Atomically restore a validated `fathomdb.logical-export/v1` archive into a
/// fresh, empty Schema-39 database.
pub fn restore_logical_archive_v1(
    conn: &mut Connection,
    archive: &LogicalExportV1,
) -> Result<LogicalExportRestoreReportV1, LogicalExportError> {
    validate_logical_export_v1(archive)?;
    register_sqlite_vec_extension();

    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|err| LogicalExportError::storage("begin restore transaction", err))?;

    let user_version: u32 = tx
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|err| LogicalExportError::storage("read target user_version", err))?;
    if user_version != SCHEMA_VERSION {
        return Err(LogicalExportError::new(
            LogicalExportErrorCode::UnsupportedSchemaVersion,
            format!(
                "restore requires fresh Schema {SCHEMA_VERSION} database, found {user_version}"
            ),
        ));
    }

    if let Err(unclassified) = audit_schema_39_sqlite_master(&tx) {
        return Err(LogicalExportError::new(
            LogicalExportErrorCode::UnclassifiedSchemaTable,
            format!("unclassified target tables in sqlite_master: {}", unclassified.join(", ")),
        ));
    }

    ensure_target_database_empty(&tx)?;

    // 1. Insert all 12 DurableAuthority tables preserving exact original write_cursors.
    let mut authority_rows_restored: u64 = 0;
    authority_rows_restored +=
        insert_projection_registry(&tx, &archive.authority.projection_registry)?;
    authority_rows_restored += insert_vector_kinds(&tx, &archive.authority.vector_kinds)?;
    authority_rows_restored +=
        insert_operational_collections(&tx, &archive.authority.operational_collections)?;
    authority_rows_restored += insert_canonical_nodes(&tx, &archive.authority.canonical_nodes)?;
    authority_rows_restored += insert_canonical_edges(&tx, &archive.authority.canonical_edges)?;
    authority_rows_restored +=
        insert_operational_mutations(&tx, &archive.authority.operational_mutations)?;
    authority_rows_restored += insert_operational_state(&tx, &archive.authority.operational_state)?;
    authority_rows_restored +=
        insert_artifact_revisions(&tx, &archive.authority.artifact_revisions)?;
    authority_rows_restored += insert_source_versions(&tx, &archive.authority.source_versions)?;
    authority_rows_restored += insert_source_links(&tx, &archive.authority.source_links)?;
    authority_rows_restored +=
        insert_source_dependencies(&tx, &archive.authority.source_dependencies)?;
    authority_rows_restored += insert_commit_timestamps(&tx, &archive.authority.commit_timestamps)?;

    // 2. Insert dependency closures BEFORE projection rebuild so closure eligibility
    //    gates apply identically to the source store.
    let mut audit_rows_restored: u64 = 0;
    let mut max_closure_sequence: i64 = 0;
    if let Some(audit) = &archive.audit {
        for row in &audit.dependency_closures {
            max_closure_sequence = max_closure_sequence.max(row.closure_sequence);
        }
        audit_rows_restored += insert_dependency_closures(&tx, &audit.dependency_closures)?;
    }

    // 3. Rebuild all 6 DerivedProjection tables via the real engine projection pipeline.
    let registry = load_projection_registry(&tx)
        .map_err(|err| LogicalExportError::storage("load projection registry", err))?;
    let now_epoch = current_epoch_seconds();
    let mut canonical_cursors: BTreeSet<u64> = BTreeSet::new();
    for row in canonical_node_rows(&tx)
        .map_err(|err| LogicalExportError::storage("load canonical_nodes for projection", err))?
    {
        canonical_cursors.insert(row.cursor);
        let eligible =
            dependency_closure::projection_owner_is_eligible_at(&tx, row.cursor, now_epoch)
                .map_err(|err| {
                    LogicalExportError::storage("check node projection eligibility", err)
                })?;
        if !eligible {
            continue;
        }
        project_canonical_node_row(
            &tx,
            row.cursor,
            &row.kind,
            &row.body,
            row.row_kind,
            ProjectionPass::Write,
            row.attr_projected,
            row.properties.as_ref(),
            Some(&registry),
            None,
            None,
            None,
            None,
        )
        .map_err(|err| LogicalExportError::storage("project canonical node", err))?;
    }

    struct EdgeRebuildRow {
        cursor: u64,
        kind: String,
        body: Option<String>,
        superseded_at: Option<i64>,
        properties: Option<PropertyMap>,
    }
    let edge_rows: Vec<EdgeRebuildRow> = {
        let mut stmt = tx
            .prepare(
                "SELECT write_cursor, kind, body, superseded_at, properties \
                 FROM canonical_edges ORDER BY write_cursor ASC",
            )
            .map_err(|err| LogicalExportError::storage("prepare edge projection scan", err))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(EdgeRebuildRow {
                    cursor: row.get(0)?,
                    kind: row.get(1)?,
                    body: row.get(2)?,
                    superseded_at: row.get(3)?,
                    properties: decode_properties_row_col(row, 4)?,
                })
            })
            .map_err(|err| LogicalExportError::storage("query edges for projection", err))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|err| LogicalExportError::storage("collect edges for projection", err))?;
        rows
    };
    for edge in edge_rows {
        canonical_cursors.insert(edge.cursor);
        let eligible =
            dependency_closure::projection_owner_is_eligible_at(&tx, edge.cursor, now_epoch)
                .map_err(|err| {
                    LogicalExportError::storage("check edge projection eligibility", err)
                })?;
        if !eligible {
            continue;
        }
        project_canonical_edge_row(
            &tx,
            edge.cursor,
            &edge.kind,
            edge.body.as_deref(),
            ProjectionPass::Write,
            edge.properties.as_ref(),
            edge.superseded_at.is_none(),
            Some(&registry),
            None,
            None,
        )
        .map_err(|err| LogicalExportError::storage("project canonical edge", err))?;
    }

    // Record `non_canonical` terminal rows for non-canonical write cursors,
    // including orphaned non-canonical cursors (multi-op AdminSchema, overwritten
    // LatestState, retention-pruned AppendOnlyLog) exported from
    // `_fathomdb_projection_terminal`.
    let mut non_canonical_cursors: BTreeSet<u64> = BTreeSet::new();
    for &cursor in &archive.authority.non_canonical_terminal_cursors {
        let cursor = cursor as u64;
        if !canonical_cursors.contains(&cursor) {
            non_canonical_cursors.insert(cursor);
        }
    }
    for row in &archive.authority.operational_mutations {
        let cursor = row.write_cursor as u64;
        if !canonical_cursors.contains(&cursor) {
            non_canonical_cursors.insert(cursor);
        }
    }
    for row in &archive.authority.operational_state {
        let cursor = row.write_cursor as u64;
        if !canonical_cursors.contains(&cursor) {
            non_canonical_cursors.insert(cursor);
        }
    }
    for row in &archive.authority.commit_timestamps {
        let cursor = row.write_cursor as u64;
        if !canonical_cursors.contains(&cursor) {
            non_canonical_cursors.insert(cursor);
        }
    }
    for cursor in non_canonical_cursors {
        record_projection_terminal(
            &tx,
            cursor,
            "up_to_date",
            ProjectionTerminalOwnerKind::NonCanonical,
        )
        .map_err(|err| {
            LogicalExportError::storage("record non_canonical projection terminal", err)
        })?;
    }

    // 4. Restore OptInVector (or ensure default embedder profile & vector_default shape).
    let (vectors_restored, mean_vec_pinned) =
        restore_vectors_in_tx(&tx, archive.vectors.as_ref(), &canonical_cursors)?;

    // 5. Synchronize Schema-39 internal metadata (_fathomdb_open_state,
    //    _fathomdb_projection_generations, _fathomdb_read_visibility_state,
    //    _fathomdb_scope_visibility_watermarks).
    synchronize_open_state(&tx, archive, max_closure_sequence)?;

    // Recompute projection cursor watermark from terminal records.
    store_projection_cursor(&tx, 0)
        .map_err(|err| LogicalExportError::storage("reset projection cursor", err))?;
    let projection_cursor_after = advance_projection_cursor(&tx)
        .map_err(|err| LogicalExportError::storage("advance projection cursor", err))?;

    // Freshly restored database has zero deleted/retained FTS terms.
    physical_erasure::mark_clean_in_tx(&tx)
        .map_err(|err| LogicalExportError::storage("mark restored FTS sanitation clean", err))?;

    // Install serving projection generation matching restored registry + embedder profile.
    let serving_generation_id =
        projection_generation::install_restored_generation(&tx).map_err(|err| {
            LogicalExportError::storage("install restored projection generation", err)
        })?;

    // 6. Insert actuation receipts + source refs AFTER serving generation exists so
    //    non-null projection_generation_id binds cleanly to the restored serving generation.
    if let Some(audit) = &archive.audit {
        audit_rows_restored += insert_actuation_receipts(
            &tx,
            &audit.actuation_receipts,
            serving_generation_id.as_str(),
        )?;
        audit_rows_restored +=
            insert_actuation_receipt_source_refs(&tx, &audit.actuation_receipt_source_refs)?;
    }

    // 7. Refresh visibility nonces and ensure coalescing = 0.
    synchronize_visibility_state(&tx)?;

    tx.commit().map_err(|err| LogicalExportError::storage("commit restore transaction", err))?;

    Ok(LogicalExportRestoreReportV1 {
        authority_rows_restored,
        audit_rows_restored,
        vectors_restored,
        mean_vec_pinned,
        serving_generation_id: serving_generation_id.as_str().to_string(),
        projection_cursor_after,
    })
}

fn ensure_target_database_empty(tx: &Connection) -> Result<(), LogicalExportError> {
    for entry in TABLE_MANIFEST {
        match entry.export_class {
            TableExportClass::DurableAuthority | TableExportClass::OptInAudit => {
                // Migration 004 seeds the built-in `projection_failures` operational
                // collection on every fresh Schema-39 database; treat that single
                // schema-seeded row as part of an empty target database.
                let sql = if entry.name == "operational_collections" {
                    "SELECT COUNT(*) FROM operational_collections WHERE name != 'projection_failures'"
                        .to_string()
                } else {
                    format!("SELECT COUNT(*) FROM {}", entry.name)
                };
                let count: i64 = tx
                    .query_row(&sql, [], |row| row.get(0))
                    .map_err(|err| LogicalExportError::storage("check empty target table", err))?;
                if count != 0 {
                    return Err(LogicalExportError::new(
                        LogicalExportErrorCode::TargetDatabaseNotEmpty,
                        format!("target table '{}' already contains {count} row(s)", entry.name),
                    ));
                }
            }
            TableExportClass::OptInVector
                if matches!(entry.name, "_fathomdb_vector_rows" | "_fathomdb_vector_spans") =>
            {
                let count: i64 = tx
                    .query_row(&format!("SELECT COUNT(*) FROM {}", entry.name), [], |row| {
                        row.get(0)
                    })
                    .map_err(|err| {
                        LogicalExportError::storage("check empty vector metadata", err)
                    })?;
                if count != 0 {
                    return Err(LogicalExportError::new(
                        LogicalExportErrorCode::TargetDatabaseNotEmpty,
                        format!("target table '{}' already contains {count} row(s)", entry.name),
                    ));
                }
            }
            _ => {}
        }
    }
    let vec_exists: bool = tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
            [DEFAULT_VECTOR_PARTITION],
            |row| row.get(0),
        )
        .map_err(|err| LogicalExportError::storage("check vector_default existence", err))?;
    if vec_exists {
        let count: i64 = tx
            .query_row(&format!("SELECT COUNT(*) FROM {DEFAULT_VECTOR_PARTITION}"), [], |row| {
                row.get(0)
            })
            .map_err(|err| LogicalExportError::storage("check empty vector_default", err))?;
        if count != 0 {
            return Err(LogicalExportError::new(
                LogicalExportErrorCode::TargetDatabaseNotEmpty,
                format!("target '{DEFAULT_VECTOR_PARTITION}' already contains {count} row(s)"),
            ));
        }
    }
    Ok(())
}

fn restore_vectors_in_tx(
    tx: &Connection,
    vectors: Option<&LogicalExportVectorsV1>,
    canonical_cursors: &BTreeSet<u64>,
) -> Result<(u64, bool), LogicalExportError> {
    // Always leave `_fathomdb_embed_probe` empty so the next Engine open with an
    // embedder cleanly collects and persists fresh probe baselines.
    tx.execute("DELETE FROM _fathomdb_embed_probe", [])
        .map_err(|err| LogicalExportError::storage("clear _fathomdb_embed_probe", err))?;

    let Some(vectors) = vectors else {
        // Ensure a valid default embedder profile and empty vector_default exist.
        let profile_exists: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM _fathomdb_embedder_profiles WHERE profile = ?1)",
                [DEFAULT_VECTOR_PROFILE],
                |row| row.get(0),
            )
            .map_err(|err| LogicalExportError::storage("check default embedder profile", err))?;
        let dim = if profile_exists {
            tx.query_row(
                "SELECT dimension FROM _fathomdb_embedder_profiles WHERE profile = ?1",
                [DEFAULT_VECTOR_PROFILE],
                |row| row.get::<_, u32>(0),
            )
            .map_err(|err| LogicalExportError::storage("read default embedder dimension", err))?
        } else {
            let def = default_embedder_identity();
            tx.execute(
                "INSERT INTO _fathomdb_embedder_profiles(profile, name, revision, dimension, mean_vec) \
                 VALUES(?1, ?2, ?3, ?4, NULL)",
                params![DEFAULT_VECTOR_PROFILE, def.name, def.revision, def.dimension],
            )
            .map_err(|err| LogicalExportError::storage("seed default embedder profile", err))?;
            def.dimension
        };
        ensure_vector_partition(tx, dim)
            .map_err(|err| LogicalExportError::storage("ensure vector_default", err))?;
        reconcile_vector_attr_columns(tx, dim)
            .map_err(|err| LogicalExportError::storage("reconcile vector attr columns", err))?;
        return Ok((0, false));
    };

    let identity = EmbedderIdentity::new(
        vectors.embedder.name.clone(),
        vectors.embedder.revision.clone(),
        vectors.embedder.dimension,
    );
    let dim = identity.dimension;
    let should_pin_mean = identity_requires_mean_centering(&identity)
        && (vectors.rows.len() as u64) >= MEAN_VEC_PIN_THRESHOLD;
    let derived_mean: Option<Vec<f32>> = if should_pin_mean {
        let mut acc = MeanAccumulator::new(dim as usize);
        for row in &vectors.rows {
            acc.add(&row.embedding);
        }
        Some(acc.materialize())
    } else {
        None
    };
    let mean_blob: Option<Vec<u8>> = derived_mean.as_deref().map(encode_vector_blob);

    tx.execute(
        "INSERT OR REPLACE INTO _fathomdb_embedder_profiles(profile, name, revision, dimension, mean_vec) \
         VALUES(?1, ?2, ?3, ?4, ?5)",
        params![DEFAULT_VECTOR_PROFILE, identity.name, identity.revision, dim, mean_blob],
    )
    .map_err(|err| LogicalExportError::storage("write _fathomdb_embedder_profiles", err))?;

    // If target DB had a pre-existing empty vector_default with a different dimension, drop it first.
    if let Some(existing_dim) = vector_partition_dimension(tx)
        .map_err(|err| LogicalExportError::storage("inspect target vector_default dim", err))?
    {
        if existing_dim != dim {
            tx.execute_batch(&format!("DROP TABLE {DEFAULT_VECTOR_PARTITION};")).map_err(
                |err| LogicalExportError::storage("drop empty mismatched vector_default", err),
            )?;
        }
    }
    ensure_vector_partition(tx, dim)
        .map_err(|err| LogicalExportError::storage("create vector_default", err))?;
    reconcile_vector_attr_columns(tx, dim)
        .map_err(|err| LogicalExportError::storage("reconcile vector_default attr columns", err))?;

    // Cache canonical node bodies + properties for attr_<hex> extraction.
    let mut node_bodies: BTreeMap<u64, (String, Option<PropertyMap>)> = BTreeMap::new();
    {
        let mut stmt = tx
            .prepare("SELECT write_cursor, body, properties FROM canonical_nodes")
            .map_err(|err| LogicalExportError::storage("prepare canonical node body cache", err))?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, u64>(0)?,
                    row.get::<_, String>(1)?,
                    decode_properties_row_col(row, 2)?,
                ))
            })
            .map_err(|err| LogicalExportError::storage("query canonical node body cache", err))?;
        for r in rows {
            let (cursor, body, props) = r.map_err(|err| {
                LogicalExportError::storage("read canonical node body cache", err)
            })?;
            node_bodies.insert(cursor, (body, props));
        }
    }

    let mut restored: u64 = 0;
    for row in &vectors.rows {
        if !canonical_cursors.contains(&row.write_cursor) {
            return Err(LogicalExportError::new(
                LogicalExportErrorCode::MalformedArchive,
                format!(
                    "vector row {} references missing canonical write_cursor {}",
                    row.rowid, row.write_cursor
                ),
            ));
        }
        let source_type = resolve_source_type_on(tx, &row.kind).map_err(|_| {
            LogicalExportError::new(
                LogicalExportErrorCode::MalformedArchive,
                format!(
                    "vector kind '{}' is not committable or enrolled in _fathomdb_vector_kinds",
                    row.kind
                ),
            )
        })?;
        let (lifecycle, valid_from, valid_until, scope) =
            vector_eligibility_and_scope_for_cursor(tx, row.write_cursor as i64)
                .map_err(|err| LogicalExportError::storage("compute vector eligibility", err))?;
        let (owner_body, owner_props) = match node_bodies.get(&row.write_cursor) {
            Some((body, props)) => (body.as_str(), props.as_ref()),
            None => ("", None),
        };
        let (cols_sql, ph_sql, attr_vals) =
            vector_attr_insert_fragments(tx, owner_body, owner_props, 11)
                .map_err(|err| LogicalExportError::storage("extract vector attr fragments", err))?;

        let raw_blob = encode_vector_blob(&row.embedding);
        let bin_input_blob = match &derived_mean {
            Some(mean) => encode_vector_blob(&subtract_mean(&row.embedding, mean)),
            None => raw_blob.clone(),
        };

        tx.execute(
            "INSERT INTO _fathomdb_vector_rows(rowid, kind, write_cursor) VALUES(?1, ?2, ?3)",
            params![row.rowid, row.kind, row.write_cursor],
        )
        .map_err(|err| LogicalExportError::storage("insert _fathomdb_vector_rows", err))?;

        tx.execute(
            "INSERT INTO _fathomdb_vector_spans(vector_rowid, write_cursor, window_number, byte_start, byte_end, body_length) \
             VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                row.rowid,
                row.write_cursor,
                row.window_number,
                row.byte_start,
                row.byte_end,
                row.body_length,
            ],
        )
        .map_err(|err| LogicalExportError::storage("insert _fathomdb_vector_spans", err))?;

        let sql = format!(
            "INSERT INTO {DEFAULT_VECTOR_PARTITION}(\
                rowid, embedding, embedding_bin, source_type, kind, created_at, status, lifecycle, valid_from, valid_until, scope{cols_sql}\
             ) VALUES(?1, ?2, vec_quantize_binary(?3), ?4, ?5, ?6, '', ?7, ?8, ?9, ?10{ph_sql})"
        );
        let mut binds: Vec<rusqlite::types::Value> = vec![
            rusqlite::types::Value::Integer(row.rowid),
            rusqlite::types::Value::Blob(raw_blob),
            rusqlite::types::Value::Blob(bin_input_blob),
            rusqlite::types::Value::Text(source_type.to_string()),
            rusqlite::types::Value::Text(row.kind.clone()),
            rusqlite::types::Value::Integer(row.created_at),
            rusqlite::types::Value::Integer(lifecycle),
            rusqlite::types::Value::Integer(valid_from),
            rusqlite::types::Value::Integer(valid_until),
            rusqlite::types::Value::Text(scope),
        ];
        binds.extend(attr_vals);
        tx.execute(&sql, rusqlite::params_from_iter(binds.iter()))
            .map_err(|err| LogicalExportError::storage("insert vector_default", err))?;

        record_projection_terminal(
            tx,
            row.write_cursor,
            "up_to_date",
            ProjectionTerminalOwnerKind::Canonical,
        )
        .map_err(|err| LogicalExportError::storage("record vector projection terminal", err))?;

        tx.execute(
            "INSERT INTO _fathomdb_projection_state(kind, last_enqueued_cursor, updated_at) \
             VALUES(?1, ?2, 0) \
             ON CONFLICT(kind) DO UPDATE SET last_enqueued_cursor = MAX(last_enqueued_cursor, excluded.last_enqueued_cursor)",
            params![row.kind, row.write_cursor],
        )
        .map_err(|err| LogicalExportError::storage("update _fathomdb_projection_state", err))?;

        restored = restored.saturating_add(1);
    }

    Ok((restored, should_pin_mean))
}

fn synchronize_open_state(
    tx: &Connection,
    archive: &LogicalExportV1,
    max_closure_sequence: i64,
) -> Result<(), LogicalExportError> {
    let upsert = |key: &str, value: &str| -> Result<(), LogicalExportError> {
        tx.execute(
            "INSERT INTO _fathomdb_open_state(key, value) VALUES(?1, ?2) \
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )
        .map_err(|err| LogicalExportError::storage("upsert _fathomdb_open_state", err))?;
        Ok(())
    };

    upsert("_fathomdb_physical_erasure_policy", &archive.physical_erasure_policy)?;
    upsert("_fathomdb_dependency_generation", &archive.dependency_generation.to_string())?;
    upsert("_fathomdb_closure_sequence", &max_closure_sequence.max(0).to_string())?;
    upsert("search_index_tokenizer_reproject_complete", "1")?;
    upsert("tc33_edge_vector_prune_complete", "1")?;

    // Ensure required target-database identity keys exist (preserving existing fresh keys).
    for (key, random_bytes) in [
        ("_fathomdb_database_id", 16_i64),
        ("_fathomdb_read_context_key", 32_i64),
        ("_fathomdb_erasure_store", 16_i64),
    ] {
        tx.execute(
            "INSERT OR IGNORE INTO _fathomdb_open_state(key, value) VALUES(?1, lower(hex(randomblob(?2))))",
            params![key, random_bytes],
        )
        .map_err(|err| LogicalExportError::storage("ensure identity key in _fathomdb_open_state", err))?;
    }
    Ok(())
}

fn synchronize_visibility_state(tx: &Connection) -> Result<(), LogicalExportError> {
    tx.execute(
        "UPDATE _fathomdb_read_visibility_state \
         SET generation = MAX(generation, 1), \
             state_nonce = lower(hex(randomblob(32))), \
             coalescing = 0, \
             global_struct_gen = MAX(global_struct_gen, 1), \
             global_struct_nonce = lower(hex(randomblob(32))) \
         WHERE singleton = 1",
        [],
    )
    .map_err(|err| {
        LogicalExportError::storage("synchronize _fathomdb_read_visibility_state", err)
    })?;
    tx.execute(
        "UPDATE _fathomdb_scope_visibility_watermarks SET state_nonce = lower(hex(randomblob(32)))",
        [],
    )
    .map_err(|err| {
        LogicalExportError::storage("synchronize _fathomdb_scope_visibility_watermarks", err)
    })?;
    Ok(())
}

// ── Export table readers (deterministic ordering) ───────────────────────────

fn query_rows<T, F>(
    tx: &Connection,
    sql: &str,
    label: &str,
    mapper: F,
) -> Result<Vec<T>, LogicalExportError>
where
    F: FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
{
    let mut stmt = tx
        .prepare(sql)
        .map_err(|err| LogicalExportError::storage(&format!("prepare export {label}"), err))?;
    let rows = stmt
        .query_map([], mapper)
        .map_err(|err| LogicalExportError::storage(&format!("query {label}"), err))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|err| LogicalExportError::storage(&format!("collect {label}"), err))?;
    Ok(rows)
}

fn export_canonical_nodes(
    tx: &Connection,
) -> Result<Vec<CanonicalNodeExportRowV1>, LogicalExportError> {
    query_rows(
        tx,
        "SELECT write_cursor, kind, body, source_id, logical_id, superseded_at, row_kind, \
                importance, state, reason, valid_from, valid_until, properties, scope \
         FROM canonical_nodes ORDER BY write_cursor ASC",
        "canonical_nodes",
        |row| {
            Ok(CanonicalNodeExportRowV1 {
                write_cursor: row.get(0)?,
                kind: row.get(1)?,
                body: row.get(2)?,
                source_id: row.get(3)?,
                logical_id: row.get(4)?,
                superseded_at: row.get(5)?,
                row_kind: row.get(6)?,
                importance: row.get(7)?,
                state: row.get(8)?,
                reason: row.get(9)?,
                valid_from: row.get(10)?,
                valid_until: row.get(11)?,
                properties: row.get(12)?,
                scope: row.get(13)?,
            })
        },
    )
}

fn export_canonical_edges(
    tx: &Connection,
) -> Result<Vec<CanonicalEdgeExportRowV1>, LogicalExportError> {
    query_rows(
        tx,
        "SELECT write_cursor, kind, from_id, to_id, source_id, logical_id, superseded_at, \
                body, t_valid, t_invalid, confidence, extractor_model_id, temporal_fallback, \
                properties, scope \
         FROM canonical_edges ORDER BY write_cursor ASC",
        "canonical_edges",
        |row| {
            Ok(CanonicalEdgeExportRowV1 {
                write_cursor: row.get(0)?,
                kind: row.get(1)?,
                from_id: row.get(2)?,
                to_id: row.get(3)?,
                source_id: row.get(4)?,
                logical_id: row.get(5)?,
                superseded_at: row.get(6)?,
                body: row.get(7)?,
                t_valid: row.get(8)?,
                t_invalid: row.get(9)?,
                confidence: row.get(10)?,
                extractor_model_id: row.get(11)?,
                temporal_fallback: row.get(12)?,
                properties: row.get(13)?,
                scope: row.get(14)?,
            })
        },
    )
}

fn export_operational_collections(
    tx: &Connection,
) -> Result<Vec<OperationalCollectionExportRowV1>, LogicalExportError> {
    query_rows(
        tx,
        "SELECT name, kind, schema_json, retention_json, format_version, created_at \
         FROM operational_collections ORDER BY name ASC",
        "operational_collections",
        |row| {
            Ok(OperationalCollectionExportRowV1 {
                name: row.get(0)?,
                kind: row.get(1)?,
                schema_json: row.get(2)?,
                retention_json: row.get(3)?,
                format_version: row.get(4)?,
                created_at: row.get(5)?,
            })
        },
    )
}

fn export_operational_mutations(
    tx: &Connection,
) -> Result<Vec<OperationalMutationExportRowV1>, LogicalExportError> {
    query_rows(
        tx,
        "SELECT id, collection_name, record_key, op_kind, payload_json, schema_id, \
                write_cursor, scope \
         FROM operational_mutations ORDER BY id ASC",
        "operational_mutations",
        |row| {
            Ok(OperationalMutationExportRowV1 {
                id: row.get(0)?,
                collection_name: row.get(1)?,
                record_key: row.get(2)?,
                op_kind: row.get(3)?,
                payload_json: row.get(4)?,
                schema_id: row.get(5)?,
                write_cursor: row.get(6)?,
                scope: row.get(7)?,
            })
        },
    )
}

fn export_operational_state(
    tx: &Connection,
) -> Result<Vec<OperationalStateExportRowV1>, LogicalExportError> {
    query_rows(
        tx,
        "SELECT collection_name, record_key, payload_json, schema_id, write_cursor, scope \
         FROM operational_state ORDER BY scope ASC, collection_name ASC, record_key ASC",
        "operational_state",
        |row| {
            Ok(OperationalStateExportRowV1 {
                collection_name: row.get(0)?,
                record_key: row.get(1)?,
                payload_json: row.get(2)?,
                schema_id: row.get(3)?,
                write_cursor: row.get(4)?,
                scope: row.get(5)?,
            })
        },
    )
}

fn export_artifact_revisions(
    tx: &Connection,
) -> Result<Vec<ArtifactRevisionExportRowV1>, LogicalExportError> {
    query_rows(
        tx,
        "SELECT schema_version, revision_id, artifact_class, write_cursor, artifact_role, \
                completeness, scope \
         FROM _fathomdb_artifact_revisions ORDER BY scope ASC, revision_id ASC",
        "_fathomdb_artifact_revisions",
        |row| {
            Ok(ArtifactRevisionExportRowV1 {
                schema_version: row.get(0)?,
                revision_id: row.get(1)?,
                artifact_class: row.get(2)?,
                write_cursor: row.get(3)?,
                artifact_role: row.get(4)?,
                completeness: row.get(5)?,
                scope: row.get(6)?,
            })
        },
    )
}

fn export_source_versions(
    tx: &Connection,
) -> Result<Vec<SourceVersionExportRowV1>, LogicalExportError> {
    query_rows(
        tx,
        "SELECT schema_version, source_id, source_version_id, source_revision_id, scope \
         FROM _fathomdb_source_versions ORDER BY scope ASC, source_id ASC, source_version_id ASC",
        "_fathomdb_source_versions",
        |row| {
            Ok(SourceVersionExportRowV1 {
                schema_version: row.get(0)?,
                source_id: row.get(1)?,
                source_version_id: row.get(2)?,
                source_revision_id: row.get(3)?,
                scope: row.get(4)?,
            })
        },
    )
}

fn export_source_links(tx: &Connection) -> Result<Vec<SourceLinkExportRowV1>, LogicalExportError> {
    query_rows(
        tx,
        "SELECT schema_version, artifact_revision_id, source_id, source_version_id, \
                source_revision_id, locator_kind, start_byte, end_byte, hash_algorithm, \
                hash_digest, scope \
         FROM _fathomdb_source_links ORDER BY scope ASC, artifact_revision_id ASC",
        "_fathomdb_source_links",
        |row| {
            Ok(SourceLinkExportRowV1 {
                schema_version: row.get(0)?,
                artifact_revision_id: row.get(1)?,
                source_id: row.get(2)?,
                source_version_id: row.get(3)?,
                source_revision_id: row.get(4)?,
                locator_kind: row.get(5)?,
                start_byte: row.get(6)?,
                end_byte: row.get(7)?,
                hash_algorithm: row.get(8)?,
                hash_digest: row.get(9)?,
                scope: row.get(10)?,
            })
        },
    )
}

fn export_source_dependencies(
    tx: &Connection,
) -> Result<Vec<SourceDependencyExportRowV1>, LogicalExportError> {
    query_rows(
        tx,
        "SELECT schema_version, dependency_id, derived_revision_id, \
                registered_dependency_generation, scope \
         FROM _fathomdb_source_dependencies ORDER BY scope ASC, dependency_id ASC",
        "_fathomdb_source_dependencies",
        |row| {
            Ok(SourceDependencyExportRowV1 {
                schema_version: row.get(0)?,
                dependency_id: row.get(1)?,
                derived_revision_id: row.get(2)?,
                registered_dependency_generation: row.get(3)?,
                scope: row.get(4)?,
            })
        },
    )
}

fn export_projection_registry(
    tx: &Connection,
) -> Result<Vec<ProjectionRegistryExportRowV1>, LogicalExportError> {
    query_rows(
        tx,
        "SELECT name, roles, fts_tokenizer, vector_embedder, vector_declared, source, \
                entity_class, vector_kinds \
         FROM _fathomdb_projection_registry ORDER BY entity_class ASC, name ASC",
        "_fathomdb_projection_registry",
        |row| {
            Ok(ProjectionRegistryExportRowV1 {
                name: row.get(0)?,
                roles: row.get(1)?,
                fts_tokenizer: row.get(2)?,
                vector_embedder: row.get(3)?,
                vector_declared: row.get(4)?,
                source: row.get(5)?,
                entity_class: row.get(6)?,
                vector_kinds: row.get(7)?,
            })
        },
    )
}

fn export_vector_kinds(tx: &Connection) -> Result<Vec<VectorKindExportRowV1>, LogicalExportError> {
    query_rows(
        tx,
        "SELECT kind, profile, created_at FROM _fathomdb_vector_kinds ORDER BY kind ASC",
        "_fathomdb_vector_kinds",
        |row| {
            Ok(VectorKindExportRowV1 {
                kind: row.get(0)?,
                profile: row.get(1)?,
                created_at: row.get(2)?,
            })
        },
    )
}

fn export_commit_timestamps(
    tx: &Connection,
) -> Result<Vec<CommitTimestampExportRowV1>, LogicalExportError> {
    query_rows(
        tx,
        "SELECT write_cursor, committed_at_ms FROM _fathomdb_commit_timestamps ORDER BY write_cursor ASC",
        "_fathomdb_commit_timestamps",
        |row| {
            Ok(CommitTimestampExportRowV1 {
                write_cursor: row.get(0)?,
                committed_at_ms: row.get(1)?,
            })
        },
    )
}

fn export_non_canonical_terminal_cursors(tx: &Connection) -> Result<Vec<i64>, LogicalExportError> {
    query_rows(
        tx,
        "SELECT write_cursor FROM _fathomdb_projection_terminal \
         WHERE owner_kind = 'non_canonical' ORDER BY write_cursor ASC",
        "_fathomdb_projection_terminal(non_canonical)",
        |row| row.get(0),
    )
}

fn export_actuation_receipts(
    tx: &Connection,
) -> Result<Vec<ActuationReceiptExportRowV1>, LogicalExportError> {
    query_rows(
        tx,
        "SELECT operation_id, schema_version, request_sha256, operations_count, outcome, \
                refused_operation_index, refused_field_path, reason_codes_json, \
                affected_revision_ids_json, resulting_write_boundary, \
                resulting_dependency_generation, pending_projection_write_cursors_json, \
                closure_operation_ids_json, projection_generation_id, scope \
         FROM _fathomdb_actuation_receipts ORDER BY scope ASC, operation_id ASC",
        "_fathomdb_actuation_receipts",
        |row| {
            Ok(ActuationReceiptExportRowV1 {
                operation_id: row.get(0)?,
                schema_version: row.get(1)?,
                request_sha256: row.get(2)?,
                operations_count: row.get(3)?,
                outcome: row.get(4)?,
                refused_operation_index: row.get(5)?,
                refused_field_path: row.get(6)?,
                reason_codes_json: row.get(7)?,
                affected_revision_ids_json: row.get(8)?,
                resulting_write_boundary: row.get(9)?,
                resulting_dependency_generation: row.get(10)?,
                pending_projection_write_cursors_json: row.get(11)?,
                closure_operation_ids_json: row.get(12)?,
                projection_generation_id: row.get(13)?,
                scope: row.get(14)?,
            })
        },
    )
}

fn export_actuation_receipt_source_refs(
    tx: &Connection,
) -> Result<Vec<ActuationReceiptSourceRefExportRowV1>, LogicalExportError> {
    query_rows(
        tx,
        "SELECT operation_id, schema_version, ref_kind, ref_value, scope \
         FROM _fathomdb_actuation_receipt_source_refs \
         ORDER BY scope ASC, operation_id ASC, ref_kind ASC, ref_value ASC",
        "_fathomdb_actuation_receipt_source_refs",
        |row| {
            Ok(ActuationReceiptSourceRefExportRowV1 {
                operation_id: row.get(0)?,
                schema_version: row.get(1)?,
                ref_kind: row.get(2)?,
                ref_value: row.get(3)?,
                scope: row.get(4)?,
            })
        },
    )
}

fn export_dependency_closures(
    tx: &Connection,
) -> Result<Vec<DependencyClosureExportRowV1>, LogicalExportError> {
    query_rows(
        tx,
        "SELECT schema_version, closure_operation_id, root_kind, root_value, cause, \
                effective_at_epoch_s, admitted_write_boundary, admitted_dependency_generation, \
                closure_sequence, retry_fingerprint, phase, affected_count, blocker_code, \
                structural_proof_write_boundary, proof_json, scope \
         FROM _fathomdb_dependency_closures ORDER BY closure_sequence ASC",
        "_fathomdb_dependency_closures",
        |row| {
            Ok(DependencyClosureExportRowV1 {
                schema_version: row.get(0)?,
                closure_operation_id: row.get(1)?,
                root_kind: row.get(2)?,
                root_value: row.get(3)?,
                cause: row.get(4)?,
                effective_at_epoch_s: row.get(5)?,
                admitted_write_boundary: row.get(6)?,
                admitted_dependency_generation: row.get(7)?,
                closure_sequence: row.get(8)?,
                retry_fingerprint: row.get(9)?,
                phase: row.get(10)?,
                affected_count: row.get(11)?,
                blocker_code: row.get(12)?,
                structural_proof_write_boundary: row.get(13)?,
                proof_json: row.get(14)?,
                scope: row.get(15)?,
            })
        },
    )
}

fn export_vectors(tx: &Connection) -> Result<LogicalExportVectorsV1, LogicalExportError> {
    // Strictly select (profile, name, revision, dimension) — NEVER mean_vec!
    let embedder = tx
        .query_row(
            "SELECT profile, name, revision, dimension FROM _fathomdb_embedder_profiles WHERE profile = ?1",
            [DEFAULT_VECTOR_PROFILE],
            |row| {
                Ok(LogicalExportEmbedderIdentityV1 {
                    profile: row.get(0)?,
                    name: row.get(1)?,
                    revision: row.get(2)?,
                    dimension: row.get(3)?,
                })
            },
        )
        .optional()
        .map_err(|err| LogicalExportError::storage("query _fathomdb_embedder_profiles", err))?
        .unwrap_or_else(|| {
            let def = default_embedder_identity();
            LogicalExportEmbedderIdentityV1 {
                profile: DEFAULT_VECTOR_PROFILE.to_string(),
                name: def.name,
                revision: def.revision,
                dimension: def.dimension,
            }
        });

    let vec_table_exists: bool = tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
            [DEFAULT_VECTOR_PARTITION],
            |row| row.get(0),
        )
        .map_err(|err| LogicalExportError::storage("check vector_default existence", err))?;
    if !vec_table_exists {
        let sidecar_count: i64 = tx
            .query_row("SELECT COUNT(*) FROM _fathomdb_vector_rows", [], |r| r.get(0))
            .map_err(|err| LogicalExportError::storage("count _fathomdb_vector_rows", err))?;
        if sidecar_count != 0 {
            return Err(LogicalExportError::new(
                LogicalExportErrorCode::Storage,
                "vector_default is absent while _fathomdb_vector_rows is non-empty",
            ));
        }
        return Ok(LogicalExportVectorsV1 { embedder, rows: Vec::new() });
    }

    let vr_count: i64 = tx
        .query_row("SELECT COUNT(*) FROM _fathomdb_vector_rows", [], |r| r.get(0))
        .map_err(|err| LogicalExportError::storage("count _fathomdb_vector_rows", err))?;
    let vs_count: i64 = tx
        .query_row("SELECT COUNT(*) FROM _fathomdb_vector_spans", [], |r| r.get(0))
        .map_err(|err| LogicalExportError::storage("count _fathomdb_vector_spans", err))?;
    let vd_count: i64 = tx
        .query_row(&format!("SELECT COUNT(*) FROM {DEFAULT_VECTOR_PARTITION}"), [], |r| r.get(0))
        .map_err(|err| LogicalExportError::storage("count vector_default", err))?;
    if vr_count != vs_count || vr_count != vd_count {
        return Err(LogicalExportError::new(
            LogicalExportErrorCode::Storage,
            format!(
                "inconsistent vector tuple counts (rows={vr_count}, spans={vs_count}, vec0={vd_count})"
            ),
        ));
    }

    let expected_blob_len = (embedder.dimension as usize).saturating_mul(4);
    // Strictly select uncentered f32 `vd.embedding` — NEVER `embedding_bin`!
    let mut stmt = tx
        .prepare(&format!(
            "SELECT vr.rowid, vr.write_cursor, vr.kind, vd.created_at, \
                    vs.window_number, vs.byte_start, vs.byte_end, vs.body_length, vd.embedding \
             FROM _fathomdb_vector_rows vr \
             JOIN _fathomdb_vector_spans vs ON vs.vector_rowid = vr.rowid AND vs.write_cursor = vr.write_cursor \
             JOIN {DEFAULT_VECTOR_PARTITION} vd ON vd.rowid = vr.rowid \
             ORDER BY vr.rowid ASC"
        ))
        .map_err(|err| LogicalExportError::storage("prepare export vectors join", err))?;
    let mut rows = Vec::with_capacity(vr_count.max(0) as usize);
    let mapped = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, u64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, Option<i64>>(6)?,
                row.get::<_, i64>(7)?,
                row.get::<_, Vec<u8>>(8)?,
            ))
        })
        .map_err(|err| LogicalExportError::storage("query export vectors", err))?;
    for item in mapped {
        let (
            rowid,
            write_cursor,
            kind,
            created_at,
            window_number,
            byte_start,
            byte_end,
            body_length,
            blob,
        ) = item.map_err(|err| LogicalExportError::storage("read exported vector row", err))?;
        if blob.len() != expected_blob_len {
            return Err(LogicalExportError::new(
                LogicalExportErrorCode::Storage,
                format!(
                    "vector rowid {rowid} embedding blob length {} != expected {expected_blob_len}",
                    blob.len()
                ),
            ));
        }
        let embedding = decode_vector_blob(&blob);
        rows.push(LogicalExportVectorRowV1 {
            rowid,
            write_cursor,
            kind,
            created_at,
            window_number,
            byte_start,
            byte_end,
            body_length,
            embedding,
        });
    }
    if rows.len() as i64 != vr_count {
        return Err(LogicalExportError::new(
            LogicalExportErrorCode::Storage,
            format!("vector join returned {} rows, expected {vr_count}", rows.len()),
        ));
    }

    Ok(LogicalExportVectorsV1 { embedder, rows })
}

// ── Restore table writers ───────────────────────────────────────────────────

fn insert_canonical_nodes(
    tx: &Connection,
    rows: &[CanonicalNodeExportRowV1],
) -> Result<u64, LogicalExportError> {
    let mut stmt = tx
        .prepare(
            "INSERT INTO canonical_nodes(\
                write_cursor, kind, body, source_id, logical_id, superseded_at, row_kind, \
                importance, state, reason, valid_from, valid_until, properties, scope\
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        )
        .map_err(|err| LogicalExportError::storage("prepare insert canonical_nodes", err))?;
    for r in rows {
        stmt.execute(params![
            r.write_cursor,
            r.kind,
            r.body,
            r.source_id,
            r.logical_id,
            r.superseded_at,
            r.row_kind,
            r.importance,
            r.state,
            r.reason,
            r.valid_from,
            r.valid_until,
            r.properties,
            r.scope,
        ])
        .map_err(|err| LogicalExportError::storage("insert canonical_nodes", err))?;
    }
    Ok(rows.len() as u64)
}

fn insert_canonical_edges(
    tx: &Connection,
    rows: &[CanonicalEdgeExportRowV1],
) -> Result<u64, LogicalExportError> {
    let mut stmt = tx
        .prepare(
            "INSERT INTO canonical_edges(\
                write_cursor, kind, from_id, to_id, source_id, logical_id, superseded_at, \
                body, t_valid, t_invalid, confidence, extractor_model_id, temporal_fallback, \
                properties, scope\
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
        )
        .map_err(|err| LogicalExportError::storage("prepare insert canonical_edges", err))?;
    for r in rows {
        stmt.execute(params![
            r.write_cursor,
            r.kind,
            r.from_id,
            r.to_id,
            r.source_id,
            r.logical_id,
            r.superseded_at,
            r.body,
            r.t_valid,
            r.t_invalid,
            r.confidence,
            r.extractor_model_id,
            r.temporal_fallback,
            r.properties,
            r.scope,
        ])
        .map_err(|err| LogicalExportError::storage("insert canonical_edges", err))?;
    }
    Ok(rows.len() as u64)
}

fn insert_operational_collections(
    tx: &Connection,
    rows: &[OperationalCollectionExportRowV1],
) -> Result<u64, LogicalExportError> {
    let mut stmt = tx
        .prepare(
            "INSERT OR REPLACE INTO operational_collections(\
                name, kind, schema_json, retention_json, format_version, created_at\
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .map_err(|err| {
            LogicalExportError::storage("prepare insert operational_collections", err)
        })?;
    for r in rows {
        stmt.execute(params![
            r.name,
            r.kind,
            r.schema_json,
            r.retention_json,
            r.format_version,
            r.created_at,
        ])
        .map_err(|err| LogicalExportError::storage("insert operational_collections", err))?;
    }
    Ok(rows.len() as u64)
}

fn insert_operational_mutations(
    tx: &Connection,
    rows: &[OperationalMutationExportRowV1],
) -> Result<u64, LogicalExportError> {
    let mut stmt = tx
        .prepare(
            "INSERT INTO operational_mutations(\
                id, collection_name, record_key, op_kind, payload_json, schema_id, \
                write_cursor, scope\
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .map_err(|err| LogicalExportError::storage("prepare insert operational_mutations", err))?;
    for r in rows {
        stmt.execute(params![
            r.id,
            r.collection_name,
            r.record_key,
            r.op_kind,
            r.payload_json,
            r.schema_id,
            r.write_cursor,
            r.scope,
        ])
        .map_err(|err| LogicalExportError::storage("insert operational_mutations", err))?;
    }
    Ok(rows.len() as u64)
}

fn insert_operational_state(
    tx: &Connection,
    rows: &[OperationalStateExportRowV1],
) -> Result<u64, LogicalExportError> {
    let mut stmt = tx
        .prepare(
            "INSERT INTO operational_state(\
                collection_name, record_key, payload_json, schema_id, write_cursor, scope\
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .map_err(|err| LogicalExportError::storage("prepare insert operational_state", err))?;
    for r in rows {
        stmt.execute(params![
            r.collection_name,
            r.record_key,
            r.payload_json,
            r.schema_id,
            r.write_cursor,
            r.scope,
        ])
        .map_err(|err| LogicalExportError::storage("insert operational_state", err))?;
    }
    Ok(rows.len() as u64)
}

fn insert_artifact_revisions(
    tx: &Connection,
    rows: &[ArtifactRevisionExportRowV1],
) -> Result<u64, LogicalExportError> {
    let mut stmt = tx
        .prepare(
            "INSERT INTO _fathomdb_artifact_revisions(\
                schema_version, revision_id, artifact_class, write_cursor, artifact_role, \
                completeness, scope\
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )
        .map_err(|err| {
            LogicalExportError::storage("prepare insert _fathomdb_artifact_revisions", err)
        })?;
    for r in rows {
        stmt.execute(params![
            r.schema_version,
            r.revision_id,
            r.artifact_class,
            r.write_cursor,
            r.artifact_role,
            r.completeness,
            r.scope,
        ])
        .map_err(|err| LogicalExportError::storage("insert _fathomdb_artifact_revisions", err))?;
    }
    Ok(rows.len() as u64)
}

fn insert_source_versions(
    tx: &Connection,
    rows: &[SourceVersionExportRowV1],
) -> Result<u64, LogicalExportError> {
    let mut stmt = tx
        .prepare(
            "INSERT INTO _fathomdb_source_versions(\
                schema_version, source_id, source_version_id, source_revision_id, scope\
             ) VALUES(?1, ?2, ?3, ?4, ?5)",
        )
        .map_err(|err| {
            LogicalExportError::storage("prepare insert _fathomdb_source_versions", err)
        })?;
    for r in rows {
        stmt.execute(params![
            r.schema_version,
            r.source_id,
            r.source_version_id,
            r.source_revision_id,
            r.scope,
        ])
        .map_err(|err| LogicalExportError::storage("insert _fathomdb_source_versions", err))?;
    }
    Ok(rows.len() as u64)
}

fn insert_source_links(
    tx: &Connection,
    rows: &[SourceLinkExportRowV1],
) -> Result<u64, LogicalExportError> {
    let mut stmt = tx
        .prepare(
            "INSERT INTO _fathomdb_source_links(\
                schema_version, artifact_revision_id, source_id, source_version_id, \
                source_revision_id, locator_kind, start_byte, end_byte, hash_algorithm, \
                hash_digest, scope\
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        )
        .map_err(|err| LogicalExportError::storage("prepare insert _fathomdb_source_links", err))?;
    for r in rows {
        stmt.execute(params![
            r.schema_version,
            r.artifact_revision_id,
            r.source_id,
            r.source_version_id,
            r.source_revision_id,
            r.locator_kind,
            r.start_byte,
            r.end_byte,
            r.hash_algorithm,
            r.hash_digest,
            r.scope,
        ])
        .map_err(|err| LogicalExportError::storage("insert _fathomdb_source_links", err))?;
    }
    Ok(rows.len() as u64)
}

fn insert_source_dependencies(
    tx: &Connection,
    rows: &[SourceDependencyExportRowV1],
) -> Result<u64, LogicalExportError> {
    let mut stmt = tx
        .prepare(
            "INSERT INTO _fathomdb_source_dependencies(\
                schema_version, dependency_id, derived_revision_id, \
                registered_dependency_generation, scope\
             ) VALUES(?1, ?2, ?3, ?4, ?5)",
        )
        .map_err(|err| {
            LogicalExportError::storage("prepare insert _fathomdb_source_dependencies", err)
        })?;
    for r in rows {
        stmt.execute(params![
            r.schema_version,
            r.dependency_id,
            r.derived_revision_id,
            r.registered_dependency_generation,
            r.scope,
        ])
        .map_err(|err| LogicalExportError::storage("insert _fathomdb_source_dependencies", err))?;
    }
    Ok(rows.len() as u64)
}

fn insert_projection_registry(
    tx: &Connection,
    rows: &[ProjectionRegistryExportRowV1],
) -> Result<u64, LogicalExportError> {
    let mut stmt = tx
        .prepare(
            "INSERT INTO _fathomdb_projection_registry(\
                name, roles, fts_tokenizer, vector_embedder, vector_declared, source, \
                entity_class, vector_kinds\
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .map_err(|err| {
            LogicalExportError::storage("prepare insert _fathomdb_projection_registry", err)
        })?;
    for r in rows {
        stmt.execute(params![
            r.name,
            r.roles,
            r.fts_tokenizer,
            r.vector_embedder,
            r.vector_declared,
            r.source,
            r.entity_class,
            r.vector_kinds,
        ])
        .map_err(|err| LogicalExportError::storage("insert _fathomdb_projection_registry", err))?;
    }
    Ok(rows.len() as u64)
}

fn insert_vector_kinds(
    tx: &Connection,
    rows: &[VectorKindExportRowV1],
) -> Result<u64, LogicalExportError> {
    let mut stmt = tx
        .prepare("INSERT INTO _fathomdb_vector_kinds(kind, profile, created_at) VALUES(?1, ?2, ?3)")
        .map_err(|err| LogicalExportError::storage("prepare insert _fathomdb_vector_kinds", err))?;
    for r in rows {
        stmt.execute(params![r.kind, r.profile, r.created_at])
            .map_err(|err| LogicalExportError::storage("insert _fathomdb_vector_kinds", err))?;
    }
    Ok(rows.len() as u64)
}

fn insert_commit_timestamps(
    tx: &Connection,
    rows: &[CommitTimestampExportRowV1],
) -> Result<u64, LogicalExportError> {
    let mut stmt = tx
        .prepare(
            "INSERT INTO _fathomdb_commit_timestamps(write_cursor, committed_at_ms) VALUES(?1, ?2)",
        )
        .map_err(|err| {
            LogicalExportError::storage("prepare insert _fathomdb_commit_timestamps", err)
        })?;
    for r in rows {
        stmt.execute(params![r.write_cursor, r.committed_at_ms]).map_err(|err| {
            LogicalExportError::storage("insert _fathomdb_commit_timestamps", err)
        })?;
    }
    Ok(rows.len() as u64)
}

fn insert_actuation_receipts(
    tx: &Connection,
    rows: &[ActuationReceiptExportRowV1],
    restored_serving_generation_id: &str,
) -> Result<u64, LogicalExportError> {
    let mut stmt = tx
        .prepare(
            "INSERT INTO _fathomdb_actuation_receipts(\
                operation_id, schema_version, request_sha256, operations_count, outcome, \
                refused_operation_index, refused_field_path, reason_codes_json, \
                affected_revision_ids_json, resulting_write_boundary, \
                resulting_dependency_generation, pending_projection_write_cursors_json, \
                closure_operation_ids_json, projection_generation_id, scope\
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
        )
        .map_err(|err| {
            LogicalExportError::storage("prepare insert _fathomdb_actuation_receipts", err)
        })?;
    for r in rows {
        let bound_generation =
            r.projection_generation_id.as_ref().map(|_| restored_serving_generation_id);
        stmt.execute(params![
            r.operation_id,
            r.schema_version,
            r.request_sha256,
            r.operations_count,
            r.outcome,
            r.refused_operation_index,
            r.refused_field_path,
            r.reason_codes_json,
            r.affected_revision_ids_json,
            r.resulting_write_boundary,
            r.resulting_dependency_generation,
            r.pending_projection_write_cursors_json,
            r.closure_operation_ids_json,
            bound_generation,
            r.scope,
        ])
        .map_err(|err| LogicalExportError::storage("insert _fathomdb_actuation_receipts", err))?;
    }
    Ok(rows.len() as u64)
}

fn insert_actuation_receipt_source_refs(
    tx: &Connection,
    rows: &[ActuationReceiptSourceRefExportRowV1],
) -> Result<u64, LogicalExportError> {
    let mut stmt = tx
        .prepare(
            "INSERT INTO _fathomdb_actuation_receipt_source_refs(\
                operation_id, schema_version, ref_kind, ref_value, scope\
             ) VALUES(?1, ?2, ?3, ?4, ?5)",
        )
        .map_err(|err| {
            LogicalExportError::storage(
                "prepare insert _fathomdb_actuation_receipt_source_refs",
                err,
            )
        })?;
    for r in rows {
        stmt.execute(params![r.operation_id, r.schema_version, r.ref_kind, r.ref_value, r.scope])
            .map_err(|err| {
            LogicalExportError::storage("insert _fathomdb_actuation_receipt_source_refs", err)
        })?;
    }
    Ok(rows.len() as u64)
}

fn insert_dependency_closures(
    tx: &Connection,
    rows: &[DependencyClosureExportRowV1],
) -> Result<u64, LogicalExportError> {
    let mut stmt = tx
        .prepare(
            "INSERT INTO _fathomdb_dependency_closures(\
                schema_version, closure_operation_id, root_kind, root_value, cause, \
                effective_at_epoch_s, admitted_write_boundary, admitted_dependency_generation, \
                closure_sequence, retry_fingerprint, phase, affected_count, blocker_code, \
                structural_proof_write_boundary, proof_json, scope\
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
        )
        .map_err(|err| {
            LogicalExportError::storage("prepare insert _fathomdb_dependency_closures", err)
        })?;
    for r in rows {
        stmt.execute(params![
            r.schema_version,
            r.closure_operation_id,
            r.root_kind,
            r.root_value,
            r.cause,
            r.effective_at_epoch_s,
            r.admitted_write_boundary,
            r.admitted_dependency_generation,
            r.closure_sequence,
            r.retry_fingerprint,
            r.phase,
            r.affected_count,
            r.blocker_code,
            r.structural_proof_write_boundary,
            r.proof_json,
            r.scope,
        ])
        .map_err(|err| LogicalExportError::storage("insert _fathomdb_dependency_closures", err))?;
    }
    Ok(rows.len() as u64)
}
