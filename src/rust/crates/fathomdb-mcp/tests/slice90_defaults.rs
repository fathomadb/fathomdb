mod common;

use common::{node, Fixture, Harness};
use serde_json::json;

#[tokio::test(flavor = "multi_thread")]
async fn s90_default_store_is_explicit_with_hints_disabled() {
    let fixture = Fixture::new();
    fixture.store("a.sqlite", vec![node("a", "note", "river a")]);
    fixture.store("b.sqlite", vec![node("b", "note", "river b")]);
    let harness = Harness::start(fixture.config(json!({"stores": {
        "a": {"path":"a.sqlite"}, "b": {"path":"b.sqlite"}
    }})))
    .await;
    let listed = harness.ok("list_stores", json!({"hints":false})).await;
    assert!(listed.get("default_store").is_some(), "required nullable default: {listed}");
    assert!(listed["default_store"].is_null());
    harness.ok("use_store", json!({"store":"b"})).await;
    let listed = harness.ok("list_stores", json!({"hints":false})).await;
    assert_eq!(listed["default_store"], "b");
    assert!(listed.get("hints").is_none());
    assert_eq!(listed["stores"][1]["default"], true);
    assert_eq!(harness.ok("search", json!({"query":"river"})).await["store"], "b");
    let other = common::connect(&harness.adapter).await;
    assert!(common::ok(&other, "list_stores", json!({})).await["default_store"].is_null());
    let _ = other.cancel().await;
    harness.shutdown().await;
    for default in [None, Some("a")] {
        let mut config = json!({"stores":{"a":{"path":"a.sqlite"}},"result_format":"text"});
        if let Some(default) = default {
            config["default_store"] = json!(default);
        }
        let harness = Harness::start(fixture.config(config)).await;
        let result = harness.call("list_stores", json!({"hints":false})).await;
        let expected = default.map(|x| json!(x)).unwrap_or_default();
        assert_eq!(
            common::text_of(&result).lines().next().unwrap(),
            format!("stores: 1 default={expected}")
        );
        harness.shutdown().await;
    }
}
