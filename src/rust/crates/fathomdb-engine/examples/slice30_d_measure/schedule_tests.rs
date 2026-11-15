//! Literal real-store schedule controls; no scaled scientific execution.
use super::provider::VaryingEmbedder;
use super::schedule::{self, Mode, OrdinaryCell};
use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use rusqlite::{Connection, OpenFlags};
use serde_json::Value;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Default)]
struct CountingProvider {
    calls: Mutex<Vec<String>>,
}

impl Embedder for CountingProvider {
    fn identity(&self) -> EmbedderIdentity {
        VaryingEmbedder.identity()
    }

    fn embed(&self, text: &str) -> Result<Vector, EmbedderError> {
        self.calls.lock().unwrap().push(text.to_owned());
        VaryingEmbedder.embed(text)
    }
}

fn queries() -> Vec<String> {
    (0..500).map(|index| format!("requestneedle{index}")).collect()
}

fn assert_ordinary_calls(report: &Value, provider: &CountingProvider, queries: &[String]) {
    let actual = provider
        .calls
        .lock()
        .unwrap()
        .iter()
        .filter(|body| body.starts_with("requestneedle"))
        .cloned()
        .collect::<Vec<_>>();
    let expected = (0..1000).map(|index| queries[index % 500].clone()).collect::<Vec<_>>();
    assert_eq!(
        actual, expected,
        "actual hybrid provider calls omitted or reordered natural requests"
    );
    let calls = report["calls"].as_array().unwrap();
    assert_eq!(calls.len(), 1000, "ordinary driver added a warmup or omitted a call");
    let mut previous_end = 0;
    for (index, call) in calls.iter().enumerate() {
        assert_eq!(call["sample_index"], index);
        assert_eq!(call["query_index"], index % 500);
        assert_eq!(call["route"], "engine_hybrid");
        assert_eq!(call["limit"], 10);
        assert_eq!(call["ok"], true);
        let start = call["started_ns"].as_u64().unwrap();
        let end = call["ended_ns"].as_u64().unwrap();
        assert!(start >= previous_end && end > start, "natural API intervals are not sequential");
        assert!(call["returned_hits"].as_u64().unwrap() <= 10);
        previous_end = end;
    }
    assert!(report["closed_ns"].as_u64().unwrap() >= previous_end);
    assert_eq!(report["collection_enabled"], false);
    assert_eq!(report["periodic_numeric_polls"], 0);
    assert_eq!(report["periodic_readiness_polls"], 0);
    assert_eq!(report["actual_overlap"], "ACTUAL_OVERLAP_UNOBSERVED");
    assert_eq!(report["runtime"]["scheduler_workers"], 2);
    assert_eq!(report["runtime"]["embed_workers"], 5);
    assert_eq!(report["runtime"]["admission_capacity"], 128);
    assert_eq!(report["close_succeeded"], true);
    assert_eq!(report["complete"], true);
}

fn assert_closed_rows(path: &Path, bodies: &[String], report: &Value) {
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let counts: (usize, usize, usize, usize, usize, usize) = connection.query_row(
        "SELECT (SELECT COUNT(*) FROM canonical_nodes WHERE kind='doc' AND state='active' AND superseded_at IS NULL), (SELECT COUNT(*) FROM vector_default), (SELECT COUNT(*) FROM _fathomdb_vector_rows), (SELECT COUNT(*) FROM _fathomdb_vector_spans), (SELECT COUNT(*) FROM _fathomdb_projection_terminal WHERE state='up_to_date'), (SELECT COUNT(*) FROM _fathomdb_projection_terminal WHERE state='failed')",
        [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
    ).unwrap();
    let n = bodies.len();
    assert_eq!(counts, (n, n, n, n, n, 0), "closed store lacks exact physical/terminal closure");
    let mut statement = connection
        .prepare("SELECT body FROM canonical_nodes WHERE kind='doc' ORDER BY write_cursor")
        .unwrap();
    let actual = statement
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    assert_eq!(actual, bodies, "driver changed natural corpus bytes/order or reused a store");
    assert_eq!(report["final"]["pending_count"], 0);
    assert_eq!(report["final"]["physical_rows"], n);
    assert_eq!(report["final"]["span_rows"], n);
    assert_eq!(report["final"]["terminal_rows"], n);
    assert_eq!(report["final"]["failed_rows"], 0);
}

fn assert_probe(probe: &Value, batch: &Value, body: &str) {
    assert_eq!(probe["cursor"], batch["cursor"]);
    assert_eq!(probe["body"], body);
    assert_eq!(probe["point"]["body"], body, "point read must retain exact natural bytes");
    assert_eq!(probe["fts"]["member"], true, "current compiler FTS probe missed the actual cursor");
    assert!(probe["point"]["started_ns"].as_u64().unwrap() >= batch["ended_ns"].as_u64().unwrap());
    assert!(
        probe["point"]["ended_ns"].as_u64().unwrap()
            > probe["point"]["started_ns"].as_u64().unwrap()
    );
    assert!(
        probe["fts"]["ended_ns"].as_u64().unwrap() > probe["fts"]["started_ns"].as_u64().unwrap()
    );
}

#[test]
fn ordinary_held_cell_seeds_frozen_then_drains_before_exact_1000_hybrid_calls() {
    let directory = tempfile::tempdir().unwrap();
    let queries = queries();
    let bodies = vec!["corpus needle alpha".to_owned(), "corpus needle beta".to_owned()];
    for repeat in 0..2 {
        let store = directory.path().join(format!("held-{repeat}.sqlite"));
        assert!(!store.exists());
        let provider = Arc::new(CountingProvider::default());
        let origin = Instant::now();
        let report = schedule::run_ordinary(
            OrdinaryCell {
                path: &store,
                mode: Mode::Held,
                queries: &queries,
                origin,
                deadline: origin + Duration::from_secs(30),
            },
            bodies.clone().into_iter(),
            provider.clone(),
        )
        .expect("tiny held cell must finish cleanup before schedule assertions");
        assert_ordinary_calls(&report, &provider, &queries);
        assert_closed_rows(&store, &bodies, &report);
        assert_eq!(report["held"]["pending_count"], 2);
        assert_eq!(report["held"]["physical_rows"], 0);
        assert_eq!(report["held"]["span_rows"], 0);
        assert_eq!(report["held"]["generation_id"], report["final"]["generation_id"]);
        assert!(!report["final"]["generation_id"].as_str().unwrap().is_empty());
        let batches = report["batches"].as_array().unwrap();
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0]["rows"], 2);
        assert!(batches[0]["ended_ns"].as_u64().unwrap() <= report["release_ns"].as_u64().unwrap());
        let probes = report["probes"].as_array().unwrap();
        assert_eq!(probes.len(), 1);
        assert_probe(&probes[0], &batches[0], "corpus needle beta");
        assert!(
            report["drain"]["started_ns"].as_u64().unwrap()
                >= report["release_ns"].as_u64().unwrap()
        );
        assert!(
            report["calls"][0]["started_ns"].as_u64().unwrap()
                >= report["drain"]["ended_ns"].as_u64().unwrap()
        );
    }
}

#[test]
fn ordinary_overlap_cell_notifies_real_cohorts_and_reads_after_first_commit_without_interbatch_drain(
) {
    let directory = tempfile::tempdir().unwrap();
    let store = directory.path().join("overlap.sqlite");
    let queries = queries();
    let bodies = (0..1001).map(|index| format!("corpus needle row{index}")).collect::<Vec<_>>();
    let provider = Arc::new(CountingProvider::default());
    let origin = Instant::now();
    let report = schedule::run_ordinary(
        OrdinaryCell {
            path: &store,
            mode: Mode::Overlap,
            queries: &queries,
            origin,
            deadline: origin + Duration::from_secs(30),
        },
        bodies.clone().into_iter(),
        provider.clone(),
    )
    .expect("tiny overlap schedule must join/close before assertions");
    assert_ordinary_calls(&report, &provider, &queries);
    assert_closed_rows(&store, &bodies, &report);
    assert!(report["held"].is_null());
    assert!(report["release_ns"].is_null());
    let batches = report["batches"].as_array().unwrap();
    assert_eq!(batches.len(), 2);
    assert_eq!(batches[0]["rows"], 1000);
    assert_eq!(batches[1]["rows"], 1);
    assert!(batches[1]["cursor"].as_u64().unwrap() > batches[0]["cursor"].as_u64().unwrap());
    let probes = report["probes"].as_array().unwrap();
    assert_eq!(probes.len(), 2);
    assert_probe(&probes[0], &batches[0], "corpus needle row999");
    assert_probe(&probes[1], &batches[1], "corpus needle row1000");
    assert!(
        report["calls"][0]["started_ns"].as_u64().unwrap()
            >= batches[0]["ended_ns"].as_u64().unwrap()
    );
    assert!(
        report["drain"]["started_ns"].as_u64().unwrap()
            >= report["writer"]["ended_ns"].as_u64().unwrap()
    );
    assert!(
        report["drain"]["started_ns"].as_u64().unwrap()
            >= report["reader"]["ended_ns"].as_u64().unwrap()
    );
    assert_eq!(report["drain"]["calls_after_seed_started"], 1);
}
