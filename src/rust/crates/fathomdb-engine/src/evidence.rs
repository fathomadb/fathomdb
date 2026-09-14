use std::fmt::{Display, Formatter};

use rusqlite::{Connection, OptionalExtension};

use crate::{
    frozen_read, ArtifactRevisionId, CanonicalHash, DependencyId, EngineError, FrozenReadContextV1,
    LifecycleState, ProjectionGenerationId, SearchResult, SoftFallbackBranch, SourceDependencyV1,
    SourceLocator, SourceRevisionId,
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
const GENERATION_TAIL_DOMAIN: &[u8] = b"fathomdb.evidence.generation-tail.v1\0";
const GRAPH_EDGE_DOMAIN: &[u8] = b"fathomdb.evidence.graph-edge.v1\0";

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
    artifact_kind: String,
    artifact_state: Option<String>,
    artifact_superseded: bool,
    edge_valid: bool,
    source_id: String,
    source_version_id: String,
    source_revision_id: String,
    locator: SourceLocator,
    hash_digest: String,
    source_body: String,
    source_cursor: u64,
    source_kind: String,
    source_state: String,
    source_superseded: bool,
}

type SourceLinkRow = (String, String, String, String, Option<i64>, Option<i64>, String);
type SourceArtifactRow = (String, String, Option<i64>, i64, String, Option<i64>, Option<i64>);
type NodeArtifactRow = (Option<String>, String, String, Option<i64>, Option<i64>, Option<i64>);
type EdgeArtifactRow = (Option<String>, String, Option<i64>, Option<i64>, Option<i64>);

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
    generation_nonce: [u8; 16],
    generation_ciphertext: Vec<u8>,
    arm: EvidenceArmV1,
    contribution: EvidenceContributionV1,
    graph_origin: Option<crate::CapturedGraphOrigin>,
    graph_edge_commitment: Option<[u8; 32]>,
}

pub(crate) fn build_search_result(
    connection: &Connection,
    frozen: &FrozenReadContextV1,
    mut search_result: SearchResult,
    include_explanation: bool,
    graph_origins: &std::collections::HashMap<u64, crate::CapturedGraphOrigin>,
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
        let graph_origin = if hit.branch == SoftFallbackBranch::GraphArm {
            Some(graph_origins.get(&hit.write_cursor).cloned().ok_or_else(|| {
                EvidenceErrorV1::new(
                    EvidenceErrorReasonV1::EvidenceIncomplete,
                    format!("/results/{index}/graphOrigin"),
                )
            })?)
        } else {
            None
        };
        if matches!(graph_origin, Some(crate::CapturedGraphOrigin::EntitySeed)) {
            return Err(EvidenceErrorV1::new(
                EvidenceErrorReasonV1::EvidenceCorrupt,
                format!("/results/{index}/graphOrigin"),
            )
            .into());
        }
        let graph_edge_commitment = match graph_origin.as_ref() {
            Some(crate::CapturedGraphOrigin::EdgeSeed { edge_cursor })
            | Some(crate::CapturedGraphOrigin::Traversal { edge_cursor, .. }) => {
                let (revision, _, _, _, source_revision) = load_graph_edge_revision(
                    connection,
                    *edge_cursor,
                    frozen.effective_valid_at,
                    frozen.context.view.include_out_of_window,
                )?;
                crate::validate_dependency_chain(
                    connection,
                    &source_revision,
                    &revision,
                    crate::DependencyValidationMode::Persisted,
                )
                .map_err(|_| {
                    EngineError::Evidence(EvidenceErrorV1::new(
                        EvidenceErrorReasonV1::EvidenceCorrupt,
                        format!("/results/{index}/graphOrigin"),
                    ))
                })?;
                Some(keyed(&key, GRAPH_EDGE_DOMAIN, revision.as_bytes()))
            }
            _ => None,
        };
        let stored = load_stored(
            connection,
            artifact_class,
            hit.write_cursor,
            frozen.effective_valid_at,
            frozen.context.view.include_out_of_window,
        )?;
        if stored.completeness != "complete" {
            return Err(EvidenceErrorV1::new(
                EvidenceErrorReasonV1::EvidenceIncomplete,
                format!("/results/{index}/provenance"),
            )
            .into());
        }
        validate_source_bytes(&stored).map_err(|_| {
            EngineError::Evidence(EvidenceErrorV1::new(
                EvidenceErrorReasonV1::EvidenceCorrupt,
                format!("/results/{index}/provenance"),
            ))
        })?;
        validate_full_provenance(connection, &stored).map_err(|_| {
            EngineError::Evidence(EvidenceErrorV1::new(
                EvidenceErrorReasonV1::EvidenceCorrupt,
                format!("/results/{index}/provenance"),
            ))
        })?;
        load_dependency(connection, &stored.artifact_revision_id).map_err(|_| {
            EngineError::Evidence(EvidenceErrorV1::new(
                EvidenceErrorReasonV1::EvidenceCorrupt,
                format!("/results/{index}/provenance"),
            ))
        })?;
        let locator_bytes = locator_bytes(&stored.locator);
        let contribution = contribution(per_hit)?;
        let generation_nonce = random_nonce(connection)?;
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
            generation_nonce,
            generation_ciphertext: protect_generation(
                &key,
                &generation_nonce,
                generation.as_str().as_bytes(),
            ),
            arm: EvidenceArmV1::from_branch(hit.branch),
            contribution,
            graph_origin,
            graph_edge_commitment,
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
        request.context.context.view.include_out_of_window,
    )
    .map_err(|_| EngineError::Evidence(EvidenceErrorV1::unavailable()))?;
    if payload.artifact_commitment
        != keyed(&key, ARTIFACT_DOMAIN, stored.artifact_revision_id.as_bytes())
        || payload.source_commitment
            != keyed(&key, SOURCE_DOMAIN, stored.source_revision_id.as_bytes())
        || payload.locator_commitment
            != keyed(&key, LOCATOR_DOMAIN, &locator_bytes(&stored.locator))
        || payload.hash_commitment != keyed(&key, HASH_DOMAIN, stored.hash_digest.as_bytes())
    {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    let (graph_origin, graph_provenance) =
        match (&payload.graph_origin, payload.graph_edge_commitment) {
            (None, None) if payload.arm != EvidenceArmV1::GraphArm => (None, None),
            (Some(crate::CapturedGraphOrigin::EdgeSeed { edge_cursor }), Some(commitment))
                if payload.arm == EvidenceArmV1::GraphArm =>
            {
                let (revision, from_id, to_id, _, source_revision_id) = load_graph_edge_revision(
                    connection,
                    *edge_cursor,
                    payload.effective_valid_at,
                    request.context.context.view.include_out_of_window,
                )
                .map_err(|_| EngineError::Evidence(EvidenceErrorV1::unavailable()))?;
                if commitment != keyed(&key, GRAPH_EDGE_DOMAIN, revision.as_bytes())
                    || stored
                        .logical_id
                        .as_ref()
                        .is_none_or(|logical| logical != &from_id && logical != &to_id)
                    || !source_revision_is_eligible(
                        connection,
                        &source_revision_id,
                        payload.effective_valid_at,
                        request.context.context.view.include_out_of_window,
                    )?
                {
                    return Err(EvidenceErrorV1::unavailable().into());
                }
                (
                    Some(EvidenceGraphOriginV1::EdgeSeed {
                        edge_artifact_revision_id: revision.clone(),
                    }),
                    Some((revision, source_revision_id)),
                )
            }
            (
                Some(crate::CapturedGraphOrigin::Traversal { edge_cursor, hop_count }),
                Some(commitment),
            ) if payload.arm == EvidenceArmV1::GraphArm => {
                let (revision, from_id, to_id, _, source_revision_id) = load_graph_edge_revision(
                    connection,
                    *edge_cursor,
                    payload.effective_valid_at,
                    request.context.context.view.include_out_of_window,
                )
                .map_err(|_| EngineError::Evidence(EvidenceErrorV1::unavailable()))?;
                if commitment != keyed(&key, GRAPH_EDGE_DOMAIN, revision.as_bytes())
                    || stored
                        .logical_id
                        .as_ref()
                        .is_none_or(|logical| logical != &from_id && logical != &to_id)
                    || !source_revision_is_eligible(
                        connection,
                        &source_revision_id,
                        payload.effective_valid_at,
                        request.context.context.view.include_out_of_window,
                    )?
                {
                    return Err(EvidenceErrorV1::unavailable().into());
                }
                (
                    Some(EvidenceGraphOriginV1::Traversal {
                        edge_artifact_revision_id: revision.clone(),
                        hop_count: *hop_count,
                    }),
                    Some((revision, source_revision_id)),
                )
            }
            _ => return Err(EvidenceErrorV1::unavailable().into()),
        };
    if stored.artifact_superseded || stored.source_superseded {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    if crate::dependency_closure::active_barrier_for_source(connection, &stored.source_revision_id)?
    {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    let eligibility = &request.context.context.eligibility;
    let artifact_eligible = match payload.artifact_class {
        EvidenceArtifactClassV1::Node => crate::text_hit_passes_filter(
            connection,
            payload.write_cursor,
            &stored.artifact_kind,
            Some(eligibility),
        ),
        EvidenceArtifactClassV1::Edge => crate::edge_fts_hit_passes_filter(
            connection,
            payload.write_cursor,
            &stored.artifact_kind,
            Some(eligibility),
        ),
    }
    .map_err(|_| EngineError::Storage)?;
    if !artifact_eligible {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    // A source is a separately authorized canonical artifact. Domain-oriented
    // kind/source-type constraints select the returned artifact, while access-
    // bearing metadata and declared attributes constrain both artifact and bytes.
    let mut source_filter = eligibility.clone();
    source_filter.kind = None;
    source_filter.source_type = None;
    if !crate::text_hit_passes_filter(
        connection,
        stored.source_cursor,
        &stored.source_kind,
        Some(&source_filter),
    )
    .map_err(|_| EngineError::Storage)?
    {
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
    if stored.completeness != "complete" {
        return Err(
            EvidenceErrorV1::new(EvidenceErrorReasonV1::EvidenceIncomplete, "/provenance").into()
        );
    }
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
    validate_full_provenance(connection, &stored)?;
    if let Some((edge_revision, source_revision)) = graph_provenance {
        crate::validate_dependency_chain(
            connection,
            &source_revision,
            &edge_revision,
            crate::DependencyValidationMode::Persisted,
        )
        .map_err(|_| {
            EngineError::Evidence(EvidenceErrorV1::new(
                EvidenceErrorReasonV1::EvidenceCorrupt,
                "/projectionOrigin/graphOrigin",
            ))
        })?;
    }
    let generation = resolve_generation(
        connection,
        &key,
        &payload.generation_nonce,
        &payload.generation_ciphertext,
    )?;
    let dependency = load_dependency(connection, &stored.artifact_revision_id)?;
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
            graph_origin,
        },
        retrieval_contribution: payload.contribution,
        dependency,
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
    include_out_of_window: bool,
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
    let link: Option<SourceLinkRow> = connection
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
    let source: Option<SourceArtifactRow> = connection
        .query_row(
            "SELECT n.body,n.state,n.superseded_at,n.write_cursor,n.kind,n.valid_from,n.valid_until \
             FROM _fathomdb_artifact_revisions ar \
             JOIN canonical_nodes n ON n.write_cursor=ar.write_cursor \
             WHERE ar.revision_id=?1 AND ar.artifact_class='node' \
               AND ar.artifact_role='canonical_source'",
            [&source_revision_id],
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
    let (
        source_body,
        source_state,
        source_superseded_at,
        source_cursor,
        source_kind,
        source_valid_from,
        source_valid_until,
    ) = source.ok_or_else(EvidenceErrorV1::unavailable)?;
    if !include_out_of_window
        && (source_valid_from.is_some_and(|start| start > effective)
            || source_valid_until.is_some_and(|end| end <= effective))
    {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    let (logical_id, artifact_kind, artifact_state, artifact_superseded, edge_valid) = match class {
        EvidenceArtifactClassV1::Node => {
            let row: Option<NodeArtifactRow> = connection
                .query_row(
                    "SELECT logical_id,kind,state,superseded_at,valid_from,valid_until \
                         FROM canonical_nodes WHERE write_cursor=?1",
                    [cursor],
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
                .optional()
                .map_err(|_| EngineError::Storage)?;
            let (logical, kind, state, superseded, valid_from, valid_until) =
                row.ok_or_else(EvidenceErrorV1::unavailable)?;
            let valid = valid_from.is_none_or(|start| start <= effective)
                && valid_until.is_none_or(|end| end > effective);
            if !include_out_of_window && !valid {
                return Err(EvidenceErrorV1::unavailable().into());
            }
            (logical, kind, Some(state), superseded.is_some(), true)
        }
        EvidenceArtifactClassV1::Edge => {
            let row: Option<EdgeArtifactRow> = connection
                .query_row(
                    "SELECT logical_id,kind,superseded_at,t_valid,t_invalid FROM canonical_edges \
                     WHERE write_cursor=?1",
                    [cursor],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
                )
                .optional()
                .map_err(|_| EngineError::Storage)?;
            let (logical, kind, superseded, valid_from, valid_until) =
                row.ok_or_else(EvidenceErrorV1::unavailable)?;
            let valid = include_out_of_window
                || (valid_from.is_none_or(|start| start <= effective)
                    && valid_until.is_none_or(|end| end > effective));
            (logical, kind, None, superseded.is_some(), valid)
        }
    };
    Ok(StoredEvidence {
        artifact_revision_id,
        completeness,
        logical_id,
        artifact_kind,
        artifact_state,
        artifact_superseded,
        edge_valid,
        source_id,
        source_version_id,
        source_revision_id,
        locator,
        hash_digest,
        source_body,
        source_cursor: u64::try_from(source_cursor).map_err(|_| EngineError::Storage)?,
        source_kind,
        source_state,
        source_superseded: source_superseded_at.is_some(),
    })
}

fn load_graph_edge_revision(
    connection: &Connection,
    cursor: u64,
    effective: i64,
    include_out_of_window: bool,
) -> Result<(String, String, String, String, String), EngineError> {
    let cursor = i64::try_from(cursor).map_err(|_| EvidenceErrorV1::unavailable())?;
    connection
        .query_row(
            "SELECT ar.revision_id,e.from_id,e.to_id,e.kind,l.source_revision_id \
             FROM canonical_edges e \
             JOIN _fathomdb_artifact_revisions ar \
               ON ar.write_cursor=e.write_cursor AND ar.artifact_class='edge' \
             JOIN _fathomdb_source_links l ON l.artifact_revision_id=ar.revision_id \
             WHERE e.write_cursor=?1 AND e.superseded_at IS NULL \
               AND (?3=1 OR ((e.t_valid IS NULL OR e.t_valid<=?2) \
                             AND (e.t_invalid IS NULL OR e.t_invalid>?2))) \
               AND (e.temporal_fallback IS NULL OR e.temporal_fallback=0)",
            rusqlite::params![cursor, effective, i64::from(include_out_of_window)],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        )
        .optional()
        .map_err(|_| EngineError::Storage)?
        .ok_or_else(|| EvidenceErrorV1::unavailable().into())
}

fn source_revision_is_eligible(
    connection: &Connection,
    source_revision_id: &str,
    effective: i64,
    include_out_of_window: bool,
) -> Result<bool, EngineError> {
    if crate::dependency_closure::active_barrier_for_source(connection, source_revision_id)? {
        return Ok(false);
    }
    connection
        .query_row(
            "SELECT EXISTS(\
               SELECT 1 FROM _fathomdb_artifact_revisions ar \
               JOIN canonical_nodes n ON n.write_cursor=ar.write_cursor \
               WHERE ar.revision_id=?1 AND ar.artifact_class='node' \
                 AND ar.artifact_role='canonical_source' AND ar.completeness='complete' \
                 AND n.superseded_at IS NULL AND n.state='active' \
                 AND (?3=1 OR ((n.valid_from IS NULL OR n.valid_from<=?2) \
                               AND (n.valid_until IS NULL OR n.valid_until>?2)))\
             )",
            rusqlite::params![source_revision_id, effective, i64::from(include_out_of_window)],
            |row| row.get(0),
        )
        .map_err(|_| EngineError::Storage)
}

fn resolve_generation(
    connection: &Connection,
    key: &[u8],
    nonce: &[u8; 16],
    ciphertext: &[u8],
) -> Result<ProjectionGenerationId, EngineError> {
    let plaintext = protect_generation(key, nonce, ciphertext);
    let generation = String::from_utf8(plaintext).map_err(|_| EvidenceErrorV1::unavailable())?;
    let generation = ProjectionGenerationId::new(generation)
        .map_err(|_| EngineError::Evidence(EvidenceErrorV1::unavailable()))?;
    let exists: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM _fathomdb_projection_generations \
             WHERE generation_id=?1)",
            [generation.as_str()],
            |row| row.get(0),
        )
        .map_err(|_| EngineError::Storage)?;
    if !exists {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    Ok(generation)
}

fn validate_source_bytes(stored: &StoredEvidence) -> Result<(), EngineError> {
    CanonicalHash::sha256(stored.hash_digest.clone()).map_err(|_| {
        EvidenceErrorV1::new(EvidenceErrorReasonV1::EvidenceCorrupt, "/canonicalSourceHash")
    })?;
    if crate::canonical_body_hash(&stored.source_body) != stored.hash_digest {
        return Err(EvidenceErrorV1::new(
            EvidenceErrorReasonV1::EvidenceCorrupt,
            "/canonicalSourceHash",
        )
        .into());
    }
    slice(&stored.source_body, &stored.locator).map(|_| ())
}

fn protect_generation(key: &[u8], nonce: &[u8; 16], value: &[u8]) -> Vec<u8> {
    let first = frozen_read::hmac_sha256(key, GENERATION_DOMAIN, nonce);
    let second = frozen_read::hmac_sha256(key, GENERATION_TAIL_DOMAIN, nonce);
    value.iter().zip(first.iter().chain(second.iter())).map(|(value, mask)| value ^ mask).collect()
}

fn random_nonce(connection: &Connection) -> Result<[u8; 16], EngineError> {
    let bytes: Vec<u8> = connection
        .query_row("SELECT randomblob(16)", [], |row| row.get(0))
        .map_err(|_| EngineError::Storage)?;
    bytes.try_into().map_err(|_| EngineError::Storage)
}

fn validate_full_provenance(
    connection: &Connection,
    stored: &StoredEvidence,
) -> Result<(), EngineError> {
    let validation = if stored.artifact_revision_id == stored.source_revision_id {
        crate::load_persisted_canonical_source(connection, &stored.source_revision_id)
            .and_then(|source| source.ok_or(EngineError::Storage).map(|_| ()))
    } else {
        crate::validate_dependency_chain(
            connection,
            &stored.source_revision_id,
            &stored.artifact_revision_id,
            crate::DependencyValidationMode::Persisted,
        )
    };
    validation.map_err(|_| {
        EngineError::Evidence(EvidenceErrorV1::new(
            EvidenceErrorReasonV1::EvidenceCorrupt,
            "/provenance",
        ))
    })
}

fn load_dependency(
    connection: &Connection,
    artifact_revision_id: &str,
) -> Result<Option<SourceDependencyV1>, EngineError> {
    let row: Option<(i64, String, i64, String)> = connection
        .query_row(
            "SELECT d.schema_version,d.dependency_id,d.registered_dependency_generation,\
                    l.source_revision_id \
             FROM _fathomdb_source_dependencies d \
             LEFT JOIN _fathomdb_source_links l \
               ON l.artifact_revision_id=d.derived_revision_id \
             WHERE d.derived_revision_id=?1",
            [artifact_revision_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let Some((schema, dependency_id, generation, source_revision_id)) = row else {
        return Ok(None);
    };
    crate::validate_persisted_dependency_row(
        connection,
        schema,
        &dependency_id,
        artifact_revision_id,
        generation,
    )
    .map_err(|_| {
        EngineError::Evidence(EvidenceErrorV1::new(
            EvidenceErrorReasonV1::EvidenceCorrupt,
            "/dependency",
        ))
    })?;
    crate::validate_dependency_chain(
        connection,
        &source_revision_id,
        artifact_revision_id,
        crate::DependencyValidationMode::Persisted,
    )
    .map_err(|_| {
        EngineError::Evidence(EvidenceErrorV1::new(
            EvidenceErrorReasonV1::EvidenceCorrupt,
            "/dependency",
        ))
    })?;
    Ok(Some(SourceDependencyV1 {
        schema_version: 1,
        dependency_id: DependencyId(dependency_id),
        source_revision_id: SourceRevisionId(source_revision_id),
        derived_revision_id: ArtifactRevisionId(artifact_revision_id.to_string()),
        registered_dependency_generation: u64::try_from(generation)
            .map_err(|_| EngineError::Storage)?,
    }))
}

fn keyed(key: &[u8], domain: &[u8], value: &[u8]) -> [u8; 32] {
    frozen_read::hmac_sha256(key, domain, value)
}

#[cfg(feature = "test-hooks")]
const INTRINSIC_NODE_PREFLIGHT_SQL: &str = "WITH requested(ordinal,write_cursor) AS (\
    SELECT CAST(key AS INTEGER),CAST(value AS INTEGER) FROM json_each(?1)) \
    SELECT q.ordinal,r.schema_version,r.revision_id,r.artifact_role,r.completeness,\
           n.logical_id,n.kind,n.body,n.source_id,n.state,n.superseded_at,n.valid_from,n.valid_until,\
           NULL,NULL,NULL,\
           l.schema_version,l.source_id,l.source_version_id,l.source_revision_id,l.locator_kind,\
           l.start_byte,l.end_byte,l.hash_algorithm,l.hash_digest,\
           sr.schema_version,sr.artifact_class,sr.artifact_role,sr.completeness,\
           sn.body,sn.source_id,sn.state,sn.superseded_at,sn.valid_from,sn.valid_until,sn.kind,\
           sv.schema_version,sv.source_id,sv.source_version_id,sv.source_revision_id,\
           sl.schema_version,sl.source_id,sl.source_version_id,sl.source_revision_id,sl.locator_kind,\
           sl.start_byte,sl.end_byte,sl.hash_algorithm,sl.hash_digest,\
           d.schema_version,d.dependency_id,d.derived_revision_id,d.registered_dependency_generation,\
           CAST(g.value AS INTEGER),\
           EXISTS(SELECT 1 FROM _fathomdb_dependency_closures c WHERE c.phase!='complete' AND (\
             (c.root_kind='source_revision' AND c.root_value=l.source_revision_id) OR\
             (c.root_kind='source_bucket' AND c.root_value=l.source_id)))\
     FROM requested q \
     JOIN _fathomdb_artifact_revisions r \
       ON r.artifact_class='node' AND r.write_cursor=q.write_cursor \
     JOIN canonical_nodes n ON n.write_cursor=r.write_cursor \
     JOIN _fathomdb_source_links l ON l.artifact_revision_id=r.revision_id \
     JOIN _fathomdb_artifact_revisions sr ON sr.revision_id=l.source_revision_id \
     JOIN canonical_nodes sn ON sn.write_cursor=sr.write_cursor \
     JOIN _fathomdb_source_versions sv ON sv.source_revision_id=l.source_revision_id \
     JOIN _fathomdb_source_links sl ON sl.artifact_revision_id=l.source_revision_id \
     LEFT JOIN _fathomdb_source_dependencies d ON d.derived_revision_id=r.revision_id \
     JOIN _fathomdb_open_state g ON g.key='_fathomdb_dependency_generation' \
     ORDER BY q.ordinal";

#[cfg(feature = "test-hooks")]
const INTRINSIC_EDGE_PREFLIGHT_SQL: &str = "WITH requested(ordinal,write_cursor) AS (\
    SELECT CAST(key AS INTEGER),CAST(value AS INTEGER) FROM json_each(?1)) \
    SELECT q.ordinal,r.schema_version,r.revision_id,r.artifact_role,r.completeness,\
           e.logical_id,e.kind,e.body,e.source_id,NULL,e.superseded_at,e.t_valid,e.t_invalid,\
           e.from_id,e.to_id,e.temporal_fallback,\
           l.schema_version,l.source_id,l.source_version_id,l.source_revision_id,l.locator_kind,\
           l.start_byte,l.end_byte,l.hash_algorithm,l.hash_digest,\
           sr.schema_version,sr.artifact_class,sr.artifact_role,sr.completeness,\
           sn.body,sn.source_id,sn.state,sn.superseded_at,sn.valid_from,sn.valid_until,sn.kind,\
           sv.schema_version,sv.source_id,sv.source_version_id,sv.source_revision_id,\
           sl.schema_version,sl.source_id,sl.source_version_id,sl.source_revision_id,sl.locator_kind,\
           sl.start_byte,sl.end_byte,sl.hash_algorithm,sl.hash_digest,\
           d.schema_version,d.dependency_id,d.derived_revision_id,d.registered_dependency_generation,\
           CAST(g.value AS INTEGER),\
           EXISTS(SELECT 1 FROM _fathomdb_dependency_closures c WHERE c.phase!='complete' AND (\
             (c.root_kind='source_revision' AND c.root_value=l.source_revision_id) OR\
             (c.root_kind='source_bucket' AND c.root_value=l.source_id)))\
     FROM requested q \
     JOIN _fathomdb_artifact_revisions r \
       ON r.artifact_class='edge' AND r.write_cursor=q.write_cursor \
     JOIN canonical_edges e ON e.write_cursor=r.write_cursor \
     JOIN _fathomdb_source_links l ON l.artifact_revision_id=r.revision_id \
     JOIN _fathomdb_artifact_revisions sr ON sr.revision_id=l.source_revision_id \
     JOIN canonical_nodes sn ON sn.write_cursor=sr.write_cursor \
     JOIN _fathomdb_source_versions sv ON sv.source_revision_id=l.source_revision_id \
     JOIN _fathomdb_source_links sl ON sl.artifact_revision_id=l.source_revision_id \
     LEFT JOIN _fathomdb_source_dependencies d ON d.derived_revision_id=r.revision_id \
     JOIN _fathomdb_open_state g ON g.key='_fathomdb_dependency_generation' \
     ORDER BY q.ordinal";

#[cfg(feature = "test-hooks")]
#[derive(Clone)]
pub(crate) struct IntrinsicAuthorityForTest {
    database_commitment: [u8; 32],
    context_commitment: [u8; 32],
    key: Vec<u8>,
}

#[cfg(feature = "test-hooks")]
pub(crate) fn intrinsic_authority_for_test(
    connection: &Connection,
    frozen: &FrozenReadContextV1,
) -> Result<IntrinsicAuthorityForTest, EngineError> {
    let (database_id, key) = frozen_read::page_cursor_material(connection)?;
    let context = frozen_read::validate_context(&frozen.context)?;
    Ok(IntrinsicAuthorityForTest {
        database_commitment: keyed(&key, DATABASE_DOMAIN, database_id.as_bytes()),
        context_commitment: keyed(&key, CONTEXT_DOMAIN, &context),
        key,
    })
}

#[cfg(feature = "test-hooks")]
#[derive(Clone)]
pub(crate) struct IntrinsicMaterialForTest {
    artifact_class: EvidenceArtifactClassV1,
    write_cursor: u64,
    artifact_revision_id: String,
    logical_id: Option<String>,
    artifact_kind: String,
    artifact_body: Option<String>,
    artifact_lifecycle: String,
    source_id: String,
    source_version_id: String,
    source_revision_id: String,
    locator: SourceLocator,
    hash_digest: String,
    source_lifecycle: String,
    dependency_id: Option<String>,
    dependency_generation: Option<u64>,
    edge_from: Option<String>,
    edge_to: Option<String>,
}

#[cfg(feature = "test-hooks")]
pub(crate) struct IntrinsicPreflightForTest {
    pub nodes: Vec<IntrinsicMaterialForTest>,
    pub edges: Vec<IntrinsicMaterialForTest>,
    pub stats: crate::GraphEvidencePreflightStatsForTest,
}

#[cfg(feature = "test-hooks")]
fn parse_locator(
    kind: String,
    start: Option<i64>,
    end: Option<i64>,
) -> Result<SourceLocator, EngineError> {
    match (kind.as_str(), start, end) {
        ("whole_body", None, None) => Ok(SourceLocator::WholeBody),
        ("utf8_bytes", Some(start), Some(end)) if start >= 0 && end >= start => {
            Ok(SourceLocator::utf8_bytes(start as u64, end as u64))
        }
        _ => Err(EvidenceErrorV1::new(
            EvidenceErrorReasonV1::EvidenceCorrupt,
            "/provenance/sourceLocator",
        )
        .into()),
    }
}

#[cfg(feature = "test-hooks")]
fn load_intrinsic_batch_for_test(
    connection: &Connection,
    sql: &str,
    artifact_class: EvidenceArtifactClassV1,
    cursors: &[u64],
    frozen: &FrozenReadContextV1,
    sources: &mut std::collections::HashMap<String, (std::sync::Arc<String>, String)>,
    stats: &mut crate::GraphEvidencePreflightStatsForTest,
) -> Result<Vec<IntrinsicMaterialForTest>, EngineError> {
    if cursors.len() > 50 {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    stats.data_statement_count += 1;
    let json = serde_json::to_string(cursors).map_err(|_| EngineError::Storage)?;
    let mut statement = connection.prepare(sql).map_err(|error| {
        eprintln!("Slice 15 preflight prepare: {error}");
        EngineError::Storage
    })?;
    let mut rows = statement.query([json]).map_err(|error| {
        eprintln!("Slice 15 preflight query: {error}");
        EngineError::Storage
    })?;
    let mut material = Vec::with_capacity(cursors.len());
    while let Some(row) = rows.next().map_err(|_| EngineError::Storage)? {
        let registry_schema: i64 = row.get(1).map_err(|_| EngineError::Storage)?;
        let artifact_revision_id: String = row.get(2).map_err(|_| EngineError::Storage)?;
        let role: String = row.get(3).map_err(|_| EngineError::Storage)?;
        let completeness: String = row.get(4).map_err(|_| EngineError::Storage)?;
        let logical_id: Option<String> = row.get(5).map_err(|_| EngineError::Storage)?;
        let artifact_kind: String = row.get(6).map_err(|_| EngineError::Storage)?;
        let artifact_body: Option<String> = row.get(7).map_err(|_| EngineError::Storage)?;
        let artifact_source_id: Option<String> = row.get(8).map_err(|_| EngineError::Storage)?;
        let artifact_state: Option<String> = row.get(9).map_err(|_| EngineError::Storage)?;
        let artifact_superseded: Option<i64> = row.get(10).map_err(|_| EngineError::Storage)?;
        let artifact_valid_from: Option<i64> = row.get(11).map_err(|_| EngineError::Storage)?;
        let artifact_valid_until: Option<i64> = row.get(12).map_err(|_| EngineError::Storage)?;
        let edge_from: Option<String> = row.get(13).map_err(|_| EngineError::Storage)?;
        let edge_to: Option<String> = row.get(14).map_err(|_| EngineError::Storage)?;
        let temporal_fallback: Option<i64> = row.get(15).map_err(|_| EngineError::Storage)?;
        let link_schema: i64 = row.get(16).map_err(|_| EngineError::Storage)?;
        let source_id: String = row.get(17).map_err(|_| EngineError::Storage)?;
        let source_version_id: String = row.get(18).map_err(|_| EngineError::Storage)?;
        let source_revision_id: String = row.get(19).map_err(|_| EngineError::Storage)?;
        let locator = parse_locator(
            row.get(20).map_err(|_| EngineError::Storage)?,
            row.get(21).map_err(|_| EngineError::Storage)?,
            row.get(22).map_err(|_| EngineError::Storage)?,
        )?;
        let hash_algorithm: String = row.get(23).map_err(|_| EngineError::Storage)?;
        let hash_digest: String = row.get(24).map_err(|_| EngineError::Storage)?;
        let source_registry_schema: i64 = row.get(25).map_err(|_| EngineError::Storage)?;
        let source_class: String = row.get(26).map_err(|_| EngineError::Storage)?;
        let source_role: String = row.get(27).map_err(|_| EngineError::Storage)?;
        let source_completeness: String = row.get(28).map_err(|_| EngineError::Storage)?;
        let source_body: String = row.get(29).map_err(|_| EngineError::Storage)?;
        let canonical_source_id: Option<String> = row.get(30).map_err(|_| EngineError::Storage)?;
        let source_state: String = row.get(31).map_err(|_| EngineError::Storage)?;
        let source_superseded: Option<i64> = row.get(32).map_err(|_| EngineError::Storage)?;
        let source_valid_from: Option<i64> = row.get(33).map_err(|_| EngineError::Storage)?;
        let source_valid_until: Option<i64> = row.get(34).map_err(|_| EngineError::Storage)?;
        let source_kind: String = row.get(35).map_err(|_| EngineError::Storage)?;
        let version_schema: i64 = row.get(36).map_err(|_| EngineError::Storage)?;
        let version_source_id: String = row.get(37).map_err(|_| EngineError::Storage)?;
        let version_id: String = row.get(38).map_err(|_| EngineError::Storage)?;
        let version_revision: String = row.get(39).map_err(|_| EngineError::Storage)?;
        let self_schema: i64 = row.get(40).map_err(|_| EngineError::Storage)?;
        let self_source_id: String = row.get(41).map_err(|_| EngineError::Storage)?;
        let self_version_id: String = row.get(42).map_err(|_| EngineError::Storage)?;
        let self_revision: String = row.get(43).map_err(|_| EngineError::Storage)?;
        let self_locator: String = row.get(44).map_err(|_| EngineError::Storage)?;
        let self_start: Option<i64> = row.get(45).map_err(|_| EngineError::Storage)?;
        let self_end: Option<i64> = row.get(46).map_err(|_| EngineError::Storage)?;
        let self_algorithm: String = row.get(47).map_err(|_| EngineError::Storage)?;
        let self_hash: String = row.get(48).map_err(|_| EngineError::Storage)?;
        let dependency_schema: Option<i64> = row.get(49).map_err(|_| EngineError::Storage)?;
        let dependency_id: Option<String> = row.get(50).map_err(|_| EngineError::Storage)?;
        let dependency_revision: Option<String> = row.get(51).map_err(|_| EngineError::Storage)?;
        let dependency_generation: Option<i64> = row.get(52).map_err(|_| EngineError::Storage)?;
        let current_dependency_generation: i64 = row.get(53).map_err(|_| EngineError::Storage)?;
        let closure_active: bool = row.get(54).map_err(|_| EngineError::Storage)?;
        let effective = frozen.effective_valid_at;
        let in_window = |start: Option<i64>, end: Option<i64>| {
            frozen.context.view.include_out_of_window
                || (start.is_none_or(|value| value <= effective)
                    && end.is_none_or(|value| value > effective))
        };
        if completeness != "complete" || source_completeness != "complete" {
            return Err(EvidenceErrorV1::new(
                EvidenceErrorReasonV1::EvidenceIncomplete,
                "/provenance",
            )
            .into());
        }
        let dependency_generation_u64 =
            dependency_generation.and_then(|value| u64::try_from(value).ok());
        let dependency_absent = dependency_schema.is_none()
            && dependency_id.is_none()
            && dependency_revision.is_none()
            && dependency_generation.is_none();
        let dependency_valid = dependency_absent
            || (dependency_schema == Some(1)
                && dependency_id.as_deref().is_some_and(|value| DependencyId::new(value).is_ok())
                && dependency_revision.as_deref() == Some(artifact_revision_id.as_str())
                && dependency_generation_u64.is_some_and(|value| {
                    value > 0
                        && i64::try_from(value)
                            .is_ok_and(|value| value <= current_dependency_generation)
                }));
        let artifact_lifecycle_valid = match artifact_class {
            EvidenceArtifactClassV1::Node => {
                artifact_state.as_deref() == Some("active")
                    && artifact_superseded.is_none()
                    && in_window(artifact_valid_from, artifact_valid_until)
            }
            EvidenceArtifactClassV1::Edge => {
                artifact_superseded.is_none()
                    && temporal_fallback.unwrap_or(0) == 0
                    && in_window(artifact_valid_from, artifact_valid_until)
            }
        };
        let source_type_valid = frozen
            .context
            .eligibility
            .source_type
            .as_deref()
            .is_none_or(|expected| expected == source_kind);
        let metadata_valid = registry_schema == 1
            && role == "derived_semantic"
            && link_schema == 1
            && source_registry_schema == 1
            && source_class == "node"
            && source_role == "canonical_source"
            && version_schema == 1
            && self_schema == 1
            && artifact_source_id.as_deref() == Some(source_id.as_str())
            && canonical_source_id.as_deref() == Some(source_id.as_str())
            && version_source_id == source_id
            && self_source_id == source_id
            && version_id == source_version_id
            && self_version_id == source_version_id
            && version_revision == source_revision_id
            && self_revision == source_revision_id
            && self_locator == "whole_body"
            && self_start.is_none()
            && self_end.is_none()
            && hash_algorithm == "sha256"
            && self_algorithm == "sha256"
            && self_hash == hash_digest
            && source_state == "active"
            && source_superseded.is_none()
            && in_window(source_valid_from, source_valid_until)
            && source_type_valid
            && artifact_lifecycle_valid
            && dependency_valid
            && !closure_active;
        if !metadata_valid
            || ArtifactRevisionId::new(artifact_revision_id.clone()).is_err()
            || SourceRevisionId::new(source_revision_id.clone()).is_err()
            || CanonicalHash::sha256(hash_digest.clone()).is_err()
            || slice(&source_body, &locator).is_err()
        {
            return Err(EvidenceErrorV1::new(
                EvidenceErrorReasonV1::EvidenceCorrupt,
                "/provenance",
            )
            .into());
        }
        if let Some((existing_body, existing_digest)) = sources.get(&source_revision_id) {
            if existing_body.as_str() != source_body || existing_digest != &hash_digest {
                return Err(EvidenceErrorV1::new(
                    EvidenceErrorReasonV1::EvidenceCorrupt,
                    "/provenance",
                )
                .into());
            }
        } else {
            if crate::canonical_body_hash(&source_body) != hash_digest {
                return Err(EvidenceErrorV1::new(
                    EvidenceErrorReasonV1::EvidenceCorrupt,
                    "/provenance",
                )
                .into());
            }
            stats.source_hash_count += 1;
            stats.source_bytes_hashed += source_body.len();
            let source = std::sync::Arc::new(source_body);
            sources.insert(source_revision_id.clone(), (source, hash_digest.clone()));
        }
        let write_cursor =
            cursors.get(material.len()).copied().ok_or_else(EvidenceErrorV1::unavailable)?;
        material.push(IntrinsicMaterialForTest {
            artifact_class,
            write_cursor,
            artifact_revision_id,
            logical_id,
            artifact_kind,
            artifact_body,
            artifact_lifecycle: artifact_state.unwrap_or_else(|| "active".to_string()),
            source_id,
            source_version_id,
            source_revision_id,
            locator,
            hash_digest,
            source_lifecycle: source_state,
            dependency_id,
            dependency_generation: dependency_generation_u64,
            edge_from,
            edge_to,
        });
    }
    if material.len() != cursors.len() {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    Ok(material)
}

#[cfg(feature = "test-hooks")]
pub(crate) fn preflight_intrinsic_batches_for_test(
    connection: &Connection,
    frozen: &FrozenReadContextV1,
    node_cursors: &[u64],
    edge_cursors: &[u64],
) -> Result<IntrinsicPreflightForTest, EngineError> {
    let mut stats = crate::GraphEvidencePreflightStatsForTest::default();
    let mut sources = std::collections::HashMap::new();
    let nodes = load_intrinsic_batch_for_test(
        connection,
        INTRINSIC_NODE_PREFLIGHT_SQL,
        EvidenceArtifactClassV1::Node,
        node_cursors,
        frozen,
        &mut sources,
        &mut stats,
    )?;
    stats.node_rows = nodes.len();
    let edges = load_intrinsic_batch_for_test(
        connection,
        INTRINSIC_EDGE_PREFLIGHT_SQL,
        EvidenceArtifactClassV1::Edge,
        edge_cursors,
        frozen,
        &mut sources,
        &mut stats,
    )?;
    stats.edge_rows = edges.len();
    Ok(IntrinsicPreflightForTest { nodes, edges, stats })
}

#[cfg(feature = "test-hooks")]
pub(crate) fn explain_intrinsic_preflights_for_test(
    connection: &Connection,
) -> Result<Vec<String>, EngineError> {
    [INTRINSIC_NODE_PREFLIGHT_SQL, INTRINSIC_EDGE_PREFLIGHT_SQL]
        .into_iter()
        .map(|sql| {
            let mut statement = connection
                .prepare(&format!("EXPLAIN QUERY PLAN {sql}"))
                .map_err(|_| EngineError::Storage)?;
            let details = statement
                .query_map(["[1]"], |row| row.get::<_, String>(3))
                .map_err(|_| EngineError::Storage)?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|_| EngineError::Storage)?;
            Ok(details.join(" | "))
        })
        .collect()
}

#[cfg(feature = "test-hooks")]
fn encode_intrinsic_material_for_test(
    material: &IntrinsicMaterialForTest,
    direction: Option<crate::TraversalDirection>,
) -> Vec<u8> {
    fn push_string(bytes: &mut Vec<u8>, value: &str) {
        frozen_read::encode_string(bytes, value);
    }

    let mut bytes = Vec::new();
    push_string(&mut bytes, material.artifact_revision_id.as_str());
    push_string(&mut bytes, material.logical_id.as_deref().unwrap_or(""));
    push_string(&mut bytes, &material.artifact_kind);
    push_string(&mut bytes, material.artifact_body.as_deref().unwrap_or(""));
    push_string(&mut bytes, &material.artifact_lifecycle);
    push_string(&mut bytes, &material.source_id);
    push_string(&mut bytes, &material.source_version_id);
    push_string(&mut bytes, &material.source_revision_id);
    bytes.extend_from_slice(&locator_bytes(&material.locator));
    push_string(&mut bytes, &material.hash_digest);
    push_string(&mut bytes, &material.source_lifecycle);
    push_string(&mut bytes, material.dependency_id.as_deref().unwrap_or(""));
    frozen_read::encode_u64(&mut bytes, material.dependency_generation.unwrap_or(0));
    push_string(&mut bytes, material.edge_from.as_deref().unwrap_or(""));
    push_string(&mut bytes, material.edge_to.as_deref().unwrap_or(""));
    bytes.push(match direction {
        None => 0,
        Some(crate::TraversalDirection::Outgoing) => 1,
        Some(crate::TraversalDirection::Incoming) => 2,
        Some(crate::TraversalDirection::Both) => 3,
    });
    bytes
}

#[cfg(feature = "test-hooks")]
pub(crate) fn mint_intrinsic_reference_for_test(
    authority: &IntrinsicAuthorityForTest,
    frozen: &FrozenReadContextV1,
    material: &IntrinsicMaterialForTest,
    direction: Option<crate::TraversalDirection>,
    disclosure: &[u8],
) -> Result<(String, EvidenceRefV1), EngineError> {
    let mut payload = Vec::new();
    frozen_read::encode_u32(&mut payload, 1);
    payload.push(material.artifact_class.tag());
    frozen_read::encode_u64(&mut payload, material.write_cursor);
    frozen_read::encode_i64(&mut payload, frozen.effective_valid_at);
    payload.extend_from_slice(&authority.database_commitment);
    payload.extend_from_slice(&authority.context_commitment);
    payload.extend_from_slice(&keyed(
        &authority.key,
        b"fathomdb.graph-intrinsic.slice15\0",
        &encode_intrinsic_material_for_test(material, direction),
    ));
    payload.extend_from_slice(&frozen_read::hmac_sha256(
        &authority.key,
        b"fathomdb.graph-disclosure.slice15\0",
        disclosure,
    ));
    payload.push(match direction {
        None => 0,
        Some(crate::TraversalDirection::Outgoing) => 1,
        Some(crate::TraversalDirection::Incoming) => 2,
        Some(crate::TraversalDirection::Both) => 3,
    });
    let mac = frozen_read::hmac_sha256(&authority.key, b"fathomdb.graph-ref.slice15\0", &payload);
    let reference =
        format!("fdgi1.{}.{}", frozen_read::hex_encode(&payload), frozen_read::hex_encode(&mac));
    Ok((material.artifact_revision_id.clone(), EvidenceRefV1::new(reference)?))
}

#[cfg(feature = "test-hooks")]
pub(crate) struct IntrinsicEvidenceForTest {
    pub artifact_class: EvidenceArtifactClassV1,
    pub artifact_revision_id: String,
    pub logical_id: Option<String>,
    pub canonical_source_body: String,
    pub evidence_text: String,
    pub artifact_kind: String,
    pub artifact_body: Option<String>,
    pub source_id: String,
    pub source_version_id: String,
    pub source_revision_id: String,
    pub canonical_source_hash: String,
    pub artifact_lifecycle: String,
    pub source_lifecycle: String,
    pub dependency_id: Option<String>,
    pub edge_from: Option<String>,
    pub edge_to: Option<String>,
    pub edge_direction: Option<crate::TraversalDirection>,
}

#[cfg(feature = "test-hooks")]
struct IntrinsicReferenceForTest {
    artifact_class: EvidenceArtifactClassV1,
    write_cursor: u64,
    effective_valid_at: i64,
    database_commitment: [u8; 32],
    context_commitment: [u8; 32],
    material_commitment: [u8; 32],
    direction: Option<crate::TraversalDirection>,
}

#[cfg(feature = "test-hooks")]
fn decode_intrinsic_reference_for_test(
    reference: &EvidenceRefV1,
    key: &[u8],
) -> Result<IntrinsicReferenceForTest, EngineError> {
    let mut parts = reference.as_str().split('.');
    let (Some("fdgi1"), Some(payload_hex), Some(mac_hex), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(EvidenceErrorV1::unavailable().into());
    };
    let payload = frozen_read::hex_decode(payload_hex).ok_or_else(EvidenceErrorV1::unavailable)?;
    let mac = frozen_read::hex_decode(mac_hex).ok_or_else(EvidenceErrorV1::unavailable)?;
    let expected = frozen_read::hmac_sha256(key, b"fathomdb.graph-ref.slice15\0", &payload);
    if !frozen_read::constant_time_eq(&mac, &expected) || payload.len() != 150 {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    let read_u32 = |range: std::ops::Range<usize>| {
        u32::from_be_bytes(payload[range].try_into().expect("fixed intrinsic payload"))
    };
    let read_u64 = |range: std::ops::Range<usize>| {
        u64::from_be_bytes(payload[range].try_into().expect("fixed intrinsic payload"))
    };
    let read_i64 = |range: std::ops::Range<usize>| {
        i64::from_be_bytes(payload[range].try_into().expect("fixed intrinsic payload"))
    };
    if read_u32(0..4) != 1 {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    let artifact_class =
        EvidenceArtifactClassV1::from_tag(payload[4]).ok_or_else(EvidenceErrorV1::unavailable)?;
    let direction = match payload[149] {
        0 => None,
        1 => Some(crate::TraversalDirection::Outgoing),
        2 => Some(crate::TraversalDirection::Incoming),
        3 => Some(crate::TraversalDirection::Both),
        _ => return Err(EvidenceErrorV1::unavailable().into()),
    };
    if (artifact_class == EvidenceArtifactClassV1::Node) != direction.is_none() {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    Ok(IntrinsicReferenceForTest {
        artifact_class,
        write_cursor: read_u64(5..13),
        effective_valid_at: read_i64(13..21),
        database_commitment: payload[21..53].try_into().expect("fixed intrinsic payload"),
        context_commitment: payload[53..85].try_into().expect("fixed intrinsic payload"),
        material_commitment: payload[85..117].try_into().expect("fixed intrinsic payload"),
        direction,
    })
}

#[cfg(feature = "test-hooks")]
pub(crate) fn resolve_intrinsic_reference_for_test(
    connection: &Connection,
    reference: &EvidenceRefV1,
    frozen: &FrozenReadContextV1,
) -> Result<IntrinsicEvidenceForTest, EngineError> {
    let (database_id, key) = frozen_read::page_cursor_material(connection)?;
    let payload = decode_intrinsic_reference_for_test(reference, &key)?;
    let context_bytes = frozen_read::validate_context(&frozen.context)?;
    if payload.database_commitment != keyed(&key, DATABASE_DOMAIN, database_id.as_bytes())
        || payload.context_commitment != keyed(&key, CONTEXT_DOMAIN, &context_bytes)
        || payload.effective_valid_at != frozen.effective_valid_at
    {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    let stored = load_stored(
        connection,
        payload.artifact_class,
        payload.write_cursor,
        payload.effective_valid_at,
        frozen.context.view.include_out_of_window,
    )
    .map_err(|_| EngineError::Evidence(EvidenceErrorV1::unavailable()))?;
    if stored.artifact_superseded
        || stored.source_superseded
        || crate::dependency_closure::active_barrier_for_source(
            connection,
            &stored.source_revision_id,
        )?
    {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    let eligible = match payload.artifact_class {
        EvidenceArtifactClassV1::Node => crate::text_hit_passes_filter(
            connection,
            payload.write_cursor,
            &stored.artifact_kind,
            Some(&frozen.context.eligibility),
        ),
        EvidenceArtifactClassV1::Edge => {
            let mut edge_filter = frozen.context.eligibility.clone();
            edge_filter.kind = None;
            crate::edge_fts_hit_passes_filter(
                connection,
                payload.write_cursor,
                &stored.artifact_kind,
                Some(&edge_filter),
            )
        }
    }
    .map_err(|_| EngineError::Storage)?;
    if !eligible || stored.completeness != "complete" {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    validate_source_bytes(&stored)?;
    validate_full_provenance(connection, &stored)?;
    let (artifact_body, edge_from, edge_to) = match payload.artifact_class {
        EvidenceArtifactClassV1::Node => (
            Some(
                connection
                    .query_row(
                        "SELECT body FROM canonical_nodes WHERE write_cursor=?1",
                        [i64::try_from(payload.write_cursor).map_err(|_| EngineError::Storage)?],
                        |row| row.get(0),
                    )
                    .map_err(|_| EngineError::Storage)?,
            ),
            None,
            None,
        ),
        EvidenceArtifactClassV1::Edge => connection
            .query_row(
                "SELECT body,from_id,to_id FROM canonical_edges WHERE write_cursor=?1",
                [i64::try_from(payload.write_cursor).map_err(|_| EngineError::Storage)?],
                |row| Ok((row.get(0)?, Some(row.get(1)?), Some(row.get(2)?))),
            )
            .map_err(|_| EngineError::Storage)?,
    };
    let dependency = load_dependency(connection, &stored.artifact_revision_id)?;
    let material = IntrinsicMaterialForTest {
        artifact_class: payload.artifact_class,
        write_cursor: payload.write_cursor,
        artifact_revision_id: stored.artifact_revision_id.clone(),
        logical_id: stored.logical_id.clone(),
        artifact_kind: stored.artifact_kind.clone(),
        artifact_body: artifact_body.clone(),
        artifact_lifecycle: stored.artifact_state.clone().unwrap_or_else(|| "active".to_string()),
        source_id: stored.source_id.clone(),
        source_version_id: stored.source_version_id.clone(),
        source_revision_id: stored.source_revision_id.clone(),
        locator: stored.locator.clone(),
        hash_digest: stored.hash_digest.clone(),
        source_lifecycle: stored.source_state.clone(),
        dependency_id: dependency.as_ref().map(|value| value.dependency_id.as_str().to_string()),
        dependency_generation: dependency
            .as_ref()
            .map(|value| value.registered_dependency_generation),
        edge_from: edge_from.clone(),
        edge_to: edge_to.clone(),
    };
    let expected_material = keyed(
        &key,
        b"fathomdb.graph-intrinsic.slice15\0",
        &encode_intrinsic_material_for_test(&material, payload.direction),
    );
    if payload.material_commitment != expected_material {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    let evidence_text = slice(&stored.source_body, &stored.locator)?;
    Ok(IntrinsicEvidenceForTest {
        artifact_class: payload.artifact_class,
        artifact_revision_id: stored.artifact_revision_id,
        logical_id: stored.logical_id,
        canonical_source_body: stored.source_body,
        evidence_text,
        artifact_kind: stored.artifact_kind,
        artifact_body,
        source_id: stored.source_id,
        source_version_id: stored.source_version_id,
        source_revision_id: stored.source_revision_id,
        canonical_source_hash: stored.hash_digest,
        artifact_lifecycle: stored.artifact_state.unwrap_or_else(|| "active".to_string()),
        source_lifecycle: stored.source_state,
        dependency_id: dependency.map(|value| value.dependency_id.as_str().to_string()),
        edge_from,
        edge_to,
        edge_direction: payload.direction,
    })
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
    ] {
        bytes.extend_from_slice(&commitment);
    }
    bytes.extend_from_slice(&payload.generation_nonce);
    frozen_read::encode_u32(
        &mut bytes,
        u32::try_from(payload.generation_ciphertext.len())
            .expect("projection generation ciphertext is bounded"),
    );
    bytes.extend_from_slice(&payload.generation_ciphertext);
    bytes.push(payload.arm.tag());
    encode_optional_u32(&mut bytes, payload.contribution.vector_rank);
    encode_optional_u32(&mut bytes, payload.contribution.text_rank);
    encode_optional_u32(&mut bytes, payload.contribution.graph_rank);
    encode_f64(&mut bytes, payload.contribution.fused_score);
    encode_optional_f64(&mut bytes, payload.contribution.ce_score);
    encode_f64(&mut bytes, payload.contribution.blended_score);
    encode_optional_f64(&mut bytes, payload.contribution.importance);
    encode_optional_f64(&mut bytes, payload.contribution.confidence);
    match (&payload.graph_origin, payload.graph_edge_commitment) {
        (None, None) => bytes.push(0),
        (Some(crate::CapturedGraphOrigin::EntitySeed), None) => {
            unreachable!("query-matched entity seeds remain text/vector representatives")
        }
        (Some(crate::CapturedGraphOrigin::EdgeSeed { edge_cursor }), Some(commitment)) => {
            bytes.push(2);
            frozen_read::encode_u64(&mut bytes, *edge_cursor);
            bytes.extend_from_slice(&commitment);
        }
        (
            Some(crate::CapturedGraphOrigin::Traversal { edge_cursor, hop_count }),
            Some(commitment),
        ) => {
            bytes.push(3);
            frozen_read::encode_u64(&mut bytes, *edge_cursor);
            frozen_read::encode_u32(&mut bytes, *hop_count);
            bytes.extend_from_slice(&commitment);
        }
        _ => unreachable!("graph edge origin and commitment are constructed together"),
    }
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
    let generation_nonce = cursor.array16()?;
    let generation_length =
        usize::try_from(cursor.u32()?).map_err(|_| EvidenceErrorV1::unavailable())?;
    if generation_length == 0 || generation_length > 64 {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    let generation_ciphertext = cursor.take(generation_length)?.to_vec();
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
    let (graph_origin, graph_edge_commitment) = match cursor.u8()? {
        0 => (None, None),
        2 => (
            Some(crate::CapturedGraphOrigin::EdgeSeed { edge_cursor: cursor.u64()? }),
            Some(cursor.array32()?),
        ),
        3 => (
            Some(crate::CapturedGraphOrigin::Traversal {
                edge_cursor: cursor.u64()?,
                hop_count: cursor.u32()?,
            }),
            Some(cursor.array32()?),
        ),
        _ => return Err(EvidenceErrorV1::unavailable().into()),
    };
    if cursor.offset != bytes.len() {
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
        generation_nonce,
        generation_ciphertext,
        arm,
        contribution,
        graph_origin,
        graph_edge_commitment,
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

    fn array16(&mut self) -> Result<[u8; 16], EngineError> {
        self.take(16)?.try_into().map_err(|_| EvidenceErrorV1::unavailable().into())
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

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn canonical_token_round_trips_and_single_character_tamper_fails(
            write_cursor in any::<u64>(),
            effective_valid_at in any::<i64>(),
            rank in proptest::option::of(any::<u32>()),
            fused_score in -1_000_000_f64..1_000_000_f64,
            blended_score in -1_000_000_f64..1_000_000_f64,
            seed in any::<[u8; 32]>(),
        ) {
            let key = [0x5a; 32];
            let payload = Payload {
                artifact_class: EvidenceArtifactClassV1::Node,
                write_cursor,
                effective_valid_at,
                database_commitment: seed,
                context_commitment: keyed(&key, b"test-context", &seed),
                artifact_commitment: keyed(&key, b"test-artifact", &seed),
                source_commitment: keyed(&key, b"test-source", &seed),
                locator_commitment: keyed(&key, b"test-locator", &seed),
                hash_commitment: keyed(&key, b"test-hash", &seed),
                generation_nonce: seed[..16].try_into().unwrap(),
                generation_ciphertext: protect_generation(
                    &key,
                    &seed[..16].try_into().unwrap(),
                    b"pgen1:00000000000000000000000000000000",
                ),
                arm: EvidenceArmV1::Text,
                contribution: EvidenceContributionV1 {
                    schema_version: SCHEMA_VERSION,
                    vector_rank: rank,
                    text_rank: rank,
                    graph_rank: None,
                    fused_score,
                    ce_score: None,
                    blended_score,
                    importance: None,
                    confidence: None,
                },
                graph_origin: None,
                graph_edge_commitment: None,
            };

            let token = encode_token(&key, &payload).unwrap();
            prop_assert!(token.len() <= TOKEN_MAX_BYTES);
            let decoded = decode_token(&key, &token).unwrap();
            prop_assert_eq!(encode_payload(&decoded), encode_payload(&payload));

            let mut tampered = token.into_bytes();
            let index = tampered.len() - 1;
            tampered[index] = if tampered[index] == b'0' { b'1' } else { b'0' };
            let tampered = String::from_utf8(tampered).unwrap();
            let is_unavailable = matches!(
                decode_token(&key, &tampered),
                Err(EngineError::Evidence(EvidenceErrorV1 {
                    reason: EvidenceErrorReasonV1::EvidenceUnavailable,
                    ..
                }))
            );
            prop_assert!(is_unavailable);
        }
    }

    #[test]
    fn generation_selector_resists_cross_token_known_plaintext_reuse() {
        let key = [0x31; 32];
        let generation = b"pgen1:00000000000000000000000000000000";
        let nonce_a = [0x41; 16];
        let nonce_b = [0x42; 16];
        let cipher_a = protect_generation(&key, &nonce_a, generation);
        let cipher_b = protect_generation(&key, &nonce_b, generation);
        assert_ne!(cipher_a, cipher_b);
        assert_eq!(protect_generation(&key, &nonce_a, &cipher_a), generation);

        let recovered_mask = cipher_a
            .iter()
            .zip(generation.iter())
            .map(|(cipher, plain)| cipher ^ plain)
            .collect::<Vec<_>>();
        let cross_token_guess = cipher_b
            .iter()
            .zip(recovered_mask)
            .map(|(cipher, mask)| cipher ^ mask)
            .collect::<Vec<_>>();
        assert_ne!(cross_token_guess, generation);
    }
}
