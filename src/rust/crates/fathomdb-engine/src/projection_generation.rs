use std::fmt::{Display, Formatter};

use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use sha2::{Digest, Sha256};

use super::{current_epoch_seconds, valid_caller_identity, Engine, EngineError};

const PREFIX: &str = "pgen1:";

/// Database-local identity of one in-place serving-projection epoch.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct ProjectionGenerationId(String);

impl ProjectionGenerationId {
    /// Parse a versioned projection-generation identity.
    pub fn new(value: impl Into<String>) -> Result<Self, ProjectionGenerationError> {
        Self::parse(value.into())
    }

    /// Return the stable versioned textual identity.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn parse(value: String) -> Result<Self, ProjectionGenerationError> {
        let valid = value.strip_prefix(PREFIX).is_some_and(|suffix| {
            suffix.len() == 32
                && suffix.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        });
        if valid {
            Ok(Self(value))
        } else {
            Err(ProjectionGenerationError::new(
                ProjectionGenerationErrorReason::InvalidGenerationId,
                "/expectedGenerationId",
            ))
        }
    }
}

impl Display for ProjectionGenerationId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Closed serving-generation origin.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectionGenerationOriginV1 {
    Fresh,
    LegacyUnverified,
    Configuration,
    Rebuild,
}

impl ProjectionGenerationOriginV1 {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "fresh" => Some(Self::Fresh),
            "legacy_unverified" => Some(Self::LegacyUnverified),
            "configuration" => Some(Self::Configuration),
            "rebuild" => Some(Self::Rebuild),
            _ => None,
        }
    }

    /// Stable lower-snake-case wire spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fresh => "fresh",
            Self::LegacyUnverified => "legacy_unverified",
            Self::Configuration => "configuration",
            Self::Rebuild => "rebuild",
        }
    }
}

/// Truthful completeness state for the current physical serving generation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectionReadinessV1 {
    Ready,
    Processing,
    Blocked,
    Deferred,
    Degraded,
}

impl ProjectionReadinessV1 {
    /// Stable lower-snake-case wire spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Processing => "processing",
            Self::Blocked => "blocked",
            Self::Deferred => "deferred",
            Self::Degraded => "degraded",
        }
    }
}

/// Runtime availability considered separately from physical completeness.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectionRuntimeStateV1 {
    Absent,
    Usable,
    Refused,
}

impl ProjectionRuntimeStateV1 {
    /// Stable lower-snake-case wire spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Absent => "absent",
            Self::Usable => "usable",
            Self::Refused => "refused",
        }
    }
}

/// Current serving-generation status from one SQLite snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectionGenerationStatusV1 {
    pub schema_version: u32,
    pub generation_id: ProjectionGenerationId,
    pub declaration_sha256: String,
    pub origin: ProjectionGenerationOriginV1,
    pub transition_boundary: u64,
    pub effective_at_epoch_s: i64,
    pub observed_boundary: u64,
    pub ready_through: u64,
    pub readiness: ProjectionReadinessV1,
    pub runtime_state: ProjectionRuntimeStateV1,
    pub pending_count: u64,
    pub failed_count: u64,
}

/// Point request for one Slice-25 pending projection cursor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MutationProjectionStatusRequestV1 {
    pub schema_version: u32,
    pub operation_id: String,
    pub write_cursor: u64,
    pub expected_generation_id: ProjectionGenerationId,
}

/// Point status for one receipt-owned pending projection cursor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MutationProjectionStatusV1 {
    pub schema_version: u32,
    pub operation_id: String,
    pub write_cursor: u64,
    pub generation_id: ProjectionGenerationId,
    pub effective_at_epoch_s: i64,
    pub observed_boundary: u64,
    pub ready_through: u64,
    pub readiness: ProjectionReadinessV1,
    pub runtime_state: ProjectionRuntimeStateV1,
    pub pending_count: u64,
    pub failed_count: u64,
}

#[derive(Clone)]
pub(crate) struct CachedProjectionGenerationStatus {
    visibility_generation: u64,
    data_version: u64,
    effective_at_epoch_s: i64,
    runtime_state: ProjectionRuntimeStateV1,
    status: ProjectionGenerationStatusV1,
}

#[derive(Clone)]
pub(crate) struct CachedMutationProjectionStatus {
    visibility_generation: u64,
    data_version: u64,
    effective_at_epoch_s: i64,
    runtime_state: ProjectionRuntimeStateV1,
    request: MutationProjectionStatusRequestV1,
    status: MutationProjectionStatusV1,
}

/// Closed projection-generation failure reason.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectionGenerationErrorReason {
    UnsupportedSchemaVersion,
    UnknownField,
    InvalidOperationId,
    InvalidWriteCursor,
    InvalidGenerationId,
    MutationNotTracked,
    WrongProjectionGeneration,
    ProjectionGenerationUnavailable,
    ProjectionGenerationCorrupt,
}

impl ProjectionGenerationErrorReason {
    /// Stable lower-snake-case wire spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UnsupportedSchemaVersion => "unsupported_schema_version",
            Self::UnknownField => "unknown_field",
            Self::InvalidOperationId => "invalid_operation_id",
            Self::InvalidWriteCursor => "invalid_write_cursor",
            Self::InvalidGenerationId => "invalid_generation_id",
            Self::MutationNotTracked => "mutation_not_tracked",
            Self::WrongProjectionGeneration => "wrong_projection_generation",
            Self::ProjectionGenerationUnavailable => "projection_generation_unavailable",
            Self::ProjectionGenerationCorrupt => "projection_generation_corrupt",
        }
    }
}

/// Typed projection-generation error with a privacy-safe request pointer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectionGenerationError {
    pub reason: ProjectionGenerationErrorReason,
    pub field_path: String,
}

impl ProjectionGenerationError {
    fn new(reason: ProjectionGenerationErrorReason, field_path: impl Into<String>) -> Self {
        Self { reason, field_path: field_path.into() }
    }
}

impl Display for ProjectionGenerationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{} at {}", self.reason.as_str(), self.field_path)
    }
}

impl std::error::Error for ProjectionGenerationError {}

#[derive(Clone)]
pub(crate) struct GenerationRow {
    pub(crate) id: ProjectionGenerationId,
    pub(crate) digest: String,
    pub(crate) boundary: u64,
    pub(crate) origin: ProjectionGenerationOriginV1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Completion {
    Complete,
    Pending,
    Failed,
}

#[derive(Clone, Debug)]
struct CompletionSummary {
    observed_boundary: u64,
    ready_through: u64,
    pending_count: u64,
    failed_count: u64,
}

#[derive(Default)]
struct CompletionAccumulator {
    pending_count: u64,
    failed_count: u64,
    first_incomplete: Option<u64>,
}

impl CompletionAccumulator {
    fn add(&mut self, cursor: u64, completion: Completion) -> Result<(), EngineError> {
        match completion {
            Completion::Complete => return Ok(()),
            Completion::Pending => {
                self.pending_count =
                    self.pending_count.checked_add(1).ok_or(EngineError::Storage)?;
            }
            Completion::Failed => {
                self.failed_count = self.failed_count.checked_add(1).ok_or(EngineError::Storage)?;
            }
        }
        self.first_incomplete =
            Some(self.first_incomplete.map_or(cursor, |first| first.min(cursor)));
        Ok(())
    }

    fn add_aggregate(
        &mut self,
        pending_count: u64,
        failed_count: u64,
        first_incomplete: Option<u64>,
    ) -> Result<(), EngineError> {
        self.pending_count =
            self.pending_count.checked_add(pending_count).ok_or(EngineError::Storage)?;
        self.failed_count =
            self.failed_count.checked_add(failed_count).ok_or(EngineError::Storage)?;
        if let Some(cursor) = first_incomplete {
            self.first_incomplete =
                Some(self.first_incomplete.map_or(cursor, |first| first.min(cursor)));
        }
        Ok(())
    }
}

fn corruption() -> EngineError {
    ProjectionGenerationError::new(
        ProjectionGenerationErrorReason::ProjectionGenerationCorrupt,
        "/projectionGeneration",
    )
    .into()
}

pub(crate) fn corruption_error() -> EngineError {
    corruption()
}

fn count(connection: &Connection, table: &str) -> Result<u64, EngineError> {
    let query = format!("SELECT COUNT(*) FROM {table}");
    connection.query_row(&query, [], |row| row.get::<_, u64>(0)).map_err(|_| EngineError::Storage)
}

fn authoritative_boundary(connection: &Connection) -> Result<u64, EngineError> {
    let mut boundary = 0_u64;
    for table in
        ["canonical_nodes", "canonical_edges", "operational_mutations", "operational_state"]
    {
        let query = format!("SELECT COALESCE(MAX(write_cursor),0) FROM {table}");
        let value = connection
            .query_row(&query, [], |row| row.get::<_, u64>(0))
            .map_err(|_| EngineError::Storage)?;
        boundary = boundary.max(value);
    }
    let reserved = connection
        .query_row(
            "SELECT value FROM _fathomdb_open_state WHERE key=?1",
            [fathomdb_schema::RESERVED_WRITE_CURSOR_KEY],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|_| EngineError::Storage)?
        .map(|raw| {
            raw.parse::<u64>().ok().filter(|value| value.to_string() == raw).ok_or_else(corruption)
        })
        .transpose()?
        .unwrap_or(0);
    let closure_boundary = connection
        .query_row(
            "SELECT COALESCE(MAX(admitted_write_boundary),0) \
             FROM _fathomdb_dependency_closures",
            [],
            |row| row.get::<_, u64>(0),
        )
        .map_err(|_| EngineError::Storage)?;
    Ok(boundary.max(reserved).max(closure_boundary))
}

fn bootstrap_is_fresh(connection: &Connection) -> Result<bool, EngineError> {
    for table in [
        "canonical_nodes",
        "canonical_edges",
        "operational_mutations",
        "operational_state",
        "_fathomdb_artifact_revisions",
        "_fathomdb_source_versions",
        "_fathomdb_source_links",
        "_fathomdb_source_dependencies",
        "_fathomdb_dependency_closures",
        "_fathomdb_actuation_receipts",
        "_fathomdb_actuation_receipt_source_refs",
        "_fathomdb_projection_registry",
        "canonical_attributes",
        "_fathomdb_projection_state",
        "_fathomdb_projection_terminal",
        "_fathomdb_vector_kinds",
        "_fathomdb_vector_rows",
        "_fathomdb_embed_probe",
        "search_index",
        "search_index_v2",
        "search_index_edges",
        "property_search_index",
        "vector_default",
    ] {
        if count(connection, table)? != 0 {
            return Ok(false);
        }
    }

    let seeded_collection_is_exact: bool = connection
        .query_row(
            "SELECT COUNT(*)=1 AND EXISTS(SELECT 1 FROM operational_collections \
             WHERE name='projection_failures' AND kind='append_only_log' \
               AND schema_json='{\"type\":\"object\"}' AND retention_json='{}' \
               AND format_version=1 AND created_at=0)",
            [],
            |row| row.get(0),
        )
        .map_err(|_| EngineError::Storage)?;
    if !seeded_collection_is_exact {
        return Ok(false);
    }

    let profiles: (u64, u64) = connection
        .query_row(
            "SELECT COUNT(*),COALESCE(SUM(mean_vec IS NOT NULL),0) \
             FROM _fathomdb_embedder_profiles",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|_| EngineError::Storage)?;
    if profiles != (1, 0) {
        return Ok(false);
    }

    let mut statement = connection
        .prepare("SELECT key,value FROM _fathomdb_open_state ORDER BY key")
        .map_err(|_| EngineError::Storage)?;
    let rows = statement
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
        .map_err(|_| EngineError::Storage)?;
    for row in rows {
        let (key, value) = row.map_err(|_| EngineError::Storage)?;
        let baseline = match key.as_str() {
            "_fathomdb_database_id" => {
                value.len() == 32 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
            }
            "_fathomdb_read_context_key" => {
                value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
            }
            "_fathomdb_dependency_generation"
            | "_fathomdb_closure_sequence"
            | "projection_cursor"
            | "tc33_reserved_write_cursor" => value == "0",
            "search_index_tokenizer_reproject_complete" | "tc33_edge_vector_prune_complete" => {
                value == "1"
            }
            _ => false,
        };
        if !baseline {
            return Ok(false);
        }
    }
    Ok(authoritative_boundary(connection)? == 0)
}

fn put_scalar(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend_from_slice(&(value.len() as u64).to_be_bytes());
    bytes.extend_from_slice(value.as_bytes());
}

fn declaration_digest(connection: &Connection) -> Result<String, EngineError> {
    let mut bytes = b"fathomdb.projection-serving-declaration.v1\0".to_vec();
    put_scalar(&mut bytes, "projection-serving-set/1");
    for arm in
        ["node_fts_v2", "edge_fts_v1", "attributes_v1", "property_fts_v1", "dense_default_v1"]
    {
        put_scalar(&mut bytes, arm);
    }
    let mut statement = connection
        .prepare(
            "SELECT name,roles,fts_tokenizer,vector_embedder,vector_declared,source \
             FROM _fathomdb_projection_registry ORDER BY CAST(name AS BLOB)",
        )
        .map_err(|_| EngineError::Storage)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, Option<String>>(5)?,
            ))
        })
        .map_err(|_| EngineError::Storage)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| EngineError::Storage)?;
    bytes.extend_from_slice(&(rows.len() as u64).to_be_bytes());
    for (name, roles_json, fts, embedder, declared, source) in rows {
        put_scalar(&mut bytes, &name);
        let mut roles: Vec<String> = roles_json
            .split(',')
            .map(str::trim)
            .filter(|role| !role.is_empty())
            .map(str::to_string)
            .collect();
        roles.sort();
        bytes.extend_from_slice(&(roles.len() as u64).to_be_bytes());
        for role in roles {
            put_scalar(&mut bytes, &role);
        }
        for value in [fts, embedder] {
            match value {
                Some(value) => {
                    bytes.push(1);
                    put_scalar(&mut bytes, &value);
                }
                None => bytes.push(0),
            }
        }
        bytes.push(u8::from(declared != 0));
        match source {
            Some(value) => {
                bytes.push(1);
                let segments: Vec<String> =
                    serde_json::from_str(&value).map_err(|_| corruption())?;
                bytes.extend_from_slice(&(segments.len() as u64).to_be_bytes());
                for segment in segments {
                    put_scalar(&mut bytes, &segment);
                }
            }
            None => bytes.push(0),
        }
    }
    let profile: (String, String, String, u32) = connection
        .query_row(
            "SELECT profile,name,revision,dimension FROM _fathomdb_embedder_profiles \
             WHERE profile='default'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .map_err(|_| EngineError::Storage)?;
    for value in [profile.0, profile.1, profile.2] {
        put_scalar(&mut bytes, &value);
    }
    bytes.extend_from_slice(&u64::from(profile.3).to_be_bytes());
    Ok(super::hex_encode(&Sha256::digest(bytes)))
}

fn mint_id_from_candidates(
    connection: &Connection,
    mut next_candidate: impl FnMut() -> Result<String, EngineError>,
) -> Result<ProjectionGenerationId, EngineError> {
    for _ in 0..4 {
        let suffix = next_candidate()?;
        let id = ProjectionGenerationId(format!("{PREFIX}{suffix}"));
        let exists: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM _fathomdb_projection_generations \
                 WHERE generation_id=?1)",
                [id.as_str()],
                |row| row.get(0),
            )
            .map_err(|_| EngineError::Storage)?;
        if !exists {
            return Ok(id);
        }
    }
    Err(EngineError::Storage)
}

fn mint_id(connection: &Connection) -> Result<ProjectionGenerationId, EngineError> {
    mint_id_from_candidates(connection, || {
        connection
            .query_row("SELECT lower(hex(randomblob(16)))", [], |row| row.get(0))
            .map_err(|_| EngineError::Storage)
    })
}

/// Retire the current in-place serving epoch and install its successor inside
/// the caller's already-open write transaction.
pub(crate) fn transition(
    connection: &Connection,
    origin: ProjectionGenerationOriginV1,
) -> Result<ProjectionGenerationId, EngineError> {
    let current: String = connection
        .query_row(
            "SELECT generation_id FROM _fathomdb_projection_generation_current WHERE singleton=1",
            [],
            |row| row.get(0),
        )
        .map_err(|_| corruption())?;
    ProjectionGenerationId::parse(current.clone()).map_err(|_| corruption())?;
    let boundary = authoritative_boundary(connection)?;
    let digest = declaration_digest(connection)?;
    let next = mint_id(connection)?;
    let changed = connection
        .execute(
            "UPDATE _fathomdb_projection_generations SET role='retired',retired_boundary=?2 \
             WHERE generation_id=?1 AND role='serving' AND retired_boundary IS NULL",
            params![current, boundary],
        )
        .map_err(|_| EngineError::Storage)?;
    if changed != 1 {
        return Err(corruption());
    }
    connection
        .execute(
            "INSERT INTO _fathomdb_projection_generations(\
               schema_version,generation_id,declaration_sha256,transition_boundary,role,origin\
             ) VALUES(1,?1,?2,?3,'serving',?4)",
            params![next.as_str(), digest, boundary, origin.as_str()],
        )
        .map_err(|_| EngineError::Storage)?;
    connection
        .execute(
            "UPDATE _fathomdb_projection_generation_current SET generation_id=?1 \
             WHERE singleton=1",
            [next.as_str()],
        )
        .map_err(|_| EngineError::Storage)?;
    Ok(next)
}

pub(crate) fn bootstrap(
    connection: &mut Connection,
    schema_version: u32,
) -> Result<(), EngineError> {
    if schema_version < 32 {
        return Ok(());
    }
    let generation_rows = count(connection, "_fathomdb_projection_generations")?;
    let current_rows = count(connection, "_fathomdb_projection_generation_current")?;
    if generation_rows == 0 && current_rows == 0 {
        let fresh = bootstrap_is_fresh(connection)?;
        let origin = if fresh { "fresh" } else { "legacy_unverified" };
        let boundary = authoritative_boundary(connection)?;
        let digest = declaration_digest(connection)?;
        let id = mint_id(connection)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| EngineError::Storage)?;
        tx.execute(
            "INSERT INTO _fathomdb_projection_generations(\
               schema_version,generation_id,declaration_sha256,transition_boundary,role,origin\
             ) VALUES(1,?1,?2,?3,'serving',?4)",
            params![id.as_str(), digest, boundary, origin],
        )
        .map_err(|_| EngineError::Storage)?;
        tx.execute(
            "INSERT INTO _fathomdb_projection_generation_current(singleton,generation_id) \
             VALUES(1,?1)",
            [id.as_str()],
        )
        .map_err(|_| EngineError::Storage)?;
        tx.commit().map_err(|_| EngineError::Storage)?;
        return Ok(());
    }
    if generation_rows == 0 || current_rows != 1 {
        return Err(corruption());
    }
    current_generation(connection).map(|_| ())
}

pub(crate) fn current_generation(connection: &Connection) -> Result<GenerationRow, EngineError> {
    let authority_counts: (u64, u64, u64) = connection
        .query_row(
            "SELECT COUNT(*),SUM(role='serving'),\
                    (SELECT COUNT(*) FROM _fathomdb_projection_generation_current) \
             FROM _fathomdb_projection_generations",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|_| corruption())?;
    if authority_counts.0 == 0 || authority_counts.1 != 1 || authority_counts.2 != 1 {
        return Err(corruption());
    }
    let mut history = connection
        .prepare(
            "SELECT generation_id,declaration_sha256,transition_boundary,role,origin,\
                    retired_boundary,schema_version \
             FROM _fathomdb_projection_generations",
        )
        .map_err(|_| corruption())?;
    let rows = history
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, Option<i64>>(5)?,
                row.get::<_, i64>(6)?,
            ))
        })
        .map_err(|_| corruption())?;
    for row in rows {
        let (id, digest, boundary, role, origin, retired, schema) =
            row.map_err(|_| corruption())?;
        let role_is_valid = match (role.as_str(), retired) {
            ("serving", None) => true,
            ("retired", Some(value)) => value >= 0 && value >= boundary,
            _ => false,
        };
        if ProjectionGenerationId::parse(id).is_err()
            || digest.len() != 64
            || digest.bytes().any(|byte| !(byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)))
            || boundary < 0
            || ProjectionGenerationOriginV1::parse(&origin).is_none()
            || schema != 1
            || !role_is_valid
        {
            return Err(corruption());
        }
    }
    let row: Option<(String, String, u64, String, String, Option<u64>)> = connection
        .query_row(
            "SELECT g.generation_id,g.declaration_sha256,g.transition_boundary,g.origin,\
                    g.role,g.retired_boundary \
             FROM _fathomdb_projection_generation_current c \
             JOIN _fathomdb_projection_generations g ON g.generation_id=c.generation_id \
             WHERE c.singleton=1",
            [],
            |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?))
            },
        )
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let Some((id, digest, boundary, origin, role, retired)) = row else {
        return Err(corruption());
    };
    if role != "serving"
        || retired.is_some()
        || digest.len() != 64
        || digest.bytes().any(|byte| !(byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)))
    {
        return Err(corruption());
    }
    let id = ProjectionGenerationId::parse(id).map_err(|_| corruption())?;
    let origin = ProjectionGenerationOriginV1::parse(&origin).ok_or_else(corruption)?;
    if declaration_digest(connection)? != digest {
        return Err(corruption());
    }
    Ok(GenerationRow { id, digest, boundary, origin })
}

fn member_completion(
    connection: &Connection,
    cursor: u64,
    expected_kind: &str,
    is_edge: bool,
    runtime_state: ProjectionRuntimeStateV1,
) -> Result<Completion, EngineError> {
    let terminal: Option<String> = connection
        .query_row(
            "SELECT state FROM _fathomdb_projection_terminal WHERE write_cursor=?1",
            [cursor],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let sidecar: Option<(u64, String)> = connection
        .query_row(
            "SELECT rowid,kind FROM _fathomdb_vector_rows WHERE write_cursor=?1",
            [cursor],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let physical: Option<(String, String)> = connection
        .query_row("SELECT source_type,kind FROM vector_default WHERE rowid=?1", [cursor], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let enrolled: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM _fathomdb_vector_kinds WHERE kind=?1)",
            [expected_kind],
            |row| row.get(0),
        )
        .map_err(|_| EngineError::Storage)?;

    classify_completion(
        terminal.as_deref(),
        sidecar.as_ref().map(|(rowid, kind)| (kind.as_str(), *rowid == cursor)),
        physical.as_ref().map(|(source_type, kind)| (source_type.as_str(), kind.as_str())),
        expected_kind,
        is_edge,
        enrolled,
        runtime_state,
    )
}

fn classify_completion(
    terminal: Option<&str>,
    sidecar: Option<(&str, bool)>,
    physical: Option<(&str, &str)>,
    expected_kind: &str,
    is_edge: bool,
    enrolled: bool,
    runtime_state: ProjectionRuntimeStateV1,
) -> Result<Completion, EngineError> {
    let expected_source_type = super::resolve_source_type(expected_kind)?;
    if terminal == Some("up_to_date")
        && sidecar.is_some_and(|(kind, identity_matches)| kind == expected_kind && identity_matches)
        && physical.is_some_and(|(source_type, kind)| {
            source_type == expected_source_type && kind == expected_kind
        })
        && (enrolled || !is_edge)
    {
        return Ok(Completion::Complete);
    }
    if terminal == Some("failed") && sidecar.is_none() && physical.is_none() {
        return Ok(Completion::Failed);
    }
    if terminal.is_none() && sidecar.is_none() && physical.is_none() && enrolled {
        return Ok(Completion::Pending);
    }
    if !is_edge
        && terminal == Some("up_to_date")
        && sidecar.is_none()
        && physical.is_none()
        && !enrolled
        && runtime_state != ProjectionRuntimeStateV1::Usable
    {
        return Ok(Completion::Pending);
    }
    Err(corruption())
}

fn physical_member_completion_at(
    connection: &Connection,
    cursor: u64,
    effective_at: i64,
    runtime_state: ProjectionRuntimeStateV1,
) -> Result<Option<Completion>, EngineError> {
    let Some(kind) = dense_member_kind_at(connection, cursor, effective_at)? else {
        return Ok(None);
    };
    let is_edge: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM canonical_edges WHERE write_cursor=?1)",
            [cursor],
            |row| row.get(0),
        )
        .map_err(|_| EngineError::Storage)?;
    member_completion(connection, cursor, &kind, is_edge, runtime_state).map(Some)
}

pub(crate) fn dense_member_kind_at(
    connection: &Connection,
    cursor: u64,
    effective_at: i64,
) -> Result<Option<String>, EngineError> {
    let node: Option<(String, String, String, bool, bool)> = connection
        .query_row(
            "SELECT n.kind,n.row_kind,n.state,\
                    EXISTS(SELECT 1 FROM _fathomdb_artifact_revisions r \
                      JOIN _fathomdb_source_dependencies d \
                        ON d.derived_revision_id=r.revision_id \
                      WHERE r.artifact_class='node' AND r.write_cursor=n.write_cursor),\
                    EXISTS(SELECT 1 FROM _fathomdb_vector_kinds vk WHERE vk.kind=n.kind) \
             FROM canonical_nodes n WHERE n.write_cursor=?1",
            [cursor],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        )
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let edge_exists: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM canonical_edges WHERE write_cursor=?1)",
            [cursor],
            |row| row.get(0),
        )
        .map_err(|_| EngineError::Storage)?;
    if node.is_some() && edge_exists {
        return Err(corruption());
    }
    if let Some((kind, row_kind, state, registered, enrolled)) = node {
        let declared =
            super::vector_projection_declared(connection).map_err(|_| EngineError::Storage)?;
        if (!declared && !enrolled)
            || !matches!(row_kind.as_str(), "leaf" | "coverage")
            || !super::kind_is_vector_committable(&kind)
            || (registered && state != "active")
            || !super::dependency_closure::projection_owner_is_eligible_at(
                connection,
                cursor,
                effective_at,
            )?
        {
            return Ok(None);
        }
        return Ok(Some(kind));
    }
    let edge: Option<(Option<String>, Option<i64>, Option<i64>)> = connection
        .query_row(
            "SELECT body,superseded_at,t_invalid FROM canonical_edges WHERE write_cursor=?1",
            [cursor],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let Some((body, superseded_at, t_invalid)) = edge else {
        return Ok(None);
    };
    if body.is_none()
        || superseded_at.is_some()
        || t_invalid.is_some_and(|end| end <= effective_at)
        || !super::dependency_closure::projection_owner_is_eligible_at(
            connection,
            cursor,
            effective_at,
        )?
    {
        return Ok(None);
    }
    Ok(Some("edge_fact".to_string()))
}

fn completion_aggregate_row(
    connection: &Connection,
    sql: &str,
    parameters: impl rusqlite::Params,
) -> Result<(u64, u64, Option<u64>), EngineError> {
    let (pending, failed, first_incomplete, corrupt): (u64, u64, Option<u64>, u64) = connection
        .query_row(sql, parameters, |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))
        .map_err(|_| EngineError::Storage)?;
    if corrupt != 0 {
        return Err(corruption());
    }
    Ok((pending, failed, first_incomplete))
}

fn aggregate_unregistered_node_completion(
    connection: &Connection,
    node_arm_declared: bool,
    runtime_state: ProjectionRuntimeStateV1,
) -> Result<(u64, u64, Option<u64>), EngineError> {
    let sql = "WITH members AS (
        SELECT n.write_cursor AS cursor,n.kind AS expected_kind,
               CASE n.kind
                 WHEN 'email' THEN 'email' WHEN 'article' THEN 'article'
                 WHEN 'paper' THEN 'paper' WHEN 'meeting' THEN 'meeting'
                 WHEN 'note' THEN 'note' WHEN 'todo' THEN 'todo'
                 WHEN 'doc' THEN 'article' END AS expected_source_type,
               pt.state AS terminal,vr.rowid AS sidecar_rowid,vr.kind AS sidecar_kind,
               vd.source_type AS physical_source_type,vd.kind AS physical_kind,
               (vk.kind IS NOT NULL) AS enrolled
        FROM canonical_nodes n
        LEFT JOIN _fathomdb_projection_terminal pt ON pt.write_cursor=n.write_cursor
        LEFT JOIN _fathomdb_vector_rows vr ON vr.write_cursor=n.write_cursor
        LEFT JOIN vector_default vd ON vd.rowid=n.write_cursor
        LEFT JOIN _fathomdb_vector_kinds vk ON vk.kind=n.kind
        WHERE n.row_kind IN ('leaf','coverage')
          AND n.kind IN ('email','article','paper','meeting','note','todo','doc')
          AND (?1!=0 OR vk.kind IS NOT NULL)
          AND NOT EXISTS(
            SELECT 1 FROM _fathomdb_artifact_revisions r
            JOIN _fathomdb_source_dependencies d ON d.derived_revision_id=r.revision_id
            WHERE r.artifact_class='node' AND r.write_cursor=n.write_cursor
          )
      ), classified AS (
        SELECT cursor,
          COALESCE(terminal='up_to_date' AND sidecar_rowid=cursor
            AND sidecar_kind=expected_kind
            AND physical_source_type=expected_source_type
            AND physical_kind=expected_kind,0) AS complete,
          COALESCE(terminal='failed' AND sidecar_rowid IS NULL
            AND physical_source_type IS NULL,0) AS failed,
          COALESCE(((terminal IS NULL AND sidecar_rowid IS NULL
              AND physical_source_type IS NULL AND enrolled)
            OR (terminal='up_to_date' AND sidecar_rowid IS NULL
              AND physical_source_type IS NULL AND NOT enrolled AND ?2=0)),0) AS pending
        FROM members
      )
      SELECT COALESCE(SUM(pending),0),COALESCE(SUM(failed),0),
             MIN(CASE WHEN pending OR failed THEN cursor END),
             COALESCE(SUM(NOT (complete OR failed OR pending)),0)
      FROM classified";
    completion_aggregate_row(
        connection,
        sql,
        params![
            i64::from(node_arm_declared),
            i64::from(runtime_state == ProjectionRuntimeStateV1::Usable)
        ],
    )
}

fn aggregate_unregistered_edge_completion(
    connection: &Connection,
    effective_at: i64,
) -> Result<(u64, u64, Option<u64>), EngineError> {
    let sql = "WITH members AS (
        SELECT e.write_cursor AS cursor,pt.state AS terminal,
               vr.rowid AS sidecar_rowid,vr.kind AS sidecar_kind,
               vd.source_type AS physical_source_type,vd.kind AS physical_kind,
               (vk.kind IS NOT NULL) AS enrolled
        FROM canonical_edges e
        LEFT JOIN _fathomdb_projection_terminal pt ON pt.write_cursor=e.write_cursor
        LEFT JOIN _fathomdb_vector_rows vr ON vr.write_cursor=e.write_cursor
        LEFT JOIN vector_default vd ON vd.rowid=e.write_cursor
        LEFT JOIN _fathomdb_vector_kinds vk ON vk.kind='edge_fact'
        WHERE e.body IS NOT NULL AND e.superseded_at IS NULL
          AND (e.t_invalid IS NULL OR e.t_invalid>?1)
          AND NOT EXISTS(
            SELECT 1 FROM _fathomdb_artifact_revisions r
            JOIN _fathomdb_source_dependencies d ON d.derived_revision_id=r.revision_id
            WHERE r.artifact_class='edge' AND r.write_cursor=e.write_cursor
          )
      ), classified AS (
        SELECT cursor,
          COALESCE(terminal='up_to_date' AND sidecar_rowid=cursor
            AND sidecar_kind='edge_fact' AND physical_source_type='edge_fact'
            AND physical_kind='edge_fact' AND enrolled,0) AS complete,
          COALESCE(terminal='failed' AND sidecar_rowid IS NULL
            AND physical_source_type IS NULL,0) AS failed,
          COALESCE(terminal IS NULL AND sidecar_rowid IS NULL
            AND physical_source_type IS NULL AND enrolled,0) AS pending
        FROM members
      )
      SELECT COALESCE(SUM(pending),0),COALESCE(SUM(failed),0),
             MIN(CASE WHEN pending OR failed THEN cursor END),
             COALESCE(SUM(NOT (complete OR failed OR pending)),0)
      FROM classified";
    completion_aggregate_row(connection, sql, [effective_at])
}

fn add_registered_completion(
    connection: &Connection,
    effective_at: i64,
    runtime_state: ProjectionRuntimeStateV1,
    artifact_class: &str,
    accumulator: &mut CompletionAccumulator,
) -> Result<(), EngineError> {
    let owner_table = if artifact_class == "node" { "canonical_nodes" } else { "canonical_edges" };
    let sql = format!(
        "SELECT DISTINCT owner.write_cursor FROM {owner_table} owner
         JOIN _fathomdb_artifact_revisions r ON r.write_cursor=owner.write_cursor
           AND r.artifact_class=?1
         JOIN _fathomdb_source_dependencies d ON d.derived_revision_id=r.revision_id
         ORDER BY owner.write_cursor"
    );
    let mut statement = connection.prepare(&sql).map_err(|_| EngineError::Storage)?;
    let rows = statement
        .query_map([artifact_class], |row| row.get::<_, u64>(0))
        .map_err(|_| EngineError::Storage)?;
    for row in rows {
        let cursor = row.map_err(|_| EngineError::Storage)?;
        if let Some(completion) =
            physical_member_completion_at(connection, cursor, effective_at, runtime_state)?
        {
            accumulator.add(cursor, completion)?;
        }
    }
    Ok(())
}

fn empty_physical_fast_path(
    connection: &Connection,
    effective_at: i64,
    node_arm_declared: bool,
) -> Result<Option<CompletionAccumulator>, EngineError> {
    let terminal_rows: u64 = connection
        .query_row("SELECT COUNT(*) FROM _fathomdb_projection_terminal", [], |row| row.get(0))
        .map_err(|_| EngineError::Storage)?;
    let sidecar_rows: u64 = connection
        .query_row("SELECT COUNT(*) FROM _fathomdb_vector_rows", [], |row| row.get(0))
        .map_err(|_| EngineError::Storage)?;
    let physical_rows: u64 = connection
        .query_row("SELECT COUNT(*) FROM vector_default", [], |row| row.get(0))
        .map_err(|_| EngineError::Storage)?;
    if terminal_rows != 0 || sidecar_rows != 0 || physical_rows != 0 {
        return Ok(None);
    }
    let dependency_rows: u64 = connection
        .query_row("SELECT COUNT(*) FROM _fathomdb_source_dependencies", [], |row| row.get(0))
        .map_err(|_| EngineError::Storage)?;
    if dependency_rows != 0 {
        return Ok(None);
    }

    let (node_count, first_node, unenrolled_nodes): (u64, Option<u64>, u64) = connection
        .query_row(
            "SELECT COUNT(*),MIN(n.write_cursor),COALESCE(SUM(vk.kind IS NULL),0)
             FROM canonical_nodes n
             LEFT JOIN _fathomdb_vector_kinds vk ON vk.kind=n.kind
             WHERE n.row_kind IN ('leaf','coverage')
               AND n.kind IN ('email','article','paper','meeting','note','todo','doc')
               AND (?1!=0 OR vk.kind IS NOT NULL)",
            [i64::from(node_arm_declared)],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|_| EngineError::Storage)?;
    let (edge_count, first_edge): (u64, Option<u64>) = connection
        .query_row(
            "SELECT COUNT(*),MIN(write_cursor) FROM canonical_edges
             WHERE body IS NOT NULL AND superseded_at IS NULL
               AND (t_invalid IS NULL OR t_invalid>?1)",
            [effective_at],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|_| EngineError::Storage)?;
    let edge_enrolled: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM _fathomdb_vector_kinds WHERE kind='edge_fact')",
            [],
            |row| row.get(0),
        )
        .map_err(|_| EngineError::Storage)?;
    if unenrolled_nodes != 0 || (edge_count != 0 && !edge_enrolled) {
        return Err(corruption());
    }
    let pending_count = node_count.checked_add(edge_count).ok_or(EngineError::Storage)?;
    let first_incomplete = first_node.into_iter().chain(first_edge).min();
    Ok(Some(CompletionAccumulator { pending_count, failed_count: 0, first_incomplete }))
}

fn physical_completion(
    connection: &Connection,
    effective_at: i64,
    runtime_state: ProjectionRuntimeStateV1,
) -> Result<CompletionSummary, EngineError> {
    let observed_boundary = authoritative_boundary(connection)?;
    let cross_owner_cursor: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM canonical_nodes n \
               JOIN canonical_edges e ON e.write_cursor=n.write_cursor LIMIT 1)",
            [],
            |row| row.get(0),
        )
        .map_err(|_| EngineError::Storage)?;
    if cross_owner_cursor {
        return Err(corruption());
    }

    let node_arm_declared =
        super::vector_projection_declared(connection).map_err(|_| EngineError::Storage)?;
    let has_explicit_node_enrolment: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM _fathomdb_vector_kinds WHERE kind!='edge_fact')",
            [],
            |row| row.get(0),
        )
        .map_err(|_| EngineError::Storage)?;
    let accumulator = if let Some(accumulator) =
        empty_physical_fast_path(connection, effective_at, node_arm_declared)?
    {
        accumulator
    } else {
        let mut accumulator = CompletionAccumulator::default();
        if node_arm_declared || has_explicit_node_enrolment {
            let (pending, failed, first) = aggregate_unregistered_node_completion(
                connection,
                node_arm_declared,
                runtime_state,
            )?;
            accumulator.add_aggregate(pending, failed, first)?;
            add_registered_completion(
                connection,
                effective_at,
                runtime_state,
                "node",
                &mut accumulator,
            )?;
        }
        let (pending, failed, first) =
            aggregate_unregistered_edge_completion(connection, effective_at)?;
        accumulator.add_aggregate(pending, failed, first)?;
        add_registered_completion(
            connection,
            effective_at,
            runtime_state,
            "edge",
            &mut accumulator,
        )?;
        accumulator
    };

    let ready_through =
        accumulator.first_incomplete.map_or(observed_boundary, |cursor| cursor.saturating_sub(1));
    Ok(CompletionSummary {
        observed_boundary,
        ready_through,
        pending_count: accumulator.pending_count,
        failed_count: accumulator.failed_count,
    })
}

fn status_in_snapshot(
    connection: &Connection,
    runtime_state: ProjectionRuntimeStateV1,
    effective_at_epoch_s: i64,
) -> Result<ProjectionGenerationStatusV1, EngineError> {
    let generation = current_generation(connection)?;
    let completion = physical_completion(connection, effective_at_epoch_s, runtime_state)?;
    let readiness = if generation.origin == ProjectionGenerationOriginV1::LegacyUnverified
        || completion.failed_count != 0
    {
        ProjectionReadinessV1::Degraded
    } else if completion.pending_count == 0 {
        ProjectionReadinessV1::Ready
    } else {
        match runtime_state {
            ProjectionRuntimeStateV1::Absent => ProjectionReadinessV1::Blocked,
            ProjectionRuntimeStateV1::Refused => ProjectionReadinessV1::Deferred,
            ProjectionRuntimeStateV1::Usable => ProjectionReadinessV1::Processing,
        }
    };
    Ok(ProjectionGenerationStatusV1 {
        schema_version: 1,
        generation_id: generation.id,
        declaration_sha256: generation.digest,
        origin: generation.origin,
        transition_boundary: generation.boundary,
        effective_at_epoch_s,
        observed_boundary: completion.observed_boundary,
        ready_through: completion.ready_through,
        readiness,
        runtime_state,
        pending_count: completion.pending_count,
        failed_count: completion.failed_count,
    })
}

impl Engine {
    fn cached_status_in_snapshot(
        &self,
        connection: &Connection,
        runtime_state: ProjectionRuntimeStateV1,
        effective_at_epoch_s: i64,
    ) -> Result<ProjectionGenerationStatusV1, EngineError> {
        let visibility_generation = super::frozen_read::load_visibility_generation(connection)
            .map_err(|_| EngineError::Storage)?;
        let data_version: u64 = connection
            .query_row("PRAGMA data_version", [], |row| row.get(0))
            .map_err(|_| EngineError::Storage)?;
        let mut cache =
            self.projection_generation_status_cache.lock().map_err(|_| EngineError::Storage)?;
        if let Some(cached) = cache.as_ref() {
            if cached.visibility_generation == visibility_generation
                && cached.data_version == data_version
                && cached.effective_at_epoch_s == effective_at_epoch_s
                && cached.runtime_state == runtime_state
            {
                return Ok(cached.status.clone());
            }
        }
        let status = status_in_snapshot(connection, runtime_state, effective_at_epoch_s)?;
        *cache = Some(CachedProjectionGenerationStatus {
            visibility_generation,
            data_version,
            effective_at_epoch_s,
            runtime_state,
            status: status.clone(),
        });
        Ok(status)
    }

    /// Read the current serving-generation identity and physical completeness.
    pub fn read_projection_generation_status(
        &self,
    ) -> Result<ProjectionGenerationStatusV1, EngineError> {
        self.ensure_open()?;
        let mut guard = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = guard.as_mut().ok_or(EngineError::Closing)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(|_| EngineError::Storage)?;
        let effective_at_epoch_s = current_epoch_seconds();
        let runtime_state = if self.runtime_embedder.is_none() {
            ProjectionRuntimeStateV1::Absent
        } else if self.dense_disabled.load(std::sync::atomic::Ordering::SeqCst) {
            ProjectionRuntimeStateV1::Refused
        } else {
            ProjectionRuntimeStateV1::Usable
        };
        let status = self.cached_status_in_snapshot(&tx, runtime_state, effective_at_epoch_s)?;
        tx.commit().map_err(|_| EngineError::Storage)?;
        Ok(status)
    }

    /// Read readiness for one cursor already recorded by a Slice-25 receipt.
    pub fn read_mutation_projection_status(
        &self,
        request: MutationProjectionStatusRequestV1,
    ) -> Result<MutationProjectionStatusV1, EngineError> {
        if request.schema_version != 1 {
            return Err(ProjectionGenerationError::new(
                ProjectionGenerationErrorReason::UnsupportedSchemaVersion,
                "/schemaVersion",
            )
            .into());
        }
        if !valid_caller_identity(&request.operation_id) {
            return Err(ProjectionGenerationError::new(
                ProjectionGenerationErrorReason::InvalidOperationId,
                "/operationId",
            )
            .into());
        }
        if request.write_cursor == 0 {
            return Err(ProjectionGenerationError::new(
                ProjectionGenerationErrorReason::InvalidWriteCursor,
                "/writeCursor",
            )
            .into());
        }
        self.ensure_open()?;
        let mut guard = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = guard.as_mut().ok_or(EngineError::Closing)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(|_| EngineError::Storage)?;
        let runtime_state = if self.runtime_embedder.is_none() {
            ProjectionRuntimeStateV1::Absent
        } else if self.dense_disabled.load(std::sync::atomic::Ordering::SeqCst) {
            ProjectionRuntimeStateV1::Refused
        } else {
            ProjectionRuntimeStateV1::Usable
        };
        let effective_at_epoch_s = current_epoch_seconds();
        let visibility_generation = super::frozen_read::load_visibility_generation(&tx)
            .map_err(|_| EngineError::Storage)?;
        let data_version: u64 = tx
            .query_row("PRAGMA data_version", [], |row| row.get(0))
            .map_err(|_| EngineError::Storage)?;
        {
            let cache =
                self.mutation_projection_status_cache.lock().map_err(|_| EngineError::Storage)?;
            if let Some(cached) = cache.as_ref() {
                if cached.visibility_generation == visibility_generation
                    && cached.data_version == data_version
                    && cached.effective_at_epoch_s == effective_at_epoch_s
                    && cached.runtime_state == runtime_state
                    && cached.request == request
                {
                    let status = cached.status.clone();
                    tx.commit().map_err(|_| EngineError::Storage)?;
                    return Ok(status);
                }
            }
        }
        let cache_request = request.clone();
        let raw_receipt: Option<(String, Option<String>)> = tx
            .query_row(
                "SELECT outcome,request_sha256 FROM _fathomdb_actuation_receipts \
                 WHERE operation_id=?1",
                [&request.operation_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|_| EngineError::Storage)?;
        let Some((outcome, digest)) = raw_receipt else {
            return Err(ProjectionGenerationError::new(
                ProjectionGenerationErrorReason::MutationNotTracked,
                "/operationId",
            )
            .into());
        };
        if outcome == "erased" {
            return Err(ProjectionGenerationError::new(
                ProjectionGenerationErrorReason::MutationNotTracked,
                "/operationId",
            )
            .into());
        }
        let Some(receipt) = super::actuation::load_receipt(
            &tx,
            &request.operation_id,
            digest.as_deref().unwrap_or_default(),
            None,
        )?
        else {
            return Err(ProjectionGenerationError::new(
                ProjectionGenerationErrorReason::MutationNotTracked,
                "/operationId",
            )
            .into());
        };
        if !receipt.pending_projection_write_cursors.contains(&request.write_cursor) {
            return Err(ProjectionGenerationError::new(
                ProjectionGenerationErrorReason::MutationNotTracked,
                "/writeCursor",
            )
            .into());
        }
        let Some(stored_generation) = receipt.projection_generation_id else {
            return Err(ProjectionGenerationError::new(
                ProjectionGenerationErrorReason::ProjectionGenerationUnavailable,
                "/projectionGeneration",
            )
            .into());
        };
        if request.expected_generation_id != stored_generation {
            return Err(ProjectionGenerationError::new(
                ProjectionGenerationErrorReason::WrongProjectionGeneration,
                "/expectedGenerationId",
            )
            .into());
        }
        let generation = current_generation(&tx)?;
        if generation.id != stored_generation {
            return Err(ProjectionGenerationError::new(
                ProjectionGenerationErrorReason::ProjectionGenerationUnavailable,
                "/projectionGeneration",
            )
            .into());
        }
        let status = self.cached_status_in_snapshot(&tx, runtime_state, effective_at_epoch_s)?;
        let owner_completion = physical_member_completion_at(
            &tx,
            request.write_cursor,
            effective_at_epoch_s,
            runtime_state,
        )?
        .ok_or_else(|| {
            EngineError::from(ProjectionGenerationError::new(
                ProjectionGenerationErrorReason::ProjectionGenerationUnavailable,
                "/projectionGeneration",
            ))
        })?;
        let (owner_readiness, owner_pending, owner_failed) = match owner_completion {
            Completion::Complete => (ProjectionReadinessV1::Ready, false, false),
            Completion::Failed => (ProjectionReadinessV1::Degraded, false, true),
            Completion::Pending => {
                let readiness = match runtime_state {
                    ProjectionRuntimeStateV1::Absent => ProjectionReadinessV1::Blocked,
                    ProjectionRuntimeStateV1::Refused => ProjectionReadinessV1::Deferred,
                    ProjectionRuntimeStateV1::Usable => ProjectionReadinessV1::Processing,
                };
                (readiness, true, false)
            }
        };
        let result = MutationProjectionStatusV1 {
            schema_version: 1,
            operation_id: request.operation_id,
            write_cursor: request.write_cursor,
            generation_id: status.generation_id,
            effective_at_epoch_s: status.effective_at_epoch_s,
            observed_boundary: status.observed_boundary,
            ready_through: status.ready_through,
            readiness: owner_readiness,
            runtime_state: status.runtime_state,
            pending_count: u64::from(owner_pending),
            failed_count: u64::from(owner_failed),
        };
        tx.commit().map_err(|_| EngineError::Storage)?;
        *self.mutation_projection_status_cache.lock().map_err(|_| EngineError::Storage)? =
            Some(CachedMutationProjectionStatus {
                visibility_generation,
                data_version,
                effective_at_epoch_s,
                runtime_state,
                request: cache_request,
                status: result.clone(),
            });
        Ok(result)
    }
}

pub(crate) fn current_generation_id(
    connection: &Connection,
) -> Result<ProjectionGenerationId, EngineError> {
    current_generation(connection).map(|row| row.id)
}

pub(crate) fn parse_persisted_generation_id(
    value: String,
) -> Result<ProjectionGenerationId, EngineError> {
    ProjectionGenerationId::parse(value).map_err(|_| corruption())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_mint_retries_collisions_without_reusing_history() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "CREATE TABLE _fathomdb_projection_generations(
                   generation_id TEXT PRIMARY KEY
                 );
                 INSERT INTO _fathomdb_projection_generations(generation_id)
                 VALUES('pgen1:00000000000000000000000000000000');",
            )
            .unwrap();
        let mut candidates = [
            "00000000000000000000000000000000",
            "00000000000000000000000000000000",
            "11111111111111111111111111111111",
        ]
        .into_iter();
        let minted =
            mint_id_from_candidates(&connection, || Ok(candidates.next().unwrap().to_string()))
                .unwrap();
        assert_eq!(minted.as_str(), "pgen1:11111111111111111111111111111111");

        let exhausted = mint_id_from_candidates(&connection, || {
            Ok("00000000000000000000000000000000".to_string())
        });
        assert!(matches!(exhausted, Err(EngineError::Storage)));
    }

    #[test]
    fn completion_classifier_is_closed_over_every_persisted_shape() {
        let terminals = [None, Some("up_to_date"), Some("failed"), Some("foreign")];
        let sidecars = [None, Some(("doc", true)), Some(("doc", false)), Some(("wrong", true))];
        let physicals =
            [None, Some(("article", "doc")), Some(("wrong", "doc")), Some(("article", "wrong"))];
        let runtimes = [
            ProjectionRuntimeStateV1::Absent,
            ProjectionRuntimeStateV1::Usable,
            ProjectionRuntimeStateV1::Refused,
        ];

        for terminal in terminals {
            for sidecar in sidecars {
                for physical in physicals {
                    for is_edge in [false, true] {
                        for enrolled in [false, true] {
                            for runtime in runtimes {
                                let expected = if terminal == Some("up_to_date")
                                    && sidecar == Some(("doc", true))
                                    && physical == Some(("article", "doc"))
                                    && (enrolled || !is_edge)
                                {
                                    Some(Completion::Complete)
                                } else if terminal == Some("failed")
                                    && sidecar.is_none()
                                    && physical.is_none()
                                {
                                    Some(Completion::Failed)
                                } else if terminal.is_none()
                                    && sidecar.is_none()
                                    && physical.is_none()
                                    && enrolled
                                {
                                    Some(Completion::Pending)
                                } else if !is_edge
                                    && terminal == Some("up_to_date")
                                    && sidecar.is_none()
                                    && physical.is_none()
                                    && !enrolled
                                    && runtime != ProjectionRuntimeStateV1::Usable
                                {
                                    Some(Completion::Pending)
                                } else {
                                    None
                                };
                                let actual = classify_completion(
                                    terminal, sidecar, physical, "doc", is_edge, enrolled, runtime,
                                );
                                match expected {
                                    Some(expected) => match actual {
                                        Ok(actual) => assert_eq!(actual, expected),
                                        Err(error) => panic!(
                                            "expected {expected:?}: terminal={terminal:?} sidecar={sidecar:?} physical={physical:?} edge={is_edge} enrolled={enrolled} runtime={runtime:?} error={error:?}"
                                        ),
                                    },
                                    None => assert!(
                                        matches!(
                                            actual,
                                            Err(EngineError::ProjectionGeneration(ref error))
                                                if error.reason
                                                    == ProjectionGenerationErrorReason::ProjectionGenerationCorrupt
                                        ),
                                        "unexpected classification: terminal={terminal:?} sidecar={sidecar:?} physical={physical:?} edge={is_edge} enrolled={enrolled} runtime={runtime:?} actual={actual:?}",
                                    ),
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
