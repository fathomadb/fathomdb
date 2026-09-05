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
    pending: Vec<u64>,
    failed: Vec<u64>,
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
    let mut pending = Vec::new();
    let mut failed = Vec::new();

    let node_arm_declared =
        super::vector_projection_declared(connection).map_err(|_| EngineError::Storage)?;
    let has_explicit_node_enrolment: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM _fathomdb_vector_kinds WHERE kind!='edge_fact')",
            [],
            |row| row.get(0),
        )
        .map_err(|_| EngineError::Storage)?;
    if node_arm_declared || has_explicit_node_enrolment {
        let sql = "SELECT n.write_cursor,n.kind,n.row_kind,n.state,\
                          EXISTS(SELECT 1 FROM _fathomdb_artifact_revisions r \
                            JOIN _fathomdb_source_dependencies d \
                              ON d.derived_revision_id=r.revision_id \
                            WHERE r.artifact_class='node' AND r.write_cursor=n.write_cursor),\
                          pt.state,vr.rowid,vr.kind,vd.source_type,vd.kind,(vk.kind IS NOT NULL) \
                   FROM canonical_nodes n \
                   LEFT JOIN _fathomdb_projection_terminal pt \
                     ON pt.write_cursor=n.write_cursor \
                   LEFT JOIN _fathomdb_vector_rows vr ON vr.write_cursor=n.write_cursor \
                   LEFT JOIN vector_default vd ON vd.rowid=n.write_cursor \
                   LEFT JOIN _fathomdb_vector_kinds vk ON vk.kind=n.kind \
                   ORDER BY n.write_cursor";
        let mut statement = connection.prepare(sql).map_err(|_| EngineError::Storage)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, u64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, bool>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<u64>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, Option<String>>(8)?,
                    row.get::<_, Option<String>>(9)?,
                    row.get::<_, bool>(10)?,
                ))
            })
            .map_err(|_| EngineError::Storage)?;
        for row in rows {
            let (
                cursor,
                kind,
                row_kind,
                state,
                registered,
                terminal,
                sidecar_rowid,
                sidecar,
                physical_source_type,
                physical_kind,
                enrolled,
            ) = row.map_err(|_| EngineError::Storage)?;
            if (!node_arm_declared && !enrolled)
                || !matches!(row_kind.as_str(), "leaf" | "coverage")
                || !super::kind_is_vector_committable(&kind)
                || (registered && state != "active")
                || (registered
                    && !super::dependency_closure::projection_owner_is_eligible_at(
                        connection,
                        cursor,
                        effective_at,
                    )?)
            {
                continue;
            }
            let physical = physical_source_type.as_deref().zip(physical_kind.as_deref());
            match classify_completion(
                terminal.as_deref(),
                sidecar.as_deref().map(|kind| (kind, sidecar_rowid == Some(cursor))),
                physical,
                &kind,
                false,
                enrolled,
                runtime_state,
            )? {
                Completion::Complete => {}
                Completion::Pending => pending.push(cursor),
                Completion::Failed => failed.push(cursor),
            }
        }
    }

    let mut statement = connection
        .prepare(
            "SELECT e.write_cursor,\
                    EXISTS(SELECT 1 FROM _fathomdb_artifact_revisions r \
                      JOIN _fathomdb_source_dependencies d \
                        ON d.derived_revision_id=r.revision_id \
                      WHERE r.artifact_class='edge' AND r.write_cursor=e.write_cursor),\
                    pt.state,vr.rowid,vr.kind,vd.source_type,vd.kind,(vk.kind IS NOT NULL) \
             FROM canonical_edges e \
             LEFT JOIN _fathomdb_projection_terminal pt ON pt.write_cursor=e.write_cursor \
             LEFT JOIN _fathomdb_vector_rows vr ON vr.write_cursor=e.write_cursor \
             LEFT JOIN vector_default vd ON vd.rowid=e.write_cursor \
             LEFT JOIN _fathomdb_vector_kinds vk ON vk.kind='edge_fact' \
             WHERE e.body IS NOT NULL AND e.superseded_at IS NULL \
               AND (e.t_invalid IS NULL OR e.t_invalid>?1) ORDER BY e.write_cursor",
        )
        .map_err(|_| EngineError::Storage)?;
    let rows = statement
        .query_map([effective_at], |row| {
            Ok((
                row.get::<_, u64>(0)?,
                row.get::<_, bool>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<u64>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, bool>(7)?,
            ))
        })
        .map_err(|_| EngineError::Storage)?;
    for row in rows {
        let (
            cursor,
            registered,
            terminal,
            sidecar_rowid,
            sidecar,
            physical_source_type,
            physical_kind,
            enrolled,
        ) = row.map_err(|_| EngineError::Storage)?;
        if registered
            && !super::dependency_closure::projection_owner_is_eligible_at(
                connection,
                cursor,
                effective_at,
            )?
        {
            continue;
        }
        let physical = physical_source_type.as_deref().zip(physical_kind.as_deref());
        match classify_completion(
            terminal.as_deref(),
            sidecar.as_deref().map(|kind| (kind, sidecar_rowid == Some(cursor))),
            physical,
            "edge_fact",
            true,
            enrolled,
            runtime_state,
        )? {
            Completion::Complete => {}
            Completion::Pending => pending.push(cursor),
            Completion::Failed => failed.push(cursor),
        }
    }
    pending.sort_unstable();
    failed.sort_unstable();
    let first_incomplete = pending.first().into_iter().chain(failed.first()).min().copied();
    let ready_through =
        first_incomplete.map_or(observed_boundary, |cursor| cursor.saturating_sub(1));
    Ok(CompletionSummary { observed_boundary, ready_through, pending, failed })
}

fn status_in_snapshot(
    connection: &Connection,
    runtime_state: ProjectionRuntimeStateV1,
    effective_at_epoch_s: i64,
) -> Result<ProjectionGenerationStatusV1, EngineError> {
    let generation = current_generation(connection)?;
    let completion = physical_completion(connection, effective_at_epoch_s, runtime_state)?;
    let readiness = if generation.origin == ProjectionGenerationOriginV1::LegacyUnverified
        || !completion.failed.is_empty()
    {
        ProjectionReadinessV1::Degraded
    } else if completion.pending.is_empty() {
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
        pending_count: u64::try_from(completion.pending.len()).map_err(|_| EngineError::Storage)?,
        failed_count: u64::try_from(completion.failed.len()).map_err(|_| EngineError::Storage)?,
    })
}

impl Engine {
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
        let status = status_in_snapshot(&tx, runtime_state, effective_at_epoch_s)?;
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
        let status = status_in_snapshot(&tx, runtime_state, effective_at_epoch_s)?;
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
