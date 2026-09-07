//! Slice 60 FIX-3 RED: RFC 6901 tilde-only unknown-field paths.

use fathomdb_engine::{decode_graph_expand_request_v1, GraphExpansionErrorReasonV1};
use serde_json::Value;

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../../../../dev/fixtures/slice60-fix3-rfc6901-v1.json"))
        .unwrap()
}

fn request() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../dev/fixtures/slice60-graph-expand-conformance-v1.json"
    ))
    .unwrap()
}

fn error(value: &Value) -> fathomdb_engine::GraphExpansionErrorV1 {
    decode_graph_expand_request_v1(&serde_json::to_vec(&value["request"]).unwrap()).unwrap_err()
}

#[test]
fn top_level_tilde_only_unknown_uses_rfc6901_escape() {
    let mut value = request();
    let fixture = fixture();
    let field = fixture["topLevelUnknown"].as_str().unwrap();
    value["request"][field] = true.into();
    let error = error(&value);
    assert_eq!(error.reason, GraphExpansionErrorReasonV1::UnknownField);
    assert_eq!(error.field_path, fixture["topLevelPath"].as_str().unwrap());
}

#[test]
fn nested_tilde_only_unknown_uses_rfc6901_escape() {
    let mut value = request();
    let fixture = fixture();
    let field = fixture["nestedContextUnknown"].as_str().unwrap();
    value["request"]["context"][field] = true.into();
    let error = error(&value);
    assert_eq!(error.reason, GraphExpansionErrorReasonV1::UnknownField);
    assert_eq!(error.field_path, fixture["nestedContextPath"].as_str().unwrap());
}
