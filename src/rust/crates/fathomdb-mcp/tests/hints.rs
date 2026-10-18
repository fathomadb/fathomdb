//! Hints sidecar (`dev/design/agent-access/hints-sidecar.md`; ADR-0.8.29):
//! configuration and per-call argument, the reasons each tool emits, their
//! bounds, trust rules and budget accounting, against real stores through
//! the in-process protocol harness.

mod common;

use common::{node, Fixture, Harness};
use fathomdb_sdk::PreparedWrite;
use serde_json::{json, Value};

fn river_notes(count: usize, body_len: usize) -> Vec<PreparedWrite> {
    (0..count)
        .map(|i| {
            let mut body = format!("river crossing entry number {i} ");
            while body.len() < body_len {
                body.push_str("and the water ran on past the old stone bridge ");
            }
            node(&format!("note:{i:03}"), "note", &body)
        })
        .collect()
}

async fn harness_with(writes: Vec<PreparedWrite>, extra: Value) -> (Fixture, Harness) {
    let fixture = Fixture::new();
    fixture.store("a.sqlite", writes);
    let mut config = json!({"stores": {"a": {"path": "a.sqlite"}}, "default_store": "a"});
    for (key, value) in extra.as_object().unwrap() {
        config[key] = value.clone();
    }
    let harness = Harness::start(fixture.config(config)).await;
    (fixture, harness)
}

/// § 5: every tool declares and accepts a per-call `hints` boolean,
/// including a continuation call.
#[tokio::test(flavor = "multi_thread")]
async fn every_tool_accepts_a_hints_argument() {
    let (_fixture, harness) = harness_with(river_notes(12, 60), json!({})).await;
    for tool in harness.tools().await {
        let schema = serde_json::to_value(&tool.input_schema).unwrap();
        assert_eq!(
            schema["properties"]["hints"]["type"],
            json!(["boolean", "null"]),
            "{}: {schema}",
            tool.name
        );
    }
    harness.ok("list_stores", json!({"hints": false})).await;
    harness.ok("use_store", json!({"store": "a", "hints": true})).await;
    harness.ok("orient", json!({"hints": false})).await;
    let page = harness.ok("search", json!({"query": "river", "limit": 5, "hints": false})).await;
    let continuation = page["continuation"].as_str().expect("a full page continues").to_string();
    harness.ok("search", json!({"continuation": continuation, "hints": true})).await;
    harness.ok("fetch", json!({"refs": ["l:note:001"], "hints": false})).await;
    harness.ok("expand", json!({"refs": ["l:note:001"], "hints": false})).await;
    harness
        .ok("feedback", json!({"response_id": page["response_id"], "used": [], "hints": false}))
        .await;
    harness.shutdown().await;
}

fn hints_of(result: &Value) -> Vec<Value> {
    result.get("hints").and_then(Value::as_array).cloned().unwrap_or_default()
}

/// § 8 test 5 and § 3.3: `no_results` carries `mode: "hybrid"` only for a
/// text search on a store whose hybrid route is usable. Here condition 1
/// (the call ran in text mode) and conditions 2-3 (a usable embedder that
/// the configuration permits) are made false in turn through the adapter;
/// this build has no embedder, so the positive case and condition 4
/// (`dense_disabled`) are tested through the pure `search_hints` with
/// `hybrid_available` true and false (fathomdb-engine `hints` unit tests).
#[tokio::test(flavor = "multi_thread")]
async fn empty_text_search_suggests_hybrid_only_when_hybrid_available() {
    let (_fixture, harness) = harness_with(river_notes(3, 40), json!({})).await;
    let no_embedder = harness.ok("search", json!({"query": "zebra", "mode": "text"})).await;
    assert_eq!(no_embedder["completeness"], "empty");
    assert_eq!(
        hints_of(&no_embedder),
        vec![json!({"action": "search", "reason": "no_results"})],
        "conditions 2-3 false: rephrase, no mode"
    );
    let hybrid = harness.ok("search", json!({"query": "zebra", "mode": "hybrid"})).await;
    let no_results: Vec<Value> =
        hints_of(&hybrid).into_iter().filter(|hint| hint["reason"] == "no_results").collect();
    assert_eq!(
        no_results,
        vec![json!({"action": "search", "reason": "no_results"})],
        "condition 1 false: a hybrid search never suggests a mode"
    );
    harness.shutdown().await;
}

/// § 8 test 6 and § 6 B2-2: an empty search with a degradation entry gets
/// `degraded` then `no_results`, and no `index_not_ready` word.
#[tokio::test(flavor = "multi_thread")]
async fn empty_with_degradation_emits_no_results_and_degraded() {
    let (_fixture, harness) = harness_with(river_notes(3, 40), json!({})).await;
    let result = harness.call("search", json!({"query": "zebra", "mode": "hybrid"})).await;
    let found = result.structured_content.clone().unwrap();
    assert_eq!(found["completeness"], "empty_not_ready");
    assert_eq!(
        hints_of(&found),
        vec![
            json!({"action": "none", "reason": "degraded", "arm": "vector", "cause": "embedder_unavailable"}),
            json!({"action": "search", "reason": "no_results"}),
        ]
    );
    let text = common::text_of(&result);
    assert!(!found.to_string().contains("index_not_ready"), "{found}");
    assert!(!text.contains("index_not_ready"), "{text}");
    let hint_lines: Vec<&str> = text.lines().filter(|line| line.starts_with("hint: ")).collect();
    assert_eq!(hint_lines.len(), 2, "one text line per hint: {text}");
    assert!(hint_lines[0].starts_with("hint: none "), "{text}");
    assert!(hint_lines[1].starts_with("hint: search "), "{text}");
    harness.shutdown().await;
}

/// § 8 test 7 and § 6 B2-3: a degraded hybrid search names its first
/// degradation entry, in canonical order, in one `degraded` hint.
#[tokio::test(flavor = "multi_thread")]
async fn degraded_hybrid_emits_degraded_hint_matching_first_degradation_entry() {
    let (_fixture, harness) = harness_with(river_notes(3, 40), json!({})).await;
    let found = harness.ok("search", json!({"query": "river", "mode": "hybrid"})).await;
    assert!(!found["results"].as_array().unwrap().is_empty());
    let first = &found["degradation"][0];
    assert!(first.is_object(), "{found}");
    let degraded: Vec<Value> =
        hints_of(&found).into_iter().filter(|hint| hint["reason"] == "degraded").collect();
    assert_eq!(
        degraded,
        vec![
            json!({"action": "none", "reason": "degraded", "arm": first["arm"], "cause": first["reason"]})
        ]
    );
    harness.shutdown().await;
}

/// § 8 test 12 and § 5: precedence is the call's `hints`, then the config
/// key, then the default (on). All six combinations of call none/true/false
/// and config true/false.
#[tokio::test(flavor = "multi_thread")]
async fn per_call_hints_argument_overrides_config() {
    for config_hints in [true, false] {
        let (_fixture, harness) =
            harness_with(river_notes(3, 40), json!({"hints": config_hints})).await;
        for call_hints in [None, Some(true), Some(false)] {
            let mut args = json!({"query": "zebra"});
            if let Some(call_hints) = call_hints {
                args["hints"] = json!(call_hints);
            }
            let found = harness.ok("search", args).await;
            let expected = call_hints.unwrap_or(config_hints);
            assert_eq!(
                found.get("hints").is_some(),
                expected,
                "config {config_hints}, call {call_hints:?}: {found}"
            );
        }
        harness.shutdown().await;
    }
    let (_fixture, harness) = harness_with(river_notes(3, 40), json!({})).await;
    let found = harness.ok("search", json!({"query": "zebra"})).await;
    assert!(found.get("hints").is_some(), "on by default: {found}");
    harness.shutdown().await;
}
