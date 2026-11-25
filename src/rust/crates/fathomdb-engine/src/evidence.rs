use std::fmt::{Display, Formatter};

use crate::dependency::SourceDependencyV1;
#[cfg(feature = "test-hooks")]
use crate::errors::EngineError;
use crate::frozen_read::FrozenReadContextV1;
use crate::identity::{ArtifactRevisionId, CanonicalHash, SourceRevisionId};
use crate::projection_generation::ProjectionGenerationId;
use crate::properties::{PropertyMap, PropertyPredicate};
use crate::provenance::SourceLocator;
use crate::record_lifecycle::LifecycleState;
use crate::search_types::{
    GraphArmOptions, GraphArmSeedSource, GraphSeedMode, SearchResult, SoftFallbackBranch,
    DEFAULT_GRAPH_SEED_K, DEFAULT_SEARCH_RESULT_LIMIT,
};
use crate::Engine;

mod codec;
mod graph;
mod ranked;
#[cfg(test)]
mod tests;

pub use codec::encode_resolved_graph_evidence_v1;
pub(crate) use graph::{
    graph_evidence_authority, graph_request_commitment, mint_graph_evidence_reference,
    preflight_graph_evidence, resolve_graph_evidence, GraphEvidenceAuthority,
    GraphEvidenceDirection, GraphEvidenceDisclosure,
};
pub(crate) use ranked::{build_search_result, resolve, resolve_many_raw};

pub(super) const SCHEMA_VERSION: u32 = 1;
/// Maximum number of evidence references accepted in a single
/// [`crate::Engine::resolve_evidence_many`] request (enforced on raw count
/// before deduplication).
pub const EVIDENCE_RESOLVE_MANY_MAX: usize = 50;
pub(super) const TOKEN_MAX_BYTES: usize = 2_048;

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

    pub(super) fn tag(self) -> u8 {
        match self {
            Self::Node => 0,
            Self::Edge => 1,
        }
    }

    pub(super) fn from_tag(tag: u8) -> Option<Self> {
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

    pub(super) fn from_branch(branch: SoftFallbackBranch) -> Self {
        match branch {
            SoftFallbackBranch::Vector => Self::Vector,
            SoftFallbackBranch::Text => Self::Text,
            SoftFallbackBranch::TextEdge => Self::TextEdge,
            SoftFallbackBranch::GraphArm => Self::GraphArm,
        }
    }

    pub(super) fn tag(self) -> u8 {
        match self {
            Self::Vector => 0,
            Self::Text => 1,
            Self::TextEdge => 2,
            Self::GraphArm => 3,
        }
    }

    pub(super) fn from_tag(tag: u8) -> Option<Self> {
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
    EdgeSeed { edge_artifact_revision_id: String, seed_source: GraphArmSeedSource },
    Traversal { edge_artifact_revision_id: String, hop_count: u32, seed_source: GraphArmSeedSource },
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

/// Opaque, database-authenticated reference to one disclosed graph artifact.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphEvidenceRefV1(String);

impl GraphEvidenceRefV1 {
    /// Construct an opaque graph-evidence reference carrier.
    pub fn new(value: impl Into<String>) -> Result<Self, EvidenceErrorV1> {
        let value = value.into();
        if value.is_empty() || value.len() > TOKEN_MAX_BYTES {
            return Err(EvidenceErrorV1::unavailable());
        }
        Ok(Self(value))
    }

    /// Return the opaque token without interpreting it.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Positional evidence references for one graph target and its winning edge.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphEvidenceSidecarEntryV1 {
    pub schema_version: u32,
    pub target_index: u32,
    pub target_artifact_revision_id: ArtifactRevisionId,
    pub target_evidence_ref: GraphEvidenceRefV1,
    pub terminal_edge_artifact_revision_id: ArtifactRevisionId,
    pub terminal_edge_evidence_ref: GraphEvidenceRefV1,
}

/// Opt-in exact-evidence sidecar for constrained graph expansion.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphEvidenceSidecarV1 {
    pub schema_version: u32,
    pub entries: Vec<GraphEvidenceSidecarEntryV1>,
}

/// Request to resolve one graph-evidence reference under frozen authority.
#[derive(Clone, Debug, PartialEq)]
pub struct GraphEvidenceResolveRequestV1 {
    pub schema_version: u32,
    pub evidence_ref: GraphEvidenceRefV1,
    pub context: FrozenReadContextV1,
}

/// Exact immutable graph artifact selected by constrained expansion.
#[derive(Clone, Debug, PartialEq)]
pub enum GraphEvidenceArtifactV1 {
    Node {
        logical_id: String,
        kind: String,
        body: String,
        /// Optional typed scalar property map stored outside `body`.
        properties: Option<PropertyMap>,
    },
    Edge {
        logical_id: Option<String>,
        kind: String,
        body: Option<String>,
        /// Optional typed scalar property map stored outside `body`.
        properties: Option<PropertyMap>,
        from: String,
        to: String,
    },
}

/// Canonical intrinsic evidence for one exact graph artifact revision.
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedGraphEvidenceV1 {
    pub schema_version: u32,
    pub artifact_revision_id: ArtifactRevisionId,
    pub artifact: GraphEvidenceArtifactV1,
    pub source_id: String,
    pub source_version_id: String,
    pub source_revision_id: SourceRevisionId,
    pub locator: SourceLocator,
    pub canonical_source_body: String,
    pub evidence_text: String,
    pub canonical_source_hash: CanonicalHash,
    pub effective_valid_at: i64,
    pub artifact_lifecycle: EvidenceArtifactLifecycleV1,
    pub source_lifecycle_state: LifecycleState,
    pub dependency: Option<SourceDependencyV1>,
}

/// Versioned request for one opt-in evidence-producing search.
#[derive(Clone, Debug)]
#[non_exhaustive]
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
    pub graph_seed: GraphSeedMode,
    pub graph_seed_k: usize,
    pub graph_edge_kinds: Vec<String>,
    pub graph_edge_predicates: Vec<PropertyPredicate>,
    pub hints: bool,
}

impl EvidenceSearchRequestV1 {
    /// Construct a V1 evidence-producing search request with default ranking
    /// and graph-arm options.
    #[must_use]
    pub fn new(query: String, context: FrozenReadContextV1) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            query,
            context,
            rerank_depth: 0,
            use_graph_arm: false,
            alpha: 0.3,
            pool_n: 0,
            include_explanation: false,
            limit: DEFAULT_SEARCH_RESULT_LIMIT as u32,
            graph_seed: GraphSeedMode::GraphFts,
            graph_seed_k: DEFAULT_GRAPH_SEED_K,
            graph_edge_kinds: Vec::new(),
            graph_edge_predicates: Vec::new(),
            hints: false,
        }
    }

    /// Set the cross-encoder rerank depth.
    #[must_use]
    pub fn with_rerank_depth(mut self, rerank_depth: u32) -> Self {
        self.rerank_depth = rerank_depth;
        self
    }

    /// Enable or disable the opt-in graph-BFS third arm.
    #[must_use]
    pub fn with_use_graph_arm(mut self, use_graph_arm: bool) -> Self {
        self.use_graph_arm = use_graph_arm;
        self
    }

    /// Set the cross-encoder blend weight `alpha`.
    #[must_use]
    pub fn with_alpha(mut self, alpha: f64) -> Self {
        self.alpha = alpha;
        self
    }

    /// Set the rerank pool size `pool_n`.
    #[must_use]
    pub fn with_pool_n(mut self, pool_n: u32) -> Self {
        self.pool_n = pool_n;
        self
    }

    /// Enable or disable the retrieval explanation sidecar.
    #[must_use]
    pub fn with_explain(mut self, include_explanation: bool) -> Self {
        self.include_explanation = include_explanation;
        self
    }

    /// Set the ranked result limit (`1..=100`).
    #[must_use]
    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = limit;
        self
    }

    /// Set the graph-arm depth-0 seed mode.
    #[must_use]
    pub fn with_graph_seed(mut self, graph_seed: GraphSeedMode) -> Self {
        self.graph_seed = graph_seed;
        self
    }

    /// Set the maximum unique fused/reranked seeds (`1..=100`).
    #[must_use]
    pub fn with_graph_seed_k(mut self, graph_seed_k: usize) -> Self {
        self.graph_seed_k = graph_seed_k;
        self
    }

    /// Restrict graph-arm edge seeding and traversal to `graph_edge_kinds`.
    #[must_use]
    pub fn with_graph_edge_kinds(mut self, graph_edge_kinds: Vec<String>) -> Self {
        self.graph_edge_kinds = graph_edge_kinds;
        self
    }

    /// Restrict graph-arm edge seeding and traversal with closed inline property predicates.
    #[must_use]
    pub fn with_graph_edge_predicates(
        mut self,
        graph_edge_predicates: Vec<PropertyPredicate>,
    ) -> Self {
        self.graph_edge_predicates = graph_edge_predicates;
        self
    }

    /// Enable or disable opt-in search hints on the nested [`SearchResult`].
    #[must_use]
    pub fn with_hints(mut self, hints: bool) -> Self {
        self.hints = hints;
        self
    }

    pub(crate) fn graph_arm_options(&self) -> GraphArmOptions {
        GraphArmOptions {
            graph_seed: self.graph_seed,
            graph_seed_k: self.graph_seed_k,
            graph_edge_kinds: self.graph_edge_kinds.clone(),
            graph_edge_predicates: self.graph_edge_predicates.clone(),
        }
    }
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
#[derive(Clone, Debug, PartialEq)]
pub struct EvidenceResolveRequestV1 {
    pub schema_version: u32,
    pub evidence_ref: EvidenceRefV1,
    pub context: FrozenReadContextV1,
}

/// Versioned request to resolve a bounded batch of evidence references under
/// one authenticated frozen read context (`1..=EVIDENCE_RESOLVE_MANY_MAX`).
#[derive(Clone, Debug, PartialEq)]
pub struct EvidenceResolveManyRequestV1 {
    pub schema_version: u32,
    pub evidence_refs: Vec<EvidenceRefV1>,
    pub context: FrozenReadContextV1,
}

/// Per-reference outcome returned by [`crate::Engine::resolve_evidence_many`].
#[derive(Clone, Debug, PartialEq)]
pub enum EvidenceResolveManyItemV1 {
    /// Authorized evidence resolved for this request slot.
    Resolved(Box<ResolvedEvidenceV1>),
    /// Per-slot refusal carrying a nondisclosing reason and `/evidenceRefs/{i}` path.
    Refused(EvidenceErrorV1),
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
    /// Optional typed scalar property map stored on the resolved artifact.
    pub properties: Option<PropertyMap>,
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

pub(super) const INTRINSIC_NODE_PREFLIGHT_SQL: &str = "WITH requested(ordinal,write_cursor) AS (\
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
           CAST(g.value AS INTEGER),sr.write_cursor,\
           EXISTS(SELECT 1 FROM _fathomdb_dependency_closures c WHERE c.scope=r.scope AND c.phase!='complete' AND (\
             (c.root_kind='source_revision' AND c.root_value=l.source_revision_id) OR\
             (c.root_kind='source_bucket' AND c.root_value=l.source_id))),\
           CASE WHEN n.write_cursor IS NOT NULL /*TARGET_ELIGIBILITY*/ THEN 1 ELSE 0 END,\
           CASE WHEN sn.write_cursor IS NOT NULL /*SOURCE_ELIGIBILITY*/ THEN 1 ELSE 0 END,\
           n.properties,\
           CASE WHEN n.superseded_at IS NOT NULL AND n.logical_id IS NOT NULL AND EXISTS(\
             SELECT 1 FROM canonical_nodes cur \
             WHERE cur.scope=n.scope AND cur.logical_id=n.logical_id AND cur.superseded_at IS NULL AND cur.state!='active') \
             THEN 1 ELSE 0 END,\
           CASE WHEN sn.superseded_at IS NOT NULL AND sn.logical_id IS NOT NULL AND EXISTS(\
             SELECT 1 FROM canonical_nodes cur \
             WHERE cur.scope=sn.scope AND cur.logical_id=sn.logical_id AND cur.superseded_at IS NULL AND cur.state!='active') \
             THEN 1 ELSE 0 END \
     FROM requested q \
     LEFT JOIN _fathomdb_artifact_revisions r \
       ON r.artifact_class='node' AND r.write_cursor=q.write_cursor \
     LEFT JOIN canonical_nodes n ON n.scope=r.scope AND n.write_cursor=r.write_cursor \
     LEFT JOIN _fathomdb_source_links l ON l.scope=r.scope AND l.artifact_revision_id=r.revision_id \
     LEFT JOIN _fathomdb_artifact_revisions sr ON sr.scope=r.scope AND sr.revision_id=l.source_revision_id \
     LEFT JOIN canonical_nodes sn ON sn.scope=sr.scope AND sn.write_cursor=sr.write_cursor \
     LEFT JOIN _fathomdb_source_versions sv ON sv.scope=r.scope AND sv.source_revision_id=l.source_revision_id \
     LEFT JOIN _fathomdb_source_links sl ON sl.scope=r.scope AND sl.artifact_revision_id=l.source_revision_id \
     LEFT JOIN _fathomdb_source_dependencies d ON d.scope=r.scope AND d.derived_revision_id=r.revision_id \
     LEFT JOIN _fathomdb_open_state g ON g.key='_fathomdb_dependency_generation' \
     ORDER BY q.ordinal";

pub(super) const INTRINSIC_EDGE_PREFLIGHT_SQL: &str = "WITH requested(ordinal,write_cursor) AS (\
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
           CAST(g.value AS INTEGER),sr.write_cursor,\
           EXISTS(SELECT 1 FROM _fathomdb_dependency_closures c WHERE c.scope=r.scope AND c.phase!='complete' AND (\
             (c.root_kind='source_revision' AND c.root_value=l.source_revision_id) OR\
             (c.root_kind='source_bucket' AND c.root_value=l.source_id))),\
           CASE WHEN e.write_cursor IS NOT NULL /*TARGET_ELIGIBILITY*/ THEN 1 ELSE 0 END,\
           CASE WHEN sn.write_cursor IS NOT NULL /*SOURCE_ELIGIBILITY*/ THEN 1 ELSE 0 END,\
           e.properties,\
           0,\
           CASE WHEN sn.superseded_at IS NOT NULL AND sn.logical_id IS NOT NULL AND EXISTS(\
             SELECT 1 FROM canonical_nodes cur \
             WHERE cur.scope=sn.scope AND cur.logical_id=sn.logical_id AND cur.superseded_at IS NULL AND cur.state!='active') \
             THEN 1 ELSE 0 END \
     FROM requested q \
     LEFT JOIN _fathomdb_artifact_revisions r \
       ON r.artifact_class='edge' AND r.write_cursor=q.write_cursor \
     LEFT JOIN canonical_edges e ON e.scope=r.scope AND e.write_cursor=r.write_cursor \
     LEFT JOIN _fathomdb_source_links l ON l.scope=r.scope AND l.artifact_revision_id=r.revision_id \
     LEFT JOIN _fathomdb_artifact_revisions sr ON sr.scope=r.scope AND sr.revision_id=l.source_revision_id \
     LEFT JOIN canonical_nodes sn ON sn.scope=sr.scope AND sn.write_cursor=sr.write_cursor \
     LEFT JOIN _fathomdb_source_versions sv ON sv.scope=r.scope AND sv.source_revision_id=l.source_revision_id \
     LEFT JOIN _fathomdb_source_links sl ON sl.scope=r.scope AND sl.artifact_revision_id=l.source_revision_id \
     LEFT JOIN _fathomdb_source_dependencies d ON d.scope=r.scope AND d.derived_revision_id=r.revision_id \
     LEFT JOIN _fathomdb_open_state g ON g.key='_fathomdb_dependency_generation' \
     ORDER BY q.ordinal";

impl Engine {
    #[cfg(feature = "test-hooks")]
    #[doc(hidden)]
    pub fn explain_graph_evidence_preflights_for_test(&self) -> Result<Vec<String>, EngineError> {
        let _operation = self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
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
}
