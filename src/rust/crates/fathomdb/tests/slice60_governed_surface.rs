//! Slice 60 RED oracle for the additive default-facade graph surface.

use fathomdb::{
    Engine, EngineError, GraphExpandRequestV1, GraphExpandResultV1,
    GraphExpansionDegradationCodeV1, GraphExpansionErrorReasonV1, GraphExpansionErrorV1,
    GraphExpansionExplanationV1, GraphOriginV1, GraphProjectionOriginV1,
    GraphProjectionReadinessV1, GraphReadContextV1, GraphReadModeV1, GraphSeedSourceV1,
    GraphSeedV1, GraphTargetExplanationV1, GraphTargetV1, ResolvedGraphSeedV1,
};

#[test]
fn complete_graph_expand_contract_is_reexported_by_default_facade() {
    let _request: Option<GraphExpandRequestV1> = None;
    let _result: Option<GraphExpandResultV1> = None;
    let _seed: Option<GraphSeedV1> = None;
    let _context: Option<GraphReadContextV1> = None;
    let _resolved_seed: Option<ResolvedGraphSeedV1> = None;
    let _origin: Option<GraphOriginV1> = None;
    let _target: Option<GraphTargetV1> = None;
    let _explanation: Option<GraphExpansionExplanationV1> = None;
    let _target_explanation: Option<GraphTargetExplanationV1> = None;
    let _error: Option<GraphExpansionErrorV1> = None;
    let _codes = [
        GraphExpansionDegradationCodeV1::QuerySeedTextFallback,
        GraphExpansionDegradationCodeV1::ProjectionLegacyUnverified,
        GraphExpansionDegradationCodeV1::ProjectionProcessing,
        GraphExpansionDegradationCodeV1::ProjectionBlocked,
        GraphExpansionDegradationCodeV1::ProjectionDeferred,
        GraphExpansionDegradationCodeV1::ProjectionDegraded,
    ];
    let _origins = [
        GraphProjectionOriginV1::NotApplicable,
        GraphProjectionOriginV1::Fresh,
        GraphProjectionOriginV1::LegacyUnverified,
        GraphProjectionOriginV1::Configuration,
        GraphProjectionOriginV1::Rebuild,
    ];
    let _readiness = [
        GraphProjectionReadinessV1::NotApplicable,
        GraphProjectionReadinessV1::Ready,
        GraphProjectionReadinessV1::Processing,
        GraphProjectionReadinessV1::Blocked,
        GraphProjectionReadinessV1::Deferred,
        GraphProjectionReadinessV1::Degraded,
    ];
    let _seed_sources = [GraphSeedSourceV1::Query, GraphSeedSourceV1::Explicit];
    let _read_modes = [GraphReadModeV1::Current, GraphReadModeV1::Frozen];
    let _reasons = [
        GraphExpansionErrorReasonV1::UnsupportedSchemaVersion,
        GraphExpansionErrorReasonV1::UnknownField,
        GraphExpansionErrorReasonV1::GraphSeedInvalid,
        GraphExpansionErrorReasonV1::GraphDirectionInvalid,
        GraphExpansionErrorReasonV1::GraphEdgeKindsInvalid,
        GraphExpansionErrorReasonV1::GraphTargetKindsInvalid,
        GraphExpansionErrorReasonV1::GraphContextInvalid,
        GraphExpansionErrorReasonV1::GraphDepthInvalid,
        GraphExpansionErrorReasonV1::GraphResultLimitInvalid,
        GraphExpansionErrorReasonV1::GraphWorkLimitInvalid,
        GraphExpansionErrorReasonV1::GraphSeedUnavailable,
        GraphExpansionErrorReasonV1::GraphExpansionBoundExceeded,
        GraphExpansionErrorReasonV1::GraphProjectionUnavailable,
        GraphExpansionErrorReasonV1::GraphCorrupt,
    ];
    let _method: fn(&Engine, &GraphExpandRequestV1) -> Result<GraphExpandResultV1, EngineError> =
        Engine::graph_expand;
}

#[test]
fn graph_expand_is_governed_not_operator_gated() {
    let source = include_str!("../src/lib.rs");
    let before_operator_boundary = source
        .split("#[cfg(feature = \"operator\")]")
        .next()
        .expect("facade has an operator boundary");
    assert!(before_operator_boundary.contains("GraphExpandRequestV1"));
    assert!(before_operator_boundary.contains("GraphExpandResultV1"));
}
