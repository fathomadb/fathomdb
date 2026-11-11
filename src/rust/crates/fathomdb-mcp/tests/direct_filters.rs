//! Closed predicates forwarded through the real provider-free direct text route.
mod common;

use std::sync::Arc;

use common::{node, refs, structured_config, Fixture, Harness};
use fathomdb_sdk::{
    Engine, PreparedWrite, ProjectionRole, ProjectionSpec, SearchFilter, TextSearchOptions,
};
use serde_json::{json, Value};

const PINNED: i64 = 1_900_000_000;

fn projection(name: &str, role: ProjectionRole) -> ProjectionSpec {
    ProjectionSpec {
        name: name.into(),
        roles: [role].into(),
        fts: None,
        vector: None,
        source: None,
    }
}

fn edge(id: &str, body: &str) -> PreparedWrite {
    let mut write = common::edge("link", "a", "b");
    if let PreparedWrite::Edge { logical_id, body: content, .. } = &mut write {
        *logical_id = Some(id.into());
        *content = Some(body.into());
    }
    write
}

fn filter(args: &Value) -> Option<SearchFilter> {
    let mut out = SearchFilter::default();
    out.kind = args["kind"].as_str().map(str::to_owned);
    out.source_type = args["source_type"].as_str().map(str::to_owned);
    out.status = args["status"].as_str().map(str::to_owned);
    out.created_after = args["created_after"].as_i64();
    if let Some(attributes) = args["attributes"].as_object() {
        out.attributes = attributes
            .iter()
            .map(|(key, value)| (key.clone(), value.as_str().unwrap().to_owned()))
            .collect();
    }
    Some(out)
}

fn sdk_hits(engine: &Engine, query: &str, args: &Value, limit: usize) -> Vec<(String, f64)> {
    let mut options = TextSearchOptions::default();
    options.filter = filter(args);
    options.view.valid_as_of = Some(PINNED);
    options.limit = limit;
    engine
        .search_text_only(query, options)
        .unwrap()
        .results
        .into_iter()
        .map(|hit| (hit.id.to_prefixed(), hit.score))
        .collect()
}

async fn hosted(engine: Arc<Engine>) -> Harness {
    let mut config = structured_config();
    config.hints = false;
    let harness = Harness::start(config).await;
    harness.adapter.register_engine("a", engine).unwrap();
    harness.ok("use_store", json!({"store":"a"})).await;
    harness
}

#[tokio::test]
async fn direct_text_forwards_every_closed_filter_and_conjunction() {
    let fixture = Fixture::new();
    let engine = Arc::new(Engine::open(fixture.path("direct.sqlite"), Default::default()).unwrap());
    engine
        .configure_projections(
            &[
                projection("group", ProjectionRole::Filterable),
                projection("level", ProjectionRole::Filterable),
            ],
            &[],
        )
        .unwrap();
    engine.write(&[
        node("a","doc",r#"{"text":"filterriver one","group":"wanted","level":2,"status":"open","created_at":0}"#),
        node("b","note",r#"{"text":"filterriver two","group":"other","level":2}"#),
        edge("e","filterriver edge"),
    ]).unwrap();
    let harness = hosted(engine.clone()).await;
    let mut failures = Vec::new();
    for (predicates, expected) in [
        (json!({"kind":"doc"}), vec!["l:a"]),
        (json!({"kind":"link"}), vec!["l:e"]),
        (json!({"source_type":"edge_fact"}), vec!["l:e"]),
        (json!({"source_type":"article","kind":"doc"}), vec!["l:a"]),
        (json!({"attributes":{"group":"wanted","level":"2"}}), vec!["l:a"]),
        (json!({"attributes":{"group":"wanted","level":"3"}}), vec![]),
        (json!({"kind":"doc","source_type":"edge_fact"}), vec![]),
        (json!({"status":""}), vec![]),
        (json!({"status":"open"}), vec![]),
        (json!({"created_after":0}), vec![]),
        (
            json!({"created_after":0,"status":"open","kind":"doc","source_type":"article","attributes":{"group":"wanted"}}),
            vec![],
        ),
    ] {
        let control = sdk_hits(&engine, "filterriver", &predicates, 100);
        assert_eq!(
            control.iter().map(|hit| hit.0.as_str()).collect::<Vec<_>>(),
            expected,
            "authored SDK fixture {predicates}"
        );
        for explicit_mode in [false, true] {
            let mut args = predicates.clone();
            args["query"] = json!("filterriver");
            args["as_of"] = json!(PINNED);
            args["limit"] = json!(100);
            if explicit_mode {
                args["mode"] = json!("text");
            }
            let result = harness.call("search", args).await;
            if result.is_error == Some(true) {
                failures.push(format!(
                    "{predicates}, explicit={explicit_mode}: {}",
                    common::text_of(&result)
                ));
                continue;
            }
            let found = result.structured_content.unwrap();
            assert_eq!(refs(&found), expected, "{predicates}");
            let actual = found["results"]
                .as_array()
                .unwrap()
                .iter()
                .map(|hit| {
                    (hit["ref"].as_str().unwrap().to_owned(), hit["score"].as_f64().unwrap())
                })
                .collect::<Vec<_>>();
            assert_eq!(actual, control, "same-snapshot exact SDK IDs/scores {predicates}");
            assert_eq!(found["mode"], "text");
            assert_eq!(found["degradation"], json!([]));
            assert!(found["results"].as_array().unwrap().iter().all(|hit| hit["arm"] == "text"));
        }
    }
    for predicates in [json!({}), json!({"attributes":{}})] {
        let mut args = predicates.clone();
        args["query"] = json!("filterriver");
        args["as_of"] = json!(PINNED);
        let result = harness.call("search", args).await;
        if result.is_error == Some(true) {
            failures.push(format!("empty predicates: {}", common::text_of(&result)));
            continue;
        }
        assert_eq!(
            refs(&result.structured_content.unwrap()),
            sdk_hits(&engine, "filterriver", &predicates, 10)
                .iter()
                .map(|hit| hit.0.clone())
                .collect::<Vec<_>>()
        );
    }
    let (direct, hybrid) = harness.adapter.seams().search_route_calls();
    assert_eq!(hybrid, 0, "no hybrid SDK dispatch for text predicates");
    if direct != 24 {
        failures.push(format!("direct SDK dispatch expected24, got{direct}"));
    }
    harness.shutdown().await;
    engine.close().unwrap();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[tokio::test]
async fn direct_text_filters_precede_candidate_cap_and_keep_node_attribute_scope() {
    let fixture = Fixture::new();
    let engine = Arc::new(Engine::open(fixture.path("cap.sqlite"), Default::default()).unwrap());
    engine
        .configure_projections(
            &[
                projection("group", ProjectionRole::Filterable),
                projection("level", ProjectionRole::Filterable),
            ],
            &[],
        )
        .unwrap();
    let mut writes = (0..120)
        .map(|i| {
            node(
                &format!("n{i}"),
                "doc",
                &format!(r#"{{"text":"capriver","group":"other","level":1,"i":{i}}}"#),
            )
        })
        .collect::<Vec<_>>();
    let body =
        format!(r#"{{"text":"capriver {}","group":"wanted","level":2}}"#, "padding ".repeat(100));
    writes.push(node("target", "doc", &body));
    writes.push(edge("twin", &body));
    engine.write(&writes).unwrap();
    let unfiltered = sdk_hits(&engine, "capriver", &json!({}), 100);
    assert!(!unfiltered.iter().any(|hit| hit.0 == "l:target"), "target below unfiltered cap");
    let harness = hosted(engine.clone()).await;
    let mut failures = Vec::new();
    for (predicates, expected) in [
        (
            json!({"kind":"doc","source_type":"article","attributes":{"group":"wanted","level":"2"}}),
            vec!["l:target"],
        ),
        (json!({"attributes":{"group":"wanted","level":"2"}}), vec!["l:target"]),
        (json!({"attributes":{"group":"wanted","level":"3"}}), vec![]),
    ] {
        assert_eq!(
            sdk_hits(&engine, "capriver", &predicates, 1)
                .iter()
                .map(|hit| hit.0.as_str())
                .collect::<Vec<_>>(),
            expected
        );
        let mut args = predicates.clone();
        args["query"] = json!("capriver");
        args["limit"] = json!(1);
        args["as_of"] = json!(PINNED);
        let result = harness.call("search", args).await;
        if result.is_error == Some(true) {
            failures.push(common::text_of(&result));
            continue;
        }
        assert_eq!(refs(&result.structured_content.unwrap()), expected);
    }
    harness.shutdown().await;
    engine.close().unwrap();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[tokio::test]
async fn direct_text_validates_attributes_even_for_no_matches() {
    let fixture = Fixture::new();
    let engine =
        Arc::new(Engine::open(fixture.path("invalid.sqlite"), Default::default()).unwrap());
    engine.configure_projections(&[projection("rankonly", ProjectionRole::Rankable)], &[]).unwrap();
    let harness = hosted(engine.clone()).await;
    let mut failures = Vec::new();
    for attribute in ["unknown", "rankonly"] {
        let error =
            harness.err("search", json!({"query":"nohit","attributes":{attribute:"wanted"}})).await;
        assert_eq!(error["code"], "bad_argument");
        if !error["detail"].as_str().is_some_and(|detail| detail.contains("InvalidFilterError")) {
            failures.push(format!("{attribute}: {error}"));
        }
    }
    for invalid in [
        json!({"kind":1}),
        json!({"source_type":true}),
        json!({"status":[]}),
        json!({"created_after":"1"}),
        json!({"attributes":{"group":1}}),
        json!({"attributes":[]}),
        json!({"created_after":1.5}),
    ] {
        let mut args = invalid;
        args["query"] = json!("nohit");
        let error = harness.err("search", args).await;
        assert_eq!(error["code"], "bad_argument");
    }
    harness.shutdown().await;
    engine.close().unwrap();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[tokio::test]
async fn direct_text_predicates_survive_paging_and_define_canonical_identity() {
    let fixture = Fixture::new();
    let engine = Arc::new(Engine::open(fixture.path("pages.sqlite"), Default::default()).unwrap());
    engine
        .configure_projections(
            &[
                projection("group", ProjectionRole::Filterable),
                projection("level", ProjectionRole::Filterable),
            ],
            &[],
        )
        .unwrap();
    engine
        .write(
            &(0..24)
                .map(|i| {
                    node(
                        &format!("n{i:02}"),
                        "doc",
                        &format!(
                            r#"{{"text":"pageriver entry {i:02}","group":"wanted","level":2}}"#
                        ),
                    )
                })
                .collect::<Vec<_>>(),
        )
        .unwrap();
    let harness = hosted(engine.clone()).await;
    let predicates =
        json!({"kind":"doc","source_type":"article","attributes":{"level":"2","group":"wanted"}});
    let mut first = predicates.clone();
    first["query"] = json!("pageriver");
    first["limit"] = json!(5);
    first["as_of"] = json!(PINNED);
    let result = harness.call("search", first.clone()).await;
    let error = result.is_error == Some(true);
    if error {
        harness.shutdown().await;
        engine.close().unwrap();
        panic!("filtered initial page must succeed: {}", common::text_of(&result));
    }
    let mut page = result.structured_content.unwrap();
    let control = sdk_hits(&engine, "pageriver", &predicates, 100);
    assert_eq!(control.len(), 24);
    let original_id = page["response_id"].clone();
    let mut reordered = first.clone();
    reordered["attributes"] = serde_json::from_str(r#"{"group":"wanted","level":"2"}"#).unwrap();
    assert_eq!(harness.ok("search", reordered).await["response_id"], original_id);
    let mut seen = Vec::new();
    for _ in 0..10 {
        seen.extend(refs(&page));
        let Some(handle) = page["continuation"].as_str() else { break };
        for override_arg in [
            json!({"attributes":{}}),
            json!({"kind":"doc"}),
            json!({"source_type":"article"}),
            json!({"status":""}),
            json!({"created_after":0}),
        ] {
            let mut args = override_arg;
            args["continuation"] = json!(handle);
            assert_eq!(harness.err("search", args).await["code"], "bad_argument");
        }
        page = harness.ok("search", json!({"continuation":handle})).await;
        assert_eq!(page["validity_instant"], PINNED);
        assert_eq!(page["continuation_exact"], false);
    }
    assert_eq!(seen, control.iter().map(|hit| hit.0.clone()).collect::<Vec<_>>());
    let base = harness.ok("search", json!({"query":"pageriver","as_of":PINNED,"limit":5})).await;
    for predicate in [
        json!({"kind":"doc"}),
        json!({"source_type":"article"}),
        json!({"status":""}),
        json!({"created_after":0}),
        json!({"attributes":{"group":"wanted"}}),
    ] {
        let mut args = predicate;
        args["query"] = json!("pageriver");
        args["as_of"] = json!(PINNED);
        args["limit"] = json!(5);
        assert_ne!(harness.ok("search", args).await["response_id"], base["response_id"]);
    }
    let first = harness.ok("search", first).await;
    engine
        .write(&[node("new", "doc", r#"{"text":"pageriver newest","group":"other","level":2}"#)])
        .unwrap();
    let next = harness.ok("search", json!({"continuation":first["continuation"]})).await;
    let eligible = sdk_hits(&engine, "pageriver", &predicates, 100);
    assert!(refs(&next).iter().all(|reference| eligible.iter().any(|hit| &hit.0 == reference)));
    assert!(!refs(&next).contains(&"l:new".to_owned()));
    assert_eq!(next["continuation_exact"], false);
    harness.shutdown().await;
    engine.close().unwrap();
}
