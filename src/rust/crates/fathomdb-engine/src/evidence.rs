use std::fmt::{Display, Formatter};

use rusqlite::{Connection, OptionalExtension};

use crate::{
    frozen_read, CanonicalHash, EngineError, FrozenReadContextV1, LifecycleState,
    ProjectionGenerationId, SearchResult, SoftFallbackBranch, SourceDependencyV1, SourceLocator,
};

const SCHEMA_VERSION: u32 = 1;
const TOKEN_PREFIX: &str = "fdbev1";
const TOKEN_MAX_BYTES: usize = 2_048;
const TOKEN_DOMAIN: &[u8] = b"fathomdb.evidence-ref.v1\0";
const DATABASE_DOMAIN: &[u8] = b"fathomdb.evidence.database.v1\0";
const CONTEXT_DOMAIN: &[u8] = b"fathomdb.evidence.context.v1\0";
const ARTIFACT_DOMAIN: &[u8] = b"fathomdb.evidence.artifact.v1\0";
const SOURCE_DOMAIN: &[u8] = b"fathomdb.evidence.source.v1\0";
const LOCATOR_DOMAIN: &[u8] = b"fathomdb.evidence.locator.v1\0";
const HASH_DOMAIN: &[u8] = b"fathomdb.evidence.hash.v1\0";
const GENERATION_DOMAIN: &[u8] = b"fathomdb.evidence.generation.v1\0";

/// Closed canonical artifact class carried by source evidence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceArtifactClassV1 {
    Node,
    Edge,
}

impl EvidenceArtifactClassV1 {
    /// Stable lower-snake-case wire spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Node => "node",
            Self::Edge => "edge",
        }
    }

    fn from_hit_branch(branch: SoftFallbackBranch) -> Self {
        if branch == SoftFallbackBranch::TextEdge {
            Self::Edge
        } else {
            Self::Node
        }
    }

    fn tag(self) -> u8 {
        match self {
            Self::Node => 0,
            Self::Edge => 1,
        }
    }

    fn from_tag(tag: u8) -> Option<Self> {
        match tag {
            0 => Some(Self::Node),
            1 => Some(Self::Edge),
            _ => None,
        }
    }
}

/// Class-correct lifecycle state for an evidence artifact.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EvidenceArtifactLifecycleV1 {
    Node { state: LifecycleState, superseded: bool },
    Edge { superseded: bool, valid_at_effective: bool },
}

/// Closed retrieval-arm vocabulary in an evidence contribution.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceArmV1 {
    Vector,
    Text,
    TextEdge,
    GraphArm,
}

impl EvidenceArmV1 {
    /// Stable lower-snake-case wire spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Vector => "vector",
            Self::Text => "text",
            Self::TextEdge => "text_edge",
            Self::GraphArm => "graph_arm",
        }
    }

    fn from_branch(branch: SoftFallbackBranch) -> Self {
        match branch {
            SoftFallbackBranch::Vector => Self::Vector,
            SoftFallbackBranch::Text => Self::Text,
            SoftFallbackBranch::TextEdge => Self::TextEdge,
            SoftFallbackBranch::GraphArm => Self::GraphArm,
        }
    }

    fn tag(self) -> u8 {
        match self {
            Self::Vector => 0,
            Self::Text => 1,
            Self::TextEdge => 2,
            Self::GraphArm => 3,
        }
    }

    fn from_tag(tag: u8) -> Option<Self> {
        match tag {
            0 => Some(Self::Vector),
            1 => Some(Self::Text),
            2 => Some(Self::TextEdge),
            3 => Some(Self::GraphArm),
            _ => None,
        }
    }
}

/// Compact graph origin. Full path replay is outside schema version 1.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EvidenceGraphOriginV1 {
    EntitySeed,
    EdgeSeed { edge_artifact_revision_id: String },
    Traversal { edge_artifact_revision_id: String, hop_count: u32 },
}

/// Structural origin of one evidence-bearing search result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceProjectionOriginV1 {
    pub schema_version: u32,
    pub artifact_class: EvidenceArtifactClassV1,
    pub representative_arm: EvidenceArmV1,
    pub projection_generation_id: ProjectionGenerationId,
    pub graph_origin: Option<EvidenceGraphOriginV1>,
}

/// Exact ranking contribution copied from the existing explanation calculation.
#[derive(Clone, Debug, PartialEq)]
pub struct EvidenceContributionV1 {
    pub schema_version: u32,
    pub vector_rank: Option<u32>,
    pub text_rank: Option<u32>,
    pub graph_rank: Option<u32>,
    pub fused_score: f64,
    pub ce_score: Option<f64>,
    pub blended_score: f64,
    pub importance: Option<f64>,
    pub confidence: Option<f64>,
}

/// Opaque, database-authenticated evidence reference.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceRefV1(String);

impl EvidenceRefV1 {
    /// Construct an opaque reference carrier.
    ///
    /// Authentication and database binding are checked by
    /// [`crate::Engine::resolve_evidence`].
    pub fn new(value: impl Into<String>) -> Result<Self, EvidenceErrorV1> {
        let value = value.into();
        if value.is_empty() || value.len() > TOKEN_MAX_BYTES {
            return Err(EvidenceErrorV1::unavailable());
        }
        Ok(Self(value))
    }

    /// Return the opaque token text without decoding it.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Versioned request for one opt-in evidence-producing search.
#[derive(Clone, Debug)]
pub struct EvidenceSearchRequestV1 {
    pub schema_version: u32,
    pub query: String,
    pub context: FrozenReadContextV1,
    pub rerank_depth: u32,
    pub use_graph_arm: bool,
    pub alpha: f64,
    pub pool_n: u32,
    pub include_explanation: bool,
    pub limit: u32,
}

/// Position-preserving evidence carrier for one search hit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceSidecarEntryV1 {
    pub schema_version: u32,
    pub result_index: u32,
    pub artifact_revision_id: String,
    pub evidence_ref: EvidenceRefV1,
}

/// Search result plus one evidence reference per hit.
#[derive(Clone, Debug, PartialEq)]
pub struct EvidenceSearchResultV1 {
    pub schema_version: u32,
    pub search_result: SearchResult,
    pub evidence: Vec<EvidenceSidecarEntryV1>,
}

/// Versioned request to resolve one evidence reference.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceResolveRequestV1 {
    pub schema_version: u32,
    pub evidence_ref: EvidenceRefV1,
    pub context: FrozenReadContextV1,
}

/// Exact source bytes and structural metadata authorized by the supplied view.
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedEvidenceV1 {
    pub schema_version: u32,
    pub logical_id: Option<String>,
    pub artifact_revision_id: String,
    pub source_id: String,
    pub source_version_id: String,
    pub source_revision_id: String,
    pub locator: SourceLocator,
    pub canonical_source_body: String,
    pub evidence_text: String,
    pub canonical_source_hash: CanonicalHash,
    pub effective_valid_at: i64,
    pub artifact_lifecycle: EvidenceArtifactLifecycleV1,
    pub source_lifecycle_state: LifecycleState,
    pub projection_origin: EvidenceProjectionOriginV1,
    pub retrieval_contribution: EvidenceContributionV1,
    pub dependency: Option<SourceDependencyV1>,
}

/// Closed evidence-refusal vocabulary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceErrorReasonV1 {
    UnsupportedSchemaVersion,
    UnknownField,
    EvidenceUnavailable,
    EvidenceIncomplete,
    EvidenceCorrupt,
}

impl EvidenceErrorReasonV1 {
    /// Stable lower-snake-case wire spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UnsupportedSchemaVersion => "unsupported_schema_version",
            Self::UnknownField => "unknown_field",
            Self::EvidenceUnavailable => "evidence_unavailable",
            Self::EvidenceIncomplete => "evidence_incomplete",
            Self::EvidenceCorrupt => "evidence_corrupt",
        }
    }
}

/// Typed evidence refusal with a privacy-safe request pointer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceErrorV1 {
    pub reason: EvidenceErrorReasonV1,
    pub field_path: String,
}

impl EvidenceErrorV1 {
    pub(crate) fn new(reason: EvidenceErrorReasonV1, field_path: impl Into<String>) -> Self {
        Self { reason, field_path: field_path.into() }
    }

    pub(crate) fn unavailable() -> Self {
        Self::new(EvidenceErrorReasonV1::EvidenceUnavailable, "/evidenceRef")
    }
}

impl Display for EvidenceErrorV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{} at {}", self.reason.as_str(), self.field_path)
    }
}

impl std::error::Error for EvidenceErrorV1 {}

#[derive(Clone, Debug)]
struct StoredEvidence {
    artifact_revision_id: String,
    completeness: String,
    logical_id: Option<String>,
    artifact_state: Option<String>,
    artifact_superseded: bool,
    edge_valid: bool,
    source_id: String,
    source_version_id: String,
    source_revision_id: String,
    locator: SourceLocator,
    hash_digest: String,
    source_body: String,
    source_state: String,
    source_superseded: bool,
}

#[derive(Clone, Debug)]
struct Payload {
    artifact_class: EvidenceArtifactClassV1,
    write_cursor: u64,
    effective_valid_at: i64,
    database_commitment: [u8; 32],
    context_commitment: [u8; 32],
    artifact_commitment: [u8; 32],
    source_commitment: [u8; 32],
    locator_commitment: [u8; 32],
    hash_commitment: [u8; 32],
    generation_commitment: [u8; 32],
    arm: EvidenceArmV1,
    contribution: EvidenceContributionV1,
}

pub(crate) fn build_search_result(
    connection: &Connection,
    frozen: &FrozenReadContextV1,
    mut search_result: SearchResult,
    include_explanation: bool,
) -> Result<EvidenceSearchResultV1, EngineError> {
    let explanation = search_result.explanation.as_ref().ok_or_else(|| {
        EvidenceErrorV1::new(EvidenceErrorReasonV1::EvidenceCorrupt, "/explanation")
    })?;
    if explanation.per_hit.len() != search_result.results.len() {
        return Err(EvidenceErrorV1::new(
            EvidenceErrorReasonV1::EvidenceCorrupt,
            "/explanation/perHit",
        )
        .into());
    }
    let (database_id, key) = frozen_read::page_cursor_material(connection)?;
    let generation = crate::projection_generation::current_generation_id(connection)?;
    let context_bytes = frozen_read::validate_context(&frozen.context)?;
    let database_commitment = keyed(&key, DATABASE_DOMAIN, database_id.as_bytes());
    let context_commitment = keyed(&key, CONTEXT_DOMAIN, &context_bytes);
    let mut evidence = Vec::with_capacity(search_result.results.len());
    for (index, (hit, per_hit)) in
        search_result.results.iter().zip(explanation.per_hit.iter()).enumerate()
    {
        if per_hit.id != hit.write_cursor {
            return Err(EvidenceErrorV1::new(
                EvidenceErrorReasonV1::EvidenceCorrupt,
                "/explanation/perHit",
            )
            .into());
        }
        let artifact_class = EvidenceArtifactClassV1::from_hit_branch(hit.branch);
        if hit.branch == SoftFallbackBranch::GraphArm {
            return Err(EvidenceErrorV1::new(
                EvidenceErrorReasonV1::EvidenceIncomplete,
                format!("/results/{index}/graphOrigin"),
            )
            .into());
        }
        let stored =
            load_stored(connection, artifact_class, hit.write_cursor, frozen.effective_valid_at)?;
        if stored.completeness != "complete" {
            return Err(EvidenceErrorV1::new(
                EvidenceErrorReasonV1::EvidenceIncomplete,
                format!("/results/{index}/provenance"),
            )
            .into());
        }
        let locator_bytes = locator_bytes(&stored.locator);
        let contribution = contribution(per_hit)?;
        let payload = Payload {
            artifact_class,
            write_cursor: hit.write_cursor,
            effective_valid_at: frozen.effective_valid_at,
            database_commitment,
            context_commitment,
            artifact_commitment: keyed(
                &key,
                ARTIFACT_DOMAIN,
                stored.artifact_revision_id.as_bytes(),
            ),
            source_commitment: keyed(&key, SOURCE_DOMAIN, stored.source_revision_id.as_bytes()),
            locator_commitment: keyed(&key, LOCATOR_DOMAIN, &locator_bytes),
            hash_commitment: keyed(&key, HASH_DOMAIN, stored.hash_digest.as_bytes()),
            generation_commitment: keyed(&key, GENERATION_DOMAIN, generation.as_str().as_bytes()),
            arm: EvidenceArmV1::from_branch(hit.branch),
            contribution,
        };
        let token = encode_token(&key, &payload)?;
        evidence.push(EvidenceSidecarEntryV1 {
            schema_version: SCHEMA_VERSION,
            result_index: u32::try_from(index).map_err(|_| EngineError::Storage)?,
            artifact_revision_id: stored.artifact_revision_id,
            evidence_ref: EvidenceRefV1(token),
        });
    }
    if !include_explanation {
        search_result.explanation = None;
    }
    Ok(EvidenceSearchResultV1 { schema_version: SCHEMA_VERSION, search_result, evidence })
}

pub(crate) fn resolve(
    connection: &Connection,
    request: &EvidenceResolveRequestV1,
) -> Result<ResolvedEvidenceV1, EngineError> {
    if request.schema_version != SCHEMA_VERSION {
        return Err(EvidenceErrorV1::new(
            EvidenceErrorReasonV1::UnsupportedSchemaVersion,
            "/schemaVersion",
        )
        .into());
    }
    let (database_id, key) = frozen_read::page_cursor_material(connection)?;
    let payload = decode_token(&key, request.evidence_ref.as_str())?;
    let context_bytes = frozen_read::validate_context(&request.context.context)?;
    if payload.database_commitment != keyed(&key, DATABASE_DOMAIN, database_id.as_bytes())
        || payload.context_commitment != keyed(&key, CONTEXT_DOMAIN, &context_bytes)
        || payload.effective_valid_at != request.context.effective_valid_at
    {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    let stored = load_stored(
        connection,
        payload.artifact_class,
        payload.write_cursor,
        payload.effective_valid_at,
    )
    .map_err(|_| EngineError::Evidence(EvidenceErrorV1::unavailable()))?;
    let generation = crate::projection_generation::current_generation_id(connection)?;
    if payload.artifact_commitment
        != keyed(&key, ARTIFACT_DOMAIN, stored.artifact_revision_id.as_bytes())
        || payload.source_commitment
            != keyed(&key, SOURCE_DOMAIN, stored.source_revision_id.as_bytes())
        || payload.locator_commitment
            != keyed(&key, LOCATOR_DOMAIN, &locator_bytes(&stored.locator))
        || payload.hash_commitment != keyed(&key, HASH_DOMAIN, stored.hash_digest.as_bytes())
        || payload.generation_commitment
            != keyed(&key, GENERATION_DOMAIN, generation.as_str().as_bytes())
    {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    if stored.completeness != "complete" {
        return Err(
            EvidenceErrorV1::new(EvidenceErrorReasonV1::EvidenceIncomplete, "/provenance").into()
        );
    }
    if stored.artifact_superseded || stored.source_superseded {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    let source_state = LifecycleState::from_str_opt(&stored.source_state)
        .filter(|state| *state == LifecycleState::Active)
        .ok_or_else(EvidenceErrorV1::unavailable)?;
    let artifact_lifecycle = match payload.artifact_class {
        EvidenceArtifactClassV1::Node => {
            let state = stored
                .artifact_state
                .as_deref()
                .and_then(LifecycleState::from_str_opt)
                .filter(|state| *state == LifecycleState::Active)
                .ok_or_else(EvidenceErrorV1::unavailable)?;
            EvidenceArtifactLifecycleV1::Node { state, superseded: false }
        }
        EvidenceArtifactClassV1::Edge => {
            if !stored.edge_valid {
                return Err(EvidenceErrorV1::unavailable().into());
            }
            EvidenceArtifactLifecycleV1::Edge { superseded: false, valid_at_effective: true }
        }
    };
    let canonical_source_hash =
        CanonicalHash::sha256(stored.hash_digest.clone()).map_err(|_| {
            EvidenceErrorV1::new(EvidenceErrorReasonV1::EvidenceCorrupt, "/canonicalSourceHash")
        })?;
    let actual_hash = crate::canonical_body_hash(&stored.source_body);
    if actual_hash != stored.hash_digest {
        return Err(EvidenceErrorV1::new(
            EvidenceErrorReasonV1::EvidenceCorrupt,
            "/canonicalSourceHash",
        )
        .into());
    }
    let evidence_text = slice(&stored.source_body, &stored.locator)?;
    Ok(ResolvedEvidenceV1 {
        schema_version: SCHEMA_VERSION,
        logical_id: stored.logical_id,
        artifact_revision_id: stored.artifact_revision_id,
        source_id: stored.source_id,
        source_version_id: stored.source_version_id,
        source_revision_id: stored.source_revision_id,
        locator: stored.locator,
        canonical_source_body: stored.source_body,
        evidence_text,
        canonical_source_hash,
        effective_valid_at: payload.effective_valid_at,
        artifact_lifecycle,
        source_lifecycle_state: source_state,
        projection_origin: EvidenceProjectionOriginV1 {
            schema_version: SCHEMA_VERSION,
            artifact_class: payload.artifact_class,
            representative_arm: payload.arm,
            projection_generation_id: generation,
            graph_origin: None,
        },
        retrieval_contribution: payload.contribution,
        dependency: None,
    })
}

fn contribution(per_hit: &crate::PerHitExplain) -> Result<EvidenceContributionV1, EngineError> {
    let values = [
        Some(per_hit.fused_score),
        per_hit.ce_score,
        Some(per_hit.blended),
        per_hit.importance,
        per_hit.confidence,
    ];
    if values.into_iter().flatten().any(|value| !value.is_finite()) {
        return Err(EvidenceErrorV1::new(
            EvidenceErrorReasonV1::EvidenceCorrupt,
            "/explanation/perHit",
        )
        .into());
    }
    Ok(EvidenceContributionV1 {
        schema_version: SCHEMA_VERSION,
        vector_rank: per_hit.vector_rank,
        text_rank: per_hit.text_rank,
        graph_rank: per_hit.graph_rank,
        fused_score: per_hit.fused_score,
        ce_score: per_hit.ce_score,
        blended_score: per_hit.blended,
        importance: per_hit.importance,
        confidence: per_hit.confidence,
    })
}

fn load_stored(
    connection: &Connection,
    class: EvidenceArtifactClassV1,
    cursor: u64,
    effective: i64,
) -> Result<StoredEvidence, EngineError> {
    let cursor = i64::try_from(cursor).map_err(|_| EvidenceErrorV1::unavailable())?;
    let artifact: Option<(String, String)> = connection
        .query_row(
            "SELECT revision_id,completeness FROM _fathomdb_artifact_revisions \
             WHERE artifact_class=?1 AND write_cursor=?2",
            rusqlite::params![class.as_str(), cursor],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let (artifact_revision_id, completeness) = artifact.ok_or_else(EvidenceErrorV1::unavailable)?;
    let link: Option<(String, String, String, String, Option<i64>, Option<i64>, String)> =
        connection
            .query_row(
                "SELECT source_id,source_version_id,source_revision_id,locator_kind,\
                    start_byte,end_byte,hash_digest FROM _fathomdb_source_links \
             WHERE artifact_revision_id=?1",
                [&artifact_revision_id],
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
            .optional()
            .map_err(|_| EngineError::Storage)?;
    let (source_id, source_version_id, source_revision_id, locator_kind, start, end, hash_digest) =
        link.ok_or_else(|| {
            EvidenceErrorV1::new(EvidenceErrorReasonV1::EvidenceIncomplete, "/provenance")
        })?;
    let locator = match (locator_kind.as_str(), start, end) {
        ("whole_body", None, None) => SourceLocator::WholeBody,
        ("utf8_bytes", Some(start), Some(end)) if start >= 0 && end >= 0 => {
            SourceLocator::utf8_bytes(start as u64, end as u64)
        }
        _ => {
            return Err(EvidenceErrorV1::new(
                EvidenceErrorReasonV1::EvidenceCorrupt,
                "/provenance/sourceLocator",
            )
            .into())
        }
    };
    let source: Option<(String, String, Option<i64>)> = connection
        .query_row(
            "SELECT n.body,n.state,n.superseded_at FROM _fathomdb_artifact_revisions ar \
             JOIN canonical_nodes n ON n.write_cursor=ar.write_cursor \
             WHERE ar.revision_id=?1 AND ar.artifact_class='node' \
               AND ar.artifact_role='canonical_source'",
            [&source_revision_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let (source_body, source_state, source_superseded_at) =
        source.ok_or_else(EvidenceErrorV1::unavailable)?;
    let (logical_id, artifact_state, artifact_superseded, edge_valid) = match class {
        EvidenceArtifactClassV1::Node => {
            let row: Option<(Option<String>, String, Option<i64>, Option<i64>, Option<i64>)> =
                connection
                    .query_row(
                        "SELECT logical_id,state,superseded_at,valid_from,valid_until \
                         FROM canonical_nodes WHERE write_cursor=?1",
                        [cursor],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
                    )
                    .optional()
                    .map_err(|_| EngineError::Storage)?;
            let (logical, state, superseded, valid_from, valid_until) =
                row.ok_or_else(EvidenceErrorV1::unavailable)?;
            let valid = valid_from.is_none_or(|start| start <= effective)
                && valid_until.is_none_or(|end| end > effective);
            if !valid {
                return Err(EvidenceErrorV1::unavailable().into());
            }
            (logical, Some(state), superseded.is_some(), true)
        }
        EvidenceArtifactClassV1::Edge => {
            let row: Option<(Option<String>, Option<i64>, Option<i64>, Option<i64>)> = connection
                .query_row(
                    "SELECT logical_id,superseded_at,t_valid,t_invalid FROM canonical_edges \
                     WHERE write_cursor=?1",
                    [cursor],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                )
                .optional()
                .map_err(|_| EngineError::Storage)?;
            let (logical, superseded, valid_from, valid_until) =
                row.ok_or_else(EvidenceErrorV1::unavailable)?;
            let valid = valid_from.is_none_or(|start| start <= effective)
                && valid_until.is_none_or(|end| end > effective);
            (logical, None, superseded.is_some(), valid)
        }
    };
    Ok(StoredEvidence {
        artifact_revision_id,
        completeness,
        logical_id,
        artifact_state,
        artifact_superseded,
        edge_valid,
        source_id,
        source_version_id,
        source_revision_id,
        locator,
        hash_digest,
        source_body,
        source_state,
        source_superseded: source_superseded_at.is_some(),
    })
}

fn keyed(key: &[u8], domain: &[u8], value: &[u8]) -> [u8; 32] {
    frozen_read::hmac_sha256(key, domain, value)
}

fn locator_bytes(locator: &SourceLocator) -> Vec<u8> {
    match locator {
        SourceLocator::WholeBody => vec![0],
        SourceLocator::Utf8Bytes { start_inclusive, end_exclusive } => {
            let mut bytes = vec![1];
            bytes.extend_from_slice(&start_inclusive.to_be_bytes());
            bytes.extend_from_slice(&end_exclusive.to_be_bytes());
            bytes
        }
    }
}

fn slice(body: &str, locator: &SourceLocator) -> Result<String, EngineError> {
    match locator {
        SourceLocator::WholeBody => Ok(body.to_string()),
        SourceLocator::Utf8Bytes { start_inclusive, end_exclusive } => {
            let start = usize::try_from(*start_inclusive).map_err(|_| corrupt_locator())?;
            let end = usize::try_from(*end_exclusive).map_err(|_| corrupt_locator())?;
            body.get(start..end).map(ToString::to_string).ok_or_else(corrupt_locator)
        }
    }
}

fn corrupt_locator() -> EngineError {
    EvidenceErrorV1::new(EvidenceErrorReasonV1::EvidenceCorrupt, "/provenance/sourceLocator").into()
}

fn encode_token(key: &[u8], payload: &Payload) -> Result<String, EngineError> {
    let bytes = encode_payload(payload);
    let mac = frozen_read::hmac_sha256(key, TOKEN_DOMAIN, &bytes);
    let token = format!(
        "{TOKEN_PREFIX}.{}.{}",
        frozen_read::hex_encode(&bytes),
        frozen_read::hex_encode(&mac)
    );
    if token.len() > TOKEN_MAX_BYTES {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    Ok(token)
}

fn encode_payload(payload: &Payload) -> Vec<u8> {
    let mut bytes = Vec::new();
    frozen_read::encode_u32(&mut bytes, SCHEMA_VERSION);
    bytes.push(payload.artifact_class.tag());
    frozen_read::encode_u64(&mut bytes, payload.write_cursor);
    frozen_read::encode_i64(&mut bytes, payload.effective_valid_at);
    for commitment in [
        payload.database_commitment,
        payload.context_commitment,
        payload.artifact_commitment,
        payload.source_commitment,
        payload.locator_commitment,
        payload.hash_commitment,
        payload.generation_commitment,
    ] {
        bytes.extend_from_slice(&commitment);
    }
    bytes.push(payload.arm.tag());
    encode_optional_u32(&mut bytes, payload.contribution.vector_rank);
    encode_optional_u32(&mut bytes, payload.contribution.text_rank);
    encode_optional_u32(&mut bytes, payload.contribution.graph_rank);
    encode_f64(&mut bytes, payload.contribution.fused_score);
    encode_optional_f64(&mut bytes, payload.contribution.ce_score);
    encode_f64(&mut bytes, payload.contribution.blended_score);
    encode_optional_f64(&mut bytes, payload.contribution.importance);
    encode_optional_f64(&mut bytes, payload.contribution.confidence);
    bytes.push(0); // graph origin: none in the first GREEN increment
    bytes
}

fn decode_token(key: &[u8], token: &str) -> Result<Payload, EngineError> {
    if token.len() > TOKEN_MAX_BYTES {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    let mut parts = token.split('.');
    let (Some(prefix), Some(payload_hex), Some(mac_hex), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(EvidenceErrorV1::unavailable().into());
    };
    if prefix != TOKEN_PREFIX {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    let bytes = frozen_read::hex_decode(payload_hex).ok_or_else(EvidenceErrorV1::unavailable)?;
    let mac = frozen_read::hex_decode(mac_hex).ok_or_else(EvidenceErrorV1::unavailable)?;
    let expected = frozen_read::hmac_sha256(key, TOKEN_DOMAIN, &bytes);
    if !frozen_read::constant_time_eq(&mac, &expected) {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    let payload = decode_payload(&bytes)?;
    if encode_payload(&payload) != bytes {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    Ok(payload)
}

fn decode_payload(bytes: &[u8]) -> Result<Payload, EngineError> {
    let mut cursor = PayloadCursor { bytes, offset: 0 };
    if cursor.u32()? != SCHEMA_VERSION {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    let artifact_class =
        EvidenceArtifactClassV1::from_tag(cursor.u8()?).ok_or_else(EvidenceErrorV1::unavailable)?;
    let write_cursor = cursor.u64()?;
    let effective_valid_at = cursor.i64()?;
    let database_commitment = cursor.array32()?;
    let context_commitment = cursor.array32()?;
    let artifact_commitment = cursor.array32()?;
    let source_commitment = cursor.array32()?;
    let locator_commitment = cursor.array32()?;
    let hash_commitment = cursor.array32()?;
    let generation_commitment = cursor.array32()?;
    let arm = EvidenceArmV1::from_tag(cursor.u8()?).ok_or_else(EvidenceErrorV1::unavailable)?;
    let contribution = EvidenceContributionV1 {
        schema_version: SCHEMA_VERSION,
        vector_rank: cursor.optional_u32()?,
        text_rank: cursor.optional_u32()?,
        graph_rank: cursor.optional_u32()?,
        fused_score: cursor.f64()?,
        ce_score: cursor.optional_f64()?,
        blended_score: cursor.f64()?,
        importance: cursor.optional_f64()?,
        confidence: cursor.optional_f64()?,
    };
    if cursor.u8()? != 0 || cursor.offset != bytes.len() {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    Ok(Payload {
        artifact_class,
        write_cursor,
        effective_valid_at,
        database_commitment,
        context_commitment,
        artifact_commitment,
        source_commitment,
        locator_commitment,
        hash_commitment,
        generation_commitment,
        arm,
        contribution,
    })
}

fn encode_optional_u32(bytes: &mut Vec<u8>, value: Option<u32>) {
    match value {
        Some(value) => {
            bytes.push(1);
            frozen_read::encode_u32(bytes, value);
        }
        None => bytes.push(0),
    }
}

fn encode_f64(bytes: &mut Vec<u8>, value: f64) {
    bytes.extend_from_slice(&value.to_bits().to_be_bytes());
}

fn encode_optional_f64(bytes: &mut Vec<u8>, value: Option<f64>) {
    match value {
        Some(value) => {
            bytes.push(1);
            encode_f64(bytes, value);
        }
        None => bytes.push(0),
    }
}

struct PayloadCursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl PayloadCursor<'_> {
    fn take(&mut self, count: usize) -> Result<&[u8], EngineError> {
        let end = self.offset.checked_add(count).ok_or_else(EvidenceErrorV1::unavailable)?;
        let value = self.bytes.get(self.offset..end).ok_or_else(EvidenceErrorV1::unavailable)?;
        self.offset = end;
        Ok(value)
    }

    fn u8(&mut self) -> Result<u8, EngineError> {
        Ok(self.take(1)?[0])
    }

    fn u32(&mut self) -> Result<u32, EngineError> {
        Ok(u32::from_be_bytes(
            self.take(4)?.try_into().map_err(|_| EvidenceErrorV1::unavailable())?,
        ))
    }

    fn u64(&mut self) -> Result<u64, EngineError> {
        Ok(u64::from_be_bytes(
            self.take(8)?.try_into().map_err(|_| EvidenceErrorV1::unavailable())?,
        ))
    }

    fn i64(&mut self) -> Result<i64, EngineError> {
        Ok(i64::from_be_bytes(
            self.take(8)?.try_into().map_err(|_| EvidenceErrorV1::unavailable())?,
        ))
    }

    fn array32(&mut self) -> Result<[u8; 32], EngineError> {
        self.take(32)?.try_into().map_err(|_| EvidenceErrorV1::unavailable().into())
    }

    fn optional_u32(&mut self) -> Result<Option<u32>, EngineError> {
        match self.u8()? {
            0 => Ok(None),
            1 => Ok(Some(self.u32()?)),
            _ => Err(EvidenceErrorV1::unavailable().into()),
        }
    }

    fn f64(&mut self) -> Result<f64, EngineError> {
        let value = f64::from_bits(self.u64()?);
        if value.is_finite() {
            Ok(value)
        } else {
            Err(EvidenceErrorV1::unavailable().into())
        }
    }

    fn optional_f64(&mut self) -> Result<Option<f64>, EngineError> {
        match self.u8()? {
            0 => Ok(None),
            1 => Ok(Some(self.f64()?)),
            _ => Err(EvidenceErrorV1::unavailable().into()),
        }
    }
}
