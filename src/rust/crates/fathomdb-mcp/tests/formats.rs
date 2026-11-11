//! The compact wire contract, through real MCP connections. Each connection
//! captures and validates its own schemas; no global schema cache is used.
mod common;

use common::Harness;
use fathomdb_mcp::config::Config;
use rmcp::ServerHandler;

#[tokio::test]
async fn default_discovery_has_no_output_schema() {
    let text = Harness::start(Config::default()).await;
    let structured = Harness::start(
        Config::from_json_str(r#"{"result_format":"structured"}"#, std::path::Path::new("."))
            .unwrap(),
    )
    .await;
    let a = text.tools().await;
    let b = structured.tools().await;
    let mut names: Vec<_> = a.iter().map(|tool| tool.name.as_ref()).collect();
    names.sort();
    assert_eq!(
        names,
        ["expand", "feedback", "fetch", "list_stores", "orient", "search", "use_store"]
    );
    for tool in &a {
        assert!(
            tool.output_schema.is_none(),
            "default {} must not advertise outputSchema",
            tool.name
        );
        let other = b.iter().find(|other| other.name == tool.name).unwrap();
        assert!(other.output_schema.is_some());
        assert_eq!(tool.input_schema, other.input_schema);
        assert!(text.adapter.connection().get_tool(&tool.name).unwrap().output_schema.is_none());
        assert!(structured
            .adapter
            .connection()
            .get_tool(&tool.name)
            .unwrap()
            .output_schema
            .is_some());
    }
    text.shutdown().await;
    structured.shutdown().await;
}

use common::{node, text_of, Fixture};
use rmcp::model::{CallToolRequestParams, CallToolResult};
use serde_json::{json, Value};

async fn wire(harness: &Harness, tool: &str, args: Value) -> CallToolResult {
    let tools = harness.tools().await;
    let result = harness
        .client
        .call_tool(
            CallToolRequestParams::new(tool.to_string())
                .with_arguments(args.as_object().unwrap().clone()),
        )
        .await
        .unwrap();
    if let Some(payload) = &result.structured_content {
        if let Some(schema) = &tools.iter().find(|entry| entry.name == tool).unwrap().output_schema
        {
            let schema = Value::Object(schema.as_ref().clone());
            let compiled = jsonschema::JSONSchema::options()
                .with_draft(jsonschema::Draft::Draft202012)
                .compile(&schema)
                .unwrap();
            assert!(compiled.is_valid(payload), "{tool}: {payload}");
        }
    }
    result
}

fn wire_config(fixture: &Fixture, extra: Value) -> Config {
    let mut config = json!({"stores":{"a":{"path":"a.sqlite"}},"default_store":"a","hints":false});
    for (key, value) in extra.as_object().unwrap() {
        config[key] = value.clone();
    }
    Config::from_json_str(&config.to_string(), fixture.dir.path()).unwrap()
}

#[tokio::test]
async fn list_stores_text_has_one_block_and_quoted_names() {
    let fixture = Fixture::new();
    fixture.store("a.sqlite", vec![node("one", "note", "river")]);
    let harness = Harness::start(wire_config(&fixture, json!({}))).await;
    let listed = wire(&harness, "list_stores", json!({})).await;
    assert!(listed.structured_content.is_none(), "default results must carry only text");
    assert_eq!(text_of(&listed), "stores: 1\n\"a\" open=false readiness=closed default");
    for (tool, args) in [
        ("use_store", json!({"store":"a"})),
        ("orient", json!({})),
        ("search", json!({"query":"river"})),
        ("fetch", json!({"refs":["l:one"]})),
        ("expand", json!({"refs":["l:one"]})),
        ("feedback", json!({"response_id":"expired","used":[]})),
    ] {
        let result = wire(&harness, tool, args).await;
        assert_ne!(result.is_error, Some(true), "{tool}: {}", text_of(&result));
        assert!(result.structured_content.is_none(), "{tool} default must carry only text");
        assert!(!text_of(&result).is_empty());
    }
    let error = wire(&harness, "search", json!({"query":""})).await;
    assert_eq!(error.is_error, Some(true));
    assert!(error.structured_content.is_none());
    text_of(&error);
    harness.shutdown().await;
}
