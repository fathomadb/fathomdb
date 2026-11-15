//! A supplied search view flag must be a boolean, including on continuations.
mod common;

use common::{node, text_of, Fixture, Harness};
use serde_json::{json, Value};

async fn start(format: &str) -> (Fixture, Harness) {
    let fixture = Fixture::new();
    fixture.store(
        "null.sqlite",
        vec![node("one", "note", "nullriver one"), node("two", "note", "nullriver two")],
    );
    let harness = Harness::start(fixture.config(json!({"stores":{"a":{"path":"null.sqlite"}},"default_store":"a","hints":false,"result_format":format}))).await;
    (fixture, harness)
}

fn field(result: &rmcp::model::CallToolResult, format: &str, name: &str) -> Option<String> {
    if format == "structured" {
        result.structured_content.as_ref().unwrap()[name].as_str().map(str::to_owned)
    } else {
        text_of(result)
            .lines()
            .next()
            .unwrap()
            .split_whitespace()
            .find_map(|word| word.strip_prefix(&format!("{name}=")).map(str::to_owned))
    }
}

#[tokio::test]
async fn superseded_null_is_rejected_while_absent_and_boolean_values_remain_valid() {
    for format in ["structured", "text"] {
        let (_fixture, harness) = start(format).await;
        let mut request = json!({"query":"nullriver","as_of":1900000000,"max_tokens":100_000});
        let absent = harness.call("search", request.clone()).await;
        assert_ne!(absent.is_error, Some(true));
        request["include_superseded"] = json!(false);
        let explicit_false = harness.call("search", request.clone()).await;
        assert_ne!(explicit_false.is_error, Some(true));
        assert_eq!(
            field(&absent, format, "response_id"),
            field(&explicit_false, format, "response_id")
        );
        request["include_superseded"] = json!(true);
        let explicit_true = harness.call("search", request.clone()).await;
        assert_ne!(explicit_true.is_error, Some(true));
        request["include_superseded"] = Value::Null;
        let routes = harness.adapter.seams().search_route_calls();
        let invalid = harness.call("search", request).await;
        assert_eq!(
            invalid.is_error,
            Some(true),
            "present null is not a boolean search view flag: {}",
            text_of(&invalid)
        );
        assert!(text_of(&invalid).contains("bad_argument"));
        assert_eq!(
            harness.adapter.seams().search_route_calls(),
            routes,
            "invalid flags never reach either SDK search route"
        );
        let tools = serde_json::to_value(harness.tools().await).unwrap();
        let schema =
            &tools.as_array().unwrap().iter().find(|tool| tool["name"] == "search").unwrap()
                ["inputSchema"];
        let schema = jsonschema::JSONSchema::options()
            .with_draft(jsonschema::Draft::Draft202012)
            .compile(schema)
            .unwrap();
        assert!(schema.is_valid(&json!({"query":"nullriver"})));
        assert!(schema.is_valid(&json!({"query":"nullriver","include_superseded":false})));
        assert!(schema.is_valid(&json!({"query":"nullriver","include_superseded":true})));
        assert!(!schema.is_valid(&json!({"query":"nullriver","include_superseded":null})));
        harness.shutdown().await;
    }
}

#[tokio::test]
async fn superseded_null_cannot_bypass_continuation_override_validation() {
    for format in ["structured", "text"] {
        let (_fixture, harness) = start(format).await;
        let first = harness
            .call(
                "search",
                json!({"query":"nullriver","limit":1,"as_of":1900000000,"max_tokens":100_000}),
            )
            .await;
        assert_ne!(first.is_error, Some(true));
        let handle =
            field(&first, format, "continuation").expect("two authored hits require another page");
        let routes = harness.adapter.seams().search_route_calls();
        let invalid =
            harness.call("search", json!({"continuation":handle,"include_superseded":null})).await;
        assert_eq!(
            invalid.is_error,
            Some(true),
            "null must not erase a supplied view override: {}",
            text_of(&invalid)
        );
        assert!(text_of(&invalid).contains("bad_argument"));
        assert_eq!(
            harness.adapter.seams().search_route_calls(),
            routes,
            "refuse before any continuation SDK invocation"
        );
        harness.shutdown().await;
    }
}
