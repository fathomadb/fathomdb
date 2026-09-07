//! Slice 60 FIX-1 RED: canonical bytes, closure precedence, and test-only seams.

use fathomdb_engine::{
    decode_graph_expand_request_v1, encode_graph_expand_request_v1, GraphExpandRequestV1,
    GraphExpansionErrorReasonV1, GraphReadContextV1, GraphSeedV1, IdSpace, ReadContextV1, ReadView,
    SearchFilter, TraversalDirection,
};
use serde_json::Value;

fn request() -> GraphExpandRequestV1 {
    GraphExpandRequestV1 {
        schema_version: 1,
        seed: GraphSeedV1::Explicit {
            schema_version: 1,
            logical_ids: vec![IdSpace::logical("seed-a")],
        },
        direction: TraversalDirection::Outgoing,
        edge_kinds: vec!["supports".into()],
        target_kinds: vec!["fact".into()],
        context: GraphReadContextV1::Current {
            schema_version: 1,
            context: ReadContextV1::new(
                ReadView { valid_as_of: Some(1_700_000_000), ..ReadView::default() },
                SearchFilter::default(),
            )
            .unwrap(),
        },
        max_depth: 2,
        result_limit: 10,
        max_work_units: 100,
        include_explanation: true,
    }
}

fn decode(value: Value) -> fathomdb_engine::GraphExpansionErrorV1 {
    decode_graph_expand_request_v1(&serde_json::to_vec(&value).unwrap()).unwrap_err()
}

#[test]
fn raw_canonical_request_bytes_follow_declaration_order_without_value_normalization() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../../dev/fixtures/slice60-fix1-canonical-request-v1.json"
    ))
    .unwrap();
    let expected = fixture["request"].as_str().unwrap().as_bytes();
    assert_eq!(encode_graph_expand_request_v1(&request()).unwrap(), expected);
}

#[test]
fn seed_unknown_field_precedes_missing_or_invalid_discriminant_with_escaped_pointer() {
    for seed in [
        serde_json::json!({"schemaVersion": 1, "a/b~c": true}),
        serde_json::json!({"schemaVersion": 1, "type": "invalid", "a/b~c": true}),
    ] {
        let mut value: Value = serde_json::from_str(include_str!(
            "../../../../../dev/fixtures/slice60-graph-expand-conformance-v1.json"
        ))
        .unwrap();
        value["request"]["seed"] = seed;
        let error = decode(value["request"].clone());
        assert_eq!(error.reason, GraphExpansionErrorReasonV1::UnknownField);
        assert_eq!(error.field_path, "/seed/a~1b~0c");
    }
}

#[test]
fn graph_rendezvous_is_test_hooks_gated_owned_and_production_explain_reuses_incident_builder() {
    let source = include_str!("../src/graph_expand.rs");
    assert!(source.contains("#[cfg(feature = \"test-hooks\")]\nstruct GraphExpandPinRendezvous"));
    assert!(!source.contains("static BEFORE_PIN_HOOK: OneShotHook"));
    assert!(source.contains("graph_expand_incident_sql"));
    assert!(source.contains("EXPLAIN QUERY PLAN {sql}"));
}

#[test]
fn real_database_high_bound_rss_dependency_erasure_and_projection_matrix_have_owned_test_seams() {
    let source = include_str!("../src/graph_expand.rs");
    for required in [
        "GraphExpandMeasurementForTest",
        "measure_graph_expand_for_test",
        "seed_graph_expand_dependency_closure_for_test",
        "seed_graph_expand_erasure_for_test",
        "seed_graph_expand_projection_state_for_test",
    ] {
        assert!(source.contains(required), "missing required FIX-1 seam {required}");
    }
}
