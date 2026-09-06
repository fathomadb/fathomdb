use std::fmt::{Display, Formatter};

#[cfg(feature = "operator")]
use rusqlite::{Connection, OptionalExtension};

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
                    let node_exists: bool = connection
                        .query_row(
                            "SELECT EXISTS(SELECT 1 FROM canonical_nodes WHERE write_cursor=?1)",
                            [cursor],
                            |row| row.get(0),
                        )
                        .map_err(|_| EngineError::Storage)?;
                    if class != "node" || !node_exists {
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
                        let version_matches: bool = connection
                            .query_row(
                                "SELECT EXISTS(SELECT 1 FROM _fathomdb_source_versions \
                                 WHERE source_revision_id=?1 AND source_id=?2 AND source_version_id=?3 \
                                 AND schema_version=1)",
                                rusqlite::params![source_revision, source_id, version_id],
                                |row| row.get(0),
                            )
                            .map_err(|_| EngineError::Storage)?;
                        if !version_matches {
                            item = Some(finding(
                                DataPlaneIntegrityFindingCodeV1::DependencySourceVersionMismatch,
                                DataPlaneIntegritySeverityV1::Critical,
                            ));
                        } else {
                            let self_matches: bool = connection
                                .query_row(
                                    "SELECT EXISTS(SELECT 1 FROM _fathomdb_source_links \
                                     WHERE artifact_revision_id=?1 AND source_revision_id=?1 \
                                     AND source_id=?2 AND source_version_id=?3 AND schema_version=1 \
                                     AND locator_kind='whole_body' AND start_byte IS NULL \
                                     AND end_byte IS NULL AND hash_algorithm='sha256')",
                                    rusqlite::params![source_revision, source_id, version_id],
                                    |row| row.get(0),
                                )
                                .map_err(|_| EngineError::Storage)?;
                            if !self_matches {
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
    let mut rows = registry.query([i64::from(remaining) + 1]).map_err(|_| EngineError::Storage)?;
    while rows.next().map_err(|_| EngineError::Storage)?.is_some() {
        take_work(aggregate_checked, max_work_units)?;
    }
    drop(rows);
    drop(registry);

    for (table, code) in [
        ("search_index", DataPlaneIntegrityFindingCodeV1::NodeBodyFtsMissing),
        ("search_index_v2", DataPlaneIntegrityFindingCodeV1::NodeBodyFtsV2Missing),
    ] {
        let remaining = max_work_units.saturating_sub(*aggregate_checked);
        let mut expected = connection
            .prepare(
                "SELECT n.write_cursor,r.revision_id FROM canonical_nodes n \
                 LEFT JOIN _fathomdb_artifact_revisions r \
                   ON r.artifact_class='node' AND r.write_cursor=n.write_cursor \
                 ORDER BY n.write_cursor LIMIT ?1",
            )
            .map_err(|_| EngineError::Storage)?;
        let entries = expected
            .query_map([i64::from(remaining) + 1], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, Option<String>>(1)?))
            })
            .map_err(|_| EngineError::Storage)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|_| EngineError::Storage)?;
        for (cursor, revision) in entries {
            take_work(aggregate_checked, max_work_units)?;
            let sql = format!("SELECT count(*) FROM {table} WHERE write_cursor=?1");
            let count: i64 = connection
                .query_row(&sql, [cursor], |row| row.get(0))
                .map_err(|_| EngineError::Storage)?;
            if count == 0 {
                let mut item = finding(code, DataPlaneIntegritySeverityV1::Critical);
                item.write_cursor = u64::try_from(cursor).ok();
                item.artifact_revision_ids = revision.into_iter().collect();
                push_finding(findings, item, max_findings)?;
            } else if count != 1 {
                let mut item = finding(
                    DataPlaneIntegrityFindingCodeV1::SearchProjectionIdentityMismatch,
                    DataPlaneIntegritySeverityV1::Error,
                );
                item.write_cursor = u64::try_from(cursor).ok();
                item.artifact_revision_ids = revision.into_iter().collect();
                push_finding(findings, item, max_findings)?;
            }
        }
    }
    Ok(*aggregate_checked - start)
}

#[cfg(feature = "operator")]
fn projection_generation_findings(
    connection: &Connection,
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
            }
        }
    }
    Ok(*aggregate_checked - start)
}

#[cfg(feature = "operator")]
pub(crate) fn execute(
    connection: &mut Connection,
    request: DataPlaneIntegrityRequestV1,
) -> Result<DataPlaneIntegrityResultV1, EngineError> {
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
                request.max_work_units,
                request.max_findings,
                &mut checked_count,
                &mut findings,
            )?,
            DataPlaneIntegrityCheckV1::ProjectionGeneration => {
                let (count, observed) = projection_generation_findings(
                    &transaction,
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
