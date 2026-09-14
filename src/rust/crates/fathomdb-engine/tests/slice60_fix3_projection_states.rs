//! Slice 60 FIX-3 supplemental RED: real query expansion projection-state matrix.

#![cfg(feature = "test-hooks")]

use fathomdb_engine::{
    Engine, GraphExpandProjectionStateForTest, GraphExpandRequestV1,
    GraphExpansionDegradationCodeV1 as Code, GraphProjectionOriginV1 as Origin,
    GraphProjectionReadinessV1 as Readiness, GraphReadContextV1, GraphSeedV1, IdSpace,
    InitialState, PreparedWrite, ReadContextV1, ReadView, SearchFilter, SourceId,
    TraversalDirection,
};
use fathomdb_schema::SQLITE_SUFFIX;
use tempfile::TempDir;

fn current_request(seed: GraphSeedV1) -> GraphExpandRequestV1 {
    GraphExpandRequestV1 {
        schema_version: 1,
        seed,
        direction: TraversalDirection::Outgoing,
        edge_kinds: Vec::new(),
        target_kinds: Vec::new(),
        context: GraphReadContextV1::Current {
            schema_version: 1,
            context: ReadContextV1::new(
                ReadView { valid_as_of: Some(1_700_000_000), ..ReadView::default() },
                SearchFilter::default(),
            )
            .unwrap(),
        },
        max_depth: 0,
        result_limit: 1,
        max_work_units: 1,
        include_explanation: true,
        include_evidence: false,
    }
}

fn query_request() -> GraphExpandRequestV1 {
    current_request(GraphSeedV1::Query {
        schema_version: 1,
        text: "projection state witness".into(),
        ranked_limit: 1,
    })
}

#[test]
fn real_graph_expand_observes_every_projection_origin_readiness_degradation_combination() {
    let directory = TempDir::new().unwrap();
    let database = directory.path().join(format!("projection-state{SQLITE_SUFFIX}"));
    let opened = Engine::open(&database).unwrap();
    opened
        .engine
        .write(&[PreparedWrite::Node {
            logical_id: Some("root".into()),
            kind: "fact".into(),
            body: "projection state witness".into(),
            source_id: SourceId::new("test:slice60-fix3-projection").unwrap(),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        }])
        .unwrap();

    let explicit = opened
        .engine
        .graph_expand(&current_request(GraphSeedV1::Explicit {
            schema_version: 1,
            logical_ids: vec![IdSpace::logical("root")],
        }))
        .unwrap();
    let explicit_explanation = explicit.explanation.unwrap();
    assert_eq!(explicit_explanation.projection_origin, Origin::NotApplicable);
    assert_eq!(explicit_explanation.projection_readiness, Readiness::NotApplicable);
    assert!(explicit.degradation_codes.is_empty());

    let cases = [
        (
            GraphExpandProjectionStateForTest::fresh_ready(),
            Origin::Fresh,
            Readiness::Ready,
            vec![Code::QuerySeedTextFallback],
        ),
        (
            GraphExpandProjectionStateForTest::projection_legacy_unverified_degraded(),
            Origin::LegacyUnverified,
            Readiness::Degraded,
            vec![
                Code::QuerySeedTextFallback,
                Code::ProjectionLegacyUnverified,
                Code::ProjectionDegraded,
            ],
        ),
        (
            GraphExpandProjectionStateForTest::configuration_processing(),
            Origin::Configuration,
            Readiness::Processing,
            vec![Code::QuerySeedTextFallback, Code::ProjectionProcessing],
        ),
        (
            GraphExpandProjectionStateForTest::configuration_blocked(),
            Origin::Configuration,
            Readiness::Blocked,
            vec![Code::QuerySeedTextFallback, Code::ProjectionBlocked],
        ),
        (
            GraphExpandProjectionStateForTest::rebuild_deferred(),
            Origin::Rebuild,
            Readiness::Deferred,
            vec![Code::QuerySeedTextFallback, Code::ProjectionDeferred],
        ),
    ];
    for (state, origin, readiness, degradation_codes) in cases {
        let result = opened
            .engine
            .graph_expand_with_projection_state_for_test(&query_request(), state)
            .unwrap();
        assert!(result.complete);
        assert_eq!(result.degradation_codes, degradation_codes);
        let explanation = result.explanation.unwrap();
        assert_eq!(explanation.projection_origin, origin);
        assert_eq!(explanation.projection_readiness, readiness);
        assert_eq!(explanation.degradation_codes, result.degradation_codes);
    }
}
