//! Timing instrument for fresh versus cached verification on disposable stores.
//! See dev/experiments/fresh-open-verification/README.md. No production changes.
#![cfg(all(feature = "test-hooks", feature = "default-embedder"))]

use fathomdb_embedder::CandleBgeEmbedder;
use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{Engine, InitialState, PreparedWrite, SourceId};
use fathomdb_schema::SQLITE_SUFFIX;
use rusqlite::Connection;
use serde_json::json;
use std::{
    path::Path,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Instant,
};
use tempfile::TempDir;

struct Counted {
    inner: Arc<CandleBgeEmbedder>,
    calls: AtomicUsize,
}
impl Embedder for Counted {
    fn identity(&self) -> EmbedderIdentity {
        self.inner.identity()
    }
    fn embed(&self, text: &str) -> Result<Vector, EmbedderError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.inner.embed(text)
    }
    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vector>, EmbedderError> {
        self.calls.fetch_add(texts.len(), Ordering::SeqCst);
        self.inner.embed_batch(texts)
    }
}
fn load_model() -> CandleBgeEmbedder {
    let resolution = fathomdb_embedder::resolve_default_embedder_device_from_env().unwrap();
    let weights = fathomdb_embedder::loader::load_pinned_default_embedder().unwrap();
    assert_eq!(weights.bytes_downloaded, 0, "measurement requires cached assets");
    assert_eq!(
        weights
            .events
            .iter()
            .filter(|event| matches!(
                event,
                fathomdb_embedder::EmbedderEvent::DefaultEmbedderCacheHit { .. }
            ))
            .count(),
        3
    );
    CandleBgeEmbedder::new_from_weights_with_device_resolution(weights, &resolution).unwrap()
}
fn counted(inner: Arc<CandleBgeEmbedder>) -> Arc<Counted> {
    Arc::new(Counted { inner, calls: AtomicUsize::new(0) })
}
fn scalar(conn: &Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |row| row.get(0)).unwrap()
}
fn fingerprint(conn: &Connection) -> String {
    conn.query_row("SELECT value FROM _fathomdb_open_state WHERE key='vector_equivalence_verified_fingerprint'", [], |row| row.get(0)).unwrap()
}
fn seed(path: &Path, model: Arc<CandleBgeEmbedder>, rows: usize) {
    let setup = Engine::open_with_embedder_for_test(path, counted(model.clone())).unwrap();
    setup.engine.configure_vector_kind_for_test("doc").unwrap();
    let writes: Vec<_> = (0..rows).map(|i| PreparedWrite::Node {
        kind: "doc".into(), body: format!("Verification timing document {i}. This corpus discusses astronomy, databases, memory and information retrieval."),
        source_id: SourceId::new("fresh-open-measurement").unwrap(), logical_id: Some(format!("doc-{i}")),
        state: InitialState::Active, reason: None, valid_from: None, valid_until: None,
    }).collect();
    setup.engine.write(&writes).unwrap();
    setup.engine.drain(120_000).unwrap();
    setup.engine.close().unwrap();
    let counter = counted(model);
    let baseline = Engine::open_with_embedder_for_test(path, counter.clone()).unwrap();
    assert!(baseline.report.embedder_mean_vec_pinned);
    assert_eq!(counter.calls.load(Ordering::SeqCst), 90);
    assert_eq!(baseline.engine.read_embedding_readiness().unwrap().pending_count, 0);
    baseline.engine.close().unwrap();
    let conn = Connection::open(path).unwrap();
    assert_eq!(scalar(&conn, "SELECT count(*) FROM _fathomdb_vector_rows"), rows as i64);
    assert_eq!(
        scalar(
            &conn,
            "SELECT length(mean_vec) FROM _fathomdb_embedder_profiles WHERE profile='default'"
        ),
        1536
    );
    assert_eq!(scalar(&conn, "SELECT count(*) FROM _fathomdb_embed_probe"), 45);
    assert!(!fingerprint(&conn).is_empty());
}
fn sample(
    template: &Path,
    shared: Arc<CandleBgeEmbedder>,
    lifecycle: &str,
    fresh: bool,
    pair: usize,
    measured: bool,
) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("sample{SQLITE_SUFFIX}"));
    std::fs::copy(template, &path).unwrap();
    let conn = Connection::open(&path).unwrap();
    let expected_marker = fingerprint(&conn);
    if fresh {
        conn.execute(
            "DELETE FROM _fathomdb_open_state WHERE key='vector_equivalence_verified_fingerprint'",
            [],
        )
        .unwrap();
    }
    drop(conn);
    let total = Instant::now();
    let load = Instant::now();
    let model = if lifecycle == "reload" { Arc::new(load_model()) } else { shared };
    let load_ms = load.elapsed().as_secs_f64() * 1000.0;
    let counter = counted(model);
    let started = Instant::now();
    let opened = Engine::open_with_embedder_for_test(&path, counter.clone()).unwrap();
    let open_ms = started.elapsed().as_secs_f64() * 1000.0;
    let calls = counter.calls.load(Ordering::SeqCst);
    assert_eq!(calls, if fresh { 45 } else { 0 });
    let close = Instant::now();
    opened.engine.close().unwrap();
    drop(opened);
    drop(counter);
    let close_ms = close.elapsed().as_secs_f64() * 1000.0;
    let total_ms = total.elapsed().as_secs_f64() * 1000.0;
    let conn = Connection::open(&path).unwrap();
    assert_eq!(fingerprint(&conn), expected_marker);
    if measured {
        println!(
            "FRESH_OPEN_COST {}",
            json!({"pair":pair,"lifecycle":lifecycle,
            "verification":if fresh {"fresh"} else {"cached"},"probe_embeds":calls,
            "model_load_ms":load_ms,"open_ms":open_ms,"close_ms":close_ms,"total_ms":total_ms})
        );
    }
}

#[test]
#[ignore = "real pinned BGE model, explicit GPU/CPU timing run only"]
fn fresh_open_cost() {
    let repeats: usize =
        std::env::var("FRESH_OPEN_PAIRS").unwrap_or_else(|_| "10".into()).parse().unwrap();
    let rows: usize =
        std::env::var("FRESH_OPEN_ROWS").unwrap_or_else(|_| "300".into()).parse().unwrap();
    assert!(repeats >= 2 && rows >= 256);
    let requested = std::env::var("FATHOMDB_EMBED_DEVICE").expect("explicit device required");
    assert!(requested == "cpu" || requested.starts_with("cuda:"));
    let model = Arc::new(load_model());
    let device = model.device_label();
    assert_eq!(device, requested, "no device fallback permitted");
    let dir = TempDir::new().unwrap();
    let empty = dir.path().join(format!("vectorless{SQLITE_SUFFIX}"));
    let control = counted(model.clone());
    let opened = Engine::open_with_embedder_for_test(&empty, control.clone()).unwrap();
    opened.engine.close().unwrap();
    assert_eq!(control.calls.load(Ordering::SeqCst), 0);
    let template = dir.path().join(format!("template{SQLITE_SUFFIX}"));
    seed(&template, model.clone(), rows);
    println!(
        "FRESH_OPEN_META {}",
        json!({"requested":requested,"effective":device,"pooling":"Mean",
        "identity":format!("{:?}",model.identity()),"rows":rows,"pairs_per_lifecycle":repeats,
        "mean_pinned":true,"probe_count":45,"vectorless_embeds":0,
        "loader_download_bytes":0,"verified_cache_hits_per_construction":3,
        "sqlite_version":rusqlite::version(),"rayon_num_threads":std::env::var("RAYON_NUM_THREADS").ok()})
    );
    for lifecycle in ["reuse", "reload"] {
        sample(&template, model.clone(), lifecycle, false, 0, false);
        sample(&template, model.clone(), lifecycle, true, 0, false);
        for pair in 0..repeats {
            for fresh in if pair % 2 == 0 { [false, true] } else { [true, false] } {
                sample(&template, model.clone(), lifecycle, fresh, pair, true);
            }
        }
    }
}
