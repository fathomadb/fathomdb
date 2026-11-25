use super::*;
use serde_json::json;

pub(crate) const CLOSURE_SCHEMA_VERSION: u32 = 30;
const CLOSURE_SEQUENCE_KEY: &str = "_fathomdb_closure_sequence";

/// Engine-minted identity of one dependency-closure operation.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ClosureOperationId(String);

impl ClosureOperationId {
    /// Return the opaque Engine-minted identifier.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Closed lookup for one dependency-closure operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClosureLookupV1 {
    pub(crate) schema_version: u32,
    pub(crate) closure_operation_id: ClosureOperationId,
}

impl ClosureLookupV1 {
    /// Validate a schema-version-1 closure lookup.
    ///
    /// # Errors
    ///
    /// Returns `closure_operation_id_invalid` when the supplied ID is not an
    /// Engine-minted closure identifier.
    pub fn new(value: impl Into<String>) -> Result<Self, DependencyClosureError> {
        let value = value.into();
        if !valid_closure_id(&value) {
            return Err(DependencyClosureError::new(
                DependencyClosureErrorReason::ClosureOperationIdInvalid,
                "/closureOperationId",
            ));
        }
        Ok(Self { schema_version: 1, closure_operation_id: ClosureOperationId(value) })
    }
}

/// Root whose lifecycle loss admitted a closure operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClosureRootV1 {
    /// One immutable canonical source revision.
    SourceRevision { source_revision_id: SourceRevisionId },
    /// Every canonical and derived artifact in one source bucket.
    SourceBucket { source_id: SourceId },
}

/// Closed reason a dependency closure was admitted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClosureCauseV1 {
    Superseded,
    SoftDeleted,
    Purged,
    SourceErased,
}

impl ClosureCauseV1 {
    /// Stable lower-snake-case wire spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Superseded => "superseded",
            Self::SoftDeleted => "soft_deleted",
            Self::Purged => "purged",
            Self::SourceErased => "source_erased",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "superseded" => Some(Self::Superseded),
            "soft_deleted" => Some(Self::SoftDeleted),
            "purged" => Some(Self::Purged),
            "source_erased" => Some(Self::SourceErased),
            _ => None,
        }
    }

    fn is_physical(self) -> bool {
        matches!(self, Self::Purged | Self::SourceErased)
    }
}

/// Durable phase of one dependency closure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClosurePhaseV1 {
    Proving,
    AtRestPending,
    Complete,
    Incomplete,
}

impl ClosurePhaseV1 {
    /// Stable lower-snake-case wire spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Proving => "proving",
            Self::AtRestPending => "at_rest_pending",
            Self::Complete => "complete",
            Self::Incomplete => "incomplete",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "proving" => Some(Self::Proving),
            "at_rest_pending" => Some(Self::AtRestPending),
            "complete" => Some(Self::Complete),
            "incomplete" => Some(Self::Incomplete),
            _ => None,
        }
    }
}

/// Scalar zero-proof for a completed dependency closure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClosureProofV1 {
    pub schema_version: u32,
    pub proof_write_boundary: u64,
    pub current_active_dependent_nodes: u64,
    pub current_derived_edges: u64,
    pub view_eligible_dependents: u64,
    pub ownerless_projection_rows: u64,
    pub post_admission_registrations: u64,
    pub remaining_dependency_rows: Option<u64>,
    pub remaining_canonical_rows: Option<u64>,
    pub remaining_projection_rows: Option<u64>,
    pub remaining_receipt_reference_rows: Option<u64>,
}

/// Current durable status of one dependency closure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClosureStatusV1 {
    pub schema_version: u32,
    pub closure_operation_id: ClosureOperationId,
    pub root: ClosureRootV1,
    pub cause: ClosureCauseV1,
    pub phase: ClosurePhaseV1,
    pub effective_at_epoch_s: i64,
    pub admitted_write_boundary: u64,
    pub admitted_dependency_generation: u64,
    pub affected_count: u64,
    pub blocker_code: Option<String>,
    pub proof: Option<ClosureProofV1>,
}

/// Closed reason for a dependency-closure request refusal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DependencyClosureErrorReason {
    UnsupportedSchemaVersion,
    UnknownField,
    ClosureOperationIdInvalid,
}

impl DependencyClosureErrorReason {
    /// Stable lower-snake-case wire spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UnsupportedSchemaVersion => "unsupported_schema_version",
            Self::UnknownField => "unknown_field",
            Self::ClosureOperationIdInvalid => "closure_operation_id_invalid",
        }
    }
}

/// Typed dependency-closure refusal with an RFC 6901 request pointer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DependencyClosureError {
    pub reason: DependencyClosureErrorReason,
    pub field_path: String,
}

impl DependencyClosureError {
    pub(crate) fn new(reason: DependencyClosureErrorReason, field_path: impl Into<String>) -> Self {
        Self { reason, field_path: field_path.into() }
    }
}

impl Display for DependencyClosureError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{} at {}", self.reason.as_str(), self.field_path)
    }
}

impl Error for DependencyClosureError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SoftClosureMode {
    Complete,
    Proving,
}

#[derive(Clone)]
struct DirectDependent {
    revision_id: String,
    artifact_class: String,
    write_cursor: i64,
}

pub(crate) type PhysicalDependencyPlan = Vec<(String, Vec<(String, i64)>)>;

pub(crate) struct PhysicalClosureAdmission<'a> {
    pub scope: &'a str,
    pub root_kind: &'a str,
    pub root_value: &'a str,
    pub retry_verb: &'a str,
    pub retry_argument: &'a str,
    pub cause: ClosureCauseV1,
    pub boundary: u64,
    pub affected_count: usize,
}

pub(crate) struct PhysicalProofScope {
    scope: String,
    cursors: Vec<i64>,
    revisions: Vec<String>,
    source_link_artifact_revisions: Vec<String>,
    source_version_keys: Vec<(String, String)>,
    receipt_refs: Vec<(String, String)>,
}

pub(crate) fn physical_proof_scope(
    connection: &Connection,
    scope: &str,
    cursors: &[i64],
    receipt_refs: &BTreeSet<(String, String)>,
    source_bucket: Option<&str>,
) -> Result<PhysicalProofScope, EngineError> {
    let mut revisions = Vec::new();
    for cursor in cursors {
        let revision = connection
            .query_row(
                "SELECT revision_id FROM _fathomdb_artifact_revisions WHERE scope=?1 AND write_cursor=?2",
                params![scope, cursor],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|_| EngineError::Storage)?;
        if let Some(revision) = revision {
            revisions.push(revision);
        }
    }
    revisions.sort();
    revisions.dedup();
    let mut source_link_artifact_revisions = BTreeSet::new();
    let mut source_version_keys = BTreeSet::new();
    for revision in &revisions {
        let mut statement = connection
            .prepare(
                "SELECT artifact_revision_id FROM _fathomdb_source_links \
                 WHERE scope=?1 AND (artifact_revision_id=?2 OR source_revision_id=?2)",
            )
            .map_err(|_| EngineError::Storage)?;
        let rows = statement
            .query_map(params![scope, revision], |row| row.get::<_, String>(0))
            .map_err(|_| EngineError::Storage)?;
        for row in rows {
            source_link_artifact_revisions.insert(row.map_err(|_| EngineError::Storage)?);
        }
        let mut statement = connection
            .prepare(
                "SELECT source_id,source_version_id FROM _fathomdb_source_versions \
                 WHERE scope=?1 AND source_revision_id=?2",
            )
            .map_err(|_| EngineError::Storage)?;
        let rows = statement
            .query_map(params![scope, revision], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|_| EngineError::Storage)?;
        for row in rows {
            source_version_keys.insert(row.map_err(|_| EngineError::Storage)?);
        }
    }
    if let Some(source_id) = source_bucket {
        let mut statement = connection
            .prepare(
                "SELECT artifact_revision_id FROM _fathomdb_source_links \
                 WHERE scope=?1 AND source_id=?2",
            )
            .map_err(|_| EngineError::Storage)?;
        let rows = statement
            .query_map(params![scope, source_id], |row| row.get::<_, String>(0))
            .map_err(|_| EngineError::Storage)?;
        for row in rows {
            source_link_artifact_revisions.insert(row.map_err(|_| EngineError::Storage)?);
        }
        let mut statement = connection
            .prepare(
                "SELECT source_id,source_version_id FROM _fathomdb_source_versions \
                 WHERE scope=?1 AND source_id=?2",
            )
            .map_err(|_| EngineError::Storage)?;
        let rows = statement
            .query_map(params![scope, source_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|_| EngineError::Storage)?;
        for row in rows {
            source_version_keys.insert(row.map_err(|_| EngineError::Storage)?);
        }
    }
    let mut cursors = cursors.to_vec();
    cursors.sort_unstable();
    cursors.dedup();
    Ok(PhysicalProofScope {
        scope: scope.to_string(),
        cursors,
        revisions,
        source_link_artifact_revisions: source_link_artifact_revisions.into_iter().collect(),
        source_version_keys: source_version_keys.into_iter().collect(),
        receipt_refs: receipt_refs.iter().cloned().collect(),
    })
}

fn proof_semantics_are_valid(proof: &ClosureProofV1, cause: ClosureCauseV1) -> bool {
    let common_zeros = proof.current_active_dependent_nodes == 0
        && proof.current_derived_edges == 0
        && proof.view_eligible_dependents == 0
        && proof.ownerless_projection_rows == 0
        && proof.post_admission_registrations == 0
        && proof.view_eligible_dependents
            == proof.current_active_dependent_nodes.saturating_add(proof.current_derived_edges);
    let physical_zeros = [
        proof.remaining_dependency_rows,
        proof.remaining_canonical_rows,
        proof.remaining_projection_rows,
        proof.remaining_receipt_reference_rows,
    ];
    common_zeros
        && if cause.is_physical() {
            physical_zeros.iter().all(|value| *value == Some(0))
        } else {
            physical_zeros.iter().all(Option::is_none)
        }
}

pub(crate) fn physical_dependents_for_sources(
    connection: &Connection,
    scope: &str,
    source_revisions: &[String],
) -> Result<PhysicalDependencyPlan, EngineError> {
    let mut result = Vec::new();
    for source_revision in source_revisions {
        let dependents = direct_dependents(connection, scope, source_revision)?;
        for dependent in &dependents {
            validate_dependency_chain(
                connection,
                scope,
                source_revision,
                &dependent.revision_id,
                DependencyValidationMode::Persisted,
            )?;
        }
        if !dependents.is_empty() {
            result.push((
                source_revision.clone(),
                dependents
                    .into_iter()
                    .map(|dependent| (dependent.artifact_class, dependent.write_cursor))
                    .collect(),
            ));
        }
    }
    Ok(result)
}

fn physical_proof(boundary: u64) -> ClosureProofV1 {
    ClosureProofV1 {
        schema_version: 1,
        proof_write_boundary: boundary,
        current_active_dependent_nodes: 0,
        current_derived_edges: 0,
        view_eligible_dependents: 0,
        ownerless_projection_rows: 0,
        post_admission_registrations: 0,
        remaining_dependency_rows: Some(0),
        remaining_canonical_rows: Some(0),
        remaining_projection_rows: Some(0),
        remaining_receipt_reference_rows: Some(0),
    }
}

pub(crate) fn measure_physical_closures(
    connection: &Connection,
    ids: &[ClosureOperationId],
    scope: &PhysicalProofScope,
) -> Result<(), EngineError> {
    if ids.is_empty() {
        return Ok(());
    }
    let mut active_nodes = 0_i64;
    let mut current_edges = 0_i64;
    let mut canonical_rows = 0_i64;
    let mut projection_rows = 0_i64;
    for cursor in &scope.cursors {
        let (nodes, active): (i64, i64) = connection
            .query_row(
                "SELECT COUNT(*),COALESCE(SUM(state='active' AND superseded_at IS NULL),0) \
                 FROM canonical_nodes WHERE scope=?1 AND write_cursor=?2",
                params![&scope.scope, cursor],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|_| EngineError::Storage)?;
        let (edges, current): (i64, i64) = connection
            .query_row(
                "SELECT COUNT(*),COALESCE(SUM(superseded_at IS NULL),0) \
                 FROM canonical_edges WHERE scope=?1 AND write_cursor=?2",
                params![&scope.scope, cursor],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|_| EngineError::Storage)?;
        canonical_rows = canonical_rows.saturating_add(nodes).saturating_add(edges);
        active_nodes = active_nodes.saturating_add(active);
        current_edges = current_edges.saturating_add(current);
    }
    let encoded_cursors =
        serde_json::to_string(&scope.cursors).map_err(|_| EngineError::Storage)?;
    for projection in ROW_OWNED_PROJECTIONS {
        let sql = format!(
            "SELECT COUNT(*) FROM {} WHERE {} IN (SELECT value FROM json_each(?1))",
            projection.table, projection.cursor_column
        );
        let count: i64 = connection
            .query_row(&sql, [&encoded_cursors], |row| row.get(0))
            .map_err(|_| EngineError::Storage)?;
        projection_rows = projection_rows.saturating_add(count);
    }
    for revision in &scope.revisions {
        let count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM _fathomdb_artifact_revisions WHERE scope=?1 AND revision_id=?2",
                params![&scope.scope, revision],
                |row| row.get(0),
            )
            .map_err(|_| EngineError::Storage)?;
        canonical_rows = canonical_rows.saturating_add(count);
    }
    for artifact_revision in &scope.source_link_artifact_revisions {
        let count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM _fathomdb_source_links WHERE scope=?1 AND artifact_revision_id=?2",
                params![&scope.scope, artifact_revision],
                |row| row.get(0),
            )
            .map_err(|_| EngineError::Storage)?;
        canonical_rows = canonical_rows.saturating_add(count);
    }
    for (source_id, source_version_id) in &scope.source_version_keys {
        let count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM _fathomdb_source_versions \
                 WHERE scope=?1 AND source_id=?2 AND source_version_id=?3",
                params![&scope.scope, source_id, source_version_id],
                |row| row.get(0),
            )
            .map_err(|_| EngineError::Storage)?;
        canonical_rows = canonical_rows.saturating_add(count);
    }
    let mut dependency_rows = 0_i64;
    for revision in &scope.revisions {
        let count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM _fathomdb_source_dependencies \
                 WHERE scope=?1 AND derived_revision_id=?2",
                params![&scope.scope, revision],
                |row| row.get(0),
            )
            .map_err(|_| EngineError::Storage)?;
        dependency_rows = dependency_rows.saturating_add(count);
    }
    let mut receipt_rows = 0_i64;
    for (kind, value) in &scope.receipt_refs {
        let count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM _fathomdb_actuation_receipt_source_refs \
                 WHERE scope=?1 AND ref_kind=?2 AND ref_value=?3",
                params![&scope.scope, kind, value],
                |row| row.get(0),
            )
            .map_err(|_| EngineError::Storage)?;
        receipt_rows = receipt_rows.saturating_add(count);
    }
    for id in ids {
        let (boundary, admitted_generation): (i64, i64) = connection
            .query_row(
                "SELECT admitted_write_boundary,admitted_dependency_generation \
                 FROM _fathomdb_dependency_closures WHERE scope=?1 AND closure_operation_id=?2",
                params![&scope.scope, id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|_| EngineError::Storage)?;
        let post_admission: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM _fathomdb_source_dependencies \
                 WHERE scope=?1 AND registered_dependency_generation>?2",
                params![&scope.scope, admitted_generation],
                |row| row.get(0),
            )
            .map_err(|_| EngineError::Storage)?;
        let proof = ClosureProofV1 {
            schema_version: 1,
            proof_write_boundary: u64::try_from(boundary).map_err(|_| EngineError::Storage)?,
            current_active_dependent_nodes: u64::try_from(active_nodes)
                .map_err(|_| EngineError::Storage)?,
            current_derived_edges: u64::try_from(current_edges)
                .map_err(|_| EngineError::Storage)?,
            view_eligible_dependents: u64::try_from(active_nodes.saturating_add(current_edges))
                .map_err(|_| EngineError::Storage)?,
            ownerless_projection_rows: u64::try_from(projection_rows)
                .map_err(|_| EngineError::Storage)?,
            post_admission_registrations: u64::try_from(post_admission)
                .map_err(|_| EngineError::Storage)?,
            remaining_dependency_rows: Some(
                u64::try_from(dependency_rows).map_err(|_| EngineError::Storage)?,
            ),
            remaining_canonical_rows: Some(
                u64::try_from(canonical_rows).map_err(|_| EngineError::Storage)?,
            ),
            remaining_projection_rows: Some(
                u64::try_from(projection_rows).map_err(|_| EngineError::Storage)?,
            ),
            remaining_receipt_reference_rows: Some(
                u64::try_from(receipt_rows).map_err(|_| EngineError::Storage)?,
            ),
        };
        let has_residue = proof.current_active_dependent_nodes != 0
            || proof.current_derived_edges != 0
            || proof.view_eligible_dependents != 0
            || proof.ownerless_projection_rows != 0
            || proof.post_admission_registrations != 0
            || proof.remaining_dependency_rows != Some(0)
            || proof.remaining_canonical_rows != Some(0)
            || proof.remaining_projection_rows != Some(0)
            || proof.remaining_receipt_reference_rows != Some(0);
        if has_residue {
            return Err(EngineError::Storage);
        }
        let changed = connection
            .execute(
                "UPDATE _fathomdb_dependency_closures SET proof_json=?2,\
                 structural_proof_write_boundary=?3 WHERE scope=?4 AND closure_operation_id=?1",
                params![id.as_str(), proof_json(&proof)?, boundary, &scope.scope],
            )
            .map_err(|_| EngineError::Storage)?;
        if changed != 1 {
            return Err(EngineError::Storage);
        }
    }
    Ok(())
}

pub(crate) fn record_physical_closure(
    connection: &Connection,
    admission: PhysicalClosureAdmission<'_>,
) -> Result<Option<ClosureOperationId>, EngineError> {
    if !admission.cause.is_physical() || admission.affected_count == 0 {
        return Ok(None);
    }
    let generation = load_dependency_generation(connection)?;
    let effective_at = current_epoch_seconds();
    let (sequence, id, _) = next_identity(
        connection,
        admission.root_kind,
        admission.root_value,
        admission.cause,
        effective_at,
        admission.boundary,
        generation,
    )?;
    let retry = closure_digest(&[
        "fathomdb.dependency-closure-retry.v1\0",
        admission.retry_verb,
        admission.retry_argument,
    ]);
    let proof = physical_proof(admission.boundary);
    connection
        .execute(
            "INSERT INTO _fathomdb_dependency_closures(\
               schema_version,closure_operation_id,root_kind,root_value,cause,\
               effective_at_epoch_s,admitted_write_boundary,admitted_dependency_generation,\
               closure_sequence,retry_fingerprint,phase,affected_count,blocker_code,\
               structural_proof_write_boundary,proof_json,scope\
             ) VALUES(1,?1,?2,?3,?4,?5,?6,?7,?8,?9,'at_rest_pending',?10,NULL,?6,?11,?12)",
            params![
                id.as_str(),
                admission.root_kind,
                admission.root_value,
                admission.cause.as_str(),
                effective_at,
                i64::try_from(admission.boundary).map_err(|_| EngineError::Storage)?,
                i64::try_from(generation).map_err(|_| EngineError::Storage)?,
                i64::try_from(sequence).map_err(|_| EngineError::Storage)?,
                retry,
                i64::try_from(admission.affected_count).map_err(|_| EngineError::Storage)?,
                proof_json(&proof)?,
                admission.scope,
            ],
        )
        .map_err(|_| EngineError::Storage)?;
    store_sequence(connection, sequence)?;
    Ok(Some(id))
}

pub(crate) fn complete_physical_closures_in(
    connection: &Connection,
    ids: &[ClosureOperationId],
) -> Result<(), EngineError> {
    validate_physical_closures(connection, ids)?;
    for id in ids {
        let changed = connection
            .execute(
                "UPDATE _fathomdb_dependency_closures SET phase='complete',blocker_code=NULL \
             WHERE closure_operation_id=?1 AND phase IN ('at_rest_pending','incomplete') \
             AND cause IN ('purged','source_erased')",
                [id.as_str()],
            )
            .map_err(|_| EngineError::Storage)?;
        if changed != 1 {
            return Err(EngineError::Storage);
        }
    }
    Ok(())
}

pub(crate) fn validate_physical_closures(
    connection: &Connection,
    ids: &[ClosureOperationId],
) -> Result<(), EngineError> {
    for id in ids {
        validate_physical_zero(connection, id)?;
    }
    Ok(())
}

#[cfg(feature = "operator")]
pub(crate) struct PendingPhysicalClosure {
    pub(crate) id: ClosureOperationId,
    pub(crate) cause: String,
    pub(crate) phase: String,
    pub(crate) blocker: Option<String>,
    pub(crate) sequence: u64,
}

#[cfg(feature = "operator")]
pub(crate) fn pending_physical_closures(
    connection: &Connection,
) -> Result<Vec<PendingPhysicalClosure>, EngineError> {
    let mut statement = connection
        .prepare(
            "SELECT closure_operation_id,cause,phase,blocker_code,closure_sequence \
             FROM _fathomdb_dependency_closures \
             WHERE cause IN ('purged','source_erased') AND phase!='complete' \
             ORDER BY closure_sequence",
        )
        .map_err(|_| EngineError::Storage)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, i64>(4)?,
            ))
        })
        .map_err(|_| EngineError::Storage)?;
    rows.map(|row| {
        let (id, cause, phase, blocker, sequence) = row.map_err(|_| EngineError::Storage)?;
        if !valid_closure_id(&id) || sequence <= 0 {
            return Err(EngineError::Storage);
        }
        Ok(PendingPhysicalClosure {
            id: ClosureOperationId(id),
            cause,
            phase,
            blocker,
            sequence: sequence as u64,
        })
    })
    .collect()
}

pub(crate) fn mark_physical_incomplete(
    connection: &Connection,
    ids: &[ClosureOperationId],
    blocker_code: &str,
) -> Result<(), EngineError> {
    if ids.is_empty() {
        return Ok(());
    }
    if !["telemetry_redaction", "wal_checkpoint"].contains(&blocker_code) {
        return Err(EngineError::Storage);
    }
    connection.execute_batch("BEGIN IMMEDIATE").map_err(|_| EngineError::Storage)?;
    let result = (|| {
        crate::frozen_read::require_clean_visibility(connection)?;
        for id in ids {
            let changed = connection
                .execute(
                    "UPDATE _fathomdb_dependency_closures \
                     SET phase='incomplete',blocker_code=?2 \
                     WHERE closure_operation_id=?1 \
                       AND phase IN ('at_rest_pending','incomplete') \
                       AND cause IN ('purged','source_erased')",
                    params![id.as_str(), blocker_code],
                )
                .map_err(|_| EngineError::Storage)?;
            if changed != 1 {
                return Err(EngineError::Storage);
            }
        }
        Ok(())
    })();
    match result {
        Ok(()) => connection.execute_batch("COMMIT").map_err(|_| EngineError::Storage),
        Err(error) => {
            let _ = connection.execute_batch("ROLLBACK");
            Err(error)
        }
    }
}

fn validate_physical_zero(
    connection: &Connection,
    id: &ClosureOperationId,
) -> Result<(), EngineError> {
    let row: (String, String, i64, i64, i64, String, String) = connection
        .query_row(
            "SELECT root_kind,root_value,admitted_write_boundary,\
                    admitted_dependency_generation,structural_proof_write_boundary,proof_json,scope \
             FROM _fathomdb_dependency_closures \
             WHERE closure_operation_id=?1 AND phase IN ('at_rest_pending','incomplete') \
               AND cause IN ('purged','source_erased')",
            [id.as_str()],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            },
        )
        .map_err(|_| EngineError::Storage)?;
    let proof = parse_proof(&row.5, row.4)?;
    if proof.current_active_dependent_nodes != 0
        || proof.current_derived_edges != 0
        || proof.view_eligible_dependents != 0
        || proof.ownerless_projection_rows != 0
        || proof.post_admission_registrations != 0
        || proof.remaining_dependency_rows != Some(0)
        || proof.remaining_canonical_rows != Some(0)
        || proof.remaining_projection_rows != Some(0)
        || proof.remaining_receipt_reference_rows != Some(0)
    {
        return Err(EngineError::Storage);
    }
    let current_write_boundary: i64 = connection
        .query_row(
            "SELECT COALESCE(MAX(write_cursor),0) FROM (\
               SELECT write_cursor FROM canonical_nodes \
               UNION ALL SELECT write_cursor FROM canonical_edges \
               UNION ALL SELECT write_cursor FROM operational_mutations \
               UNION ALL SELECT write_cursor FROM operational_state\
             )",
            [],
            |value| value.get(0),
        )
        .map_err(|_| EngineError::Storage)?;
    let admitted_generation = u64::try_from(row.3).map_err(|_| EngineError::Storage)?;
    let safe_boundary = physical_erasure::closure_safe_boundary(
        connection,
        id,
        u64::try_from(row.2).map_err(|_| EngineError::Storage)?,
        admitted_generation.saturating_add(1),
    )?;
    if current_write_boundary > i64::try_from(safe_boundary).map_err(|_| EngineError::Storage)?
        || load_dependency_generation(connection)? != admitted_generation.saturating_add(1)
    {
        return Err(EngineError::Storage);
    }
    let remaining: i64 = match row.0.as_str() {
        "source_revision" => connection
            .query_row(
                "SELECT \
                   (SELECT COUNT(*) FROM _fathomdb_artifact_revisions WHERE scope=?1 AND revision_id=?2) + \
                   (SELECT COUNT(*) FROM _fathomdb_source_links WHERE scope=?1 \
                      AND (artifact_revision_id=?2 OR source_revision_id=?2))",
                params![&row.6, &row.1],
                |value| value.get(0),
            )
            .map_err(|_| EngineError::Storage)?,
        "source_bucket" => connection
            .query_row(
                "SELECT \
                   (SELECT COUNT(*) FROM canonical_nodes WHERE scope=?1 AND source_id=?2) + \
                   (SELECT COUNT(*) FROM canonical_edges WHERE scope=?1 AND source_id=?2) + \
                   (SELECT COUNT(*) FROM _fathomdb_source_links WHERE scope=?1 AND source_id=?2) + \
                   (SELECT COUNT(*) FROM _fathomdb_source_versions WHERE scope=?1 AND source_id=?2)",
                params![&row.6, &row.1],
                |value| value.get(0),
            )
            .map_err(|_| EngineError::Storage)?,
        _ => return Err(EngineError::Storage),
    };
    if remaining != 0 {
        return Err(EngineError::Storage);
    }
    Ok(())
}

pub(crate) fn pending_physical_retry(
    connection: &Connection,
    scope: &str,
    verb: &str,
    argument: &str,
) -> Result<Vec<ClosureOperationId>, EngineError> {
    let fingerprint = closure_digest(&["fathomdb.dependency-closure-retry.v1\0", verb, argument]);
    let mut statement = connection
        .prepare(
            "SELECT closure_operation_id FROM _fathomdb_dependency_closures \
             WHERE scope=?1 AND retry_fingerprint=?2 AND phase!='complete' \
             ORDER BY closure_sequence",
        )
        .map_err(|_| EngineError::Storage)?;
    let rows = statement
        .query_map(params![scope, fingerprint], |row| row.get::<_, String>(0))
        .map_err(|_| EngineError::Storage)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|_| EngineError::Storage)?;
    rows.into_iter()
        .map(|id| {
            if valid_closure_id(&id) {
                Ok(ClosureOperationId(id))
            } else {
                Err(EngineError::Storage)
            }
        })
        .collect()
}

#[cfg(feature = "operator")]
pub(crate) fn pending_physical_retry_any_scope(
    connection: &Connection,
    verb: &str,
    argument: &str,
) -> Result<Vec<ClosureOperationId>, EngineError> {
    let fingerprint = closure_digest(&["fathomdb.dependency-closure-retry.v1\0", verb, argument]);
    let mut statement = connection
        .prepare(
            "SELECT closure_operation_id FROM _fathomdb_dependency_closures \
             WHERE retry_fingerprint=?1 AND phase!='complete' \
             ORDER BY closure_sequence",
        )
        .map_err(|_| EngineError::Storage)?;
    let rows = statement
        .query_map([fingerprint], |row| row.get::<_, String>(0))
        .map_err(|_| EngineError::Storage)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|_| EngineError::Storage)?;
    rows.into_iter()
        .map(|id| {
            if valid_closure_id(&id) {
                Ok(ClosureOperationId(id))
            } else {
                Err(EngineError::Storage)
            }
        })
        .collect()
}

pub(crate) fn maintain_before_writer(connection: &Connection) -> Result<(), EngineError> {
    crate::frozen_read::require_clean_visibility(connection)?;
    guard_no_pending_physical(connection)?;
    let ids = {
        let mut statement = connection
            .prepare(
                "SELECT closure_operation_id FROM _fathomdb_dependency_closures \
                 WHERE phase IN ('proving','incomplete') \
                   AND cause IN ('superseded','soft_deleted') \
                 ORDER BY closure_sequence LIMIT 32",
            )
            .map_err(|_| EngineError::Storage)?;
        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|_| EngineError::Storage)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|_| EngineError::Storage)?;
        rows
    };
    if ids.is_empty() {
        return Ok(());
    }
    connection.execute_batch("BEGIN IMMEDIATE").map_err(|_| EngineError::Storage)?;
    let outcome = (|| {
        crate::frozen_read::require_clean_visibility(connection)?;
        guard_no_pending_physical(connection)?;
        for id in ids {
            if !valid_closure_id(&id) {
                return Err(EngineError::Storage);
            }
            finalize_soft_closure(connection, &ClosureOperationId(id))?;
        }
        Ok(())
    })();
    match outcome {
        Ok(()) => connection.execute_batch("COMMIT").map_err(|_| EngineError::Storage),
        Err(error) => {
            let _ = connection.execute_batch("ROLLBACK");
            Err(error)
        }
    }
}

pub(crate) fn guard_no_pending_physical(connection: &Connection) -> Result<(), EngineError> {
    let physical: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM _fathomdb_dependency_closures \
             WHERE phase!='complete' AND cause IN ('purged','source_erased'))",
            [],
            |row| row.get(0),
        )
        .map_err(|_| EngineError::Storage)?;
    if physical || physical_erasure::active(connection)? {
        return Err(EngineError::ErasureIncomplete {
            stage: "dependency_closure".to_string(),
            detail: "physical cleanup remains owed and this mutation is outside its admitted scope"
                .to_string(),
        });
    }
    Ok(())
}

pub(crate) fn projection_owner_is_eligible_at(
    connection: &Connection,
    cursor: u64,
    effective_at: i64,
) -> Result<bool, EngineError> {
    let cursor = i64::try_from(cursor).map_err(|_| EngineError::Storage)?;
    let source: Option<(String, String)> = connection
        .prepare_cached(
            "SELECT r.scope, l.source_revision_id FROM _fathomdb_artifact_revisions r \
             JOIN _fathomdb_source_dependencies d \
               ON d.scope=r.scope AND d.derived_revision_id=r.revision_id \
             JOIN _fathomdb_source_links l \
               ON l.scope=r.scope AND l.artifact_revision_id=r.revision_id \
             WHERE r.write_cursor=?1",
        )
        .map_err(|_| EngineError::Storage)?
        .query_row([cursor], |row| Ok((row.get(0)?, row.get(1)?)))
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let Some((scope, source)) = source else {
        return Ok(true);
    };
    Ok(!active_barrier_for_source(connection, &scope, &source)?
        && source_revision_is_strictly_eligible(connection, &scope, &source, effective_at)?)
}

pub(crate) fn vector_arm_requires_fallback(
    connection: &Connection,
    has_source_dependencies: bool,
    include_superseded: bool,
    include_inactive: bool,
    include_out_of_window: bool,
    effective_at: i64,
    scope: &crate::scope::ScopePredicate,
) -> rusqlite::Result<bool> {
    #[cfg(any(test, feature = "slice30-observer"))]
    let observation = crate::slice30_observer::span("vector_arm_requires_fallback");
    if !has_source_dependencies {
        #[cfg(any(test, feature = "slice30-observer"))]
        observation.boolean_result(&Ok::<bool, rusqlite::Error>(false), false);
        return Ok(false);
    }
    let mut binds = Vec::new();
    let now_idx = if !include_out_of_window {
        binds.push(rusqlite::types::Value::Integer(effective_at));
        Some(1)
    } else {
        None
    };
    let mut owner_scope_sql = String::new();
    scope.append_sql_values("owner.scope", &mut owner_scope_sql, &mut binds);
    let mut source_predicate = String::new();
    if !include_superseded {
        source_predicate.push_str(" AND source_node.superseded_at IS NULL");
    }
    if !include_inactive {
        source_predicate.push_str(" AND source_node.state='active'");
    }
    if let Some(now_idx) = now_idx {
        source_predicate.push_str(&format!(
            " AND (source_node.valid_from IS NULL OR source_node.valid_from <= ?{now_idx}) \
             AND (source_node.valid_until IS NULL OR source_node.valid_until > ?{now_idx})"
        ));
    }
    let sql = format!(
        "SELECT EXISTS(\
           SELECT 1 FROM _fathomdb_artifact_revisions owner \
           JOIN _fathomdb_source_dependencies dependency \
             ON dependency.scope=owner.scope AND dependency.derived_revision_id=owner.revision_id \
           JOIN _fathomdb_source_links derived_link \
             ON derived_link.scope=owner.scope AND derived_link.artifact_revision_id=owner.revision_id \
           WHERE ({owner_scope_sql}) AND (EXISTS(\
             SELECT 1 FROM _fathomdb_dependency_closures closure \
             LEFT JOIN _fathomdb_source_links source_link \
               ON source_link.scope=owner.scope \
              AND source_link.artifact_revision_id=derived_link.source_revision_id \
             WHERE closure.scope=owner.scope AND closure.phase!='complete' AND (\
               (closure.root_kind='source_revision' \
                 AND closure.root_value=derived_link.source_revision_id) OR \
               (closure.root_kind='source_bucket' \
                 AND closure.root_value=source_link.source_id)\
             )\
           ) OR NOT EXISTS(\
             SELECT 1 FROM _fathomdb_artifact_revisions source_revision \
             JOIN canonical_nodes source_node \
               ON source_node.scope=source_revision.scope \
              AND source_node.write_cursor=source_revision.write_cursor \
             WHERE source_revision.scope=owner.scope \
               AND source_revision.revision_id=derived_link.source_revision_id \
               AND source_revision.artifact_class='node' \
               AND source_revision.artifact_role='canonical_source' \
               AND source_revision.completeness='complete'{source_predicate}\
           ))\
         )"
    );
    let result =
        connection.query_row(&sql, rusqlite::params_from_iter(binds.iter()), |row| row.get(0));
    #[cfg(any(test, feature = "slice30-observer"))]
    observation.boolean_result(&result, true);
    result
}

pub(crate) fn has_source_dependencies(
    connection: &Connection,
    scope: &crate::scope::ScopePredicate,
) -> rusqlite::Result<bool> {
    #[cfg(any(test, feature = "slice30-observer"))]
    let observation = crate::slice30_observer::span("has_source_dependencies");
    let mut binds = Vec::new();
    let mut scope_sql = String::new();
    scope.append_sql_values("scope", &mut scope_sql, &mut binds);
    let sql = format!(
        "SELECT EXISTS(SELECT 1 FROM _fathomdb_source_dependencies WHERE ({scope_sql}) LIMIT 1)"
    );
    let result = crate::prepare_search_statement(connection, &sql).and_then(|mut statement| {
        statement.query_row(rusqlite::params_from_iter(binds.iter()), |row| row.get(0))
    });
    #[cfg(any(test, feature = "slice30-observer"))]
    observation.boolean_result(&result, true);
    result
}

pub(crate) fn all_nodes_directly_eligible(
    connection: &Connection,
    include_superseded: bool,
    include_inactive: bool,
    include_out_of_window: bool,
    effective_at: i64,
    scope: &crate::scope::ScopePredicate,
) -> rusqlite::Result<bool> {
    #[cfg(any(test, feature = "slice30-observer"))]
    let observation = crate::slice30_observer::span("all_nodes_directly_eligible");
    let mut binds = vec![
        rusqlite::types::Value::Integer(effective_at),
        rusqlite::types::Value::Integer(i64::from(include_superseded)),
        rusqlite::types::Value::Integer(i64::from(include_inactive)),
        rusqlite::types::Value::Integer(i64::from(include_out_of_window)),
    ];
    let mut scope_sql = String::new();
    scope.append_sql_values("n.scope", &mut scope_sql, &mut binds);
    let sql = format!(
        "SELECT EXISTS(SELECT 1 FROM canonical_nodes n WHERE ({scope_sql}) AND (\
           (?2=0 AND n.superseded_at IS NOT NULL) OR \
           (?3=0 AND n.state!='active') OR \
           (?4=0 AND ((n.valid_from IS NOT NULL AND n.valid_from>?1) OR \
                      (n.valid_until IS NOT NULL AND n.valid_until<=?1)))) LIMIT 1)"
    );
    let unsafe_row = crate::prepare_search_statement(connection, &sql).and_then(|mut statement| {
        statement.query_row(rusqlite::params_from_iter(binds.iter()), |row| row.get::<_, bool>(0))
    });
    let result = unsafe_row.map(|unsafe_row| !unsafe_row);
    #[cfg(any(test, feature = "slice30-observer"))]
    observation.boolean_result(&result, true);
    result
}

pub(crate) fn read_eligibility_sql(
    alias: &str,
    include_superseded: bool,
    include_inactive: bool,
    include_out_of_window: bool,
    now_idx: usize,
) -> String {
    read_eligibility_sql_for_cursor(
        &format!("{alias}.write_cursor"),
        include_superseded,
        include_inactive,
        include_out_of_window,
        now_idx,
    )
}

fn scope_expression_for_cursor(cursor_expression: &str) -> String {
    match cursor_expression.strip_suffix("write_cursor") {
        Some(prefix) => format!("{prefix}scope"),
        None => {
            debug_assert!(
                false,
                "expected `...write_cursor` expression, got `{cursor_expression}`"
            );
            "scope".to_string()
        }
    }
}

pub(crate) fn read_eligibility_sql_for_cursor(
    cursor_expression: &str,
    include_superseded: bool,
    include_inactive: bool,
    include_out_of_window: bool,
    now_idx: usize,
) -> String {
    let scope_expression = scope_expression_for_cursor(cursor_expression);
    let mut source_predicate = String::new();
    if !include_superseded {
        source_predicate.push_str(" AND source_node.superseded_at IS NULL");
    }
    if !include_inactive {
        source_predicate.push_str(" AND source_node.state='active'");
    }
    if !include_out_of_window {
        source_predicate.push_str(&format!(
            " AND (source_node.valid_from IS NULL OR source_node.valid_from <= ?{now_idx}) \
             AND (source_node.valid_until IS NULL OR source_node.valid_until > ?{now_idx})"
        ));
    }
    format!(
        "{} AND NOT EXISTS(\
           SELECT 1 FROM _fathomdb_artifact_revisions source_owner \
           JOIN _fathomdb_source_dependencies source_dep \
             ON source_dep.scope=source_owner.scope \
            AND source_dep.derived_revision_id=source_owner.revision_id \
           JOIN _fathomdb_source_links derived_link \
             ON derived_link.scope=source_owner.scope \
            AND derived_link.artifact_revision_id=source_owner.revision_id \
           WHERE source_owner.scope={scope_expression} \
             AND source_owner.write_cursor={cursor_expression} \
             AND NOT EXISTS(\
               SELECT 1 FROM _fathomdb_artifact_revisions source_revision \
               JOIN canonical_nodes source_node \
                 ON source_node.scope=source_revision.scope \
                AND source_node.write_cursor=source_revision.write_cursor \
               WHERE source_revision.scope=source_owner.scope \
                 AND source_revision.revision_id=derived_link.source_revision_id \
                 AND source_revision.artifact_class='node' \
                 AND source_revision.artifact_role='canonical_source' \
                 AND source_revision.completeness='complete'{source_predicate}\
             )\
         )",
        read_barrier_sql_for_cursor(cursor_expression)
    )
}

/// Historical derived-artifact read eligibility for `AsOf(as_of_cursor)` graph
/// reconstruction. Enforces active closure barriers (`phase != 'complete'`) and
/// requires every registered canonical source to physically exist at `as_of`,
/// remain active (both the `as_of` revision and its logical head), and satisfy
/// world-time validity unless `include_out_of_window` is set.
pub(crate) fn read_eligibility_sql_historical_for_cursor(
    cursor_expression: &str,
    as_of_idx: usize,
    now_idx: usize,
    include_out_of_window: bool,
) -> String {
    let scope_expression = scope_expression_for_cursor(cursor_expression);
    let mut source_predicate = format!(
        " AND source_node.write_cursor <= ?{as_of_idx} \
         AND (source_node.superseded_at IS NULL OR source_node.superseded_at > ?{as_of_idx}) \
         AND source_node.state='active' \
         AND NOT EXISTS(\
           SELECT 1 FROM canonical_nodes scur \
           WHERE scur.scope=source_node.scope \
             AND scur.logical_id=source_node.logical_id \
             AND scur.superseded_at IS NULL \
             AND scur.state!='active'\
         )"
    );
    if !include_out_of_window {
        source_predicate.push_str(&format!(
            " AND (source_node.valid_from IS NULL OR source_node.valid_from <= ?{now_idx}) \
             AND (source_node.valid_until IS NULL OR source_node.valid_until > ?{now_idx})"
        ));
    }
    format!(
        "{} AND NOT EXISTS(\
           SELECT 1 FROM _fathomdb_artifact_revisions source_owner \
           JOIN _fathomdb_source_dependencies source_dep \
             ON source_dep.scope=source_owner.scope \
            AND source_dep.derived_revision_id=source_owner.revision_id \
           JOIN _fathomdb_source_links derived_link \
             ON derived_link.scope=source_owner.scope \
            AND derived_link.artifact_revision_id=source_owner.revision_id \
           WHERE source_owner.scope={scope_expression} \
             AND source_owner.write_cursor={cursor_expression} \
             AND NOT EXISTS(\
               SELECT 1 FROM _fathomdb_artifact_revisions source_revision \
               JOIN canonical_nodes source_node \
                 ON source_node.scope=source_revision.scope \
                AND source_node.write_cursor=source_revision.write_cursor \
               WHERE source_revision.scope=source_owner.scope \
                 AND source_revision.revision_id=derived_link.source_revision_id \
                 AND source_revision.artifact_class='node' \
                 AND source_revision.artifact_role='canonical_source' \
                 AND source_revision.completeness='complete'{source_predicate}\
             )\
         )",
        read_barrier_sql_for_cursor(cursor_expression)
    )
}

pub(crate) fn read_barrier_sql_for_cursor(cursor_expression: &str) -> String {
    let scope_expression = scope_expression_for_cursor(cursor_expression);
    format!(
        " AND NOT EXISTS(\
           SELECT 1 FROM _fathomdb_artifact_revisions closure_owner \
           JOIN _fathomdb_source_dependencies closure_dep \
             ON closure_dep.scope=closure_owner.scope \
            AND closure_dep.derived_revision_id=closure_owner.revision_id \
           JOIN _fathomdb_source_links closure_link \
             ON closure_link.scope=closure_owner.scope \
            AND closure_link.artifact_revision_id=closure_owner.revision_id \
           JOIN _fathomdb_source_links closure_source \
             ON closure_source.scope=closure_owner.scope \
            AND closure_source.artifact_revision_id=closure_link.source_revision_id \
           JOIN _fathomdb_dependency_closures closure_op \
             ON closure_op.scope=closure_owner.scope \
            AND closure_op.phase!='complete' AND (\
               (closure_op.root_kind='source_revision' \
                 AND closure_op.root_value=closure_link.source_revision_id) OR \
               (closure_op.root_kind='source_bucket' \
                 AND closure_op.root_value=closure_source.source_id)) \
           WHERE closure_owner.scope={scope_expression} \
             AND closure_owner.write_cursor={cursor_expression}\
         )"
    )
}

pub(crate) fn validate_closure_state_on_open(
    connection: &Connection,
    schema_version: u32,
) -> Result<(), EngineOpenError> {
    if schema_version < CLOSURE_SCHEMA_VERSION {
        return Ok(());
    }
    let valid = (|| -> Result<bool, rusqlite::Error> {
        let Some(sequence) = valid_closure_sequence(connection)? else {
            return Ok(false);
        };
        let current_boundary: i64 = connection.query_row(
            "SELECT COALESCE(MAX(write_cursor),0) FROM (\
               SELECT write_cursor FROM canonical_nodes \
               UNION ALL SELECT write_cursor FROM canonical_edges \
               UNION ALL SELECT write_cursor FROM operational_mutations \
               UNION ALL SELECT write_cursor FROM operational_state \
               UNION ALL SELECT admitted_write_boundary AS write_cursor \
                 FROM _fathomdb_dependency_closures\
             )",
            [],
            |row| row.get(0),
        )?;
        let mut statement = connection.prepare(
            "SELECT closure_operation_id,retry_fingerprint,root_kind,root_value,cause,phase,\
                    admitted_write_boundary,admitted_dependency_generation,closure_sequence,\
                    affected_count,blocker_code,structural_proof_write_boundary,proof_json \
             FROM _fathomdb_dependency_closures",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, i64>(7)?,
                row.get::<_, i64>(8)?,
                row.get::<_, i64>(9)?,
                row.get::<_, Option<String>>(10)?,
                row.get::<_, Option<i64>>(11)?,
                row.get::<_, Option<String>>(12)?,
            ))
        })?;
        for row in rows {
            let (
                id,
                fingerprint,
                root_kind,
                root_value,
                cause,
                phase,
                boundary,
                generation,
                row_sequence,
                affected,
                blocker,
                proof_boundary,
                encoded_proof,
            ) = row?;
            let Some(cause) = ClosureCauseV1::parse(&cause) else {
                return Ok(false);
            };
            let Some(phase) = ClosurePhaseV1::parse(&phase) else {
                return Ok(false);
            };
            let root_valid = match root_kind.as_str() {
                "source_revision" => SourceRevisionId::new(root_value).is_ok(),
                "source_bucket" => stored_source_id_is_valid(&root_value),
                _ => false,
            };
            let proof = match (proof_boundary, encoded_proof) {
                (Some(boundary), Some(encoded)) => parse_proof(&encoded, boundary).ok(),
                (None, None) => None,
                _ => return Ok(false),
            };
            let blocker_valid = blocker.as_deref().is_none_or(|value| {
                [
                    "projection_state_unavailable",
                    "proof_unavailable",
                    "telemetry_redaction",
                    "wal_checkpoint",
                ]
                .contains(&value)
            });
            let phase_valid =
                match phase {
                    ClosurePhaseV1::Complete => blocker.is_none() && proof.is_some(),
                    ClosurePhaseV1::Proving => {
                        blocker.is_none() && proof.is_none() && !cause.is_physical()
                    }
                    ClosurePhaseV1::AtRestPending => {
                        blocker.is_none() && proof.is_some() && cause.is_physical()
                    }
                    ClosurePhaseV1::Incomplete => {
                        blocker.is_some() && (proof.is_some() == cause.is_physical())
                    }
                } && proof.as_ref().is_none_or(|proof| proof_semantics_are_valid(proof, cause));
            if !valid_closure_id(&id)
                || !valid_hash(&fingerprint)
                || !root_valid
                || !blocker_valid
                || !phase_valid
                || boundary < 0
                || generation < 0
                || row_sequence <= 0
                || affected <= 0
                || row_sequence as u64 > sequence
                || proof.as_ref().is_some_and(|proof| {
                    proof.proof_write_boundary > current_boundary.max(0) as u64
                })
            {
                return Ok(false);
            }
        }
        Ok(true)
    })()
    .unwrap_or(false);
    if valid {
        return Ok(());
    }
    Err(schema_corruption("_fathomdb_dependency_closures"))
}

fn schema_corruption(table: &'static str) -> EngineOpenError {
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

fn canonical_sequence(value: &str) -> Option<u64> {
    if value.is_empty()
        || (value.len() > 1 && value.starts_with('0'))
        || !value.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    value.parse::<u64>().ok().filter(|value| *value <= i64::MAX as u64)
}

fn valid_closure_sequence(connection: &Connection) -> rusqlite::Result<Option<u64>> {
    let value: String = connection.query_row(
        "SELECT value FROM _fathomdb_open_state WHERE key=?1",
        [CLOSURE_SEQUENCE_KEY],
        |row| row.get(0),
    )?;
    let Some(sequence) = canonical_sequence(&value) else {
        return Ok(None);
    };
    let max_sequence: i64 = connection.query_row(
        "SELECT COALESCE(MAX(closure_sequence),0) FROM _fathomdb_dependency_closures",
        [],
        |row| row.get(0),
    )?;
    if max_sequence < 0 || sequence < max_sequence as u64 {
        return Ok(None);
    }
    Ok(Some(sequence))
}

fn load_sequence(connection: &Connection) -> Result<u64, EngineError> {
    let value: String = connection
        .query_row(
            "SELECT value FROM _fathomdb_open_state WHERE key=?1",
            [CLOSURE_SEQUENCE_KEY],
            |row| row.get(0),
        )
        .map_err(|_| EngineError::Storage)?;
    canonical_sequence(&value).ok_or(EngineError::Storage)
}

fn reserve_sequence(connection: &Connection) -> Result<u64, EngineError> {
    load_sequence(connection)?
        .checked_add(1)
        .filter(|value| *value <= i64::MAX as u64)
        .ok_or(EngineError::Storage)
}

fn store_sequence(connection: &Connection, sequence: u64) -> Result<(), EngineError> {
    let changed = connection
        .execute(
            "UPDATE _fathomdb_open_state SET value=?1 WHERE key=?2",
            params![sequence.to_string(), CLOSURE_SEQUENCE_KEY],
        )
        .map_err(|_| EngineError::Storage)?;
    if changed != 1 {
        return Err(EngineError::Storage);
    }
    Ok(())
}

fn valid_hash(value: &str) -> bool {
    value.len() == 64
        && value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_closure_id(value: &str) -> bool {
    value.strip_prefix("_fdb:c:").is_some_and(valid_hash)
}

fn closure_digest(parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update((part.len() as u64).to_be_bytes());
        hasher.update(part.as_bytes());
    }
    crate::identity::hex_encode(&hasher.finalize())
}

fn next_identity(
    connection: &Connection,
    root_kind: &str,
    root_value: &str,
    cause: ClosureCauseV1,
    effective_at: i64,
    boundary: u64,
    dependency_generation: u64,
) -> Result<(u64, ClosureOperationId, String), EngineError> {
    let sequence = reserve_sequence(connection)?;
    let sequence_text = sequence.to_string();
    let effective_text = effective_at.to_string();
    let boundary_text = boundary.to_string();
    let generation_text = dependency_generation.to_string();
    let id = ClosureOperationId(format!(
        "_fdb:c:{}",
        closure_digest(&[
            "fathomdb.dependency-closure.v1\0",
            &sequence_text,
            root_kind,
            root_value,
            cause.as_str(),
            &effective_text,
            &boundary_text,
            &generation_text,
        ])
    ));
    let retry = closure_digest(&[
        "fathomdb.dependency-closure-retry.v1\0",
        cause.as_str(),
        root_kind,
        root_value,
    ]);
    Ok((sequence, id, retry))
}

pub(crate) fn active_barrier_for_source(
    connection: &Connection,
    scope: &str,
    source_revision_id: &str,
) -> Result<bool, EngineError> {
    connection
        .query_row(
            "SELECT EXISTS(\
               SELECT 1 FROM _fathomdb_dependency_closures c \
               WHERE c.scope=?1 AND c.phase != 'complete' AND (\
                 (c.root_kind='source_revision' AND c.root_value=?2) OR\
                 (c.root_kind='source_bucket' AND c.root_value=(\
                   SELECT source_id FROM _fathomdb_source_links \
                   WHERE scope=?1 AND artifact_revision_id=?2\
                 ))\
               )\
             )",
            params![scope, source_revision_id],
            |row| row.get(0),
        )
        .map_err(|_| EngineError::Storage)
}

pub(crate) fn source_revision_is_strictly_eligible(
    connection: &Connection,
    scope: &str,
    source_revision_id: &str,
    effective_at: i64,
) -> Result<bool, EngineError> {
    connection
        .query_row(
            "SELECT EXISTS(\
               SELECT 1 FROM _fathomdb_artifact_revisions r \
               JOIN canonical_nodes n ON n.scope=r.scope AND n.write_cursor=r.write_cursor \
               WHERE r.scope=?1 AND r.revision_id=?2 AND r.artifact_class='node' \
                 AND r.artifact_role='canonical_source' AND r.completeness='complete' \
                 AND n.superseded_at IS NULL AND n.state='active' \
                 AND (n.valid_from IS NULL OR n.valid_from <= ?3) \
                 AND (n.valid_until IS NULL OR n.valid_until > ?3)\
             )",
            params![scope, source_revision_id, effective_at],
            |row| row.get(0),
        )
        .map_err(|_| EngineError::Storage)
}

pub(crate) fn source_revision_is_eligible_as_of(
    connection: &Connection,
    scope: &str,
    source_revision_id: &str,
    effective_at: i64,
    as_of_cursor: u64,
    include_out_of_window: bool,
) -> Result<bool, EngineError> {
    let as_of_i64 = i64::try_from(as_of_cursor).unwrap_or(i64::MAX);
    if include_out_of_window {
        connection
            .query_row(
                "SELECT EXISTS(\
                   SELECT 1 FROM _fathomdb_artifact_revisions r \
                   JOIN canonical_nodes n ON n.scope=r.scope AND n.write_cursor=r.write_cursor \
                   WHERE r.scope=?1 AND r.revision_id=?2 AND r.artifact_class='node' \
                     AND r.artifact_role='canonical_source' AND r.completeness='complete' \
                     AND n.write_cursor <= ?3 \
                     AND (n.superseded_at IS NULL OR n.superseded_at > ?3) \
                     AND n.state='active' \
                     AND NOT EXISTS(\
                       SELECT 1 FROM canonical_nodes cur \
                       WHERE cur.scope=n.scope AND cur.logical_id=n.logical_id \
                         AND cur.superseded_at IS NULL \
                         AND cur.state!='active'\
                     )\
                 )",
                params![scope, source_revision_id, as_of_i64],
                |row| row.get(0),
            )
            .map_err(|_| EngineError::Storage)
    } else {
        connection
            .query_row(
                "SELECT EXISTS(\
                   SELECT 1 FROM _fathomdb_artifact_revisions r \
                   JOIN canonical_nodes n ON n.scope=r.scope AND n.write_cursor=r.write_cursor \
                   WHERE r.scope=?1 AND r.revision_id=?2 AND r.artifact_class='node' \
                     AND r.artifact_role='canonical_source' AND r.completeness='complete' \
                     AND n.write_cursor <= ?3 \
                     AND (n.superseded_at IS NULL OR n.superseded_at > ?3) \
                     AND n.state='active' \
                     AND NOT EXISTS(\
                       SELECT 1 FROM canonical_nodes cur \
                       WHERE cur.scope=n.scope AND cur.logical_id=n.logical_id \
                         AND cur.superseded_at IS NULL \
                         AND cur.state!='active'\
                     ) \
                     AND (n.valid_from IS NULL OR n.valid_from <= ?4) \
                     AND (n.valid_until IS NULL OR n.valid_until > ?4)\
                 )",
                params![scope, source_revision_id, as_of_i64, effective_at],
                |row| row.get(0),
            )
            .map_err(|_| EngineError::Storage)
    }
}

fn direct_dependents(
    connection: &Connection,
    scope: &str,
    source_revision_id: &str,
) -> Result<Vec<DirectDependent>, EngineError> {
    let mut statement = connection
        .prepare(
            "SELECT d.derived_revision_id,r.artifact_class,r.write_cursor \
             FROM _fathomdb_source_dependencies d \
             JOIN _fathomdb_source_links l ON l.scope=d.scope AND l.artifact_revision_id=d.derived_revision_id \
             JOIN _fathomdb_artifact_revisions r ON r.scope=d.scope AND r.revision_id=d.derived_revision_id \
             WHERE d.scope=?1 AND l.source_revision_id=?2 \
             ORDER BY d.derived_revision_id,d.dependency_id",
        )
        .map_err(|_| EngineError::Storage)?;
    let rows = statement
        .query_map(params![scope, source_revision_id], |row| {
            Ok(DirectDependent {
                revision_id: row.get(0)?,
                artifact_class: row.get(1)?,
                write_cursor: row.get(2)?,
            })
        })
        .map_err(|_| EngineError::Storage)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|_| EngineError::Storage)?;
    Ok(rows)
}

fn apply_soft_effects(
    connection: &Connection,
    scope: &str,
    dependents: &[DirectDependent],
    boundary: u64,
) -> Result<(), EngineError> {
    let boundary = i64::try_from(boundary).map_err(|_| EngineError::Storage)?;
    for dependent in dependents {
        match dependent.artifact_class.as_str() {
            "node" => {
                connection
                    .execute(
                        "UPDATE canonical_nodes SET state='deleted',reason='source_lifecycle' \
                         WHERE scope=?1 AND write_cursor=?2 AND superseded_at IS NULL AND state='active'",
                        params![scope, dependent.write_cursor],
                    )
                    .map_err(|_| EngineError::Storage)?;
                erase_row_projections(connection, dependent.write_cursor)
                    .map_err(|_| EngineError::Storage)?;
            }
            "edge" => {
                connection
                    .execute(
                        "UPDATE canonical_edges SET superseded_at=?1 \
                         WHERE scope=?2 AND write_cursor=?3 AND superseded_at IS NULL",
                        params![boundary, scope, dependent.write_cursor],
                    )
                    .map_err(|_| EngineError::Storage)?;
                erase_row_projections(connection, dependent.write_cursor)
                    .map_err(|_| EngineError::Storage)?;
            }
            _ => return Err(EngineError::Storage),
        }
    }
    Ok(())
}

fn soft_proof(
    connection: &Connection,
    scope: &str,
    source_revision_id: &str,
    admitted_generation: u64,
    boundary: u64,
) -> Result<ClosureProofV1, EngineError> {
    let dependents = direct_dependents(connection, scope, source_revision_id)?;
    let active_nodes: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM _fathomdb_source_dependencies d \
             JOIN _fathomdb_source_links l ON l.scope=d.scope AND l.artifact_revision_id=d.derived_revision_id \
             JOIN _fathomdb_artifact_revisions r ON r.scope=d.scope AND r.revision_id=d.derived_revision_id \
             JOIN canonical_nodes n ON n.scope=r.scope AND n.write_cursor=r.write_cursor \
             WHERE d.scope=?1 AND l.source_revision_id=?2 AND n.superseded_at IS NULL AND n.state='active'",
            params![scope, source_revision_id],
            |row| row.get(0),
        )
        .map_err(|_| EngineError::Storage)?;
    let current_edges: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM _fathomdb_source_dependencies d \
             JOIN _fathomdb_source_links l ON l.scope=d.scope AND l.artifact_revision_id=d.derived_revision_id \
             JOIN _fathomdb_artifact_revisions r ON r.scope=d.scope AND r.revision_id=d.derived_revision_id \
             JOIN canonical_edges e ON e.scope=r.scope AND e.write_cursor=r.write_cursor \
             WHERE d.scope=?1 AND l.source_revision_id=?2 AND e.superseded_at IS NULL",
            params![scope, source_revision_id],
            |row| row.get(0),
        )
        .map_err(|_| EngineError::Storage)?;
    let newer: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM _fathomdb_source_dependencies d \
             JOIN _fathomdb_source_links l ON l.scope=d.scope AND l.artifact_revision_id=d.derived_revision_id \
             WHERE d.scope=?1 AND l.source_revision_id=?2 AND d.registered_dependency_generation>?3",
            params![
                scope,
                source_revision_id,
                i64::try_from(admitted_generation).map_err(|_| EngineError::Storage)?
            ],
            |row| row.get(0),
        )
        .map_err(|_| EngineError::Storage)?;
    let mut projection_rows = 0_i64;
    for dependent in &dependents {
        for projection in ROW_OWNED_PROJECTIONS.iter() {
            let sql = format!(
                "SELECT COUNT(*) FROM {} WHERE {}=?1",
                projection.table, projection.cursor_column
            );
            let count: i64 = connection
                .query_row(&sql, [dependent.write_cursor], |row| row.get(0))
                .map_err(|_| EngineError::Storage)?;
            projection_rows = projection_rows.saturating_add(count);
        }
    }
    let active = u64::try_from(active_nodes).map_err(|_| EngineError::Storage)?;
    let edges = u64::try_from(current_edges).map_err(|_| EngineError::Storage)?;
    Ok(ClosureProofV1 {
        schema_version: 1,
        proof_write_boundary: boundary,
        current_active_dependent_nodes: active,
        current_derived_edges: edges,
        view_eligible_dependents: active.saturating_add(edges),
        ownerless_projection_rows: u64::try_from(projection_rows)
            .map_err(|_| EngineError::Storage)?,
        post_admission_registrations: u64::try_from(newer).map_err(|_| EngineError::Storage)?,
        remaining_dependency_rows: None,
        remaining_canonical_rows: None,
        remaining_projection_rows: None,
        remaining_receipt_reference_rows: None,
    })
}

fn proof_json(proof: &ClosureProofV1) -> Result<String, EngineError> {
    serde_json::to_string(&json!({
        "schema_version": proof.schema_version,
        "proof_write_boundary": proof.proof_write_boundary,
        "current_active_dependent_nodes": proof.current_active_dependent_nodes,
        "current_derived_edges": proof.current_derived_edges,
        "view_eligible_dependents": proof.view_eligible_dependents,
        "ownerless_projection_rows": proof.ownerless_projection_rows,
        "post_admission_registrations": proof.post_admission_registrations,
        "remaining_dependency_rows": proof.remaining_dependency_rows,
        "remaining_canonical_rows": proof.remaining_canonical_rows,
        "remaining_projection_rows": proof.remaining_projection_rows,
        "remaining_receipt_reference_rows": proof.remaining_receipt_reference_rows,
    }))
    .map_err(|_| EngineError::Storage)
}

pub(crate) fn admit_soft_closure(
    connection: &Connection,
    scope: &str,
    source_revision_id: &str,
    cause: ClosureCauseV1,
    boundary: u64,
    mode: SoftClosureMode,
) -> Result<Option<ClosureOperationId>, EngineError> {
    if cause.is_physical() {
        return Err(EngineError::Storage);
    }
    let dependents = direct_dependents(connection, scope, source_revision_id)?;
    if dependents.is_empty() {
        return Ok(None);
    }
    for dependent in &dependents {
        validate_dependency_chain(
            connection,
            scope,
            source_revision_id,
            &dependent.revision_id,
            DependencyValidationMode::Persisted,
        )?;
    }
    let admitted_generation = load_dependency_generation(connection)?;
    let effective_at = current_epoch_seconds();
    let (sequence, id, retry) = next_identity(
        connection,
        "source_revision",
        source_revision_id,
        cause,
        effective_at,
        boundary,
        admitted_generation,
    )?;
    apply_soft_effects(connection, scope, &dependents, boundary)?;
    let proof = (mode == SoftClosureMode::Complete)
        .then(|| soft_proof(connection, scope, source_revision_id, admitted_generation, boundary))
        .transpose()?;
    let phase = if proof.is_some() { "complete" } else { "proving" };
    let encoded = proof.as_ref().map(proof_json).transpose()?;
    connection
        .execute(
            "INSERT INTO _fathomdb_dependency_closures(\
               schema_version,closure_operation_id,root_kind,root_value,cause,\
               effective_at_epoch_s,admitted_write_boundary,admitted_dependency_generation,\
               closure_sequence,retry_fingerprint,phase,affected_count,blocker_code,\
               structural_proof_write_boundary,proof_json,scope\
             ) VALUES(1,?1,'source_revision',?2,?3,?4,?5,?6,?7,?8,?9,?10,NULL,?11,?12,?13)",
            params![
                id.as_str(),
                source_revision_id,
                cause.as_str(),
                effective_at,
                i64::try_from(boundary).map_err(|_| EngineError::Storage)?,
                i64::try_from(admitted_generation).map_err(|_| EngineError::Storage)?,
                i64::try_from(sequence).map_err(|_| EngineError::Storage)?,
                retry,
                phase,
                i64::try_from(dependents.len()).map_err(|_| EngineError::Storage)?,
                proof.as_ref().map(|_| i64::try_from(boundary).unwrap_or(i64::MAX)),
                encoded,
                scope,
            ],
        )
        .map_err(|_| EngineError::Storage)?;
    store_sequence(connection, sequence)?;
    Ok(Some(id))
}

pub(crate) fn source_revision_for_cursor(
    connection: &Connection,
    scope: &str,
    cursor: i64,
) -> Result<Option<String>, EngineError> {
    connection
        .prepare_cached(
            "SELECT revision_id FROM _fathomdb_artifact_revisions \
             WHERE scope=?1 AND write_cursor=?2 AND artifact_role='canonical_source'",
        )
        .and_then(|mut stmt| stmt.query_row(params![scope, cursor], |row| row.get(0)).optional())
        .map_err(|_| EngineError::Storage)
}

pub(crate) fn guard_derived_reactivation(
    connection: &Connection,
    scope: &str,
    cursor: i64,
) -> Result<(), EngineError> {
    let source_revision: Option<String> = connection
        .query_row(
            "SELECT l.source_revision_id FROM _fathomdb_artifact_revisions r \
             JOIN _fathomdb_source_dependencies d \
               ON d.scope=r.scope AND d.derived_revision_id=r.revision_id \
             JOIN _fathomdb_source_links l \
               ON l.scope=r.scope AND l.artifact_revision_id=r.revision_id \
             WHERE r.scope=?1 AND r.write_cursor=?2",
            params![scope, cursor],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let Some(source_revision) = source_revision else {
        return Ok(());
    };
    if active_barrier_for_source(connection, scope, &source_revision)? {
        return Err(DependencyError::new(
            DependencyErrorReason::DependencyClosureActive,
            "/sourceRevisionId",
        )
        .into());
    }
    if !source_revision_is_strictly_eligible(
        connection,
        scope,
        &source_revision,
        current_epoch_seconds(),
    )? {
        return Err(DependencyError::new(
            DependencyErrorReason::DependencySourceIneligible,
            "/sourceRevisionId",
        )
        .into());
    }
    Ok(())
}

pub(crate) fn finalize_soft_closure(
    connection: &Connection,
    id: &ClosureOperationId,
) -> Result<(), EngineError> {
    let row: Option<(String, String, i64, i64, i64)> = connection
        .query_row(
            "SELECT scope,root_value,admitted_dependency_generation,admitted_write_boundary,affected_count \
             FROM _fathomdb_dependency_closures WHERE closure_operation_id=?1 \
               AND cause IN ('superseded','soft_deleted') AND phase IN ('proving','incomplete')",
            [id.as_str()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        )
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let Some((scope, source, generation, boundary, affected_count)) = row else {
        return Ok(());
    };
    let registered: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM _fathomdb_source_dependencies d \
             JOIN _fathomdb_source_links l ON l.scope=d.scope AND l.artifact_revision_id=d.derived_revision_id \
             WHERE d.scope=?1 AND l.source_revision_id=?2",
            params![&scope, &source],
            |row| row.get(0),
        )
        .map_err(|_| EngineError::Storage)?;
    let proof = soft_proof(
        connection,
        &scope,
        &source,
        u64::try_from(generation).map_err(|_| EngineError::Storage)?,
        u64::try_from(boundary).map_err(|_| EngineError::Storage)?,
    )?;
    let valid = registered == affected_count
        && proof.current_active_dependent_nodes == 0
        && proof.current_derived_edges == 0
        && proof.view_eligible_dependents == 0
        && proof.ownerless_projection_rows == 0
        && proof.post_admission_registrations == 0;
    if valid {
        connection
            .execute(
                "UPDATE _fathomdb_dependency_closures SET phase='complete',blocker_code=NULL,\
                 structural_proof_write_boundary=?2,proof_json=?3 \
                 WHERE closure_operation_id=?1",
                params![id.as_str(), boundary, proof_json(&proof)?,],
            )
            .map_err(|_| EngineError::Storage)?;
    } else {
        connection
            .execute(
                "UPDATE _fathomdb_dependency_closures SET phase='incomplete',\
                 blocker_code='proof_unavailable',structural_proof_write_boundary=NULL,\
                 proof_json=NULL WHERE closure_operation_id=?1",
                [id.as_str()],
            )
            .map_err(|_| EngineError::Storage)?;
    }
    Ok(())
}

fn parse_proof(value: &str, boundary: i64) -> Result<ClosureProofV1, EngineError> {
    let object = serde_json::from_str::<Value>(value)
        .ok()
        .and_then(|value| value.as_object().cloned())
        .ok_or(EngineError::Storage)?;
    let unsigned = |key: &str| object.get(key).and_then(Value::as_u64).ok_or(EngineError::Storage);
    let optional = |key: &str| -> Result<Option<u64>, EngineError> {
        match object.get(key) {
            Some(Value::Null) => Ok(None),
            Some(value) => value.as_u64().map(Some).ok_or(EngineError::Storage),
            None => Err(EngineError::Storage),
        }
    };
    let proof = ClosureProofV1 {
        schema_version: u32::try_from(unsigned("schema_version")?)
            .map_err(|_| EngineError::Storage)?,
        proof_write_boundary: unsigned("proof_write_boundary")?,
        current_active_dependent_nodes: unsigned("current_active_dependent_nodes")?,
        current_derived_edges: unsigned("current_derived_edges")?,
        view_eligible_dependents: unsigned("view_eligible_dependents")?,
        ownerless_projection_rows: unsigned("ownerless_projection_rows")?,
        post_admission_registrations: unsigned("post_admission_registrations")?,
        remaining_dependency_rows: optional("remaining_dependency_rows")?,
        remaining_canonical_rows: optional("remaining_canonical_rows")?,
        remaining_projection_rows: optional("remaining_projection_rows")?,
        remaining_receipt_reference_rows: optional("remaining_receipt_reference_rows")?,
    };
    if proof.schema_version != 1
        || proof.proof_write_boundary
            != u64::try_from(boundary).map_err(|_| EngineError::Storage)?
        || object.len() != 11
    {
        return Err(EngineError::Storage);
    }
    Ok(proof)
}

impl Engine {
    /// Return the current status of one opaque dependency-closure operation.
    ///
    /// # Errors
    ///
    /// Returns `Storage` when persisted closure state violates the closed
    /// schema or monotonic boundary invariants.
    pub fn read_dependency_closure(
        &self,
        lookup: ClosureLookupV1,
    ) -> Result<Option<ClosureStatusV1>, EngineError> {
        let _operation = self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        valid_closure_sequence(connection)
            .map_err(|_| EngineError::Storage)?
            .ok_or(EngineError::Storage)?;
        let scope = self.scope_predicate();
        let mut scope_sql = String::new();
        let mut params_values =
            vec![rusqlite::types::Value::Text(lookup.closure_operation_id.as_str().to_string())];
        scope.append_sql_values("scope", &mut scope_sql, &mut params_values);
        let sql = format!(
            "SELECT schema_version,closure_operation_id,root_kind,root_value,cause,phase,\
                    effective_at_epoch_s,admitted_write_boundary,\
                    admitted_dependency_generation,closure_sequence,affected_count,\
                    blocker_code,structural_proof_write_boundary,proof_json \
             FROM _fathomdb_dependency_closures WHERE closure_operation_id=?1 AND ({scope_sql})"
        );
        #[allow(clippy::type_complexity)]
        let row: Option<(
            i64,
            String,
            String,
            String,
            String,
            String,
            i64,
            i64,
            i64,
            i64,
            i64,
            Option<String>,
            Option<i64>,
            Option<String>,
        )> = connection
            .query_row(&sql, rusqlite::params_from_iter(params_values.iter()), |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                    row.get(9)?,
                    row.get(10)?,
                    row.get(11)?,
                    row.get(12)?,
                    row.get(13)?,
                ))
            })
            .optional()
            .map_err(|_| EngineError::Storage)?;
        let Some((
            schema,
            id,
            root_kind,
            root_value,
            cause,
            phase_text,
            effective_at,
            boundary,
            generation,
            sequence,
            affected,
            blocker,
            proof_boundary,
            encoded_proof,
        )) = row
        else {
            return Ok(None);
        };
        if schema != 1
            || sequence <= 0
            || affected <= 0
            || !valid_closure_id(&id)
            || id != lookup.closure_operation_id.as_str()
            || u64::try_from(boundary).is_err()
            || u64::try_from(generation).is_err()
        {
            return Err(EngineError::Storage);
        }
        let cause = ClosureCauseV1::parse(&cause).ok_or(EngineError::Storage)?;
        let phase = ClosurePhaseV1::parse(&phase_text).ok_or(EngineError::Storage)?;
        let root = match root_kind.as_str() {
            "source_revision" => ClosureRootV1::SourceRevision {
                source_revision_id: SourceRevisionId::new(root_value)
                    .map_err(|_| EngineError::Storage)?,
            },
            "source_bucket" => ClosureRootV1::SourceBucket {
                source_id: if stored_source_id_is_valid(&root_value) {
                    SourceId(root_value)
                } else {
                    return Err(EngineError::Storage);
                },
            },
            _ => return Err(EngineError::Storage),
        };
        let proof = match (proof_boundary, encoded_proof) {
            (Some(boundary), Some(value)) => Some(parse_proof(&value, boundary)?),
            (None, None) => None,
            _ => return Err(EngineError::Storage),
        };
        let phase_shape_valid = match phase {
            ClosurePhaseV1::Complete => blocker.is_none() && proof.is_some(),
            ClosurePhaseV1::Proving => blocker.is_none() && proof.is_none() && !cause.is_physical(),
            ClosurePhaseV1::AtRestPending => {
                blocker.is_none() && proof.is_some() && cause.is_physical()
            }
            ClosurePhaseV1::Incomplete => {
                blocker.as_deref().is_some_and(|value| {
                    [
                        "projection_state_unavailable",
                        "proof_unavailable",
                        "telemetry_redaction",
                        "wal_checkpoint",
                    ]
                    .contains(&value)
                }) && (proof.is_some() == cause.is_physical())
            }
        } && proof
            .as_ref()
            .is_none_or(|proof| proof_semantics_are_valid(proof, cause));
        if !phase_shape_valid {
            return Err(EngineError::Storage);
        }
        let current_boundary = self.next_cursor.load(Ordering::SeqCst);
        if proof.as_ref().is_some_and(|proof| proof.proof_write_boundary > current_boundary) {
            return Err(EngineError::Storage);
        }
        Ok(Some(ClosureStatusV1 {
            schema_version: lookup.schema_version,
            closure_operation_id: ClosureOperationId(id),
            root,
            cause,
            phase,
            effective_at_epoch_s: effective_at,
            admitted_write_boundary: u64::try_from(boundary).map_err(|_| EngineError::Storage)?,
            admitted_dependency_generation: u64::try_from(generation)
                .map_err(|_| EngineError::Storage)?,
            affected_count: u64::try_from(affected).map_err(|_| EngineError::Storage)?,
            blocker_code: blocker,
            proof,
        }))
    }
}
