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

#[tokio::test(flavor = "multi_thread")]
async fn an_arrival_is_queued_whatever_its_predicted_wait() {
    let fixture = Fixture::new();
    let harness = Harness::start(store_config(
        &fixture,
        json!({"admission_cap": 1, "call_deadline_ms": 10000}),
    ))
    .await;
    harness.ok("orient", json!({"store": "a"})).await;
    let seams = harness.adapter.seams().clone();
    // Recent searches took 9 s: a call behind a running one is predicted to
    // wait past its 10 s deadline.
    seams.seed_service_times("a", "search_text", &millis(&[9000; 8]));
    seams.block_calls("a");
    let running = spawn_call(&harness, "search", search_a());
    wait_for("the running call to be held", Duration::from_secs(5), || seams.held_calls("a") == 1)
        .await;
    let started = Instant::now();
    let arrival = spawn_call(&harness, "search", search_a());
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(!arrival.is_finished(), "an arrival with room in the queue is queued");
    // It waits until it no longer has the 9 s it needs, then is refused.
    let error = error_of(&arrival.await.unwrap());
    let elapsed = started.elapsed();
    assert_eq!(error["busy"]["reason"], "deadline", "{error}");
    assert!(elapsed >= Duration::from_millis(900), "refused early: {elapsed:?}");
    assert!(elapsed < Duration::from_millis(1500), "refused late: {elapsed:?}");
    seams.release_calls("a");
    let _ = running.await.unwrap();
    harness.shutdown().await;
}

fn expand_a() -> Value {
    json!({"store": "a", "refs": ["l:note:a"]})
}

#[tokio::test(flavor = "multi_thread")]
async fn a_cold_tool_uses_the_50_ms_floor() {
    let fixture = Fixture::new();
    let harness = Harness::start(store_config(
        &fixture,
        json!({"admission_cap": 1, "call_deadline_ms": 1000}),
    ))
    .await;
    harness.ok("orient", json!({"store": "a"})).await;
    let seams = harness.adapter.seams().clone();
    // Fewer than 8 completed calls: however slow, the kind is still cold.
    seams.seed_service_times("a", "fetch", &millis(&[500; 7]));
    seams.seed_service_times("a", "expand", &millis(&[500; 7]));

    // 60 ms left when the permit frees: the call starts.
    seams.block_calls("a");
    let running = spawn_call(&harness, "search", search_a());
    wait_for("the running call to be held", Duration::from_secs(5), || seams.held_calls("a") == 1)
        .await;
    let started = Instant::now();
    let waiter = spawn_call(&harness, "fetch", fetch_a());
    tokio::time::sleep_until((started + Duration::from_millis(940)).into()).await;
    seams.release_calls("a");
    let _ = running.await.unwrap();
    let waiter = waiter.await.unwrap();
    assert_ne!(waiter.is_error, Some(true), "60 ms left: starts: {:?}", waiter.structured_content);

    // 40 ms left when the permit frees: the call was refused at 50 ms left.
    seams.block_calls("a");
    let running = spawn_call(&harness, "search", search_a());
    wait_for("the running call to be held", Duration::from_secs(5), || seams.held_calls("a") == 1)
        .await;
    let started = Instant::now();
    let waiter = spawn_call(&harness, "expand", expand_a());
    tokio::time::sleep_until((started + Duration::from_millis(960)).into()).await;
    seams.release_calls("a");
    let _ = running.await.unwrap();
    let error = error_of(&waiter.await.unwrap());
    assert_eq!(error["busy"]["reason"], "deadline", "40 ms left: refused: {error}");
    harness.shutdown().await;
}

/// 128 samples whose nearest-rank p90 is `p90_ms` and whose mean is far
/// below it.
fn window_with_p90(fast_ms: u64, p90_ms: u64) -> Vec<Duration> {
    let mut samples = vec![fast_ms; 115];
    samples.extend([p90_ms; 13]);
    millis(&samples)
}

/// Run one waiter of `tool` behind a held search, freeing the permit
/// `release_after` after the waiter arrived; returns its result and when it
/// returned.
async fn wait_behind_a_search(
    harness: &Harness,
    tool: &str,
    args: Value,
    release_after: Duration,
) -> (CallToolResult, Duration) {
    let seams = harness.adapter.seams().clone();
    seams.block_calls("a");
    let running = spawn_call(harness, "search", search_a());
    wait_for("the running call to be held", Duration::from_secs(5), || seams.held_calls("a") == 1)
        .await;
    let started = Instant::now();
    let waiter = spawn_call(harness, tool, args);
    let returned = tokio::spawn(async move {
        let result = waiter.await.unwrap();
        (result, started.elapsed())
    });
    tokio::time::sleep_until((started + release_after).into()).await;
    seams.release_calls("a");
    let _ = running.await.unwrap();
    returned.await.unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_waiter_starts_only_if_p90_of_its_tool_fits() {
    let fixture = Fixture::new();
    let harness = Harness::start(store_config(
        &fixture,
        json!({"admission_cap": 1, "call_deadline_ms": 1000}),
    ))
    .await;
    harness.ok("orient", json!({"store": "a"})).await;
    harness.adapter.seams().seed_service_times("a", "fetch", &window_with_p90(10, 300));

    let (started, _) =
        wait_behind_a_search(&harness, "fetch", fetch_a(), Duration::from_millis(650)).await;
    assert_ne!(
        started.is_error,
        Some(true),
        "350 ms left: starts: {:?}",
        started.structured_content
    );

    let (refused, elapsed) =
        wait_behind_a_search(&harness, "fetch", fetch_a(), Duration::from_millis(750)).await;
    let error = error_of(&refused);
    assert_eq!(error["busy"]["reason"], "deadline", "250 ms left: refused: {error}");
    // Refused when 300 ms were left, not at the 50 ms floor.
    assert!(elapsed >= Duration::from_millis(650), "refused early: {elapsed:?}");
    assert!(elapsed < Duration::from_millis(800), "refused late: {elapsed:?}");
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_start_check_is_per_tool_kind() {
    let fixture = Fixture::new();
    let harness = Harness::start(store_config(
        &fixture,
        json!({"admission_cap": 1, "call_deadline_ms": 1000}),
    ))
    .await;
    harness.ok("orient", json!({"store": "a"})).await;
    harness.adapter.seams().seed_service_times("a", "expand", &millis(&[800; 128]));

    let (refused, _) =
        wait_behind_a_search(&harness, "expand", expand_a(), Duration::from_millis(500)).await;
    let error = error_of(&refused);
    assert_eq!(error["busy"]["reason"], "deadline", "slow expand is shed: {error}");

    let (started, _) =
        wait_behind_a_search(&harness, "fetch", fetch_a(), Duration::from_millis(500)).await;
    assert_ne!(
        started.is_error,
        Some(true),
        "a slow expand window does not shed fetch: {:?}",
        started.structured_content
    );
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_window_keeps_the_last_128_completed_calls() {
    let fixture = Fixture::new();
    let harness = Harness::start(store_config(
        &fixture,
        json!({"admission_cap": 1, "call_deadline_ms": 1000}),
    ))
    .await;
    harness.ok("orient", json!({"store": "a"})).await;
    let seams = harness.adapter.seams().clone();
    // 128 fast calls, then 128 slow ones. The p90 of the slow 128 is
    // 600 ms; the p90 of all 256 would be 200 ms.
    seams.seed_service_times("a", "fetch", &millis(&[10; 128]));
    let mut slow = vec![200; 110];
    slow.extend([600; 18]);
    seams.seed_service_times("a", "fetch", &millis(&slow));

    let (refused, elapsed) =
        wait_behind_a_search(&harness, "fetch", fetch_a(), Duration::from_millis(600)).await;
    let error = error_of(&refused);
    assert_eq!(error["busy"]["reason"], "deadline", "400 ms left < 600 ms: {error}");
    assert!(elapsed >= Duration::from_millis(350), "refused early: {elapsed:?}");
    assert!(elapsed < Duration::from_millis(550), "refused late: {elapsed:?}");
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn abandoned_calls_are_not_service_time_samples() {
    let fixture = Fixture::new();
    let harness = Harness::start(store_config(
        &fixture,
        json!({"admission_cap": 1, "call_deadline_ms": 500, "run_limit_ms": 500}),
    ))
    .await;
    harness.ok("orient", json!({"store": "a"})).await;
    let seams = harness.adapter.seams().clone();
    seams.seed_service_times("a", "fetch", &millis(&[10; 8]));
    seams.block_calls("a");
    let error = harness.err("fetch", fetch_a()).await;
    assert_eq!(error["class"], "timeout", "the call is abandoned at its deadline: {error}");
    tokio::time::sleep(Duration::from_millis(1000)).await;
    // The abandoned call returns after about 1.5 s. Had it been a sample,
    // the p90 of nine fetch samples would be 1.5 s, more than any fetch has.
    seams.release_calls("a");
    let next = harness.call("fetch", fetch_a()).await;
    assert_ne!(next.is_error, Some(true), "{:?}", next.structured_content);
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn another_store_in_use_is_busy_store_switch() {
    let fixture = Fixture::new();
    fixture.store("b.sqlite", vec![node("note:c", "note", "gamma river crossing")]);
    let harness = Harness::start(store_config(
        &fixture,
        json!({"stores": {"a": {"path": "a.sqlite"}, "b": {"path": "b.sqlite"}}}),
    ))
    .await;
    harness.ok("orient", json!({"store": "a"})).await;
    let seams = harness.adapter.seams().clone();
    seams.block_calls("a");
    let running = spawn_call(&harness, "search", search_a());
    wait_for("the running call to be held", Duration::from_secs(5), || seams.held_calls("a") == 1)
        .await;
    let started = Instant::now();
    let error = harness.err("search", json!({"store": "b", "query": "gamma"})).await;
    assert_eq!(error["class"], "busy", "{error}");
    assert_eq!(error["code"], "store_switch", "{error}");
    assert!(started.elapsed() < Duration::from_millis(300), "refused at once");
    seams.release_calls("a");
    let _ = running.await.unwrap();
    harness.shutdown().await;
}

fn has_key(value: &Value, key: &str) -> bool {
    match value {
        Value::Object(map) => map.contains_key(key) || map.values().any(|v| has_key(v, key)),
        Value::Array(items) => items.iter().any(|v| has_key(v, key)),
        _ => false,
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn no_busy_result_carries_retry_after_ms() {
    let fixture = Fixture::new();
    fixture.store("b.sqlite", vec![node("note:c", "note", "gamma river crossing")]);
    let harness = Harness::start(store_config(
        &fixture,
        json!({
            "stores": {"a": {"path": "a.sqlite"}, "b": {"path": "b.sqlite"}},
            "admission_cap": 1,
            "queue_limit": 1,
            "call_deadline_ms": 600
        }),
    ))
    .await;
    harness.ok("orient", json!({"store": "a"})).await;
    let seams = harness.adapter.seams().clone();
    seams.block_calls("a");
    let running = spawn_call(&harness, "search", search_a());
    wait_for("the running call to be held", Duration::from_secs(5), || seams.held_calls("a") == 1)
        .await;
    let waiting = spawn_call(&harness, "search", search_a());
    tokio::time::sleep(Duration::from_millis(100)).await;
    // Each result was validated against the output schema by the client.
    let queue_full = error_of(&spawn_call(&harness, "search", search_a()).await.unwrap());
    let store_switch = harness.err("search", json!({"store": "b", "query": "gamma"})).await;
    let deadline = error_of(&waiting.await.unwrap());
    assert_eq!(queue_full["busy"]["reason"], "queue_full");
    assert_eq!(store_switch["code"], "store_switch");
    assert_eq!(deadline["busy"]["reason"], "deadline");
    let mut overloaded = deadline.clone();
    overloaded["busy"]["reason"] = json!("engine_overloaded");

    let tools = serde_json::to_value(harness.tools().await).unwrap();
    let search = tools.as_array().unwrap().iter().find(|tool| tool["name"] == "search").unwrap();
    let schema = jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .compile(&search["outputSchema"])
        .unwrap();
    for error in [&queue_full, &store_switch, &deadline, &overloaded] {
        assert!(!has_key(error, "retry_after_ms"), "{error}");
        assert!(schema.is_valid(&json!({"error": error})), "{error}");
        if error.get("busy").is_some() {
            let mut advised = error.clone();
            advised["busy"]["retry_after_ms"] = json!(250);
            assert!(!schema.is_valid(&json!({"error": advised})), "schema admits {advised}");
        }
    }
    seams.release_calls("a");
    let _ = running.await.unwrap();
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn queued_calls_get_progress_notifications() {
    let fixture = Fixture::new();
    let harness = Harness::start(store_config(&fixture, json!({"admission_cap": 1}))).await;
    harness.ok("orient", json!({"store": "a"})).await;
    let seams = harness.adapter.seams().clone();
    seams.block_calls("a");
    let running = spawn_call(&harness, "search", search_a());
    wait_for("the running call to be held", Duration::from_secs(5), || seams.held_calls("a") == 1)
        .await;
    // The rmcp test client sends a progress token with every request.
    let waiting = spawn_call(&harness, "search", search_a());
    tokio::time::sleep(Duration::from_millis(2300)).await;
    seams.release_calls("a");
    let _ = running.await.unwrap();
    let waited = waiting.await.unwrap();
    assert_ne!(waited.is_error, Some(true), "{:?}", waited.structured_content);
    let queued: Vec<(f64, String)> = harness
        .client
        .service()
        .progress
        .lock()
        .unwrap()
        .iter()
        .filter_map(|note| {
            let message = note.message.clone()?;
            message.starts_with("queued:").then_some((note.progress, message))
        })
        .collect();
    assert!(queued.len() >= 2, "one notification each second while queued: {queued:?}");
    for (_, message) in &queued {
        assert_eq!(message, "queued: 1 waiting, 1 running");
    }
    assert!(queued.windows(2).all(|pair| pair[0].0 < pair[1].0), "progress increases: {queued:?}");
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn concurrent_arrivals_never_overfill_the_queue() {
    let fixture = Fixture::new();
    let harness = Harness::start(store_config(
        &fixture,
        json!({"admission_cap": 1, "queue_limit": 1, "call_deadline_ms": 3000}),
    ))
    .await;
    harness.ok("orient", json!({"store": "a"})).await;
    let seams = harness.adapter.seams().clone();
    seams.block_calls("a");
    let running = spawn_call(&harness, "search", search_a());
    wait_for("the running call to be held", Duration::from_secs(5), || seams.held_calls("a") == 1)
        .await;
    // Two arrivals reach the arrival check together while the line is empty.
    seams.hold_arrivals("a");
    let first = spawn_call(&harness, "search", search_a());
    let second = spawn_call(&harness, "search", search_a());
    tokio::time::sleep(Duration::from_millis(200)).await;
    seams.release_arrivals("a");
    tokio::time::sleep(Duration::from_millis(100)).await;
    let full = error_of(&spawn_call(&harness, "search", search_a()).await.unwrap());
    assert_eq!(full["busy"]["reason"], "queue_full", "{full}");
    assert_eq!(full["busy"]["queued"], 1, "queue_limit 1 admits one waiter: {full}");
    seams.release_calls("a");
    let _ = running.await.unwrap();
    let outcomes = [first.await.unwrap(), second.await.unwrap()];
    let refused = outcomes.iter().filter(|result| result.is_error == Some(true)).count();
    assert_eq!(refused, 1, "one of the two arrivals is refused queue_full");
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn panicked_calls_are_not_service_time_samples() {
    let fixture = Fixture::new();
    let harness = Harness::start(store_config(
        &fixture,
        json!({"admission_cap": 1, "call_deadline_ms": 3000}),
    ))
    .await;
    harness.ok("orient", json!({"store": "a"})).await;
    let seams = harness.adapter.seams().clone();
    // Seven fast samples: one more makes the kind warm.
    seams.seed_service_times("a", "fetch", &millis(&[10; 7]));
    seams.block_calls("a");
    seams.panic_calls("a");
    let panicking = spawn_call(&harness, "fetch", fetch_a());
    wait_for("the panicking call to be held", Duration::from_secs(5), || {
        seams.held_calls("a") == 1
    })
    .await;
    let started = Instant::now();
    let waiter = spawn_call(&harness, "fetch", fetch_a());
    // The held call panics after about 2.5 s, when the waiter has about
    // 500 ms left. Had it been a sample, the p90 of eight fetch samples
    // would be 2.5 s and the waiter would be refused.
    tokio::time::sleep_until((started + Duration::from_millis(2500)).into()).await;
    assert!(seams.release_one_call("a"));
    let panicked = error_of(&panicking.await.unwrap());
    assert_eq!(panicked["code"], "panic", "{panicked}");
    // The waiter, once admitted, is held until its own call may not panic.
    seams.stop_panicking_calls("a");
    seams.release_calls("a");
    let waiter = waiter.await.unwrap();
    assert_ne!(waiter.is_error, Some(true), "{:?}", waiter.structured_content);
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_started_call_past_the_wait_deadline_returns_its_result() {
    let fixture = Fixture::new();
    let harness = Harness::start(store_config(
        &fixture,
        json!({"admission_cap": 1, "call_deadline_ms": 1000, "run_limit_ms": 5000}),
    ))
    .await;
    harness.ok("orient", json!({"store": "a"})).await;
    let seams = harness.adapter.seams().clone();
    seams.seed_service_times("a", "fetch", &millis(&[10; 7]));
    seams.block_calls("a");
    let started = Instant::now();
    let slow = spawn_call(&harness, "fetch", fetch_a());
    tokio::time::sleep(Duration::from_millis(2000)).await;
    seams.release_calls("a");
    let slow = slow.await.unwrap();
    assert_ne!(
        slow.is_error,
        Some(true),
        "the started call answers: {:?}",
        slow.structured_content
    );
    assert!(started.elapsed() >= Duration::from_millis(2000));
    // Its 2 s is the eighth fetch sample, so the p90 is now 2 s: a fetch
    // that has to wait cannot start within its 1 s and is refused at once.
    let (refused, elapsed) =
        wait_behind_a_search(&harness, "fetch", fetch_a(), Duration::from_millis(600)).await;
    let error = error_of(&refused);
    assert_eq!(error["busy"]["reason"], "deadline", "{error}");
    assert!(elapsed < Duration::from_millis(300), "the 2 s sample counts: {elapsed:?}");
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_running_call_is_abandoned_at_the_run_limit() {
    let fixture = Fixture::new();
    let harness = Harness::start(store_config(
        &fixture,
        json!({"admission_cap": 1, "call_deadline_ms": 1000, "run_limit_ms": 1000}),
    ))
    .await;
    harness.ok("orient", json!({"store": "a"})).await;
    let seams = harness.adapter.seams().clone();

    // Admitted at once and held for 3 s: timeout/deadline at the run limit,
    // and the adapter keeps serving.
    seams.block_calls("a");
    let started = Instant::now();
    let error = harness.err("search", search_a()).await;
    let elapsed = started.elapsed();
    assert_eq!(error["class"], "timeout", "{error}");
    assert_eq!(error["code"], "deadline", "{error}");
    assert!(elapsed >= Duration::from_millis(900), "returned early: {elapsed:?}");
    assert!(elapsed <= Duration::from_millis(1100), "returned late: {elapsed:?}");
    let listed = harness.ok("list_stores", json!({})).await;
    assert_eq!(listed["stores"][0]["readiness"], "ready");
    tokio::time::sleep(Duration::from_millis(2000)).await;
    seams.release_calls("a");
    let found = harness.ok("search", search_a()).await;
    assert_eq!(found["results"][0]["ref"], "l:note:a");

    // The run limit is measured from the permit: a call that waited 400 ms
    // for its permit times out 1 s after it got it, not 1 s after arrival.
    seams.block_calls("a");
    let running = spawn_call(&harness, "search", search_a());
    wait_for("the running call to be held", Duration::from_secs(5), || seams.held_calls("a") == 1)
        .await;
    let started = Instant::now();
    let waiter = spawn_call(&harness, "search", search_a());
    tokio::time::sleep_until((started + Duration::from_millis(400)).into()).await;
    assert!(seams.release_one_call("a"));
    let running = running.await.unwrap();
    assert_ne!(running.is_error, Some(true), "{:?}", running.structured_content);
    let error = error_of(&waiter.await.unwrap());
    let elapsed = started.elapsed();
    assert_eq!(error["class"], "timeout", "{error}");
    assert!(elapsed >= Duration::from_millis(1300), "measured from arrival: {elapsed:?}");
    assert!(elapsed <= Duration::from_millis(1550), "returned late: {elapsed:?}");
    seams.release_calls("a");
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_kind_slower_than_the_deadline_still_starts_on_a_free_permit() {
    let fixture = Fixture::new();
    let harness = Harness::start(store_config(
        &fixture,
        json!({"admission_cap": 1, "call_deadline_ms": 1000, "run_limit_ms": 5000}),
    ))
    .await;
    harness.ok("orient", json!({"store": "a"})).await;
    // Recent fetches took 2 s, more than the 1 s deadline. A fetch that
    // finds a permit free has not waited, so the start check does not
    // apply: it starts, and its own time becomes a new sample.
    harness.adapter.seams().seed_service_times("a", "fetch", &millis(&[2000; 8]));
    let started = harness.call("fetch", fetch_a()).await;
    assert_ne!(started.is_error, Some(true), "{:?}", started.structured_content);
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn an_abandoned_call_keeps_its_permit_and_is_counted() {
    let fixture = Fixture::new();
    let harness = Harness::start(store_config(
        &fixture,
        json!({"admission_cap": 1, "call_deadline_ms": 500, "run_limit_ms": 500}),
    ))
    .await;
    harness.ok("orient", json!({"store": "a"})).await;
    let seams = harness.adapter.seams().clone();
    seams.block_calls("a");
    let error = harness.err("search", search_a()).await;
    assert_eq!(error["class"], "timeout", "abandoned at the run limit: {error}");
    let probe = harness.err("search", search_a()).await;
    assert_eq!(probe["busy"]["abandoned"], 1, "{probe}");
    assert_eq!(probe["busy"]["in_flight"], 1, "the abandoned call keeps its permit: {probe}");

    seams.release_calls("a");
    let found = harness.ok("search", search_a()).await;
    assert_eq!(found["results"][0]["ref"], "l:note:a", "the permit is free again");
    seams.block_calls("a");
    let running = spawn_call(&harness, "search", search_a());
    wait_for("the running call to be held", Duration::from_secs(5), || seams.held_calls("a") == 1)
        .await;
    let probe = harness.err("search", search_a()).await;
    assert_eq!(probe["busy"]["abandoned"], 0, "{probe}");
    assert_eq!(probe["busy"]["in_flight"], 1, "{probe}");
    seams.release_calls("a");
    let _ = running.await.unwrap();
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn stuck_is_reported_when_every_permit_is_abandoned() {
    let fixture = Fixture::new();
    let harness = Harness::start(store_config(
        &fixture,
        json!({"admission_cap": 2, "call_deadline_ms": 500, "run_limit_ms": 500}),
    ))
    .await;
    harness.ok("orient", json!({"store": "a"})).await;
    let seams = harness.adapter.seams().clone();
    seams.block_calls("a");
    let error = harness.err("search", search_a()).await;
    assert_eq!(error["class"], "timeout", "{error}");
    // One of two permits abandoned: not stuck.
    let running = spawn_call(&harness, "search", search_a());
    wait_for("the second call to be held", Duration::from_secs(5), || seams.held_calls("a") == 2)
        .await;
    let probe = harness.err("search", search_a()).await;
    assert_eq!(probe["busy"]["abandoned"], 1, "{probe}");
    assert_eq!(probe["busy"]["stuck"], false, "{probe}");
    // Both abandoned: stuck.
    let error = error_of(&running.await.unwrap());
    assert_eq!(error["class"], "timeout", "{error}");
    let probe = harness.err("search", search_a()).await;
    assert_eq!(probe["busy"]["abandoned"], 2, "{probe}");
    assert_eq!(probe["busy"]["stuck"], true, "{probe}");
    seams.release_calls("a");
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_client_cancel_of_a_running_call_abandons_it() {
    let fixture = Fixture::new();
    let harness = Harness::start(store_config(
        &fixture,
        json!({"admission_cap": 1, "call_deadline_ms": 600, "run_limit_ms": 10000}),
    ))
    .await;
    harness.ok("orient", json!({"store": "a"})).await;
    let seams = harness.adapter.seams().clone();
    seams.block_calls("a");
    let mut raw = common::RawClient::connect(&harness.adapter).await;
    let running =
        raw.send_request("tools/call", json!({"name": "search", "arguments": search_a()})).await;
    wait_for("the running call to be held", Duration::from_secs(5), || seams.held_calls("a") == 1)
        .await;
    // rmcp drops the cancelled call's response, so its effect is read from
    // another call's busy object: abandoned until its thread returns.
    raw.cancel(running).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    let probe = harness.err("search", search_a()).await;
    assert_eq!(probe["busy"]["abandoned"], 1, "{probe}");
    assert_eq!(probe["busy"]["in_flight"], 1, "the read still holds its permit: {probe}");

    seams.release_calls("a");
    let found = harness.ok("search", search_a()).await;
    assert_eq!(found["results"][0]["ref"], "l:note:a");
    seams.block_calls("a");
    let held = spawn_call(&harness, "search", search_a());
    wait_for("a new call to be held", Duration::from_secs(5), || seams.held_calls("a") == 1).await;
    let probe = harness.err("search", search_a()).await;
    assert_eq!(probe["busy"]["abandoned"], 0, "released when the thread returned: {probe}");
    seams.release_calls("a");
    let _ = held.await.unwrap();
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn shutdown_wakes_queued_calls_with_not_ready_draining() {
    let fixture = Fixture::new();
    let harness = Harness::start(store_config(
        &fixture,
        json!({"admission_cap": 1, "call_deadline_ms": 5000}),
    ))
    .await;
    harness.ok("orient", json!({"store": "a"})).await;
    let seams = harness.adapter.seams().clone();
    seams.block_calls("a");
    let running = spawn_call(&harness, "search", search_a());
    wait_for("the running call to be held", Duration::from_secs(5), || seams.held_calls("a") == 1)
        .await;
    let waiting = spawn_call(&harness, "search", search_a());
    tokio::time::sleep(Duration::from_millis(200)).await;
    let adapter = harness.adapter.clone();
    let started = Instant::now();
    let shutdown = tokio::spawn(async move { adapter.shutdown().await });
    let error = error_of(&waiting.await.unwrap());
    let elapsed = started.elapsed();
    assert_eq!(error["class"], "not_ready", "{error}");
    assert_eq!(error["code"], "draining", "{error}");
    assert!(elapsed < Duration::from_millis(500), "woken at once: {elapsed:?}");
    seams.release_calls("a");
    let _ = running.await.unwrap();
    shutdown.await.unwrap();
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_store_with_queued_calls_does_not_expire_idle() {
    let fixture = Fixture::new();
    let harness = Harness::start(store_config(
        &fixture,
        json!({"admission_cap": 1, "idle_timeout_secs": 1, "call_deadline_ms": 5000}),
    ))
    .await;
    harness.ok("orient", json!({"store": "a"})).await;
    let seams = harness.adapter.seams().clone();
    seams.block_calls("a");
    let running = spawn_call(&harness, "search", search_a());
    wait_for("the running call to be held", Duration::from_secs(5), || seams.held_calls("a") == 1)
        .await;
    let waiting = spawn_call(&harness, "search", search_a());
    let started = Instant::now();
    while started.elapsed() < Duration::from_millis(2000) {
        assert_eq!(common::readiness(&harness.client, "a").await, "ready");
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(!waiting.is_finished(), "the call is still queued");
    seams.release_calls("a");
    let _ = running.await.unwrap();
    let waited = waiting.await.unwrap();
    assert_ne!(waited.is_error, Some(true), "{:?}", waited.structured_content);
    harness.shutdown().await;
}
