//! Engine bridge (design §9 order 5): a panic on the blocking thread is a
//! typed internal error that releases its admission slot (design §6.4).
//! Admission and the run limit (MR-033) are tested in `admission.rs`.

mod common;

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
