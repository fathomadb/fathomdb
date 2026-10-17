//! Typed contract (design §9 order 10): every tool declares input and
//! output schemas in JSON Schema draft 2020-12 (MR-071.2), and every
//! structured result — success or tool error — validates against its tool's
//! output schema (MR-071.1). The shared test client also validates every
//! `structuredContent` the rest of the suite receives.

mod common;

use common::{edge, node, Fixture, Harness};
use serde_json::{json, Value};

const DRAFT_2020_12: &str = "https://json-schema.org/draft/2020-12/schema";

fn compile(schema: &Value) -> jsonschema::JSONSchema {
    jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .compile(schema)
        .unwrap_or_else(|error| panic!("schema does not compile: {error}"))
}

#[tokio::test(flavor = "multi_thread")]
async fn every_tool_declares_draft_2020_12_input_and_output_schemas() {
    let harness = Harness::start(fathomdb_mcp::config::Config::default()).await;
    let tools = serde_json::to_value(harness.tools().await).unwrap();
    for tool in tools.as_array().unwrap() {
        let name = tool["name"].as_str().unwrap();
        for key in ["inputSchema", "outputSchema"] {
            let schema = &tool[key];
            assert_eq!(schema["$schema"], DRAFT_2020_12, "{name} {key}: {schema}");
            assert_eq!(schema["type"], "object", "{name} {key} has an object root");
            compile(schema);
        }
    }
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn output_schemas_are_not_vacuous() {
    let harness = Harness::start(fathomdb_mcp::config::Config::default()).await;
    let tools = serde_json::to_value(harness.tools().await).unwrap();
    let schema_of = |name: &str| -> jsonschema::JSONSchema {
        let tool = tools.as_array().unwrap().iter().find(|tool| tool["name"] == name).unwrap();
        compile(&tool["outputSchema"])
    };
    let list = schema_of("list_stores");
    assert!(list.is_valid(&json!({"stores": []})));
    assert!(!list.is_valid(&json!({"stores": "none"})));
    assert!(!list.is_valid(&json!({})));
    let search = schema_of("search");
    assert!(!search.is_valid(&json!({"results": []})), "the envelope is required");
    assert!(!search.is_valid(&json!({"error": {"class": "nope"}})), "error classes are closed");
    assert!(search.is_valid(&json!({"error": {
        "class": "busy", "code": "admission", "message": "m", "remedy": "r"
    }})));
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn every_tool_result_and_error_validates_against_its_output_schema() {
    let fixture = Fixture::new();
    fixture.store(
        "a.sqlite",
        vec![
            node("note:1", "note", "alpha river crossing"),
            node("note:2", "note", "beta river crossing"),
            edge("link", "note:1", "note:2"),
        ],
    );
    let harness = Harness::start(fixture.config(json!({
        "stores": {"a": {"path": "a.sqlite"}, "missing": {"path": "nope.sqlite"}},
        "telemetry_dir": "telemetry"
    })))
    .await;
    // Each call is validated by the shared client; these cover every result
    // shape, including per-item errors, explanations and tool errors.
    harness.ok("list_stores", json!({})).await;
    harness.ok("use_store", json!({"store": "a"})).await;
    harness.ok("orient", json!({})).await;
    let text = harness.ok("search", json!({"query": "river", "limit": 1, "explain": true})).await;
    harness.ok("search", json!({"continuation": text["continuation"]})).await;
    let hybrid =
        harness.ok("search", json!({"query": "river", "mode": "hybrid", "explain": true})).await;
    harness.ok("search", json!({"query": "zebra", "mode": "hybrid"})).await;
    harness.ok("fetch", json!({"refs": ["l:note:1", "l:none", "h:x", "bad"]})).await;
    harness.ok("expand", json!({"refs": ["l:note:1"], "limit": 1})).await;
    harness
        .ok("feedback", json!({"response_id": hybrid["response_id"], "used": ["l:note:1"]}))
        .await;
    harness.ok("feedback", json!({"response_id": text["response_id"], "used": []})).await;
    harness.err("use_store", json!({"store": "nope"})).await;
    harness.err("orient", json!({"store": "missing"})).await;
    harness.err("search", json!({"query": ""})).await;
    harness.err("fetch", json!({"refs": []})).await;
    harness.err("expand", json!({"refs": ["h:x"]})).await;
    harness.err("feedback", json!({"response_id": hybrid["response_id"], "used": ["l:x"]})).await;
    harness.err("list_stores", json!({"unexpected": 1})).await;
    harness.shutdown().await;
}
