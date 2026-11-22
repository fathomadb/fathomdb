//! Search view flags through real MCP calls and same-state SDK controls.
mod common;

use std::sync::Arc;

use common::{node, text_of, Fixture, Harness};
use fathomdb_sdk::{
    Engine, PreparedWrite, ProjectionRole, ProjectionSpec, ReadView, SearchFilter, SearchFilterArg,
    SearchOptions, SnippetOptions, TextSearchOptions,
};
use rmcp::model::CallToolResult;
use serde_json::{json, Value};

const PINNED: i64 = 1_900_000_000;
const QUERY: &str = "historyriver";

#[derive(Debug, PartialEq)]
struct Row {
    reference: String,
    snippet: String,
    score: f64,
    source: Option<String>,
}

struct Page {
    rows: Vec<Row>,
    response_id: String,
    continuation: Option<String>,
    validity_instant: i64,
    continuation_exact: Option<bool>,
    cut: bool,
}

fn header<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    text.lines()
        .next()
        .unwrap()
        .split_whitespace()
        .find_map(|word| word.strip_prefix(name).and_then(|tail| tail.strip_prefix('=')))
}

fn page(result: &CallToolResult, format: &str, max_tokens: u64) -> Page {
    let text = text_of(result);
    if format == "structured" {
        let value = result.structured_content.as_ref().expect("structured payload");
        assert_eq!(text, serde_json::to_string(value).unwrap());
        assert!((text.len() as u64 * 2).div_ceil(4) <= max_tokens);
        Page {
            rows: value["results"]
                .as_array()
                .unwrap()
                .iter()
                .map(|hit| Row {
                    reference: hit["ref"].as_str().unwrap().into(),
                    snippet: hit["snippet"].as_str().unwrap().into(),
                    score: hit["score"].as_f64().unwrap(),
                    source: hit["source_id"].as_str().map(str::to_owned),
                })
                .collect(),
            response_id: value["response_id"].as_str().unwrap().into(),
            continuation: value["continuation"].as_str().map(str::to_owned),
            validity_instant: value["validity_instant"].as_i64().unwrap(),
            continuation_exact: value["continuation_exact"].as_bool(),
            cut: value["budget"]["cut"].as_bool().unwrap(),
        }
    } else {
        assert!(result.structured_content.is_none());
        let count = header(&text, "tokens").unwrap().split_once('/').unwrap();
        assert_eq!(count.0.parse::<u64>().unwrap(), (text.len() as u64).div_ceil(4));
        assert_eq!(count.1.parse::<u64>().unwrap(), max_tokens);
        assert!((text.len() as u64).div_ceil(4) <= max_tokens);
        let rows = text
            .lines()
            .filter(|line| line.starts_with('"'))
            .map(|line| {
                let mut reference = serde_json::Deserializer::from_str(line).into_iter::<String>();
                let id = reference.next().unwrap().unwrap();
                let (fields, snippet) = line[reference.byte_offset()..].split_once(" | ").unwrap();
                let score =
                    fields.split_whitespace().find_map(|word| word.strip_prefix("score=")).unwrap();
                assert_eq!(score.split_once('.').unwrap().1.len(), 3);
                let source = fields.split_once(" src=").map(|(_, quoted)| {
                    serde_json::Deserializer::from_str(quoted)
                        .into_iter::<String>()
                        .next()
                        .unwrap()
                        .unwrap()
                });
                Row {
                    reference: id,
                    snippet: snippet.into(),
                    score: score.parse().unwrap(),
                    source,
                }
            })
            .collect();
        Page {
            rows,
            response_id: header(&text, "response_id").unwrap().into(),
            continuation: header(&text, "continuation").map(str::to_owned),
            validity_instant: header(&text, "validity_instant").unwrap().parse().unwrap(),
            continuation_exact: header(&text, "continuation_exact").map(|value| value == "true"),
            cut: text.lines().next().unwrap().split_whitespace().any(|word| word == "cut"),
        }
    }
}

async fn search(harness: &Harness, args: Value, format: &str) -> Page {
    let max_tokens = args["max_tokens"].as_u64().unwrap_or(100_000);
    let result = harness.call("search", args.clone()).await;
    assert_ne!(
        result.is_error,
        Some(true),
        "F11 search view must be accepted: {args}: {}",
        text_of(&result)
    );
    let page = page(&result, format, max_tokens);
    assert_eq!(page.validity_instant, PINNED);
    page
}

fn body(label: &str, group: &str) -> String {
    json!({"text":format!("historyriver {label}"),"group":group,"pad":"padding ".repeat(20)})
        .to_string()
}

fn edge(body: &str) -> PreparedWrite {
    let mut write = common::body_edge("link", "from", "to", body);
    if let PreparedWrite::Edge { logical_id, .. } = &mut write {
        *logical_id = Some("version-edge".into());
    }
    write
}

fn history() -> (Fixture, Arc<Engine>) {
    let fixture = Fixture::new();
    let engine = Arc::new(Engine::open(fixture.path("views.sqlite"), Default::default()).unwrap());
    engine
        .configure_projections(
            &[ProjectionSpec {
                name: "group".into(),
                roles: [ProjectionRole::Filterable].into(),
                fts: None,
                vector: None,
                source: None,
                entity_class: fathomdb_sdk::PropertyEntityClass::Node,
            }],
            &[],
        )
        .unwrap();
    engine
        .write(&[
            node("from", "note", "edge endpoint from"),
            node("to", "note", "edge endpoint to"),
            node("version", "note", &body("retired edition", "wanted")),
            node("twins", "note", &body("identical edition", "wanted")),
            node("solo", "note", &body("identityriver standalone", "wanted")),
            edge(&body("retired edge", "wanted")),
        ])
        .unwrap();
    engine
        .write(&[
            node("version", "note", &body("current edition", "other")),
            common::sourced_node(
                "twins",
                "note",
                &body("identical edition", "wanted"),
                "fixture:twins-new",
            ),
            edge(&body("current edge", "other")),
        ])
        .unwrap();
    (fixture, engine)
}

async fn hosted(engine: Arc<Engine>, format: &str) -> Harness {
    let mut config = common::structured_config();
    config.result_format = format.into();
    config.hints = false;
    let harness = Harness::start(config).await;
    harness.adapter.register_engine("a", engine).unwrap();
    let result = harness.call("use_store", json!({"store":"a"})).await;
    assert_ne!(result.is_error, Some(true));
    harness
}

fn sdk_rows(
    engine: &Engine,
    mode: &str,
    superseded: bool,
    query: &str,
    filter: Option<SearchFilter>,
) -> Vec<Row> {
    let view = ReadView {
        include_superseded: superseded,
        valid_as_of: Some(PINNED),
        ..ReadView::default()
    };
    let mut snippet = SnippetOptions::default();
    snippet.max_bytes = 300;
    snippet.max_windows = 1;
    snippet.head_bytes = 48;
    snippet.max_point_queries = 100;
    let result = if mode == "text" {
        let mut options = TextSearchOptions::default();
        options.view = view;
        options.filter = filter;
        options.limit = 100;
        options.snippet = Some(snippet);
        engine.search_text_only(query, options).unwrap()
    } else {
        let mut options = SearchOptions::default();
        options.view = view;
        options.filter = filter.map(SearchFilterArg::Search);
        options.limit = 100;
        options.snippet = Some(snippet);
        engine.search(query, options).unwrap()
    };
    let snippets = result.snippets.unwrap();
    assert_eq!(result.results.len(), snippets.len());
    result
        .results
        .into_iter()
        .zip(snippets)
        .map(|(hit, snippet)| {
            assert_eq!(snippet.text, hit.body, "authored fixture bodies fit the whole snippet");
            Row {
                reference: hit.id.to_prefixed(),
                snippet: snippet.text,
                score: hit.score,
                source: hit.source_id,
            }
        })
        .collect()
}

fn assert_rows(actual: &[Row], expected: &[Row], format: &str) {
    assert_eq!(actual.len(), expected.len());
    for (actual, expected) in actual.iter().zip(expected) {
        assert_eq!(actual.reference, expected.reference);
        assert_eq!(actual.snippet, expected.snippet);
        assert_eq!(actual.source, expected.source);
        if format == "structured" {
            assert_eq!(actual.score, expected.score);
        } else {
            assert_eq!(actual.score, format!("{:.3}", expected.score).parse::<f64>().unwrap());
        }
    }
}

fn assert_authored(rows: &[Row], superseded: bool) {
    let mut actual =
        rows.iter().map(|row| (row.reference.clone(), row.snippet.clone())).collect::<Vec<_>>();
    let mut expected = vec![
        ("l:version".into(), body("current edition", "other")),
        ("l:twins".into(), body("identical edition", "wanted")),
        ("l:solo".into(), body("identityriver standalone", "wanted")),
        ("l:version-edge".into(), body("current edge", "other")),
    ];
    if superseded {
        expected.extend([
            ("l:version".into(), body("retired edition", "wanted")),
            ("l:twins".into(), body("identical edition", "wanted")),
            ("l:version-edge".into(), body("retired edge", "wanted")),
        ]);
    }
    let mut twin_sources = rows
        .iter()
        .filter(|row| row.reference == "l:twins")
        .map(|row| row.source.as_deref().unwrap())
        .collect::<Vec<_>>();
    twin_sources.sort();
    assert_eq!(
        twin_sources,
        if superseded { vec!["fixture", "fixture:twins-new"] } else { vec!["fixture:twins-new"] }
    );
    actual.sort();
    expected.sort();
    assert_eq!(actual, expected, "authored record/version membership, not adapter self-parity");
}

fn args(mode: &str, limit: usize) -> Value {
    json!({"query":QUERY,"mode":mode,"as_of":PINNED,"limit":limit,"max_tokens":100_000})
}

#[tokio::test]
async fn search_superseded_view_returns_distinct_node_edge_and_equal_body_revisions() {
    let (_fixture, engine) = history();
    for format in ["structured", "text"] {
        let harness = hosted(engine.clone(), format).await;
        for mode in ["text", "hybrid"] {
            let expected = sdk_rows(&engine, mode, true, QUERY, None);
            assert_authored(&expected, true);
            let mut request = args(mode, 100);
            request["include_superseded"] = json!(true);
            let found = search(&harness, request, format).await;
            assert_rows(&found.rows, &expected, format);
            let tools = serde_json::to_value(harness.tools().await).unwrap();
            let schema =
                &tools.as_array().unwrap().iter().find(|tool| tool["name"] == "search").unwrap()
                    ["inputSchema"];
            assert!(schema["properties"].get("include_superseded").is_some());
            let input = jsonschema::JSONSchema::options()
                .with_draft(jsonschema::Draft::Draft202012)
                .compile(schema)
                .unwrap();
            assert!(input.is_valid(&json!({"query":QUERY,"include_superseded":true})));
            assert!(!input.is_valid(&json!({"query":QUERY,"include_superseded":"true"})));
            let strict = sdk_rows(&engine, mode, false, QUERY, None);
            assert_authored(&strict, false);
            assert_rows(&search(&harness, args(mode, 100), format).await.rows, &strict, format);
        }
        harness.shutdown().await;
    }
    engine.close().unwrap();
}

#[tokio::test]
async fn superseded_canonical_identity_and_continuations_keep_the_effective_view() {
    let (_fixture, engine) = history();
    for format in ["structured", "text"] {
        let harness = hosted(engine.clone(), format).await;
        for mode in ["text", "hybrid"] {
            let mut request = args(mode, 2);
            request["query"] = json!("identityriver");
            let absent = search(&harness, request.clone(), format).await;
            request["include_superseded"] = json!(false);
            let explicit_false = search(&harness, request.clone(), format).await;
            assert_eq!(absent.response_id, explicit_false.response_id);
            request["include_superseded"] = json!(true);
            let relaxed = search(&harness, request, format).await;
            assert_eq!(absent.rows, relaxed.rows, "same selected record isolates request identity");
            assert_ne!(absent.response_id, relaxed.response_id);

            let expected = sdk_rows(&engine, mode, true, QUERY, None);
            let mut request = args(mode, 2);
            request["include_superseded"] = json!(true);
            let mut found = search(&harness, request, format).await;
            let mut rows = Vec::new();
            for _ in 0..10 {
                assert_eq!(found.continuation_exact, Some(false));
                rows.extend(found.rows);
                let Some(handle) = found.continuation else {
                    break;
                };
                for flag in [true, false] {
                    let result = harness
                        .call("search", json!({"continuation":handle,"include_superseded":flag}))
                        .await;
                    assert_eq!(result.is_error, Some(true), "explicit view override must refuse");
                    assert!(text_of(&result).contains("bad_argument"));
                }
                found =
                    search(&harness, json!({"continuation":handle,"max_tokens":100_000}), format)
                        .await;
            }
            assert_rows(&rows, &expected, format);
            for wrong_type in [json!("true"), json!(1), json!([]), json!({})] {
                let mut request = args(mode, 2);
                request["include_superseded"] = wrong_type;
                let result = harness.call("search", request).await;
                assert_eq!(result.is_error, Some(true));
                assert!(text_of(&result).contains("bad_argument"));
            }
        }
        harness.shutdown().await;
    }
    engine.close().unwrap();
}

#[tokio::test]
async fn superseded_filters_and_budgeted_pages_preserve_ranked_occurrences() {
    let (_fixture, engine) = history();
    for format in ["structured", "text"] {
        let harness = hosted(engine.clone(), format).await;
        for mode in ["text", "hybrid"] {
            let mut filter = SearchFilter::default();
            filter.attributes = vec![("group".into(), "wanted".into())];
            let expected = sdk_rows(&engine, mode, true, QUERY, Some(filter));
            let mut authored =
                expected.iter().map(|row| row.reference.as_str()).collect::<Vec<_>>();
            authored.sort();
            assert_eq!(authored, vec!["l:solo", "l:twins", "l:twins", "l:version"]);
            let mut request = args(mode, 100);
            request["include_superseded"] = json!(true);
            request["attributes"] = json!({"group":"wanted"});
            assert_rows(&search(&harness, request, format).await.rows, &expected, format);
            for (kind, source_type, wanted) in [
                (
                    Some("note"),
                    None,
                    vec!["l:solo", "l:twins", "l:twins", "l:version", "l:version"],
                ),
                (None, Some("edge_fact"), vec!["l:version-edge", "l:version-edge"]),
                (Some("note"), Some("edge_fact"), vec![]),
            ] {
                let mut filter = SearchFilter::default();
                filter.kind = kind.map(str::to_owned);
                filter.source_type = source_type.map(str::to_owned);
                let control = sdk_rows(&engine, mode, true, QUERY, Some(filter));
                let mut ids = control.iter().map(|row| row.reference.as_str()).collect::<Vec<_>>();
                ids.sort();
                assert_eq!(ids, wanted);
                let mut request = args(mode, 100);
                request["include_superseded"] = json!(true);
                if let Some(kind) = kind {
                    request["kind"] = json!(kind);
                }
                if let Some(source_type) = source_type {
                    request["source_type"] = json!(source_type);
                }
                assert_rows(&search(&harness, request, format).await.rows, &control, format);
            }
            let mut invalid = args(mode, 100);
            invalid["query"] = json!("nomatchingterm");
            invalid["include_superseded"] = json!(true);
            invalid["attributes"] = json!({"undeclared":"wanted"});
            let error = harness.call("search", invalid).await;
            assert_eq!(error.is_error, Some(true));
            assert!(text_of(&error).contains("InvalidFilterError"));
            for predicate in [json!({"status":""}), json!({"created_after":0})] {
                let mut request = args(mode, 100);
                request["include_superseded"] = json!(true);
                request.as_object_mut().unwrap().extend(predicate.as_object().unwrap().clone());
                assert!(
                    search(&harness, request, format).await.rows.is_empty(),
                    "missing historical metadata never matches"
                );
            }
            let expected = sdk_rows(&engine, mode, true, QUERY, None);
            let budget = if format == "text" { 256 } else { 1000 };
            let mut request = args(mode, 100);
            request["include_superseded"] = json!(true);
            request["max_tokens"] = json!(budget);
            let mut found = search(&harness, request, format).await;
            assert!(found.cut, "authored long whole hits must exercise the budget cut");
            let mut rows = Vec::new();
            for _ in 0..10 {
                assert!(
                    !found.rows.is_empty(),
                    "selected fixture budget admits a whole leading hit"
                );
                rows.extend(found.rows);
                let Some(handle) = found.continuation else {
                    break;
                };
                found =
                    search(&harness, json!({"continuation":handle,"max_tokens":budget}), format)
                        .await;
            }
            assert_rows(&rows, &expected, format);
        }
        harness.shutdown().await;
    }
    engine.close().unwrap();
}

#[tokio::test]
async fn historical_revisions_compose_with_atomic_feedback_and_not_recorded_precedence() {
    for format in ["structured", "text"] {
        for (telemetry, mode, reason) in [
            (true, "hybrid", None),
            (false, "hybrid", Some("telemetry_disabled")),
            (true, "text", Some("text_mode")),
        ] {
            let (fixture, engine) = history();
            let mut config = common::structured_config();
            config.result_format = format.into();
            config.hints = false;
            if telemetry {
                config.telemetry_dir = Some(fixture.path("telemetry"));
            }
            let harness = Harness::start(config).await;
            harness.adapter.register_engine("a", engine.clone()).unwrap();
            assert_ne!(harness.call("use_store", json!({"store":"a"})).await.is_error, Some(true));
            let mut request = args(mode, 100);
            request["include_superseded"] = json!(true);
            let found = search(&harness, request, format).await;
            assert_eq!(found.rows.iter().filter(|row| row.reference == "l:version").count(), 2);
            assert_eq!(found.rows.iter().filter(|row| row.reference == "l:twins").count(), 2);
            let sink = fixture.path("telemetry").join(format!("a.{}.1.jsonl", std::process::id()));
            for (used, unused) in [
                (json!(["l:solo", "l:version"]), json!([])),
                (json!(["l:solo"]), json!(["l:twins"])),
            ] {
                let response = harness
                    .call(
                        "feedback",
                        json!({"response_id":found.response_id,"used":used,"unused":unused}),
                    )
                    .await;
                let text = text_of(&response);
                if let Some(reason) = reason {
                    assert_ne!(response.is_error, Some(true));
                    if format == "structured" {
                        let output = response.structured_content.unwrap();
                        assert_eq!(output["status"], "not_recorded");
                        assert_eq!(output["reason"], reason);
                    } else {
                        assert!(text.contains("status=not_recorded"));
                        assert!(text.contains(&format!("reason={reason}")));
                    }
                } else {
                    assert_eq!(response.is_error, Some(true));
                    assert!(text.contains("invalid_request"));
                    assert!(text.contains("bad_argument"));
                }
                let records = std::fs::read_to_string(&sink).unwrap_or_default();
                assert!(
                    records
                        .lines()
                        .all(|line| serde_json::from_str::<Value>(line).unwrap()["type"]
                            != "feedback"),
                    "no partial feedback label may escape"
                );
            }
            if reason.is_none() {
                let result = harness
                    .call("feedback", json!({"response_id":found.response_id,"used":["l:solo"]}))
                    .await;
                assert_ne!(
                    result.is_error,
                    Some(true),
                    "unmentioned historical ambiguity must not block unique labels"
                );
                let records = std::fs::read_to_string(&sink).unwrap();
                let labels = records
                    .lines()
                    .map(|line| serde_json::from_str::<Value>(line).unwrap())
                    .filter(|row| row["type"] == "feedback")
                    .collect::<Vec<_>>();
                assert_eq!(labels.len(), 1);
            }
            harness.shutdown().await;
            engine.close().unwrap();
        }
    }
}

fn pending(id: &str, label: &str) -> PreparedWrite {
    let mut write = node(id, "note", &body(label, "wanted"));
    if let PreparedWrite::Node { state, .. } = &mut write {
        *state = fathomdb_sdk::InitialState::Pending;
    }
    write
}

fn timed_edge(id: &str, label: &str, from: Option<i64>, until: Option<i64>) -> PreparedWrite {
    let mut write = edge(&body(label, "wanted"));
    if let PreparedWrite::Edge { logical_id, to, t_valid, t_invalid, .. } = &mut write {
        *logical_id = Some(id.into());
        *to = format!("to:{id}");
        *t_valid = from;
        *t_invalid = until;
    }
    write
}

fn inactive_history() -> (Fixture, Arc<Engine>) {
    let (fixture, engine) = history();
    let mut window = node("window", "note", &body("window", "wanted"));
    if let PreparedWrite::Node { valid_from, valid_until, .. } = &mut window {
        *valid_from = Some(PINNED);
        *valid_until = Some(PINNED + 10);
    }
    engine
        .write(&[
            pending("version", "pending edition"),
            pending("pending", "pending record"),
            node("deleted", "note", &body("deleted record", "wanted")),
            window,
            node("to:expired", "note", "timed endpoint"),
            node("to:expiring", "note", "timed endpoint"),
            node("to:future-valid", "note", "timed endpoint"),
            timed_edge("expired", "expired edge", None, Some(PINNED)),
            timed_edge("expiring", "expiring edge", None, Some(PINNED + 10)),
            timed_edge("future-valid", "future-valid edge", Some(PINNED + 20), None),
        ])
        .unwrap();
    engine.transition("deleted", fathomdb_sdk::LifecycleState::Deleted, Some("retracted")).unwrap();
    (fixture, engine)
}

fn view_rows(
    engine: &Engine,
    mode: &str,
    view: ReadView,
    query: &str,
    filter: Option<SearchFilter>,
) -> Vec<Row> {
    let mut snippet = SnippetOptions::default();
    snippet.max_bytes = 300;
    snippet.max_windows = 1;
    snippet.head_bytes = 48;
    snippet.max_point_queries = 100;
    let result = if mode == "text" {
        let mut options = TextSearchOptions::default();
        options.view = view;
        options.filter = filter;
        options.limit = 100;
        options.snippet = Some(snippet);
        engine.search_text_only(query, options).unwrap()
    } else {
        let mut options = SearchOptions::default();
        options.view = view;
        options.filter = filter.map(SearchFilterArg::Search);
        options.limit = 100;
        options.snippet = Some(snippet);
        engine.search(query, options).unwrap()
    };
    let snippets = result.snippets.unwrap();
    assert_eq!(result.results.len(), snippets.len());
    result
        .results
        .into_iter()
        .zip(snippets)
        .map(|(hit, snippet)| {
            assert_eq!(snippet.text, hit.body);
            Row {
                reference: hit.id.to_prefixed(),
                snippet: snippet.text,
                score: hit.score,
                source: hit.source_id,
            }
        })
        .collect()
}

fn authored_view(superseded: bool, inactive: bool, instant: i64) -> Vec<(String, String)> {
    let mut expected = vec![
        ("l:twins".into(), body("identical edition", "wanted")),
        ("l:solo".into(), body("identityriver standalone", "wanted")),
        ("l:version-edge".into(), body("current edge", "other")),
        ("l:future-valid".into(), body("future-valid edge", "wanted")),
    ];
    if superseded {
        expected.extend([
            ("l:version".into(), body("retired edition", "wanted")),
            ("l:version".into(), body("current edition", "other")),
            ("l:twins".into(), body("identical edition", "wanted")),
            ("l:version-edge".into(), body("retired edge", "wanted")),
        ]);
    }
    if inactive {
        expected.extend([
            ("l:version".into(), body("pending edition", "wanted")),
            ("l:pending".into(), body("pending record", "wanted")),
            ("l:deleted".into(), body("deleted record", "wanted")),
        ]);
    }
    if (PINNED..PINNED + 10).contains(&instant) {
        expected.push(("l:window".into(), body("window", "wanted")));
    }
    if instant < PINNED {
        expected.push(("l:expired".into(), body("expired edge", "wanted")));
    }
    if instant < PINNED + 10 {
        expected.push(("l:expiring".into(), body("expiring edge", "wanted")));
    }
    expected.sort();
    expected
}

fn view(superseded: bool, inactive: bool, instant: i64) -> ReadView {
    ReadView {
        include_superseded: superseded,
        include_inactive: inactive,
        valid_as_of: Some(instant),
        ..ReadView::default()
    }
}

async fn search_at(harness: &Harness, request: Value, format: &str, instant: i64) -> Page {
    let budget = request["max_tokens"].as_u64().unwrap();
    let result = harness.call("search", request.clone()).await;
    assert_ne!(
        result.is_error,
        Some(true),
        "F11 inactive search view must be accepted: {request}: {}",
        text_of(&result)
    );
    let found = page(&result, format, budget);
    assert_eq!(found.validity_instant, instant);
    found
}

#[tokio::test]
async fn inactive_and_combined_views_obey_authored_node_lifecycle_and_edge_expiry() {
    let (_fixture, engine) = inactive_history();
    for format in ["structured", "text"] {
        let harness = hosted(engine.clone(), format).await;
        for mode in ["text", "hybrid"] {
            for (superseded, inactive) in
                [(false, true), (true, true), (false, false), (true, false)]
            {
                for instant in [PINNED - 1, PINNED, PINNED + 9, PINNED + 10] {
                    let control =
                        view_rows(&engine, mode, view(superseded, inactive, instant), QUERY, None);
                    let mut membership = control
                        .iter()
                        .map(|row| (row.reference.clone(), row.snippet.clone()))
                        .collect::<Vec<_>>();
                    membership.sort();
                    assert_eq!(membership,authored_view(superseded,inactive,instant),"SDK fixture independently follows node windows and t_invalid-only edge expiry");
                    let mut request = args(mode, 100);
                    request["include_superseded"] = json!(superseded);
                    request["include_inactive"] = json!(inactive);
                    request["as_of"] = json!(instant);
                    let found = search_at(&harness, request, format, instant).await;
                    assert_rows(&found.rows, &control, format);
                }
            }
            let tools = serde_json::to_value(harness.tools().await).unwrap();
            let schema =
                &tools.as_array().unwrap().iter().find(|tool| tool["name"] == "search").unwrap()
                    ["inputSchema"];
            assert!(schema["properties"].get("include_inactive").is_some());
            assert!(schema["properties"].get("include_out_of_window").is_none());
            let schema = jsonschema::JSONSchema::options()
                .with_draft(jsonschema::Draft::Draft202012)
                .compile(schema)
                .unwrap();
            assert!(schema.is_valid(
                &json!({"query":QUERY,"include_superseded":true,"include_inactive":true})
            ));
            assert!(!schema.is_valid(&json!({"query":QUERY,"include_inactive":"true"})));
        }
        harness.shutdown().await;
    }
    engine.close().unwrap();
}

#[tokio::test]
async fn inactive_identity_filters_and_budgeted_continuations_keep_both_flags() {
    let (_fixture, engine) = inactive_history();
    for format in ["structured", "text"] {
        let harness = hosted(engine.clone(), format).await;
        for mode in ["text", "hybrid"] {
            for superseded in [false, true] {
                let mut request = args(mode, 100);
                request["query"] = json!("identityriver");
                request["include_superseded"] = json!(superseded);
                let absent = search(&harness, request.clone(), format).await;
                request["include_inactive"] = json!(false);
                let explicit = search(&harness, request.clone(), format).await;
                assert_eq!(absent.rows, explicit.rows);
                assert_eq!(absent.response_id, explicit.response_id);
                request["include_inactive"] = json!(true);
                let relaxed = search(&harness, request, format).await;
                assert_eq!(
                    absent.rows, relaxed.rows,
                    "isolate the canonical flag from record selection"
                );
                assert_ne!(absent.response_id, relaxed.response_id);
            }
            let mut filter = SearchFilter::default();
            filter.attributes = vec![("group".into(), "wanted".into())];
            let control = view_rows(&engine, mode, view(true, true, PINNED), QUERY, Some(filter));
            let mut ids = control.iter().map(|row| row.reference.as_str()).collect::<Vec<_>>();
            ids.sort();
            assert_eq!(
                ids,
                vec![
                    "l:deleted",
                    "l:pending",
                    "l:solo",
                    "l:twins",
                    "l:twins",
                    "l:version",
                    "l:version",
                    "l:window"
                ]
            );
            let mut request = args(mode, 100);
            request["include_superseded"] = json!(true);
            request["include_inactive"] = json!(true);
            request["attributes"] = json!({"group":"wanted"});
            assert_rows(&search(&harness, request, format).await.rows, &control, format);
            for wrong_type in [json!("true"), json!(1), json!([]), json!({})] {
                let mut request = args(mode, 2);
                request["include_inactive"] = wrong_type;
                let error = harness.call("search", request).await;
                assert_eq!(error.is_error, Some(true));
                assert!(text_of(&error).contains("bad_argument"));
            }
            let error =
                harness.call("search", json!({"query":QUERY,"include_out_of_window":true})).await;
            assert_eq!(error.is_error, Some(true));
            assert!(text_of(&error).contains("unknown_argument"));
            assert!(text_of(&error).contains("include_out_of_window"));
            let expected = view_rows(&engine, mode, view(true, true, PINNED), QUERY, None);
            for budget in [100_000, if format == "text" { 256 } else { 1000 }] {
                let mut request = args(mode, 2);
                if budget != 100_000 {
                    request["limit"] = json!(100);
                }
                request["include_superseded"] = json!(true);
                request["include_inactive"] = json!(true);
                request["max_tokens"] = json!(budget);
                let mut found = search(&harness, request, format).await;
                if budget != 100_000 {
                    assert!(found.cut);
                }
                let mut rows = Vec::new();
                for _ in 0..20 {
                    assert_eq!(found.continuation_exact, Some(false));
                    assert!(!found.rows.is_empty(), "authored budget admits a whole leading hit");
                    rows.extend(found.rows);
                    let Some(handle) = found.continuation else { break };
                    for name in ["include_superseded", "include_inactive"] {
                        for flag in [false, true] {
                            let mut request = json!({"continuation":handle});
                            request[name] = json!(flag);
                            let error = harness.call("search", request).await;
                            assert_eq!(error.is_error, Some(true));
                            assert!(text_of(&error).contains("bad_argument"));
                        }
                    }
                    found = search(
                        &harness,
                        json!({"continuation":handle,"max_tokens":budget}),
                        format,
                    )
                    .await;
                }
                assert_rows(&rows, &expected, format);
            }
        }
        harness.shutdown().await;
    }
    engine.close().unwrap();
}

#[tokio::test]
async fn inactive_historical_search_keeps_fetch_latest_eligible_resolution() {
    let (_fixture, engine) = inactive_history();
    for format in ["structured", "text"] {
        let harness = hosted(engine.clone(), format).await;
        let mut request = args("text", 100);
        request["include_superseded"] = json!(true);
        request["include_inactive"] = json!(true);
        let found = search(&harness, request, format).await;
        assert_eq!(found.rows.iter().filter(|row| row.reference == "l:version").count(), 3);
        for (superseded, inactive, expected) in [
            (false, false, None),
            (true, false, Some(body("current edition", "other"))),
            (false, true, Some(body("pending edition", "wanted"))),
            (true, true, Some(body("pending edition", "wanted"))),
        ] {
            let result=harness.call("fetch",json!({"refs":["l:version"],"include_superseded":superseded,"include_inactive":inactive,"as_of":PINNED,"max_tokens":100_000})).await;
            assert_ne!(result.is_error, Some(true));
            let text = text_of(&result);
            if format == "structured" {
                let output = result.structured_content.unwrap();
                if let Some(expected) = expected {
                    assert_eq!(output["items"][0]["record"]["body"], expected);
                } else {
                    assert_eq!(output["items"][0]["error"]["code"], "not_found");
                }
            } else {
                assert!(result.structured_content.is_none());
                if let Some(expected) = expected {
                    let (_, record) = text.split_once("\nrecord ").unwrap();
                    let (header, framed) = record.split_once('\n').unwrap();
                    let bytes = header
                        .split_whitespace()
                        .find_map(|word| word.strip_prefix("bytes="))
                        .unwrap()
                        .parse::<usize>()
                        .unwrap();
                    assert_eq!(bytes, expected.len());
                    assert_eq!(&framed.as_bytes()[..bytes], expected.as_bytes());
                    assert_eq!(&framed.as_bytes()[bytes..], b"\n");
                } else {
                    assert!(text.contains("error \"l:version\" not_found/not_found"));
                }
            }
        }
        harness.shutdown().await;
    }
    engine.close().unwrap();
}
