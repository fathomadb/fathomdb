//! Engine bridge (design §9 order 5): the call deadline returns a typed
//! timeout and the adapter keeps serving (MR-033.1); the admission cap counts
//! abandoned calls (MR-033.2); a panic on the blocking thread is a typed
//! internal error that releases its admission slot (design §6.4).

mod common;

use std::time::{Duration, Instant};

use common::{node, Fixture, Harness};
use serde_json::json;

fn harness_config(fixture: &Fixture, extra: serde_json::Value) -> fathomdb_mcp::config::Config {
    fixture.store("a.sqlite", vec![node("note:a", "note", "alpha river crossing")]);
    let mut config = json!({"stores": {"a": {"path": "a.sqlite"}}, "idle_timeout_secs": 0});
    for (key, value) in extra.as_object().unwrap() {
        config[key] = value.clone();
    }
    fixture.config(config)
}

#[tokio::test(flavor = "multi_thread")]
async fn a_blocked_engine_call_returns_a_typed_timeout_at_the_deadline() {
    let fixture = Fixture::new();
    let harness = Harness::start(harness_config(&fixture, json!({"call_deadline_ms": 500}))).await;
    harness.ok("orient", json!({"store": "a"})).await;
    harness.adapter.seams().block_calls("a");
    let started = Instant::now();
    let error = harness.err("search", json!({"store": "a", "query": "alpha"})).await;
    let elapsed = started.elapsed();
    assert_eq!(error["class"], "timeout");
    assert_eq!(error["code"], "deadline");
    assert!(elapsed >= Duration::from_millis(450), "returned early: {elapsed:?}");
    assert!(elapsed < Duration::from_millis(1500), "returned late: {elapsed:?}");
    // The adapter keeps serving while the abandoned call is still running.
    let listed = harness.ok("list_stores", json!({})).await;
    assert_eq!(listed["stores"][0]["readiness"], "ready");
    harness.adapter.seams().release_calls("a");
    let found = harness.ok("search", json!({"store": "a", "query": "alpha"})).await;
    assert_eq!(found["results"][0]["ref"], "l:note:a");
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_admission_cap_counts_abandoned_calls() {
    let fixture = Fixture::new();
    let harness = Harness::start(harness_config(
        &fixture,
        json!({"call_deadline_ms": 400, "admission_cap": 2}),
    ))
    .await;
    harness.ok("orient", json!({"store": "a"})).await;
    harness.adapter.seams().block_calls("a");
    for _ in 0..2 {
        let error = harness.err("search", json!({"store": "a", "query": "alpha"})).await;
        assert_eq!(error["class"], "timeout");
    }
    // Two abandoned calls still hold the store's two admission slots.
    let started = Instant::now();
    let error = harness.err("search", json!({"store": "a", "query": "alpha"})).await;
    assert_eq!(error["class"], "busy", "{error}");
    assert_eq!(error["code"], "admission");
    assert!(started.elapsed() < Duration::from_millis(300), "busy is immediate");

    harness.adapter.seams().release_calls("a");
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let result = harness.call("search", json!({"store": "a", "query": "alpha"})).await;
        if result.is_error != Some(true) {
            break;
        }
        assert!(Instant::now() < deadline, "slots were never released");
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn concurrent_calls_never_exceed_the_admission_cap() {
    let fixture = Fixture::new();
    let harness = Harness::start(harness_config(
        &fixture,
        json!({"call_deadline_ms": 600, "admission_cap": 2}),
    ))
    .await;
    harness.ok("orient", json!({"store": "a"})).await;
    harness.adapter.seams().block_calls("a");
    let mut calls = Vec::new();
    for _ in 0..5 {
        let client = harness.client.peer().clone();
        calls.push(tokio::spawn(async move {
            common::err_peer(&client, "search", json!({"store": "a", "query": "alpha"})).await
        }));
    }
    let mut classes = Vec::new();
    for call in calls {
        classes.push(call.await.unwrap()["class"].as_str().unwrap().to_string());
    }
    classes.sort();
    assert_eq!(classes, vec!["busy", "busy", "busy", "timeout", "timeout"]);
    harness.adapter.seams().release_calls("a");
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_panic_on_the_blocking_thread_is_internal_panic_and_frees_its_slot() {
    let fixture = Fixture::new();
    let harness = Harness::start(harness_config(&fixture, json!({"admission_cap": 1}))).await;
    harness.ok("orient", json!({"store": "a"})).await;
    harness.adapter.seams().panic_calls("a");
    let error = harness.err("search", json!({"store": "a", "query": "alpha"})).await;
    assert_eq!(error["class"], "internal");
    assert_eq!(error["code"], "panic");
    harness.adapter.seams().stop_panicking_calls("a");
    // With a cap of 1, this only succeeds if the panicked call released it.
    let found = harness.ok("search", json!({"store": "a", "query": "alpha"})).await;
    assert_eq!(found["results"][0]["ref"], "l:note:a");
    harness.shutdown().await;
}
