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

/// § 8 test 1 and § 2.2: one `snippet_cut` hint lists every cut ref in rank
/// order up to 50, and counts the rest in `refs_omitted`.
#[tokio::test(flavor = "multi_thread")]
async fn search_with_cut_snippets_emits_one_fetch_hint_with_every_cut_ref() {
    let (_fixture, harness) =
        harness_with(river_notes(60, 200), json!({"snippet_chars": 40})).await;
    let found =
        harness.ok("search", json!({"query": "river", "limit": 60, "max_tokens": 20_000})).await;
    let results = found["results"].as_array().unwrap();
    assert_eq!(results.len(), 60, "nothing cut by the budget: {}", found["budget"]);
    assert!(results.iter().all(|hit| hit["snippet_cut"] == true));
    let cut: Vec<Value> =
        hints_of(&found).into_iter().filter(|hint| hint["reason"] == "snippet_cut").collect();
    let expected: Vec<Value> = results[..50].iter().map(|hit| hit["ref"].clone()).collect();
    assert_eq!(
        cut,
        vec![
            json!({"action": "fetch", "reason": "snippet_cut", "refs": expected, "refs_omitted": 10})
        ]
    );
    harness.shutdown().await;
}

/// § 8 test 14 and § 2.2: one hint per reason, at most 50 refs, and a ref
/// longer than 128 bytes is left out of the hint (but stays in `results`)
/// and counted in `refs_omitted`.
#[tokio::test(flavor = "multi_thread")]
async fn hint_bounds_hold() {
    let long_id = |i: usize| format!("note:{i:03}:{}", "x".repeat(130));
    let writes: Vec<PreparedWrite> = river_notes(60, 200)
        .into_iter()
        .enumerate()
        .map(|(i, write)| match write {
            PreparedWrite::Node { body, .. } if i % 10 == 3 => node(&long_id(i), "note", &body),
            other => other,
        })
        .collect();
    let (_fixture, harness) = harness_with(writes, json!({"snippet_chars": 40})).await;
    let found =
        harness.ok("search", json!({"query": "river", "limit": 60, "max_tokens": 20_000})).await;
    let results: Vec<String> = common::refs(&found);
    assert_eq!(results.len(), 60, "nothing cut by the budget");
    let long: Vec<&String> = results.iter().filter(|reference| reference.len() > 128).collect();
    assert_eq!(long.len(), 6, "long refs stay in results");
    let hints = hints_of(&found);
    let mut reasons: Vec<&str> =
        hints.iter().map(|hint| hint["reason"].as_str().unwrap()).collect();
    reasons.dedup();
    assert_eq!(reasons.len(), hints.len(), "one hint per reason: {hints:?}");
    let cut = hints.iter().find(|hint| hint["reason"] == "snippet_cut").expect("snippet_cut");
    let listed: Vec<&str> =
        cut["refs"].as_array().unwrap().iter().map(|r| r.as_str().unwrap()).collect();
    let short: Vec<&str> = results.iter().filter(|r| r.len() <= 128).map(String::as_str).collect();
    assert_eq!(listed, short[..50].to_vec(), "the first 50 short cut refs, in order");
    assert_eq!(cut["refs_omitted"], 10, "6 long refs and 4 beyond 50");
    harness.shutdown().await;
}

fn paging_hints(found: &Value) -> Vec<Value> {
    hints_of(found)
        .into_iter()
        .filter(|hint| hint["reason"] == "more_results" || hint["reason"] == "cut_by_budget")
        .collect()
}

/// § 8 test 4 and § 2.2: `more_results` and `cut_by_budget` carry exactly the
/// result's `continuation`, never both, and neither without one.
#[tokio::test(flavor = "multi_thread")]
async fn continuation_hint_equals_result_continuation() {
    let (_fixture, harness) =
        harness_with(river_notes(30, 400), json!({"snippet_chars": 40})).await;
    let full = harness.ok("search", json!({"query": "river", "limit": 5})).await;
    assert_eq!(full["completeness"], "limit_reached");
    assert_eq!(
        paging_hints(&full),
        vec![
            json!({"action": "search", "reason": "more_results", "continuation": full["continuation"]})
        ]
    );
    let cut = harness.ok("search", json!({"query": "river", "limit": 20, "max_tokens": 700})).await;
    assert_eq!(cut["completeness"], "cut_by_budget");
    assert_eq!(
        paging_hints(&cut),
        vec![json!({
            "action": "search",
            "reason": "cut_by_budget",
            "continuation": cut["continuation"],
            "max_tokens": 700
        })]
    );
    let last = harness.ok("search", json!({"query": "river", "limit": 50})).await;
    assert_eq!(last["continuation"], Value::Null);
    assert_eq!(paging_hints(&last), Vec::<Value>::new(), "no continuation, no paging hint");
    for max_tokens in [256, 400, 700, 1500, 4000] {
        for limit in [1, 5, 20, 30] {
            let found = harness
                .ok("search", json!({"query": "river", "limit": limit, "max_tokens": max_tokens}))
                .await;
            let paging = paging_hints(&found);
            assert!(paging.len() <= 1, "never both: {paging:?}");
            match paging.first() {
                Some(hint) => assert_eq!(hint["continuation"], found["continuation"]),
                None => assert!(
                    found["continuation"].is_null() || found["hints"].is_null(),
                    "a continuation gets a paging hint unless hints were dropped: {found}"
                ),
            }
        }
    }
    harness.shutdown().await;
}

fn body_cut_hint(result: &Value) -> Option<Value> {
    hints_of(result).into_iter().find(|hint| hint["reason"] == "body_cut")
}

/// § 8 test 8 and § 3.3: `fetch` and `expand` results with cut bodies get one
/// `body_cut` hint naming the cut refs; re-fetching them with the hinted
/// `max_tokens` returns them uncut.
#[tokio::test(flavor = "multi_thread")]
async fn fetch_body_cut_emits_hint_with_sufficient_max_tokens() {
    let mut writes = river_notes(6, 2_000);
    for i in 1..6 {
        writes.push(common::edge("link", "note:000", &format!("note:{i:03}")));
    }
    let (_fixture, harness) = harness_with(writes, json!({})).await;
    let refs: Vec<String> = (1..6).map(|i| format!("l:note:{i:03}")).collect();
    let fetched = harness.ok("fetch", json!({"refs": refs, "max_tokens": 1_000})).await;
    let cut: Vec<Value> = fetched["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|item| item["record"]["body_cut"] == true)
        .map(|item| item["ref"].clone())
        .collect();
    assert!(!cut.is_empty(), "the budget cut bodies: {fetched}");
    let hint = body_cut_hint(&fetched).expect("a body_cut hint");
    assert_eq!(hint["action"], "fetch");
    assert_eq!(hint["refs"], json!(cut));
    let max_tokens = hint["max_tokens"].as_u64().expect("max_tokens");
    let again = harness.ok("fetch", json!({"refs": hint["refs"], "max_tokens": max_tokens})).await;
    for item in again["items"].as_array().unwrap() {
        assert_eq!(item["record"]["body_cut"], false, "uncut at the hinted budget: {item}");
    }
    assert!(body_cut_hint(&again).is_none());

    let expanded = harness.ok("expand", json!({"refs": ["l:note:000"], "max_tokens": 1_000})).await;
    let cut: Vec<Value> = expanded["targets"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|target| target["body_cut"] == true)
        .map(|target| target["ref"].clone())
        .collect();
    assert!(!cut.is_empty(), "the budget cut bodies: {expanded}");
    let hint = body_cut_hint(&expanded).expect("a body_cut hint on expand");
    assert_eq!(hint["refs"], json!(cut));
    let max_tokens = hint["max_tokens"].as_u64().expect("max_tokens");
    let again = harness.ok("fetch", json!({"refs": hint["refs"], "max_tokens": max_tokens})).await;
    for item in again["items"].as_array().unwrap() {
        assert_eq!(item["record"]["body_cut"], false, "uncut at the hinted budget: {item}");
    }
    harness.shutdown().await;
}

/// § 2.1 and review fix 5: `list_stores` emits `default_store_set` naming the
/// session default store, and nothing when the session has none.
#[tokio::test(flavor = "multi_thread")]
async fn list_stores_emits_default_store_set_hint() {
    let fixture = Fixture::new();
    fixture.store("a.sqlite", river_notes(1, 40));
    let harness =
        Harness::start(fixture.config(json!({"stores": {"a": {"path": "a.sqlite"}}}))).await;
    let none = harness.ok("list_stores", json!({})).await;
    assert_eq!(hints_of(&none), Vec::<Value>::new(), "no default store, no hint");
    harness.ok("use_store", json!({"store": "a"})).await;
    let result = harness.call("list_stores", json!({})).await;
    let listed = result.structured_content.clone().unwrap();
    assert_eq!(
        hints_of(&listed),
        vec![json!({"action": "search", "reason": "default_store_set", "store": "a"})]
    );
    let text = common::text_of(&result);
    assert!(
        text.lines().any(|line| line.starts_with("hint: search store=\"a\" (default_store_set: ")),
        "{text}"
    );
    let off = harness.ok("list_stores", json!({"hints": false})).await;
    assert_eq!(hints_of(&off), Vec::<Value>::new());
    harness.shutdown().await;
}

/// § 8 test 9 and § 5.1: hints count against the budget like any field:
/// `used_tokens` is ceil(bytes / 4) with hints present, and every response
/// stays within `max_tokens` for every budget of at least 256.
#[tokio::test(flavor = "multi_thread")]
async fn hints_count_against_the_budget() {
    let mut writes = river_notes(40, 600);
    for i in 1..20 {
        writes.push(common::edge("link", "note:000", &format!("note:{i:03}")));
    }
    let (_fixture, harness) = harness_with(writes, json!({"snippet_chars": 80})).await;
    let fetch_refs: Vec<String> = (0..20).map(|i| format!("l:note:{i:03}")).collect();
    let mut with_hints = 0;
    for max_tokens in [256_u64, 300, 512, 1000, 2500, 4000, 20_000] {
        let calls = [
            ("search", json!({"query": "river", "limit": 20, "max_tokens": max_tokens})),
            ("search", json!({"query": "river", "mode": "hybrid", "max_tokens": max_tokens})),
            ("search", json!({"query": "zebra", "mode": "hybrid", "max_tokens": max_tokens})),
            ("fetch", json!({"refs": fetch_refs, "max_tokens": max_tokens})),
            ("expand", json!({"refs": ["l:note:000"], "max_tokens": max_tokens})),
        ];
        for (tool, args) in calls {
            let result = harness.call(tool, args).await;
            if result.is_error == Some(true) {
                continue;
            }
            let structured = result.structured_content.clone().unwrap();
            if structured.get("hints").is_some() {
                with_hints += 1;
            }
            let text = common::text_of(&result);
            let whole = serde_json::to_string(&structured).unwrap().len() + text.len();
            assert!(tokens(whole) <= max_tokens, "{tool} at {max_tokens}: {}", tokens(whole));
            let mut without = structured.clone();
            without.as_object_mut().unwrap().remove("budget");
            let used = tokens(serde_json::to_string(&without).unwrap().len() + text.len());
            assert_eq!(structured["budget"]["used_tokens"], used, "{tool} at {max_tokens}");
        }
    }
    assert!(with_hints >= 20, "hints were present in most responses: {with_hints}");
    harness.shutdown().await;
}

fn tokens(bytes: usize) -> u64 {
    (bytes as u64).div_ceil(4)
}

/// § 8 test 18 and § 2.3: every ref in a text block is JSON-quoted — in
/// `search` hit lines, `fetch` item lines and `expand` target lines, as in
/// hint lines — so a writer-chosen id reads as data. Unconditional: hints
/// on or off.
#[tokio::test(flavor = "multi_thread")]
async fn hit_line_refs_are_quoted_in_text_blocks() {
    let odd = "note:odd kind=\"fake\" (cut)";
    let writes = vec![
        node(odd, "note", "river crossing with an odd id"),
        node("note:2", "note", "river crossing plain"),
        common::edge("link", odd, "note:2"),
        common::edge("link", "note:2", odd),
    ];
    let (_fixture, harness) = harness_with(writes, json!({})).await;
    let quoted = |reference: &str| serde_json::to_string(reference).unwrap();
    let odd_ref = format!("l:{odd}");
    for hints in [true, false] {
        let result = harness.call("search", json!({"query": "river", "hints": hints})).await;
        let text = common::text_of(&result);
        let found = result.structured_content.clone().unwrap();
        for (index, reference) in common::refs(&found).iter().enumerate() {
            let line = format!("\n{}. {} kind=", index + 1, quoted(reference));
            assert!(text.contains(&line), "search hit line {line:?} in {text}");
        }
        let result =
            harness.call("fetch", json!({"refs": [odd_ref, "l:note:2"], "hints": hints})).await;
        let text = common::text_of(&result);
        for reference in [odd_ref.as_str(), "l:note:2"] {
            let line = format!("\n{} kind=", quoted(reference));
            assert!(text.contains(&line), "fetch item line {line:?} in {text}");
        }
        let result = harness.call("expand", json!({"refs": ["l:note:2"], "hints": hints})).await;
        let text = common::text_of(&result);
        let line = format!("\n{} kind=", quoted(&odd_ref));
        assert!(text.contains(&line), "expand target line {line:?} in {text}");
    }
    harness.shutdown().await;
}
