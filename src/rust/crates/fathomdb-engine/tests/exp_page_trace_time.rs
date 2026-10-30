//! Per-statement wall time in the page tracer
//! (`dev/experiments/ac013-scale-search/`).
//!
//! `src/page_trace.rs` attributes page requests to SQL statements. The
//! scale-search experiment also needs each statement's elapsed time, taken
//! from the `SQLITE_TRACE_PROFILE` nanoseconds, inclusive and exclusive of
//! nested statements like the page counts. The env is process-global, so this
//! binary sets `FATHOMDB_PERF_PAGE_TRACE` once before any engine opens.

#![cfg(feature = "test-hooks")]

use std::sync::{Arc, Once};

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{take_page_trace_for_test, Engine, InitialState, PreparedWrite, SourceId};
use fathomdb_schema::SQLITE_SUFFIX;
use tempfile::TempDir;

fn enable_trace() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| std::env::set_var("FATHOMDB_PERF_PAGE_TRACE", "1"));
}

#[derive(Debug)]
struct FixedEmbedder;

impl Embedder for FixedEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("fixed", "page-trace-time", 8)
    }

    fn embed(&self, text: &str) -> Result<Vector, EmbedderError> {
        let seed = text.len() as f32;
        Ok((0..8).map(|i| (seed + i as f32).sin()).collect())
    }
}

#[test]
fn page_trace_records_statement_time_inclusive_and_exclusive() {
    enable_trace();
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join(format!("page_trace_time{SQLITE_SUFFIX}"));
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(FixedEmbedder)).expect("open");
    let engine = &opened.engine;
    engine.configure_vector_kind_for_test("doc").expect("vector kind");
    let batch: Vec<PreparedWrite> = (0..200)
        .map(|i| PreparedWrite::Node {
            kind: "doc".to_string(),
            body: format!("quokka trace body number {i} with shared words"),
            source_id: SourceId::new("test:fixture").expect("source"),
            logical_id: None,
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        })
        .collect();
    engine.write(&batch).expect("write");
    engine.drain(60_000).expect("drain");
    let _ = take_page_trace_for_test();
    for _ in 0..5 {
        engine.search("quokka shared words").expect("search");
    }
    let rows = take_page_trace_for_test();
    let fts = rows
        .iter()
        .find(|row| row.sql.contains("search_index MATCH") && row.nested_runs == 0)
        .expect("the top-level FTS statement is traced");
    assert!(fts.inclusive_ns > 0, "statement time must be recorded: {fts:?}");
    assert!(fts.exclusive_ns <= fts.inclusive_ns, "exclusive time excludes children: {fts:?}");
    let nested_ns: u64 =
        rows.iter().filter(|row| row.nested_runs > 0).map(|r| r.inclusive_ns).sum();
    assert!(nested_ns > 0, "FTS5's nested lookups carry their own time");
    let total_exclusive: u64 = rows.iter().map(|r| r.exclusive_ns).sum();
    let top_inclusive: u64 =
        rows.iter().filter(|row| row.nested_runs < row.runs).map(|r| r.inclusive_ns).sum();
    assert!(
        total_exclusive <= top_inclusive,
        "exclusive times partition the top-level time: {total_exclusive} > {top_inclusive}"
    );
    opened.engine.close().expect("close");
}
