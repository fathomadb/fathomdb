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
    fn set(values: &[(&'static str, String)]) -> Self {
        let prior = values
            .iter()
            .map(|(key, value)| {
                let prior = std::env::var(key).ok();
                unsafe { std::env::set_var(key, value) };
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
        .expect("route witness")
        .lines()
        .map(|line| {
            serde_json::from_str::<Value>(line).expect("JSON witness")["route"]
                .as_str()
                .expect("route string")
                .to_string()
        })
        .collect()
}

#[test]
fn rank_fast_matches_full_sort_and_falls_back_for_ties_and_edges() {
    let _lock = ENV_LOCK.lock().expect("environment lock");
    let dir = TempDir::new().expect("tempdir");
    let witness = dir.path().join("routes.jsonl");
    let _env = EnvGuard::set(&[
        ("FATHOMDB_PERF_EXPERIMENTS", "1".to_string()),
        ("FATHOMDB_PERF_FTS_RANK_FAST", "1".to_string()),
        ("FATHOMDB_PERF_FTS_ROUTE_WITNESS", witness.display().to_string()),
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

    unsafe { std::env::remove_var("FATHOMDB_PERF_FTS_RANK_FAST") };
    let baseline = open(&dir, "strict-boundary");
    let full =
        baseline.engine.search_text_only_with_limit("scale02fast", 10).expect("full-sort search");
    assert_eq!(fast.results, full.results);
    baseline.engine.close().expect("close baseline engine");
    unsafe { std::env::set_var("FATHOMDB_PERF_FTS_RANK_FAST", "1") };

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

    unsafe { std::env::remove_var("FATHOMDB_PERF_FTS_RANK_FAST") };
    let baseline = open(&dir, "overfetch-dedup");
    let full = baseline
        .engine
        .search_text_only_with_limit("scale02dedup", 10)
        .expect("full-sort overfetch search");
    assert_eq!(fast.results, full.results);
    baseline.engine.close().expect("close overfetch baseline");
    unsafe { std::env::set_var("FATHOMDB_PERF_FTS_RANK_FAST", "1") };

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
