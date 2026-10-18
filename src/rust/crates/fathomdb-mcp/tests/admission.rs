//! Admission queue (queue-admission § 6.1, tests of § 9): a call that finds
//! every permit of its store held waits in FIFO order while its deadline
//! allows, and is refused as `busy/admission` with a reason otherwise.

mod common;

use std::time::{Duration, Instant};

use common::{node, Fixture, Harness};
use rmcp::model::{CallToolRequestParams, CallToolResult};
use serde_json::{json, Value};
use tokio::task::JoinHandle;

fn store_config(fixture: &Fixture, extra: Value) -> fathomdb_mcp::config::Config {
    fixture.store(
        "a.sqlite",
        vec![
            node("note:a", "note", "alpha river crossing"),
            node("note:b", "note", "beta river crossing"),
        ],
    );
    let mut config = json!({"stores": {"a": {"path": "a.sqlite"}}, "idle_timeout_secs": 0});
    for (key, value) in extra.as_object().unwrap() {
        config[key] = value.clone();
    }
    fixture.config(config)
}

/// Start a call on its own task through the harness client.
fn spawn_call(harness: &Harness, tool: &str, args: Value) -> JoinHandle<CallToolResult> {
    let peer = harness.client.peer().clone();
    let tool = tool.to_string();
    tokio::spawn(async move {
        let Value::Object(arguments) = args else { panic!("arguments must be an object") };
        let result = peer
            .call_tool(CallToolRequestParams::new(tool.clone()).with_arguments(arguments))
            .await
            .unwrap_or_else(|error| panic!("{tool}: protocol error {error:?}"));
        if let Some(structured) = &result.structured_content {
            common::assert_output_valid(&tool, structured);
        }
        result
    })
}

fn search_a() -> Value {
    json!({"store": "a", "query": "alpha"})
}

#[tokio::test(flavor = "multi_thread")]
async fn an_excess_call_waits_for_a_permit_and_then_runs() {
    let fixture = Fixture::new();
    let harness = Harness::start(store_config(&fixture, json!({"admission_cap": 1}))).await;
    harness.ok("orient", json!({"store": "a"})).await;
    harness.adapter.seams().block_calls("a");
    let first = spawn_call(&harness, "search", search_a());
    tokio::time::sleep(Duration::from_millis(100)).await;
    let started = Instant::now();
    let second = spawn_call(&harness, "search", search_a());
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(!second.is_finished(), "the second call waits for the permit");
    harness.adapter.seams().release_calls("a");
    let second = second.await.unwrap();
    assert_ne!(second.is_error, Some(true), "the waiter runs: {:?}", second.structured_content);
    assert!(started.elapsed() >= Duration::from_millis(200));
    let first = first.await.unwrap();
    assert_ne!(first.is_error, Some(true), "{:?}", first.structured_content);
    harness.shutdown().await;
}

fn error_of(result: &CallToolResult) -> Value {
    assert_eq!(
        result.is_error,
        Some(true),
        "expected a tool error: {:?}",
        result.structured_content
    );
    result.structured_content.as_ref().expect("structuredContent")["error"].clone()
}

#[tokio::test(flavor = "multi_thread")]
async fn an_excess_call_is_refused_when_too_little_deadline_remains() {
    let fixture = Fixture::new();
    let harness = Harness::start(store_config(
        &fixture,
        json!({"admission_cap": 1, "call_deadline_ms": 1000}),
    ))
    .await;
    harness.ok("orient", json!({"store": "a"})).await;
    harness.adapter.seams().block_calls("a");
    let first = spawn_call(&harness, "search", search_a());
    tokio::time::sleep(Duration::from_millis(100)).await;
    let started = Instant::now();
    let second = spawn_call(&harness, "search", search_a()).await.unwrap();
    let elapsed = started.elapsed();
    let error = error_of(&second);
    assert_eq!(error["class"], "busy", "{error}");
    assert_eq!(error["code"], "admission");
    assert_eq!(error["busy"]["reason"], "deadline", "{error}");
    assert_eq!(error["busy"]["in_flight"], 1, "{error}");
    // A cold tool needs 50 ms, so the wait ends at deadline - 50 ms.
    assert!(elapsed >= Duration::from_millis(850), "refused too early: {elapsed:?}");
    assert!(elapsed <= Duration::from_millis(990), "refused at the deadline: {elapsed:?}");
    harness.adapter.seams().release_calls("a");
    let _ = first.await.unwrap();
    harness.shutdown().await;
}
