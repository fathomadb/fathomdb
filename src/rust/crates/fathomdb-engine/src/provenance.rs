use super::*;

/// Completeness recorded for an artifact revision owner.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProvenanceCompleteness {
    /// Exact source version, revision, locator and hash are present.
    Complete,
    /// The artifact is usable but exact source provenance is unavailable.
    MigratedIncomplete,
}

impl ProvenanceCompleteness {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::MigratedIncomplete => "migrated_incomplete",
        }
    }
}

/// Closed v1 locator into exact UTF-8 source bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceLocator {
    /// The entire canonical source body.
    WholeBody,
    /// A half-open range whose offsets count UTF-8 bytes.
    Utf8Bytes {
        /// Inclusive byte offset.
        start_inclusive: u64,
        /// Exclusive byte offset.
        end_exclusive: u64,
    },
}

impl SourceLocator {
    /// Construct the whole-body locator.
    #[must_use]
    pub fn whole_body() -> Self {
        Self::WholeBody
    }

    /// Construct a UTF-8 byte range. Bounds and code-point alignment are
    /// validated against the referenced canonical source during the write.
    #[must_use]
    pub fn utf8_bytes(start_inclusive: u64, end_exclusive: u64) -> Self {
        Self::Utf8Bytes { start_inclusive, end_exclusive }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProvenanceRole {
    Canonical,
    Derived,
}

/// Closed schema-version-1 provenance attached to a versioned write.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WriteProvenanceV1 {
    pub(crate) schema_version: u32,
    pub(crate) role: ProvenanceRole,
    pub(crate) artifact_revision_id: ArtifactRevisionId,
    pub(crate) source_version_id: SourceVersionId,
    pub(crate) source_revision_id: Option<SourceRevisionId>,
    pub(crate) locator: Option<SourceLocator>,
    pub(crate) canonical_source_hash: Option<CanonicalHash>,
}

impl WriteProvenanceV1 {
    /// Describe a canonical source node. The Engine stores a whole-body
    /// self-link and computes the source hash from the exact UTF-8 body.
    #[must_use]
    pub fn canonical(
        artifact_revision_id: ArtifactRevisionId,
        source_version_id: SourceVersionId,
    ) -> Self {
        Self {
            schema_version: 1,
            role: ProvenanceRole::Canonical,
            artifact_revision_id,
            source_version_id,
            source_revision_id: None,
            locator: None,
            canonical_source_hash: None,
        }
    }

    /// Describe an artifact derived from an already-stored canonical source.
    #[must_use]
    pub fn derived(
        artifact_revision_id: ArtifactRevisionId,
        source_version_id: SourceVersionId,
        source_revision_id: SourceRevisionId,
        locator: SourceLocator,
        canonical_source_hash: CanonicalHash,
    ) -> Self {
        Self {
            schema_version: 1,
            role: ProvenanceRole::Derived,
            artifact_revision_id,
            source_version_id,
            source_revision_id: Some(source_revision_id),
            locator: Some(locator),
            canonical_source_hash: Some(canonical_source_hash),
        }
    }
}

/// Versioned node input preserving the legacy node fields and adding exact
/// provenance without changing `PreparedWrite::Node`.
#[derive(Clone, Debug, PartialEq)]
pub struct ProvenancedNodeV1 {
    /// Caller-defined node kind.
    pub kind: String,
    /// Exact UTF-8 artifact body.
    pub body: String,
    /// Erasure and source-family identity.
    pub source_id: SourceId,
    /// Optional stable logical identity used for supersession.
    pub logical_id: Option<String>,
    /// Initial lifecycle state.
    pub state: InitialState,
    /// Optional advisory lifecycle reason.
    pub reason: Option<String>,
    /// Inclusive world-time validity bound in epoch seconds.
    pub valid_from: Option<i64>,
    /// Exclusive world-time validity bound in epoch seconds.
    pub valid_until: Option<i64>,
    /// Closed schema-version-1 provenance.
    pub provenance: WriteProvenanceV1,
}

/// Versioned edge input preserving the legacy edge fields and adding exact
/// provenance without changing `PreparedWrite::Edge`.
#[derive(Clone, Debug, PartialEq)]
pub struct ProvenancedEdgeV1 {
    /// Caller-defined edge kind.
    pub kind: String,
    /// Logical identity of the source endpoint.
    pub from: String,
    /// Logical identity of the destination endpoint.
    pub to: String,
    /// Erasure and source-family identity.
    pub source_id: SourceId,
    /// Optional stable logical identity used for supersession.
    pub logical_id: Option<String>,
    /// Optional exact UTF-8 relationship body.
    pub body: Option<String>,
    /// Inclusive event-time validity bound in epoch seconds.
    pub t_valid: Option<i64>,
    /// Exclusive event-time validity bound in epoch seconds.
    pub t_invalid: Option<i64>,
    /// Optional extraction confidence in the closed interval `[0, 1]`.
    pub confidence: Option<f64>,
    /// Optional opaque extractor model identity.
    pub extractor_model_id: Option<String>,
    /// Whether event time fell back to ingestion time.
    pub temporal_fallback: Option<bool>,
    /// Closed schema-version-1 derived provenance.
    pub provenance: WriteProvenanceV1,
}

/// Closed machine-readable reason for a provenance refusal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProvenanceErrorReason {
    RevisionIdInvalid,
    RevisionIdConflict,
    SourceVersionInvalid,
    SourceVersionConflict,
    SourceRevisionMissing,
    SourceMismatch,
    LocatorInvalid,
    HashInvalid,
    HashMismatch,
    UnsupportedSchemaVersion,
    UnknownField,
    RoleInvalid,
    ProvenanceInUse,
    SourceRevisionIneligible,
    SourceClosureActive,
}

impl ProvenanceErrorReason {
    /// Stable lower-snake-case wire spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RevisionIdInvalid => "revision_id_invalid",
            Self::RevisionIdConflict => "revision_id_conflict",
            Self::SourceVersionInvalid => "source_version_invalid",
            Self::SourceVersionConflict => "source_version_conflict",
            Self::SourceRevisionMissing => "source_revision_missing",
            Self::SourceMismatch => "source_mismatch",
            Self::LocatorInvalid => "locator_invalid",
            Self::HashInvalid => "hash_invalid",
            Self::HashMismatch => "hash_mismatch",
            Self::UnsupportedSchemaVersion => "unsupported_schema_version",
            Self::UnknownField => "unknown_field",
            Self::RoleInvalid => "role_invalid",
            Self::ProvenanceInUse => "provenance_in_use",
            Self::SourceRevisionIneligible => "source_revision_ineligible",
            Self::SourceClosureActive => "source_closure_active",
        }
    }
}

/// Typed provenance refusal with an RFC 6901 pointer over canonical camel-case
/// wire names.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProvenanceError {
    /// Closed machine-readable refusal reason.
    pub reason: ProvenanceErrorReason,
    /// RFC 6901 pointer over canonical camel-case wire names.
    pub field_path: String,
}

impl ProvenanceError {
    pub(crate) fn new(reason: ProvenanceErrorReason, field_path: impl Into<String>) -> Self {
        Self { reason, field_path: field_path.into() }
    }
}

impl Display for ProvenanceError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} at {}", self.reason.as_str(), self.field_path)
    }
}

impl Error for ProvenanceError {}

pub(crate) const DEFAULT_PROVENANCE_ROW_CAP: u64 = 1_000_000;

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

impl Engine {
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
}
