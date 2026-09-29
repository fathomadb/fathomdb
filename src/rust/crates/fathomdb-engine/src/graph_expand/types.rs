use std::fmt::{Display, Formatter};

use crate::frozen_read::{FrozenReadContextV1, ReadContextV1};
use crate::identity::IdSpace;
use crate::search_types::{StructuralDependencyStateV1, StructuralLifecycleStateV1};

pub(crate) const SCHEMA_VERSION: u32 = 1;

/// The source from which graph-expansion seeds were resolved.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GraphSeedSourceV1 {
    /// Logical nodes selected by indexed full-text ranking.
    Query,
    /// Logical nodes supplied explicitly by the caller.
    Explicit,
}

/// The database read mode used by a graph expansion.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GraphReadModeV1 {
    /// A current reader transaction.
    Current,
    /// An authenticated frozen reader transaction.
    Frozen,
}

/// The serving projection generation's origin.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GraphProjectionOriginV1 {
    /// No retrieval projection applies to explicit seeding.
    NotApplicable,
    /// A generation minted for a fresh database.
    Fresh,
    /// A legacy generation whose provenance cannot be fully verified.
    LegacyUnverified,
    /// A generation minted by projection configuration.
    Configuration,
    /// A generation minted by a rebuild.
    Rebuild,
}

/// Readiness of the projection generation observed by graph expansion.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GraphProjectionReadinessV1 {
    /// No retrieval projection applies to explicit seeding.
    NotApplicable,
    /// Projection work is complete.
    Ready,
    /// Projection work is actively processing.
    Processing,
    /// Projection work is blocked by absent runtime configuration.
    Blocked,
    /// Projection work is deferred by runtime policy.
    Deferred,
    /// Projection work completed with a hard degradation.
    Degraded,
}

/// Stable graph-expansion degradation vocabulary.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum GraphExpansionDegradationCodeV1 {
    /// Query seeding deliberately used logical-node FTS without a dense arm.
    QuerySeedTextFallback,
    /// The serving projection has legacy-unverified provenance.
    ProjectionLegacyUnverified,
    /// Projection work is processing.
    ProjectionProcessing,
    /// Projection work is blocked.
    ProjectionBlocked,
    /// Projection work is deferred.
    ProjectionDeferred,
    /// Projection work is degraded.
    ProjectionDegraded,
}

/// Closed seed carrier for graph expansion.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GraphSeedV1 {
    /// Resolve up to `ranked_limit` logical nodes through native FTS.
    Query { schema_version: u32, text: String, ranked_limit: u32 },
    /// Resolve caller-ordered logical identifiers.
    Explicit { schema_version: u32, logical_ids: Vec<IdSpace> },
}

/// Closed current-or-frozen read context.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GraphReadContextV1 {
    /// Use a newly pinned current reader snapshot.
    Current { schema_version: u32, context: ReadContextV1 },
    /// Use an authenticated frozen reader snapshot.
    Frozen { schema_version: u32, context: FrozenReadContextV1 },
}

/// Versioned, bounded constrained graph-expansion request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphExpandRequestV1 {
    pub schema_version: u32,
    pub seed: GraphSeedV1,
    pub direction: TraversalDirection,
    pub edge_kinds: Vec<String>,
    pub target_kinds: Vec<String>,
    pub context: GraphReadContextV1,
    pub max_depth: u32,
    pub result_limit: u32,
    pub max_work_units: u64,
    pub include_explanation: bool,
    pub include_evidence: bool,
}

/// A logical seed resolved inside the graph-expansion snapshot.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ResolvedGraphSeedV1 {
    pub schema_version: u32,
    pub logical_id: String,
    pub seed_ordinal: u32,
    pub query_score: Option<f64>,
}

/// Compact deterministic origin for one returned target.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct GraphOriginV1 {
    pub schema_version: u32,
    pub seed_logical_id: String,
    pub seed_ordinal: u32,
    pub predecessor_logical_id: String,
    pub target_logical_id: String,
    pub hop_count: u32,
    pub terminal_edge_kind: String,
    pub terminal_direction: TraversalDirection,
}

/// One graph-expansion target with its compact origin.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct GraphTargetV1 {
    pub schema_version: u32,
    pub logical_id: String,
    pub kind: String,
    pub body: String,
    pub write_cursor: u64,
    pub origin: GraphOriginV1,
}

/// Structural explanation for one target.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct GraphTargetExplanationV1 {
    pub schema_version: u32,
    pub target_index: u32,
    pub origin: GraphOriginV1,
    pub lifecycle_state: StructuralLifecycleStateV1,
    pub dependency_state: StructuralDependencyStateV1,
}

/// Optional compact explanation sidecar for graph expansion.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct GraphExpansionExplanationV1 {
    pub schema_version: u32,
    pub correlation_id: String,
    pub seed_source: GraphSeedSourceV1,
    pub read_mode: GraphReadModeV1,
    pub projection_generation_id: Option<String>,
    pub projection_origin: GraphProjectionOriginV1,
    pub projection_readiness: GraphProjectionReadinessV1,
    pub degradation_codes: Vec<GraphExpansionDegradationCodeV1>,
    pub per_target: Vec<GraphTargetExplanationV1>,
}

/// Complete, deterministic one-page graph-expansion result.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct GraphExpandResultV1 {
    pub schema_version: u32,
    pub seeds: Vec<ResolvedGraphSeedV1>,
    pub targets: Vec<GraphTargetV1>,
    pub complete: bool,
    pub work_units: u64,
    pub degradation_codes: Vec<GraphExpansionDegradationCodeV1>,
    pub explanation: Option<GraphExpansionExplanationV1>,
    pub evidence: Option<crate::evidence::GraphEvidenceSidecarV1>,
}

/// Closed graph-expansion refusal reason.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GraphExpansionErrorReasonV1 {
    UnsupportedSchemaVersion,
    UnknownField,
    GraphSeedInvalid,
    GraphDirectionInvalid,
    GraphEdgeKindsInvalid,
    GraphTargetKindsInvalid,
    GraphContextInvalid,
    GraphDepthInvalid,
    GraphResultLimitInvalid,
    GraphWorkLimitInvalid,
    GraphSeedUnavailable,
    GraphExpansionBoundExceeded,
    GraphProjectionUnavailable,
    GraphCorrupt,
}

/// Typed graph-expansion refusal with an RFC 6901 field path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphExpansionErrorV1 {
    pub schema_version: u32,
    pub reason: GraphExpansionErrorReasonV1,
    pub field_path: String,
}

impl GraphExpansionErrorReasonV1 {
    /// Stable lower-snake-case wire spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UnsupportedSchemaVersion => "unsupported_schema_version",
            Self::UnknownField => "unknown_field",
            Self::GraphSeedInvalid => "graph_seed_invalid",
            Self::GraphDirectionInvalid => "graph_direction_invalid",
            Self::GraphEdgeKindsInvalid => "graph_edge_kinds_invalid",
            Self::GraphTargetKindsInvalid => "graph_target_kinds_invalid",
            Self::GraphContextInvalid => "graph_context_invalid",
            Self::GraphDepthInvalid => "graph_depth_invalid",
            Self::GraphResultLimitInvalid => "graph_result_limit_invalid",
            Self::GraphWorkLimitInvalid => "graph_work_limit_invalid",
            Self::GraphSeedUnavailable => "graph_seed_unavailable",
            Self::GraphExpansionBoundExceeded => "graph_expansion_bound_exceeded",
            Self::GraphProjectionUnavailable => "graph_projection_unavailable",
            Self::GraphCorrupt => "graph_corrupt",
        }
    }
}

impl GraphExpansionErrorV1 {
    pub(super) fn new(reason: GraphExpansionErrorReasonV1, field_path: impl Into<String>) -> Self {
        Self { schema_version: SCHEMA_VERSION, reason, field_path: field_path.into() }
    }
}

impl Display for GraphExpansionErrorV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{} at {}", self.reason.as_str(), self.field_path)
    }
}

impl std::error::Error for GraphExpansionErrorV1 {}

/// Compose graph-expansion degradation codes from the three contract axes.
#[doc(hidden)]
#[must_use]
fn graph_expansion_degradation_codes_impl(
    seed_source: GraphSeedSourceV1,
    origin: GraphProjectionOriginV1,
    readiness: GraphProjectionReadinessV1,
) -> Vec<GraphExpansionDegradationCodeV1> {
    let mut codes = Vec::new();
    if seed_source == GraphSeedSourceV1::Query {
        codes.push(GraphExpansionDegradationCodeV1::QuerySeedTextFallback);
    }
    if origin == GraphProjectionOriginV1::LegacyUnverified {
        codes.push(GraphExpansionDegradationCodeV1::ProjectionLegacyUnverified);
    }
    match readiness {
        GraphProjectionReadinessV1::Processing => {
            codes.push(GraphExpansionDegradationCodeV1::ProjectionProcessing);
        }
        GraphProjectionReadinessV1::Blocked => {
            codes.push(GraphExpansionDegradationCodeV1::ProjectionBlocked);
        }
        GraphProjectionReadinessV1::Deferred => {
            codes.push(GraphExpansionDegradationCodeV1::ProjectionDeferred);
        }
        GraphProjectionReadinessV1::Degraded => {
            codes.push(GraphExpansionDegradationCodeV1::ProjectionDegraded);
        }
        GraphProjectionReadinessV1::NotApplicable | GraphProjectionReadinessV1::Ready => {}
    }
    codes.sort();
    codes.dedup();
    codes
}

pub(super) fn graph_expansion_degradation_codes(
    seed_source: GraphSeedSourceV1,
    origin: GraphProjectionOriginV1,
    readiness: GraphProjectionReadinessV1,
) -> Vec<GraphExpansionDegradationCodeV1> {
    graph_expansion_degradation_codes_impl(seed_source, origin, readiness)
}

/// Compose graph-expansion degradation codes for test-only matrix coverage.
#[cfg(feature = "test-hooks")]
#[doc(hidden)]
#[must_use]
pub fn graph_expansion_degradation_codes_for_test(
    seed_source: GraphSeedSourceV1,
    origin: GraphProjectionOriginV1,
    readiness: GraphProjectionReadinessV1,
) -> Vec<GraphExpansionDegradationCodeV1> {
    graph_expansion_degradation_codes_impl(seed_source, origin, readiness)
}

// ===== Slice 20 (G5/G6) — graph traversal types =========================

/// Slice 20 (G5) — direction of graph traversal for
/// [`crate::Engine::graph_neighbors`] / [`crate::Engine::search_expand`].
///
/// `Outgoing` follows edges where the root is the `from_id` (source).
/// `Incoming` follows edges where the root is the `to_id` (target).
/// `Both` follows edges in either direction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraversalDirection {
    Outgoing,
    Incoming,
    Both,
}
