//! Addressable stores (design §9 order 4b): per-store answers, listing
//! without paths, no creation without permission, per-store handles, name
//! rejection, roots, and embedded mode with host engines. MR-004.1,
//! MR-005.1/.2/.3/.5/.6.

mod common;

use std::sync::Arc;

use common::{connect, node, Fixture, Harness};
use fathomdb_mcp::config::Config;
use fathomdb_mcp::Adapter;
use fathomdb_sdk::{Engine, OpenOptions};
use serde_json::json;

#[tokio::test(flavor = "multi_thread")]
async fn each_store_returns_only_its_own_records() {
    let fixture = Fixture::new();
    for (file, id) in [("a.sqlite", "note:a"), ("b.sqlite", "note:b"), ("c.sqlite", "note:c")] {
        fixture.store(file, vec![node(id, "note", "shared river crossing words")]);
    }
    let harness = Harness::start(fixture.config(json!({
        "stores": {"a": {"path": "a.sqlite"}, "b": {"path": "b.sqlite"}, "c": {"path": "c.sqlite"}}
    })))
    .await;
    for store in ["a", "b", "c"] {
        let found = harness.ok("search", json!({"store": store, "query": "river"})).await;
        assert_eq!(found["store"], store);
        assert_eq!(common::refs(&found), vec![format!("l:note:{store}")], "{found}");
        for other in ["a", "b", "c"].into_iter().filter(|other| *other != store) {
            let fetched = harness
                .ok("fetch", json!({"store": store, "refs": [format!("l:note:{other}")]}))
                .await;
            assert_eq!(fetched["items"][0]["error"]["class"], "not_found");
        }
    }
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_listing_names_every_store_with_its_state_and_no_paths() {
    let fixture = Fixture::new();
    fixture.store("a.sqlite", vec![node("note:a", "note", "alpha")]);
    std::fs::create_dir(fixture.path("root")).unwrap();
    fixture.store("root/r1.sqlite", vec![]);
    let harness = Harness::start(fixture.config(json!({
        "stores": {"a": {"path": "a.sqlite"}},
        "roots": [{"path": "root"}],
        "default_store": "a"
    })))
    .await;
    harness.ok("orient", json!({"store": "a"})).await;
    let listed = harness.ok("list_stores", json!({})).await;
    assert_eq!(
        listed["stores"],
        json!([
            {"name": "a", "open": true, "readiness": "ready", "default": true},
            {"name": "r1", "open": false, "readiness": "closed", "default": false}
        ])
    );
    let text = serde_json::to_string(&listed).unwrap();
    let root = fixture.dir.path().to_string_lossy().to_string();
    assert!(!text.contains(&root) && !text.contains(".sqlite"), "no paths: {text}");
    let rooted = harness.ok("orient", json!({"store": "r1"})).await;
    assert_eq!(rooted["store"], "r1");
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn files_are_created_only_where_the_registry_allows_it() {
    let fixture = Fixture::new();
    let harness = Harness::start(fixture.config(json!({
        "stores": {
            "fixed": {"path": "fixed.sqlite"},
            "growable": {"path": "growable.sqlite", "create": true}
        }
    })))
    .await;
    let error = harness.err("orient", json!({"store": "fixed"})).await;
    assert_eq!(error["class"], "unavailable");
    assert_eq!(error["code"], "missing_file");
    assert!(!fixture.path("fixed.sqlite").exists(), "MR-005.5");
    harness.ok("orient", json!({"store": "growable"})).await;
    assert!(fixture.path("growable.sqlite").exists());
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn names_outside_the_registry_are_rejected_without_touching_files() {
    let fixture = Fixture::new();
    fixture.store("a.sqlite", vec![node("note:a", "note", "alpha")]);
    std::fs::create_dir(fixture.path("root")).unwrap();
    let harness = Harness::start(fixture.config(json!({
        "stores": {"alpha": {"path": "a.sqlite"}},
        "roots": [{"path": "root"}]
    })))
    .await;
    let outside = fixture.path("a.sqlite").to_string_lossy().to_string();
    for name in ["../a", "..", "/etc/passwd", outside.as_str(), "root/a", "a\\b"] {
        let error = harness.err("search", json!({"store": name, "query": "alpha"})).await;
        assert_eq!(error["class"], "invalid_request", "{name}: {error}");
        assert_eq!(error["code"], "bad_store_name", "{name}: {error}");
        assert!(error.get("candidates").is_none());
    }
    let error = harness.err("search", json!({"store": "alpah", "query": "alpha"})).await;
    assert_eq!(error["code"], "unknown_store");
    assert_eq!(error["candidates"], json!(["alpha"]));
    let error = harness.err("use_store", json!({"store": "../a"})).await;
    assert_eq!(error["code"], "bad_store_name");
    let entries: Vec<_> = std::fs::read_dir(fixture.path("root")).unwrap().collect();
    assert!(entries.is_empty(), "MR-005.3: nothing created");
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn handles_from_one_store_are_rejected_on_another() {
    let fixture = Fixture::new();
    // Distinct bodies: search deduplicates hits on body.
    let many: Vec<_> = (0..5)
        .map(|i| node(&format!("note:{i}"), "note", &format!("river words number {i}")))
        .collect();
    fixture.store("a.sqlite", many.clone());
    fixture.store("b.sqlite", many);
    let harness = Harness::start(fixture.config(json!({
        "stores": {"a": {"path": "a.sqlite"}, "b": {"path": "b.sqlite"}},
        "idle_timeout_secs": 0
    })))
    .await;
    let page = harness.ok("search", json!({"store": "a", "query": "river", "limit": 2})).await;
    let continuation = page["continuation"].as_str().expect("continuation").to_string();
    let response_id = page["response_id"].as_str().unwrap().to_string();
    let error = harness.err("search", json!({"store": "b", "continuation": continuation})).await;
    assert_eq!(error["class"], "invalid_request");
    assert_eq!(error["code"], "stale_handle", "MR-005.6: {error}");
    let error = harness
        .err("feedback", json!({"store": "b", "response_id": response_id, "used": ["l:note:0"]}))
        .await;
    assert_eq!(error["class"], "invalid_request");
    assert_eq!(error["code"], "wrong_store", "MR-024.5: {error}");
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn use_store_sets_the_session_default_per_connection() {
    let fixture = Fixture::new();
    fixture.store("a.sqlite", vec![node("note:a", "note", "alpha river")]);
    fixture.store("b.sqlite", vec![node("note:b", "note", "beta river")]);
    let harness = Harness::start(fixture.config(json!({
        "stores": {"a": {"path": "a.sqlite"}, "b": {"path": "b.sqlite"}}
    })))
    .await;
    let error = harness.err("search", json!({"query": "river"})).await;
    assert_eq!(error["code"], "bad_argument", "no store and no default: {error}");
    assert_eq!(harness.ok("use_store", json!({"store": "b"})).await, json!({"store": "b"}));
    let found = harness.ok("search", json!({"query": "river"})).await;
    assert_eq!(found["store"], "b");
    let other = connect(&harness.adapter).await;
    let error = common::err(&other, "search", json!({"query": "river"})).await;
    assert_eq!(error["code"], "bad_argument", "the default is per session");
    let _ = other.cancel().await;
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn embedded_mode_serves_host_engines_without_a_second_open() {
    let fixture = Fixture::new();
    fixture.store("one.sqlite", vec![node("note:1", "note", "first host river")]);
    fixture.store("two.sqlite", vec![node("note:2", "note", "second host river")]);
    let one = Arc::new(Engine::open(fixture.path("one.sqlite"), OpenOptions::default()).unwrap());
    let two = Arc::new(Engine::open(fixture.path("two.sqlite"), OpenOptions::default()).unwrap());
    let adapter = Adapter::new(Config::default()).unwrap();
    adapter.register_engine("one", one.clone()).unwrap();
    adapter.register_engine("two", two.clone()).unwrap();
    assert!(adapter.register_engine("one", one.clone()).is_err(), "duplicate names refused");
    assert!(adapter.register_engine("../x", one.clone()).is_err(), "bad names refused");
    let client = connect(&adapter).await;
    // A second `Engine::open` of either file in this process would fail with
    // DatabaseLocked, so these answers prove the host engines were used.
    let first = common::ok(&client, "search", json!({"store": "one", "query": "river"})).await;
    assert_eq!(common::refs(&first), vec!["l:note:1"]);
    let second = common::ok(&client, "search", json!({"store": "two", "query": "river"})).await;
    assert_eq!(common::refs(&second), vec!["l:note:2"]);
    assert_eq!(first["epoch"], 1);
    let listed = common::ok(&client, "list_stores", json!({})).await;
    assert_eq!(listed["stores"].as_array().unwrap().len(), 2);
    let _ = client.cancel().await;
    adapter.shutdown().await;
    // The host still owns its engines after the adapter shuts down.
    assert!(one.search_text_only("river", Default::default()).is_ok());
    one.close().unwrap();
    two.close().unwrap();
}
