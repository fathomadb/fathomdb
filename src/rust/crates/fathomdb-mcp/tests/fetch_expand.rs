//! Fetch and expand (design §9 order 7): order and per-item errors
//! (MR-052), `l:` only (MR-042.2), malformed versus missing (MR-054.2), the
//! same content as the SDK under the same view (MR-040.2, MR-070.1), as-of on
//! fetch (MR-023.1), budgets, and expansion bounds (MR-053).

mod common;

use common::{edge, node, refs, text_of, windowed_node, Fixture, Harness};
use fathomdb_sdk::{
    read, Engine, InitialState, LifecycleState, OpenOptions, PreparedWrite, ReadView, SourceId,
    TextSearchOptions,
};
use serde_json::{json, Value};

async fn harness_with(fixture: &Fixture, writes: Vec<PreparedWrite>) -> Harness {
    fixture.store("a.sqlite", writes);
    Harness::start(fixture.config(json!({
        "stores": {"a": {"path": "a.sqlite"}},
        "default_store": "a",
        "idle_timeout_secs": 0
    })))
    .await
}

/// As [`harness_with`], with a generous call deadline for the large-graph
/// tests, so a slow runner cannot turn a full scan into timeout/deadline.
async fn harness_with_long_deadline(fixture: &Fixture, writes: Vec<PreparedWrite>) -> Harness {
    fixture.store("a.sqlite", writes);
    Harness::start(fixture.config(json!({
        "stores": {"a": {"path": "a.sqlite"}},
        "default_store": "a",
        "idle_timeout_secs": 0,
        "call_deadline_ms": 60000
    })))
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn fetch_returns_one_item_per_reference_in_request_order() {
    let fixture = Fixture::new();
    let harness = harness_with(
        &fixture,
        vec![node("note:a", "note", "alpha body"), node("note:b", "doc", "beta body")],
    )
    .await;
    let fetched = harness
        .ok(
            "fetch",
            json!({"refs": ["l:note:b", "l:missing", "h:abc", "garbage", "l:", "p:1", "l:note:a"]}),
        )
        .await;
    let items = fetched["items"].as_array().unwrap();
    assert_eq!(items.len(), 7, "MR-052.1");
    assert_eq!(
        items[0],
        json!({"ref": "l:note:b", "record": {"kind": "doc", "body": "beta body", "body_cut": false}})
    );
    assert_eq!(items[1]["ref"], "l:missing");
    assert_eq!(items[1]["error"]["class"], "not_found", "MR-054.2: well-formed but missing");
    assert_eq!(items[2]["error"]["code"], "unsupported_ref", "MR-042.2");
    assert_eq!(items[3]["error"]["class"], "invalid_request", "malformed");
    assert_eq!(items[3]["error"]["code"], "bad_argument");
    assert_eq!(items[4]["error"]["code"], "bad_argument", "an empty logical id is malformed");
    assert_eq!(items[5]["error"]["code"], "unsupported_ref");
    assert_eq!(items[6]["record"]["body"], "alpha body", "MR-052.2: one bad ref fails no other");
    assert_eq!(fetched["epoch"], 1);
    assert!(fetched["validity_instant"].is_i64());
    let text = text_of(&harness.call("fetch", json!({"refs": ["l:note:a"]})).await);
    assert!(text.contains("\"alpha body\""), "{text}");
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn fetch_validates_the_reference_count() {
    let fixture = Fixture::new();
    let harness = harness_with(&fixture, vec![node("note:a", "note", "alpha")]).await;
    let error = harness.err("fetch", json!({"refs": []})).await;
    assert_eq!(error["code"], "bad_argument");
    let many: Vec<String> = (0..51).map(|i| format!("l:n{i}")).collect();
    let error = harness.err("fetch", json!({"refs": many})).await;
    assert_eq!(error["code"], "bad_argument");
    let fifty: Vec<String> = (0..50).map(|i| format!("l:n{i}")).collect();
    let fetched = harness.ok("fetch", json!({"refs": fifty, "max_tokens": 20_000})).await;
    assert_eq!(fetched["items"].as_array().unwrap().len(), 50);
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn fetch_honours_as_of() {
    let fixture = Fixture::new();
    let t1 = 1_700_000_000_i64;
    let t2 = 1_800_000_000_i64;
    let harness =
        harness_with(&fixture, vec![windowed_node("note:w", "windowed", Some(t1), Some(t2))]).await;
    let inside = harness.ok("fetch", json!({"refs": ["l:note:w"], "as_of": t1})).await;
    assert_eq!(inside["items"][0]["record"]["body"], "windowed", "MR-023.1");
    assert_eq!(inside["validity_instant"], t1);
    let outside = harness.ok("fetch", json!({"refs": ["l:note:w"], "as_of": t2})).await;
    assert_eq!(outside["items"][0]["error"]["class"], "not_found");
    harness.shutdown().await;
}

fn governed_fixture() -> Vec<PreparedWrite> {
    vec![
        node("note:live", "note", "live river record"),
        node("note:old", "note", "first river version"),
        windowed_node("note:expired", "expired river record", Some(1), Some(2)),
        windowed_node("note:future", "future river record", Some(4_000_000_000), None),
        PreparedWrite::Node {
            kind: "note".into(),
            body: "pending river record".into(),
            source_id: SourceId::new("fixture").unwrap(),
            logical_id: Some("note:pending".into()),
            state: InitialState::Pending,
            reason: None,
            valid_from: None,
            valid_until: None,
        },
        node("note:gone", "note", "retracted river record"),
    ]
}

/// Records as the SDK reads them, as fetch items would render them.
fn sdk_items(engine: &Engine, ids: &[String], view: &ReadView) -> Vec<Value> {
    read::get_many(engine, ids, Some(view))
        .unwrap()
        .into_iter()
        .zip(ids)
        .map(|(record, id)| match record {
            Some(record) => {
                json!({"ref": format!("l:{id}"), "kind": record.kind, "body": record.body})
            }
            None => json!({"ref": format!("l:{id}"), "missing": true}),
        })
        .collect()
}

fn adapter_items(fetched: &Value) -> Vec<Value> {
    fetched["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| match item.get("record") {
            Some(record) => {
                json!({"ref": item["ref"], "kind": record["kind"], "body": record["body"]})
            }
            None => {
                assert_eq!(item["error"]["class"], "not_found", "{item}");
                json!({"ref": item["ref"], "missing": true})
            }
        })
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn fetch_and_search_return_exactly_what_the_sdk_returns_under_the_same_view() {
    let fixture = Fixture::new();
    let path = fixture.store("a.sqlite", governed_fixture());
    let instant = 1_900_000_000_i64;
    let ids: Vec<String> =
        ["note:live", "note:old", "note:expired", "note:future", "note:pending", "note:gone"]
            .iter()
            .map(|id| id.to_string())
            .collect();
    let views = [(false, false), (true, false), (false, true), (true, true)];
    let (expected_fetch, expected_search) = {
        let engine = Engine::open(&path, OpenOptions::default()).unwrap();
        engine.write(&[node("note:old", "note", "second river version")]).unwrap();
        engine.transition("note:gone", LifecycleState::Deleted, Some("retracted")).unwrap();
        let fetches: Vec<Vec<Value>> = views
            .iter()
            .map(|(include_superseded, include_inactive)| {
                let view = ReadView {
                    include_superseded: *include_superseded,
                    include_inactive: *include_inactive,
                    valid_as_of: Some(instant),
                    ..ReadView::default()
                };
                sdk_items(&engine, &ids, &view)
            })
            .collect();
        let view = ReadView { valid_as_of: Some(instant), ..ReadView::default() };
        let mut options = TextSearchOptions::default();
        options.view = view;
        options.limit = 20;
        options.snippet = None;
        let hits = engine
            .search_text_only("river", options)
            .unwrap()
            .results
            .into_iter()
            .map(|hit| hit.id.to_prefixed())
            .collect::<Vec<_>>();
        engine.close().unwrap();
        (fetches, hits)
    };
    // The fixture really contains hidden records: the default view hides some.
    assert!(expected_fetch[0].iter().any(|item| item.get("missing").is_some()));

    let harness = Harness::start(fixture.config(json!({
        "stores": {"a": {"path": "a.sqlite"}},
        "default_store": "a"
    })))
    .await;
    let refs_arg: Vec<String> = ids.iter().map(|id| format!("l:{id}")).collect();
    for ((include_superseded, include_inactive), expected) in views.iter().zip(&expected_fetch) {
        let fetched = harness
            .ok(
                "fetch",
                json!({
                    "refs": refs_arg,
                    "as_of": instant,
                    "include_superseded": include_superseded,
                    "include_inactive": include_inactive,
                    "max_tokens": 20_000
                }),
            )
            .await;
        assert_eq!(&adapter_items(&fetched), expected, "MR-070.1 / MR-040.2");
    }
    let found =
        harness.ok("search", json!({"query": "river", "limit": 20, "as_of": instant})).await;
    assert_eq!(refs(&found), expected_search, "MR-070.1: search hides what the SDK hides");
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn fetch_bodies_are_cut_to_fit_the_budget() {
    let fixture = Fixture::new();
    let long = "a long body sentence. ".repeat(300);
    let harness = harness_with(
        &fixture,
        vec![node("note:1", "note", &long), node("note:2", "note", "short body")],
    )
    .await;
    for max_tokens in [256_u64, 400, 1000, 3000] {
        let result = harness
            .call("fetch", json!({"refs": ["l:note:1", "l:note:2"], "max_tokens": max_tokens}))
            .await;
        let structured = result.structured_content.clone().unwrap();
        assert_ne!(result.is_error, Some(true), "{structured}");
        let whole = serde_json::to_string(&structured).unwrap().len() + text_of(&result).len();
        assert!((whole as u64).div_ceil(4) <= max_tokens, "MR-041.1 at {max_tokens}");
        assert_eq!(structured["items"][0]["record"]["body_cut"], true);
        let body = structured["items"][0]["record"]["body"].as_str().unwrap();
        assert!(long.starts_with(body), "a cut body is a prefix of the record body");
        assert_eq!(structured["budget"]["cut"], true);
    }
    let full = harness.ok("fetch", json!({"refs": ["l:note:1"], "max_tokens": 100_000})).await;
    assert_eq!(full["items"][0]["record"]["body"], long);
    assert_eq!(full["items"][0]["record"]["body_cut"], false);
    harness.shutdown().await;
}

fn star_graph() -> Vec<PreparedWrite> {
    let mut writes = vec![node("hub", "topic", "hub body")];
    for i in 0..30 {
        writes.push(node(&format!("leaf:{i:02}"), "note", &format!("leaf body {i}")));
    }
    for i in 0..30 {
        writes.push(edge("link", "hub", &format!("leaf:{i:02}")));
    }
    writes.push(node("chain:b", "note", "chain b"));
    writes.push(node("chain:c", "doc", "chain c"));
    writes.push(edge("next", "leaf:00", "chain:b"));
    writes.push(edge("next", "chain:b", "chain:c"));
    writes
}

#[tokio::test(flavor = "multi_thread")]
async fn expand_follows_edges_within_its_bounds() {
    let fixture = Fixture::new();
    let harness = harness_with(&fixture, star_graph()).await;
    let default = harness.ok("expand", json!({"refs": ["l:hub"]})).await;
    let targets = default["targets"].as_array().unwrap();
    assert_eq!(targets.len(), 20, "default limit 20");
    assert_eq!(default["bound"], "result_limit", "MR-053.2: names the cap that was reached");
    assert_eq!(default["complete"], true);
    for target in targets {
        assert_eq!(target["hop_count"], 1);
        assert_eq!(target["terminal_edge_kind"], "link");
        assert!(target["ref"].as_str().unwrap().starts_with("l:leaf:"));
    }
    let five = harness.ok("expand", json!({"refs": ["l:hub"], "limit": 5})).await;
    assert_eq!(five["targets"].as_array().unwrap().len(), 5, "MR-053.1");
    let none = harness.ok("expand", json!({"refs": ["l:hub"], "max_depth": 0})).await;
    assert_eq!(none["targets"], json!([]));
    assert_eq!(none["bound"], Value::Null);

    let chain = harness.ok("expand", json!({"refs": ["l:leaf:00"], "max_depth": 2})).await;
    let hops: Vec<(String, u64)> = chain["targets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| (t["ref"].as_str().unwrap().to_string(), t["hop_count"].as_u64().unwrap()))
        .collect();
    assert_eq!(hops, vec![("l:chain:b".to_string(), 1), ("l:chain:c".to_string(), 2)]);
    assert_eq!(chain["bound"], Value::Null);
    let only_docs = harness
        .ok("expand", json!({"refs": ["l:leaf:00"], "max_depth": 2, "target_kinds": ["doc"]}))
        .await;
    assert_eq!(only_docs["targets"].as_array().unwrap().len(), 1);
    let incoming = harness.ok("expand", json!({"refs": ["l:leaf:03"], "direction": "in"})).await;
    assert_eq!(incoming["targets"][0]["ref"], "l:hub");
    let filtered =
        harness.ok("expand", json!({"refs": ["l:leaf:00"], "edge_kinds": ["link"]})).await;
    assert_eq!(filtered["targets"], json!([]));
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn expand_rejects_out_of_bound_arguments_and_unsupported_references() {
    let fixture = Fixture::new();
    let harness = harness_with(&fixture, star_graph()).await;
    for args in [
        json!({"refs": []}),
        json!({"refs": (0..11).map(|i| format!("l:leaf:{i:02}")).collect::<Vec<_>>()}),
        json!({"refs": ["l:hub"], "max_depth": 4}),
        json!({"refs": ["l:hub"], "limit": 0}),
        json!({"refs": ["l:hub"], "limit": 51}),
        json!({"refs": ["l:hub"], "direction": "sideways"}),
        json!({"refs": ["hub"]}),
    ] {
        let error = harness.err("expand", args.clone()).await;
        assert_eq!(error["class"], "invalid_request", "{args}: {error}");
    }
    let error = harness.err("expand", json!({"refs": ["h:abc"]})).await;
    assert_eq!(error["code"], "unsupported_ref", "MR-042.2");
    let error = harness.err("expand", json!({"refs": ["l:nobody"]})).await;
    assert_eq!(error["class"], "not_found", "MR-054.2");
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn expand_bodies_fit_the_budget() {
    let fixture = Fixture::new();
    let mut writes = vec![node("hub", "topic", "hub")];
    for i in 0..10 {
        writes.push(node(&format!("leaf:{i}"), "note", &"long leaf body ".repeat(100)));
        writes.push(edge("link", "hub", &format!("leaf:{i}")));
    }
    let harness = harness_with(&fixture, writes).await;
    for max_tokens in [256_u64, 600, 2000] {
        let result =
            harness.call("expand", json!({"refs": ["l:hub"], "max_tokens": max_tokens})).await;
        let structured = result.structured_content.clone().unwrap();
        assert_ne!(result.is_error, Some(true), "{structured}");
        let whole = serde_json::to_string(&structured).unwrap().len() + text_of(&result).len();
        assert!((whole as u64).div_ceil(4) <= max_tokens, "MR-041.1 at {max_tokens}");
        assert_eq!(structured["budget"]["cut"], true);
    }
    harness.shutdown().await;
}

fn hub_with_leaves(leaves: usize) -> Vec<PreparedWrite> {
    let mut writes = vec![node("hub", "topic", "hub body")];
    for i in 0..leaves {
        writes.push(node(&format!("leaf:{i:05}"), "note", "leaf"));
        writes.push(edge("link", "hub", &format!("leaf:{i:05}")));
    }
    writes
}

#[tokio::test(flavor = "multi_thread")]
async fn expand_from_a_seed_with_1500_edges_returns_the_limit() {
    let fixture = Fixture::new();
    let harness = harness_with_long_deadline(&fixture, hub_with_leaves(1_500)).await;
    let expanded = harness.ok("expand", json!({"refs": ["l:hub"], "limit": 5})).await;
    assert_eq!(expanded["targets"].as_array().unwrap().len(), 5, "{expanded}");
    assert_eq!(expanded["bound"], "result_limit");
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn expand_past_the_engine_work_bound_is_bound_exceeded() {
    let fixture = Fixture::new();
    let harness = harness_with_long_deadline(&fixture, hub_with_leaves(10_001)).await;
    let error = harness.err("expand", json!({"refs": ["l:hub"], "limit": 5})).await;
    assert_eq!(error["class"], "invalid_request", "{error}");
    assert_eq!(error["code"], "bound_exceeded");
    let remedy = error["remedy"].as_str().unwrap();
    assert!(!remedy.contains("edge_kinds"), "kinds are filtered after counting: {remedy}");
    assert!(error["message"].as_str().unwrap().contains("10000"), "{error}");
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn expand_names_the_budget_when_it_drops_targets() {
    let fixture = Fixture::new();
    let mut writes = vec![node("hub", "topic", "hub")];
    for i in 0..10 {
        writes.push(node(&format!("leaf:{i}"), "note", &"long leaf body ".repeat(100)));
        writes.push(edge("link", "hub", &format!("leaf:{i}")));
    }
    let harness = harness_with(&fixture, writes).await;
    let expanded = harness.ok("expand", json!({"refs": ["l:hub"], "max_tokens": 256})).await;
    let kept = expanded["targets"].as_array().unwrap().len();
    assert!(kept < 10, "the budget must drop targets for this test: kept {kept}");
    assert_eq!(expanded["bound"], "budget", "CR-7: {expanded}");
    assert_eq!(expanded["budget"]["cut"], true);
    let roomy = harness.ok("expand", json!({"refs": ["l:hub"], "max_tokens": 100_000})).await;
    assert_eq!(roomy["targets"].as_array().unwrap().len(), 10);
    assert_eq!(roomy["bound"], serde_json::Value::Null);
    harness.shutdown().await;
}
