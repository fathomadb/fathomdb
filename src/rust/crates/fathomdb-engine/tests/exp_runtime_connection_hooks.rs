//! Experiment hooks for the projection/runtime connections
//! (`dev/experiments/ac013-seed-connections/`).
//!
//! `FATHOMDB_PERF_RUNTIME_PRAGMAS` must reach every connection opened by
//! `open_runtime_connection` (projection dispatcher and workers). The env is
//! process-global, so this binary sets one value before any engine opens and
//! every test here is compatible with it.

#![cfg(feature = "test-hooks")]

use std::sync::{Arc, Once};

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{Engine, InitialState, PreparedWrite, SourceId};
use fathomdb_schema::SQLITE_SUFFIX;
use tempfile::TempDir;

const RUNTIME_CACHE_SIZE: i64 = -12_345;

fn set_runtime_pragmas() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        std::env::set_var("FATHOMDB_PERF_EXPERIMENTS", "1");
        std::env::set_var(
            "FATHOMDB_PERF_RUNTIME_PRAGMAS",
            format!("cache_size={RUNTIME_CACHE_SIZE}"),
        );
    });
}

#[derive(Debug)]
struct FixedEmbedder;

impl Embedder for FixedEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("fixed", "runtime-hooks", 8)
    }

    fn embed(&self, text: &str) -> Result<Vector, EmbedderError> {
        let seed = text.len() as f32;
        Ok((0..8).map(|i| (seed + i as f32).sin()).collect())
    }
}

#[test]
fn runtime_pragma_hook_applies_cache_size_to_runtime_connections() {
    set_runtime_pragmas();
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join(format!("runtime_pragmas{SQLITE_SUFFIX}"));
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(FixedEmbedder)).expect("open");
    assert_eq!(
        opened.engine.runtime_pragma_for_test("cache_size").expect("runtime cache_size"),
        RUNTIME_CACHE_SIZE,
        "open_runtime_connection must apply FATHOMDB_PERF_RUNTIME_PRAGMAS"
    );
    opened.engine.close().expect("close");
}

#[test]
fn runtime_cache_counters_cover_writer_dispatcher_and_workers() {
    set_runtime_pragmas();
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join(format!("runtime_counters{SQLITE_SUFFIX}"));
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(FixedEmbedder)).expect("open");
    let engine = &opened.engine;
    engine.configure_vector_kind_for_test("doc").expect("vector kind");
    let rows = 5;
    let batch: Vec<PreparedWrite> = (0..rows)
        .map(|i| PreparedWrite::Node {
            kind: "doc".to_string(),
            body: format!("runtime counter body {i}"),
            source_id: SourceId::new("test:fixture").expect("source"),
            logical_id: None,
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        })
        .collect();
    engine.write(&batch).expect("write");
    engine.drain(30_000).expect("drain");
    assert_eq!(engine.vector_row_count_for_test().expect("vector rows"), rows as u64);

    let (hit, miss, _) = engine.writer_cache_status_for_test().expect("writer status");
    assert!(hit + miss > 0, "writer counters must move after a write");

    let status = engine.runtime_cache_status_for_test();
    let dispatcher: Vec<_> = status.iter().filter(|s| s.role == "projection_dispatcher").collect();
    let workers: Vec<_> = status.iter().filter(|s| s.role == "projection_worker").collect();
    assert_eq!(dispatcher.len(), 1, "one dispatcher entry: {status:?}");
    assert!(dispatcher[0].rounds > 0 && dispatcher[0].cache_hit + dispatcher[0].cache_miss > 0);
    assert!(!workers.is_empty(), "at least one worker committed: {status:?}");
    assert_eq!(workers.iter().map(|w| w.jobs).sum::<u64>(), rows as u64, "{status:?}");
    for worker in &workers {
        assert!(worker.rounds > 0 && worker.cache_hit + worker.cache_miss > 0, "{worker:?}");
        assert!(worker.cache_used_bytes > 0, "{worker:?}");
    }
    opened.engine.close().expect("close");
}
