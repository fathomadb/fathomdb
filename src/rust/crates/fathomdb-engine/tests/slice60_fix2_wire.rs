//! Slice 60 FIX-2 RED: executable wire precedence and Unicode byte parity.

use fathomdb_engine::{
    decode_graph_expand_request_v1, decode_graph_expand_result_v1, encode_graph_expand_request_v1,
    encode_graph_expand_result_v1, GraphExpandRequestV1, GraphExpansionErrorReasonV1,
    GraphReadContextV1, GraphSeedV1, ReadContextV1, ReadView, SearchFilter, TraversalDirection,
};
use serde_json::{json, Value};

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../../../../dev/fixtures/slice60-fix2-unicode-v1.json"))
        .unwrap()
}

fn unicode_request() -> GraphExpandRequestV1 {
    GraphExpandRequestV1 {
        schema_version: 1,
        seed: GraphSeedV1::Query { schema_version: 1, text: "café".into(), ranked_limit: 1 },
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
        include_explanation: false,
        include_evidence: false,
    }
}

fn context_error(context: Value) -> fathomdb_engine::GraphExpansionErrorV1 {
    let mut request: Value = serde_json::from_str(include_str!(
        "../../../../../dev/fixtures/slice60-graph-expand-conformance-v1.json"
    ))
    .unwrap();
    request["request"]["context"] = context;
    decode_graph_expand_request_v1(&serde_json::to_vec(&request["request"]).unwrap()).unwrap_err()
}

#[test]
fn context_union_closes_before_missing_or_invalid_discriminant_with_escaped_path() {
    for context in [
        json!({"schemaVersion": 1, "a/b~c": true}),
        json!({"schemaVersion": 1, "type": "invalid", "a/b~c": true}),
    ] {
        let error = context_error(context);
        assert_eq!(error.reason, GraphExpansionErrorReasonV1::UnknownField);
        assert_eq!(error.field_path, "/context/a~1b~0c");
    }
}

#[test]
fn context_union_reports_discriminant_only_after_closure() {
    for context in [json!({"schemaVersion": 1}), json!({"schemaVersion": 1, "type": "invalid"})] {
        let error = context_error(context);
        assert_eq!(error.reason, GraphExpansionErrorReasonV1::GraphContextInvalid);
        assert_eq!(error.field_path, "/context/type");
    }
}

#[test]
fn unicode_request_and_native_result_are_raw_utf8_declaration_order_bytes() {
    let fixture = fixture();
    let request = fixture["request"].as_str().unwrap().as_bytes();
    let result = fixture["result"].as_str().unwrap().as_bytes();
    assert!(request.windows("café".len()).any(|window| window == "café".as_bytes()));
    assert_eq!(encode_graph_expand_request_v1(&unicode_request()).unwrap(), request);
    let decoded = decode_graph_expand_result_v1(result).unwrap();
    assert_eq!(encode_graph_expand_result_v1(&decoded).unwrap(), result);
}
