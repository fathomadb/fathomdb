//! Slice 60 FIX-4 RED: source projection status drives graph expansion mapping.

#![cfg(feature = "test-hooks")]

use fathomdb_engine::{
    Engine, GraphExpandRequestV1, GraphExpansionDegradationCodeV1 as Code,
    GraphProjectionOriginV1 as GraphOrigin, GraphProjectionReadinessV1 as GraphReadiness,
    GraphReadContextV1, GraphSeedV1, InitialState, PreparedWrite, ProjectionGenerationOriginV1,
    ProjectionReadinessV1, ReadContextV1, ReadView, SearchFilter, SourceId, TraversalDirection,
};
use fathomdb_schema::SQLITE_SUFFIX;
use serde::Deserialize;
use tempfile::TempDir;

#[derive(Deserialize)]
struct Fixture {
    states: Vec<State>,
}
#[derive(Deserialize)]
struct State {
    origin: String,
    readiness: String,
    codes: Vec<String>,
}

fn request() -> GraphExpandRequestV1 {
    GraphExpandRequestV1 {
        schema_version: 1,
        seed: GraphSeedV1::Query {
            schema_version: 1,
            text: "projection state witness".into(),
            ranked_limit: 1,
        },
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
    }
}

fn source_origin(value: &str) -> ProjectionGenerationOriginV1 {
    match value {
        "fresh" => ProjectionGenerationOriginV1::Fresh,
        "legacy_unverified" => ProjectionGenerationOriginV1::LegacyUnverified,
        "configuration" => ProjectionGenerationOriginV1::Configuration,
        "rebuild" => ProjectionGenerationOriginV1::Rebuild,
        _ => panic!("fixture origin"),
    }
}

fn source_readiness(value: &str) -> ProjectionReadinessV1 {
    match value {
        "ready" => ProjectionReadinessV1::Ready,
        "processing" => ProjectionReadinessV1::Processing,
        "blocked" => ProjectionReadinessV1::Blocked,
        "deferred" => ProjectionReadinessV1::Deferred,
        "degraded" => ProjectionReadinessV1::Degraded,
        _ => panic!("fixture readiness"),
    }
}

#[test]
fn graph_expand_maps_injected_source_projection_statuses_through_production_mapping() {
    let fixture: Fixture = serde_json::from_str(include_str!(
        "../../../../../dev/fixtures/slice60-fix4-projection-generation-v1.json"
    ))
    .unwrap();
    let directory = TempDir::new().unwrap();
    let opened = Engine::open(directory.path().join(format!("projection{SQLITE_SUFFIX}"))).unwrap();
    opened
        .engine
        .write(&[PreparedWrite::Node {
            logical_id: Some("root".into()),
            kind: "fact".into(),
            body: "projection state witness".into(),
            source_id: SourceId::new("fix4-projection").unwrap(),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        }])
        .unwrap();

    for state in fixture.states {
        let result = opened
            .engine
            .graph_expand_with_projection_generation_for_test(
                &request(),
                source_origin(&state.origin),
                source_readiness(&state.readiness),
            )
            .unwrap();
        let explanation = result.explanation.unwrap();
        assert_eq!(
            explanation.projection_origin,
            match state.origin.as_str() {
                "fresh" => GraphOrigin::Fresh,
                "legacy_unverified" => GraphOrigin::LegacyUnverified,
                "configuration" => GraphOrigin::Configuration,
                "rebuild" => GraphOrigin::Rebuild,
                _ => unreachable!(),
            }
        );
        assert_eq!(
            explanation.projection_readiness,
            match state.readiness.as_str() {
                "ready" => GraphReadiness::Ready,
                "processing" => GraphReadiness::Processing,
                "blocked" => GraphReadiness::Blocked,
                "deferred" => GraphReadiness::Deferred,
                "degraded" => GraphReadiness::Degraded,
                _ => unreachable!(),
            }
        );
        let expected = state
            .codes
            .iter()
            .map(|code| match code.as_str() {
                "query_seed_text_fallback" => Code::QuerySeedTextFallback,
                "projection_legacy_unverified" => Code::ProjectionLegacyUnverified,
                "projection_processing" => Code::ProjectionProcessing,
                "projection_blocked" => Code::ProjectionBlocked,
                "projection_deferred" => Code::ProjectionDeferred,
                "projection_degraded" => Code::ProjectionDegraded,
                _ => unreachable!(),
            })
            .collect::<Vec<_>>();
        assert_eq!(result.degradation_codes, expected);
    }
}
