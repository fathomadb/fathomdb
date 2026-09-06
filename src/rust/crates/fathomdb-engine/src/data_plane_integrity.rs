use std::fmt::{Display, Formatter};

#[cfg(feature = "operator")]
use std::collections::{BTreeMap, BTreeSet};

#[cfg(feature = "operator")]
use rusqlite::{Connection, OptionalExtension};

#[cfg(feature = "operator")]
use sha2::{Digest, Sha256};

#[cfg(feature = "operator")]
use crate::{current_epoch_seconds, load_dependency_generation, load_next_cursor, EngineError};

const SCHEMA_VERSION: u32 = 1;
const MAX_WORK_UNITS: u32 = 10_000;
const MAX_FINDINGS: u32 = 100;

#[cfg(feature = "operator")]
type StoredSourceLink =
    (i64, String, String, String, String, Option<i64>, Option<i64>, String, String);

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
        if checks.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(DataPlaneIntegrityErrorV1::new(
                DataPlaneIntegrityErrorReasonV1::DuplicateCheck,
                "/checks",
            ));
        }
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
    pub reason: DataPlaneIntegrityErrorReasonV1,
    pub field_path: String,
}

impl DataPlaneIntegrityErrorV1 {
    pub(crate) fn new(reason: DataPlaneIntegrityErrorReasonV1, path: impl Into<String>) -> Self {
        Self { reason, field_path: path.into() }
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
    finding: DataPlaneIntegrityFindingV1,
    max_findings: u32,
) -> Result<(), EngineError> {
    if findings.len() >= max_findings as usize {
        return Err(bound_error());
    }
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
fn dependency_findings(
    connection: &Connection,
    max_work_units: u32,
    max_findings: u32,
    aggregate_checked: &mut u32,
    findings: &mut Vec<DataPlaneIntegrityFindingV1>,
) -> Result<u32, EngineError> {
    let start = *aggregate_checked;
    take_work(aggregate_checked, max_work_units)?;
    let generation = load_dependency_generation(connection)?;
    let remaining = max_work_units.saturating_sub(*aggregate_checked);
    let sql =
        "SELECT schema_version,dependency_id,derived_revision_id,registered_dependency_generation \
               FROM _fathomdb_source_dependencies ORDER BY dependency_id LIMIT ?1";
    let mut statement = connection.prepare(sql).map_err(|_| EngineError::Storage)?;
    let mut rows = statement.query([i64::from(remaining) + 1]).map_err(|_| EngineError::Storage)?;
    while let Some(row) = rows.next().map_err(|_| EngineError::Storage)? {
        take_work(aggregate_checked, max_work_units)?;
        let schema: i64 = row.get(0).map_err(|_| EngineError::Storage)?;
        let dependency_id: String = row.get(1).map_err(|_| EngineError::Storage)?;
        let derived_revision: String = row.get(2).map_err(|_| EngineError::Storage)?;
        let registered_generation: i64 = row.get(3).map_err(|_| EngineError::Storage)?;
        let mut item = if schema != 1
            || !crate::valid_caller_identity(&dependency_id)
            || registered_generation <= 0
        {
            Some(finding(
                DataPlaneIntegrityFindingCodeV1::DependencyRowInvalid,
                DataPlaneIntegritySeverityV1::Error,
            ))
        } else {
            None
        };
        let derived_owner: Option<(String, String, String, i64)> = connection
            .query_row(
                "SELECT artifact_class,artifact_role,completeness,write_cursor \
                 FROM _fathomdb_artifact_revisions WHERE revision_id=?1",
                [&derived_revision],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()
            .map_err(|_| EngineError::Storage)?;
        if item.is_none() && derived_owner.is_none() {
            item = Some(finding(
                DataPlaneIntegrityFindingCodeV1::DependencyDerivedOwnerMissing,
                DataPlaneIntegritySeverityV1::Critical,
            ));
        }
        if item.is_none() {
            let (class, role, completeness, cursor) = derived_owner.as_ref().unwrap();
            let owner_exists: bool = connection
                .query_row(
                    if class == "node" {
                        "SELECT EXISTS(SELECT 1 FROM canonical_nodes WHERE write_cursor=?1)"
                    } else {
                        "SELECT EXISTS(SELECT 1 FROM canonical_edges WHERE write_cursor=?1)"
                    },
                    [cursor],
                    |row| row.get(0),
                )
                .map_err(|_| EngineError::Storage)?;
            if !matches!(class.as_str(), "node" | "edge") || !owner_exists {
                item = Some(finding(
                    DataPlaneIntegrityFindingCodeV1::DependencyDerivedOwnerMissing,
                    DataPlaneIntegritySeverityV1::Critical,
                ));
            } else if role != "derived_semantic" || completeness != "complete" {
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
                        Ok((
                            row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?,
                            row.get(5)?, row.get(6)?, row.get(7)?, row.get(8)?,
                        ))
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
            let (
                link_schema,
                source_id,
                version_id,
                source_revision,
                locator,
                start,
                end,
                algo,
                digest,
            ) = link.as_ref().unwrap();
            let link_valid = *link_schema == 1
                && crate::valid_caller_identity(source_id)
                && crate::valid_caller_identity(version_id)
                && crate::valid_caller_identity(source_revision)
                && ((*locator == "whole_body" && start.is_none() && end.is_none())
                    || (*locator == "utf8_bytes"
                        && start.is_some_and(|value| value >= 0)
                        && end.is_some_and(|value| value >= 0)
                        && start < end))
                && algo == "sha256"
                && digest.len() == 64
                && digest
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
                && source_revision != &derived_revision;
            if !link_valid {
                item = Some(finding(
                    DataPlaneIntegrityFindingCodeV1::DependencySourceLinkMismatch,
                    DataPlaneIntegritySeverityV1::Critical,
                ));
            } else {
                let source_owner: Option<(String, String, String, i64)> = connection
                    .query_row(
                        "SELECT artifact_class,artifact_role,completeness,write_cursor \
                         FROM _fathomdb_artifact_revisions WHERE revision_id=?1",
                        [source_revision],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                    )
                    .optional()
                    .map_err(|_| EngineError::Storage)?;
                if let Some((class, role, completeness, cursor)) = source_owner {
                    let canonical_node: Option<(String, String)> = connection
                        .query_row(
                            "SELECT source_id,body FROM canonical_nodes WHERE write_cursor=?1",
                            [cursor],
                            |row| Ok((row.get(0)?, row.get(1)?)),
                        )
                        .optional()
                        .map_err(|_| EngineError::Storage)?;
                    if class != "node" || canonical_node.is_none() {
                        item = Some(finding(
                            DataPlaneIntegrityFindingCodeV1::DependencySourceOwnerMissing,
                            DataPlaneIntegritySeverityV1::Critical,
                        ));
                    } else if role != "canonical_source" || completeness != "complete" {
                        item = Some(finding(
                            DataPlaneIntegrityFindingCodeV1::DependencySourceRoleInvalid,
                            DataPlaneIntegritySeverityV1::Error,
                        ));
                    } else {
                        let (canonical_source_id, canonical_body) =
                            canonical_node.ok_or(EngineError::Storage)?;
                        let selected = match (locator.as_str(), *start, *end) {
                            ("whole_body", None, None) => Some(canonical_body.as_bytes()),
                            ("utf8_bytes", Some(start), Some(end)) => usize::try_from(start)
                                .ok()
                                .zip(usize::try_from(end).ok())
                                .and_then(|(start, end)| {
                                    (canonical_body.is_char_boundary(start)
                                        && canonical_body.is_char_boundary(end))
                                    .then(|| canonical_body.as_bytes().get(start..end))
                                    .flatten()
                                }),
                            _ => None,
                        };
                        let selected_digest = selected.map(|bytes| {
                            Sha256::digest(bytes)
                                .iter()
                                .map(|byte| format!("{byte:02x}"))
                                .collect::<String>()
                        });
                        if canonical_source_id != *source_id
                            || selected_digest.as_deref() != Some(digest)
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
                            let self_link: Option<(String, String, String, String)> = connection
                                .query_row(
                                    "SELECT source_id,source_version_id,hash_algorithm,hash_digest \
                                     FROM _fathomdb_source_links \
                                     WHERE artifact_revision_id=?1 AND source_revision_id=?1 \
                                     AND schema_version=1 \
                                     AND locator_kind='whole_body' AND start_byte IS NULL \
                                     AND end_byte IS NULL",
                                    [source_revision],
                                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                                )
                                .optional()
                                .map_err(|_| EngineError::Storage)?;
                            let canonical_digest = Sha256::digest(canonical_body.as_bytes())
                                .iter()
                                .map(|byte| format!("{byte:02x}"))
                                .collect::<String>();
                            if !self_link.is_some_and(
                                |(self_source, self_version, algorithm, self_digest)| {
                                    self_source == *source_id
                                        && self_version == *version_id
                                        && algorithm == "sha256"
                                        && self_digest == canonical_digest
                                },
                            ) {
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
            }
        }
        if item.is_none()
            && u64::try_from(registered_generation).ok().is_none_or(|value| value > generation)
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
            if crate::valid_caller_identity(&derived_revision) {
                item.artifact_revision_ids.push(derived_revision);
            }
            push_finding(findings, item, max_findings)?;
        }
    }
    Ok(*aggregate_checked - start)
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

    for (table, code) in [
        ("search_index", DataPlaneIntegrityFindingCodeV1::NodeBodyFtsMissing),
        ("search_index_v2", DataPlaneIntegrityFindingCodeV1::NodeBodyFtsV2Missing),
    ] {
        let remaining = max_work_units.saturating_sub(*aggregate_checked);
        let mut expected = connection
            .prepare(
                "SELECT n.write_cursor,r.revision_id,n.body,n.kind FROM canonical_nodes n \
                 LEFT JOIN _fathomdb_artifact_revisions r \
                   ON r.artifact_class='node' AND r.write_cursor=n.write_cursor \
                 ORDER BY n.write_cursor LIMIT ?1",
            )
            .map_err(|_| EngineError::Storage)?;
        let entries = expected
            .query_map([i64::from(remaining) + 1], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(|_| EngineError::Storage)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|_| EngineError::Storage)?;
        let mut expected_by_cursor = BTreeMap::new();
        for (cursor, revision, body, kind) in entries {
            take_work(aggregate_checked, max_work_units)?;
            expected_by_cursor.insert(cursor, (revision, body, kind));
        }
        let remaining = max_work_units.saturating_sub(*aggregate_checked);
        let sql =
            format!("SELECT rowid,write_cursor,body,kind FROM {table} ORDER BY rowid LIMIT ?1");
        let mut physical_statement = connection.prepare(&sql).map_err(|_| EngineError::Storage)?;
        let physical = physical_statement
            .query_map([i64::from(remaining) + 1], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(|_| EngineError::Storage)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|_| EngineError::Storage)?;
        let mut physical_by_cursor: BTreeMap<i64, Vec<(String, String)>> = BTreeMap::new();
        for (_rowid, cursor, body, kind) in physical {
            take_work(aggregate_checked, max_work_units)?;
            physical_by_cursor.entry(cursor).or_default().push((body, kind));
        }
        for (cursor, (revision, body, kind)) in &expected_by_cursor {
            let candidates = physical_by_cursor.get(cursor);
            if candidates.is_none() {
                let mut item = finding(code, DataPlaneIntegritySeverityV1::Critical);
                item.write_cursor = u64::try_from(*cursor).ok();
                item.artifact_revision_ids = revision.iter().cloned().collect();
                push_finding(findings, item, max_findings)?;
            } else if candidates
                .is_none_or(|rows| rows.len() != 1 || rows[0].0 != *body || rows[0].1 != *kind)
            {
                let mut item = finding(
                    DataPlaneIntegrityFindingCodeV1::SearchProjectionIdentityMismatch,
                    DataPlaneIntegritySeverityV1::Error,
                );
                item.write_cursor = u64::try_from(*cursor).ok();
                item.artifact_revision_ids = revision.iter().cloned().collect();
                push_finding(findings, item, max_findings)?;
            }
        }
        for cursor in physical_by_cursor.keys() {
            if !expected_by_cursor.contains_key(cursor) {
                let mut item = finding(
                    DataPlaneIntegrityFindingCodeV1::SearchProjectionOwnerMissing,
                    DataPlaneIntegritySeverityV1::Critical,
                );
                item.write_cursor = u64::try_from(*cursor).ok();
                push_finding(findings, item, max_findings)?;
            }
        }
    }

    let remaining = max_work_units.saturating_sub(*aggregate_checked);
    let mut edge_statement = connection
        .prepare(
            "SELECT e.write_cursor,r.revision_id,e.body,e.kind FROM canonical_edges e \
             LEFT JOIN _fathomdb_artifact_revisions r \
               ON r.artifact_class='edge' AND r.write_cursor=e.write_cursor \
             WHERE e.body IS NOT NULL ORDER BY e.write_cursor LIMIT ?1",
        )
        .map_err(|_| EngineError::Storage)?;
    let edge_entries = edge_statement
        .query_map([i64::from(remaining) + 1], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(|_| EngineError::Storage)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|_| EngineError::Storage)?;
    let mut expected_edges = BTreeMap::new();
    for (cursor, revision, body, kind) in edge_entries {
        take_work(aggregate_checked, max_work_units)?;
        expected_edges.insert(cursor, (revision, body, kind));
    }
    let remaining = max_work_units.saturating_sub(*aggregate_checked);
    let mut edge_physical_statement = connection
        .prepare(
            "SELECT rowid,write_cursor,body,kind FROM search_index_edges \
             ORDER BY rowid LIMIT ?1",
        )
        .map_err(|_| EngineError::Storage)?;
    let edge_physical = edge_physical_statement
        .query_map([i64::from(remaining) + 1], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(|_| EngineError::Storage)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|_| EngineError::Storage)?;
    let mut physical_edges: BTreeMap<i64, Vec<(String, String)>> = BTreeMap::new();
    for (_rowid, cursor, body, kind) in edge_physical {
        take_work(aggregate_checked, max_work_units)?;
        physical_edges.entry(cursor).or_default().push((body, kind));
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
            Some(rows) if rows.len() != 1 || rows[0].0 != *body || rows[0].1 != *kind => {
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
    for cursor in physical_edges.keys() {
        if !expected_edges.contains_key(cursor) {
            let mut item = finding(
                DataPlaneIntegrityFindingCodeV1::SearchProjectionOwnerMissing,
                DataPlaneIntegritySeverityV1::Critical,
            );
            item.write_cursor = u64::try_from(*cursor).ok();
            push_finding(findings, item, max_findings)?;
        }
    }

    let mut expected_dense = BTreeMap::new();
    for (table, class) in [("canonical_nodes", "node"), ("canonical_edges", "edge")] {
        let remaining = max_work_units.saturating_sub(*aggregate_checked);
        let sql = format!("SELECT write_cursor FROM {table} ORDER BY write_cursor LIMIT ?1");
        let mut statement = connection.prepare(&sql).map_err(|_| EngineError::Storage)?;
        let cursors = statement
            .query_map([i64::from(remaining) + 1], |row| row.get::<_, i64>(0))
            .map_err(|_| EngineError::Storage)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|_| EngineError::Storage)?;
        for cursor in cursors {
            let cursor_u64 = u64::try_from(cursor).map_err(|_| EngineError::Storage)?;
            if crate::projection_generation::dense_member_kind_at(
                connection,
                cursor_u64,
                effective_at_epoch_s,
            )?
            .is_some()
            {
                take_work(aggregate_checked, max_work_units)?;
                let revision: Option<String> = connection
                    .query_row(
                        "SELECT revision_id FROM _fathomdb_artifact_revisions \
                         WHERE artifact_class=?1 AND write_cursor=?2",
                        rusqlite::params![class, cursor],
                        |row| row.get(0),
                    )
                    .optional()
                    .map_err(|_| EngineError::Storage)?;
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
    let mut physical_dense = BTreeSet::new();
    for (table, cursor_column) in
        [("_fathomdb_vector_rows", "write_cursor"), ("vector_default", "rowid")]
    {
        let remaining = max_work_units.saturating_sub(*aggregate_checked);
        let sql = format!("SELECT {cursor_column} FROM {table} ORDER BY {cursor_column} LIMIT ?1");
        let mut statement = connection.prepare(&sql).map_err(|_| EngineError::Storage)?;
        let cursors = statement
            .query_map([i64::from(remaining) + 1], |row| row.get::<_, i64>(0))
            .map_err(|_| EngineError::Storage)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|_| EngineError::Storage)?;
        for cursor in cursors {
            physical_dense.insert(u64::try_from(cursor).map_err(|_| EngineError::Storage)?);
        }
    }
    for cursor in physical_dense {
        take_work(aggregate_checked, max_work_units)?;
        if !expected_dense.contains_key(&cursor) {
            let cursor_i64 = i64::try_from(cursor).map_err(|_| EngineError::Storage)?;
            let owner_exists: bool = connection
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM canonical_nodes WHERE write_cursor=?1) \
                     OR EXISTS(SELECT 1 FROM canonical_edges WHERE write_cursor=?1)",
                    [cursor_i64],
                    |row| row.get(0),
                )
                .map_err(|_| EngineError::Storage)?;
            let mut item = finding(
                if owner_exists {
                    DataPlaneIntegrityFindingCodeV1::DenseProjectionOutsideMembership
                } else {
                    DataPlaneIntegrityFindingCodeV1::DenseProjectionOwnerMissing
                },
                DataPlaneIntegritySeverityV1::Critical,
            );
            item.write_cursor = Some(cursor);
            push_finding(findings, item, max_findings)?;
        }
    }

    let mut expected_attributes = BTreeMap::new();
    let mut expected_properties = BTreeMap::new();
    for (name, stored) in &registry_snapshot {
        if !stored.wants_eav() {
            continue;
        }
        let remaining = max_work_units.saturating_sub(*aggregate_checked);
        let mut owner_statement = connection
            .prepare(
                "SELECT n.write_cursor,r.revision_id,n.body FROM canonical_nodes n \
                 LEFT JOIN _fathomdb_artifact_revisions r \
                   ON r.artifact_class='node' AND r.write_cursor=n.write_cursor \
                 WHERE n.state='active' AND n.superseded_at IS NULL \
                 ORDER BY n.write_cursor LIMIT ?1",
            )
            .map_err(|_| EngineError::Storage)?;
        let owners = owner_statement
            .query_map([i64::from(remaining) + 1], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|_| EngineError::Storage)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|_| EngineError::Storage)?;
        for (cursor, revision, body) in owners {
            let Some(value) = crate::extract_scalar_attribute(connection, &body, name, stored)
                .map_err(|_| EngineError::Storage)?
            else {
                continue;
            };
            take_work(aggregate_checked, max_work_units)?;
            expected_attributes.insert((cursor, name.clone()), (value.clone(), revision.clone()));
            if stored.wants_property_fts() {
                take_work(aggregate_checked, max_work_units)?;
                expected_properties.insert((cursor, name.clone()), (value, revision));
            }
        }
    }
    for (table, missing_code, expected_members) in [
        (
            "canonical_attributes",
            DataPlaneIntegrityFindingCodeV1::CanonicalAttributeMissing,
            &expected_attributes,
        ),
        (
            "property_search_index",
            DataPlaneIntegrityFindingCodeV1::PropertyFtsMissing,
            &expected_properties,
        ),
    ] {
        let remaining = max_work_units.saturating_sub(*aggregate_checked);
        let sql = format!(
            "SELECT rowid,write_cursor,attr_name,attr_value FROM {table} ORDER BY rowid LIMIT ?1"
        );
        let mut physical_statement = connection.prepare(&sql).map_err(|_| EngineError::Storage)?;
        let physical = physical_statement
            .query_map([i64::from(remaining) + 1], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(|_| EngineError::Storage)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|_| EngineError::Storage)?;
        let mut physical_members: BTreeMap<(i64, String), Vec<String>> = BTreeMap::new();
        for (_rowid, cursor, name, value) in physical {
            take_work(aggregate_checked, max_work_units)?;
            physical_members.entry((cursor, name)).or_default().push(value);
        }
        for ((cursor, name), (expected_value, revision)) in expected_members {
            match physical_members.get(&(*cursor, name.clone())) {
                None => {
                    let mut item = finding(missing_code, DataPlaneIntegritySeverityV1::Critical);
                    item.write_cursor = u64::try_from(*cursor).ok();
                    item.artifact_revision_ids = revision.iter().cloned().collect();
                    push_finding(findings, item, max_findings)?;
                }
                Some(values) if values.len() != 1 || values[0] != *expected_value => {
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
        for (cursor, name) in physical_members.keys() {
            if !expected_members.contains_key(&(*cursor, name.clone())) {
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
                item.write_cursor = u64::try_from(*cursor).ok();
                push_finding(findings, item, max_findings)?;
            }
        }
    }
    Ok(*aggregate_checked - start)
}

#[cfg(feature = "operator")]
fn projection_generation_findings(
    connection: &Connection,
    effective_at_epoch_s: i64,
    observed_write_boundary: u64,
    max_work_units: u32,
    max_findings: u32,
    aggregate_checked: &mut u32,
    findings: &mut Vec<DataPlaneIntegrityFindingV1>,
) -> Result<(u32, String), EngineError> {
    let start = *aggregate_checked;
    take_work(aggregate_checked, max_work_units)?;
    let current: Option<String> = connection
        .query_row(
            "SELECT generation_id FROM _fathomdb_projection_generation_current WHERE singleton=1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let mut valid = current.as_deref().is_some_and(valid_generation_id);
    if let Some(id) = current.as_ref() {
        take_work(aggregate_checked, max_work_units)?;
        let generation_valid: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM _fathomdb_projection_generations \
                 WHERE generation_id=?1 AND schema_version=1 AND role='serving' \
                 AND retired_boundary IS NULL AND transition_boundary>=0 \
                 AND length(declaration_sha256)=64 \
                 AND declaration_sha256 NOT GLOB '*[^0-9a-f]*')",
                [id],
                |row| row.get(0),
            )
            .map_err(|_| EngineError::Storage)?;
        valid &= generation_valid;
        if valid
            && crate::projection_generation::status_in_snapshot(
                connection,
                crate::ProjectionRuntimeStateV1::Absent,
                effective_at_epoch_s,
                observed_write_boundary,
            )
            .is_err()
        {
            valid = false;
        }
    }
    if !valid {
        let mut item = finding(
            DataPlaneIntegrityFindingCodeV1::ProjectionGenerationCorrupt,
            DataPlaneIntegritySeverityV1::Critical,
        );
        item.projection_generation_id = current.clone().filter(|id| valid_generation_id(id));
        push_finding(findings, item, max_findings)?;
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
    let mut statement = connection
        .prepare(
            "SELECT rowid,operation_id, \
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
                            AND projection_generation_id GLOB 'pgen1:[0-9a-f]*')) \
             FROM _fathomdb_actuation_receipts ORDER BY operation_id LIMIT ?1",
        )
        .map_err(|_| EngineError::Storage)?;
    let guarded = statement
        .query_map([i64::from(remaining) + 1], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, bool>(2)?,
                row.get::<_, bool>(3)?,
                row.get::<_, bool>(4)?,
                row.get::<_, bool>(5)?,
                row.get::<_, bool>(6)?,
                row.get::<_, bool>(7)?,
            ))
        })
        .map_err(|_| EngineError::Storage)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|_| EngineError::Storage)?;
    for (rowid, operation_id, schema_ok, count_ok, outcome_ok, boundary_ok, json_ok, gen_ok) in
        guarded
    {
        take_work(aggregate_checked, max_work_units)?;
        if !(schema_ok
            && count_ok
            && outcome_ok
            && boundary_ok
            && json_ok
            && gen_ok
            && crate::valid_caller_identity(&operation_id))
        {
            let mut item = finding(
                DataPlaneIntegrityFindingCodeV1::MutationReceiptCorrupt,
                DataPlaneIntegritySeverityV1::Error,
            );
            if crate::valid_caller_identity(&operation_id) {
                item.operation_id = Some(operation_id);
            }
            push_finding(findings, item, max_findings)?;
            continue;
        }
        let (operations_count, outcome, boundary, json, generation): (
            Option<i64>,
            String,
            Option<i64>,
            String,
            Option<String>,
        ) = connection
            .query_row(
                "SELECT operations_count,outcome,resulting_write_boundary,\
                        pending_projection_write_cursors_json,projection_generation_id \
                 FROM _fathomdb_actuation_receipts WHERE rowid=?1",
                [rowid],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            )
            .map_err(|_| EngineError::Storage)?;
        let values: Vec<String> = serde_json::from_str(&json).map_err(|_| EngineError::Storage)?;
        let cursors = values.iter().map(|value| canonical_u64(value)).collect::<Option<Vec<_>>>();
        let coherent = operations_count.is_some_and(|value| (1..=128).contains(&value))
            || (outcome == "erased" && operations_count.is_none());
        let coherent = coherent
            && cursors.as_ref().is_some_and(|items| {
                items.windows(2).all(|pair| pair[0] < pair[1])
                    && items.iter().all(|cursor| *cursor > 0)
                    && operations_count.is_none_or(|count| items.len() <= count as usize)
            })
            && if values.is_empty() {
                generation.is_none()
            } else {
                generation.as_deref().is_some_and(valid_generation_id)
            }
            && if matches!(outcome.as_str(), "refused" | "erased") {
                values.is_empty() && boundary.is_none()
            } else {
                boundary.is_some_and(|value| value >= 0)
            };
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
                match crate::projection_generation::physical_member_completion_at(
                    connection,
                    cursor,
                    effective_at_epoch_s,
                    crate::ProjectionRuntimeStateV1::Absent,
                ) {
                    Err(_) => {
                        let mut item = finding(
                            DataPlaneIntegrityFindingCodeV1::MutationReadinessCorrupt,
                            DataPlaneIntegritySeverityV1::Error,
                        );
                        item.operation_id = Some(operation_id.clone());
                        item.projection_generation_id = generation.clone();
                        item.write_cursor = Some(cursor);
                        push_finding(findings, item, max_findings)?;
                    }
                    Ok(None) => {
                        let mut item = finding(
                            DataPlaneIntegrityFindingCodeV1::MutationReadinessUnavailable,
                            DataPlaneIntegritySeverityV1::Error,
                        );
                        item.operation_id = Some(operation_id.clone());
                        item.projection_generation_id = generation.clone();
                        item.write_cursor = Some(cursor);
                        push_finding(findings, item, max_findings)?;
                    }
                    Ok(Some(_)) => {}
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
    let dependency_generation = load_dependency_generation(&transaction)?;
    let mut projection_generation_id: String = transaction
        .query_row(
            "SELECT generation_id FROM _fathomdb_projection_generation_current WHERE singleton=1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| EngineError::Storage)?
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
