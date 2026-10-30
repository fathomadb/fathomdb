//! Experiment hooks for the drain investigation
//! (`dev/experiments/ac013-seed-drain/`).
//!
//! `FATHOMDB_PERF_PAGE_TRACE=all` must trace every connection role, not only
//! the readers: the writer, the projection dispatcher, the projection workers
//! and runtime probes, keyed by role. The tracer aggregate and the env are
//! process-global, so this binary holds one test per concern and serializes
//! them on `LOCK`.

#![cfg(feature = "test-hooks")]

use std::sync::{Arc, Mutex};

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{take_page_trace_for_test, Engine, InitialState, PreparedWrite, SourceId};
use fathomdb_schema::SQLITE_SUFFIX;
use tempfile::TempDir;

static LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug)]
struct FixedEmbedder;

impl Embedder for FixedEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("fixed", "drain-trace-hooks", 8)
    }

    fn embed(&self, text: &str) -> Result<Vector, EmbedderError> {
        let seed = text.len() as f32;
        Ok((0..8).map(|i| (seed + i as f32).sin()).collect())
    }
}

fn seed_and_drain(name: &str, rows: usize, search: bool) {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join(format!("{name}{SQLITE_SUFFIX}"));
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(FixedEmbedder)).expect("open");
    let engine = &opened.engine;
    engine.configure_vector_kind_for_test("doc").expect("vector kind");
    let batch: Vec<PreparedWrite> = (0..rows)
        .map(|i| PreparedWrite::Node {
            kind: "doc".to_string(),
            body: format!("drain trace body {i}"),
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
    if search {
        engine.search("drain trace body").expect("search");
    }
    opened.engine.close().expect("close");
}

#[test]
fn page_trace_all_covers_writer_dispatcher_and_workers_by_role() {
    let _guard = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    std::env::set_var("FATHOMDB_PERF_PAGE_TRACE", "all");
    let _ = take_page_trace_for_test();
    seed_and_drain("trace_all", 5, false);
    let rows = take_page_trace_for_test();
    std::env::remove_var("FATHOMDB_PERF_PAGE_TRACE");

    let described: Vec<String> = rows.iter().map(|row| format!("{row:?}")).collect();
    let has = |role: &str, sql: &str| {
        described.iter().any(|d| d.contains(&format!("role: \"{role}\"")) && d.contains(sql))
    };
    assert!(has("writer", "INSERT INTO canonical_nodes"), "writer insert traced: {described:#?}");
    assert!(
        has("projection_dispatcher", "SELECT pending.write_cursor"),
        "dispatcher scan traced: {described:#?}"
    );
    assert!(
        has("projection_worker", "INSERT INTO vector_default"),
        "worker vector insert traced: {described:#?}"
    );
    assert!(
        described.iter().any(|d| d.contains("role: \"projection_worker\"")
            && d.contains(" ns: ")
            && !d.contains(" ns: 0,")),
        "worker statements carry wall time: {described:#?}"
    );
}

#[test]
fn page_trace_one_keeps_reader_only_scope() {
    let _guard = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    std::env::set_var("FATHOMDB_PERF_PAGE_TRACE", "1");
    let _ = take_page_trace_for_test();
    seed_and_drain("trace_one", 3, true);
    let rows = take_page_trace_for_test();
    std::env::remove_var("FATHOMDB_PERF_PAGE_TRACE");
    let described: Vec<String> = rows.iter().map(|row| format!("{row:?}")).collect();
    assert!(!described.is_empty(), "the search must leave reader rows");
    assert!(
        described.iter().all(|d| d.contains("role: \"reader\"")),
        "FATHOMDB_PERF_PAGE_TRACE=1 traces readers only: {described:#?}"
    );
}
