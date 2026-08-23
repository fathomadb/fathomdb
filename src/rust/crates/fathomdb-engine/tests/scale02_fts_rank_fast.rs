//! SCALE-02's experiment-only FTS rank fast-path equivalence controls.

use fathomdb_embedder::NoopEmbedder;
use fathomdb_engine::{EmbedderChoice, Engine, InitialState, PreparedWrite, SourceId};
use fathomdb_schema::SQLITE_SUFFIX;
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

fn open(dir: &TempDir, name: &str) -> fathomdb_engine::OpenedEngine {
    Engine::open_with_choice(
        dir.path().join(format!("{name}{SQLITE_SUFFIX}")),
        EmbedderChoice::Caller(Arc::new(NoopEmbedder::default())),
    )
    .expect("open")
}

fn node(logical_id: &str, body: String) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".to_string(),
        body,
        source_id: SourceId::new("scale-02:test").expect("source"),
        logical_id: Some(logical_id.to_string()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

fn routes(path: &std::path::Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(|line| {
            serde_json::from_str::<Value>(line).expect("JSON witness")["route"]
                .as_str()
                .expect("route string")
                .to_string()
        })
        .collect()
}

fn boundary_witnesses(path: &std::path::Path) -> Vec<Value> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("JSON boundary witness"))
        .collect()
}

#[test]
fn rank_fast_is_the_production_direct_text_path() {
    let _lock = ENV_LOCK.lock().expect("environment lock");
    let dir = TempDir::new().expect("tempdir");
    let witness = dir.path().join("production-routes.jsonl");
    let _env = EnvGuard::update(&[
        ("FATHOMDB_PERF_EXPERIMENTS", None),
        ("FATHOMDB_PERF_FTS_RANK_FAST", None),
        ("FATHOMDB_PERF_FTS_FORCE_FULL_SORT", None),
        ("FATHOMDB_PERF_FTS_ROUTE_WITNESS", Some(witness.display().to_string())),
    ]);
    let opened = open(&dir, "production");
    opened
        .engine
        .write(&[
            node("production-a", "scale02production first".to_string()),
            node("production-b", "scale02production second".to_string()),
        ])
        .expect("write production corpus");
    opened.engine.drain(10_000).expect("drain production corpus");
    opened.engine.search_text_only("scale02production").expect("production search");
    opened.engine.close().expect("close production engine");

    assert_eq!(routes(&witness), ["rank_fast"]);
}

#[test]
fn rank_fast_matches_full_sort_and_falls_back_for_ties_and_edges() {
    let _lock = ENV_LOCK.lock().expect("environment lock");
    let dir = TempDir::new().expect("tempdir");
    let witness = dir.path().join("routes.jsonl");
    let _env = EnvGuard::update(&[
        ("FATHOMDB_PERF_EXPERIMENTS", Some("1".to_string())),
        ("FATHOMDB_PERF_FTS_RANK_FAST", Some("1".to_string())),
        ("FATHOMDB_PERF_FTS_FORCE_FULL_SORT", None),
        ("FATHOMDB_PERF_FTS_ROUTE_WITNESS", Some(witness.display().to_string())),
    ]);

    let opened = open(&dir, "strict-boundary");
    let writes = (1..=20)
        .map(|rank| {
            node(&format!("node-{rank}"), format!("{} filler words", "scale02fast ".repeat(rank)))
        })
        .collect::<Vec<_>>();
    opened.engine.write(&writes).expect("write nodes");
    opened.engine.drain(10_000).expect("drain");
    let fast =
        opened.engine.search_text_only_with_limit("scale02fast", 10).expect("rank-fast search");
    opened.engine.close().expect("close fast engine");
    assert!(routes(&witness).contains(&"rank_fast".to_string()));

    unsafe { std::env::set_var("FATHOMDB_PERF_FTS_FORCE_FULL_SORT", "1") };
    let baseline = open(&dir, "strict-boundary");
    let full =
        baseline.engine.search_text_only_with_limit("scale02fast", 10).expect("full-sort search");
    assert_eq!(fast.results, full.results);
    baseline.engine.close().expect("close baseline engine");
    unsafe { std::env::remove_var("FATHOMDB_PERF_FTS_FORCE_FULL_SORT") };

    let overfetch = open(&dir, "overfetch-dedup");
    let mut writes = (1..=10)
        .map(|rank| {
            node(
                &format!("unique-{rank}"),
                format!("{} unique-{rank}", "scale02dedup ".repeat(30 - rank)),
            )
        })
        .collect::<Vec<_>>();
    writes.extend(
        (1..=20).map(|rank| node(&format!("duplicate-{rank}"), "scale02dedup weak".to_string())),
    );
    overfetch.engine.write(&writes).expect("write overfetch corpus");
    overfetch.engine.drain(10_000).expect("drain overfetch corpus");
    let fast = overfetch
        .engine
        .search_text_only_with_limit("scale02dedup", 10)
        .expect("rank-fast overfetch search");
    overfetch.engine.close().expect("close overfetch engine");

    unsafe { std::env::set_var("FATHOMDB_PERF_FTS_FORCE_FULL_SORT", "1") };
    let baseline = open(&dir, "overfetch-dedup");
    let full = baseline
        .engine
        .search_text_only_with_limit("scale02dedup", 10)
        .expect("full-sort overfetch search");
    assert_eq!(fast.results, full.results);
    baseline.engine.close().expect("close overfetch baseline");
    unsafe { std::env::remove_var("FATHOMDB_PERF_FTS_FORCE_FULL_SORT") };

    let tied = open(&dir, "tie");
    let tied_writes = (1..=101)
        .map(|rank| node(&format!("tie-{rank}"), "scale02tie identical".to_string()))
        .collect::<Vec<_>>();
    tied.engine.write(&tied_writes).expect("write ties");
    tied.engine.drain(10_000).expect("drain ties");
    tied.engine.search_text_only_with_limit("scale02tie", 1).expect("tie search");
    tied.engine.close().expect("close tie engine");
    assert!(routes(&witness).contains(&"full_sort_fallback".to_string()));

    let edged = open(&dir, "edge");
    edged
        .engine
        .write(&[
            node("edge-a", "scale02edge first".to_string()),
            node("edge-b", "scale02edge second".to_string()),
            PreparedWrite::Edge {
                kind: "link".to_string(),
                from: "edge-a".to_string(),
                to: "edge-b".to_string(),
                source_id: SourceId::new("scale-02:test").expect("source"),
                logical_id: Some("edge-link".to_string()),
                body: Some("scale02edge relation".to_string()),
                t_valid: None,
                t_invalid: None,
                confidence: None,
                extractor_model_id: None,
                temporal_fallback: None,
            },
        ])
        .expect("write edge corpus");
    edged.engine.drain(10_000).expect("drain edge corpus");
    edged.engine.search_text_only("scale02edge").expect("edge search");
    edged.engine.close().expect("close edge engine");
    assert!(routes(&witness).contains(&"full_sort_ineligible".to_string()));
}

#[test]
fn rank_stream_completes_the_boundary_tie_without_changing_results() {
    let _lock = ENV_LOCK.lock().expect("environment lock");
    let dir = TempDir::new().expect("tempdir");
    let route_witness = dir.path().join("stream-routes.jsonl");
    let boundary_witness = dir.path().join("stream-boundary.jsonl");
    let connection_witness = dir.path().join("stream-connections.jsonl");
    let query_plan_witness = dir.path().join("stream-query-plan.jsonl");
    let _env = EnvGuard::update(&[
        ("FATHOMDB_PERF_EXPERIMENTS", Some("1".to_string())),
        ("FATHOMDB_PERF_FTS_FORCE_FULL_SORT", None),
        ("FATHOMDB_PERF_FTS_STREAM_TIES", Some("1".to_string())),
        ("FATHOMDB_PERF_FTS_ROUTE_WITNESS", Some(route_witness.display().to_string())),
        ("FATHOMDB_PERF_FTS_BOUNDARY_WITNESS", Some(boundary_witness.display().to_string())),
        ("FATHOMDB_PERF_FTS_QUERY_PLAN_WITNESS", Some(query_plan_witness.display().to_string())),
        ("FATHOMDB_PERF_CONNECTION_PRAGMA_WITNESS", Some(connection_witness.display().to_string())),
    ]);

    let opened = open(&dir, "stream-ties");
    let tied_writes = (1..=101)
        .map(|rank| node(&format!("stream-tie-{rank}"), "scale02stream identical".to_string()))
        .collect::<Vec<_>>();
    opened.engine.write(&tied_writes).expect("write ties");
    opened.engine.drain(10_000).expect("drain ties");
    let streamed = opened
        .engine
        .search_text_only_with_limit("scale02stream", 10)
        .expect("streamed tie search");
    opened.engine.close().expect("close streamed engine");

    unsafe { std::env::set_var("FATHOMDB_PERF_FTS_FORCE_FULL_SORT", "1") };
    let baseline = open(&dir, "stream-ties");
    let full = baseline
        .engine
        .search_text_only_with_limit("scale02stream", 10)
        .expect("full-sort tie search");
    baseline.engine.close().expect("close baseline engine");

    assert_eq!(streamed.results, full.results);
    assert_eq!(routes(&route_witness), ["rank_stream_tie_completed"]);
    assert_eq!(
        boundary_witnesses(&boundary_witness),
        [serde_json::json!({
            "schema_version": "scale-02-fts-boundary.v1",
            "route": "rank_stream_tie_completed",
            "candidate_limit": 100,
            "rows_consumed": 101,
            "boundary_group_size": 101,
        })]
    );
    assert_eq!(
        boundary_witnesses(&query_plan_witness),
        [serde_json::json!({
            "schema_version": "scale-02-fts-query-plan.v1",
            "statement": "stream_complete_boundary_tie",
            "uses_temp_btree_for_order_by": false,
        })]
    );
    let connections = boundary_witnesses(&connection_witness);
    assert!(connections.iter().any(|row| {
        row["schema_version"] == "scale-02-connection-settings.v1"
            && row["role"] == "writer"
            && row["journal_mode"] == "wal"
            && row["synchronous"] == 1
    }));
    assert!(connections.iter().any(|row| {
        row["schema_version"] == "scale-02-connection-settings.v1"
            && row["role"] == "reader"
            && row["journal_mode"] == "wal"
    }));
}
