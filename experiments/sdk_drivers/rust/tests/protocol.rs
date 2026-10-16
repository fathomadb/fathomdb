use fathomdb_sdk_benchmark_driver::{handle_request, parse_request};
use serde_json::json;

fn request() -> serde_json::Value {
    json!({
        "schema_version": "sdk.driver.v1",
        "mode": "run-repetition",
        "sdk": "rust",
        "operation_id": "engine.search_text_only",
        "operation_version": 1,
        "database_path": "/tmp/nonexistent-sdk-driver-fixture.sqlite",
        "queries": [{"id": "q1", "text": "alpha"}],
        "arguments": {"limit": 10, "view": {"valid_as_of": 1700000000}},
        "warmup_per_query": 0,
        "timeout_ms": 1000,
        "expected_native_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    })
}

#[test]
fn rejects_unknown_options_before_open() {
    let mut raw = request();
    raw["arguments"]["unregistered"] = json!(true);
    assert!(parse_request(&raw.to_string()).is_err());
}

#[test]
fn rejects_missing_fixed_validity_before_open() {
    let mut raw = request();
    raw["arguments"]["view"] = json!({});
    assert!(parse_request(&raw.to_string()).is_err());
}

#[test]
fn capabilities_advertise_only_implemented_operations() {
    let response = handle_request(r#"{"schema_version":"sdk.driver.v1","mode":"capabilities"}"#);
    assert_eq!(response["status"], "complete");
    assert_eq!(
        response["operations"],
        json!([
            {"operation_id":"engine.search_text_only","operation_version":1},
            {"operation_id":"engine.search_projected_text","operation_version":1},
            {"operation_id":"engine.search","operation_version":1},
            {"operation_id":"read.get","operation_version":1},
            {"operation_id":"graph.neighbors","operation_version":1}
        ])
    );
    assert!(response["identity"]["runtime"].as_str().unwrap().starts_with("rustc "));
}

#[test]
fn operation_options_are_closed_and_typed() {
    let mut read = request();
    read["operation_id"] = json!("read.get");
    read["arguments"] = json!({"view": {"valid_as_of": 1_700_000_000}});
    assert!(parse_request(&read.to_string()).is_ok());
    read["arguments"]["limit"] = json!(10);
    assert!(parse_request(&read.to_string()).is_err());

    let mut graph = request();
    graph["operation_id"] = json!("graph.neighbors");
    graph["arguments"] = json!({"depth": 1, "view": {"valid_as_of": 1_700_000_000}});
    assert!(parse_request(&graph.to_string()).is_ok());
    graph["arguments"]["depth"] = json!(4);
    assert!(parse_request(&graph.to_string()).is_err());
}

#[test]
fn binary_digest_mismatch_blocks_before_database_open() {
    let response = handle_request(&request().to_string());
    assert_eq!(response["status"], "blocked");
    assert_eq!(response["error"]["kind"], "ProtocolError");
    assert_eq!(response["error"]["message"], "driver binary SHA-256 mismatch");
    assert_eq!(response["samples"], json!([]));
}
