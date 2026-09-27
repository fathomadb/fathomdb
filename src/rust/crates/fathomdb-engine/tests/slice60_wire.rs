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
        include_evidence: false,
    }
}

fn decode_request(value: &Value) -> fathomdb_engine::GraphExpansionErrorV1 {
    decode_graph_expand_request_v1(&serde_json::to_vec(value).unwrap()).unwrap_err()
}

fn decode_response(value: &Value) -> fathomdb_engine::GraphExpansionErrorV1 {
    decode_graph_expand_result_v1(&serde_json::to_vec(value).unwrap()).unwrap_err()
}

struct CoherentResponseInput<'a> {
    seed_logical_id: &'a str,
    target_logical_id: &'a str,
    predecessor_logical_id: &'a str,
    body: &'a str,
    write_cursor: u64,
    work_units: u64,
    query_score: Option<f64>,
    include_evidence: bool,
}

fn coherent_response(input: CoherentResponseInput<'_>) -> Value {
    let mut value = fixture()["response"].clone();
    value["seeds"][0]["logicalId"] = json!(input.seed_logical_id);
    value["seeds"][0]["queryScore"] = json!(input.query_score);
    value["targets"][0]["logicalId"] = json!(input.target_logical_id);
    value["targets"][0]["body"] = json!(input.body);
    value["targets"][0]["writeCursor"] = json!(input.write_cursor.to_string());
    value["targets"][0]["origin"]["seedLogicalId"] = json!(input.seed_logical_id);
    value["targets"][0]["origin"]["predecessorLogicalId"] = json!(input.predecessor_logical_id);
    value["targets"][0]["origin"]["targetLogicalId"] = json!(input.target_logical_id);
    value["explanation"]["perTarget"][0]["origin"] = value["targets"][0]["origin"].clone();
    value["workUnits"] = json!(input.work_units.to_string());
    if input.include_evidence {
        value["evidence"] = json!({
            "schemaVersion": 1,
            "entries": [{
                "schemaVersion": 1,
                "targetIndex": 0,
                "targetArtifactRevisionId": "target-r1",
                "targetEvidenceRef": "target-evidence-ref",
                "terminalEdgeArtifactRevisionId": "edge-r1",
                "terminalEdgeEvidenceRef": "edge-evidence-ref"
            }]
        });
    }
    value
}

#[test]
fn canonical_fixture_round_trips_request_and_response() {
    let request_fixture: Value = serde_json::from_str(include_str!(
        "../../../../../dev/fixtures/slice60-fix1-canonical-request-v1.json"
    ))
    .unwrap();
    let request_bytes = request_fixture["request"].as_str().unwrap().as_bytes();
    assert_eq!(encode_graph_expand_request_v1(&request()).unwrap(), request_bytes);
    assert_eq!(decode_graph_expand_request_v1(request_bytes).unwrap(), request());

    let response_bytes =
        include_str!("../../../../../dev/fixtures/slice60-fix1-canonical-result-v1.json")
            .trim_end()
            .as_bytes();
    let decoded = decode_graph_expand_result_v1(response_bytes).unwrap();
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
            ("/schemaVersion", json!(1), "/a/b~c", json!(true)),
            GraphExpansionErrorReasonV1::UnknownField,
            "/a~1b~0c",
        ),
    ] {
        let mut value = base.clone();
        if let Some(slot) = value.pointer_mut(mutate.0) {
            *slot = mutate.1.clone();
        }
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
        if let Some(slot) = value.pointer_mut(version_path) {
            *slot = json!(2);
        }
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

    for (mutation_path, invalid, expected_path) in [
        (
            "/targets/0/origin/terminalDirection",
            json!("sideways"),
            "/targets/0/origin/terminalDirection",
        ),
        ("/explanation/projectionOrigin", json!("unknown"), "/explanation/projectionOrigin"),
        ("/explanation/projectionReadiness", json!("unknown"), "/explanation/projectionReadiness"),
        ("/degradationCodes", json!(["unknown"]), "/degradationCodes/0"),
    ] {
        let mut value = base.clone();
        *value.pointer_mut(mutation_path).unwrap() = invalid;
        let error = decode_response(&value);
        assert_eq!(error.reason, GraphExpansionErrorReasonV1::GraphCorrupt);
        assert_eq!(error.field_path, expected_path);
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

    #[test]
    fn result_codec_round_trips_coherent_generated_carriers(
        seed_logical_id in "[A-Za-z0-9][A-Za-z0-9._:-]{0,31}",
        target_logical_id in "[A-Za-z0-9][A-Za-z0-9._:-]{0,31}",
        predecessor_logical_id in "[A-Za-z0-9][A-Za-z0-9._:-]{0,31}",
        body in "[ -~]{1,64}",
        write_cursor in any::<u64>(),
        work_units in any::<u64>(),
        score_numerator in proptest::option::of(-10_000_i16..=10_000),
        include_evidence in any::<bool>(),
    ) {
        let query_score = score_numerator.map(|score| f64::from(score) / 10.0);
        let value = coherent_response(CoherentResponseInput {
            seed_logical_id: &seed_logical_id,
            target_logical_id: &target_logical_id,
            predecessor_logical_id: &predecessor_logical_id,
            body: &body,
            write_cursor,
            work_units,
            query_score,
            include_evidence,
        });
        let decoded = decode_graph_expand_result_v1(&serde_json::to_vec(&value).unwrap()).unwrap();
        let encoded = encode_graph_expand_result_v1(&decoded).unwrap();
        let encoded_value: Value = serde_json::from_slice(&encoded).unwrap();

        prop_assert_eq!(&encoded_value["targets"][0]["writeCursor"], &json!(write_cursor.to_string()));
        prop_assert_eq!(&encoded_value["workUnits"], &json!(work_units.to_string()));
        prop_assert_eq!(decode_graph_expand_result_v1(&encoded).unwrap(), decoded);
    }

    #[test]
    fn result_codec_rejects_generated_nonzero_first_evidence_position(
        target_index in 1_u32..=u32::MAX,
        target_logical_id in "[A-Za-z0-9][A-Za-z0-9._:-]{0,31}",
        write_cursor in any::<u64>(),
    ) {
        let mut value = coherent_response(CoherentResponseInput {
            seed_logical_id: "seed-a",
            target_logical_id: &target_logical_id,
            predecessor_logical_id: "middle-b",
            body: "generated target body",
            write_cursor,
            work_units: 2,
            query_score: None,
            include_evidence: true,
        });
        value["evidence"]["entries"][0]["targetIndex"] = json!(target_index);

        let error = decode_response(&value);
        prop_assert_eq!(error.reason, GraphExpansionErrorReasonV1::GraphCorrupt);
        prop_assert_eq!(error.field_path, "/evidence/entries/0");
    }
}
