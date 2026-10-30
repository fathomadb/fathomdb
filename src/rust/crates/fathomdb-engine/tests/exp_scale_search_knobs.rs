//! Scale-search experiment knobs (`dev/experiments/ac013-scale-search/`,
//! `src/scale_search_experiment.rs`).
//!
//! The knobs are read once per process, so this binary turns them all on
//! before any engine opens. Tests share one lock because some of them toggle
//! the per-search `FATHOMDB_FTS_*_FOR_TEST` controls.

#![cfg(feature = "test-hooks")]

use std::sync::{Arc, Mutex, Once};

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    eligibility_cache_stats_for_test, Engine, InitialState, PreparedWrite, SoftFallbackBranch,
    SourceId,
};
use fathomdb_schema::SQLITE_SUFFIX;
use tempfile::TempDir;

static LOCK: Mutex<()> = Mutex::new(());

fn knobs_on() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        std::env::set_var("FATHOMDB_PERF_EXPERIMENTS", "1");
        std::env::set_var("FATHOMDB_PERF_ELIGIBILITY", "cache");
        std::env::set_var("FATHOMDB_PERF_FTS_ROWID_CURSOR", "1");
        std::env::set_var("FATHOMDB_PERF_TEXT_ARM", "deferred");
    });
}

#[derive(Debug)]
struct FixedEmbedder;

impl Embedder for FixedEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("fixed", "scale-search-knobs", 8)
    }

    fn embed(&self, text: &str) -> Result<Vector, EmbedderError> {
        let seed = text.len() as f32;
        Ok((0..8).map(|i| (seed + i as f32).sin()).collect())
    }
}

fn node(body: String, logical_id: Option<String>) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".to_string(),
        body,
        source_id: SourceId::new("test:fixture").expect("source"),
        logical_id,
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

#[test]
fn eligibility_cache_hits_on_an_unchanged_snapshot_and_misses_after_a_write() {
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    knobs_on();
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join(format!("elig_cache{SQLITE_SUFFIX}"));
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(FixedEmbedder)).expect("open");
    let engine = &opened.engine;
    engine.configure_vector_kind_for_test("doc").expect("vector kind");
    let batch: Vec<PreparedWrite> =
        (0..40).map(|i| node(format!("wombat body {i}"), Some(format!("w-{i}")))).collect();
    engine.write(&batch).expect("write");
    engine.drain(60_000).expect("drain");

    let first = engine.search("wombat").expect("first search");
    assert_eq!(first.soft_fallback, None, "a clean store runs the vector arm");
    let (hits0, misses0) = eligibility_cache_stats_for_test();
    let second = engine.search("wombat body").expect("second search");
    let (hits1, misses1) = eligibility_cache_stats_for_test();
    assert_eq!(second.soft_fallback, None);
    assert_eq!(misses1, misses0, "an unchanged snapshot must not rescan");
    assert_eq!(hits1, hits0 + 2, "both direct-eligibility answers come from the cache");

    // A supersession leaves the old version's vector row behind; the vector
    // arm must now be declined, so a stale cached "eligible" would be wrong.
    engine.write(&[node("wombat body 0 revised".to_string(), Some("w-0".to_string()))]).unwrap();
    engine.drain(60_000).expect("drain");
    let after = engine.search("wombat").expect("search after supersession");
    let (_, misses2) = eligibility_cache_stats_for_test();
    assert!(misses2 > misses1, "a new generation must miss");
    assert_eq!(
        after.soft_fallback.map(|f| f.branch),
        Some(SoftFallbackBranch::Vector),
        "the recomputed answer declines the vector arm"
    );
    opened.engine.close().expect("close");
}
