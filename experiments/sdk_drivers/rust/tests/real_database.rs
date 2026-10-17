use std::fs;
use std::process::Command;

use fathomdb_sdk::{
    admin, Engine, InitialState, OpenOptions, PreparedWrite, RuntimeSqliteMode, SourceId,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

#[test]
fn real_database_exercises_fts_read_and_graph_sdk_calls() {
    let directory = tempfile::tempdir().expect("temporary fixture directory");
    let database = directory.path().join("fixture.sqlite");
    admin::configure_runtime(RuntimeSqliteMode::Performance).expect("configure fixture runtime");
    let engine = Engine::open(&database, OpenOptions::default()).expect("open fixture");
    engine
        .write(&[
            PreparedWrite::Node {
                kind: "doc".into(),
                body: "amber otter".into(),
                source_id: SourceId::new("sdk:doc-1").expect("source ID"),
                logical_id: Some("doc-1".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: Some(1_700_000_001),
                valid_until: None,
            },
            PreparedWrite::Node {
                kind: "doc".into(),
                body: "blue fox".into(),
                source_id: SourceId::new("sdk:doc-2").expect("source ID"),
                logical_id: Some("doc-2".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            },
            PreparedWrite::Edge {
                kind: "links".into(),
                from: "doc-1".into(),
                to: "doc-2".into(),
                source_id: SourceId::new("sdk:edge-1").expect("source ID"),
                logical_id: None,
                body: None,
                t_valid: None,
                t_invalid: None,
                confidence: None,
                extractor_model_id: None,
                temporal_fallback: None,
            },
        ])
        .expect("write fixture");
    engine.drain(10_000).expect("drain fixture");
    engine.close().expect("close fixture");

    let binary = env!("CARGO_BIN_EXE_fathomdb-sdk-benchmark-driver");
    let digest = format!("{:x}", Sha256::digest(fs::read(binary).expect("read binary")));
    let request = json!({
        "schema_version": "sdk.driver.v1",
        "mode": "run-repetition",
        "sdk": "rust",
        "operation_id": "engine.search_text_only",
        "operation_version": 1,
        "database_path": database,
        "queries": [{"id": "q1", "text": "amber otter"}],
        "arguments": {"limit": 10, "view": {"valid_as_of": 1700000002}},
        "warmup_per_query": 1,
        "timeout_ms": 10000,
        "expected_native_sha256": digest
    });
    let request_file = directory.path().join("request.json");
    fs::write(&request_file, request.to_string()).expect("write request");
    let output = Command::new(binary).arg(&request_file).output().expect("run driver");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let response: Value = serde_json::from_slice(&output.stdout).expect("single JSON response");
    assert_eq!(response["status"], "complete", "{response}");
    if cfg!(target_os = "linux") {
        assert!(response["resources"]["driver_peak_rss_bytes"].as_u64().unwrap() > 0);
    }
    assert_eq!(response["samples"][0]["ids"], json!(["l:doc-1"]));
    assert_eq!(response["samples"][0]["query_id"], "q1");
    assert!(response["samples"][0]["elapsed_ns"].as_u64().is_some());
    assert_eq!(response["closed"], true);
    assert_eq!(response["error"], Value::Null);

    let mut before_validity = request;
    before_validity["arguments"]["view"]["valid_as_of"] = json!(1_700_000_000);
    fs::write(&request_file, before_validity.to_string()).expect("write boundary request");
    let output = Command::new(binary).arg(&request_file).output().expect("run boundary driver");
    let response: Value = serde_json::from_slice(&output.stdout).expect("boundary response");
    assert_eq!(response["status"], "complete", "{response}");
    assert_eq!(response["samples"][0]["ids"], json!([]));

    let mut read_request = before_validity;
    read_request["operation_id"] = json!("read.get");
    read_request["arguments"] = json!({"view": {"valid_as_of": 1_700_000_002}});
    read_request["queries"] = json!([{"id": "q1", "text": "doc-1"}]);
    fs::write(&request_file, read_request.to_string()).expect("write read request");
    let output = Command::new(binary).arg(&request_file).output().expect("run read driver");
    let response: Value = serde_json::from_slice(&output.stdout).expect("read response");
    assert_eq!(response["status"], "complete", "{response}");
    assert_eq!(response["samples"][0]["ids"], json!(["l:doc-1"]));

    read_request["arguments"]["view"]["valid_as_of"] = json!(1_700_000_000);
    fs::write(&request_file, read_request.to_string()).expect("write read boundary request");
    let output = Command::new(binary).arg(&request_file).output().expect("run read boundary");
    let response: Value = serde_json::from_slice(&output.stdout).expect("read boundary response");
    assert_eq!(response["status"], "complete", "{response}");
    assert_eq!(response["samples"][0]["ids"], json!([]));

    let mut graph_request = read_request;
    graph_request["operation_id"] = json!("graph.neighbors");
    graph_request["arguments"] = json!({"depth": 1, "view": {"valid_as_of": 1_700_000_002}});
    fs::write(&request_file, graph_request.to_string()).expect("write graph request");
    let output = Command::new(binary).arg(&request_file).output().expect("run graph driver");
    let response: Value = serde_json::from_slice(&output.stdout).expect("graph response");
    assert_eq!(response["status"], "complete", "{response}");
    assert_eq!(response["samples"][0]["ids"], json!(["l:doc-2"]));
}
