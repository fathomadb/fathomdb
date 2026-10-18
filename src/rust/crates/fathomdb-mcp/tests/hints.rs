//! Hints sidecar (`dev/design/agent-access/hints-sidecar.md`; ADR-0.8.29):
//! configuration and per-call argument, the reasons each tool emits, their
//! bounds, trust rules and budget accounting, against real stores through
//! the in-process protocol harness.

mod common;

use common::{node, Fixture, Harness};
use fathomdb_sdk::PreparedWrite;
use serde_json::{json, Value};

fn river_notes(count: usize, body_len: usize) -> Vec<PreparedWrite> {
    (0..count)
        .map(|i| {
            let mut body = format!("river crossing entry number {i} ");
            while body.len() < body_len {
                body.push_str("and the water ran on past the old stone bridge ");
            }
            node(&format!("note:{i:03}"), "note", &body)
        })
        .collect()
}

async fn harness_with(writes: Vec<PreparedWrite>, extra: Value) -> (Fixture, Harness) {
    let fixture = Fixture::new();
    fixture.store("a.sqlite", writes);
    let mut config = json!({"stores": {"a": {"path": "a.sqlite"}}, "default_store": "a"});
    for (key, value) in extra.as_object().unwrap() {
        config[key] = value.clone();
    }
    let harness = Harness::start(fixture.config(config)).await;
    (fixture, harness)
}

/// § 5: every tool declares and accepts a per-call `hints` boolean,
/// including a continuation call.
#[tokio::test(flavor = "multi_thread")]
async fn every_tool_accepts_a_hints_argument() {
    let (_fixture, harness) = harness_with(river_notes(12, 60), json!({})).await;
    for tool in harness.tools().await {
        let schema = serde_json::to_value(&tool.input_schema).unwrap();
        assert_eq!(
            schema["properties"]["hints"]["type"],
            json!(["boolean", "null"]),
            "{}: {schema}",
            tool.name
        );
    }
    harness.ok("list_stores", json!({"hints": false})).await;
    harness.ok("use_store", json!({"store": "a", "hints": true})).await;
    harness.ok("orient", json!({"hints": false})).await;
    let page = harness.ok("search", json!({"query": "river", "limit": 5, "hints": false})).await;
    let continuation = page["continuation"].as_str().expect("a full page continues").to_string();
    harness.ok("search", json!({"continuation": continuation, "hints": true})).await;
    harness.ok("fetch", json!({"refs": ["l:note:001"], "hints": false})).await;
    harness.ok("expand", json!({"refs": ["l:note:001"], "hints": false})).await;
    harness
        .ok("feedback", json!({"response_id": page["response_id"], "used": [], "hints": false}))
        .await;
    harness.shutdown().await;
}
