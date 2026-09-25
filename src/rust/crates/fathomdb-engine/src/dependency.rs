use super::*;

/// Closed registration request for one canonical-source-to-derived relation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceDependencyRegistrationV1 {
    pub(crate) schema_version: u32,
    pub(crate) dependency_id: DependencyId,
    pub(crate) source_revision_id: SourceRevisionId,
    pub(crate) derived_revision_id: ArtifactRevisionId,
}

impl SourceDependencyRegistrationV1 {
    /// Validate and construct a schema-version-1 dependency registration.
    ///
    /// # Errors
    ///
    /// Returns a dependency-local typed identity refusal in request field order.
    pub fn new(
        dependency_id: impl Into<String>,
        source_revision_id: impl Into<String>,
        derived_revision_id: impl Into<String>,
    ) -> Result<Self, DependencyError> {
        let dependency_id = DependencyId::new(dependency_id)?;
        let source_revision_id = source_revision_id.into();
        if !valid_caller_identity(&source_revision_id) {
            return Err(DependencyError::new(
                DependencyErrorReason::DependencyReferenceInvalid,
                "/sourceRevisionId",
            ));
        }
        let derived_revision_id = derived_revision_id.into();
        if !valid_caller_identity(&derived_revision_id) {
            return Err(DependencyError::new(
                DependencyErrorReason::DependencyReferenceInvalid,
                "/derivedRevisionId",
            ));
        }
        Ok(Self {
            schema_version: 1,
            dependency_id,
            source_revision_id: SourceRevisionId(source_revision_id),
            derived_revision_id: ArtifactRevisionId(derived_revision_id),
        })
    }
}

/// Closed request for dependencies whose pinned source is one revision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DependencySourceLookupV1 {
    schema_version: u32,
    source_revision_id: SourceRevisionId,
}

impl DependencySourceLookupV1 {
    /// Validate and construct a source-side lookup.
    ///
    /// # Errors
    ///
    /// Returns `dependency_reference_invalid` when the revision ID is outside the grammar.
    pub fn new(source_revision_id: impl Into<String>) -> Result<Self, DependencyError> {
        let source_revision_id = source_revision_id.into();
        if !valid_caller_identity(&source_revision_id) {
            return Err(DependencyError::new(
                DependencyErrorReason::DependencyReferenceInvalid,
                "/sourceRevisionId",
            ));
        }
        Ok(Self { schema_version: 1, source_revision_id: SourceRevisionId(source_revision_id) })
    }
}

/// Closed request for the dependency registered to one derived revision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DependencyDerivedLookupV1 {
    schema_version: u32,
    derived_revision_id: ArtifactRevisionId,
}

impl DependencyDerivedLookupV1 {
    /// Validate and construct a derived-side lookup.
    ///
    /// # Errors
    ///
    /// Returns `dependency_reference_invalid` when the revision ID is outside the grammar.
    pub fn new(derived_revision_id: impl Into<String>) -> Result<Self, DependencyError> {
        let derived_revision_id = derived_revision_id.into();
        if !valid_caller_identity(&derived_revision_id) {
            return Err(DependencyError::new(
                DependencyErrorReason::DependencyReferenceInvalid,
                "/derivedRevisionId",
            ));
        }
        Ok(Self { schema_version: 1, derived_revision_id: ArtifactRevisionId(derived_revision_id) })
    }
}

/// One immutable source dependency registration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceDependencyV1 {
    /// Closed response schema discriminator.
    pub schema_version: u32,
    /// Caller-authored immutable dependency identity.
    pub dependency_id: DependencyId,
    /// Pinned canonical source revision.
    pub source_revision_id: SourceRevisionId,
    /// Pinned complete derived revision.
    pub derived_revision_id: ArtifactRevisionId,
    /// Independent generation allocated when the registration was committed.
    pub registered_dependency_generation: u64,
}

/// Bounded, deterministically ordered dependency result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DependencyListV1 {
    /// Closed response schema discriminator.
    pub schema_version: u32,
    /// At most 100 dependencies in derived-revision, dependency-ID order.
    pub items: Vec<SourceDependencyV1>,
}

/// Closed machine-readable reason for a dependency refusal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DependencyErrorReason {
    /// The caller-authored dependency ID is outside the closed identity grammar.
    DependencyIdInvalid,
    /// A requested endpoint identity is outside the closed identity grammar.
    DependencyReferenceInvalid,
    /// A requested endpoint or required provenance-chain row does not exist.
    DependencyReferenceMissing,
    /// A requested endpoint exists but has incomplete provenance.
    DependencyProvenanceIncomplete,
    /// The requested source disagrees with the derived revision's authoritative link.
    DependencyProvenanceMismatch,
    /// The requested endpoints are self-referential or have forbidden roles.
    DependencyCycleOrRoleInvalid,
    /// The dependency ID or derived revision is already owned by another relation.
    DependencyConflict,
    /// A source lookup would return more than the contract's 100-row bound.
    DependencyLookupBoundExceeded,
    /// The independent generation singleton has reached SQLite's signed maximum.
    DependencyGenerationExhausted,
    /// The pinned source exists but is not current, active, or in-window.
    DependencySourceIneligible,
    /// A nonterminal lifecycle closure fences the pinned source.
    DependencyClosureActive,
    /// The request schema discriminator is absent or not exactly version 1.
    UnsupportedSchemaVersion,
    /// A closed request contains an unrecognized field.
    UnknownField,
}

impl DependencyErrorReason {
    /// Stable lower-snake-case wire spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DependencyIdInvalid => "dependency_id_invalid",
            Self::DependencyReferenceInvalid => "dependency_reference_invalid",
            Self::DependencyReferenceMissing => "dependency_reference_missing",
            Self::DependencyProvenanceIncomplete => "dependency_provenance_incomplete",
            Self::DependencyProvenanceMismatch => "dependency_provenance_mismatch",
            Self::DependencyCycleOrRoleInvalid => "dependency_cycle_or_role_invalid",
            Self::DependencyConflict => "dependency_conflict",
            Self::DependencyLookupBoundExceeded => "dependency_lookup_bound_exceeded",
            Self::DependencyGenerationExhausted => "dependency_generation_exhausted",
            Self::DependencySourceIneligible => "dependency_source_ineligible",
            Self::DependencyClosureActive => "dependency_closure_active",
            Self::UnsupportedSchemaVersion => "unsupported_schema_version",
            Self::UnknownField => "unknown_field",
        }
    }
}

/// Typed dependency refusal with an RFC 6901 canonical request pointer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DependencyError {
    /// Closed machine-readable refusal reason.
    pub reason: DependencyErrorReason,
    /// RFC 6901 pointer over canonical camel-case request names, or empty for cross-row errors.
    pub field_path: String,
}

impl DependencyError {
    pub(crate) fn new(reason: DependencyErrorReason, field_path: impl Into<String>) -> Self {
        Self { reason, field_path: field_path.into() }
    }
}

impl Display for DependencyError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} at {}", self.reason.as_str(), self.field_path)
    }
}

impl Error for DependencyError {}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum DependencyValidationMode {
    Requested,
    Persisted,
}

#[allow(dead_code)] // The non-default constructor is the committed Slice 25 reuse seam.
#[derive(Clone, Debug)]
pub(crate) struct ProspectiveCanonicalSource {
    source_id: String,
    source_version_id: String,
    body: String,
}

/// Transaction-local dependency facts prepared during Slice 25's validate-all
/// phase, before its domain rows are inserted.
#[derive(Clone, Debug, Default)]
pub(crate) struct DependencyProspectiveState {
    valid_chains: std::collections::HashSet<(String, String)>,
    registrations_by_id: std::collections::HashMap<String, (String, String)>,
    registrations_by_derived: std::collections::HashMap<String, String>,
}

#[allow(dead_code)] // Slice 20 uses the empty view; Slice 25 supplies prospective writes.
impl DependencyProspectiveState {
    /// Derive dependency-visible endpoint facts from ordered writes. A derived
    /// write may reference a persisted source or a canonical source earlier in
    /// `writes`; later/forward references remain invalid.
    pub(crate) fn from_prepared_writes(
        connection: &Connection,
        writes: &[PreparedWrite],
    ) -> Result<Self, EngineError> {
        let mut state = Self::default();
        let mut sources = std::collections::HashMap::<String, ProspectiveCanonicalSource>::new();
        for write in writes {
            let (source_id, body, provenance, is_node) = match write {
                PreparedWrite::ProvenancedNode(node) => {
                    (node.source_id.as_str(), Some(node.body.as_str()), &node.provenance, true)
                }
                PreparedWrite::ProvenancedEdge(edge) => {
                    (edge.source_id.as_str(), edge.body.as_deref(), &edge.provenance, false)
                }
                _ => continue,
            };
            match provenance.role {
                ProvenanceRole::Canonical => {
                    if !is_node {
                        return Err(dependency_validation_error(
                            DependencyValidationMode::Requested,
                            DependencyErrorReason::DependencyCycleOrRoleInvalid,
                        ));
                    }
                    let body = body.ok_or(EngineError::Storage)?;
                    sources.insert(
                        provenance.artifact_revision_id.as_str().to_string(),
                        ProspectiveCanonicalSource {
                            source_id: source_id.to_string(),
                            source_version_id: provenance.source_version_id.as_str().to_string(),
                            body: body.to_string(),
                        },
                    );
                }
                ProvenanceRole::Derived => {
                    let source_revision =
                        provenance.source_revision_id.as_ref().ok_or(EngineError::Storage)?;
                    let source = match sources.get(source_revision.as_str()) {
                        Some(source) => source.clone(),
                        None => {
                            load_persisted_canonical_source(connection, source_revision.as_str())?
                                .ok_or_else(|| {
                                dependency_validation_error(
                                    DependencyValidationMode::Requested,
                                    DependencyErrorReason::DependencyReferenceMissing,
                                )
                            })?
                        }
                    };
                    if source_id != source.source_id
                        || provenance.source_version_id.as_str() != source.source_version_id
                        || provenance.canonical_source_hash.as_ref().is_none_or(|hash| {
                            hash.digest_hex() != canonical_body_hash(&source.body)
                        })
                        || provenance.locator.as_ref().is_none_or(|locator| {
                            checked_locator_columns(locator, &source.body).is_err()
                        })
                    {
                        return Err(dependency_validation_error(
                            DependencyValidationMode::Requested,
                            DependencyErrorReason::DependencyProvenanceMismatch,
                        ));
                    }
                    let derived_revision = provenance.artifact_revision_id.as_str();
                    if derived_revision == source_revision.as_str() {
                        return Err(dependency_validation_error(
                            DependencyValidationMode::Requested,
                            DependencyErrorReason::DependencyCycleOrRoleInvalid,
                        ));
                    }
                    state.valid_chains.insert((
                        source_revision.as_str().to_string(),
                        derived_revision.to_string(),
                    ));
                }
            }
        }
        Ok(state)
    }

    /// Add one already-validated operation to the prospective conflict view.
    pub(crate) fn record_registration(
        &mut self,
        registration: &ValidatedSourceDependencyRegistration,
    ) -> Result<(), EngineError> {
        let ValidatedSourceDependencyRegistration::Insert(request) = registration else {
            return Ok(());
        };
        let dependency_id = request.dependency_id.as_str();
        let source = request.source_revision_id.as_str();
        let derived = request.derived_revision_id.as_str();
        if self
            .registrations_by_id
            .get(dependency_id)
            .is_some_and(|stored| stored != &(source.to_string(), derived.to_string()))
            || self
                .registrations_by_derived
                .get(derived)
                .is_some_and(|stored| stored != dependency_id)
        {
            return Err(DependencyError::new(DependencyErrorReason::DependencyConflict, "").into());
        }
        self.registrations_by_id
            .insert(dependency_id.to_string(), (source.to_string(), derived.to_string()));
        self.registrations_by_derived.insert(derived.to_string(), dependency_id.to_string());
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub(crate) enum ValidatedSourceDependencyRegistration {
    Replay(SourceDependencyV1),
    ProspectiveReplay,
    Insert(SourceDependencyRegistrationV1),
}

fn dependency_validation_error(
    mode: DependencyValidationMode,
    reason: DependencyErrorReason,
) -> EngineError {
    match mode {
        DependencyValidationMode::Requested => DependencyError::new(reason, "").into(),
        DependencyValidationMode::Persisted => EngineError::Storage,
    }
}

fn source_dependency_from_request(
    request: SourceDependencyRegistrationV1,
    generation: u64,
) -> SourceDependencyV1 {
    SourceDependencyV1 {
        schema_version: request.schema_version,
        dependency_id: request.dependency_id,
        source_revision_id: request.source_revision_id,
        derived_revision_id: request.derived_revision_id,
        registered_dependency_generation: generation,
    }
}

pub(crate) fn validate_source_dependency_registration(
    connection: &Connection,
    request: SourceDependencyRegistrationV1,
    prospective: &DependencyProspectiveState,
) -> Result<ValidatedSourceDependencyRegistration, EngineError> {
    let dependency_id = request.dependency_id.as_str();
    let requested_source = request.source_revision_id.as_str();
    let requested_derived = request.derived_revision_id.as_str();

    if let Some((source, derived)) = prospective.registrations_by_id.get(dependency_id) {
        if source == requested_source && derived == requested_derived {
            return Ok(ValidatedSourceDependencyRegistration::ProspectiveReplay);
        }
        return Err(DependencyError::new(DependencyErrorReason::DependencyConflict, "").into());
    }
    if prospective.registrations_by_derived.contains_key(requested_derived) {
        return Err(DependencyError::new(DependencyErrorReason::DependencyConflict, "").into());
    }

    let by_id: Option<(i64, String, i64)> = connection
        .query_row(
            "SELECT schema_version, derived_revision_id, registered_dependency_generation \
             FROM _fathomdb_source_dependencies WHERE dependency_id=?1",
            [dependency_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let by_derived: Option<(i64, String, i64)> = connection
        .query_row(
            "SELECT schema_version, dependency_id, registered_dependency_generation \
             FROM _fathomdb_source_dependencies WHERE derived_revision_id=?1",
            [requested_derived],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(|_| EngineError::Storage)?;

    if let Some((stored_schema, stored_derived, stored_generation)) = by_id {
        validate_persisted_dependency_row(
            connection,
            stored_schema,
            dependency_id,
            &stored_derived,
            stored_generation,
        )?;
        if stored_derived != requested_derived
            || by_derived.as_ref().map(|(_, stored_id, _)| stored_id.as_str())
                != Some(dependency_id)
        {
            return Err(DependencyError::new(DependencyErrorReason::DependencyConflict, "").into());
        }
        let stored_source: String = connection
            .query_row(
                "SELECT source_revision_id FROM _fathomdb_source_links \
                 WHERE artifact_revision_id=?1",
                [requested_derived],
                |row| row.get(0),
            )
            .map_err(|_| EngineError::Storage)?;
        validate_dependency_chain(
            connection,
            &stored_source,
            requested_derived,
            DependencyValidationMode::Persisted,
        )?;
        if stored_source != requested_source {
            return Err(DependencyError::new(DependencyErrorReason::DependencyConflict, "").into());
        }
        let generation = u64::try_from(stored_generation).map_err(|_| EngineError::Storage)?;
        return Ok(ValidatedSourceDependencyRegistration::Replay(source_dependency_from_request(
            request, generation,
        )));
    }
    if let Some((stored_schema, stored_dependency_id, stored_generation)) = by_derived {
        validate_persisted_dependency_row(
            connection,
            stored_schema,
            &stored_dependency_id,
            requested_derived,
            stored_generation,
        )?;
        let stored_source: Option<String> = connection
            .query_row(
                "SELECT source_revision_id FROM _fathomdb_source_links \
                 WHERE artifact_revision_id=?1",
                [requested_derived],
                |row| row.get(0),
            )
            .optional()
            .map_err(|_| EngineError::Storage)?;
        let stored_source = stored_source.ok_or(EngineError::Storage)?;
        validate_dependency_chain(
            connection,
            &stored_source,
            requested_derived,
            DependencyValidationMode::Persisted,
        )?;
        return Err(DependencyError::new(DependencyErrorReason::DependencyConflict, "").into());
    }

    if !prospective
        .valid_chains
        .contains(&(requested_source.to_string(), requested_derived.to_string()))
    {
        validate_dependency_chain(
            connection,
            requested_source,
            requested_derived,
            DependencyValidationMode::Requested,
        )?;
    }
    Ok(ValidatedSourceDependencyRegistration::Insert(request))
}

pub(crate) fn apply_validated_source_dependency(
    connection: &Connection,
    registration: &ValidatedSourceDependencyRegistration,
    generation: u64,
) -> Result<bool, EngineError> {
    let ValidatedSourceDependencyRegistration::Insert(request) = registration else {
        return Ok(false);
    };
    connection
        .execute(
            "INSERT INTO _fathomdb_source_dependencies(\
               schema_version, dependency_id, derived_revision_id, registered_dependency_generation\
             ) VALUES(1, ?1, ?2, ?3)",
            params![
                request.dependency_id.as_str(),
                request.derived_revision_id.as_str(),
                i64::try_from(generation).map_err(|_| EngineError::Storage)?
            ],
        )
        .map_err(|_| EngineError::Storage)?;
    Ok(true)
}

pub(crate) fn canonical_dependency_generation(value: &str) -> Option<u64> {
    if value.is_empty()
        || (value.len() > 1 && value.starts_with('0'))
        || !value.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    value.parse::<u64>().ok().filter(|value| *value <= i64::MAX as u64)
}

pub(crate) fn load_dependency_generation(connection: &Connection) -> Result<u64, EngineError> {
    let value: String = connection
        .query_row(
            "SELECT value FROM _fathomdb_open_state WHERE key=?1",
            [DEPENDENCY_GENERATION_KEY],
            |row| row.get(0),
        )
        .map_err(|_| EngineError::Storage)?;
    canonical_dependency_generation(&value).ok_or(EngineError::Storage)
}

pub(crate) fn reserve_dependency_generation(connection: &Connection) -> Result<u64, EngineError> {
    let current = load_dependency_generation(connection)?;
    if current >= i64::MAX as u64 {
        return Err(
            DependencyError::new(DependencyErrorReason::DependencyGenerationExhausted, "").into()
        );
    }
    Ok(current + 1)
}

pub(crate) fn store_dependency_generation(
    connection: &Connection,
    generation: u64,
) -> Result<(), EngineError> {
    let updated = connection
        .execute(
            "UPDATE _fathomdb_open_state SET value=?1 WHERE key=?2",
            params![generation.to_string(), DEPENDENCY_GENERATION_KEY],
        )
        .map_err(|_| EngineError::Storage)?;
    if updated != 1 {
        return Err(EngineError::Storage);
    }
    Ok(())
}

fn canonical_row_for_revision(
    connection: &Connection,
    artifact_class: &str,
    write_cursor: i64,
) -> Result<Option<(String, Option<String>)>, EngineError> {
    let sql = match artifact_class {
        "node" => "SELECT source_id, body FROM canonical_nodes WHERE write_cursor=?1",
        "edge" => "SELECT source_id, body FROM canonical_edges WHERE write_cursor=?1",
        _ => return Err(EngineError::Storage),
    };
    connection
        .query_row(sql, [write_cursor], |row| Ok((row.get(0)?, row.get(1)?)))
        .optional()
        .map_err(|_| EngineError::Storage)
}

pub(crate) fn stored_source_id_is_valid(value: &str) -> bool {
    SourceId::new(value).is_ok()
}

fn stored_source_version_id_is_valid(value: &str) -> bool {
    SourceVersionId::new(value).is_ok()
}

pub(crate) fn stored_artifact_revision_id_is_valid(value: &str) -> bool {
    ArtifactRevisionId::new(value).is_ok()
}

fn stored_source_revision_id_is_valid(value: &str) -> bool {
    SourceRevisionId::new(value).is_ok()
}

fn stored_canonical_source_revision_id_is_valid(value: &str) -> bool {
    stored_artifact_revision_id_is_valid(value) && stored_source_revision_id_is_valid(value)
}

#[allow(dead_code)] // Used by the Slice 25 prospective-write constructor.
pub(crate) fn load_persisted_canonical_source(
    connection: &Connection,
    source_revision: &str,
) -> Result<Option<ProspectiveCanonicalSource>, EngineError> {
    if !stored_canonical_source_revision_id_is_valid(source_revision) {
        return Err(EngineError::Storage);
    }
    let owner: Option<(i64, String, String, String, i64)> = connection
        .query_row(
            "SELECT schema_version, artifact_class, artifact_role, completeness, write_cursor \
             FROM _fathomdb_artifact_revisions WHERE revision_id=?1",
            [source_revision],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        )
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let Some((schema, class, role, completeness, cursor)) = owner else {
        return Ok(None);
    };
    if schema != 1 {
        return Err(EngineError::Storage);
    }
    if completeness != "complete" {
        return Err(dependency_validation_error(
            DependencyValidationMode::Requested,
            DependencyErrorReason::DependencyProvenanceIncomplete,
        ));
    }
    if class != "node" || role != "canonical_source" {
        return Err(dependency_validation_error(
            DependencyValidationMode::Requested,
            DependencyErrorReason::DependencyCycleOrRoleInvalid,
        ));
    }
    let canonical: Option<(String, String)> = connection
        .query_row(
            "SELECT source_id, body FROM canonical_nodes WHERE write_cursor=?1",
            [cursor],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let version: Option<(i64, String, String, String)> = connection
        .query_row(
            "SELECT schema_version, source_id, source_version_id, source_revision_id \
             FROM _fathomdb_source_versions WHERE source_revision_id=?1",
            [source_revision],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()
        .map_err(|_| EngineError::Storage)?;
    #[allow(clippy::type_complexity)]
    let self_link: Option<(
        i64,
        String,
        String,
        String,
        String,
        Option<i64>,
        Option<i64>,
        String,
        String,
    )> = connection
        .query_row(
            "SELECT schema_version, source_id, source_version_id, source_revision_id, \
                    locator_kind, start_byte, end_byte, hash_algorithm, hash_digest \
             FROM _fathomdb_source_links WHERE artifact_revision_id=?1",
            [source_revision],
            |row| {
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
                ))
            },
        )
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let Some((canonical_source_id, body)) = canonical else {
        return Err(dependency_validation_error(
            DependencyValidationMode::Requested,
            DependencyErrorReason::DependencyReferenceMissing,
        ));
    };
    let Some((version_schema, version_source_id, version_id, version_revision)) = version else {
        return Err(dependency_validation_error(
            DependencyValidationMode::Requested,
            DependencyErrorReason::DependencyReferenceMissing,
        ));
    };
    let Some((
        link_schema,
        link_source_id,
        link_version_id,
        link_revision,
        locator,
        start,
        end,
        algorithm,
        hash,
    )) = self_link
    else {
        return Err(dependency_validation_error(
            DependencyValidationMode::Requested,
            DependencyErrorReason::DependencyReferenceMissing,
        ));
    };
    if !stored_source_id_is_valid(&canonical_source_id)
        || !stored_source_id_is_valid(&version_source_id)
        || !stored_source_id_is_valid(&link_source_id)
        || !stored_source_version_id_is_valid(&version_id)
        || !stored_source_version_id_is_valid(&link_version_id)
        || !stored_canonical_source_revision_id_is_valid(&version_revision)
        || !stored_canonical_source_revision_id_is_valid(&link_revision)
    {
        return Err(EngineError::Storage);
    }
    if version_schema != 1
        || link_schema != 1
        || canonical_source_id != version_source_id
        || canonical_source_id != link_source_id
        || version_id != link_version_id
        || version_revision != source_revision
        || link_revision != source_revision
        || locator != "whole_body"
        || start.is_some()
        || end.is_some()
        || algorithm != "sha256"
        || hash != canonical_body_hash(&body)
    {
        return Err(dependency_validation_error(
            DependencyValidationMode::Requested,
            DependencyErrorReason::DependencyProvenanceMismatch,
        ));
    }
    Ok(Some(ProspectiveCanonicalSource {
        source_id: canonical_source_id,
        source_version_id: version_id,
        body,
    }))
}

pub(crate) fn validate_dependency_chain(
    connection: &Connection,
    requested_source_revision: &str,
    derived_revision: &str,
    mode: DependencyValidationMode,
) -> Result<(), EngineError> {
    if !stored_canonical_source_revision_id_is_valid(requested_source_revision)
        || !stored_artifact_revision_id_is_valid(derived_revision)
    {
        return Err(EngineError::Storage);
    }
    if requested_source_revision == derived_revision {
        return Err(dependency_validation_error(
            mode,
            DependencyErrorReason::DependencyCycleOrRoleInvalid,
        ));
    }

    let derived_owner: Option<(i64, String, String, String)> = connection
        .query_row(
            "SELECT schema_version, artifact_class, artifact_role, completeness, write_cursor \
             FROM _fathomdb_artifact_revisions WHERE revision_id=?1",
            [derived_revision],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            },
        )
        .optional()
        .map_err(|_| EngineError::Storage)?
        .map(|(schema, class, role, completeness, cursor)| {
            if schema != 1 {
                return Err(EngineError::Storage);
            }
            Ok((cursor, class, role, completeness))
        })
        .transpose()?;
    let Some((derived_cursor, derived_class, derived_role, derived_completeness)) = derived_owner
    else {
        return Err(dependency_validation_error(
            mode,
            DependencyErrorReason::DependencyReferenceMissing,
        ));
    };
    if derived_completeness != "complete" {
        return Err(dependency_validation_error(
            mode,
            DependencyErrorReason::DependencyProvenanceIncomplete,
        ));
    }
    if derived_role != "derived_semantic" {
        return Err(dependency_validation_error(
            mode,
            DependencyErrorReason::DependencyCycleOrRoleInvalid,
        ));
    }
    let Some((derived_source_id, _)) =
        canonical_row_for_revision(connection, &derived_class, derived_cursor)?
    else {
        return Err(dependency_validation_error(
            mode,
            DependencyErrorReason::DependencyReferenceMissing,
        ));
    };

    #[allow(clippy::type_complexity)]
    let link: Option<(
        i64,
        String,
        String,
        String,
        String,
        Option<i64>,
        Option<i64>,
        String,
        String,
    )> = connection
        .query_row(
            "SELECT schema_version, source_id, source_version_id, source_revision_id, \
                    locator_kind, start_byte, end_byte, hash_algorithm, hash_digest \
             FROM _fathomdb_source_links WHERE artifact_revision_id=?1",
            [derived_revision],
            |row| {
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
                ))
            },
        )
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let Some((
        link_schema,
        link_source_id,
        link_source_version,
        linked_source_revision,
        locator_kind,
        start_byte,
        end_byte,
        hash_algorithm,
        link_hash,
    )) = link
    else {
        return Err(dependency_validation_error(
            mode,
            DependencyErrorReason::DependencyReferenceMissing,
        ));
    };
    if !stored_canonical_source_revision_id_is_valid(&linked_source_revision) {
        return Err(EngineError::Storage);
    }
    if linked_source_revision != requested_source_revision {
        return Err(dependency_validation_error(
            mode,
            DependencyErrorReason::DependencyProvenanceMismatch,
        ));
    }

    let source_owner: Option<(i64, String, String, String, i64)> = connection
        .query_row(
            "SELECT write_cursor, artifact_class, artifact_role, completeness, schema_version \
             FROM _fathomdb_artifact_revisions WHERE revision_id=?1",
            [requested_source_revision],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        )
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let Some((source_cursor, source_class, source_role, source_completeness, source_schema)) =
        source_owner
    else {
        return Err(dependency_validation_error(
            mode,
            DependencyErrorReason::DependencyReferenceMissing,
        ));
    };
    if source_completeness != "complete" {
        return Err(dependency_validation_error(
            mode,
            DependencyErrorReason::DependencyProvenanceIncomplete,
        ));
    }
    if source_role != "canonical_source" || source_class != "node" {
        return Err(dependency_validation_error(
            mode,
            DependencyErrorReason::DependencyCycleOrRoleInvalid,
        ));
    }
    let Some((canonical_source_id, canonical_body)) =
        canonical_row_for_revision(connection, &source_class, source_cursor)?
    else {
        return Err(dependency_validation_error(
            mode,
            DependencyErrorReason::DependencyReferenceMissing,
        ));
    };
    let canonical_body = canonical_body.ok_or(EngineError::Storage)?;

    let source_version: Option<(i64, String, String, String)> = connection
        .query_row(
            "SELECT schema_version, source_id, source_version_id, source_revision_id \
             FROM _fathomdb_source_versions WHERE source_revision_id=?1",
            [requested_source_revision],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()
        .map_err(|_| EngineError::Storage)?;
    #[allow(clippy::type_complexity)]
    let self_link: Option<(
        i64,
        String,
        String,
        String,
        String,
        Option<i64>,
        Option<i64>,
        String,
        String,
    )> = connection
        .query_row(
            "SELECT schema_version, source_id, source_version_id, source_revision_id, \
                    locator_kind, start_byte, end_byte, hash_algorithm, hash_digest \
             FROM _fathomdb_source_links WHERE artifact_revision_id=?1",
            [requested_source_revision],
            |row| {
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
                ))
            },
        )
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let Some((version_schema, version_source_id, version_id, version_revision)) = source_version
    else {
        return Err(dependency_validation_error(
            mode,
            DependencyErrorReason::DependencyReferenceMissing,
        ));
    };
    let Some((
        self_schema,
        self_source_id,
        self_version_id,
        self_revision,
        self_locator,
        self_start,
        self_end,
        self_algorithm,
        self_hash,
    )) = self_link
    else {
        return Err(dependency_validation_error(
            mode,
            DependencyErrorReason::DependencyReferenceMissing,
        ));
    };
    let computed_hash = Sha256::digest(canonical_body.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    if ![
        derived_source_id.as_str(),
        link_source_id.as_str(),
        canonical_source_id.as_str(),
        version_source_id.as_str(),
        self_source_id.as_str(),
    ]
    .into_iter()
    .all(stored_source_id_is_valid)
        || ![link_source_version.as_str(), version_id.as_str(), self_version_id.as_str()]
            .into_iter()
            .all(stored_source_version_id_is_valid)
        || ![version_revision.as_str(), self_revision.as_str()]
            .into_iter()
            .all(stored_canonical_source_revision_id_is_valid)
    {
        return Err(EngineError::Storage);
    }
    let locator_valid = match locator_kind.as_str() {
        "whole_body" => start_byte.is_none() && end_byte.is_none(),
        "utf8_bytes" => match (start_byte, end_byte) {
            (Some(start), Some(end)) => {
                start >= 0
                    && start <= end
                    && usize::try_from(end).is_ok_and(|end| end <= canonical_body.len())
                    && usize::try_from(start)
                        .is_ok_and(|start| canonical_body.is_char_boundary(start))
                    && usize::try_from(end).is_ok_and(|end| canonical_body.is_char_boundary(end))
            }
            _ => false,
        },
        _ => false,
    };
    let consistent = link_schema == 1
        && source_schema == 1
        && version_schema == 1
        && self_schema == 1
        && derived_source_id == link_source_id
        && canonical_source_id == link_source_id
        && version_source_id == link_source_id
        && self_source_id == link_source_id
        && version_id == link_source_version
        && self_version_id == link_source_version
        && version_revision == requested_source_revision
        && self_revision == requested_source_revision
        && self_locator == "whole_body"
        && self_start.is_none()
        && self_end.is_none()
        && hash_algorithm == "sha256"
        && self_algorithm == "sha256"
        && link_hash == self_hash
        && self_hash == computed_hash
        && locator_valid;
    if !consistent {
        return Err(match mode {
            DependencyValidationMode::Requested => dependency_validation_error(
                mode,
                DependencyErrorReason::DependencyProvenanceMismatch,
            ),
            DependencyValidationMode::Persisted => EngineError::Storage,
        });
    }
    if mode == DependencyValidationMode::Requested {
        if dependency_closure::active_barrier_for_source(connection, requested_source_revision)? {
            return Err(DependencyError::new(
                DependencyErrorReason::DependencyClosureActive,
                "/sourceRevisionId",
            )
            .into());
        }
        if !dependency_closure::source_revision_is_strictly_eligible(
            connection,
            requested_source_revision,
            current_epoch_seconds(),
        )? {
            return Err(DependencyError::new(
                DependencyErrorReason::DependencySourceIneligible,
                "/sourceRevisionId",
            )
            .into());
        }
    }
    Ok(())
}

pub(crate) fn validate_persisted_dependency_row(
    connection: &Connection,
    schema_version: i64,
    dependency_id: &str,
    derived_revision_id: &str,
    generation: i64,
) -> Result<(), EngineError> {
    if schema_version != 1
        || DependencyId::new(dependency_id).is_err()
        || ArtifactRevisionId::new(derived_revision_id).is_err()
    {
        return Err(EngineError::Storage);
    }
    let generation = u64::try_from(generation).map_err(|_| EngineError::Storage)?;
    if generation == 0 || generation > load_dependency_generation(connection)? {
        return Err(EngineError::Storage);
    }
    Ok(())
}

impl Engine {
    /// Register one immutable dependency over an authoritative Slice 15 source link.
    ///
    /// Exact replay is a no-op success. A successful new registration advances
    /// only the independent dependency generation; it never consumes a canonical
    /// write cursor or creates projection work.
    ///
    /// # Errors
    ///
    /// Returns a typed dependency refusal for invalid endpoints, conflicts, or
    /// generation exhaustion, and `Storage` for persisted-chain corruption.
    pub fn register_source_dependency(
        &self,
        request: SourceDependencyRegistrationV1,
    ) -> Result<SourceDependencyV1, EngineError> {
        self.ensure_open()?;
        let mut connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_mut().ok_or(EngineError::Closing)?;
        dependency_closure::maintain_before_writer(connection)?;
        let tx = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| EngineError::Storage)?;
        dependency_closure::guard_no_pending_physical(&tx)?;
        let validated = validate_source_dependency_registration(
            &tx,
            request,
            &DependencyProspectiveState::default(),
        )?;
        let validated = match validated {
            ValidatedSourceDependencyRegistration::Replay(row) => {
                tx.commit().map_err(|_| EngineError::Storage)?;
                return Ok(row);
            }
            validated => validated,
        };
        let generation = reserve_dependency_generation(&tx)?;
        let inserted = apply_validated_source_dependency(&tx, &validated, generation)?;
        if !inserted {
            return Err(EngineError::Storage);
        }
        store_dependency_generation(&tx, generation)?;
        tx.commit().map_err(|_| EngineError::Storage)?;
        let ValidatedSourceDependencyRegistration::Insert(request) = validated else {
            return Err(EngineError::Storage);
        };
        Ok(source_dependency_from_request(request, generation))
    }

    /// Return at most 100 dependencies pinned to one source revision.
    ///
    /// # Errors
    ///
    /// Returns `dependency_lookup_bound_exceeded` above 100 matches and
    /// `Storage` when a persisted dependency chain is inconsistent.
    pub fn dependencies_for_source(
        &self,
        request: DependencySourceLookupV1,
    ) -> Result<DependencyListV1, EngineError> {
        self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        let mut statement = connection
            .prepare(
                "SELECT d.schema_version, d.dependency_id, d.derived_revision_id, \
                        d.registered_dependency_generation \
                 FROM _fathomdb_source_dependencies d \
                 JOIN _fathomdb_source_links l \
                   ON l.artifact_revision_id=d.derived_revision_id \
                 WHERE l.source_revision_id=?1 \
                 ORDER BY d.derived_revision_id, d.dependency_id LIMIT 101",
            )
            .map_err(|_| EngineError::Storage)?;
        let rows = statement
            .query_map([request.source_revision_id.as_str()], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            })
            .map_err(|_| EngineError::Storage)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|_| EngineError::Storage)?;
        for (schema, dependency_id, derived_revision_id, generation) in &rows {
            validate_persisted_dependency_row(
                connection,
                *schema,
                dependency_id,
                derived_revision_id,
                *generation,
            )?;
            validate_dependency_chain(
                connection,
                request.source_revision_id.as_str(),
                derived_revision_id,
                DependencyValidationMode::Persisted,
            )?;
        }
        if rows.len() > DEPENDENCY_LOOKUP_LIMIT {
            return Err(DependencyError::new(
                DependencyErrorReason::DependencyLookupBoundExceeded,
                "",
            )
            .into());
        }
        let mut items = Vec::with_capacity(rows.len());
        for (_, dependency_id, derived_revision_id, generation) in rows {
            items.push(SourceDependencyV1 {
                schema_version: 1,
                dependency_id: DependencyId(dependency_id),
                source_revision_id: request.source_revision_id.clone(),
                derived_revision_id: ArtifactRevisionId(derived_revision_id),
                registered_dependency_generation: u64::try_from(generation)
                    .map_err(|_| EngineError::Storage)?,
            });
        }
        Ok(DependencyListV1 { schema_version: request.schema_version, items })
    }

    /// Return the dependency registered to one derived revision, if any.
    ///
    /// # Errors
    ///
    /// Returns `Storage` when a persisted dependency chain is inconsistent.
    pub fn dependency_for_derived(
        &self,
        request: DependencyDerivedLookupV1,
    ) -> Result<Option<SourceDependencyV1>, EngineError> {
        self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        let row: Option<(i64, String, i64, String)> = connection
            .query_row(
                "SELECT d.schema_version, d.dependency_id, d.registered_dependency_generation, \
                        l.source_revision_id \
                 FROM _fathomdb_source_dependencies d \
                 LEFT JOIN _fathomdb_source_links l \
                   ON l.artifact_revision_id=d.derived_revision_id \
                 WHERE d.derived_revision_id=?1",
                [request.derived_revision_id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()
            .map_err(|_| EngineError::Storage)?;
        let Some((schema, dependency_id, generation, source_revision_id)) = row else {
            return Ok(None);
        };
        validate_persisted_dependency_row(
            connection,
            schema,
            &dependency_id,
            request.derived_revision_id.as_str(),
            generation,
        )?;
        validate_dependency_chain(
            connection,
            &source_revision_id,
            request.derived_revision_id.as_str(),
            DependencyValidationMode::Persisted,
        )?;
        Ok(Some(SourceDependencyV1 {
            schema_version: request.schema_version,
            dependency_id: DependencyId(dependency_id),
            source_revision_id: SourceRevisionId(source_revision_id),
            derived_revision_id: request.derived_revision_id,
            registered_dependency_generation: u64::try_from(generation)
                .map_err(|_| EngineError::Storage)?,
        }))
    }
}
