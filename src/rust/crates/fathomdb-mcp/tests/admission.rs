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

#[tokio::test(flavor = "multi_thread")]
async fn a_full_queue_refuses_at_once_with_queue_full() {
    let fixture = Fixture::new();
    let harness = Harness::start(store_config(
        &fixture,
        json!({"admission_cap": 1, "queue_limit": 1, "call_deadline_ms": 3000}),
    ))
    .await;
    harness.ok("orient", json!({"store": "a"})).await;
    harness.adapter.seams().block_calls("a");
    let running = spawn_call(&harness, "search", search_a());
    tokio::time::sleep(Duration::from_millis(100)).await;
    let waiting = spawn_call(&harness, "search", search_a());
    tokio::time::sleep(Duration::from_millis(100)).await;
    let started = Instant::now();
    let third = spawn_call(&harness, "search", search_a()).await.unwrap();
    let elapsed = started.elapsed();
    let error = error_of(&third);
    assert_eq!(error["class"], "busy", "{error}");
    assert_eq!(error["code"], "admission");
    assert_eq!(error["busy"]["reason"], "queue_full", "{error}");
    assert_eq!(error["busy"]["in_flight"], 1, "{error}");
    assert_eq!(error["busy"]["queued"], 1, "{error}");
    assert!(elapsed < Duration::from_millis(50), "refused at once: {elapsed:?}");
    assert!(!waiting.is_finished(), "the queued call keeps waiting");
    harness.adapter.seams().release_calls("a");
    assert_ne!(waiting.await.unwrap().is_error, Some(true), "the queued call runs");
    let _ = running.await.unwrap();
    harness.shutdown().await;
}

/// Poll `condition` every 5 ms; panics after `timeout`.
async fn wait_for(what: &str, timeout: Duration, condition: impl Fn() -> bool) {
    let deadline = Instant::now() + timeout;
    while !condition() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn waiters_are_admitted_in_arrival_order() {
    let fixture = Fixture::new();
    let harness = Harness::start(store_config(&fixture, json!({"admission_cap": 1}))).await;
    harness.ok("orient", json!({"store": "a"})).await;
    let seams = harness.adapter.seams().clone();
    seams.block_calls("a");
    let completed = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut calls = Vec::new();
    for index in 0..4 {
        let call = spawn_call(&harness, "search", search_a());
        let completed = completed.clone();
        calls.push(tokio::spawn(async move {
            let result = call.await.unwrap();
            completed.lock().unwrap().push(index);
            result
        }));
        if index == 0 {
            wait_for("the first call to be held", Duration::from_secs(5), || {
                seams.held_calls("a") == 1
            })
            .await;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    for done in 1..=4 {
        wait_for("the next call to be held", Duration::from_secs(5), || seams.held_calls("a") == 1)
            .await;
        assert!(seams.release_one_call("a"));
        wait_for("the released call to complete", Duration::from_secs(5), || {
            completed.lock().unwrap().len() == done
        })
        .await;
    }
    assert_eq!(*completed.lock().unwrap(), vec![0, 1, 2, 3], "completion order is arrival order");
    for call in calls {
        let result = call.await.unwrap();
        assert_ne!(result.is_error, Some(true), "{:?}", result.structured_content);
    }
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_client_cancel_removes_a_queued_call() {
    let fixture = Fixture::new();
    let harness = Harness::start(store_config(
        &fixture,
        json!({"admission_cap": 1, "queue_limit": 2, "call_deadline_ms": 5000}),
    ))
    .await;
    harness.ok("orient", json!({"store": "a"})).await;
    let seams = harness.adapter.seams().clone();
    seams.block_calls("a");
    let running = spawn_call(&harness, "search", search_a());
    wait_for("the running call to be held", Duration::from_secs(5), || seams.held_calls("a") == 1)
        .await;
    // The call to cancel waits on its own connection, so it can be cancelled
    // by request id.
    let mut raw = common::RawClient::connect(&harness.adapter).await;
    let cancelled =
        raw.send_request("tools/call", json!({"name": "search", "arguments": search_a()})).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    let next = spawn_call(&harness, "search", search_a());
    tokio::time::sleep(Duration::from_millis(100)).await;
    let full = error_of(&spawn_call(&harness, "search", search_a()).await.unwrap());
    assert_eq!(full["busy"]["reason"], "queue_full", "two calls wait: {full}");
    assert_eq!(full["busy"]["queued"], 2, "{full}");

    raw.cancel(cancelled).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    // The cancelled call left the line: one more call fits.
    let later = spawn_call(&harness, "search", search_a());
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(!later.is_finished(), "a call fits in the line again");
    let full = error_of(&spawn_call(&harness, "search", search_a()).await.unwrap());
    assert_eq!(full["busy"]["reason"], "queue_full", "{full}");
    assert_eq!(full["busy"]["queued"], 2, "the cancelled call is not counted: {full}");

    // On release the next waiter is admitted, not the cancelled call.
    assert!(seams.release_one_call("a"));
    let _ = running.await.unwrap();
    wait_for("the next waiter to be held", Duration::from_secs(5), || seams.held_calls("a") == 1)
        .await;
    assert!(seams.release_one_call("a"));
    let next = next.await.unwrap();
    assert_ne!(next.is_error, Some(true), "{:?}", next.structured_content);
    wait_for("the later waiter to be held", Duration::from_secs(5), || seams.held_calls("a") == 1)
        .await;
    assert!(seams.release_one_call("a"));
    let later = later.await.unwrap();
    assert_ne!(later.is_error, Some(true), "{:?}", later.structured_content);
    harness.shutdown().await;
}

/// With every permit held and `queue_limit` 1, a new call is queued and
/// refused at its deadline only if nothing else is counted as queued.
async fn assert_nothing_queued(harness: &Harness, after: &str) {
    let error = error_of(&spawn_call(harness, "search", search_a()).await.unwrap());
    assert_eq!(error["busy"]["reason"], "deadline", "after {after}: {error}");
    assert_eq!(error["busy"]["queued"], 0, "after {after}: {error}");
}

#[tokio::test(flavor = "multi_thread")]
async fn queued_returns_to_zero_on_timeout_cancel_and_panic() {
    let fixture = Fixture::new();
    let harness = Harness::start(store_config(
        &fixture,
        json!({"admission_cap": 1, "queue_limit": 1, "call_deadline_ms": 600}),
    ))
    .await;
    harness.ok("orient", json!({"store": "a"})).await;
    let seams = harness.adapter.seams().clone();
    seams.block_calls("a");
    let running = spawn_call(&harness, "search", search_a());
    wait_for("the running call to be held", Duration::from_secs(5), || seams.held_calls("a") == 1)
        .await;

    // Timeout.
    let error = error_of(&spawn_call(&harness, "search", search_a()).await.unwrap());
    assert_eq!(error["busy"]["reason"], "deadline", "{error}");
    assert_nothing_queued(&harness, "a timeout").await;

    // Client cancel.
    let mut raw = common::RawClient::connect(&harness.adapter).await;
    let cancelled =
        raw.send_request("tools/call", json!({"name": "search", "arguments": search_a()})).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    raw.cancel(cancelled).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_nothing_queued(&harness, "a cancel").await;

    // Panic in the waiting task.
    seams.panic_queued_calls("a");
    let Value::Object(arguments) = search_a() else { unreachable!() };
    let panicked = harness
        .client
        .call_tool(CallToolRequestParams::new("search").with_arguments(arguments))
        .await;
    assert!(panicked.is_err(), "a panicking handler is a JSON-RPC error: {panicked:?}");
    seams.stop_panicking_queued_calls("a");
    assert_nothing_queued(&harness, "a panic").await;

    seams.release_calls("a");
    let _ = running.await.unwrap();
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn in_flight_is_derived_from_permits() {
    let fixture = Fixture::new();
    fixture.store("b.sqlite", vec![node("note:c", "note", "gamma river crossing")]);
    let harness = Harness::start(store_config(
        &fixture,
        json!({
            "stores": {"a": {"path": "a.sqlite"}, "b": {"path": "b.sqlite"}},
            "admission_cap": 1,
            "idle_timeout_secs": 1,
            "call_deadline_ms": 1000
        }),
    ))
    .await;
    harness.ok("orient", json!({"store": "a"})).await;
    let seams = harness.adapter.seams().clone();
    seams.hold_admitted_calls("a");
    let admitted = spawn_call(&harness, "search", search_a());
    wait_for("a call between permit and guard", Duration::from_secs(5), || {
        seams.admitted_held("a") == 1
    })
    .await;
    let held_at = Instant::now();
    // A call holding a permit is in flight before its guard exists.
    let error = error_of(&spawn_call(&harness, "search", search_a()).await.unwrap());
    assert_eq!(error["busy"]["reason"], "deadline", "{error}");
    assert_eq!(error["busy"]["in_flight"], 1, "{error}");
    // ... so the open-store limit cannot drain `a` for `b` ...
    let other = harness.call("search", json!({"store": "b", "query": "gamma"})).await;
    assert_eq!(error_of(&other)["class"], "busy");
    // ... and the idle clock does not close it.
    tokio::time::sleep(Duration::from_millis(2200).saturating_sub(held_at.elapsed())).await;
    assert_eq!(common::readiness(&harness.client, "a").await, "ready");
    seams.release_admitted_calls("a");
    let _ = admitted.await.unwrap();
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_default_queue_refuses_the_33rd_waiter() {
    let fixture = Fixture::new();
    let harness = Harness::start(store_config(&fixture, json!({"admission_cap": 1}))).await;
    harness.ok("orient", json!({"store": "a"})).await;
    let seams = harness.adapter.seams().clone();
    seams.block_calls("a");
    let running = spawn_call(&harness, "search", search_a());
    wait_for("the running call to be held", Duration::from_secs(5), || seams.held_calls("a") == 1)
        .await;
    let waiters: Vec<_> = (0..32).map(|_| spawn_call(&harness, "search", search_a())).collect();
    tokio::time::sleep(Duration::from_millis(300)).await;
    let started = Instant::now();
    let refused = spawn_call(&harness, "search", search_a()).await.unwrap();
    let elapsed = started.elapsed();
    let error = error_of(&refused);
    assert_eq!(error["busy"]["reason"], "queue_full", "{error}");
    assert_eq!(error["busy"]["queued"], 32, "{error}");
    assert!(elapsed < Duration::from_millis(50), "refused at once: {elapsed:?}");
    assert!(waiters.iter().all(|waiter| !waiter.is_finished()), "the 32 waiters stay queued");
    seams.release_calls("a");
    let _ = running.await.unwrap();
    for waiter in waiters {
        let result = waiter.await.unwrap();
        assert_ne!(result.is_error, Some(true), "{:?}", result.structured_content);
    }
    harness.shutdown().await;
}

fn fetch_a() -> Value {
    json!({"store": "a", "refs": ["l:note:a"]})
}

fn millis(samples: &[u64]) -> Vec<Duration> {
    samples.iter().copied().map(Duration::from_millis).collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_late_permit_is_not_used() {
    let fixture = Fixture::new();
    let harness = Harness::start(store_config(
        &fixture,
        json!({"admission_cap": 1, "call_deadline_ms": 1000}),
    ))
    .await;
    harness.ok("orient", json!({"store": "a"})).await;
    let seams = harness.adapter.seams().clone();
    // Seven samples: still cold, so the wait itself uses the 50 ms floor.
    seams.seed_service_times("a", "fetch", &millis(&[500; 7]));
    seams.block_calls("a");
    let running = spawn_call(&harness, "fetch", fetch_a());
    wait_for("the running call to be held", Duration::from_secs(5), || seams.held_calls("a") == 1)
        .await;
    let waiter_started = Instant::now();
    let waiter = spawn_call(&harness, "fetch", fetch_a());
    // The running call completes as the eighth sample (about 750 ms), when
    // the waiter has about 250 ms left: less than the p90 it now needs.
    tokio::time::sleep_until((waiter_started + Duration::from_millis(750)).into()).await;
    seams.release_calls("a");
    let running = running.await.unwrap();
    assert_ne!(running.is_error, Some(true), "{:?}", running.structured_content);
    let waiter = waiter.await.unwrap();
    let elapsed = waiter_started.elapsed();
    let error = error_of(&waiter);
    assert_eq!(error["busy"]["reason"], "deadline", "{error}");
    assert!(elapsed >= Duration::from_millis(740), "refused before the permit came: {elapsed:?}");
    assert!(elapsed < Duration::from_millis(940), "refused at the step-3 timeout: {elapsed:?}");
    // The refused waiter gave its permit back.
    let started = Instant::now();
    let next = spawn_call(&harness, "fetch", fetch_a()).await.unwrap();
    assert_ne!(next.is_error, Some(true), "{:?}", next.structured_content);
    assert!(started.elapsed() < Duration::from_millis(300), "the permit is free");
    harness.shutdown().await;
}
