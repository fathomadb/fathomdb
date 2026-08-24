//! Slice 20 exact-result contracts for the production FTS rank stream.

use fathomdb_embedder::NoopEmbedder;
use fathomdb_engine::{EmbedderChoice, Engine, InitialState, PreparedWrite, SourceId};
use fathomdb_schema::SQLITE_SUFFIX;
use rusqlite::params;
use serde_json::Value;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

static ENV_LOCK: Mutex<()> = Mutex::new(());

struct EnvGuard(Vec<(&'static str, Option<String>)>);

impl EnvGuard {
    fn update(values: &[(&'static str, Option<String>)]) -> Self {
        let prior = values
            .iter()
            .map(|(key, value)| {
                let prior = std::env::var(key).ok();
                match value {
                    Some(value) => unsafe { std::env::set_var(key, value) },
                    None => unsafe { std::env::remove_var(key) },
                }
                (*key, prior)
            })
            .collect();
        Self(prior)
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, value) in &self.0 {
            match value {
                Some(value) => unsafe { std::env::set_var(key, value) },
                None => unsafe { std::env::remove_var(key) },
            }
        }
    }
}

fn db_path(dir: &TempDir, name: &str) -> std::path::PathBuf {
    dir.path().join(format!("{name}{SQLITE_SUFFIX}"))
}

fn open(dir: &TempDir, name: &str) -> fathomdb_engine::OpenedEngine {
    Engine::open_with_choice(
        db_path(dir, name),
        EmbedderChoice::Caller(Arc::new(NoopEmbedder::default())),
    )
    .expect("open")
}

fn node(logical_id: &str, body: String) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".to_string(),
        body,
        source_id: SourceId::new("slice20:test").expect("source id"),
        logical_id: Some(logical_id.to_string()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

fn jsonl(path: &std::path::Path) -> Vec<Value> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(|line| serde_json::from_str(line).expect("JSONL row"))
        .collect()
}

fn routes(path: &std::path::Path) -> Vec<String> {
    jsonl(path).into_iter().map(|row| row["route"].as_str().expect("route").to_string()).collect()
}

#[test]
fn production_stream_matches_full_sort_across_strict_tie_and_rank_override() {
    let _lock = ENV_LOCK.lock().expect("environment lock");
    let dir = TempDir::new().expect("tempdir");
    let route_witness = dir.path().join("routes.jsonl");
    let plan_witness = dir.path().join("query-plan.jsonl");
    let writer_witness = dir.path().join("writer.jsonl");
    let _env = EnvGuard::update(&[
        ("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST", None),
        ("FATHOMDB_FTS_FAIL_STREAM_FOR_TEST", None),
        ("FATHOMDB_FTS_ROUTE_WITNESS_FOR_TEST", Some(route_witness.display().to_string())),
        ("FATHOMDB_FTS_QUERY_PLAN_WITNESS_FOR_TEST", Some(plan_witness.display().to_string())),
        ("FATHOMDB_WRITER_PRAGMA_WITNESS_FOR_TEST", Some(writer_witness.display().to_string())),
    ]);

    let opened = open(&dir, "strict");
    let writes = (1..=140)
        .map(|rank| {
            node(
                &format!("strict-{rank}"),
                format!("{} strict-{rank}", "slice20strict ".repeat(rank)),
            )
        })
        .collect::<Vec<_>>();
    opened.engine.write(&writes).expect("write strict corpus");
    opened.engine.drain(10_000).expect("drain strict corpus");
    let streamed = opened
        .engine
        .search_text_only_with_limit("slice20strict", 100)
        .expect("stream strict boundary");
    opened.engine.close().expect("close strict stream");

    unsafe { std::env::set_var("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST", "1") };
    let control = open(&dir, "strict");
    let full = control
        .engine
        .search_text_only_with_limit("slice20strict", 100)
        .expect("full-sort strict boundary");
    control.engine.close().expect("close strict control");
    unsafe { std::env::remove_var("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST") };
    assert_eq!(streamed, full, "strict stream must equal the complete stable sort");

    let ties = open(&dir, "ties");
    let writes = (1..=140)
        .map(|rank| node(&format!("tie-{rank}"), "slice20tie identical".to_string()))
        .collect::<Vec<_>>();
    ties.engine.write(&writes).expect("write tie corpus");
    ties.engine.drain(10_000).expect("drain tie corpus");
    let streamed =
        ties.engine.search_text_only_with_limit("slice20tie", 100).expect("stream all-equal");
    ties.engine.close().expect("close tie stream");

    unsafe { std::env::set_var("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST", "1") };
    let control = open(&dir, "ties");
    let full =
        control.engine.search_text_only_with_limit("slice20tie", 100).expect("full-sort all-equal");
    control.engine.close().expect("close tie control");
    unsafe { std::env::remove_var("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST") };
    assert_eq!(streamed, full, "all-equal stream must retain the stable cursor prefix");

    let connection = rusqlite::Connection::open(db_path(&dir, "strict")).expect("open rank config");
    connection
        .execute("INSERT INTO search_index(search_index, rank) VALUES('rank', 'bm25(0.0)')", [])
        .expect("override persistent rank mapping");
    connection.close().expect("close rank config");
    let pinned = open(&dir, "strict");
    let pinned_result = pinned
        .engine
        .search_text_only_with_limit("slice20strict", 100)
        .expect("per-query bm25 pin");
    pinned.engine.close().expect("close pinned stream");
    assert_eq!(pinned_result, full_sort(&dir, "strict", "slice20strict"));

    let routes_before_failure = routes(&route_witness);
    unsafe { std::env::set_var("FATHOMDB_FTS_FAIL_STREAM_FOR_TEST", "1") };
    let failed_statement = open(&dir, "strict");
    let fallback = failed_statement
        .engine
        .search_text_only_with_limit("slice20strict", 100)
        .expect("failed stream falls back");
    failed_statement.engine.close().expect("close statement-failure fallback");
    unsafe { std::env::remove_var("FATHOMDB_FTS_FAIL_STREAM_FOR_TEST") };
    let routes_after_failure = routes(&route_witness);
    assert_eq!(
        routes_after_failure.len(),
        routes_before_failure.len() + 1,
        "the injected statement failure must record exactly one selected route"
    );
    assert_eq!(
        routes_after_failure.last().map(String::as_str),
        Some("full_sort_fallback"),
        "the injected statement failure must select the full-sort fallback"
    );
    assert_eq!(fallback, full_sort(&dir, "strict", "slice20strict"));

    let observed_routes = routes(&route_witness);
    assert!(observed_routes.contains(&"rank_stream_strict_boundary".to_string()));
    assert!(observed_routes.contains(&"rank_stream_tie_completed".to_string()));
    let observed_plans = jsonl(&plan_witness);
    assert!(!observed_plans.is_empty(), "eligible streamed calls must emit query-plan evidence");
    assert!(observed_plans.iter().all(|row| row["uses_temp_btree_for_order_by"] == false));
    assert!(jsonl(&writer_witness).iter().any(|row| {
        row["role"] == "writer" && row["journal_mode"] == "wal" && row["synchronous"] == 1
    }));
}

#[test]
fn eligible_stream_preserves_public_limit_prefixes_and_full_sort_equivalence() {
    let _lock = ENV_LOCK.lock().expect("environment lock");
    let dir = TempDir::new().expect("tempdir");
    let route_witness = dir.path().join("limit-routes.jsonl");
    let _env = EnvGuard::update(&[
        ("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST", None),
        ("FATHOMDB_FTS_FAIL_STREAM_FOR_TEST", None),
        ("FATHOMDB_FTS_ROUTE_WITNESS_FOR_TEST", Some(route_witness.display().to_string())),
        ("FATHOMDB_FTS_QUERY_PLAN_WITNESS_FOR_TEST", None),
        ("FATHOMDB_WRITER_PRAGMA_WITNESS_FOR_TEST", None),
    ]);

    let opened = open(&dir, "limits");
    let writes = (1..=140)
        .map(|rank| {
            node(&format!("limit-{rank}"), format!("{} limit-{rank}", "slice20limit ".repeat(rank)))
        })
        .collect::<Vec<_>>();
    opened.engine.write(&writes).expect("write eligible no-edge corpus");
    opened.engine.drain(10_000).expect("drain eligible no-edge corpus");

    let streamed_10 =
        opened.engine.search_text_only_with_limit("slice20limit", 10).expect("stream limit 10");
    let streamed_50 =
        opened.engine.search_text_only_with_limit("slice20limit", 50).expect("stream limit 50");
    let streamed_100 =
        opened.engine.search_text_only_with_limit("slice20limit", 100).expect("stream limit 100");
    opened.engine.close().expect("close streamed limits");

    let streamed_routes = routes(&route_witness);
    assert_eq!(streamed_routes.len(), 3, "each eligible public call must select one route");
    assert!(
        streamed_routes.iter().all(|route| route.starts_with("rank_stream_")),
        "the no-edge public limit calls must use the streamed route"
    );

    assert_eq!(streamed_10, full_sort_with_limit(&dir, "limits", "slice20limit", 10));
    assert_eq!(streamed_50, full_sort_with_limit(&dir, "limits", "slice20limit", 50));
    assert_eq!(streamed_100, full_sort_with_limit(&dir, "limits", "slice20limit", 100));
    assert_eq!(streamed_10.results, streamed_50.results[..10]);
    assert_eq!(streamed_50.results, streamed_100.results[..50]);
}

fn full_sort(dir: &TempDir, name: &str, query: &str) -> fathomdb_engine::SearchResult {
    full_sort_with_limit(dir, name, query, 100)
}

fn full_sort_with_limit(
    dir: &TempDir,
    name: &str,
    query: &str,
    limit: usize,
) -> fathomdb_engine::SearchResult {
    unsafe { std::env::set_var("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST", "1") };
    let opened = open(dir, name);
    let result =
        opened.engine.search_text_only_with_limit(query, limit).expect("full-sort control");
    opened.engine.close().expect("close full-sort control");
    unsafe { std::env::remove_var("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST") };
    result
}

#[test]
fn edges_are_ineligible_and_stream_row_errors_fall_back_without_partial_output() {
    let _lock = ENV_LOCK.lock().expect("environment lock");
    let dir = TempDir::new().expect("tempdir");
    let route_witness = dir.path().join("fallback-routes.jsonl");
    let _env = EnvGuard::update(&[
        ("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST", None),
        ("FATHOMDB_FTS_FAIL_STREAM_FOR_TEST", None),
        ("FATHOMDB_FTS_ROUTE_WITNESS_FOR_TEST", Some(route_witness.display().to_string())),
        ("FATHOMDB_FTS_QUERY_PLAN_WITNESS_FOR_TEST", None),
        ("FATHOMDB_WRITER_PRAGMA_WITNESS_FOR_TEST", None),
    ]);

    let edged = open(&dir, "edges");
    edged
        .engine
        .write(&[
            node("edge-a", "slice20edge first".to_string()),
            node("edge-b", "slice20edge second".to_string()),
            PreparedWrite::Edge {
                kind: "link".to_string(),
                from: "edge-a".to_string(),
                to: "edge-b".to_string(),
                source_id: SourceId::new("slice20:test").expect("source id"),
                logical_id: Some("edge-link".to_string()),
                body: Some("slice20edge relation".to_string()),
                t_valid: None,
                t_invalid: None,
                confidence: None,
                extractor_model_id: None,
                temporal_fallback: None,
            },
        ])
        .expect("write edge corpus");
    edged.engine.drain(10_000).expect("drain edge corpus");
    edged.engine.search_text_only("slice20edge").expect("edge-bearing exact path");
    edged.engine.close().expect("close edge engine");

    let valid = open(&dir, "fallback");
    valid
        .engine
        .write(&[
            node("fallback-a", "slice20fallback first".to_string()),
            node("fallback-b", "slice20fallback second".to_string()),
            node("fallback-c", "slice20fallback third".to_string()),
        ])
        .expect("write fallback corpus");
    valid.engine.drain(10_000).expect("drain fallback corpus");
    valid.engine.close().expect("close valid engine");
    let connection = rusqlite::Connection::open(db_path(&dir, "fallback")).expect("open FTS");
    connection
        .execute(
            "INSERT INTO search_index(body, kind, write_cursor) VALUES (?1, ?2, ?3)",
            params!["slice20fallback malformed", "doc", "not-an-integer"],
        )
        .expect("insert malformed projection row");
    connection.close().expect("close FTS");

    let reopened = open(&dir, "fallback");
    let result = reopened.engine.search_text_only("slice20fallback").expect("fallback search");
    reopened.engine.close().expect("close fallback engine");
    assert_eq!(result.results.len(), 3, "partial stream output must be discarded");
    let observed = routes(&route_witness);
    assert!(observed.contains(&"full_sort_ineligible".to_string()));
    assert!(observed.contains(&"full_sort_fallback".to_string()));
}
