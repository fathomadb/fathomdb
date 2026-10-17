//! Store lifecycle (design §9 order 4): lazy open, readiness before open,
//! open failures and retries, lock conflicts, idle close, one open store at a
//! time, reopen re-applying settings, close after a failed drain (F-44) and a
//! stalled call that must not delay another store (F-42). MR-003, MR-004.2,
//! MR-005.4, MR-005.7, MR-006.1/.2/.4/.6.

mod common;

use std::time::{Duration, Instant};

use common::{
    body_edge, can_open, connect, node, readiness, wait_for_readiness, Fixture, Harness, RawClient,
};
use fathomdb_sdk::{Engine, OpenOptions};
use serde_json::{json, Value};

fn two_stores(fixture: &Fixture, extra: Value) -> fathomdb_mcp::config::Config {
    fixture.store("a.sqlite", vec![node("note:a", "note", "alpha river crossing")]);
    fixture.store("b.sqlite", vec![node("note:b", "note", "beta river crossing")]);
    let mut config = json!({"stores": {"a": {"path": "a.sqlite"}, "b": {"path": "b.sqlite"}}});
    for (key, value) in extra.as_object().unwrap() {
        config[key] = value.clone();
    }
    fixture.config(config)
}

#[tokio::test(flavor = "multi_thread")]
async fn stores_open_lazily_on_first_use() {
    let fixture = Fixture::new();
    let harness = Harness::start(two_stores(&fixture, json!({}))).await;
    assert_eq!(readiness(&harness.client, "a").await, "closed");
    assert!(can_open(&fixture.path("a.sqlite")), "nothing opened before first use");
    let found = harness.ok("search", json!({"store": "a", "query": "alpha"})).await;
    assert_eq!(found["epoch"], 1);
    assert_eq!(readiness(&harness.client, "a").await, "ready");
    assert!(!can_open(&fixture.path("a.sqlite")), "the adapter holds the lock while open");
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn handshake_and_tools_list_do_not_wait_for_a_blocked_open() {
    let fixture = Fixture::new();
    let harness = Harness::start(two_stores(&fixture, json!({}))).await;
    harness.adapter.seams().block_open("a");
    let mut raw = RawClient::connect(&harness.adapter).await;
    let pending = raw.call("search", json!({"store": "a", "query": "alpha"})).await;
    assert_eq!(pending["isError"], true);

    let started = Instant::now();
    let second = connect(&harness.adapter).await;
    let tools = second.list_all_tools().await.unwrap();
    assert_eq!(tools.len(), 7);
    assert!(started.elapsed() < Duration::from_secs(1), "MR-003.1: {:?}", started.elapsed());
    let _ = second.cancel().await;
    harness.adapter.seams().release_all();
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_call_without_a_progress_token_gets_not_ready_with_the_phase() {
    let fixture = Fixture::new();
    let harness = Harness::start(two_stores(&fixture, json!({}))).await;
    harness.adapter.seams().block_open("a");
    let mut raw = RawClient::connect(&harness.adapter).await;
    let started = Instant::now();
    let result = raw.call("search", json!({"store": "a", "query": "alpha"})).await;
    assert!(started.elapsed() < Duration::from_secs(1), "no waiting without a progress token");
    assert_eq!(result["isError"], true);
    let error = &result["structuredContent"]["error"];
    assert_eq!(error["class"], "not_ready", "{result}");
    assert_eq!(error["code"], "opening");
    assert!(result["structuredContent"].get("results").is_none(), "not an empty result");
    // Store-independent tools still answer.
    let listed = raw.call("list_stores", json!({})).await;
    assert_ne!(listed["isError"], true);
    assert_eq!(listed["structuredContent"]["stores"][0]["readiness"], "opening");
    // With one open store at a time, another store cannot open while `a` is
    // mid-open: it is refused as busy, never answered from `a` (MR-005.7).
    let other = raw.call("search", json!({"store": "b", "query": "beta"})).await;
    assert_eq!(other["structuredContent"]["error"]["class"], "busy", "{other}");

    harness.adapter.seams().release_open("a");
    assert!(wait_for_readiness(&harness.client, "a", "ready", Duration::from_secs(10)).await);
    let found = raw.call("search", json!({"store": "a", "query": "alpha"})).await;
    assert_ne!(found["isError"], true, "{found}");
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_call_with_a_progress_token_waits_and_receives_progress_until_the_deadline() {
    let fixture = Fixture::new();
    let harness = Harness::start(two_stores(&fixture, json!({"call_deadline_ms": 1500}))).await;
    harness.adapter.seams().block_open("a");
    let started = Instant::now();
    let error = harness.err("search", json!({"store": "a", "query": "alpha"})).await;
    let waited = started.elapsed();
    assert_eq!(error["class"], "not_ready");
    assert_eq!(error["code"], "opening");
    assert!(waited >= Duration::from_millis(1400), "waited {waited:?}");
    assert!(waited < Duration::from_secs(4), "waited {waited:?}");
    tokio::time::sleep(Duration::from_millis(100)).await;
    let progress = harness.client.service().progress.lock().unwrap().clone();
    assert!(!progress.is_empty(), "progress notifications while waiting (MR-006.4)");
    assert!(progress[0].message.as_deref().unwrap_or_default().contains("opening"));

    // Released within the deadline, the waiting call gets its answer.
    let adapter = harness.adapter.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(300)).await;
        adapter.seams().release_open("a");
    });
    let found = harness.ok("search", json!({"store": "a", "query": "alpha"})).await;
    assert_eq!(found["results"][0]["ref"], "l:note:a");
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn reopen_reports_reopening_with_progress() {
    let fixture = Fixture::new();
    let harness = Harness::start(two_stores(
        &fixture,
        json!({"call_deadline_ms": 1200, "idle_timeout_secs": 1}),
    ))
    .await;
    harness.ok("search", json!({"store": "a", "query": "alpha"})).await;
    assert!(wait_for_readiness(&harness.client, "a", "closed", Duration::from_secs(5)).await);
    harness.adapter.seams().block_open("a");
    harness.client.service().progress.lock().unwrap().clear();
    let error = harness.err("search", json!({"store": "a", "query": "alpha"})).await;
    assert_eq!(error["code"], "reopening", "MR-006.4: {error}");
    tokio::time::sleep(Duration::from_millis(100)).await;
    let progress = harness.client.service().progress.lock().unwrap().clone();
    assert!(progress.iter().any(|p| p
        .message
        .as_deref()
        .unwrap_or_default()
        .contains("reopening")));
    harness.adapter.seams().release_open("a");
    let found = harness.ok("search", json!({"store": "a", "query": "alpha"})).await;
    assert_eq!(found["epoch"], 2);
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_locked_store_reports_the_typed_lock_error_and_other_stores_keep_working() {
    let fixture = Fixture::new();
    let config = two_stores(&fixture, json!({}));
    let holder = Engine::open(fixture.path("a.sqlite"), OpenOptions::default()).unwrap();
    let harness = Harness::start(config).await;

    let error = harness.err("search", json!({"store": "a", "query": "alpha"})).await;
    assert_eq!(error["class"], "unavailable", "{error}");
    assert_eq!(error["code"], "database_locked");
    assert!(error["message"].as_str().unwrap().contains("\"a\""), "names the store: {error}");
    assert!(!error["remedy"].as_str().unwrap().is_empty());
    assert_eq!(readiness(&harness.client, "a").await, "failed");

    // MR-003.3: other stores are unaffected and the process keeps serving.
    let other = harness.ok("search", json!({"store": "b", "query": "beta"})).await;
    assert_eq!(other["results"][0]["ref"], "l:note:b");

    // Without a progress token the recorded error comes back at once while
    // the retry starts.
    let mut raw = RawClient::connect(&harness.adapter).await;
    let again = raw.call("search", json!({"store": "a", "query": "alpha"})).await;
    assert_eq!(again["structuredContent"]["error"]["code"], "database_locked", "{again}");

    // MR-006.6: once the lock is gone the next call retries and succeeds.
    holder.close().unwrap();
    drop(holder);
    assert!(wait_for_readiness(&harness.client, "a", "failed", Duration::from_secs(5)).await);
    let found = harness.ok("search", json!({"store": "a", "query": "alpha"})).await;
    assert_eq!(found["results"][0]["ref"], "l:note:a");
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_failed_reopen_is_reported_and_retried() {
    let fixture = Fixture::new();
    let harness = Harness::start(two_stores(&fixture, json!({"idle_timeout_secs": 1}))).await;
    harness.ok("search", json!({"store": "a", "query": "alpha"})).await;
    assert!(wait_for_readiness(&harness.client, "a", "closed", Duration::from_secs(5)).await);
    let holder = Engine::open(fixture.path("a.sqlite"), OpenOptions::default()).unwrap();
    let error = harness.err("search", json!({"store": "a", "query": "alpha"})).await;
    assert_eq!(error["code"], "database_locked");
    holder.close().unwrap();
    drop(holder);
    let found = harness.ok("search", json!({"store": "a", "query": "alpha"})).await;
    assert_eq!(found["results"][0]["ref"], "l:note:a");
    assert_eq!(found["epoch"], 2);
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn an_idle_store_closes_at_the_timeout_and_releases_its_lock() {
    let fixture = Fixture::new();
    let harness = Harness::start(two_stores(&fixture, json!({"idle_timeout_secs": 1}))).await;
    harness.ok("search", json!({"store": "a", "query": "alpha"})).await;
    tokio::time::sleep(Duration::from_millis(400)).await;
    assert_eq!(readiness(&harness.client, "a").await, "ready", "not before the timeout");
    assert!(wait_for_readiness(&harness.client, "a", "closed", Duration::from_secs(4)).await);
    assert!(can_open(&fixture.path("a.sqlite")), "MR-005.4: the sidecar lock is released");
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn calls_reset_the_idle_clock() {
    let fixture = Fixture::new();
    let harness = Harness::start(two_stores(&fixture, json!({"idle_timeout_secs": 2}))).await;
    for _ in 0..6 {
        harness.ok("orient", json!({"store": "a"})).await;
        tokio::time::sleep(Duration::from_millis(600)).await;
        assert_eq!(readiness(&harness.client, "a").await, "ready");
    }
    assert!(wait_for_readiness(&harness.client, "a", "closed", Duration::from_secs(5)).await);
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn an_idle_timeout_of_zero_never_closes() {
    let fixture = Fixture::new();
    let harness = Harness::start(two_stores(&fixture, json!({"idle_timeout_secs": 0}))).await;
    harness.ok("search", json!({"store": "a", "query": "alpha"})).await;
    tokio::time::sleep(Duration::from_millis(2500)).await;
    assert_eq!(readiness(&harness.client, "a").await, "ready");
    assert!(!can_open(&fixture.path("a.sqlite")));
    harness.shutdown().await;
    assert!(can_open(&fixture.path("a.sqlite")), "shutdown drains and closes");
}

#[tokio::test(flavor = "multi_thread")]
async fn switching_stores_closes_the_first_before_opening_the_second() {
    let fixture = Fixture::new();
    let harness = Harness::start(two_stores(&fixture, json!({"idle_timeout_secs": 0}))).await;
    let first = harness.ok("search", json!({"store": "a", "query": "river"})).await;
    assert_eq!(first["results"][0]["ref"], "l:note:a");
    let second = harness.ok("search", json!({"store": "b", "query": "river"})).await;
    assert_eq!(second["results"][0]["ref"], "l:note:b", "never answered from a");
    assert_eq!(second["results"].as_array().unwrap().len(), 1);
    assert_eq!(readiness(&harness.client, "a").await, "closed");
    assert_eq!(readiness(&harness.client, "b").await, "ready");
    assert!(can_open(&fixture.path("a.sqlite")), "a is closed while b is open");
    assert!(!can_open(&fixture.path("b.sqlite")));
    let back = harness.ok("search", json!({"store": "a", "query": "river"})).await;
    assert_eq!(back["epoch"], 2, "switching back reopens a under a new epoch");
    harness.shutdown().await;
}

fn without_volatile(mut value: Value) -> Value {
    if let Value::Object(object) = &mut value {
        for key in ["epoch", "response_id", "continuation", "validity_instant", "budget"] {
            object.remove(key);
        }
    }
    value
}

#[tokio::test(flavor = "multi_thread")]
async fn reopen_reapplies_settings_and_tools_behave_as_before() {
    let fixture = Fixture::new();
    let telemetry = fixture.path("telemetry");
    let harness = Harness::start(two_stores(
        &fixture,
        json!({"idle_timeout_secs": 1, "telemetry_dir": "telemetry"}),
    ))
    .await;
    let pinned = 1_900_000_000_i64;
    let search = json!({"store": "a", "query": "alpha", "mode": "hybrid", "as_of": pinned});
    let fetch = json!({"store": "a", "refs": ["l:note:a"], "as_of": pinned});
    let before_search = harness.ok("search", search.clone()).await;
    let before_fetch = harness.ok("fetch", fetch.clone()).await;
    let before_orient = harness.ok("orient", json!({"store": "a"})).await;
    assert_eq!(before_search["epoch"], 1);
    let pid = std::process::id();
    let first_sink = telemetry.join(format!("a.{pid}.1.jsonl"));
    assert!(first_sink.exists(), "telemetry enabled for epoch 1");

    assert!(wait_for_readiness(&harness.client, "a", "closed", Duration::from_secs(5)).await);
    let after_search = harness.ok("search", search).await;
    let after_fetch = harness.ok("fetch", fetch).await;
    let after_orient = harness.ok("orient", json!({"store": "a"})).await;
    assert_eq!(after_search["epoch"], 2);
    let second_sink = telemetry.join(format!("a.{pid}.2.jsonl"));
    assert!(second_sink.exists(), "MR-006.2: telemetry re-applied on reopen");
    let events = std::fs::read_to_string(&second_sink).unwrap();
    assert_eq!(events.lines().filter(|line| line.contains("\"type\":\"event\"")).count(), 1);

    assert_eq!(without_volatile(before_search), without_volatile(after_search));
    assert_eq!(without_volatile(before_fetch), without_volatile(after_fetch));
    assert_eq!(without_volatile(before_orient), without_volatile(after_orient));
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_failed_drain_still_closes_and_releases_the_lock() {
    let fixture = Fixture::new();
    // A body-bearing edge leaves vector work pending; without an embedder
    // `drain` fails with EmbedderRequired.
    fixture.store(
        "a.sqlite",
        vec![
            node("n:1", "note", "alpha"),
            node("n:2", "note", "beta"),
            body_edge("link", "n:1", "n:2", "alpha links to beta"),
        ],
    );
    let config = fixture.config(json!({
        "stores": {"a": {"path": "a.sqlite"}},
        "idle_timeout_secs": 1
    }));
    let harness = Harness::start(config).await;
    harness.ok("orient", json!({"store": "a"})).await;
    assert!(!can_open(&fixture.path("a.sqlite")));
    assert!(wait_for_readiness(&harness.client, "a", "closed", Duration::from_secs(5)).await);
    assert!(can_open(&fixture.path("a.sqlite")), "F-44: close runs after a failed drain");
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_stalled_call_does_not_delay_a_call_to_another_store() {
    let fixture = Fixture::new();
    let harness = Harness::start(two_stores(
        &fixture,
        json!({"idle_timeout_secs": 1, "call_deadline_ms": 1000}),
    ))
    .await;
    harness.ok("orient", json!({"store": "a"})).await;
    harness.adapter.seams().block_calls("a");
    let error = harness.err("search", json!({"store": "a", "query": "alpha"})).await;
    assert_eq!(error["class"], "timeout");
    // Idle expiry fires while the abandoned call still runs; `a` is not idle.
    tokio::time::sleep(Duration::from_millis(1500)).await;
    assert_eq!(readiness(&harness.client, "a").await, "ready", "in-flight calls keep a store open");
    let started = Instant::now();
    let other = harness.call("search", json!({"store": "b", "query": "beta"})).await;
    let elapsed = started.elapsed();
    assert!(
        elapsed < Duration::from_millis(1500),
        "F-42: answered within its deadline: {elapsed:?}"
    );
    let structured = other.structured_content.unwrap();
    assert_eq!(structured["error"]["class"], "busy", "{structured}");

    harness.adapter.seams().release_calls("a");
    assert!(wait_for_readiness(&harness.client, "a", "closed", Duration::from_secs(5)).await);
    let found = harness.ok("search", json!({"store": "b", "query": "beta"})).await;
    assert_eq!(found["results"][0]["ref"], "l:note:b");
    harness.shutdown().await;
}
