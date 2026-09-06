use std::fmt::{Display, Formatter};

#[cfg(feature = "operator")]
use std::collections::{BTreeMap, BTreeSet};

#[cfg(feature = "operator")]
use rusqlite::types::ValueRef;
#[cfg(feature = "operator")]
use rusqlite::{Connection, OptionalExtension};

#[cfg(feature = "operator")]
use sha2::{Digest, Sha256};

#[cfg(feature = "operator")]
use crate::{current_epoch_seconds, load_next_cursor, EngineError};

const SCHEMA_VERSION: u32 = 1;
const MAX_WORK_UNITS: u32 = 10_000;
const MAX_FINDINGS: u32 = 100;
#[cfg(feature = "operator")]
const FIRST_SQLITE_ROWID: i64 = i64::MIN;

#[cfg(feature = "operator")]
const NODE_BODY_OWNER_QUERY: &str =
    "SELECT n.write_cursor,r.revision_id,n.body,n.kind FROM canonical_nodes n \
     INDEXED BY canonical_nodes_write_cursor_idx \
     LEFT JOIN _fathomdb_artifact_revisions r \
       ON r.artifact_class='node' AND r.write_cursor=n.write_cursor \
     WHERE n.write_cursor>?1 ORDER BY n.write_cursor LIMIT ?2";
#[cfg(feature = "operator")]
const EDGE_BODY_OWNER_QUERY: &str =
    "SELECT e.write_cursor,r.revision_id,e.body,e.kind,e.superseded_at,e.t_invalid \
     FROM canonical_edges e \
     INDEXED BY canonical_edges_write_cursor_idx \
     LEFT JOIN _fathomdb_artifact_revisions r \
       ON r.artifact_class='edge' AND r.write_cursor=e.write_cursor \
     WHERE e.write_cursor>?1 ORDER BY e.write_cursor LIMIT ?2";
#[cfg(feature = "operator")]
const SEARCH_V1_MEMBER_QUERY: &str = "SELECT rowid,write_cursor,body,kind FROM search_index \
     WHERE rowid>=?1 ORDER BY rowid LIMIT ?2";
#[cfg(feature = "operator")]
const SEARCH_V2_MEMBER_QUERY: &str = "SELECT rowid,write_cursor,body,kind FROM search_index_v2 \
     WHERE rowid>=?1 ORDER BY rowid LIMIT ?2";
#[cfg(feature = "operator")]
const EDGE_SEARCH_MEMBER_QUERY: &str =
    "SELECT rowid,write_cursor,body,kind FROM search_index_edges \
     WHERE rowid>=?1 ORDER BY rowid LIMIT ?2";
#[cfg(feature = "operator")]
const ATTRIBUTE_MEMBER_QUERY: &str =
    "SELECT rowid,write_cursor,attr_name,attr_value FROM canonical_attributes \
     WHERE rowid>=?1 ORDER BY rowid LIMIT ?2";
#[cfg(feature = "operator")]
const PROPERTY_MEMBER_QUERY: &str =
    "SELECT rowid,write_cursor,attr_name,attr_value FROM property_search_index \
     WHERE rowid>=?1 ORDER BY rowid LIMIT ?2";
#[cfg(feature = "operator")]
const ATTRIBUTE_OWNER_QUERY: &str =
    "SELECT n.write_cursor,r.revision_id,n.body,n.state,n.superseded_at FROM canonical_nodes n \
     INDEXED BY canonical_nodes_write_cursor_idx \
     LEFT JOIN _fathomdb_artifact_revisions r \
       ON r.artifact_class='node' AND r.write_cursor=n.write_cursor \
     WHERE n.write_cursor>?1 ORDER BY n.write_cursor LIMIT ?2";
#[cfg(feature = "operator")]
const DENSE_TERMINAL_MEMBER_QUERY: &str =
    "SELECT rowid,write_cursor FROM _fathomdb_projection_terminal \
     WHERE rowid>=?1 ORDER BY rowid LIMIT ?2";
#[cfg(feature = "operator")]
const DENSE_SIDECAR_MEMBER_QUERY: &str = "SELECT rowid,write_cursor FROM _fathomdb_vector_rows \
     WHERE rowid>=?1 ORDER BY rowid LIMIT ?2";
#[cfg(feature = "operator")]
const DENSE_VECTOR_MEMBER_QUERY: &str =
    "SELECT rowid,rowid FROM vector_default WHERE rowid>=?1 LIMIT ?2";
#[cfg(feature = "operator")]
const CURRENT_GENERATION_QUERY: &str =
    "SELECT c.generation_id,g.schema_version,g.role,g.retired_boundary,g.transition_boundary,\
            g.declaration_sha256 \
     FROM _fathomdb_projection_generation_current c \
     CROSS JOIN _fathomdb_projection_generations g ON g.generation_id=c.generation_id \
     WHERE c.singleton>?1 LIMIT ?2";
#[cfg(feature = "operator")]
const RECEIPT_GUARD_QUERY: &str = "SELECT rowid, \
            typeof(operation_id)='text' \
              AND length(CAST(operation_id AS BLOB)) BETWEEN 1 AND 128 \
              AND operation_id NOT GLOB '_fdb:*' \
              AND substr(operation_id,1,1) GLOB '[A-Za-z0-9]' \
              AND operation_id NOT GLOB '*[^A-Za-z0-9._:-]*', \
            typeof(schema_version)='integer' AND schema_version=1, \
            typeof(operations_count) IN ('integer','null'), \
            typeof(outcome)='text' AND length(outcome)<=25 \
              AND outcome IN ('committed','committed_closure_pending','refused','erased'), \
            typeof(resulting_write_boundary) IN ('integer','null'), \
            typeof(pending_projection_write_cursors_json)='text' \
              AND length(pending_projection_write_cursors_json)<=2945 \
              AND json_valid(pending_projection_write_cursors_json) \
              AND json_type(pending_projection_write_cursors_json)='array' \
              AND json_array_length(pending_projection_write_cursors_json)<=128, \
            typeof(projection_generation_id) IN ('text','null') \
              AND (projection_generation_id IS NULL OR \
                   (length(projection_generation_id)=38 \
                    AND projection_generation_id GLOB 'pgen1:[0-9a-f]*')), \
            CASE WHEN json_valid(pending_projection_write_cursors_json) \
                 AND json_type(pending_projection_write_cursors_json)='array' \
                 THEN json_array_length(pending_projection_write_cursors_json) END \
     FROM _fathomdb_actuation_receipts WHERE rowid>=?1 \
     ORDER BY rowid LIMIT ?2";

#[cfg(feature = "operator")]
#[derive(Clone, Debug, Eq, PartialEq)]
enum NullableValue<T> {
    Null,
    Value(T),
    Invalid,
}

#[cfg(feature = "operator")]
fn integer_value(row: &rusqlite::Row<'_>, index: usize) -> Option<i64> {
    match row.get_ref(index).ok()? {
        ValueRef::Integer(value) => Some(value),
        _ => None,
    }
}

#[cfg(feature = "operator")]
fn text_value(row: &rusqlite::Row<'_>, index: usize) -> Option<String> {
    match row.get_ref(index).ok()? {
        ValueRef::Text(value) => std::str::from_utf8(value).ok().map(ToOwned::to_owned),
        _ => None,
    }
}

#[cfg(feature = "operator")]
fn nullable_integer_value(row: &rusqlite::Row<'_>, index: usize) -> NullableValue<i64> {
    match row.get_ref(index) {
        Ok(ValueRef::Null) => NullableValue::Null,
        Ok(ValueRef::Integer(value)) => NullableValue::Value(value),
        _ => NullableValue::Invalid,
    }
}

#[cfg(feature = "operator")]
fn nullable_text_value(row: &rusqlite::Row<'_>, index: usize) -> NullableValue<String> {
    match row.get_ref(index) {
        Ok(ValueRef::Null) => NullableValue::Null,
        Ok(ValueRef::Text(value)) => std::str::from_utf8(value)
            .ok()
            .map(|value| NullableValue::Value(value.to_owned()))
            .unwrap_or(NullableValue::Invalid),
        _ => NullableValue::Invalid,
    }
}

#[cfg(all(feature = "operator", feature = "test-hooks"))]
pub(crate) fn candidate_queries_for_test() -> [&'static str; 12] {
    [
        NODE_BODY_OWNER_QUERY,
        EDGE_BODY_OWNER_QUERY,
        SEARCH_V1_MEMBER_QUERY,
        SEARCH_V2_MEMBER_QUERY,
        EDGE_SEARCH_MEMBER_QUERY,
        ATTRIBUTE_MEMBER_QUERY,
        PROPERTY_MEMBER_QUERY,
        DENSE_TERMINAL_MEMBER_QUERY,
        DENSE_SIDECAR_MEMBER_QUERY,
        DENSE_VECTOR_MEMBER_QUERY,
        CURRENT_GENERATION_QUERY,
        RECEIPT_GUARD_QUERY,
    ]
}

#[cfg(feature = "operator")]
struct DenseCandidates {
    valid: BTreeSet<u64>,
    invalid: usize,
}

#[cfg(feature = "operator")]
type PhysicalBodyMembers = BTreeMap<i64, Vec<(i64, Option<String>, Option<String>)>>;

#[cfg(feature = "operator")]
type PhysicalAttributeMembers = BTreeMap<(String, i64), Vec<(i64, Option<String>)>>;

#[cfg(feature = "operator")]
fn physical_dense_candidates(
    connection: &Connection,
    max_work_units: u32,
    aggregate_checked: &mut u32,
) -> Result<DenseCandidates, EngineError> {
    let mut candidates = BTreeSet::new();
    let mut invalid = 0usize;
    for sql in [DENSE_TERMINAL_MEMBER_QUERY, DENSE_SIDECAR_MEMBER_QUERY, DENSE_VECTOR_MEMBER_QUERY]
    {
        let remaining = max_work_units.saturating_sub(*aggregate_checked);
        let mut statement = connection.prepare(sql).map_err(|_| EngineError::Storage)?;
        let rows = statement
            .query_map(rusqlite::params![FIRST_SQLITE_ROWID, i64::from(remaining) + 1], |row| {
                Ok((integer_value(row, 0), integer_value(row, 1)))
            })
            .map_err(|_| EngineError::Storage)?;
        for candidate in rows {
            take_work(aggregate_checked, max_work_units)?;
            let (rowid, cursor) = candidate.map_err(|_| EngineError::Storage)?;
            if let Some(cursor) = rowid
                .zip(cursor)
                .filter(|(rowid, cursor)| rowid == cursor)
                .and_then(|(_, cursor)| u64::try_from(cursor).ok())
            {
                candidates.insert(cursor);
            } else {
                invalid = invalid.saturating_add(1);
            }
        }
    }
    Ok(DenseCandidates { valid: candidates, invalid })
}

#[cfg(feature = "operator")]
fn expected_attribute_members(
    connection: &Connection,
    name: &str,
    stored: &crate::StoredProjection,
    effective_at_epoch_s: i64,
    max_work_units: u32,
    aggregate_checked: &mut u32,
) -> Result<Vec<(i64, Option<String>, String)>, EngineError> {
    let mut members = Vec::new();
    let mut after_cursor = 0_i64;
    loop {
        let remaining = max_work_units.saturating_sub(*aggregate_checked);
        let limit = i64::from(remaining) + 1;
        let mut statement =
            connection.prepare(ATTRIBUTE_OWNER_QUERY).map_err(|_| EngineError::Storage)?;
        let owners = statement
            .query_map(rusqlite::params![after_cursor, limit], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<i64>>(4)?,
                ))
            })
            .map_err(|_| EngineError::Storage)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|_| EngineError::Storage)?;
        let exhausted = owners.len() < usize::try_from(limit).unwrap_or(usize::MAX);
        for (cursor, revision, body, state, superseded_at) in owners {
            after_cursor = cursor;
            take_work(aggregate_checked, max_work_units)?;
            if state != "active" || superseded_at.is_some() {
                continue;
            }
            let cursor_u64 = u64::try_from(cursor).map_err(|_| EngineError::Storage)?;
            if !crate::dependency_closure::projection_owner_is_eligible_at(
                connection,
                cursor_u64,
                effective_at_epoch_s,
            )? {
                continue;
            }
            let Some(value) = crate::extract_scalar_attribute(connection, &body, name, stored)
                .map_err(|_| EngineError::Storage)?
            else {
                continue;
            };
            members.push((cursor, revision, value));
        }
        if exhausted {
            break;
        }
    }
    Ok(members)
}

#[cfg(feature = "operator")]
struct StoredSourceLink {
    schema: Option<i64>,
    source_id: Option<String>,
    version_id: Option<String>,
    source_revision: Option<String>,
    locator: Option<String>,
    start: NullableValue<i64>,
    end: NullableValue<i64>,
    algorithm: Option<String>,
    digest: Option<String>,
}

#[cfg(feature = "operator")]
struct StoredArtifactOwner {
    schema: Option<i64>,
    class: Option<String>,
    role: Option<String>,
    completeness: Option<String>,
    cursor: Option<i64>,
}

#[cfg(feature = "operator")]
struct StoredCanonicalNode {
    source_id: Option<String>,
    body: Option<String>,
}

#[cfg(feature = "operator")]
fn load_artifact_owner(
    connection: &Connection,
    revision_id: &str,
) -> Result<Option<StoredArtifactOwner>, EngineError> {
    connection
        .query_row(
            "SELECT schema_version,artifact_class,artifact_role,completeness,write_cursor \
             FROM _fathomdb_artifact_revisions WHERE revision_id=?1",
            [revision_id],
            |row| {
                Ok(StoredArtifactOwner {
                    schema: integer_value(row, 0),
                    class: text_value(row, 1),
                    role: text_value(row, 2),
                    completeness: text_value(row, 3),
                    cursor: integer_value(row, 4),
                })
            },
        )
        .optional()
        .map_err(|_| EngineError::Storage)
}

#[cfg(feature = "operator")]
fn load_canonical_node(
    connection: &Connection,
    cursor: i64,
) -> Result<Option<StoredCanonicalNode>, EngineError> {
    connection
        .query_row(
            "SELECT source_id,body FROM canonical_nodes WHERE write_cursor=?1",
            [cursor],
            |row| {
                Ok(StoredCanonicalNode { source_id: text_value(row, 0), body: text_value(row, 1) })
            },
        )
        .optional()
        .map_err(|_| EngineError::Storage)
}

#[cfg(feature = "operator")]
fn load_canonical_self_link(
    connection: &Connection,
    source_revision: &str,
) -> Result<Option<[Option<String>; 4]>, EngineError> {
    connection
        .query_row(
            "SELECT source_id,source_version_id,hash_algorithm,hash_digest \
             FROM _fathomdb_source_links \
             WHERE artifact_revision_id=?1 AND source_revision_id=?1 \
             AND schema_version=1 \
             AND locator_kind='whole_body' AND start_byte IS NULL \
             AND end_byte IS NULL",
            [source_revision],
            |row| {
                Ok([text_value(row, 0), text_value(row, 1), text_value(row, 2), text_value(row, 3)])
            },
        )
        .optional()
        .map_err(|_| EngineError::Storage)
}

/// Closed set of operator integrity checks.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DataPlaneIntegrityCheckV1 {
    DependencyChain,
    ActiveSearchableOrphans,
    ProjectionGeneration,
    MutationReadiness,
}

impl DataPlaneIntegrityCheckV1 {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DependencyChain => "dependency_chain",
            Self::ActiveSearchableOrphans => "active_searchable_orphans",
            Self::ProjectionGeneration => "projection_generation",
            Self::MutationReadiness => "mutation_readiness",
        }
    }
}

/// Fixed-severity integrity finding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DataPlaneIntegritySeverityV1 {
    Error,
    Critical,
}

impl DataPlaneIntegritySeverityV1 {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Critical => "critical",
        }
    }
}

/// Stable finding code returned without canonical payload content.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(clippy::enum_variant_names)]
pub enum DataPlaneIntegrityFindingCodeV1 {
    DependencyRowInvalid,
    DependencyDerivedOwnerMissing,
    DependencyDerivedRoleInvalid,
    DependencySourceLinkMissing,
    DependencySourceLinkMismatch,
    DependencySourceOwnerMissing,
    DependencySourceRoleInvalid,
    DependencySourceVersionMismatch,
    DependencySourceSelfLinkMismatch,
    DependencyGenerationMismatch,
    NodeBodyFtsMissing,
    NodeBodyFtsV2Missing,
    EdgeBodyFtsMissing,
    CanonicalAttributeMissing,
    PropertyFtsMissing,
    SearchProjectionOwnerMissing,
    SearchProjectionOutsideMembership,
    SearchProjectionIdentityMismatch,
    DenseProjectionOwnerMissing,
    DenseProjectionPartial,
    DenseProjectionIdentityMismatch,
    DenseProjectionOutsideMembership,
    ProjectionGenerationCorrupt,
    ProjectionMemberCorrupt,
    MutationReceiptCorrupt,
    MutationReadinessUnavailable,
    MutationReadinessCorrupt,
}

impl DataPlaneIntegrityFindingCodeV1 {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DependencyRowInvalid => "dependency_row_invalid",
            Self::DependencyDerivedOwnerMissing => "dependency_derived_owner_missing",
            Self::DependencyDerivedRoleInvalid => "dependency_derived_role_invalid",
            Self::DependencySourceLinkMissing => "dependency_source_link_missing",
            Self::DependencySourceLinkMismatch => "dependency_source_link_mismatch",
            Self::DependencySourceOwnerMissing => "dependency_source_owner_missing",
            Self::DependencySourceRoleInvalid => "dependency_source_role_invalid",
            Self::DependencySourceVersionMismatch => "dependency_source_version_mismatch",
            Self::DependencySourceSelfLinkMismatch => "dependency_source_self_link_mismatch",
            Self::DependencyGenerationMismatch => "dependency_generation_mismatch",
            Self::NodeBodyFtsMissing => "node_body_fts_missing",
            Self::NodeBodyFtsV2Missing => "node_body_fts_v2_missing",
            Self::EdgeBodyFtsMissing => "edge_body_fts_missing",
            Self::CanonicalAttributeMissing => "canonical_attribute_missing",
            Self::PropertyFtsMissing => "property_fts_missing",
            Self::SearchProjectionOwnerMissing => "search_projection_owner_missing",
            Self::SearchProjectionOutsideMembership => "search_projection_outside_membership",
            Self::SearchProjectionIdentityMismatch => "search_projection_identity_mismatch",
            Self::DenseProjectionOwnerMissing => "dense_projection_owner_missing",
            Self::DenseProjectionPartial => "dense_projection_partial",
            Self::DenseProjectionIdentityMismatch => "dense_projection_identity_mismatch",
            Self::DenseProjectionOutsideMembership => "dense_projection_outside_membership",
            Self::ProjectionGenerationCorrupt => "projection_generation_corrupt",
            Self::ProjectionMemberCorrupt => "projection_member_corrupt",
            Self::MutationReceiptCorrupt => "mutation_receipt_corrupt",
            Self::MutationReadinessUnavailable => "mutation_readiness_unavailable",
            Self::MutationReadinessCorrupt => "mutation_readiness_corrupt",
        }
    }
}

/// Content-free evidence for one integrity violation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DataPlaneIntegrityFindingV1 {
    pub schema_version: u32,
    pub code: DataPlaneIntegrityFindingCodeV1,
    pub severity: DataPlaneIntegritySeverityV1,
    pub artifact_revision_ids: Vec<String>,
    pub dependency_id: Option<String>,
    pub projection_generation_id: Option<String>,
    pub operation_id: Option<String>,
    pub write_cursor: Option<u64>,
}

/// Database-local boundary shared by every requested integrity check.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DataPlaneIntegrityBoundaryV1 {
    pub schema_version: u32,
    pub effective_at_epoch_s: i64,
    pub observed_write_boundary: u64,
    pub dependency_generation: u64,
    pub projection_generation_id: String,
}

/// Bounded accounting for one check.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DataPlaneIntegrityCheckCountV1 {
    pub schema_version: u32,
    pub check: DataPlaneIntegrityCheckV1,
    pub checked_count: u32,
    pub finding_count: u32,
}

/// Complete bounded operator integrity report.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DataPlaneIntegrityResultV1 {
    pub schema_version: u32,
    pub read_boundary: DataPlaneIntegrityBoundaryV1,
    pub check_counts: Vec<DataPlaneIntegrityCheckCountV1>,
    pub checked_count: u32,
    pub findings: Vec<DataPlaneIntegrityFindingV1>,
    pub complete: bool,
}

/// Validated request for bounded operator integrity checks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DataPlaneIntegrityRequestV1 {
    pub schema_version: u32,
    pub checks: Vec<DataPlaneIntegrityCheckV1>,
    pub max_work_units: u32,
    pub max_findings: u32,
}

impl DataPlaneIntegrityRequestV1 {
    pub fn new(
        mut checks: Vec<DataPlaneIntegrityCheckV1>,
        max_work_units: u32,
        max_findings: u32,
    ) -> Result<Self, DataPlaneIntegrityErrorV1> {
        if checks.is_empty() {
            return Err(DataPlaneIntegrityErrorV1::new(
                DataPlaneIntegrityErrorReasonV1::ChecksEmpty,
                "/checks",
            ));
        }
        let mut seen = BTreeSet::new();
        for (index, check) in checks.iter().copied().enumerate() {
            if !seen.insert(check) {
                return Err(DataPlaneIntegrityErrorV1::new(
                    DataPlaneIntegrityErrorReasonV1::DuplicateCheck,
                    format!("/checks/{index}"),
                ));
            }
        }
        if !(1..=MAX_WORK_UNITS).contains(&max_work_units) {
            return Err(DataPlaneIntegrityErrorV1::new(
                DataPlaneIntegrityErrorReasonV1::IntegrityLimitInvalid,
                "/maxWorkUnits",
            ));
        }
        if !(1..=MAX_FINDINGS).contains(&max_findings) {
            return Err(DataPlaneIntegrityErrorV1::new(
                DataPlaneIntegrityErrorReasonV1::IntegrityLimitInvalid,
                "/maxFindings",
            ));
        }
        checks.sort_unstable();
        Ok(Self { schema_version: SCHEMA_VERSION, checks, max_work_units, max_findings })
    }

    pub fn all(max_work_units: u32, max_findings: u32) -> Result<Self, DataPlaneIntegrityErrorV1> {
        Self::new(
            vec![
                DataPlaneIntegrityCheckV1::DependencyChain,
                DataPlaneIntegrityCheckV1::ActiveSearchableOrphans,
                DataPlaneIntegrityCheckV1::ProjectionGeneration,
                DataPlaneIntegrityCheckV1::MutationReadiness,
            ],
            max_work_units,
            max_findings,
        )
    }
}

/// Closed reason for a data-plane-integrity refusal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DataPlaneIntegrityErrorReasonV1 {
    UnsupportedSchemaVersion,
    UnknownField,
    ChecksEmpty,
    DuplicateCheck,
    IntegrityCheckInvalid,
    IntegrityLimitInvalid,
    IntegrityBoundExceeded,
    IntegrityCorrupt,
}

impl DataPlaneIntegrityErrorReasonV1 {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UnsupportedSchemaVersion => "unsupported_schema_version",
            Self::UnknownField => "unknown_field",
            Self::ChecksEmpty => "checks_empty",
            Self::DuplicateCheck => "duplicate_check",
            Self::IntegrityCheckInvalid => "integrity_check_invalid",
            Self::IntegrityLimitInvalid => "integrity_limit_invalid",
            Self::IntegrityBoundExceeded => "integrity_bound_exceeded",
            Self::IntegrityCorrupt => "integrity_corrupt",
        }
    }
}

/// Typed, privacy-safe operator integrity error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DataPlaneIntegrityErrorV1 {
    pub schema_version: u32,
    pub reason: DataPlaneIntegrityErrorReasonV1,
    pub field_path: String,
}

impl DataPlaneIntegrityErrorV1 {
    pub(crate) fn new(reason: DataPlaneIntegrityErrorReasonV1, path: impl Into<String>) -> Self {
        Self { schema_version: SCHEMA_VERSION, reason, field_path: path.into() }
    }
}

impl Display for DataPlaneIntegrityErrorV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{} at {}", self.reason.as_str(), self.field_path)
    }
}

impl std::error::Error for DataPlaneIntegrityErrorV1 {}

#[cfg(feature = "operator")]
fn bound_error() -> EngineError {
    DataPlaneIntegrityErrorV1::new(
        DataPlaneIntegrityErrorReasonV1::IntegrityBoundExceeded,
        "/maxWorkUnits",
    )
    .into()
}

#[cfg(feature = "operator")]
fn finding_bound_error() -> EngineError {
    DataPlaneIntegrityErrorV1::new(
        DataPlaneIntegrityErrorReasonV1::IntegrityBoundExceeded,
        "/maxFindings",
    )
    .into()
}

#[cfg(feature = "operator")]
fn take_work(checked: &mut u32, limit: u32) -> Result<(), EngineError> {
    *checked = checked.checked_add(1).ok_or_else(bound_error)?;
    if *checked > limit {
        return Err(bound_error());
    }
    Ok(())
}

#[cfg(feature = "operator")]
fn push_finding(
    findings: &mut Vec<DataPlaneIntegrityFindingV1>,
    mut finding: DataPlaneIntegrityFindingV1,
    max_findings: u32,
) -> Result<(), EngineError> {
    if findings.len() >= max_findings as usize {
        return Err(finding_bound_error());
    }
    finding.artifact_revision_ids.retain(|value| crate::valid_caller_identity(value));
    finding.dependency_id =
        finding.dependency_id.filter(|value| crate::valid_caller_identity(value));
    finding.projection_generation_id =
        finding.projection_generation_id.filter(|value| valid_generation_id(value));
    finding.operation_id = finding.operation_id.filter(|value| crate::valid_caller_identity(value));
    findings.push(finding);
    Ok(())
}

#[cfg(feature = "operator")]
fn finding(
    code: DataPlaneIntegrityFindingCodeV1,
    severity: DataPlaneIntegritySeverityV1,
) -> DataPlaneIntegrityFindingV1 {
    DataPlaneIntegrityFindingV1 {
        schema_version: SCHEMA_VERSION,
        code,
        severity,
        artifact_revision_ids: Vec::new(),
        dependency_id: None,
        projection_generation_id: None,
        operation_id: None,
        write_cursor: None,
    }
}

#[cfg(feature = "operator")]
fn valid_generation_id(value: &str) -> bool {
    value.len() == 38
        && value.starts_with("pgen1:")
        && value[6..].bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(feature = "operator")]
fn canonical_u64(value: &str) -> Option<u64> {
    if value.is_empty()
        || (value.len() > 1 && value.starts_with('0'))
        || !value.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    value.parse().ok()
}

#[cfg(feature = "operator")]
fn integrity_dependency_generation(connection: &Connection) -> Result<Option<u64>, EngineError> {
    connection
        .query_row(
            "SELECT value FROM _fathomdb_open_state \
             WHERE key='_fathomdb_dependency_generation'",
            [],
            |row| {
                Ok(text_value(row, 0)
                    .and_then(|value| canonical_u64(&value))
                    .filter(|value| *value <= i64::MAX as u64))
            },
        )
        .optional()
        .map(|value| value.flatten())
        .map_err(|_| EngineError::Storage)
}

#[cfg(feature = "operator")]
fn dependency_findings(
    connection: &Connection,
    max_work_units: u32,
    max_findings: u32,
    aggregate_checked: &mut u32,
    findings: &mut Vec<DataPlaneIntegrityFindingV1>,
) -> Result<u32, EngineError> {
    let start = *aggregate_checked;
    take_work(aggregate_checked, max_work_units)?;
    let generation = integrity_dependency_generation(connection)?;
    if generation.is_none() {
        let item = finding(
            DataPlaneIntegrityFindingCodeV1::DependencyGenerationMismatch,
            DataPlaneIntegritySeverityV1::Critical,
        );
        push_finding(findings, item, max_findings)?;
    }
    let remaining = max_work_units.saturating_sub(*aggregate_checked);
    let sql =
        "SELECT schema_version,dependency_id,derived_revision_id,registered_dependency_generation \
               FROM _fathomdb_source_dependencies ORDER BY dependency_id LIMIT ?1";
    let mut statement = connection.prepare(sql).map_err(|_| EngineError::Storage)?;
    let mut rows = statement.query([i64::from(remaining) + 1]).map_err(|_| EngineError::Storage)?;
    while let Some(row) = rows.next().map_err(|_| EngineError::Storage)? {
        take_work(aggregate_checked, max_work_units)?;
        let schema = integer_value(row, 0);
        let dependency_id = text_value(row, 1).filter(|value| crate::valid_caller_identity(value));
        let derived_revision =
            text_value(row, 2).filter(|value| crate::valid_caller_identity(value));
        let registered_generation = integer_value(row, 3);
        if schema != Some(1)
            || dependency_id.is_none()
            || derived_revision.is_none()
            || registered_generation.is_none_or(|value| value < 0)
        {
            let mut item = finding(
                DataPlaneIntegrityFindingCodeV1::DependencyRowInvalid,
                DataPlaneIntegritySeverityV1::Error,
            );
            item.dependency_id = dependency_id;
            push_finding(findings, item, max_findings)?;
            continue;
        }
        let dependency_id = dependency_id.expect("validated dependency id");
        let derived_revision = derived_revision.expect("validated derived revision");
        let registered_generation = registered_generation.expect("validated generation");
        let mut item = None;
        let derived_owner = load_artifact_owner(connection, &derived_revision)?;
        if item.is_none() && derived_owner.is_none() {
            item = Some(finding(
                DataPlaneIntegrityFindingCodeV1::DependencyDerivedOwnerMissing,
                DataPlaneIntegritySeverityV1::Critical,
            ));
        }
        if item.is_none() {
            let owner = derived_owner.as_ref().expect("owner presence checked");
            let owner_exists = match (owner.class.as_deref(), owner.cursor) {
                (Some("node"), Some(cursor)) => connection
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM canonical_nodes WHERE write_cursor=?1)",
                        [cursor],
                        |row| row.get(0),
                    )
                    .map_err(|_| EngineError::Storage)?,
                (Some("edge"), Some(cursor)) => connection
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM canonical_edges WHERE write_cursor=?1)",
                        [cursor],
                        |row| row.get(0),
                    )
                    .map_err(|_| EngineError::Storage)?,
                _ => false,
            };
            if !owner_exists {
                item = Some(finding(
                    DataPlaneIntegrityFindingCodeV1::DependencyDerivedOwnerMissing,
                    DataPlaneIntegritySeverityV1::Critical,
                ));
            } else if owner.schema != Some(1)
                || owner.role.as_deref() != Some("derived_semantic")
                || owner.completeness.as_deref() != Some("complete")
            {
                item = Some(finding(
                    DataPlaneIntegrityFindingCodeV1::DependencyDerivedRoleInvalid,
                    DataPlaneIntegritySeverityV1::Error,
                ));
            }
        }
        let link: Option<StoredSourceLink> = connection
                .query_row(
                    "SELECT schema_version,source_id,source_version_id,source_revision_id,locator_kind,\
                            start_byte,end_byte,hash_algorithm,hash_digest \
                     FROM _fathomdb_source_links WHERE artifact_revision_id=?1",
                    [&derived_revision],
                    |row| {
                        Ok(StoredSourceLink {
                            schema: integer_value(row, 0),
                            source_id: text_value(row, 1)
                                .filter(|value| crate::valid_caller_identity(value)),
                            version_id: text_value(row, 2)
                                .filter(|value| crate::valid_caller_identity(value)),
                            source_revision: text_value(row, 3)
                                .filter(|value| crate::valid_caller_identity(value)),
                            locator: text_value(row, 4),
                            start: nullable_integer_value(row, 5),
                            end: nullable_integer_value(row, 6),
                            algorithm: text_value(row, 7),
                            digest: text_value(row, 8),
                        })
                    },
                )
                .optional()
                .map_err(|_| EngineError::Storage)?;
        if item.is_none() && link.is_none() {
            item = Some(finding(
                DataPlaneIntegrityFindingCodeV1::DependencySourceLinkMissing,
                DataPlaneIntegritySeverityV1::Critical,
            ));
        }
        if item.is_none() {
            let link = link.as_ref().unwrap();
            if link.source_revision.as_deref() == Some(derived_revision.as_str()) {
                item = Some(finding(
                    DataPlaneIntegrityFindingCodeV1::DependencyDerivedRoleInvalid,
                    DataPlaneIntegritySeverityV1::Error,
                ));
            }
        }
        if item.is_none() {
            let link = link.as_ref().unwrap();
            let whole_body = link.locator.as_deref() == Some("whole_body")
                && link.start == NullableValue::Null
                && link.end == NullableValue::Null;
            let byte_range = matches!(
                (&link.locator, &link.start, &link.end),
                (
                    Some(locator),
                    NullableValue::Value(start),
                    NullableValue::Value(end)
                ) if locator == "utf8_bytes" && *start >= 0 && *end > *start
            );
            let link_valid = link.schema == Some(1)
                && link.source_id.is_some()
                && link.version_id.is_some()
                && link.source_revision.is_some()
                && (whole_body || byte_range)
                && link.algorithm.as_deref() == Some("sha256")
                && link.digest.as_ref().is_some_and(|digest| digest.len() == 64)
                && link.digest.as_ref().is_some_and(|digest| {
                    digest
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
                });
            if !link_valid {
                item = Some(finding(
                    DataPlaneIntegrityFindingCodeV1::DependencySourceLinkMismatch,
                    DataPlaneIntegritySeverityV1::Critical,
                ));
            } else {
                let source_id = link.source_id.as_ref().expect("validated source id");
                let version_id = link.version_id.as_ref().expect("validated source version");
                let source_revision =
                    link.source_revision.as_ref().expect("validated source revision");
                let locator = link.locator.as_deref().expect("validated locator");
                let start = match link.start {
                    NullableValue::Value(value) => Some(value),
                    NullableValue::Null | NullableValue::Invalid => None,
                };
                let end = match link.end {
                    NullableValue::Value(value) => Some(value),
                    NullableValue::Null | NullableValue::Invalid => None,
                };
                let digest = link.digest.as_ref().expect("validated digest");
                let source_owner = load_artifact_owner(connection, source_revision)?;
                if let Some(owner) = source_owner {
                    let canonical_node = match (owner.class.as_deref(), owner.cursor) {
                        (Some("node"), Some(cursor)) => load_canonical_node(connection, cursor)?,
                        _ => None,
                    };
                    if let Some(canonical_node) = canonical_node {
                        if owner.schema != Some(1)
                            || owner.role.as_deref() != Some("canonical_source")
                            || owner.completeness.as_deref() != Some("complete")
                        {
                            item = Some(finding(
                                DataPlaneIntegrityFindingCodeV1::DependencySourceRoleInvalid,
                                DataPlaneIntegritySeverityV1::Error,
                            ));
                        } else {
                            let selected_digest = canonical_node.body.as_ref().and_then(|body| {
                                let selected = match (locator, start, end) {
                                    ("whole_body", None, None) => Some(body.as_bytes()),
                                    ("utf8_bytes", Some(start), Some(end)) => {
                                        usize::try_from(start)
                                            .ok()
                                            .zip(usize::try_from(end).ok())
                                            .and_then(|(start, end)| {
                                                (body.is_char_boundary(start)
                                                    && body.is_char_boundary(end))
                                                .then(|| body.as_bytes().get(start..end))
                                                .flatten()
                                            })
                                    }
                                    _ => None,
                                };
                                selected.map(|bytes| {
                                    Sha256::digest(bytes)
                                        .iter()
                                        .map(|byte| format!("{byte:02x}"))
                                        .collect::<String>()
                                })
                            });
                            if canonical_node.source_id.as_deref() != Some(source_id)
                                || selected_digest.as_deref() != Some(digest.as_str())
                            {
                                item = Some(finding(
                                    DataPlaneIntegrityFindingCodeV1::DependencySourceLinkMismatch,
                                    DataPlaneIntegritySeverityV1::Critical,
                                ));
                            }
                            let version_matches: bool = connection
                                .query_row(
                                    "SELECT EXISTS(SELECT 1 FROM _fathomdb_source_versions \
                                     WHERE source_revision_id=?1 AND source_id=?2 AND source_version_id=?3 \
                                     AND schema_version=1)",
                                    rusqlite::params![source_revision, source_id, version_id],
                                    |row| row.get(0),
                                )
                                .map_err(|_| EngineError::Storage)?;
                            if item.is_none() && !version_matches {
                                item = Some(finding(
                                    DataPlaneIntegrityFindingCodeV1::DependencySourceVersionMismatch,
                                    DataPlaneIntegritySeverityV1::Critical,
                                ));
                            } else if item.is_none() {
                                let self_link =
                                    load_canonical_self_link(connection, source_revision)?;
                                let canonical_digest = canonical_node.body.as_ref().map(|body| {
                                    Sha256::digest(body.as_bytes())
                                        .iter()
                                        .map(|byte| format!("{byte:02x}"))
                                        .collect::<String>()
                                });
                                if !self_link.is_some_and(|values| {
                                    values[0].as_deref() == Some(source_id)
                                        && values[1].as_deref() == Some(version_id)
                                        && values[2].as_deref() == Some("sha256")
                                        && values[3].as_ref() == canonical_digest.as_ref()
                                }) {
                                    item = Some(finding(
                                        DataPlaneIntegrityFindingCodeV1::DependencySourceSelfLinkMismatch,
                                        DataPlaneIntegritySeverityV1::Critical,
                                    ));
                                }
                            }
                        }
                    } else {
                        item = Some(finding(
                            DataPlaneIntegrityFindingCodeV1::DependencySourceOwnerMissing,
                            DataPlaneIntegritySeverityV1::Critical,
                        ));
                    }
                } else {
                    item = Some(finding(
                        DataPlaneIntegrityFindingCodeV1::DependencySourceOwnerMissing,
                        DataPlaneIntegritySeverityV1::Critical,
                    ));
                }
            }
        }
        if item.is_none()
            && generation.is_some_and(|generation| {
                u64::try_from(registered_generation)
                    .ok()
                    .is_none_or(|value| value == 0 || value > generation)
            })
        {
            item = Some(finding(
                DataPlaneIntegrityFindingCodeV1::DependencyGenerationMismatch,
                DataPlaneIntegritySeverityV1::Critical,
            ));
        }
        if let Some(mut item) = item {
            if crate::valid_caller_identity(&dependency_id) {
                item.dependency_id = Some(dependency_id);
            }
            let source_revision = link.as_ref().and_then(|stored| stored.source_revision.as_ref());
            match item.code {
                DataPlaneIntegrityFindingCodeV1::DependencyDerivedOwnerMissing
                | DataPlaneIntegrityFindingCodeV1::DependencyDerivedRoleInvalid
                | DataPlaneIntegrityFindingCodeV1::DependencySourceLinkMissing => {
                    item.artifact_revision_ids.push(derived_revision);
                }
                DataPlaneIntegrityFindingCodeV1::DependencySourceLinkMismatch => {
                    item.artifact_revision_ids.push(derived_revision);
                    item.artifact_revision_ids.extend(source_revision.cloned());
                }
                DataPlaneIntegrityFindingCodeV1::DependencySourceOwnerMissing
                | DataPlaneIntegrityFindingCodeV1::DependencySourceRoleInvalid
                | DataPlaneIntegrityFindingCodeV1::DependencySourceVersionMismatch
                | DataPlaneIntegrityFindingCodeV1::DependencySourceSelfLinkMismatch => {
                    item.artifact_revision_ids.extend(source_revision.cloned());
                }
                DataPlaneIntegrityFindingCodeV1::DependencyGenerationMismatch
                | DataPlaneIntegrityFindingCodeV1::DependencyRowInvalid
                | DataPlaneIntegrityFindingCodeV1::NodeBodyFtsMissing
                | DataPlaneIntegrityFindingCodeV1::NodeBodyFtsV2Missing
                | DataPlaneIntegrityFindingCodeV1::EdgeBodyFtsMissing
                | DataPlaneIntegrityFindingCodeV1::CanonicalAttributeMissing
                | DataPlaneIntegrityFindingCodeV1::PropertyFtsMissing
                | DataPlaneIntegrityFindingCodeV1::SearchProjectionOwnerMissing
                | DataPlaneIntegrityFindingCodeV1::SearchProjectionOutsideMembership
                | DataPlaneIntegrityFindingCodeV1::SearchProjectionIdentityMismatch
                | DataPlaneIntegrityFindingCodeV1::DenseProjectionOwnerMissing
                | DataPlaneIntegrityFindingCodeV1::DenseProjectionPartial
                | DataPlaneIntegrityFindingCodeV1::DenseProjectionIdentityMismatch
                | DataPlaneIntegrityFindingCodeV1::DenseProjectionOutsideMembership
                | DataPlaneIntegrityFindingCodeV1::ProjectionGenerationCorrupt
                | DataPlaneIntegrityFindingCodeV1::ProjectionMemberCorrupt
                | DataPlaneIntegrityFindingCodeV1::MutationReceiptCorrupt
                | DataPlaneIntegrityFindingCodeV1::MutationReadinessUnavailable
                | DataPlaneIntegrityFindingCodeV1::MutationReadinessCorrupt => {}
            }
            push_finding(findings, item, max_findings)?;
        }
    }
    Ok(*aggregate_checked - start)
}

#[cfg(feature = "operator")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OwnerRetention {
    RequiredMembership,
    LegitimateRetained,
    GovernedPruningResidue,
    CanonicalOwnerMissing,
}

#[cfg(feature = "operator")]
#[derive(Clone, Copy)]
enum RetentionPurpose {
    Body,
    Dense { required: bool },
}

#[cfg(feature = "operator")]
fn owner_retention_at(
    connection: &Connection,
    artifact_class: &str,
    cursor: i64,
    effective_at_epoch_s: i64,
    purpose: RetentionPurpose,
) -> Result<OwnerRetention, EngineError> {
    let cursor_u64 = u64::try_from(cursor).map_err(|_| EngineError::Storage)?;
    if artifact_class == "node" {
        let owner = connection
            .query_row(
                "SELECT kind,state,superseded_at FROM canonical_nodes WHERE write_cursor=?1",
                [cursor],
                |row| Ok((text_value(row, 0), text_value(row, 1), nullable_integer_value(row, 2))),
            )
            .optional()
            .map_err(|_| EngineError::Storage)?;
        let Some((kind, state, superseded)) = owner else {
            return Ok(OwnerRetention::CanonicalOwnerMissing);
        };
        let eligible = crate::dependency_closure::projection_owner_is_eligible_at(
            connection,
            cursor_u64,
            effective_at_epoch_s,
        )?;
        if !eligible {
            return Ok(OwnerRetention::GovernedPruningResidue);
        }
        return Ok(match purpose {
            RetentionPurpose::Body => OwnerRetention::RequiredMembership,
            RetentionPurpose::Dense { required } => {
                if state.as_deref() == Some("deleted") || !matches!(superseded, NullableValue::Null)
                {
                    OwnerRetention::GovernedPruningResidue
                } else if required {
                    OwnerRetention::RequiredMembership
                } else if kind.as_deref().is_some_and(crate::kind_is_vector_committable) {
                    OwnerRetention::LegitimateRetained
                } else {
                    OwnerRetention::GovernedPruningResidue
                }
            }
        });
    }
    let owner = connection
        .query_row(
            "SELECT body,superseded_at,t_invalid FROM canonical_edges WHERE write_cursor=?1",
            [cursor],
            |row| {
                Ok((
                    nullable_text_value(row, 0),
                    nullable_integer_value(row, 1),
                    nullable_integer_value(row, 2),
                ))
            },
        )
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let Some((body, superseded, t_invalid)) = owner else {
        return Ok(OwnerRetention::CanonicalOwnerMissing);
    };
    let eligible = crate::dependency_closure::projection_owner_is_eligible_at(
        connection,
        cursor_u64,
        effective_at_epoch_s,
    )?;
    if !eligible
        || !matches!(body, NullableValue::Value(_))
        || !matches!(superseded, NullableValue::Null)
    {
        return Ok(OwnerRetention::GovernedPruningResidue);
    }
    if matches!(t_invalid, NullableValue::Value(end) if end <= effective_at_epoch_s) {
        return Ok(OwnerRetention::LegitimateRetained);
    }
    if !matches!(t_invalid, NullableValue::Null | NullableValue::Value(_)) {
        return Ok(OwnerRetention::GovernedPruningResidue);
    }
    Ok(match purpose {
        RetentionPurpose::Body => OwnerRetention::RequiredMembership,
        RetentionPurpose::Dense { required: true } => OwnerRetention::RequiredMembership,
        RetentionPurpose::Dense { required: false } => OwnerRetention::LegitimateRetained,
    })
}

#[cfg(feature = "operator")]
fn revision_for_owner(
    connection: &Connection,
    artifact_class: &str,
    cursor: i64,
) -> Result<Option<String>, EngineError> {
    connection
        .query_row(
            "SELECT revision_id FROM _fathomdb_artifact_revisions \
             WHERE artifact_class=?1 AND write_cursor=?2",
            rusqlite::params![artifact_class, cursor],
            |row| Ok(text_value(row, 0)),
        )
        .optional()
        .map(|value| value.flatten().filter(|value| crate::valid_caller_identity(value)))
        .map_err(|_| EngineError::Storage)
}

#[cfg(feature = "operator")]
fn revision_for_cursor(
    connection: &Connection,
    cursor: i64,
) -> Result<Option<String>, EngineError> {
    connection
        .query_row(
            "SELECT revision_id FROM _fathomdb_artifact_revisions \
             WHERE write_cursor=?1 ORDER BY artifact_class LIMIT 1",
            [cursor],
            |row| Ok(text_value(row, 0)),
        )
        .optional()
        .map(|value| value.flatten().filter(|value| crate::valid_caller_identity(value)))
        .map_err(|_| EngineError::Storage)
}

#[cfg(feature = "operator")]
struct DensePhysicalShape {
    present_count: u8,
    complete: bool,
    terminal_only: bool,
}

#[cfg(feature = "operator")]
fn dense_physical_shape(
    connection: &Connection,
    cursor: i64,
    expected_kind: &str,
) -> Result<DensePhysicalShape, EngineError> {
    let terminal = connection
        .query_row(
            "SELECT state FROM _fathomdb_projection_terminal WHERE write_cursor=?1",
            [cursor],
            |row| Ok(text_value(row, 0)),
        )
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let sidecar = connection
        .query_row(
            "SELECT rowid,kind FROM _fathomdb_vector_rows WHERE write_cursor=?1",
            [cursor],
            |row| Ok((integer_value(row, 0), text_value(row, 1))),
        )
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let vector = connection
        .query_row("SELECT source_type,kind FROM vector_default WHERE rowid=?1", [cursor], |row| {
            Ok((text_value(row, 0), text_value(row, 1)))
        })
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let present_count =
        u8::from(terminal.is_some()) + u8::from(sidecar.is_some()) + u8::from(vector.is_some());
    let expected_source_type = crate::resolve_source_type(expected_kind)?;
    let complete = terminal.as_ref().is_some_and(|state| state.as_deref() == Some("up_to_date"))
        && sidecar.as_ref().is_some_and(|(rowid, kind)| {
            rowid.is_some_and(|rowid| rowid == cursor) && kind.as_deref() == Some(expected_kind)
        })
        && vector.as_ref().is_some_and(|(source_type, kind)| {
            source_type.as_deref() == Some(expected_source_type)
                && kind.as_deref() == Some(expected_kind)
        });
    Ok(DensePhysicalShape {
        present_count,
        complete,
        terminal_only: terminal.is_some() && sidecar.is_none() && vector.is_none(),
    })
}

#[cfg(feature = "operator")]
fn body_residue_finding(
    connection: &Connection,
    artifact_class: &str,
    cursor: i64,
    physical_body: Option<&str>,
    physical_kind: Option<&str>,
    effective_at_epoch_s: i64,
) -> Result<Option<DataPlaneIntegrityFindingV1>, EngineError> {
    let revision = revision_for_owner(connection, artifact_class, cursor)?;
    let retention = owner_retention_at(
        connection,
        artifact_class,
        cursor,
        effective_at_epoch_s,
        RetentionPurpose::Body,
    )?;
    if retention == OwnerRetention::LegitimateRetained {
        let canonical = connection
            .query_row(
                "SELECT body,kind FROM canonical_edges WHERE write_cursor=?1",
                [cursor],
                |row| Ok((nullable_text_value(row, 0), text_value(row, 1))),
            )
            .optional()
            .map_err(|_| EngineError::Storage)?;
        if canonical.is_some_and(|(body, kind)| {
            matches!(body, NullableValue::Value(value) if Some(value.as_str()) == physical_body)
                && kind.as_deref() == physical_kind
        }) {
            return Ok(None);
        }
    }
    let cursor_u64 = u64::try_from(cursor).map_err(|_| EngineError::Storage)?;
    let mut item = finding(
        match retention {
            OwnerRetention::RequiredMembership | OwnerRetention::LegitimateRetained => {
                DataPlaneIntegrityFindingCodeV1::SearchProjectionIdentityMismatch
            }
            OwnerRetention::GovernedPruningResidue => {
                DataPlaneIntegrityFindingCodeV1::SearchProjectionOutsideMembership
            }
            OwnerRetention::CanonicalOwnerMissing => {
                DataPlaneIntegrityFindingCodeV1::SearchProjectionOwnerMissing
            }
        },
        match retention {
            OwnerRetention::RequiredMembership | OwnerRetention::LegitimateRetained => {
                DataPlaneIntegritySeverityV1::Error
            }
            OwnerRetention::GovernedPruningResidue | OwnerRetention::CanonicalOwnerMissing => {
                DataPlaneIntegritySeverityV1::Critical
            }
        },
    );
    item.write_cursor = Some(cursor_u64);
    item.artifact_revision_ids = revision.into_iter().collect();
    Ok(Some(item))
}

#[cfg(feature = "operator")]
fn active_projection_findings(
    connection: &Connection,
    effective_at_epoch_s: i64,
    max_work_units: u32,
    max_findings: u32,
    aggregate_checked: &mut u32,
    findings: &mut Vec<DataPlaneIntegrityFindingV1>,
) -> Result<u32, EngineError> {
    let start = *aggregate_checked;
    let remaining = max_work_units.saturating_sub(*aggregate_checked);
    let mut registry = connection
        .prepare("SELECT name FROM _fathomdb_projection_registry ORDER BY name LIMIT ?1")
        .map_err(|_| EngineError::Storage)?;
    let names = registry
        .query_map([i64::from(remaining) + 1], |row| row.get::<_, String>(0))
        .map_err(|_| EngineError::Storage)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|_| EngineError::Storage)?;
    let mut registry_snapshot = BTreeMap::new();
    for name in names {
        take_work(aggregate_checked, max_work_units)?;
        let stored = crate::load_projection_registry_row(connection, &name)
            .map_err(|_| EngineError::Storage)?
            .ok_or(EngineError::Storage)?;
        registry_snapshot.insert(name, stored);
    }
    drop(registry);

    for (physical_query, code) in [
        (SEARCH_V1_MEMBER_QUERY, DataPlaneIntegrityFindingCodeV1::NodeBodyFtsMissing),
        (SEARCH_V2_MEMBER_QUERY, DataPlaneIntegrityFindingCodeV1::NodeBodyFtsV2Missing),
    ] {
        let remaining = max_work_units.saturating_sub(*aggregate_checked);
        let mut expected =
            connection.prepare(NODE_BODY_OWNER_QUERY).map_err(|_| EngineError::Storage)?;
        let entries = expected
            .query_map(rusqlite::params![0_i64, i64::from(remaining) + 1], |row| {
                Ok((
                    integer_value(row, 0),
                    text_value(row, 1),
                    text_value(row, 2),
                    text_value(row, 3),
                ))
            })
            .map_err(|_| EngineError::Storage)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|_| EngineError::Storage)?;
        let mut expected_by_cursor = BTreeMap::new();
        for (cursor, revision, body, kind) in entries {
            take_work(aggregate_checked, max_work_units)?;
            let Some(cursor) = cursor else {
                let item = finding(
                    DataPlaneIntegrityFindingCodeV1::SearchProjectionIdentityMismatch,
                    DataPlaneIntegritySeverityV1::Error,
                );
                push_finding(findings, item, max_findings)?;
                continue;
            };
            if owner_retention_at(
                connection,
                "node",
                cursor,
                effective_at_epoch_s,
                RetentionPurpose::Body,
            )? != OwnerRetention::RequiredMembership
            {
                continue;
            }
            expected_by_cursor.insert(cursor, (revision, body, kind));
        }
        let remaining = max_work_units.saturating_sub(*aggregate_checked);
        let mut physical_statement =
            connection.prepare(physical_query).map_err(|_| EngineError::Storage)?;
        let physical = physical_statement
            .query_map(rusqlite::params![FIRST_SQLITE_ROWID, i64::from(remaining) + 1], |row| {
                Ok((
                    integer_value(row, 0),
                    integer_value(row, 1),
                    text_value(row, 2),
                    text_value(row, 3),
                ))
            })
            .map_err(|_| EngineError::Storage)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|_| EngineError::Storage)?;
        let mut physical_by_cursor = PhysicalBodyMembers::new();
        let mut invalid_physical = Vec::new();
        for (rowid, cursor, body, kind) in physical {
            take_work(aggregate_checked, max_work_units)?;
            match (rowid, cursor) {
                (Some(rowid), Some(cursor)) if cursor >= 0 => {
                    physical_by_cursor.entry(cursor).or_default().push((rowid, body, kind));
                }
                (Some(rowid), _) => invalid_physical.push(rowid),
                (None, _) => invalid_physical.push(i64::MAX),
            }
        }
        for (cursor, (revision, body, kind)) in &expected_by_cursor {
            let candidates = physical_by_cursor.get(cursor);
            if candidates.is_none() {
                let mut item = finding(code, DataPlaneIntegritySeverityV1::Critical);
                item.write_cursor = u64::try_from(*cursor).ok();
                item.artifact_revision_ids = revision.iter().cloned().collect();
                push_finding(findings, item, max_findings)?;
            } else if candidates.is_none_or(|rows| {
                rows.len() != 1
                    || rows[0].1.as_ref() != body.as_ref()
                    || rows[0].2.as_ref() != kind.as_ref()
                    || body.is_none()
                    || kind.is_none()
            }) {
                let mut item = finding(
                    DataPlaneIntegrityFindingCodeV1::SearchProjectionIdentityMismatch,
                    DataPlaneIntegritySeverityV1::Error,
                );
                item.write_cursor = u64::try_from(*cursor).ok();
                item.artifact_revision_ids = revision.iter().cloned().collect();
                push_finding(findings, item, max_findings)?;
            }
        }
        let mut residue = physical_by_cursor
            .iter()
            .filter(|(cursor, _)| !expected_by_cursor.contains_key(cursor))
            .flat_map(|(cursor, rows)| {
                rows.iter().map(move |row| (row.0, *cursor, row.1.as_deref(), row.2.as_deref()))
            })
            .collect::<Vec<_>>();
        residue.sort_unstable_by_key(|row| row.0);
        for (_, cursor, body, kind) in residue {
            if let Some(item) =
                body_residue_finding(connection, "node", cursor, body, kind, effective_at_epoch_s)?
            {
                push_finding(findings, item, max_findings)?;
            }
        }
        invalid_physical.sort_unstable();
        for _ in invalid_physical {
            let item = finding(
                DataPlaneIntegrityFindingCodeV1::SearchProjectionIdentityMismatch,
                DataPlaneIntegritySeverityV1::Error,
            );
            push_finding(findings, item, max_findings)?;
        }
    }

    let remaining = max_work_units.saturating_sub(*aggregate_checked);
    let mut edge_statement =
        connection.prepare(EDGE_BODY_OWNER_QUERY).map_err(|_| EngineError::Storage)?;
    let edge_entries = edge_statement
        .query_map(rusqlite::params![0_i64, i64::from(remaining) + 1], |row| {
            Ok((
                integer_value(row, 0),
                text_value(row, 1),
                nullable_text_value(row, 2),
                text_value(row, 3),
            ))
        })
        .map_err(|_| EngineError::Storage)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|_| EngineError::Storage)?;
    let mut expected_edges = BTreeMap::new();
    for (cursor, revision, body, kind) in edge_entries {
        take_work(aggregate_checked, max_work_units)?;
        let Some(cursor) = cursor else {
            let item = finding(
                DataPlaneIntegrityFindingCodeV1::SearchProjectionIdentityMismatch,
                DataPlaneIntegritySeverityV1::Error,
            );
            push_finding(findings, item, max_findings)?;
            continue;
        };
        if owner_retention_at(
            connection,
            "edge",
            cursor,
            effective_at_epoch_s,
            RetentionPurpose::Body,
        )? != OwnerRetention::RequiredMembership
        {
            continue;
        }
        let NullableValue::Value(body) = body else {
            let mut item = finding(
                DataPlaneIntegrityFindingCodeV1::SearchProjectionIdentityMismatch,
                DataPlaneIntegritySeverityV1::Error,
            );
            item.write_cursor = u64::try_from(cursor).ok();
            item.artifact_revision_ids = revision.into_iter().collect();
            push_finding(findings, item, max_findings)?;
            continue;
        };
        expected_edges.insert(cursor, (revision, body, kind));
    }
    let remaining = max_work_units.saturating_sub(*aggregate_checked);
    let mut edge_physical_statement =
        connection.prepare(EDGE_SEARCH_MEMBER_QUERY).map_err(|_| EngineError::Storage)?;
    let edge_physical = edge_physical_statement
        .query_map(rusqlite::params![FIRST_SQLITE_ROWID, i64::from(remaining) + 1], |row| {
            Ok((
                integer_value(row, 0),
                integer_value(row, 1),
                text_value(row, 2),
                text_value(row, 3),
            ))
        })
        .map_err(|_| EngineError::Storage)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|_| EngineError::Storage)?;
    let mut physical_edges = PhysicalBodyMembers::new();
    let mut invalid_edge_physical = Vec::new();
    for (rowid, cursor, body, kind) in edge_physical {
        take_work(aggregate_checked, max_work_units)?;
        match (rowid, cursor) {
            (Some(rowid), Some(cursor)) if cursor >= 0 => {
                physical_edges.entry(cursor).or_default().push((rowid, body, kind));
            }
            (Some(rowid), _) => invalid_edge_physical.push(rowid),
            (None, _) => invalid_edge_physical.push(i64::MAX),
        }
    }
    for (cursor, (revision, body, kind)) in &expected_edges {
        match physical_edges.get(cursor) {
            None => {
                let mut item = finding(
                    DataPlaneIntegrityFindingCodeV1::EdgeBodyFtsMissing,
                    DataPlaneIntegritySeverityV1::Critical,
                );
                item.write_cursor = u64::try_from(*cursor).ok();
                item.artifact_revision_ids = revision.iter().cloned().collect();
                push_finding(findings, item, max_findings)?;
            }
            Some(rows)
                if rows.len() != 1
                    || rows[0].1.as_deref() != Some(body.as_str())
                    || rows[0].2.as_ref() != kind.as_ref()
                    || kind.is_none() =>
            {
                let mut item = finding(
                    DataPlaneIntegrityFindingCodeV1::SearchProjectionIdentityMismatch,
                    DataPlaneIntegritySeverityV1::Error,
                );
                item.write_cursor = u64::try_from(*cursor).ok();
                item.artifact_revision_ids = revision.iter().cloned().collect();
                push_finding(findings, item, max_findings)?;
            }
            Some(_) => {}
        }
    }
    let mut edge_residue = physical_edges
        .iter()
        .filter(|(cursor, _)| !expected_edges.contains_key(cursor))
        .flat_map(|(cursor, rows)| {
            rows.iter().map(move |row| (row.0, *cursor, row.1.as_deref(), row.2.as_deref()))
        })
        .collect::<Vec<_>>();
    edge_residue.sort_unstable_by_key(|row| row.0);
    for (_, cursor, body, kind) in edge_residue {
        if let Some(item) =
            body_residue_finding(connection, "edge", cursor, body, kind, effective_at_epoch_s)?
        {
            push_finding(findings, item, max_findings)?;
        }
    }
    invalid_edge_physical.sort_unstable();
    for _ in invalid_edge_physical {
        let item = finding(
            DataPlaneIntegrityFindingCodeV1::SearchProjectionIdentityMismatch,
            DataPlaneIntegritySeverityV1::Error,
        );
        push_finding(findings, item, max_findings)?;
    }

    let mut expected_dense = BTreeMap::new();
    let dense_authority_enabled = crate::vector_projection_declared(connection)
        .map_err(|_| EngineError::Storage)?
        || connection
            .query_row("SELECT EXISTS(SELECT 1 FROM _fathomdb_vector_kinds)", [], |row| row.get(0))
            .map_err(|_| EngineError::Storage)?;
    for (sql, class) in if dense_authority_enabled {
        &[(NODE_BODY_OWNER_QUERY, "node"), (EDGE_BODY_OWNER_QUERY, "edge")][..]
    } else {
        &[]
    } {
        let remaining = max_work_units.saturating_sub(*aggregate_checked);
        let mut statement = connection.prepare(sql).map_err(|_| EngineError::Storage)?;
        let cursors = statement
            .query_map(rusqlite::params![0_i64, i64::from(remaining) + 1], |row| {
                row.get::<_, i64>(0)
            })
            .map_err(|_| EngineError::Storage)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|_| EngineError::Storage)?;
        for cursor in cursors {
            take_work(aggregate_checked, max_work_units)?;
            let cursor_u64 = u64::try_from(cursor).map_err(|_| EngineError::Storage)?;
            let required = crate::projection_generation::dense_member_kind_at(
                connection,
                cursor_u64,
                effective_at_epoch_s,
            )?
            .is_some();
            if owner_retention_at(
                connection,
                class,
                cursor,
                effective_at_epoch_s,
                RetentionPurpose::Dense { required },
            )? == OwnerRetention::RequiredMembership
            {
                let revision = revision_for_owner(connection, class, cursor)?;
                expected_dense.insert(cursor_u64, revision);
            }
        }
    }
    for (cursor, revision) in &expected_dense {
        if crate::projection_generation::physical_member_completion_at(
            connection,
            *cursor,
            effective_at_epoch_s,
            crate::ProjectionRuntimeStateV1::Absent,
        )
        .is_err()
        {
            let cursor_i64 = i64::try_from(*cursor).map_err(|_| EngineError::Storage)?;
            let (terminal, sidecar, physical): (bool, bool, bool) = connection
                .query_row(
                    "SELECT \
                       EXISTS(SELECT 1 FROM _fathomdb_projection_terminal WHERE write_cursor=?1),\
                       EXISTS(SELECT 1 FROM _fathomdb_vector_rows WHERE write_cursor=?1),\
                       EXISTS(SELECT 1 FROM vector_default WHERE rowid=?1)",
                    [cursor_i64],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .map_err(|_| EngineError::Storage)?;
            let present = u8::from(terminal) + u8::from(sidecar) + u8::from(physical);
            let mut item = finding(
                if present == 1 || present == 2 {
                    DataPlaneIntegrityFindingCodeV1::DenseProjectionPartial
                } else {
                    DataPlaneIntegrityFindingCodeV1::DenseProjectionIdentityMismatch
                },
                DataPlaneIntegritySeverityV1::Error,
            );
            item.write_cursor = Some(*cursor);
            item.artifact_revision_ids = revision.iter().cloned().collect();
            push_finding(findings, item, max_findings)?;
        }
    }
    let physical_dense = physical_dense_candidates(connection, max_work_units, aggregate_checked)?;
    for cursor in physical_dense.valid {
        if !expected_dense.contains_key(&cursor) {
            let cursor_i64 = i64::try_from(cursor).map_err(|_| EngineError::Storage)?;
            let (node_exists, edge_exists): (bool, bool) = connection
                .query_row(
                    "SELECT \
                       EXISTS(SELECT 1 FROM canonical_nodes WHERE write_cursor=?1), \
                       EXISTS(SELECT 1 FROM canonical_edges WHERE write_cursor=?1)",
                    [cursor_i64],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .map_err(|_| EngineError::Storage)?;
            let artifact_class = if node_exists {
                Some("node")
            } else if edge_exists {
                Some("edge")
            } else {
                None
            };
            let expected_kind = if artifact_class == Some("edge") {
                "edge_fact".to_string()
            } else if artifact_class == Some("node") {
                connection
                    .query_row(
                        "SELECT kind FROM canonical_nodes WHERE write_cursor=?1",
                        [cursor_i64],
                        |row| Ok(text_value(row, 0)),
                    )
                    .map_err(|_| EngineError::Storage)?
                    .unwrap_or_default()
            } else {
                String::new()
            };
            let retention = if let Some(class) = artifact_class {
                owner_retention_at(
                    connection,
                    class,
                    cursor_i64,
                    effective_at_epoch_s,
                    RetentionPurpose::Dense { required: false },
                )?
            } else {
                OwnerRetention::CanonicalOwnerMissing
            };
            let shape = matches!(
                retention,
                OwnerRetention::RequiredMembership | OwnerRetention::LegitimateRetained
            )
            .then(|| dense_physical_shape(connection, cursor_i64, &expected_kind))
            .transpose()?;
            if retention == OwnerRetention::LegitimateRetained
                && shape.as_ref().is_some_and(|shape| shape.complete || shape.terminal_only)
            {
                continue;
            }
            let mut item = finding(
                match retention {
                    OwnerRetention::CanonicalOwnerMissing => {
                        DataPlaneIntegrityFindingCodeV1::DenseProjectionOwnerMissing
                    }
                    OwnerRetention::GovernedPruningResidue => {
                        DataPlaneIntegrityFindingCodeV1::DenseProjectionOutsideMembership
                    }
                    OwnerRetention::RequiredMembership | OwnerRetention::LegitimateRetained => {
                        if shape.as_ref().is_some_and(|shape| {
                            shape.present_count == 1 || shape.present_count == 2
                        }) {
                            DataPlaneIntegrityFindingCodeV1::DenseProjectionPartial
                        } else {
                            DataPlaneIntegrityFindingCodeV1::DenseProjectionIdentityMismatch
                        }
                    }
                },
                match retention {
                    OwnerRetention::CanonicalOwnerMissing
                    | OwnerRetention::GovernedPruningResidue => {
                        DataPlaneIntegritySeverityV1::Critical
                    }
                    OwnerRetention::RequiredMembership | OwnerRetention::LegitimateRetained => {
                        DataPlaneIntegritySeverityV1::Error
                    }
                },
            );
            item.write_cursor = Some(cursor);
            if let Some(class) = artifact_class {
                let revision = revision_for_owner(connection, class, cursor_i64)?;
                item.artifact_revision_ids = revision.into_iter().collect();
            }
            push_finding(findings, item, max_findings)?;
        }
    }
    for _ in 0..physical_dense.invalid {
        let item = finding(
            DataPlaneIntegrityFindingCodeV1::DenseProjectionIdentityMismatch,
            DataPlaneIntegritySeverityV1::Error,
        );
        push_finding(findings, item, max_findings)?;
    }

    let mut expected_attributes = BTreeMap::new();
    let mut expected_properties = BTreeMap::new();
    for (name, stored) in &registry_snapshot {
        if !stored.wants_eav() {
            continue;
        }
        for (cursor, revision, value) in expected_attribute_members(
            connection,
            name,
            stored,
            effective_at_epoch_s,
            max_work_units,
            aggregate_checked,
        )? {
            expected_attributes.insert((name.clone(), cursor), (value, revision));
        }
    }
    for (name, stored) in &registry_snapshot {
        if !stored.wants_property_fts() {
            continue;
        }
        for (cursor, revision, value) in expected_attribute_members(
            connection,
            name,
            stored,
            effective_at_epoch_s,
            max_work_units,
            aggregate_checked,
        )? {
            expected_properties.insert((name.clone(), cursor), (value, revision));
        }
    }
    for (physical_query, missing_code, expected_members) in [
        (
            ATTRIBUTE_MEMBER_QUERY,
            DataPlaneIntegrityFindingCodeV1::CanonicalAttributeMissing,
            &expected_attributes,
        ),
        (
            PROPERTY_MEMBER_QUERY,
            DataPlaneIntegrityFindingCodeV1::PropertyFtsMissing,
            &expected_properties,
        ),
    ] {
        let remaining = max_work_units.saturating_sub(*aggregate_checked);
        let mut physical_statement =
            connection.prepare(physical_query).map_err(|_| EngineError::Storage)?;
        let physical = physical_statement
            .query_map(rusqlite::params![FIRST_SQLITE_ROWID, i64::from(remaining) + 1], |row| {
                Ok((
                    integer_value(row, 0),
                    integer_value(row, 1),
                    text_value(row, 2),
                    text_value(row, 3),
                ))
            })
            .map_err(|_| EngineError::Storage)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|_| EngineError::Storage)?;
        let mut physical_members = PhysicalAttributeMembers::new();
        let mut invalid_physical = Vec::new();
        for (rowid, cursor, name, value) in physical {
            take_work(aggregate_checked, max_work_units)?;
            match (rowid, cursor, name) {
                (Some(rowid), Some(cursor), Some(name)) if cursor >= 0 => {
                    physical_members.entry((name, cursor)).or_default().push((rowid, value));
                }
                (Some(rowid), _, _) => invalid_physical.push(rowid),
                (None, _, _) => invalid_physical.push(i64::MAX),
            }
        }
        for ((name, cursor), (expected_value, revision)) in expected_members {
            match physical_members.get(&((*name).clone(), *cursor)) {
                None => {
                    let mut item = finding(missing_code, DataPlaneIntegritySeverityV1::Critical);
                    item.write_cursor = u64::try_from(*cursor).ok();
                    item.artifact_revision_ids = revision.iter().cloned().collect();
                    push_finding(findings, item, max_findings)?;
                }
                Some(values)
                    if values.len() != 1
                        || values[0].1.as_deref() != Some(expected_value.as_str()) =>
                {
                    let mut item = finding(
                        DataPlaneIntegrityFindingCodeV1::SearchProjectionIdentityMismatch,
                        DataPlaneIntegritySeverityV1::Error,
                    );
                    item.write_cursor = u64::try_from(*cursor).ok();
                    item.artifact_revision_ids = revision.iter().cloned().collect();
                    push_finding(findings, item, max_findings)?;
                }
                Some(_) => {}
            }
        }
        let mut residue = physical_members
            .iter()
            .filter(|(key, _)| !expected_members.contains_key(*key))
            .flat_map(|((name, cursor), rows)| {
                rows.iter().map(move |(rowid, _)| (*rowid, name.clone(), *cursor))
            })
            .collect::<Vec<_>>();
        residue.sort_unstable_by_key(|entry| entry.0);
        for (_, _name, cursor) in residue {
            let owner_exists: bool = connection
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM canonical_nodes WHERE write_cursor=?1)",
                    [cursor],
                    |row| row.get(0),
                )
                .map_err(|_| EngineError::Storage)?;
            let mut item = finding(
                if owner_exists {
                    DataPlaneIntegrityFindingCodeV1::SearchProjectionOutsideMembership
                } else {
                    DataPlaneIntegrityFindingCodeV1::SearchProjectionOwnerMissing
                },
                DataPlaneIntegritySeverityV1::Critical,
            );
            item.write_cursor = u64::try_from(cursor).ok();
            if owner_exists {
                let revision = revision_for_owner(connection, "node", cursor)?;
                item.artifact_revision_ids = revision.into_iter().collect();
            }
            push_finding(findings, item, max_findings)?;
        }
        invalid_physical.sort_unstable();
        for _ in invalid_physical {
            let item = finding(
                DataPlaneIntegrityFindingCodeV1::SearchProjectionIdentityMismatch,
                DataPlaneIntegritySeverityV1::Error,
            );
            push_finding(findings, item, max_findings)?;
        }
    }
    Ok(*aggregate_checked - start)
}

#[cfg(feature = "operator")]
fn projection_generation_findings(
    connection: &Connection,
    effective_at_epoch_s: i64,
    _observed_write_boundary: u64,
    max_work_units: u32,
    max_findings: u32,
    aggregate_checked: &mut u32,
    findings: &mut Vec<DataPlaneIntegrityFindingV1>,
) -> Result<(u32, String), EngineError> {
    let start = *aggregate_checked;
    take_work(aggregate_checked, max_work_units)?;
    let current_record = connection
        .query_row(CURRENT_GENERATION_QUERY, rusqlite::params![0_i64, 2_i64], |row| {
            Ok((
                text_value(row, 0),
                integer_value(row, 1),
                text_value(row, 2),
                nullable_integer_value(row, 3),
                integer_value(row, 4),
                text_value(row, 5),
            ))
        })
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let current = current_record.as_ref().and_then(|record| record.0.clone());
    let mut valid =
        current_record.as_ref().is_some_and(|(id, schema, role, retired, transition, digest)| {
            id.as_deref().is_some_and(valid_generation_id)
                && *schema == Some(1)
                && role.as_deref() == Some("serving")
                && *retired == NullableValue::Null
                && transition.is_some_and(|value| value >= 0)
                && digest.as_ref().is_some_and(|digest| {
                    digest.len() == 64
                        && digest
                            .bytes()
                            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
                })
        });
    if current_record.is_some() {
        take_work(aggregate_checked, max_work_units)?;
    }
    if valid {
        valid = crate::projection_generation::current_generation_id(connection).is_ok();
    }
    if !valid {
        let mut item = finding(
            DataPlaneIntegrityFindingCodeV1::ProjectionGenerationCorrupt,
            DataPlaneIntegritySeverityV1::Critical,
        );
        item.projection_generation_id = current.clone().filter(|id| valid_generation_id(id));
        push_finding(findings, item, max_findings)?;
    } else {
        let generation_id = current.as_ref().expect("validated current generation");
        let physical_dense =
            physical_dense_candidates(connection, max_work_units, aggregate_checked)?;
        let invalid_dense = physical_dense.invalid;
        let candidates = physical_dense.valid;
        for cursor_u64 in candidates {
            let cursor = i64::try_from(cursor_u64).map_err(|_| EngineError::Storage)?;
            let expected = crate::projection_generation::dense_member_kind_at(
                connection,
                cursor_u64,
                effective_at_epoch_s,
            )?;
            let (terminal, sidecar, vector, node_exists, edge_exists): (
                bool,
                bool,
                bool,
                bool,
                bool,
            ) = connection
                .query_row(
                    "SELECT \
                       EXISTS(SELECT 1 FROM _fathomdb_projection_terminal WHERE write_cursor=?1), \
                       EXISTS(SELECT 1 FROM _fathomdb_vector_rows WHERE write_cursor=?1), \
                       EXISTS(SELECT 1 FROM vector_default WHERE rowid=?1), \
                       EXISTS(SELECT 1 FROM canonical_nodes WHERE write_cursor=?1), \
                       EXISTS(SELECT 1 FROM canonical_edges WHERE write_cursor=?1)",
                    [cursor],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
                )
                .map_err(|_| EngineError::Storage)?;
            let physical = terminal || sidecar || vector;
            if expected.is_none() && !physical {
                continue;
            }
            let artifact_class = if node_exists {
                Some("node")
            } else if edge_exists {
                Some("edge")
            } else {
                None
            };
            if expected.is_none() {
                let retention = if let Some(class) = artifact_class {
                    owner_retention_at(
                        connection,
                        class,
                        cursor,
                        effective_at_epoch_s,
                        RetentionPurpose::Dense { required: false },
                    )?
                } else {
                    OwnerRetention::CanonicalOwnerMissing
                };
                let expected_kind = if artifact_class == Some("edge") {
                    Some("edge_fact".to_string())
                } else if artifact_class == Some("node") {
                    connection
                        .query_row(
                            "SELECT kind FROM canonical_nodes WHERE write_cursor=?1",
                            [cursor],
                            |row| Ok(text_value(row, 0)),
                        )
                        .map_err(|_| EngineError::Storage)?
                } else {
                    None
                };
                let accepted_retained = retention == OwnerRetention::LegitimateRetained
                    && expected_kind
                        .as_deref()
                        .map(|kind| dense_physical_shape(connection, cursor, kind))
                        .transpose()?
                        .is_some_and(|shape| shape.complete || shape.terminal_only);
                if accepted_retained {
                    continue;
                }
            }
            let member_valid = expected.is_some()
                && crate::projection_generation::physical_member_completion_at(
                    connection,
                    cursor_u64,
                    effective_at_epoch_s,
                    crate::ProjectionRuntimeStateV1::Absent,
                )
                .is_ok();
            if !member_valid {
                let revision = revision_for_cursor(connection, cursor)?;
                let mut item = finding(
                    DataPlaneIntegrityFindingCodeV1::ProjectionMemberCorrupt,
                    DataPlaneIntegritySeverityV1::Error,
                );
                item.projection_generation_id = Some(generation_id.clone());
                item.artifact_revision_ids = revision.into_iter().collect();
                item.write_cursor = Some(cursor_u64);
                push_finding(findings, item, max_findings)?;
            }
        }
        for _ in 0..invalid_dense {
            let mut item = finding(
                DataPlaneIntegrityFindingCodeV1::ProjectionMemberCorrupt,
                DataPlaneIntegritySeverityV1::Error,
            );
            item.projection_generation_id = Some(generation_id.clone());
            push_finding(findings, item, max_findings)?;
        }
    }
    Ok((*aggregate_checked - start, current.unwrap_or_default()))
}

#[cfg(feature = "operator")]
fn mutation_readiness_findings(
    connection: &Connection,
    effective_at_epoch_s: i64,
    max_work_units: u32,
    max_findings: u32,
    aggregate_checked: &mut u32,
    findings: &mut Vec<DataPlaneIntegrityFindingV1>,
) -> Result<u32, EngineError> {
    let start = *aggregate_checked;
    let remaining = max_work_units.saturating_sub(*aggregate_checked);
    let mut statement =
        connection.prepare(RECEIPT_GUARD_QUERY).map_err(|_| EngineError::Storage)?;
    let guarded = statement
        .query_map(rusqlite::params![FIRST_SQLITE_ROWID, i64::from(remaining) + 1], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, bool>(1)?,
                row.get::<_, bool>(2)?,
                row.get::<_, bool>(3)?,
                row.get::<_, bool>(4)?,
                row.get::<_, bool>(5)?,
                row.get::<_, bool>(6)?,
                row.get::<_, bool>(7)?,
                row.get::<_, Option<i64>>(8)?,
            ))
        })
        .map_err(|_| EngineError::Storage)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|_| EngineError::Storage)?;
    for (
        rowid,
        operation_id_ok,
        schema_ok,
        count_ok,
        outcome_ok,
        boundary_ok,
        json_ok,
        gen_ok,
        pending_count,
    ) in guarded
    {
        take_work(aggregate_checked, max_work_units)?;
        if !(operation_id_ok
            && schema_ok
            && count_ok
            && outcome_ok
            && boundary_ok
            && json_ok
            && gen_ok)
        {
            let mut item = finding(
                DataPlaneIntegrityFindingCodeV1::MutationReceiptCorrupt,
                DataPlaneIntegritySeverityV1::Error,
            );
            if operation_id_ok {
                item.operation_id = connection
                    .query_row(
                        "SELECT operation_id FROM _fathomdb_actuation_receipts WHERE rowid=?1",
                        [rowid],
                        |row| row.get(0),
                    )
                    .optional()
                    .map_err(|_| EngineError::Storage)?;
            }
            push_finding(findings, item, max_findings)?;
            continue;
        }
        let pending_count =
            pending_count.and_then(|value| u32::try_from(value).ok()).ok_or_else(bound_error)?;
        if pending_count > max_work_units.saturating_sub(*aggregate_checked) {
            return Err(bound_error());
        }
        let (operation_id, operations_count, outcome, boundary, json, generation): (
            String,
            Option<i64>,
            String,
            Option<i64>,
            String,
            Option<String>,
        ) = connection
            .query_row(
                "SELECT operation_id,operations_count,outcome,resulting_write_boundary,\
                        pending_projection_write_cursors_json,projection_generation_id \
                 FROM _fathomdb_actuation_receipts WHERE rowid=?1",
                [rowid],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )
            .map_err(|_| EngineError::Storage)?;
        let values: Vec<String> = match serde_json::from_str(&json) {
            Ok(values) => values,
            Err(_) => {
                let mut item = finding(
                    DataPlaneIntegrityFindingCodeV1::MutationReceiptCorrupt,
                    DataPlaneIntegritySeverityV1::Error,
                );
                item.operation_id = Some(operation_id);
                push_finding(findings, item, max_findings)?;
                continue;
            }
        };
        let cursors = values.iter().map(|value| canonical_u64(value)).collect::<Option<Vec<_>>>();
        let coherent = operations_count.is_some_and(|value| (1..=128).contains(&value))
            || (outcome == "erased" && operations_count.is_none());
        let coherent = coherent
            && cursors.as_ref().is_some_and(|items| {
                items.windows(2).all(|pair| pair[0] < pair[1])
                    && items.iter().all(|cursor| *cursor > 0)
                    && operations_count.is_none_or(|count| items.len() <= count as usize)
            })
            && if matches!(outcome.as_str(), "refused" | "erased") {
                values.is_empty() && boundary.is_none() && generation.is_none()
            } else {
                boundary.is_some_and(|value| {
                    value >= 0
                        && cursors.as_ref().and_then(|items| items.last()).is_none_or(|cursor| {
                            u64::try_from(value).ok().is_some_and(|v| v >= *cursor)
                        })
                })
            };
        let max_pending = cursors.as_ref().and_then(|items| items.last()).copied();
        let generation_coherent = match (max_pending, generation.as_deref()) {
            (None, None) => true,
            (None, Some(_)) => false,
            (Some(max_pending), None) => connection
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM _fathomdb_projection_generations \
                     WHERE origin='legacy_unverified' AND transition_boundary>=?1)",
                    [i64::try_from(max_pending).map_err(|_| EngineError::Storage)?],
                    |row| row.get(0),
                )
                .map_err(|_| EngineError::Storage)?,
            (Some(_), Some(generation_id)) if valid_generation_id(generation_id) => connection
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM _fathomdb_projection_generations \
                     WHERE generation_id=?1)",
                    [generation_id],
                    |row| row.get(0),
                )
                .map_err(|_| EngineError::Storage)?,
            (Some(_), Some(_)) => false,
        };
        let coherent = coherent && generation_coherent;
        if !coherent {
            let mut item = finding(
                DataPlaneIntegrityFindingCodeV1::MutationReceiptCorrupt,
                DataPlaneIntegritySeverityV1::Error,
            );
            item.operation_id = Some(operation_id);
            push_finding(findings, item, max_findings)?;
            continue;
        }
        for cursor in cursors.unwrap_or_default() {
            take_work(aggregate_checked, max_work_units)?;
            if generation.is_none() {
                let mut item = finding(
                    DataPlaneIntegrityFindingCodeV1::MutationReadinessUnavailable,
                    DataPlaneIntegritySeverityV1::Error,
                );
                item.operation_id = Some(operation_id.clone());
                item.write_cursor = Some(cursor);
                push_finding(findings, item, max_findings)?;
                continue;
            }
            let owner_exists: bool = connection
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM _fathomdb_artifact_revisions \
                     WHERE write_cursor=?1)",
                    [i64::try_from(cursor).map_err(|_| EngineError::Storage)?],
                    |row| row.get(0),
                )
                .map_err(|_| EngineError::Storage)?;
            if !owner_exists {
                let mut item = finding(
                    DataPlaneIntegrityFindingCodeV1::MutationReceiptCorrupt,
                    DataPlaneIntegritySeverityV1::Error,
                );
                item.operation_id = Some(operation_id.clone());
                item.projection_generation_id = generation.clone();
                item.write_cursor = Some(cursor);
                push_finding(findings, item, max_findings)?;
            } else {
                let expected_generation = crate::ProjectionGenerationId::new(
                    generation.as_ref().expect("validated nonempty generation").clone(),
                )
                .map_err(|_| EngineError::Storage)?;
                match crate::projection_generation::receipt_point_completion_at(
                    connection,
                    &operation_id,
                    cursor,
                    &expected_generation,
                    effective_at_epoch_s,
                    crate::ProjectionRuntimeStateV1::Absent,
                ) {
                    Err(EngineError::ProjectionGeneration(error))
                        if error.reason
                            == crate::ProjectionGenerationErrorReason::ProjectionGenerationUnavailable =>
                    {
                        let mut item = finding(
                            DataPlaneIntegrityFindingCodeV1::MutationReadinessUnavailable,
                            DataPlaneIntegritySeverityV1::Error,
                        );
                        item.operation_id = Some(operation_id.clone());
                        item.projection_generation_id = generation.clone();
                        item.write_cursor = Some(cursor);
                        push_finding(findings, item, max_findings)?;
                    }
                    Err(EngineError::ProjectionGeneration(error))
                        if error.reason
                            == crate::ProjectionGenerationErrorReason::ProjectionGenerationCorrupt =>
                    {
                        let mut item = finding(
                            DataPlaneIntegrityFindingCodeV1::MutationReadinessCorrupt,
                            DataPlaneIntegritySeverityV1::Error,
                        );
                        item.operation_id = Some(operation_id.clone());
                        item.projection_generation_id = generation.clone();
                        item.write_cursor = Some(cursor);
                        push_finding(findings, item, max_findings)?;
                    }
                    Err(EngineError::ProjectionGeneration(_)) => {
                        let mut item = finding(
                            DataPlaneIntegrityFindingCodeV1::MutationReceiptCorrupt,
                            DataPlaneIntegritySeverityV1::Error,
                        );
                        item.operation_id = Some(operation_id.clone());
                        item.projection_generation_id = generation.clone();
                        item.write_cursor = Some(cursor);
                        push_finding(findings, item, max_findings)?;
                    }
                    Err(_) => return Err(DataPlaneIntegrityErrorV1::new(
                        DataPlaneIntegrityErrorReasonV1::IntegrityCorrupt,
                        "",
                    )
                    .into()),
                    Ok(_) => {}
                }
            }
        }
    }
    Ok(*aggregate_checked - start)
}

#[cfg(feature = "operator")]
pub(crate) fn execute(
    connection: &mut Connection,
    mut request: DataPlaneIntegrityRequestV1,
) -> Result<DataPlaneIntegrityResultV1, EngineError> {
    if request.schema_version != SCHEMA_VERSION {
        return Err(DataPlaneIntegrityErrorV1::new(
            DataPlaneIntegrityErrorReasonV1::UnsupportedSchemaVersion,
            "/schemaVersion",
        )
        .into());
    }
    if request.checks.is_empty() {
        return Err(DataPlaneIntegrityErrorV1::new(
            DataPlaneIntegrityErrorReasonV1::ChecksEmpty,
            "/checks",
        )
        .into());
    }
    let mut seen = std::collections::BTreeSet::new();
    for (index, check) in request.checks.iter().copied().enumerate() {
        if !seen.insert(check) {
            return Err(DataPlaneIntegrityErrorV1::new(
                DataPlaneIntegrityErrorReasonV1::DuplicateCheck,
                format!("/checks/{index}"),
            )
            .into());
        }
    }
    if !(1..=MAX_WORK_UNITS).contains(&request.max_work_units) {
        return Err(DataPlaneIntegrityErrorV1::new(
            DataPlaneIntegrityErrorReasonV1::IntegrityLimitInvalid,
            "/maxWorkUnits",
        )
        .into());
    }
    if !(1..=MAX_FINDINGS).contains(&request.max_findings) {
        return Err(DataPlaneIntegrityErrorV1::new(
            DataPlaneIntegrityErrorReasonV1::IntegrityLimitInvalid,
            "/maxFindings",
        )
        .into());
    }
    request.checks.sort_unstable();
    let transaction = connection.transaction().map_err(|_| EngineError::Storage)?;
    let effective_at_epoch_s = current_epoch_seconds();
    let observed_write_boundary = load_next_cursor(&transaction);
    let dependency_generation = integrity_dependency_generation(&transaction)?.unwrap_or(0);
    let mut projection_generation_id = transaction
        .query_row(
            "SELECT generation_id FROM _fathomdb_projection_generation_current WHERE singleton=1",
            [],
            |row| Ok(text_value(row, 0).filter(|value| valid_generation_id(value))),
        )
        .optional()
        .map_err(|_| EngineError::Storage)?
        .flatten()
        .unwrap_or_default();

    let mut checked_count = 0u32;
    let mut check_counts = Vec::with_capacity(request.checks.len());
    let mut findings = Vec::new();
    for check in request.checks {
        let finding_start = findings.len();
        let count = match check {
            DataPlaneIntegrityCheckV1::DependencyChain => dependency_findings(
                &transaction,
                request.max_work_units,
                request.max_findings,
                &mut checked_count,
                &mut findings,
            )?,
            DataPlaneIntegrityCheckV1::ActiveSearchableOrphans => active_projection_findings(
                &transaction,
                effective_at_epoch_s,
                request.max_work_units,
                request.max_findings,
                &mut checked_count,
                &mut findings,
            )?,
            DataPlaneIntegrityCheckV1::ProjectionGeneration => {
                let (count, observed) = projection_generation_findings(
                    &transaction,
                    effective_at_epoch_s,
                    observed_write_boundary,
                    request.max_work_units,
                    request.max_findings,
                    &mut checked_count,
                    &mut findings,
                )?;
                projection_generation_id = observed;
                count
            }
            DataPlaneIntegrityCheckV1::MutationReadiness => mutation_readiness_findings(
                &transaction,
                effective_at_epoch_s,
                request.max_work_units,
                request.max_findings,
                &mut checked_count,
                &mut findings,
            )?,
        };
        check_counts.push(DataPlaneIntegrityCheckCountV1 {
            schema_version: SCHEMA_VERSION,
            check,
            checked_count: count,
            finding_count: u32::try_from(findings.len() - finding_start)
                .map_err(|_| bound_error())?,
        });
    }
    transaction.commit().map_err(|_| EngineError::Storage)?;
    Ok(DataPlaneIntegrityResultV1 {
        schema_version: SCHEMA_VERSION,
        read_boundary: DataPlaneIntegrityBoundaryV1 {
            schema_version: SCHEMA_VERSION,
            effective_at_epoch_s,
            observed_write_boundary,
            dependency_generation,
            projection_generation_id,
        },
        check_counts,
        checked_count,
        findings,
        complete: true,
    })
}
