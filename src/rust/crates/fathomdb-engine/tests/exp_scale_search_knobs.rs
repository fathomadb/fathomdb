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

#[test]
fn fts_rows_take_the_write_cursor_as_rowid() {
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    knobs_on();
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join(format!("fts_rowid{SQLITE_SUFFIX}"));
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(FixedEmbedder)).expect("open");
    let engine = &opened.engine;
    // A body-less edge takes a cursor and writes no body-FTS row, so with
    // auto-assigned rowids the node rows would be off by one.
    engine
        .write(&[PreparedWrite::Edge {
            kind: "links".to_string(),
            from: "a".to_string(),
            to: "b".to_string(),
            source_id: SourceId::new("test:fixture").expect("source"),
            logical_id: None,
            body: None,
            t_valid: None,
            t_invalid: None,
            confidence: None,
            extractor_model_id: None,
            temporal_fallback: None,
        }])
        .expect("edge");
    let batch: Vec<PreparedWrite> = (0..5).map(|i| node(format!("numbat {i}"), None)).collect();
    engine.write(&batch).expect("nodes");
    engine.drain(60_000).expect("drain");
    opened.engine.close().expect("close");
    let connection = rusqlite::Connection::open(&path).expect("reopen");
    let rows: Vec<(i64, i64)> = connection
        .prepare("SELECT rowid, write_cursor FROM search_index ORDER BY rowid")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap();
    assert_eq!(rows.len(), 5);
    for (rowid, cursor) in rows {
        assert_eq!(rowid, cursor, "FATHOMDB_PERF_FTS_ROWID_CURSOR keys FTS rows by cursor");
    }
}

/// 260 documents whose bm25 falls with their repeat count, two duplicate
/// bodies, limit 100: the Slice 71 deep-overlap fixture. Returns the path.
fn seed_deep_overlap(dir: &TempDir, name: &str) -> std::path::PathBuf {
    let path = dir.path().join(format!("{name}{SQLITE_SUFFIX}"));
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(FixedEmbedder)).expect("open");
    opened.engine.configure_vector_kind_for_test("doc").expect("vector kind");
    let mut writes: Vec<PreparedWrite> = (1..=260)
        .map(|rank| {
            node(
                format!("{} document-{rank}", "deferredhybrid ".repeat(rank)),
                (rank != 259).then(|| format!("deferred-{rank}")),
            )
        })
        .collect();
    writes.push(node("deferredhybrid duplicate body".into(), Some("dup-a".into())));
    writes.push(node("deferredhybrid duplicate body".into(), Some("dup-b".into())));
    opened.engine.write(&writes).expect("seed");
    opened.engine.drain(60_000).expect("drain");
    opened.engine.close().expect("close");
    path
}

fn routes_of(witness: &std::path::Path) -> String {
    std::fs::read_to_string(witness).unwrap_or_default()
}

#[test]
fn deferred_content_text_arm_matches_the_complete_ranking() {
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    knobs_on();
    let dir = TempDir::new().expect("tempdir");
    let path = seed_deep_overlap(&dir, "deferred_content");
    let witness = dir.path().join("routes.jsonl");
    std::env::set_var("FATHOMDB_FTS_ROUTE_WITNESS_FOR_TEST", &witness);
    std::env::remove_var("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST");
    let opened =
        Engine::open_with_embedder_for_test(&path, Arc::new(FixedEmbedder)).expect("reopen");
    let mut optimized = Vec::new();
    for (query, limit) in
        [("deferredhybrid", 100), ("deferredhybrid duplicate", 10), ("document", 5)]
    {
        optimized.push(opened.engine.search_with_limit(query, limit).expect("optimized"));
    }
    let optimized_routes = routes_of(&witness);
    std::env::set_var("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST", "1");
    let mut control = Vec::new();
    for (query, limit) in
        [("deferredhybrid", 100), ("deferredhybrid duplicate", 10), ("document", 5)]
    {
        control.push(opened.engine.search_with_limit(query, limit).expect("control"));
    }
    std::env::remove_var("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST");
    std::env::remove_var("FATHOMDB_FTS_ROUTE_WITNESS_FOR_TEST");
    opened.engine.close().expect("close");
    assert!(optimized[0].results.iter().any(|h| h.branch == SoftFallbackBranch::Vector));
    assert_eq!(optimized, control, "deferred content must preserve the complete ranking");
    assert_eq!(
        optimized_routes.matches("hybrid_deferred_content\"").count(),
        3,
        "every optimized search takes the deferred-content route: {optimized_routes}"
    );
}

#[test]
fn deferred_content_falls_back_when_an_fts_rowid_is_not_its_cursor() {
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    knobs_on();
    let dir = TempDir::new().expect("tempdir");
    let path = seed_deep_overlap(&dir, "deferred_fallback");
    let ownerless = format!("{} ownerless", "deferredhybrid ".repeat(300));
    let connection = rusqlite::Connection::open(&path).expect("open for injection");
    connection
        .execute(
            "INSERT INTO search_index(body, kind, write_cursor) VALUES (?1, 'doc', 999999)",
            [&ownerless],
        )
        .expect("ownerless row with an auto rowid");
    drop(connection);
    let witness = dir.path().join("routes.jsonl");
    std::env::set_var("FATHOMDB_FTS_ROUTE_WITNESS_FOR_TEST", &witness);
    std::env::remove_var("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST");
    let opened =
        Engine::open_with_embedder_for_test(&path, Arc::new(FixedEmbedder)).expect("reopen");
    let optimized = opened.engine.search_with_limit("deferredhybrid", 100).expect("optimized");
    let optimized_routes = routes_of(&witness);
    std::env::set_var("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST", "1");
    let control = opened.engine.search_with_limit("deferredhybrid", 100).expect("control");
    std::env::remove_var("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST");
    std::env::remove_var("FATHOMDB_FTS_ROUTE_WITNESS_FOR_TEST");
    opened.engine.close().expect("close");
    assert!(optimized.results.iter().any(|h| h.body == ownerless), "ownerless row ranks");
    assert_eq!(optimized, control);
    assert!(
        optimized_routes.contains("hybrid_deferred_content_fallback"),
        "a row whose rowid is not its cursor forces the complete path: {optimized_routes}"
    );
}
