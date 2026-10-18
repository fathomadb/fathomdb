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

#[tokio::test(flavor = "multi_thread")]
async fn input_schemas_declare_the_argument_bounds() {
    let harness = Harness::start(fathomdb_mcp::config::Config::default()).await;
    let tools = serde_json::to_value(harness.tools().await).unwrap();
    let property = |tool: &str, name: &str| -> Value {
        let tool = tools.as_array().unwrap().iter().find(|t| t["name"] == tool).unwrap();
        tool["inputSchema"]["properties"][name].clone()
    };
    let bounds = |schema: &Value, low: &str, high: &str| -> (Value, Value) {
        (schema[low].clone(), schema[high].clone())
    };
    assert_eq!(
        bounds(&property("fetch", "refs"), "minItems", "maxItems"),
        (json!(1), json!(50)),
        "fetch.refs"
    );
    assert_eq!(
        bounds(&property("expand", "refs"), "minItems", "maxItems"),
        (json!(1), json!(10)),
        "expand.refs"
    );
    assert_eq!(
        bounds(&property("expand", "max_depth"), "minimum", "maximum"),
        (json!(0), json!(3)),
        "expand.max_depth"
    );
    assert_eq!(
        bounds(&property("expand", "limit"), "minimum", "maximum"),
        (json!(1), json!(50)),
        "expand.limit"
    );
    assert_eq!(
        bounds(&property("search", "limit"), "minimum", "maximum"),
        (json!(1), json!(100)),
        "search.limit"
    );
    for tool in ["search", "fetch", "expand"] {
        assert_eq!(property(tool, "max_tokens")["minimum"], json!(256), "{tool}.max_tokens");
    }
    harness.shutdown().await;
}

/// Hints-sidecar § 8 test 15 and § 2.1: every hint reason the adapter emits
/// validates against its tool's output schema (the shared client checks
/// each result), and the schema's hint enums are closed.
#[tokio::test(flavor = "multi_thread")]
async fn hints_validate_against_output_schema() {
    let fixture = Fixture::new();
    let long = "river crossing told at length with the water high ".repeat(40);
    let mut writes: Vec<_> =
        (0..12).map(|i| node(&format!("note:{i:02}"), "note", &format!("{i} {long}"))).collect();
    writes.push(edge("link", "note:00", "note:01"));
    fixture.store("a.sqlite", writes);
    let harness = Harness::start(fixture.config(json!({
        "stores": {"a": {"path": "a.sqlite"}},
        "default_store": "a",
        "snippet_chars": 40
    })))
    .await;
    let reasons = |result: &Value| -> Vec<String> {
        result["hints"]
            .as_array()
            .map(|hints| {
                hints.iter().map(|hint| hint["reason"].as_str().unwrap().to_string()).collect()
            })
            .unwrap_or_default()
    };
    let mut seen: Vec<String> = Vec::new();
    seen.extend(reasons(&harness.ok("list_stores", json!({})).await));
    let page = harness.ok("search", json!({"query": "river", "limit": 3})).await;
    seen.extend(reasons(&page));
    seen.extend(reasons(&harness.ok("search", json!({"query": "river", "max_tokens": 400})).await));
    seen.extend(reasons(&harness.ok("search", json!({"query": "zebra", "mode": "hybrid"})).await));
    seen.extend(reasons(
        &harness.ok("fetch", json!({"refs": ["l:note:01"], "max_tokens": 256})).await,
    ));
    seen.extend(reasons(
        &harness.ok("expand", json!({"refs": ["l:note:00"], "max_tokens": 256})).await,
    ));
    seen.sort();
    seen.dedup();
    assert_eq!(
        seen,
        [
            "body_cut",
            "cut_by_budget",
            "default_store_set",
            "degraded",
            "more_results",
            "no_results",
            "snippet_cut"
        ]
    );

    let tools = serde_json::to_value(harness.tools().await).unwrap();
    let tool = tools.as_array().unwrap().iter().find(|tool| tool["name"] == "search").unwrap();
    let schema = compile(&tool["outputSchema"]);
    let mut valid = page.clone();
    assert!(schema.is_valid(&valid));
    valid["hints"] = json!([{"action": "search", "reason": "weak_results"}]);
    assert!(!schema.is_valid(&valid), "an unknown reason is rejected");
    valid["hints"] = json!([{"action": "retry", "reason": "no_results"}]);
    assert!(!schema.is_valid(&valid), "an unknown action is rejected");
    valid["hints"] =
        json!([{"action": "none", "reason": "degraded", "arm": "vector", "cause": "tired"}]);
    assert!(!schema.is_valid(&valid), "an unknown cause is rejected");
    harness.shutdown().await;
}
