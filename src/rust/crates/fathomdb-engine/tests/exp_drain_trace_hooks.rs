//! Experiment hooks for the drain investigation
//! (`dev/experiments/ac013-seed-drain/`).
//!
//! `FATHOMDB_PERF_PAGE_TRACE=all` must trace every connection role, not only
//! the readers: the writer, the projection dispatcher, the projection workers
//! and runtime probes, keyed by role. The drain phase counters
//! (`drain_phase_trace_for_test`) must time the dispatcher scan, the worker
//! embed, commit-gate wait and commit, and `drain`'s pending-work probe. The
//! tracer aggregate, the counters and the env are process-global, so this
//! binary holds one test per concern and serializes them on `LOCK`.

#![cfg(feature = "test-hooks")]

use std::sync::{Arc, Mutex};

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    drain_phase_trace_for_test, reset_drain_phase_trace_for_test, take_page_trace_for_test, Engine,
    InitialState, PreparedWrite, SourceId,
};
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

#[test]
fn drain_phase_trace_times_scan_embed_commit_and_drain_probe() {
    let _guard = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    reset_drain_phase_trace_for_test();
    let rows = 7;
    seed_and_drain("phase_trace", rows, false);
    let trace = drain_phase_trace_for_test();
    assert_eq!(trace.worker_jobs, rows as u64, "{trace:?}");
    assert_eq!(trace.dispatcher_jobs, rows as u64, "{trace:?}");
    assert!(trace.worker_batches >= 1 && trace.dispatcher_scans >= 1, "{trace:?}");
    assert!(trace.worker_embed_ns > 0 && trace.worker_commit_ns > 0, "{trace:?}");
    assert!(trace.dispatcher_scan_ns > 0, "{trace:?}");
    assert!(trace.drain_probes >= 1, "drain probed pending work at least once: {trace:?}");
    assert!(trace.drain_probe_pending < trace.drain_probes, "the last probe saw none: {trace:?}");
    assert!(trace.drain_probe_ns > 0, "{trace:?}");
    reset_drain_phase_trace_for_test();
    assert_eq!(drain_phase_trace_for_test().worker_jobs, 0, "reset clears the counters");
}

#[test]
fn projection_scan_query_plans_cover_scheduler_and_candidate_page() {
    let _guard = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join(format!("plans{SQLITE_SUFFIX}"));
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(FixedEmbedder)).expect("open");
    let plans = opened.engine.projection_scan_query_plans_for_test().expect("plans");
    let names: Vec<&str> = plans.iter().map(|(name, _, _)| name.as_str()).collect();
    assert_eq!(names, ["scheduler_scan", "candidate_page"], "{plans:#?}");
    for (name, sql, plan) in &plans {
        assert!(sql.contains("ORDER BY pending.write_cursor"), "{name}: {sql}");
        assert!(plan.iter().any(|line| line.contains("canonical_nodes")), "{name}: {plan:#?}");
    }
    opened.engine.close().expect("close");
}

/// Scheduler-scan page requests per run, from the dispatcher's trace rows.
fn scheduler_scan_requests_per_run(rows: &[fathomdb_engine::PageTraceRow]) -> f64 {
    let scans: Vec<_> = rows
        .iter()
        .filter(|row| {
            row.role == "projection_dispatcher"
                && row.sql.starts_with("SELECT pending.write_cursor, pending.kind, pending.body")
        })
        .collect();
    let runs: u64 = scans.iter().map(|row| row.runs).sum();
    let requests: i64 = scans.iter().map(|row| row.exclusive_hit + row.exclusive_miss).sum();
    assert!(runs > 0, "the dispatcher ran its scan: {rows:#?}");
    requests as f64 / runs as f64
}

fn held_seed_and_drain_traced(name: &str, rows: usize) -> Vec<fathomdb_engine::PageTraceRow> {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join(format!("{name}{SQLITE_SUFFIX}"));
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(FixedEmbedder)).expect("open");
    let engine = &opened.engine;
    engine.configure_vector_kind_for_test("doc").expect("vector kind");
    engine.set_projection_scheduler_frozen_for_test(true);
    let batch: Vec<PreparedWrite> = (0..rows)
        .map(|i| PreparedWrite::Node {
            kind: "doc".to_string(),
            body: format!("armlimit body {i} {}", "padding ".repeat(40)),
            source_id: SourceId::new("test:fixture").expect("source"),
            logical_id: None,
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        })
        .collect();
    engine.write(&batch).expect("write");
    let _ = take_page_trace_for_test();
    engine.set_projection_scheduler_frozen_for_test(false);
    engine.drain(60_000).expect("drain");
    let trace = take_page_trace_for_test();
    assert_eq!(engine.vector_row_count_for_test().expect("vector rows"), rows as u64);
    opened.engine.close().expect("close");
    trace
}

#[test]
fn drain_scan_armlimit_bounds_scheduler_scan_reads_by_page_not_backlog() {
    let _guard = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    std::env::set_var("FATHOMDB_PERF_PAGE_TRACE", "dispatcher");
    std::env::set_var("FATHOMDB_PERF_EXPERIMENTS", "1");
    std::env::set_var("FATHOMDB_PERF_DRAIN_SCAN", "armlimit");
    let trace = held_seed_and_drain_traced("armlimit", 4_000);
    std::env::remove_var("FATHOMDB_PERF_DRAIN_SCAN");
    std::env::remove_var("FATHOMDB_PERF_EXPERIMENTS");
    std::env::remove_var("FATHOMDB_PERF_PAGE_TRACE");
    // Without the knob every scan reads the whole pending backlog (about
    // 2,000 rows on average here); with it a scan reads one 256-row page per
    // arm plus the rows it skips.
    let per_run = scheduler_scan_requests_per_run(&trace);
    assert!(
        per_run < 800.0,
        "scheduler scan requests per run {per_run:.0} must not track the backlog"
    );
}
