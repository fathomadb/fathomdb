mod common;

use common::{node, Fixture, Harness};
use serde_json::json;

#[tokio::test(flavor = "multi_thread")]
async fn s90_unsupported_fields_are_absent_and_disclosed() {
    let fixture = Fixture::new();
    fixture.store("a.sqlite", vec![node("n", "note", "river crossing")]);
    let harness = Harness::start(fixture.config(json!({
        "stores": {"a": {"path": "a.sqlite"}}, "default_store": "a"
    })))
    .await;
    let orient = harness.ok("orient", json!({})).await;
    assert_eq!(
        orient["state"]["status"], "available",
        "live EP-01 state must be present: {orient}"
    );
    assert_eq!(orient["not_in_this_release"], json!(["verdict", "kinds", "counts"]));
    assert!(orient.get("kinds").is_some() && orient.get("counts").is_some());
    let search = harness.ok("search", json!({"query": "river"})).await;
    assert_eq!(search["state"]["status"], "available");
    assert!(search.get("verdict").is_none(), "unavailable-only verdict must be absent");
    for (tool, args) in [("fetch", json!({"refs": ["l:n"]})), ("expand", json!({"refs": ["l:n"]}))]
    {
        assert_eq!(harness.ok(tool, args).await["state"]["status"], "available", "{tool}");
    }
    // Unrecorded feedback (telemetry disabled) has no engine read so state is absent.
    let fb =
        harness.ok("feedback", json!({"response_id": search["response_id"], "used": []})).await;
    assert!(fb.get("state").is_none(), "not_recorded feedback has no live engine snapshot");
    harness.shutdown().await;
}
