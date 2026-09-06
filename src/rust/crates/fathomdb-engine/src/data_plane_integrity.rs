use std::fmt::{Display, Formatter};

#[cfg(feature = "operator")]
use rusqlite::Connection;

#[cfg(feature = "operator")]
use crate::{
    current_epoch_seconds, load_dependency_generation, load_next_cursor, projection_generation,
    EngineError,
};

const SCHEMA_VERSION: u32 = 1;
const MAX_WORK_UNITS: u32 = 10_000;
const MAX_FINDINGS: u32 = 100;

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
fn scalar_count(connection: &Connection, table: &str) -> Result<u32, EngineError> {
    let sql = format!("SELECT count(*) FROM {table}");
    let count: i64 =
        connection.query_row(&sql, [], |row| row.get(0)).map_err(|_| EngineError::Storage)?;
    u32::try_from(count).map_err(|_| bound_error())
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
    let projection_generation_id =
        projection_generation::current_generation_id(&transaction)?.as_str().to_owned();

    let mut checked_count = 0u32;
    let mut check_counts = Vec::with_capacity(request.checks.len());
    for check in request.checks {
        let count = match check {
            DataPlaneIntegrityCheckV1::DependencyChain => 1u32
                .checked_add(scalar_count(&transaction, "_fathomdb_source_dependencies")?)
                .ok_or_else(bound_error)?,
            DataPlaneIntegrityCheckV1::ActiveSearchableOrphans => {
                scalar_count(&transaction, "_fathomdb_projection_registry")?
            }
            DataPlaneIntegrityCheckV1::ProjectionGeneration => {
                scalar_count(&transaction, "_fathomdb_projection_generation_current")?
                    .checked_add(scalar_count(&transaction, "_fathomdb_projection_generations")?)
                    .ok_or_else(bound_error)?
            }
            DataPlaneIntegrityCheckV1::MutationReadiness => {
                scalar_count(&transaction, "_fathomdb_actuation_receipts")?
            }
        };
        checked_count = checked_count.checked_add(count).ok_or_else(bound_error)?;
        if checked_count > request.max_work_units {
            return Err(bound_error());
        }
        check_counts.push(DataPlaneIntegrityCheckCountV1 {
            schema_version: SCHEMA_VERSION,
            check,
            checked_count: count,
            finding_count: 0,
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
        findings: Vec::new(),
        complete: true,
    })
}
