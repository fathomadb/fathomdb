#![cfg(feature = "test-hooks")]

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    take_slice71_search_statement_trace_for_test, Engine, InitialState, PreparedWrite, SourceId,
};
use fathomdb_schema::SQLITE_SUFFIX;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

static ENV_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug)]
struct FixedEmbedder;

impl Embedder for FixedEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice71", "exact-hybrid", 8)
    }

    fn embed(&self, _text: &str) -> Result<Vector, EmbedderError> {
        Ok(vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0])
    }
}

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

#[test]
fn node_fts_search_has_no_per_hit_post_filter_statements() {
    let dir = TempDir::new().expect("tempdir");
    let opened = Engine::open(dir.path().join(format!("trace{SQLITE_SUFFIX}"))).expect("open");
    let writes = (0..24)
        .map(|index| PreparedWrite::Node {
            kind: "doc".into(),
            body: format!("slice71needle document {index}"),
            source_id: SourceId::new("slice71").expect("source id"),
            logical_id: Some(format!("slice71-{index}")),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        })
        .collect::<Vec<_>>();
    opened.engine.write(&writes).expect("seed");

    take_slice71_search_statement_trace_for_test();
    let result = opened.engine.search_text_only_with_limit("slice71needle", 24).expect("search");
    assert_eq!(result.results.len(), 24);
    let trace = take_slice71_search_statement_trace_for_test();
    assert_eq!(
        trace.iter().filter(|event| event.as_str() == "post_filter_source_lookup").count(),
        0,
        "eligibility must be enforced in ranked SQL, not by a statement per returned hit: {trace:?}"
    );
}

#[test]
fn hybrid_deferred_identity_matches_complete_ranked_control_at_limit_100() {
    let _lock = ENV_LOCK.lock().expect("environment lock");
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join(format!("hybrid{SQLITE_SUFFIX}"));
    let route_witness = dir.path().join("routes.jsonl");
    let _env = EnvGuard::update(&[
        ("FATHOMDB_FTS_ROUTE_WITNESS_FOR_TEST", Some(route_witness.display().to_string())),
        ("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST", None),
    ]);

    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(FixedEmbedder)).expect("open");
    opened.engine.configure_vector_kind_for_test("doc").expect("vector kind");
    let mut writes = (1..=260)
        .map(|rank| PreparedWrite::Node {
            kind: "doc".into(),
            body: format!("{} document-{rank}", "slice71hybrid ".repeat(rank)),
            source_id: SourceId::new("slice71").expect("source id"),
            logical_id: (rank != 259).then(|| format!("slice71-{rank}")),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        })
        .collect::<Vec<_>>();
    writes.push(PreparedWrite::Node {
        kind: "doc".into(),
        body: "slice71hybrid duplicate body".into(),
        source_id: SourceId::new("slice71").expect("source id"),
        logical_id: Some("slice71-duplicate-a".into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    });
    writes.push(PreparedWrite::Node {
        kind: "doc".into(),
        body: "slice71hybrid duplicate body".into(),
        source_id: SourceId::new("slice71").expect("source id"),
        logical_id: Some("slice71-duplicate-b".into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    });
    opened.engine.write(&writes).expect("seed");
    opened.engine.drain(10_000).expect("drain");

    let optimized = opened.engine.search_with_limit("slice71hybrid", 100).expect("optimized");
    unsafe { std::env::set_var("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST", "1") };
    let control = opened.engine.search_with_limit("slice71hybrid", 100).expect("control");
    assert_eq!(optimized, control, "optimized hybrid search must preserve complete-ranking output");

    let routes = std::fs::read_to_string(route_witness).expect("route witness");
    assert!(routes.contains("hybrid_deferred_identity"), "optimized route was not selected");
    assert!(routes.contains("hybrid_full_sort_forced"), "control route was not selected");
}
