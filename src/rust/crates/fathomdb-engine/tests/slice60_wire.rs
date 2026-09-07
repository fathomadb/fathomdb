//! Slice 60 RED oracle for the closed graph-expansion request and open response wire.

use fathomdb_engine::{
    decode_graph_expand_request_v1, decode_graph_expand_result_v1, encode_graph_expand_request_v1,
    encode_graph_expand_result_v1, GraphExpandRequestV1, GraphExpansionErrorReasonV1,
    GraphReadContextV1, GraphSeedV1, IdSpace, ReadContextV1, ReadView, SearchFilter,
    TraversalDirection,
};
use proptest::prelude::*;
use serde_json::{json, Value};

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../dev/fixtures/slice60-graph-expand-conformance-v1.json"
    ))
    .unwrap()
}

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

fn decode_request(value: &Value) -> fathomdb_engine::GraphExpansionErrorV1 {
    decode_graph_expand_request_v1(&serde_json::to_vec(value).unwrap()).unwrap_err()
}

fn decode_response(value: &Value) -> fathomdb_engine::GraphExpansionErrorV1 {
    decode_graph_expand_result_v1(&serde_json::to_vec(value).unwrap()).unwrap_err()
}

#[test]
fn canonical_fixture_round_trips_request_and_response() {
    let value = fixture();
    let request_bytes = serde_json::to_vec(&value["request"]).unwrap();
    assert_eq!(encode_graph_expand_request_v1(&request()).unwrap(), request_bytes);
    assert_eq!(decode_graph_expand_request_v1(&request_bytes).unwrap(), request());

    let response_bytes = serde_json::to_vec(&value["response"]).unwrap();
    let decoded = decode_graph_expand_result_v1(&response_bytes).unwrap();
    assert_eq!(encode_graph_expand_result_v1(&decoded).unwrap(), response_bytes);
}

#[test]
fn request_is_recursively_closed_with_version_then_unknown_precedence() {
    let base = fixture()["request"].clone();
    for (pointer, mutate, reason, path) in [
        (
            "top schema",
            ("/schemaVersion", json!(2), "/aaa", json!(true)),
            GraphExpansionErrorReasonV1::UnsupportedSchemaVersion,
            "/schemaVersion",
        ),
        (
            "top unknown",
            ("/schemaVersion", json!(1), "/a~1b~0c", json!(true)),
            GraphExpansionErrorReasonV1::UnknownField,
            "/a~1b~0c",
        ),
    ] {
        let mut value = base.clone();
        value.pointer_mut(mutate.0).map(|slot| *slot = mutate.1.clone());
        value.as_object_mut().unwrap().insert(mutate.2.trim_start_matches('/').into(), mutate.3);
        let error = decode_request(&value);
        assert_eq!(error.reason, reason, "{pointer}");
        assert_eq!(error.field_path, path, "{pointer}");
    }

    for (container, version_path, unknown_key, expected_unknown_path) in [
        ("/seed", "/seed/schemaVersion", "a/b~c", "/seed/a~1b~0c"),
        ("/context", "/context/schemaVersion", "a/b~c", "/context/a~1b~0c"),
        ("/context/context", "/context/context/schemaVersion", "a/b~c", "/context/context/a~1b~0c"),
    ] {
        let mut value = base.clone();
        value.pointer_mut(version_path).map(|slot| *slot = json!(2));
        value
            .pointer_mut(container)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert(unknown_key.into(), json!(true));
        let error = decode_request(&value);
        assert_eq!(error.reason, GraphExpansionErrorReasonV1::UnsupportedSchemaVersion);
        assert_eq!(error.field_path, version_path);

        *value.pointer_mut(version_path).unwrap() = json!(1);
        let error = decode_request(&value);
        assert_eq!(error.reason, GraphExpansionErrorReasonV1::UnknownField);
        assert_eq!(error.field_path, expected_unknown_path);
    }
}

#[test]
fn request_rejects_noncanonical_integers_unions_and_boolean_substitutions() {
    let base = fixture()["request"].clone();
    for (path, invalid, reason) in [
        ("/maxWorkUnits", json!(1), GraphExpansionErrorReasonV1::GraphWorkLimitInvalid),
        ("/maxWorkUnits", json!("01"), GraphExpansionErrorReasonV1::GraphWorkLimitInvalid),
        (
            "/maxWorkUnits",
            json!("18446744073709551616"),
            GraphExpansionErrorReasonV1::GraphWorkLimitInvalid,
        ),
        ("/maxDepth", json!(true), GraphExpansionErrorReasonV1::GraphDepthInvalid),
        ("/resultLimit", json!(1.5), GraphExpansionErrorReasonV1::GraphResultLimitInvalid),
        ("/includeExplanation", json!(1), GraphExpansionErrorReasonV1::GraphContextInvalid),
    ] {
        let mut value = base.clone();
        *value.pointer_mut(path).unwrap() = invalid;
        let error = decode_request(&value);
        assert_eq!(error.reason, reason, "{path}");
        assert_eq!(error.field_path, path);
    }

    let mut incoherent_seed = base.clone();
    incoherent_seed["seed"]["type"] = json!("query");
    let error = decode_request(&incoherent_seed);
    assert_eq!(error.reason, GraphExpansionErrorReasonV1::GraphSeedInvalid);
    assert_eq!(error.field_path, "/seed");
}

#[test]
fn response_accepts_additive_fields_and_rejects_closed_variants() {
    let base = fixture()["response"].clone();
    let mut additive = base.clone();
    additive.as_object_mut().unwrap().insert("futureField".into(), json!({"x": 1}));
    additive["targets"][0].as_object_mut().unwrap().insert("futureTargetField".into(), json!(true));
    decode_graph_expand_result_v1(&serde_json::to_vec(&additive).unwrap()).unwrap();

    for (path, invalid) in [
        ("/targets/0/origin/terminalDirection", json!("sideways")),
        ("/explanation/projectionOrigin", json!("unknown")),
        ("/explanation/projectionReadiness", json!("unknown")),
        ("/degradationCodes/0", json!("unknown")),
    ] {
        let mut value = base.clone();
        *value.pointer_mut(path).unwrap() = invalid;
        let error = decode_response(&value);
        assert_eq!(error.reason, GraphExpansionErrorReasonV1::GraphCorrupt);
        assert_eq!(error.field_path, path);
    }
}

#[test]
fn response_coherence_failures_report_the_first_exact_path() {
    let base = fixture()["response"].clone();
    let mutations = [
        ("/targets/0/origin/seedOrdinal", json!(1)),
        ("/targets/0/origin/seedLogicalId", json!("other")),
        ("/targets/0/origin/targetLogicalId", json!("other")),
        ("/explanation/perTarget", json!([])),
        ("/explanation/perTarget/0/targetIndex", json!(1)),
        ("/explanation/perTarget/0/origin/predecessorLogicalId", json!("other")),
        ("/explanation/degradationCodes", json!(["projection_degraded"])),
    ];
    let expected = [
        "/targets/0/origin/seedOrdinal",
        "/targets/0/origin/seedLogicalId",
        "/targets/0/origin/targetLogicalId",
        "/explanation/perTarget",
        "/explanation/perTarget/0/targetIndex",
        "/explanation/perTarget/0/origin",
        "/explanation/degradationCodes",
    ];
    for ((path, invalid), expected_path) in mutations.into_iter().zip(expected) {
        let mut value = base.clone();
        *value.pointer_mut(path).unwrap() = invalid;
        let error = decode_response(&value);
        assert_eq!(error.reason, GraphExpansionErrorReasonV1::GraphCorrupt);
        assert_eq!(error.field_path, expected_path, "mutation at {path}");
    }
}

#[test]
fn response_rejects_schema_nonfinite_scores_noncanonical_u64_and_incomplete() {
    let base = fixture()["response"].clone();
    for (path, invalid, reason, expected_path) in [
        (
            "/schemaVersion",
            json!(2),
            GraphExpansionErrorReasonV1::UnsupportedSchemaVersion,
            "/schemaVersion",
        ),
        (
            "/targets/0/writeCursor",
            json!("03"),
            GraphExpansionErrorReasonV1::GraphCorrupt,
            "/targets/0/writeCursor",
        ),
        ("/workUnits", json!(2), GraphExpansionErrorReasonV1::GraphCorrupt, "/workUnits"),
        ("/complete", json!(false), GraphExpansionErrorReasonV1::GraphCorrupt, "/complete"),
        (
            "/seeds/0/seedOrdinal",
            json!(1),
            GraphExpansionErrorReasonV1::GraphCorrupt,
            "/seeds/0/seedOrdinal",
        ),
    ] {
        let mut value = base.clone();
        *value.pointer_mut(path).unwrap() = invalid;
        let error = decode_response(&value);
        assert_eq!(error.reason, reason);
        assert_eq!(error.field_path, expected_path);
    }

    let mut nonfinite: Value = base;
    nonfinite["seeds"][0]["queryScore"] = json!("NaN");
    let error = decode_response(&nonfinite);
    assert_eq!(error.reason, GraphExpansionErrorReasonV1::GraphCorrupt);
    assert_eq!(error.field_path, "/seeds/0/queryScore");
}

proptest! {
    #[test]
    fn request_codec_round_trips_closed_valid_carriers(
        seed in "[A-Za-z0-9][A-Za-z0-9._:-]{0,31}",
        max_depth in 0_u32..=3,
        result_limit in 1_u32..=50,
        max_work_units in 1_u64..=10_000,
        direction in prop_oneof![
            Just(TraversalDirection::Outgoing),
            Just(TraversalDirection::Incoming),
            Just(TraversalDirection::Both),
        ],
    ) {
        let mut value = request();
        value.seed = GraphSeedV1::Explicit {
            schema_version: 1,
            logical_ids: vec![IdSpace::logical(seed)],
        };
        value.max_depth = max_depth;
        value.result_limit = result_limit;
        value.max_work_units = max_work_units;
        value.direction = direction;
        let bytes = encode_graph_expand_request_v1(&value).unwrap();
        prop_assert_eq!(decode_graph_expand_request_v1(&bytes).unwrap(), value);
    }
}
